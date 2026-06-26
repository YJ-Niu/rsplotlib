//! 数据元素渲染模块
//!
//! 处理所有 PlotElement 的绘制逻辑：线、散点、柱状图、填充、误差棒、饼图等。
//!
//! 主要 API：
//! - `render_elements()`: 遍历并渲染所有元素
//! - `draw_single_line()`: 绘制单条线段（用于 axhline/axvline/stem 等）

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use plotters::style::text_anchor::{HPos, Pos, VPos};

use crate::figure::axes::scale_font;
use crate::core::colormap::{autumn_color, cool_color, inferno_color, magma_color, plasma_color, spring_color, summer_color, viridis_color, winter_color};
use crate::core::colors::{RgbColor, default_color, parse_color, to_plotters_color, median};
use crate::core::elements::PlotElement;
use crate::core::marker::draw_marker;
use crate::core::text_utils::normalize_spaces;

/// 绘制单条线段（用于 axhline/axvline/stem 等）
pub fn draw_single_line<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    x1: f64, y1: f64, x2: f64, y2: f64,
    color: RgbColor, lw: f64, font_scale: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let rgb = to_plotters_color(color);
    // lw 是 points，转换为像素：lw * font_scale
    // plotters stroke_width(n) 实际渲染为 2n-1 像素，使用 stroke = max(1, width_px - 1) 接近 mpl
    let lw_px = (lw * font_scale).max(1.0).round() as u32;
    let stroke_w = (lw_px as i32 - 1).max(1) as u32;
    let style = rgb.stroke_width(stroke_w);
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(x1, y1), (x2, y2)], style,
    ))).map_err(|e| PyRuntimeError::new_err(format!("Line: {}", e)))?;
    Ok(())
}

/// 渲染所有 PlotElement
///
/// # 参数
/// - `chart`: plotters 的 chart 上下文
/// - `elements`: 所有 plot 调用收集的元素
/// - `font_scale`: 字体缩放系数
/// - `xlog`, `ylog`: 是否对数刻度
/// - `x_min`, `x_max`, `y_min`, `y_max`: 数据范围
pub fn render_elements<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    elements: &[PlotElement],
    font_scale: f64,
    xlog: bool,
    ylog: bool,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    // log 刻度坐标转换闭包
    let tx = |v: f64| if xlog { if v > 0.0 { v.log10() } else { f64::NEG_INFINITY } } else { v };
    let ty = |v: f64| if ylog { if v > 0.0 { v.log10() } else { f64::NEG_INFINITY } } else { v };

    for el in elements {
        match el {
            PlotElement::Line { x, y, color, linestyle, marker, linewidth, color_idx, solid_capstyle, markersize, .. } => {
                let col = parse_color(color, *color_idx).unwrap_or(default_color(*color_idx));
                let rgb = to_plotters_color(col);
                if !x.is_empty() && x.len() == y.len() {
                    // 构建连续有效数据段（被 None 分隔）
                    let mut segments: Vec<Vec<(f64, f64)>> = Vec::new();
                    {
                        let mut current: Vec<(f64, f64)> = Vec::new();
                        for (xv, yv) in x.iter().zip(y.iter()) {
                            if let (Some(xv), Some(yv)) = (xv, yv) {
                                let txv = tx(*xv);
                                let tyv = ty(*yv);
                                if txv.is_finite() && tyv.is_finite() {
                                    current.push((txv, tyv));
                                    continue;
                                }
                            }
                            // 遇到 None 或不合法值，结束当前段
                            if current.len() >= 2 {
                                segments.push(std::mem::take(&mut current));
                            } else {
                                current.clear();
                            }
                        }
                        if current.len() >= 2 {
                            segments.push(current);
                        }
                    }
                    if linestyle != " " {
                        for points in &segments {
                        // 将 linewidth 从 points 转换为像素：1pt = dpi/72 px，dpi = 72 * font_scale
                        // matplotlib 通过 AA 在 0.5-1.5pt 量级产生柔和的 1-3 像素宽线。
                        // plotters 无 AA，使用四舍五入以获得接近 mpl 的视觉粗细。
                        let lw_px = ((*linewidth) * font_scale).max(1.0).round() as u32;
                        let _style: ShapeStyle = rgb.stroke_width(lw_px);
                        // 对于虚线样式，使用分段绘制模拟
                        if linestyle == "--" {
                            let dash_len = *linewidth * 4.0;
                            let gap_len = *linewidth * 2.0;
                            let mut seg_start = 0usize;
                            while seg_start < points.len() - 1 {
                                let mut seg_end = seg_start + 1;
                                let mut acc_dist = 0.0;
                                while seg_end < points.len() {
                                    let dx = points[seg_end].0 - points[seg_end - 1].0;
                                    let dy = points[seg_end].1 - points[seg_end - 1].1;
                                    acc_dist += (dx * dx + dy * dy).sqrt();
                                    if acc_dist >= dash_len + gap_len { break; }
                                    seg_end += 1;
                                }
                                // 绘制dash段（前dash_len长度）
                                let mut dash_points = Vec::new();
                                dash_points.push(points[seg_start]);
                                let mut dist = 0.0;
                                for i in seg_start..seg_end.min(points.len() - 1) {
                                    let dx = points[i + 1].0 - points[i].0;
                                    let dy = points[i + 1].1 - points[i].1;
                                    let seg_len = (dx * dx + dy * dy).sqrt();
                                    if dist + seg_len <= dash_len {
                                        dash_points.push(points[i + 1]);
                                        dist += seg_len;
                                    } else {
                                        let remain = dash_len - dist;
                                        let t = remain / seg_len;
                                        dash_points.push((points[i].0 + dx * t, points[i].1 + dy * t));
                                        break;
                                    }
                                }
                                if dash_points.len() >= 2 {
                                    let lw_px_dash = ((*linewidth) * font_scale).max(1.0).round() as u32;
                                    let stroke_dash = (lw_px_dash as i32 - 1).max(1) as u32;
                                    chart.draw_series(std::iter::once(PathElement::new(dash_points, rgb.stroke_width(stroke_dash))))
                                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw dashed line: {}", e)))?;
                                }
                                seg_start = seg_end.max(seg_start + 1);
                            }
                        } else if linestyle == ":" {
                            // 点线：沿路径绘制短点段
                            let dot_len = *linewidth * 1.0;
                            let gap_len = *linewidth * 2.0;
                            let mut seg_idx = 0usize;
                            let mut pos_in_seg = 0.0f64;
                            while seg_idx < points.len() - 1 {
                                let dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                let dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                let seg_len = (dx * dx + dy * dy).sqrt();
                                if seg_len < 1e-10 {
                                    seg_idx += 1;
                                    pos_in_seg = 0.0;
                                    continue;
                                }
                                let unit_x = dx / seg_len;
                                let unit_y = dy / seg_len;
                                // 绘制一个点
                                let dot_start = pos_in_seg;
                                let dot_end = (pos_in_seg + dot_len).min(seg_len);
                                let p1 = (points[seg_idx].0 + unit_x * dot_start,
                                          points[seg_idx].1 + unit_y * dot_start);
                                let p2 = (points[seg_idx].0 + unit_x * dot_end,
                                          points[seg_idx].1 + unit_y * dot_end);
                                let lw_px_dot = ((*linewidth) * font_scale).max(1.0).round() as u32;
                                let stroke_dot = (lw_px_dot as i32 - 1).max(1) as u32;
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![p1, p2], rgb.stroke_width(stroke_dot))))
                                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw dotted line: {}", e)))?;
                                // 跳过间隙
                                let gap_end = dot_end + gap_len;
                                if gap_end < seg_len {
                                    pos_in_seg = gap_end;
                                } else {
                                    // 间隙跨越到下一段
                                    let mut remaining_gap = gap_end - seg_len;
                                    seg_idx += 1;
                                    pos_in_seg = 0.0;
                                    while seg_idx < points.len() - 1 && remaining_gap > 0.0 {
                                        let next_dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                        let next_dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                        let next_len = (next_dx * next_dx + next_dy * next_dy).sqrt();
                                        if remaining_gap < next_len {
                                            pos_in_seg = remaining_gap;
                                            remaining_gap = 0.0;
                                        } else {
                                            remaining_gap -= next_len;
                                            seg_idx += 1;
                                            pos_in_seg = 0.0;
                                        }
                                    }
                                }
                            }
                        } else if linestyle == "-." {
                            // 点划线：交替绘制长划和短点
                            let dash_len = *linewidth * 6.0;
                            let dot_len = *linewidth * 1.0;
                            let gap_len = *linewidth * 2.0;
                            let mut seg_idx = 0usize;
                            let mut pos_in_seg = 0.0f64;
                            let mut is_dash = true; // 交替 dash/dot
                            while seg_idx < points.len() - 1 {
                                let dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                let dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                let seg_len = (dx * dx + dy * dy).sqrt();
                                if seg_len < 1e-10 {
                                    seg_idx += 1;
                                    pos_in_seg = 0.0;
                                    continue;
                                }
                                let unit_x = dx / seg_len;
                                let unit_y = dy / seg_len;
                                let mark_len = if is_dash { dash_len } else { dot_len };
                                let mark_start = pos_in_seg;
                                let mark_end = (pos_in_seg + mark_len).min(seg_len);
                                let p1 = (points[seg_idx].0 + unit_x * mark_start,
                                          points[seg_idx].1 + unit_y * mark_start);
                                let p2 = (points[seg_idx].0 + unit_x * mark_end,
                                          points[seg_idx].1 + unit_y * mark_end);
                                let lw_px_mix = ((*linewidth) * font_scale).max(1.0).round() as u32;
                                let stroke_mix = (lw_px_mix as i32 - 1).max(1) as u32;
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![p1, p2], rgb.stroke_width(stroke_mix))))
                                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw dash-dot line: {}", e)))?;
                                // 跳过间隙
                                let gap_end = mark_end + gap_len;
                                is_dash = !is_dash;
                                if gap_end < seg_len {
                                    pos_in_seg = gap_end;
                                } else {
                                    let mut remaining_gap = gap_end - seg_len;
                                    seg_idx += 1;
                                    pos_in_seg = 0.0;
                                    while seg_idx < points.len() - 1 && remaining_gap > 0.0 {
                                        let next_dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                        let next_dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                        let next_len = (next_dx * next_dx + next_dy * next_dy).sqrt();
                                        if remaining_gap < next_len {
                                            pos_in_seg = remaining_gap;
                                            remaining_gap = 0.0;
                                        } else {
                                            remaining_gap -= next_len;
                                            seg_idx += 1;
                                            pos_in_seg = 0.0;
                                        }
                                    }
                                }
                            }
                        } else {
                            // 实线：使用 plotters 原生 stroke_width
                            // plotters BitMapBackend stroke_width(n) 实际渲染近似 n-1 像素（带 AA 边框）。
                            // matplotlib 通过 AA 在 1pt 量级产生 2-3 像素宽线。
                            // 公式: stroke = max(1, lw_px - 1) 是最接近 mpl 的折中：
                            //   lw=0.5: lw_px=1, stroke=1 → 1px (mpl 1px) ✓
                            //   lw=1.0: lw_px=2, stroke=1 → 1px (mpl 3px) - 略薄
                            //   lw=1.5: lw_px=3, stroke=2 → 3-5px (mpl 4px) ✓
                            //   lw=2.0: lw_px=4, stroke=3 → 5-7px (mpl 5px) ✓
                            //   lw=3.0: lw_px=6, stroke=5 → 9-11px (mpl 8px) ✓
                            let stroke_w = (lw_px as i32).max(1) as u32;
                            let style_native: ShapeStyle = rgb.stroke_width(stroke_w);
                            // 像素中心对齐修正：plotters 在渲染水平线时，线中心对应像素下边缘，
                            // 而 matplotlib 使用像素中心。这导致 rsp 的水平线比 mpl 偏高 1 像素。
                            // 修正方法：将所有 y 坐标向下偏移半像素（half a pixel）。
                            let area = chart.plotting_area();
                            let dim = area.dim_in_pixel();
                            let ph = dim.1 as f64;
                            if ph > 0.0 {
                                let y_per_pix = (y_max - y_min) / ph;
                                let y_shift = y_per_pix * 0.5;
                                let shifted_points: Vec<(f64, f64)> = points.iter()
                                    .map(|(px, py)| (*px, *py - y_shift))
                                    .collect();
                                chart.draw_series(std::iter::once(PathElement::new(shifted_points, style_native)))
                                    .map_err(|e| PyRuntimeError::new_err(format!("Native line: {}", e)))?;
                            } else {
                                chart.draw_series(std::iter::once(PathElement::new(points.clone(), style_native)))
                                    .map_err(|e| PyRuntimeError::new_err(format!("Native line: {}", e)))?;
                            }
                            // 端点形状 (solid_capstyle)：
                            // plotters 0.3.7 原生不支持 linecap，因此手动在首末端绘制
                            // 装饰图形。cap 只与有线段（marker 为空）的实线情况相关，
                            // 虚线/点线场景下端点不连续，跳过以免破坏节奏。
                            //
                            // 重要：cap 端点必须与已绘制的线段几何对齐：
                            //   1) 实际渲染线宽 = stroke_w - 1 像素，cap 直径等于这个值
                            //   2) 实线已经过 0.5 像素 y 中心对齐，cap 也要应用同样的 y_shift
                            //   3) cap 圆心 = 线段端点（与 shift 后的位置一致）
                            if linestyle == "-" && marker.as_ref().is_none_or(|m| m.is_empty()) {
                                // plotters BitMapBackend 的 stroke_width(w) 实际渲染为 2w-1 像素
                                // （中间 w-1 像素实色 + 两侧各 0.5 像素 AA），所以端点圆盘的
                                // 直径必须与线段**实际渲染宽度**严格相等，才能"完全契合"。
                                // 端点圆心 = 像素中心对齐后的线段端点（与线段几何一致）。
                                let cap_lw_px = (stroke_w as i32).max(1) as f64; // 与线段渲染宽度一致
                                // **关键：cap_y_shift 必须与上面的 y_shift 严格相同**，
                                // 这样 cap 圆心和线段端点经过 plotters coord.translate 时
                                // 走完全相同的截断路径，得到的 (i32, i32) 屏幕像素完全一致。
                                // 之前的 +0.2 是临时 hack，会让 cap 偏移 0.2/y_per_pix 像素。
                                let cap_y_shift = {
                                    let area2 = chart.plotting_area();
                                    let dim2 = area2.dim_in_pixel();
                                    let ph2 = dim2.1 as f64;
                                    if ph2 > 0.0 { (y_max - y_min) / ph2 * 0.5 } else { 0.0 }
                                };
                                draw_solid_caps(chart, points, &rgb, solid_capstyle, *linewidth, font_scale,
                                                x_min, x_max, y_min, y_max, cap_lw_px, cap_y_shift)?;
                            }
                        }
                        }
                    }
                }
                if let Some(marker_name) = marker
                    && !marker_name.is_empty() && x.len() == y.len()
                {
                    let col2 = parse_color(color, *color_idx).unwrap_or(default_color(*color_idx));
                    let rgb = to_plotters_color(col2);
                        // matplotlib markersize 单位是 points（近似直径）；直径(像素) = markersize * dpi/72
                        // 在 144dpi 下，markersize=6 (mpl 默认) 直径约为 12 像素。
                        // "." 是 matplotlib 的 1 像素点 marker，需要保持极小以免覆盖线条
                        let marker_size = if marker_name == "." {
                            // "." 1pt 像素点：线宽 <=1pt 时取 1 像素，否则 2 像素（保持可见）
                            if *linewidth <= 1.0 { 1.0 } else { 2.0 }
                        } else if marker_name == "," {
                            // "," 1/2 像素点：保持 1 像素
                            1.0
                        } else {
                            // markersize: None => matplotlib 默认 6
                            let ms = markersize.unwrap_or(6.0_f64).max(0.01);
                            // 直径(像素) ≈ markersize * font_scale
                            // plotters Circle::new 的半径转 i32 会截断，因此 6*2=12 → radius=6
                            // 渲染直径 = 2*6+1 = 13px（与 mpl ~13px 接近）
                            let diameter_px = ms * font_scale;
                            // draw_marker 中半径 = s
                            diameter_px / 2.0
                        };
                        for (xv, yv) in x.iter().zip(y.iter()) {
                            if let (Some(xv), Some(yv)) = (xv, yv) {
                                let txv = tx(*xv);
                                let tyv = ty(*yv);
                                if txv.is_finite() && tyv.is_finite() {
                                    draw_marker(chart, marker_name, txv, tyv, marker_size, rgb)
                                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw marker: {}", e)))?;
                                }
                            }
                        }
                }
            }
            PlotElement::Scatter { x, y, s, c, marker, color_idx, .. } => {
                let col = parse_color(c, *color_idx).unwrap_or(default_color(*color_idx));
                let rgb = to_plotters_color(col);
                let size = s.sqrt() * 0.4;
                for (&xv, &yv) in x.iter().zip(y.iter()) {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if txv.is_finite() && tyv.is_finite() {
                        draw_marker(chart, marker, txv, tyv, size.max(2.0), rgb)
                            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw scatter: {}", e)))?;
                    }
                }
            }
            PlotElement::ScatterMulti { x, y, s_list, c_list, marker, color_idx, .. } => {
                if x.is_empty() || y.is_empty() { continue; }
                for (i, (&xv, &yv)) in x.iter().zip(y.iter()).enumerate() {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if !txv.is_finite() || !tyv.is_finite() { continue; }
                    let c_str = c_list.as_ref().and_then(|c| c.get(i).cloned()).unwrap_or_default();
                    let col = if c_str.is_empty() {
                        parse_color("", *color_idx).unwrap_or(default_color(*color_idx + i))
                    } else {
                        parse_color(&c_str, *color_idx + i).unwrap_or(default_color(*color_idx + i))
                    };
                    let rgb = to_plotters_color(col);
                    let size = s_list.as_ref()
                        .and_then(|s| s.get(i).cloned())
                        .unwrap_or(20.0)
                        .sqrt() * 0.4;
                    draw_marker(chart, marker, txv, tyv, size.max(2.0), rgb)
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw scatter_multi: {}", e)))?;
                }
            }
            PlotElement::Bar { x, height, width, color, color_idx, .. } => {
                let col = parse_color(color, *color_idx).unwrap_or(default_color(*color_idx));
                let rgb = to_plotters_color(col);
                let fill_style: ShapeStyle = rgb.filled();
                for (&xv, &h) in x.iter().zip(height.iter()) {
                    let txv = tx(xv);
                    let th = ty(h);
                    let y0 = if ylog { f64::NEG_INFINITY } else { 0.0f64.max(y_min) };
                    if txv.is_finite() && th.is_finite() {
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(txv - width / 2.0, y0), (txv + width / 2.0, th)],
                            fill_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw bar: {}", e)))?;
                    }
                }
            }
            PlotElement::BarH { y, width, height, color, color_idx, .. } => {
                let c = if color.is_empty() { default_color(*color_idx) } else { parse_color(color, *color_idx)? };
                let rgb = to_plotters_color(c);
                let fill_style: ShapeStyle = rgb.filled();
                for (&yv, &wv) in y.iter().zip(width.iter()) {
                    let tyv = ty(yv);
                    let twv = tx(wv);
                    let bar_y0 = tyv - height / 2.0;
                    let bar_y1 = tyv + height / 2.0;
                    chart.draw_series(std::iter::once(Rectangle::new(
                        [(0.0, bar_y0), (twv, bar_y1)],
                        fill_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw barh: {}", e)))?;
                }
            }
            PlotElement::Hist { data_all, bins: num_bins_user, density, histtype, alpha, colors, color_idx, bin_edges, label: _ } => {
                if data_all.is_empty() { continue; }
                let all_data: Vec<f64> = data_all.iter().flatten().cloned().collect();
                if all_data.is_empty() { continue; }
                let (global_min, global_max) = if let Some(edges) = bin_edges {
                    (edges[0], edges[edges.len() - 1])
                } else {
                    let mn = all_data.iter().cloned().fold(f64::INFINITY, f64::min);
                    let mx = all_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    (mn, mx)
                };
                let global_range = global_max - global_min;
                let bin_edges_list: Vec<f64> = if global_range < 1e-10 {
                    // 所有值相同 -> 在值两侧各扩 0.5 形成一个单柱
                    vec![global_min - 0.5, global_min + 0.5]
                } else if let Some(edges) = bin_edges {
                    edges.clone()
                } else {
                    let bw = global_range / *num_bins_user as f64;
                    (0..=*num_bins_user).map(|i| global_min + i as f64 * bw).collect()
                };
                let effective_bins = bin_edges_list.len() - 1;
                let total_all = all_data.len() as f64;
                for (di, dataset) in data_all.iter().enumerate() {
                    if dataset.is_empty() { continue; }
                    let col_str = colors.get(di).map(|s| s.as_str()).unwrap_or("");
                    let col = parse_color(col_str, *color_idx + di).unwrap_or(default_color(*color_idx + di));
                    let rgb = to_plotters_color(col);
                    let fill_style: ShapeStyle = rgb.mix(*alpha).filled();
                    let outline_style: ShapeStyle = rgb.mix(*alpha).stroke_width(1);
                    let mut counts = vec![0usize; effective_bins];
                    for &val in dataset {
                        if val < global_min || val > global_max { continue; }
                        let bin = bin_edges_list.partition_point(|&e| e <= val) - 1;
                        if bin < effective_bins {
                            counts[bin] += 1;
                        }
                    }
                    for (i, &count) in counts.iter().enumerate() {
                        let bin_left = bin_edges_list[i];
                        let bin_right = bin_edges_list[i + 1];
                        let h = if *density { count as f64 / (total_all * (bin_right - bin_left)) } else { count as f64 };
                        if h <= 0.0 { continue; }
                        if histtype == "stepfilled" {
                            chart.draw_series(std::iter::once(Rectangle::new(
                                [(bin_left, 0.0), (bin_right, h)],
                                fill_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw hist fill: {}", e)))?;
                            chart.draw_series(std::iter::once(Rectangle::new(
                                [(bin_left, 0.0), (bin_right, h)],
                                outline_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw hist outline: {}", e)))?;
                        } else {
                            chart.draw_series(std::iter::once(Rectangle::new(
                                [(bin_left, 0.0), (bin_right, h)],
                                fill_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw hist: {}", e)))?;
                        }
                    }
                }
            }
            PlotElement::Image { data, cmap } => {
                if data.is_empty() || data[0].is_empty() { continue; }
                let d_min = data.iter().flatten().cloned().fold(f64::INFINITY, f64::min);
                let d_max = data.iter().flatten().cloned().fold(f64::NEG_INFINITY, f64::max);
                let d_range = if (d_max - d_min).abs() < 1e-10 { 1.0 } else { d_max - d_min };
                for (r, row) in data.iter().enumerate() {
                    for (c, &val) in row.iter().enumerate() {
                        let normalized = (val - d_min) / d_range;
                        let rgb = match cmap.as_str() {
                            "gray" | "grey" => { let v = (normalized * 255.0) as u8; RGBColor(v, v, v) }
                            "hot" => {
                                let r = (normalized * 3.0).clamp(0.0, 1.0);
                                let g = (normalized * 3.0 - 1.0).clamp(0.0, 1.0);
                                let b = (normalized * 3.0 - 2.0).clamp(0.0, 1.0);
                                RGBColor((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
                            }
                            "plasma" => plasma_color(normalized),
                            "inferno" => inferno_color(normalized),
                            "magma" => magma_color(normalized),
                            "cool" => cool_color(normalized),
                            "spring" => spring_color(normalized),
                            "summer" => summer_color(normalized),
                            "autumn" => autumn_color(normalized),
                            "winter" => winter_color(normalized),
                            _ => viridis_color(normalized),
                        };
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(c as f64, r as f64), ((c + 1) as f64, (r + 1) as f64)],
                            rgb.filled(),
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw image: {}", e)))?;
                    }
                }
            }
            PlotElement::Text { x, y, text, fontsize, color, font_family } => {
                let txv = tx(*x);
                let tyv = ty(*y);
                if !txv.is_finite() || !tyv.is_finite() { continue; }
                let fs = scale_font(*fontsize as f64, font_scale);
                let family_name = font_family.as_deref().unwrap_or("sans-serif");
                let font: FontDesc = (family_name, fs).into();
                let colored_font = font.color(&to_plotters_color(*color));
                // 垂直居中对齐：让 (x, y) 落在文字的几何中心，
                // 与 axhline/axvline 在同一坐标时的视觉位置一致。
                let text_style = colored_font
                    .pos(Pos::new(HPos::Left, VPos::Center));
                let normalized = normalize_spaces(text);
                chart.draw_series(std::iter::once(plotters::element::Text::new(
                    normalized,
                    (txv, tyv),
                    text_style,
                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw text: {}", e)))?;
            }
            PlotElement::HLine { y, color, linewidth, color_idx, .. } => {
                let tyv = ty(*y);
                if !tyv.is_finite() { continue; }
                let col = parse_color(color, *color_idx).unwrap_or(RgbColor(0, 0, 0));
                draw_single_line(chart, x_min, tyv, x_max, tyv, col, *linewidth, font_scale)?;
            }
            PlotElement::VLine { x, color, linewidth, color_idx, .. } => {
                let txv = tx(*x);
                if !txv.is_finite() { continue; }
                let col = parse_color(color, *color_idx).unwrap_or(RgbColor(0, 0, 0));
                draw_single_line(chart, txv, y_min, txv, y_max, col, *linewidth, font_scale)?;
            }
            PlotElement::HSpan { y1, y2, color, alpha } => {
                let ty1 = ty(*y1);
                let ty2 = ty(*y2);
                if !ty1.is_finite() || !ty2.is_finite() { continue; }
                let col = parse_color(color, 0).unwrap_or(RgbColor(128, 128, 128));
                let rgb = to_plotters_color(col);
                let top = ty1.max(ty2);
                let bottom = ty1.min(ty2);
                chart.draw_series(std::iter::once(Rectangle::new(
                    [(x_min, bottom), (x_max, top)],
                    rgb.mix(*alpha).filled(),
                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw axhspan: {}", e)))?;
            }
            PlotElement::VSpan { x1, x2, color, alpha } => {
                let tx1 = tx(*x1);
                let tx2 = tx(*x2);
                if !tx1.is_finite() || !tx2.is_finite() { continue; }
                let col = parse_color(color, 0).unwrap_or(RgbColor(128, 128, 128));
                let rgb = to_plotters_color(col);
                let left = tx1.min(tx2);
                let right = tx1.max(tx2);
                chart.draw_series(std::iter::once(Rectangle::new(
                    [(left, y_min), (right, y_max)],
                    rgb.mix(*alpha).filled(),
                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw axvspan: {}", e)))?;
            }
            PlotElement::AxLine { xy1, xy2, color, linestyle, linewidth } => {
                let col = parse_color(color, 0).unwrap_or(RgbColor(0, 0, 0));
                let tx1 = tx(xy1.0);
                let ty1 = ty(xy1.1);
                let tx2 = tx(xy2.0);
                let ty2 = ty(xy2.1);
                if tx1.is_finite() && ty1.is_finite() && tx2.is_finite() && ty2.is_finite() {
                    draw_single_line(chart, tx1, ty1, tx2, ty2, col, *linewidth, font_scale)?;
                    let _ = linestyle;
                }
            }
            PlotElement::Arrow { x1, y1, x2, y2, color, linewidth, head_size } => {
                let col = parse_color(color, 0).unwrap_or(RgbColor(0, 0, 0));
                let tx1 = tx(*x1);
                let ty1 = ty(*y1);
                let tx2 = tx(*x2);
                let ty2 = ty(*y2);
                if !(tx1.is_finite() && ty1.is_finite() && tx2.is_finite() && ty2.is_finite()) { continue; }
                draw_single_line(chart, tx1, ty1, tx2, ty2, col, *linewidth, font_scale)?;
                // 箭头头部：简单三角形
                let dx = tx2 - tx1;
                let dy = ty2 - ty1;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-10 { continue; }
                let nx = dx / len;
                let ny = dy / len;
                let head = *head_size;
                let p1 = (tx2, ty2);
                let p2 = (tx2 - head * nx - head * 0.5 * ny, ty2 - head * ny + head * 0.5 * nx);
                let p3 = (tx2 - head * nx + head * 0.5 * ny, ty2 - head * ny - head * 0.5 * nx);
                let rgb = to_plotters_color(col);
                chart.draw_series(std::iter::once(Polygon::new(
                    vec![p1, p2, p3],
                    rgb.filled(),
                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw arrow: {}", e)))?;
            }
            PlotElement::Pie { x, labels, colors, autopct, startangle } => {
                let total: f64 = x.iter().sum();
                if total <= 0.0 { continue; }
                let mut current_angle = startangle.to_radians();
                let pie_colors = colors.as_ref().map(|c| c.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                for (i, &val) in x.iter().enumerate() {
                    if val <= 0.0 { continue; }
                    let angle = (val / total) * 360.0_f64;
                    let angle_rad = angle.to_radians();
                    let end_angle = current_angle + angle_rad;
                    let col = if let Some(ref pc) = pie_colors {
                        let ci = parse_color(pc.get(i).unwrap_or(&""), i).unwrap_or(default_color(i));
                        to_plotters_color(ci)
                    } else {
                        to_plotters_color(default_color(i))
                    };
                    let steps = ((angle_rad / 0.05).ceil() as usize).max(3);
                    let mut vertices = vec![(0.0, 0.0)];
                    for j in 0..=steps {
                        let a = current_angle + (j as f64 / steps as f64) * angle_rad;
                        vertices.push((a.cos(), a.sin()));
                    }
                    chart.draw_series(std::iter::once(Polygon::new(
                        vertices, col.mix(0.85).filled(),
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw pie: {}", e)))?;
                    let mid_angle = current_angle + angle_rad / 2.0;
                    if let Some(lbls) = labels
                        && let Some(l) = lbls.get(i)
                    {
                        let label_r = 1.3;
                        let lx = mid_angle.cos() * label_r;
                        let ly = mid_angle.sin() * label_r;
                        // 使用 BLACK 让 font.color() 返回 TextStyle，再 .pos() 调整锚点
                        let pie_label_style: TextStyle = FontDesc::from(("sans-serif", scale_font(12.0, font_scale)))
                            .color(&BLACK)
                            .pos(Pos::new(HPos::Center, VPos::Center));
                        chart.draw_series(std::iter::once(plotters::element::Text::new(
                            normalize_spaces(l), (lx, ly), pie_label_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw pie label: {}", e)))?;
                    }
                    if let Some(fmt) = autopct {
                        let pct = val / total * 100.0;
                        let text = if fmt == "%1.1f%%" || fmt.contains("%%") {
                            format!("{:.1}%", pct)
                        } else if fmt == "%d%%" {
                            format!("{}%", pct as i32)
                        } else {
                            format!("{:.1}%", pct)
                        };
                        let text_r = 0.7;
                        let tx = mid_angle.cos() * text_r;
                        let ty = mid_angle.sin() * text_r;
                        let autopct_style: TextStyle = FontDesc::from(("sans-serif", scale_font(11.0, font_scale)))
                            .color(&BLACK)
                            .pos(Pos::new(HPos::Center, VPos::Center));
                        chart.draw_series(std::iter::once(plotters::element::Text::new(
                            text, (tx, ty), autopct_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw autopct: {}", e)))?;
                    }
                    current_angle = end_angle;
                }
            }
            PlotElement::FillBetween { x, y1, y2, color, alpha, .. } => {
                let col = parse_color(color, 0).unwrap_or(RgbColor(0, 128, 0));
                let rgb = to_plotters_color(col);
                if x.len() != y1.len() || x.is_empty() { continue; }
                let mut points: Vec<(f64, f64)> = Vec::with_capacity(x.len() * 2);
                for (&xv, &yv) in x.iter().zip(y1.iter()) {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                }
                for i in (0..x.len()).rev() {
                    let y2v = if i < y2.len() { y2[i] } else { 0.0 };
                    let txv = tx(x[i]);
                    let tyv = ty(y2v);
                    if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                }
                if points.len() < 3 { continue; }
                chart.draw_series(std::iter::once(Polygon::new(
                    points, rgb.mix(*alpha).filled(),
                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw fill_between: {}", e)))?;
            }
            PlotElement::Stack { x, y_series, colors, alpha, .. } => {
                if x.is_empty() || y_series.is_empty() { continue; }
                // 计算累加值：从最底层开始绘制
                let mut cumulative: Vec<f64> = vec![0.0; x.len()];
                for (si, series) in y_series.iter().enumerate() {
                    let color_str = colors.as_ref()
                        .and_then(|c| c.get(si).cloned())
                        .unwrap_or_default();
                    let col = parse_color(&color_str, 0)
                        .unwrap_or(default_color(si));
                    let rgb = to_plotters_color(col);
                    // 构造当前层的上下边界点
                    let mut points: Vec<(f64, f64)> = Vec::with_capacity(x.len() * 2);
                    for (i, &xv) in x.iter().enumerate() {
                        let upper = if i < series.len() { cumulative[i] + series[i] } else { cumulative[i] };
                        let txv = tx(xv);
                        let tyv = ty(upper);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                    }
                    for i in (0..x.len()).rev() {
                        let txv = tx(x[i]);
                        let tyv = ty(cumulative[i]);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                    }
                    // 累加
                    for (i, v) in series.iter().enumerate() {
                        if i < cumulative.len() { cumulative[i] += v; }
                    }
                    if points.len() < 3 { continue; }
                    chart.draw_series(std::iter::once(Polygon::new(
                        points, rgb.mix(*alpha).filled(),
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw stack: {}", e)))?;
                }
            }
            PlotElement::ErrorBar { x, y, yerr, xerr, fmt, color, capsize, .. } => {
                let idx = 0;
                let col = parse_color(color, idx).unwrap_or(default_color(idx));
                let rgb = to_plotters_color(col);
                let line_style: ShapeStyle = rgb.stroke_width(1);
                let cap_half = capsize / 2.0;
                for (i, (&xv, &yv)) in x.iter().zip(y.iter()).enumerate() {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if !txv.is_finite() || !tyv.is_finite() { continue; }
                    let ye = if let Some(vec) = yerr.as_ref() { if i < vec.len() { vec[i] } else { 0.0_f64 } } else { 0.0 };
                    let xe = if let Some(vec) = xerr.as_ref() { if i < vec.len() { vec[i] } else { 0.0_f64 } } else { 0.0 };
                    if ye != 0.0 {
                        let ty_lo = ty(yv - ye);
                        let ty_hi = ty(yv + ye);
                        if ty_lo.is_finite() && ty_hi.is_finite() {
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(txv, ty_lo), (txv, ty_hi)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar line: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(txv - cap_half, ty_lo), (txv + cap_half, ty_lo)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar cap: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(txv - cap_half, ty_hi), (txv + cap_half, ty_hi)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar cap: {}", e)))?;
                        }
                    }
                    if xe != 0.0 {
                        let tx_lo = tx(xv - xe);
                        let tx_hi = tx(xv + xe);
                        if tx_lo.is_finite() && tx_hi.is_finite() {
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(tx_lo, tyv), (tx_hi, tyv)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xline: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(tx_lo, tyv - cap_half), (tx_lo, tyv + cap_half)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(tx_hi, tyv - cap_half), (tx_hi, tyv + cap_half)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e)))?;
                        }
                    }
                    if !fmt.is_empty() {
                        let marker_name = fmt;
                        draw_marker(chart, marker_name, txv, tyv, 3.0, rgb)
                            .map_err(|e| PyRuntimeError::new_err(format!("ErrorBar marker: {}", e)))?;
                    }
                }
            }
            PlotElement::Stem { x, y, linefmt, markerfmt, .. } => {
                let col = RgbColor(0, 0, 200);
                let rgb = to_plotters_color(col);
                let baseline = ty(0.0);
                if linefmt == "-" || linefmt.is_empty() {
                    let lw_px = (1.0 * font_scale).round().max(1.0) as u32;
                    let line_style: ShapeStyle = rgb.stroke_width(lw_px);
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if !txv.is_finite() || !tyv.is_finite() || !baseline.is_finite() { continue; }
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(txv, baseline), (txv, tyv)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Stem line: {}", e)))?;
                    }
                } else {
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if !txv.is_finite() || !tyv.is_finite() || !baseline.is_finite() { continue; }
                        draw_single_line(chart, txv, baseline, txv, tyv, col, 1.0, font_scale)?;
                    }
                }
                for (&xv, &yv) in x.iter().zip(y.iter()) {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if !txv.is_finite() || !tyv.is_finite() { continue; }
                    draw_marker(chart, markerfmt, txv, tyv, 5.0, rgb)
                        .map_err(|e| PyRuntimeError::new_err(format!("Stem marker: {}", e)))?;
                }
            }
            PlotElement::Step { x, y, where_, color, linestyle: _, linewidth, .. } => {
                let idx = 0;
                let col = parse_color(color, idx).unwrap_or(default_color(idx));
                if x.len() < 2 || x.len() != y.len() { continue; }
                let mut points = Vec::new();
                match where_.as_str() {
                    "pre" => {
                        let txv = tx(x[0]);
                        let tyv = ty(y[0]);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                        for i in 1..x.len() {
                            let txv = tx(x[i]);
                            let tyv_prev = ty(y[i - 1]);
                            let tyv = ty(y[i]);
                            if txv.is_finite() && tyv_prev.is_finite() { points.push((txv, tyv_prev)); }
                            if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                        }
                    }
                    "post" => {
                        for i in 0..x.len() - 1 {
                            let txv = tx(x[i]);
                            let tyv = ty(y[i]);
                            let tyv_next = ty(y[i + 1]);
                            if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                            if txv.is_finite() && tyv_next.is_finite() { points.push((txv, tyv_next)); }
                        }
                        let txv = tx(x[x.len() - 1]);
                        let tyv = ty(y[y.len() - 1]);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                    }
                    _ => {
                        let txv = tx(x[0]);
                        let tyv = ty(y[0]);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                        for i in 1..x.len() {
                            let mid = (x[i - 1] + x[i]) / 2.0;
                            let tmid = tx(mid);
                            let tyv_prev = ty(y[i - 1]);
                            let tyv = ty(y[i]);
                            if tmid.is_finite() && tyv_prev.is_finite() { points.push((tmid, tyv_prev)); }
                            if tmid.is_finite() && tyv.is_finite() { points.push((tmid, tyv)); }
                        }
                        let txv = tx(x[x.len() - 1]);
                        let tyv = ty(y[y.len() - 1]);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                    }
                }
                if points.len() < 2 { continue; }
                let lw_px = ((*linewidth) * font_scale).round().max(1.0) as u32;
                let style: ShapeStyle = to_plotters_color(col).stroke_width(lw_px);
                chart.draw_series(LineSeries::new(points, style))
                    .map_err(|e| PyRuntimeError::new_err(format!("Step draw: {}", e)))?;
            }
            PlotElement::BoxPlot { data, labels, .. } => {
                let box_width = 0.6;
                for (i, series) in data.iter().enumerate() {
                    if series.is_empty() { continue; }
                    let mut sorted = series.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let n = sorted.len();
                    let min_val = sorted[0];
                    let max_val = sorted[n - 1];
                    let q1 = if n % 2 == 0 {
                        let mid = n / 2;
                        median(&sorted[0..mid])
                    } else {
                        median(&sorted[0..n / 2])
                    };
                    let q3 = if n % 2 == 0 {
                        let mid = n / 2;
                        median(&sorted[mid..])
                    } else {
                        median(&sorted[n / 2 + 1..])
                    };
                    let med = median(&sorted);
                    let iqr = q3 - q1;
                    let lower_whisker = (min_val).max(q1 - 1.5 * iqr);
                    let upper_whisker = (max_val).min(q3 + 1.5 * iqr);
                    let tq1 = ty(q1);
                    let tq3 = ty(q3);
                    let tmed = ty(med);
                    let tlower = ty(lower_whisker);
                    let tupper = ty(upper_whisker);
                    if !tq1.is_finite() || !tq3.is_finite() || !tmed.is_finite() || !tlower.is_finite() || !tupper.is_finite() { continue; }
                    let cx = (i + 1) as f64;
                    let col = to_plotters_color(default_color(i));
                    let fill_style: ShapeStyle = col.mix(0.3).filled();
                    let line_style: ShapeStyle = col.stroke_width(2);
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(cx, tlower), (cx, tupper)], line_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot whisker: {}", e)))?;
                    chart.draw_series(std::iter::once(Rectangle::new(
                        [(cx - box_width / 2.0, tq1), (cx + box_width / 2.0, tq3)], fill_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot box: {}", e)))?;
                    chart.draw_series(std::iter::once(Rectangle::new(
                        [(cx - box_width / 2.0, tq1), (cx + box_width / 2.0, tq3)], line_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot border: {}", e)))?;
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(cx - box_width / 2.0, tmed), (cx + box_width / 2.0, tmed)],
                        col.stroke_width(2).filled(),
                    ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot median: {}", e)))?;
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(cx - box_width / 4.0, tlower), (cx + box_width / 4.0, tlower)], line_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(cx - box_width / 4.0, tupper), (cx + box_width / 4.0, tupper)], line_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                    if let Some(lbls) = labels
                        && let Some(l) = lbls.get(i)
                    {
                        let box_label_style: TextStyle = FontDesc::from(("sans-serif", scale_font(11.0, font_scale)))
                            .color(&BLACK)
                            .pos(Pos::new(HPos::Center, VPos::Center));
                        chart.draw_series(std::iter::once(plotters::element::Text::new(
                            normalize_spaces(l), (cx, -0.3), box_label_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot label: {}", e)))?;
                    }
                }
            }
            PlotElement::Annotate { text, xy, xytext, fontsize, color } => {
                let col = parse_color(color, 0).unwrap_or(RgbColor(0, 0, 0));
                let rgb = to_plotters_color(col);
                let (txy_x, txy_y) = xytext.unwrap_or((xy.0, xy.1));
                let txy_x = tx(txy_x);
                let txy_y = ty(txy_y);
                let txy_xy_x = tx(xy.0);
                let txy_xy_y = ty(xy.1);
                if !txy_x.is_finite() || !txy_y.is_finite() || !txy_xy_x.is_finite() || !txy_xy_y.is_finite() { continue; }
                let arrow_style: ShapeStyle = rgb.stroke_width(1);
                chart.draw_series(std::iter::once(PathElement::new(
                    vec![(txy_x, txy_y), (txy_xy_x, txy_xy_y)], arrow_style,
                ))).map_err(|e| PyRuntimeError::new_err(format!("Annotate arrow: {}", e)))?;
                let anno_style: TextStyle = FontDesc::from(("sans-serif", scale_font(*fontsize, font_scale)))
                    .color(&rgb)
                    .pos(Pos::new(HPos::Center, VPos::Center));
                chart.draw_series(std::iter::once(plotters::element::Text::new(
                    normalize_spaces(text), (txy_x, txy_y), anno_style,
                ))).map_err(|e| PyRuntimeError::new_err(format!("Annotate text: {}", e)))?;
            }
        }
    }
    Ok(())
}

/// 在 Line 段的首末两端按 `solid_capstyle` 绘制端点装饰。
///
/// plotters 0.3.7 原生不支持 `stroke-linecap`，因此手动模拟：
///
/// - `butt` (matplotlib 默认)：不绘制任何装饰，线条在端点处被垂直切断。
/// - `round`：在两端各画一个**半圆**（沿切线方向凸出），用 SVG 路径精确构造，
///   几何上严格等于"直径 = 线宽的半圆"，与 matplotlib 的
///   `stroke-linecap="round"` 视觉一致。比直接画一个填充圆更精确：
///   1) 圆心位于线段端点（已应用 y_shift），与线段几何完全重合
///   2) 直径严格等于 `cap_lw_px`（= 实际线宽），不会出现"比线粗 1 像素"
///   3) 圆弧是真正的 Bezier 曲线，光滑无折线
/// - `projecting`：在两端各画一个**填充矩形**，沿线的切线方向延伸出
///   `cap_lw_px/2` 像素的方形凸出部分。
///
/// 关键几何参数：
/// - `cap_lw_px`：实际线宽（像素），由调用方传入以保证与线段 stroke_w 一致
/// - `cap_y_shift`：线段 y 中心对齐偏移，cap 应用同样的偏移以保持端点对齐
///
/// 仅在实线 (`-`) 且无 marker 时调用；虚线/点线场景下端点不连续，跳过。
fn draw_solid_caps<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    points: &[(f64, f64)],
    rgb: &plotters::style::RGBColor,
    capstyle: &str,
    _linewidth: f64,
    _font_scale: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    cap_lw_px: f64,
    cap_y_shift: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if capstyle.eq_ignore_ascii_case("butt") || capstyle.is_empty() {
        // 默认：plotters 自身行为已是 butt（端点垂直切断），不需要额外绘制
        return Ok(());
    }
    if points.len() < 2 {
        return Ok(());
    }
    // 应用 y_shift 后的端点（与已绘制的线段严格对齐）
    let p0 = (points[0].0, points[0].1 - cap_y_shift);
    let p1 = (points[points.len() - 1].0, points[points.len() - 1].1 - cap_y_shift);
    // 内点（用于计算切线方向）也要应用相同 y_shift，否则 cap 的切线方向会偏
    let next1 = (points[1].0, points[1].1 - cap_y_shift);
    let prev_n = (points[points.len() - 2].0, points[points.len() - 2].1 - cap_y_shift);
    if capstyle.eq_ignore_ascii_case("round") {
        // 圆头：直径 = cap_lw_px（与线段 stroke_w 严格相等），构造 SVG 路径半圆。
        // 为保证光滑，使用 64 段直线逼近整个半圆，肉眼完全无折线感。
        draw_round_cap(chart, p0, next1, cap_lw_px, x_min, x_max, y_min, y_max, rgb, true)?;
        draw_round_cap(chart, p1, prev_n, cap_lw_px, x_min, x_max, y_min, y_max, rgb, false)?;
        return Ok(());
    }
    if capstyle.eq_ignore_ascii_case("projecting") {
        // 方头：在每个端点沿切线方向延伸 cap_lw_px/2 像素。
        // 端点方向（首段/末段）—— 内点也应用 y_shift
        let (start_pt, start_next) = (p0, next1);
        let (end_pt, end_prev) = (p1, prev_n);
        // 单位方向向量
        let sdx = start_next.0 - start_pt.0;
        let sdy = start_next.1 - start_pt.1;
        let slen = (sdx * sdx + sdy * sdy).sqrt().max(1e-9);
        let sux = sdx / slen;
        let suy = sdy / slen;
        let edx = end_pt.0 - end_prev.0;
        let edy = end_pt.1 - end_prev.1;
        let elen = (edx * edx + edy * edy).sqrt().max(1e-9);
        let eux = edx / elen;
        let euy = edy / elen;
        // 像素/数据单位换算
        let area = chart.plotting_area();
        let dim = area.dim_in_pixel();
        let pw = dim.0 as f64;
        let ph = dim.1 as f64;
        let x_per_pix = if pw > 0.0 { (x_max - x_min) / pw } else { 1.0 };
        let y_per_pix = if ph > 0.0 { (y_max - y_min) / ph } else { 1.0 };
        let cap_px = cap_lw_px / 2.0;
        // 沿切线方向的像素偏移（数据空间）
        // screen_tangent = cap_px * (sux, suy)（在屏幕坐标中）
        // data_tangent = (cap_px * sux * x_per_pix, cap_px * suy * y_per_pix)
        // 法线方向同理
        let s_tan_x = cap_px * sux * x_per_pix;
        let s_tan_y = cap_px * suy * y_per_pix;
        let s_nor_x = cap_px * (-suy) * x_per_pix;
        let s_nor_y = cap_px * (sux) * y_per_pix;
        // 起始端矩形：4 个角点
        //  a: 端点外侧 + 法线 -cap/2
        //  b: 端点外侧 + 法线 +cap/2
        //  c: 端点 + 法线 +cap/2
        //  d: 端点 + 法线 -cap/2
        let sa = (start_pt.0 + s_tan_x - s_nor_x,
                  start_pt.1 + s_tan_y - s_nor_y);
        let sb = (start_pt.0 + s_tan_x + s_nor_x,
                  start_pt.1 + s_tan_y + s_nor_y);
        let sc = (start_pt.0 + s_nor_x,
                  start_pt.1 + s_nor_y);
        let sd = (start_pt.0 - s_nor_x,
                  start_pt.1 - s_nor_y);
        chart.draw_series(std::iter::once(Polygon::new(
            vec![sa, sb, sc, sd], rgb.filled(),
        ))).map_err(|e| PyRuntimeError::new_err(format!("Cap square (start): {}", e)))?;
        // 结束端矩形（沿切线反方向延伸）
        let e_tan_x = cap_px * eux * x_per_pix;
        let e_tan_y = cap_px * euy * y_per_pix;
        let e_nor_x = cap_px * (-euy) * x_per_pix;
        let e_nor_y = cap_px * (eux) * y_per_pix;
        let ea = (end_pt.0 + e_tan_x - e_nor_x,
                  end_pt.1 + e_tan_y - e_nor_y);
        let eb = (end_pt.0 + e_tan_x + e_nor_x,
                  end_pt.1 + e_tan_y + e_nor_y);
        let ec = (end_pt.0 + e_nor_x,
                  end_pt.1 + e_nor_y);
        let ed = (end_pt.0 - e_nor_x,
                  end_pt.1 - e_nor_y);
        chart.draw_series(std::iter::once(Polygon::new(
            vec![ea, eb, ec, ed], rgb.filled(),
        ))).map_err(|e| PyRuntimeError::new_err(format!("Cap square (end): {}", e)))?;
        return Ok(());
    }
    // 未知 capstyle：静默忽略
    Ok(())
}

/// 在端点处画一个**实心圆**（full disc）作为端点装饰。
///
/// 圆心位于线段端点，圆盘直径 = `cap_lw_px`（与线段实际线宽一致）。
/// 圆盘的后半（在切线方向指向线段内部）会被线段本身覆盖，
/// 前半（在切线方向指向线段外部）则作为端点凸出。
/// 这种实现确保圆盘与线段**完全契合**，无空隙、无错位。
///
/// **关键：为什么不用 Polygon 逼近？**
/// plotters BitMapBackend 的 `fill_polygon` 是**无 AA 的硬边光栅化**
/// （扫描线算法按"像素中心点是否在多边形内"判断），再多段数也改变不了锯齿本质。
/// 而 `plotters::element::Circle` 走的是 **`draw_circle` + 自带 AA 的边缘像素混合**
/// （`style.color().mix(v)` 做 alpha 渐变），无论多少段多边形都不如这一个调用光滑。
///
/// `endpoint`：线段端点（已应用 y_shift）
/// `cap_lw_px`：直径 = 实际线宽（像素）
/// `_next_point` / `_is_start`：保留参数以维持调用接口一致；圆盘中心对称不再需要切线方向
fn draw_round_cap<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    endpoint: (f64, f64),
    _next_point: (f64, f64),
    cap_lw_px: f64,
    _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64,
    rgb: &plotters::style::RGBColor,
    _is_start: bool,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    // plotters::element::Circle 接受 (中心坐标, 半径像素, 样式)
    // 内部调用 backend.draw_circle，自带 AA 边缘像素混合（rasterizer::draw_circle）
    // → 圆盘边缘完全光滑，无需任何多边形段数堆叠
    let cx = endpoint.0;
    let cy = endpoint.1;
    // 注意：Circle::new 的 size 是**半径**（像素），不是直径
    let radius_px = (cap_lw_px / 2.0).max(0.5);
    let style: ShapeStyle = rgb.filled();
    let circle_elem = Circle::new((cx, cy), radius_px, style);
    chart.draw_series(std::iter::once(circle_elem))
        .map_err(|e| PyRuntimeError::new_err(format!("Cap round circle: {}", e)))?;
    Ok(())
}
