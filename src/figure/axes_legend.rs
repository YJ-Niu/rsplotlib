//! 图例渲染模块
//!
//! 在指定位置绘制图例框，包含每个 plot 调用对应的标签、线段、marker。

use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use plotters::style::text_anchor::{HPos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::core::colors::{RgbColor, to_plotters_color};
use crate::core::elements::PlotElement;
use crate::core::marker::draw_marker;
use crate::figure::axes::{DEFAULT_FONT_SCALE, scale_font};
use crate::utils::mathtext::{self, HAlign, VAlign};

/// 采样一个矩形填充区域内的代表点（3x3 网格），用于图例 "best" 位置的遮挡评估。
fn push_rect(pts: &mut Vec<(f64, f64)>, x0: f64, x1: f64, y0: f64, y1: f64) {
    let (xl, xr) = (x0.min(x1), x0.max(x1));
    let (yb, yt) = (y0.min(y1), y0.max(y1));
    for &fx in &[0.0, 0.5, 1.0] {
        for &fy in &[0.0, 0.5, 1.0] {
            pts.push((xl + (xr - xl) * fx, yb + (yt - yb) * fy));
        }
    }
}

/// 计算矩形区域内的数据点密度（用于评估空白区域）
fn region_density(
    pts: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    rect_x1: f64,
    rect_y1: f64,
    rect_x2: f64,
    rect_y2: f64,
) -> f64 {
    let rect_area = (rect_x2 - rect_x1).abs() * (rect_y2 - rect_y1).abs();
    if rect_area <= 0.0 {
        return 1.0;
    }

    let data_range_area = (x_max - x_min).abs() * (y_max - y_min).abs();
    let relative_area = rect_area / data_range_area;

    let count = pts
        .iter()
        .filter(|&&(x, y)| x >= rect_x1 && x <= rect_x2 && y >= rect_y1 && y <= rect_y2)
        .count();

    if count == 0 {
        0.0
    } else {
        (count as f64 / relative_area).min(1.0)
    }
}

/// 智能查找最佳图例位置
/// 策略：密集扫描整个绘图区域，找到真正的空白位置，优先选择零密度区域
fn find_best_blank_region(
    pts: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    legend_width: f64,
    legend_height: f64,
    target_hpos: HPos,
    target_vpos: VPos,
) -> (f64, f64, f64, f64) {
    let px = (x_max - x_min).abs() * 0.02;
    let py = (y_max - y_min).abs() * 0.02;

    let x_range = (x_max - x_min).abs();
    let y_range = (y_max - y_min).abs();

    // 使用更密集的网格扫描整个绘图区域
    let steps_x = 12;
    let steps_y = 12;

    let scan_x_start = x_min + px;
    let scan_x_end = x_max - px - legend_width;
    let scan_y_start = y_min + py;
    let scan_y_end = y_max - py - legend_height;

    let mut zero_density_candidates: Vec<(f64, f64, f64, f64)> = Vec::new();
    let mut all_candidates: Vec<(f64, f64, f64, f64, f64)> = Vec::new();

    if scan_x_end > scan_x_start && scan_y_end > scan_y_start {
        let step_x = (scan_x_end - scan_x_start) / (steps_x - 1) as f64;
        let step_y = (scan_y_end - scan_y_start) / (steps_y - 1) as f64;

        for i in 0..steps_x {
            for j in 0..steps_y {
                let x1 = scan_x_start + step_x * i as f64;
                let y1 = scan_y_start + step_y * j as f64;
                let x2 = x1 + legend_width;
                let y2 = y1 + legend_height;

                // 计算该区域内的数据点数量（直接计数，比密度更直观）
                let count = pts
                    .iter()
                    .filter(|&&(x, y)| x >= x1 && x <= x2 && y >= y1 && y <= y2)
                    .count();

                // 转换为密度（0-1范围）
                let density = if count == 0 {
                    0.0
                } else {
                    let rect_area = (x2 - x1) * (y2 - y1);
                    let data_area = x_range * y_range;
                    let relative_area = rect_area / data_area;
                    (count as f64 / relative_area).min(1.0)
                };

                all_candidates.push((x1, y1, x2, y2, density));

                // 如果密度为0，加入零密度候选列表
                if count == 0 {
                    zero_density_candidates.push((x1, y1, x2, y2));
                }
            }
        }
    }

    // 确定目标位置（用户期望的位置）
    let target_center_x = match target_hpos {
        HPos::Left => x_min + x_range * 0.15,
        HPos::Center => (x_min + x_max) / 2.0,
        HPos::Right => x_max - x_range * 0.15,
    };

    let target_center_y = match target_vpos {
        VPos::Bottom => y_min + y_range * 0.15,
        VPos::Center => (y_min + y_max) / 2.0,
        VPos::Top => y_max - y_range * 0.15,
    };

    // 如果有零密度的位置，从中选择最接近目标位置的
    if !zero_density_candidates.is_empty() {
        let best = zero_density_candidates
            .iter()
            .min_by(|a, b| {
                let dist_a = ((a.0 + a.2) / 2.0 - target_center_x).powi(2)
                    + ((a.1 + a.3) / 2.0 - target_center_y).powi(2);
                let dist_b = ((b.0 + b.2) / 2.0 - target_center_x).powi(2)
                    + ((b.1 + b.3) / 2.0 - target_center_y).powi(2);
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        return *best;
    }

    // 如果没有零密度位置，从所有候选中选择密度最低的
    if all_candidates.is_empty() {
        return (
            x_min + px,
            y_min + py,
            x_min + px + legend_width,
            y_min + py + legend_height,
        );
    }

    // 按密度升序排序，选择密度最低的位置
    all_candidates.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap_or(std::cmp::Ordering::Equal));

    // 找到所有与最低密度相近的位置（容差范围内）
    let min_density = all_candidates[0].4;
    let tolerance = 0.05;
    let low_density_candidates: Vec<_> = all_candidates
        .iter()
        .filter(|c| c.4 <= min_density + tolerance)
        .collect();

    // 从低密度位置中选择最接近目标位置的
    let best = low_density_candidates
        .iter()
        .min_by(|a, b| {
            let dist_a = ((a.0 + a.2) / 2.0 - target_center_x).powi(2)
                + ((a.1 + a.3) / 2.0 - target_center_y).powi(2);
            let dist_b = ((b.0 + b.2) / 2.0 - target_center_x).powi(2)
                + ((b.1 + b.3) / 2.0 - target_center_y).powi(2);
            dist_a
                .partial_cmp(&dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    (best.0, best.1, best.2, best.3)
}

/// 从所有绘图元素中收集代表性数据点（数据坐标），供图例自动避让使用。
/// 对填充类元素（柱状/直方/填充区）采样其覆盖区域，对线/点类采样其顶点。
fn collect_data_points(elements: &[PlotElement]) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for el in elements {
        match el {
            PlotElement::Line { x, y, .. } => {
                for (xi, yi) in x.iter().zip(y.iter()) {
                    if xi.is_finite() && yi.is_finite() {
                        pts.push((*xi, *yi));
                    }
                }
            }
            PlotElement::Scatter { x, y, .. }
            | PlotElement::ScatterMulti { x, y, .. }
            | PlotElement::Stem { x, y, .. }
            | PlotElement::Step { x, y, .. }
            | PlotElement::ErrorBar { x, y, .. } => {
                for (xv, yv) in x.iter().zip(y.iter()) {
                    pts.push((*xv, *yv));
                }
            }
            PlotElement::Bar {
                x, height, width, ..
            } => {
                for (xc, h) in x.iter().zip(height.iter()) {
                    push_rect(&mut pts, xc - width / 2.0, xc + width / 2.0, 0.0, *h);
                }
            }
            PlotElement::BarH {
                y, width, height, ..
            } => {
                for (yc, w) in y.iter().zip(width.iter()) {
                    push_rect(&mut pts, 0.0, *w, yc - height / 2.0, yc + height / 2.0);
                }
            }
            PlotElement::Hist {
                bars,
                outlines,
                orientation,
                ..
            } => {
                let horizontal = orientation == "horizontal";
                for ds in bars {
                    for &(pl, pr, vb, vt) in ds {
                        if horizontal {
                            push_rect(&mut pts, vb, vt, pl, pr);
                        } else {
                            push_rect(&mut pts, pl, pr, vb, vt);
                        }
                    }
                }
                // histtype="step" 只填充 outlines（bars 为空），需采样阶梯折线顶点，
                // 否则 "best" 自动避让看不到这条曲线，会把图例压在曲线上。
                for ds in outlines {
                    for &(pos, val) in ds {
                        if horizontal {
                            pts.push((val, pos));
                        } else {
                            pts.push((pos, val));
                        }
                    }
                }
            }
            PlotElement::Violin {
                positions,
                widths,
                vert,
                ..
            } => {
                let is_vertical = *vert;
                for (di, &pos) in positions.iter().enumerate() {
                    let width = *widths.get(di).unwrap_or(&0.5);
                    if is_vertical {
                        push_rect(&mut pts, pos - width, pos + width, 0.0, 1.0);
                    } else {
                        push_rect(&mut pts, 0.0, 1.0, pos - width, pos + width);
                    }
                }
            }
            PlotElement::FillBetween { x, y1, y2, .. } => {
                for (i, &xi) in x.iter().enumerate() {
                    let yl = *y1.get(i).unwrap_or(&0.0);
                    let yh = *y2.get(i).unwrap_or(&0.0);
                    push_rect(&mut pts, xi, xi, yl, yh);
                }
            }
            PlotElement::Stack { x, y_series, .. } => {
                let mut acc = vec![0.0f64; x.len()];
                for series in y_series {
                    for i in 0..x.len().min(series.len()) {
                        let top = acc[i] + series[i];
                        push_rect(&mut pts, x[i], x[i], acc[i], top);
                        acc[i] = top;
                    }
                }
            }
            _ => {}
        }
    }
    pts
}

/// 计算图例框的四角坐标（数据坐标）。
fn box_from_anchor(
    h_pos: HPos,
    v_pos: VPos,
    x_anchor: f64,
    y_anchor: f64,
    legend_width: f64,
    legend_height: f64,
) -> (f64, f64, f64, f64) {
    let (box_x1, box_x2) = match h_pos {
        HPos::Right => (x_anchor - legend_width, x_anchor),
        HPos::Left => (x_anchor, x_anchor + legend_width),
        HPos::Center => (x_anchor - legend_width / 2.0, x_anchor + legend_width / 2.0),
    };
    let (box_y1, box_y2) = match v_pos {
        VPos::Top => (y_anchor - legend_height, y_anchor),
        VPos::Bottom => (y_anchor, y_anchor + legend_height),
        VPos::Center => (
            y_anchor - legend_height / 2.0,
            y_anchor + legend_height / 2.0,
        ),
    };
    (box_x1, box_y1, box_x2, box_y2)
}

/// 在候选位置中挑选与数据遮挡最少的图例框（matplotlib `loc='best'` 语义）。
/// 沿绘图区域四周以一定步长扫描，生成大量候选位置，取遮挡点数最少者。
/// 支持尝试多种图例尺寸（如水平布局和垂直布局），选择最佳的位置和尺寸组合。
fn best_box(
    pts: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    legend_width: f64,
    legend_height: f64,
    alt_width: Option<f64>,
    alt_height: Option<f64>,
) -> (f64, f64, f64, f64) {
    let x_range = (x_max - x_min).abs();
    let y_range = (y_max - y_min).abs();
    let px = x_range * 0.02;
    let py = y_range * 0.02;

    let mut candidates: Vec<(f64, f64, f64, f64)> = Vec::new();

    let mut add_candidate = |bx1: f64, by1: f64, bx2: f64, by2: f64| {
        let bx1_clamped = bx1.max(x_min + px);
        let bx2_clamped = bx2.min(x_max - px);
        let by1_clamped = by1.max(y_min + py);
        let by2_clamped = by2.min(y_max - py);
        if bx2_clamped > bx1_clamped && by2_clamped > by1_clamped {
            candidates.push((bx1_clamped, by1_clamped, bx2_clamped, by2_clamped));
        }
    };

    let sizes = if let (Some(aw), Some(ah)) = (alt_width, alt_height) {
        vec![(legend_width, legend_height), (aw, ah)]
    } else {
        vec![(legend_width, legend_height)]
    };

    for &(lw, lh) in &sizes {
        let x_step = (x_range * 0.08).max(lw * 0.5);
        let y_step = (y_range * 0.08).max(lh * 0.5);

        let mut x_pos = x_min + px;
        while x_pos + lw <= x_max - px {
            add_candidate(x_pos, y_min + py, x_pos + lw, y_min + py + lh);
            x_pos += x_step;
        }
        if x_max - px - lw > x_min + px {
            add_candidate(x_max - px - lw, y_min + py, x_max - px, y_min + py + lh);
        }

        x_pos = x_min + px;
        while x_pos + lw <= x_max - px {
            add_candidate(x_pos, y_max - py - lh, x_pos + lw, y_max - py);
            x_pos += x_step;
        }
        if x_max - px - lw > x_min + px {
            add_candidate(x_max - px - lw, y_max - py - lh, x_max - px, y_max - py);
        }

        let mut y_pos = y_min + py;
        while y_pos + lh <= y_max - py {
            add_candidate(x_min + px, y_pos, x_min + px + lw, y_pos + lh);
            y_pos += y_step;
        }
        if y_max - py - lh > y_min + py {
            add_candidate(x_min + px, y_max - py - lh, x_min + px + lw, y_max - py);
        }

        y_pos = y_min + py;
        while y_pos + lh <= y_max - py {
            add_candidate(x_max - px - lw, y_pos, x_max - px, y_pos + lh);
            y_pos += y_step;
        }
        if y_max - py - lh > y_min + py {
            add_candidate(x_max - px - lw, y_max - py - lh, x_max - px, y_max - py);
        }
    }

    if candidates.is_empty() {
        return (
            x_max - px - legend_width,
            y_min + py,
            x_max - px,
            y_min + py + legend_height,
        );
    }

    let mut best = candidates[0];
    let mut best_score = usize::MAX;
    for &(bx1, by1, bx2, by2) in &candidates {
        let score = pts
            .iter()
            .filter(|&&(x, y)| x >= bx1 && x <= bx2 && y >= by1 && y <= by2)
            .count();
        if score < best_score {
            best_score = score;
            best = (bx1, by1, bx2, by2);
            if score == 0 {
                break;
            }
        }
    }
    best
}

/// 生成圆角矩形的多边形顶点（数据坐标）。
///
/// `rx` / `ry` 分别为 x / y 方向的圆角半径（数据坐标）。调用方应根据像素比例
/// 换算这两个半径，使圆角在 x、y 两个方向上呈现出视觉一致的圆弧。
/// 返回的顶点按逆时针顺序排列，可直接用于 `Polygon` 填充；描边时把首点追加到末尾闭合。
fn rounded_rect_points(x1: f64, y1: f64, x2: f64, y2: f64, rx: f64, ry: f64) -> Vec<(f64, f64)> {
    let xl = x1.min(x2);
    let xr = x1.max(x2);
    let yb = y1.min(y2);
    let yt = y1.max(y2);
    // 半径不得超过半边长，避免相邻圆角重叠
    let rx = rx.clamp(0.0, (xr - xl) / 2.0);
    let ry = ry.clamp(0.0, (yt - yb) / 2.0);

    const STEPS: usize = 8;
    // 四个圆角：圆心 (cx, cy) 与起止角度（弧度），逆时针
    let half_pi = std::f64::consts::FRAC_PI_2;
    let pi = std::f64::consts::PI;
    let corners = [
        (xr - rx, yb + ry, -half_pi, 0.0),                  // 右下
        (xr - rx, yt - ry, 0.0, half_pi),                   // 右上
        (xl + rx, yt - ry, half_pi, pi),                    // 左上
        (xl + rx, yb + ry, pi, 1.5 * std::f64::consts::PI), // 左下
    ];
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(corners.len() * (STEPS + 1));
    for (cx, cy, a0, a1) in corners {
        for s in 0..=STEPS {
            let t = a0 + (a1 - a0) * (s as f64 / STEPS as f64);
            pts.push((cx + rx * t.cos(), cy + ry * t.sin()));
        }
    }
    pts
}

/// 渲染图例（如果设置了 `legend_loc` 且 `legend_labels` 非空）
///
/// # 参数
/// - `chart`: plotters 的 chart 上下文
/// - `legend_loc`: 图例位置字符串（如 "upper right"、"lower left"、"best" 等）
/// - `legend_labels`: 标签列表，每项为 (label, color, linestyle, marker, linewidth)
/// - `elements`: 已绘制的数据元素，用于 "best" 自动避让计算
/// - `font_scale`: 字体缩放系数
/// - `x_min`, `x_max`, `y_min`, `y_max`: 数据范围（log 刻度下为 log10 变换后的值）
/// - `xlog`, `ylog`: 对应轴是否为对数刻度
/// - `facecolor`: 图例框背景色，`None` 时用默认白色
/// - `framealpha`: 图例框背景不透明度，`None` 时用默认 0.85
/// - `edgecolor`: 图例框边框色，`None` 时用默认浅灰
/// - `fontsize`: 图例文字基础字号（point），`None` 时用默认 11.0
/// - `ncol`: 图例列数，`None` 时根据位置和空间自动判定
#[allow(clippy::too_many_arguments)]
pub fn draw_legend<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    legend_loc: Option<&String>,
    legend_labels: &[(String, RgbColor, String, Option<String>, f64, f64)],
    elements: &[PlotElement],
    font_scale: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    xlog: bool,
    ylog: bool,
    facecolor: Option<RgbColor>,
    framealpha: Option<f64>,
    edgecolor: Option<RgbColor>,
    fontsize: Option<f64>,
    ncol: Option<usize>,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if let Some(loc) = legend_loc {
        if legend_labels.is_empty() {
            return Ok(());
        }

        let entry_count = legend_labels.len();
        let x_range = (x_max - x_min).abs();
        let y_range = (y_max - y_min).abs();

        // 数据坐标 <-> 像素换算：图例的尺寸与间距均以像素/字号设定，再换算到数据坐标，
        // 从而不随数据范围畸变。
        let (pw, ph) = chart.plotting_area().dim_in_pixel();
        let x_per_px = if pw > 0 {
            x_range / pw as f64
        } else {
            x_range * 0.001
        };
        let y_per_px = if ph > 0 {
            y_range / ph as f64
        } else {
            y_range * 0.001
        };

        // 图例文字像素字号（与下方文字绘制保持一致）。基础字号可由调用方覆盖
        // （如 stylely 缩放），默认 11.0 point。
        let base_fs = fontsize.unwrap_or(11.0);
        let label_fs = scale_font(base_fs * DEFAULT_FONT_SCALE, font_scale);

        let pad_h_px = 5.0 * font_scale;
        let handle_px = label_fs * 1.4;
        let gap_px = 3.0 * font_scale;
        let col_gap_px = 7.0 * font_scale;
        let max_text_px = legend_labels
            .iter()
            .map(|(label, ..)| mathtext::measure_plain(label.as_str(), None, label_fs).0)
            .fold(0.0_f64, f64::max);
        // 每列内容宽度（含左侧 padding，不含右 padding）。
        // 总图例宽度 = N * entry_width_px + (N-1) * col_gap_px + pad_h_px（最右列右 padding）
        // 这样列与列之间间距 = col_gap_px（而非 col_gap_px + 2*pad_h_px），避免多列时图例右侧出现大片空白。
        let entry_width_px = pad_h_px + handle_px + gap_px + max_text_px;

        let row_px = label_fs * 1.35;
        let pad_v_px = label_fs * 0.35;
        let entry_height = row_px * y_per_px;

        let ncol = if let Some(n) = ncol {
            n.max(1).min(entry_count)
        } else {
            let available_width_px = (pw as f64 - 40.0 * font_scale).max(100.0);
            let single_col_width = entry_width_px;
            let max_possible_ncol = (available_width_px / single_col_width).floor() as usize;

            let is_top_bottom_loc = matches!(
                loc.as_str(),
                "upper center"
                    | "lower center"
                    | "upper right"
                    | "upper left"
                    | "lower right"
                    | "lower left"
                    | "best"
            );

            if is_top_bottom_loc && max_possible_ncol >= 2 && entry_count >= 2 {
                let pts = collect_data_points(elements);

                let px = x_range * 0.02;
                let py = y_range * 0.02;

                let mut best_ncol = 1;
                let mut best_density = 1.0;

                for try_ncol in 1..=max_possible_ncol.min(entry_count) {
                    let rows_per_col_try = entry_count.div_ceil(try_ncol);
                    let legend_height_try_px = row_px * rows_per_col_try as f64 + 2.0 * pad_v_px;
                    let legend_height_try = legend_height_try_px * y_per_px;
                    let legend_width_try_px = entry_width_px * try_ncol as f64
                        + col_gap_px * (try_ncol - 1) as f64
                        + pad_h_px;
                    let legend_width_try = legend_width_try_px * x_per_px;

                    let mut current_density: f64 = 1.0;
                    let candidate_positions = match loc.as_str() {
                        "upper right" => vec![(HPos::Right, VPos::Top)],
                        "upper left" => vec![(HPos::Left, VPos::Top)],
                        "lower right" => vec![(HPos::Right, VPos::Bottom)],
                        "lower left" => vec![(HPos::Left, VPos::Bottom)],
                        "upper center" => vec![(HPos::Center, VPos::Top)],
                        "lower center" => vec![(HPos::Center, VPos::Bottom)],
                        "best" => vec![
                            (HPos::Right, VPos::Top),
                            (HPos::Left, VPos::Top),
                            (HPos::Right, VPos::Bottom),
                            (HPos::Left, VPos::Bottom),
                        ],
                        _ => vec![
                            (HPos::Right, VPos::Top),
                            (HPos::Left, VPos::Top),
                            (HPos::Right, VPos::Bottom),
                            (HPos::Left, VPos::Bottom),
                        ],
                    };

                    for &(h_pos, v_pos) in &candidate_positions {
                        let (x_anchor, y_anchor) = match (h_pos, v_pos) {
                            (HPos::Left, VPos::Top) => (x_min + px, y_max - py),
                            (HPos::Right, VPos::Top) => (x_max - px, y_max - py),
                            (HPos::Left, VPos::Bottom) => (x_min + px, y_min + py),
                            (HPos::Right, VPos::Bottom) => (x_max - px, y_min + py),
                            (HPos::Center, VPos::Top) => ((x_min + x_max) / 2.0, y_max - py),
                            (HPos::Center, VPos::Bottom) => ((x_min + x_max) / 2.0, y_min + py),
                            _ => ((x_min + x_max) / 2.0, (y_min + y_max) / 2.0),
                        };
                        let (bx1, by1, bx2, by2) = box_from_anchor(
                            h_pos,
                            v_pos,
                            x_anchor,
                            y_anchor,
                            legend_width_try,
                            legend_height_try,
                        );
                        let density =
                            region_density(&pts, x_min, x_max, y_min, y_max, bx1, by1, bx2, by2);
                        current_density = current_density.min(density);
                    }

                    if current_density < best_density {
                        best_density = current_density;
                        best_ncol = try_ncol;
                        if best_density == 0.0 {
                            break;
                        }
                    }
                }
                best_ncol
            } else {
                1
            }
        };

        let rows_per_col = entry_count.div_ceil(ncol);
        let legend_height_px = row_px * rows_per_col as f64 + 2.0 * pad_v_px;

        let legend_height = legend_height_px * y_per_px;

        // 计算最大可用图例高度，以及需要省略的条目数
        let _px = x_range * 0.02;
        let py = y_range * 0.02;
        let max_legend_height = y_max - y_min - 2.0 * py;

        let mut display_entries = legend_labels;
        let mut needs_ellipsis = false;

        if legend_height > max_legend_height {
            let max_entries =
                ((max_legend_height - 2.0 * pad_v_px * y_per_px) / entry_height).floor() as usize;
            if max_entries < entry_count {
                display_entries = &legend_labels[..max_entries.max(1)];
                needs_ellipsis = true;
            }
        }

        let display_count = display_entries.len();
        let display_rows = display_count.div_ceil(ncol);
        // 行主序布局：实际使用的列数就是 ncol（每行都用 ncol 列，最后一行可能不满）
        let actual_ncol = ncol;
        // 总图例宽度 = N 列内容（每列含左 pad_h） + (N-1) 列间距 + 最右列右 pad_h
        let legend_width_px =
            entry_width_px * actual_ncol as f64 + col_gap_px * (actual_ncol - 1) as f64 + pad_h_px;
        let legend_width = legend_width_px * x_per_px;
        let rows_with_ellipsis = if needs_ellipsis {
            display_rows + 1
        } else {
            display_rows
        };
        let display_legend_height_px = row_px * rows_with_ellipsis as f64 + 2.0 * pad_v_px;
        let display_legend_height = display_legend_height_px * y_per_px;

        // 收集数据点，用于空白区域检测
        let mut data_pts = collect_data_points(elements);
        if xlog || ylog {
            for p in data_pts.iter_mut() {
                if xlog {
                    p.0 = if p.0 > 0.0 {
                        p.0.log10()
                    } else {
                        x_min.min(x_max)
                    };
                }
                if ylog {
                    p.1 = if p.1 > 0.0 {
                        p.1.log10()
                    } else {
                        y_min.min(y_max)
                    };
                }
            }
        }

        // 已知固定位置：先尝试目标位置，然后在附近搜索空白区域
        // 其余（含 "best" 与未识别值）自动避让数据
        let (box_x1, box_y1, box_x2, box_y2) = match loc.as_str() {
            "upper right" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Right,
                VPos::Top,
            ),
            "upper left" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Left,
                VPos::Top,
            ),
            "lower right" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Right,
                VPos::Bottom,
            ),
            "lower left" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Left,
                VPos::Bottom,
            ),
            "center" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Center,
                VPos::Center,
            ),
            "right" | "center right" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Right,
                VPos::Center,
            ),
            "center left" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Left,
                VPos::Center,
            ),
            "lower center" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Center,
                VPos::Bottom,
            ),
            "upper center" => find_best_blank_region(
                &data_pts,
                x_min,
                x_max,
                y_min,
                y_max,
                legend_width,
                display_legend_height,
                HPos::Center,
                VPos::Top,
            ),
            _ => {
                let vertical_width = entry_width_px * x_per_px;
                let vertical_rows = display_count;
                let vertical_height_px = row_px * vertical_rows as f64 + 2.0 * pad_v_px;
                let vertical_height = vertical_height_px * y_per_px;
                best_box(
                    &data_pts,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    legend_width,
                    display_legend_height,
                    Some(vertical_width),
                    Some(vertical_height),
                )
            }
        };

        // 图例框背景/边框样式：默认沿用半透明白底 + 浅灰边框；
        // 调用方（如 stylely 捕获的样式）可覆盖为任意颜色与不透明度。
        // 当背景色接近白色且未指定边框色时，自动使用稍深的灰色以确保可见性。
        let fc = facecolor.unwrap_or(RgbColor(255, 255, 255));
        let alpha = framealpha.unwrap_or(0.85).clamp(0.0, 1.0);
        let ec = if let Some(c) = edgecolor {
            c
        } else {
            let luminance =
                (fc.0 as f64 * 0.299 + fc.1 as f64 * 0.587 + fc.2 as f64 * 0.114) / 255.0;
            if luminance > 0.9 {
                RgbColor(153, 153, 153)
            } else {
                RgbColor(180, 180, 180)
            }
        };
        let _bg_fill: ShapeStyle = to_plotters_color(fc).mix(alpha).filled();
        let bg_border: ShapeStyle = to_plotters_color(ec).stroke_width(1);

        // 圆角半径：以像素为基准，再按数据/像素比例换算到数据坐标，
        // 使 x、y 两个方向的圆角在视觉上一致（圆弧而非椭圆弧）。
        let r_px = 8.0 * font_scale;
        let rx = if pw > 0 {
            r_px * x_range / pw as f64
        } else {
            0.0
        };
        let ry = if ph > 0 {
            r_px * y_range / ph as f64
        } else {
            0.0
        };
        let corner_pts = rounded_rect_points(box_x1, box_y1, box_x2, box_y2, rx, ry);

        // 半透明白色圆角填充
        chart
            .draw_series(std::iter::once(Polygon::new(corner_pts.clone(), _bg_fill)))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend bg: {}", e)))?;
        // 圆角边框（闭合路径）
        let mut border_pts = corner_pts;
        if let Some(&first) = border_pts.first() {
            border_pts.push(first);
        }
        chart
            .draw_series(std::iter::once(PathElement::new(border_pts, bg_border)))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend border: {}", e)))?;

        // 图例线段的虚线/点线间隔需以像素为基准，再换算到数据坐标，
        // 否则固定的数据单位间隔在不同数据范围下会失效（例如整段被一个"虚线"填满而显示为实线）。
        let dash_unit = font_scale * x_per_px;

        for (i, (label, color, ls, marker_opt, lw, alpha)) in display_entries.iter().enumerate() {
            let col = i % ncol;
            let row = i / ncol;

            let col_offset_px = col as f64 * (entry_width_px + col_gap_px);
            let x_col_start = box_x1 + col_offset_px * x_per_px;

            let y_pos =
                box_y2 - pad_v_px * y_per_px - entry_height * 0.5 - row as f64 * entry_height;

            let x_line_start = x_col_start + pad_h_px * x_per_px;
            let x_line_end = x_line_start + handle_px * x_per_px;
            let x_text = x_line_end + gap_px * x_per_px;

            let rgb = to_plotters_color(*color);
            // 使用实际的 linewidth（与数据线保持一致），将 points 转换为像素
            // plotters stroke_width(n) 实际渲染为 2n-1 像素，使用 stroke = max(1, width_px - 1) 接近 mpl
            let lw_px = ((*lw) * font_scale).max(1.0).round() as u32;
            let legend_stroke = (lw_px as i32 - 1).max(1) as u32;
            let line_style: ShapeStyle = rgb.stroke_width(legend_stroke);

            // 根据线型绘制图例线段或填充色块
            match ls.as_str() {
                "fill" => {
                    let rect_height = handle_px * y_per_px * 0.6;
                    let y_bottom = y_pos - rect_height / 2.0;
                    let y_top = y_pos + rect_height / 2.0;
                    chart
                        .draw_series(std::iter::once(Rectangle::new(
                            [(x_line_start, y_bottom), (x_line_end, y_top)],
                            rgb.mix(*alpha).filled(),
                        )))
                        .map_err(|e| PyRuntimeError::new_err(format!("Legend fill: {}", e)))?;
                }
                "--" => {
                    let dash_len = 6.0 * dash_unit;
                    let gap_len = 8.0 * dash_unit;
                    let mut pos = x_line_start;
                    let mut drawing = true;
                    while pos < x_line_end {
                        let seg_end = if drawing {
                            (pos + dash_len).min(x_line_end)
                        } else {
                            (pos + gap_len).min(x_line_end)
                        };
                        if drawing {
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(pos, y_pos), (seg_end, y_pos)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("Legend dashed: {}", e))
                                })?;
                        }
                        pos = seg_end;
                        drawing = !drawing;
                    }
                }
                ":" => {
                    let dot_len = 1.5 * dash_unit;
                    let gap_len = 3.0 * dash_unit;
                    let mut pos = x_line_start;
                    let mut drawing = true;
                    while pos < x_line_end {
                        let seg_end = if drawing {
                            (pos + dot_len).min(x_line_end)
                        } else {
                            (pos + gap_len).min(x_line_end)
                        };
                        if drawing {
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(pos, y_pos), (seg_end, y_pos)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("Legend dotted: {}", e))
                                })?;
                        }
                        pos = seg_end;
                        drawing = !drawing;
                    }
                }
                "-." => {
                    let dash_len = 6.0 * dash_unit;
                    let dot_len = 1.5 * dash_unit;
                    let gap_len = 3.0 * dash_unit;
                    let mut pos = x_line_start;
                    let mut is_dash = true;
                    while pos < x_line_end {
                        let mark_len = if is_dash { dash_len } else { dot_len };
                        let seg_end = (pos + mark_len).min(x_line_end);
                        chart
                            .draw_series(std::iter::once(PathElement::new(
                                vec![(pos, y_pos), (seg_end, y_pos)],
                                line_style,
                            )))
                            .map_err(|e| {
                                PyRuntimeError::new_err(format!("Legend dash-dot: {}", e))
                            })?;
                        pos = seg_end;
                        let gap_end = (pos + gap_len).min(x_line_end);
                        pos = gap_end;
                        is_dash = !is_dash;
                    }
                }
                _ => {
                    chart
                        .draw_series(std::iter::once(PathElement::new(
                            vec![(x_line_start, y_pos), (x_line_end, y_pos)],
                            line_style,
                        )))
                        .map_err(|e| {
                            PyRuntimeError::new_err(format!("Failed to draw legend line: {}", e))
                        })?;
                }
            }

            if let Some(mkr) = marker_opt
                && !mkr.is_empty()
            {
                let mid_x = (x_line_start + x_line_end) / 2.0;
                draw_marker(chart, mkr, mid_x, y_pos, x_range * 0.01, rgb, rgb, 1.0, 0.0).map_err(
                    |e| PyRuntimeError::new_err(format!("Failed to draw legend marker: {}", e)),
                )?;
            }

            let text_nudge = if mathtext::contains_ir(label) {
                -0.45 * label_fs
            } else {
                -0.2 * label_fs
            };
            mathtext::draw_math_chart(
                chart,
                x_text,
                y_pos,
                label,
                label_fs,
                BLACK,
                None,
                HAlign::Left,
                VAlign::Top,
                0.0,
                0.0,
                text_nudge,
                None,
                x_min,
                x_max,
                y_min,
                y_max,
            )?;
        }

        if needs_ellipsis {
            let ellipsis_row = display_rows;
            let ellipsis_y_pos = box_y2
                - pad_v_px * y_per_px
                - entry_height * 0.5
                - ellipsis_row as f64 * entry_height;
            let ellipsis_x =
                box_x1 + pad_h_px * x_per_px + handle_px * x_per_px + gap_px * x_per_px;
            let ellipsis_text = "...";
            mathtext::draw_math_chart(
                chart,
                ellipsis_x,
                ellipsis_y_pos,
                ellipsis_text,
                label_fs,
                BLACK,
                None,
                HAlign::Left,
                VAlign::Top,
                0.0,
                0.0,
                -0.2 * label_fs,
                None,
                x_min,
                x_max,
                y_min,
                y_max,
            )?;
        }
    }
    Ok(())
}
