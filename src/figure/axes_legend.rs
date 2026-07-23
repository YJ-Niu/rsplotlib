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
/// 候选按偏好顺序排列，遮挡点数相同时取靠前者。
fn best_box(
    pts: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    legend_width: f64,
    legend_height: f64,
) -> (f64, f64, f64, f64) {
    let x_range = (x_max - x_min).abs();
    let y_range = (y_max - y_min).abs();
    let px = x_range * 0.02;
    let py = y_range * 0.02;
    let xr2 = x_max - px;
    let xr1 = xr2 - legend_width;
    let xl1 = x_min + px;
    let xl2 = xl1 + legend_width;
    let xc1 = (x_min + x_max) / 2.0 - legend_width / 2.0;
    let xc2 = xc1 + legend_width;
    let yt2 = y_max - py;
    let yt1 = yt2 - legend_height;
    let yb1 = y_min + py;
    let yb2 = yb1 + legend_height;
    let yc1 = (y_min + y_max) / 2.0 - legend_height / 2.0;
    let yc2 = yc1 + legend_height;
    // 偏好顺序：四角优先，其次边中，最后正中。
    let candidates = [
        (xr1, yt1, xr2, yt2), // upper right
        (xl1, yt1, xl2, yt2), // upper left
        (xr1, yb1, xr2, yb2), // lower right
        (xl1, yb1, xl2, yb2), // lower left
        (xr1, yc1, xr2, yc2), // center right
        (xl1, yc1, xl2, yc2), // center left
        (xc1, yt1, xc2, yt2), // upper center
        (xc1, yb1, xc2, yb2), // lower center
        (xc1, yc1, xc2, yc2), // center
    ];
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

        let pad_h_px = 8.0 * font_scale;
        let handle_px = label_fs * 1.6;
        let gap_px = 3.5 * font_scale;
        let col_gap_px = 12.0 * font_scale;
        let max_text_px = legend_labels
            .iter()
            .map(|(label, ..)| mathtext::measure_plain(label.as_str(), None, label_fs).0)
            .fold(0.0_f64, f64::max);
        let entry_width_px = pad_h_px + handle_px + gap_px + max_text_px + pad_h_px;

        let row_px = label_fs * 1.6;
        let pad_v_px = label_fs * 0.55;
        let entry_height = row_px * y_per_px;

        let ncol = if let Some(n) = ncol {
            n.max(1).min(entry_count)
        } else {
            let available_width_px = (pw as f64 - 40.0 * font_scale).max(100.0);
            let single_col_width = entry_width_px;
            let max_possible_ncol = (available_width_px / single_col_width).floor() as usize;

            let is_center_loc = matches!(loc.as_str(), "upper center" | "lower center" | "center");

            if is_center_loc && max_possible_ncol >= 2 && entry_count >= 2 {
                max_possible_ncol.min(entry_count)
            } else {
                1
            }
        };

        let rows_per_col = entry_count.div_ceil(ncol);
        let legend_width_px = entry_width_px * ncol as f64 + col_gap_px * (ncol - 1) as f64;
        let legend_height_px = row_px * rows_per_col as f64 + 2.0 * pad_v_px;

        let legend_width = legend_width_px * x_per_px;
        let legend_height = legend_height_px * y_per_px;

        // 已知固定位置直接定位；其余（含 "best" 与未识别值）自动避让数据。
        // 内边距：取数据范围的 2%，避免图例紧贴坐标轴边界
        let px = x_range * 0.02;
        let py = y_range * 0.02;
        let (box_x1, mut box_y1, box_x2, box_y2) = match loc.as_str() {
            "upper right" => box_from_anchor(
                HPos::Right,
                VPos::Top,
                x_max - px,
                y_max - py,
                legend_width,
                legend_height,
            ),
            "upper left" => box_from_anchor(
                HPos::Left,
                VPos::Top,
                x_min + px,
                y_max - py,
                legend_width,
                legend_height,
            ),
            "lower right" => box_from_anchor(
                HPos::Right,
                VPos::Bottom,
                x_max - px,
                y_min + py,
                legend_width,
                legend_height,
            ),
            "lower left" => box_from_anchor(
                HPos::Left,
                VPos::Bottom,
                x_min + px,
                y_min + py,
                legend_width,
                legend_height,
            ),
            "center" => box_from_anchor(
                HPos::Center,
                VPos::Center,
                (x_min + x_max) / 2.0,
                (y_min + y_max) / 2.0,
                legend_width,
                legend_height,
            ),
            "right" | "center right" => box_from_anchor(
                HPos::Right,
                VPos::Center,
                x_max - px,
                (y_min + y_max) / 2.0,
                legend_width,
                legend_height,
            ),
            "center left" => box_from_anchor(
                HPos::Left,
                VPos::Center,
                x_min + px,
                (y_min + y_max) / 2.0,
                legend_width,
                legend_height,
            ),
            "lower center" => box_from_anchor(
                HPos::Center,
                VPos::Bottom,
                (x_min + x_max) / 2.0,
                y_min + py,
                legend_width,
                legend_height,
            ),
            "upper center" => box_from_anchor(
                HPos::Center,
                VPos::Top,
                (x_min + x_max) / 2.0,
                y_max - py,
                legend_width,
                legend_height,
            ),
            _ => {
                let mut pts = collect_data_points(elements);
                // collect_data_points 采样的是原始数据值，而候选框坐标与 x_min..y_max
                // 均在（可能经 log10 变换的）显示坐标系中。log 刻度下需对采样点做同样变换，
                // 否则数据点落到错误位置，自动避让会把图例压在数据上。
                // 非正值（如柱状基线 0）在 log 下不可见，钳到对应轴的下界。
                if xlog || ylog {
                    for p in pts.iter_mut() {
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
                best_box(
                    &pts,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    legend_width,
                    legend_height,
                )
            }
        };

        let max_legend_height = y_max - y_min - 2.0 * py;
        let legend_height = entry_height * entry_count as f64 + 2.0 * pad_v_px * y_per_px;
        if legend_height > max_legend_height {
            box_y1 = box_y2 - legend_height;
        }

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

        for (i, (label, color, ls, marker_opt, lw, alpha)) in legend_labels.iter().enumerate() {
            let col = i / rows_per_col;
            let row = i % rows_per_col;

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
    }
    Ok(())
}
