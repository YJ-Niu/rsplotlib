//! 网格线渲染模块
//!
//! 提供主网格线和副网格线的绘制功能：
//! - `draw_grid_lines()`: 通用网格线绘制（支持实线、虚线、点线）
//! - `draw_thick_polyline()`: 使用填充多边形绘制指定像素宽度的实线（后备方案）
//! - `filter_minor_ticks()`: 过滤与主刻度位置重叠的副刻度，避免副网格线覆盖主网格线

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;

use crate::core::colors::{RgbColor, to_plotters_color};

/// 过滤掉与主刻度位置重叠的副刻度
///
/// 容差为主刻度步长的 1e-6 倍（最小 1e-9），保证只有真正重叠的副刻度被过滤。
pub fn filter_minor_ticks(minor: &[f64], major: &[f64]) -> Vec<f64> {
    if major.len() < 2 {
        return minor.to_vec();
    }
    let tol = major.windows(2).map(|w| (w[1] - w[0]).abs())
        .fold(f64::INFINITY, f64::min) * 1e-6;
    let tol = tol.max(1e-9);
    minor.iter()
        .filter(|m| !major.iter().any(|mj| (*m - mj).abs() < tol))
        .cloned()
        .collect()
}

/// 绘制主或副网格线
///
/// # 参数
/// - `chart`: plotters 的 chart 上下文
/// - `vertical`: true 表示绘制垂直网格线（在 x=tick 处），false 表示水平
/// - `ticks`: 刻度位置数组（数据坐标）
/// - `color`: 网格线颜色
/// - `lw`: 网格线宽度（points，仅用于向后兼容保留；实际像素宽度由 `is_major` 决定）
/// - `ls`: 网格线样式（Some("--") 虚线，Some(":") 点线，None 实线）
/// - `is_major`: 是否主网格线（主=2px，副=1px，按 README 要求）
/// - `font_scale`: 字体缩放系数（保留以兼容旧调用方）
/// - `x_min`, `x_max`, `y_min`, `y_max`: 数据范围
pub fn draw_grid_lines<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    vertical: bool,
    ticks: &[f64],
    color: RgbColor,
    _lw: f64,
    ls: Option<&str>,
    _font_scale: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let rgb = to_plotters_color(color);
    // lw 是 points，需要转换为像素：1pt = dpi/72 px
    // dpi = 72 * font_scale, 所以 px = lw * font_scale
    // matplotlib 通过 AA 在 0.5pt 量级产生柔和灰线；plotters 无 AA，
    // plotters stroke_width(n) 实际渲染为 2n-1 像素（带 AA），所以
    // 使用 stroke = max(1, width_px - 1) 接近 mpl 的视觉粗细
    // 主网格线固定为 2 像素（按 README 要求），副网格线为 1 像素
    let stroke_w = 1u32;
    let style = rgb.stroke_width(stroke_w);

    // 获取绘图区域像素尺寸，用于 dash 长度从像素转换到数据坐标
    let area = chart.plotting_area();
    let dim = area.dim_in_pixel();
    let pw = dim.0 as f64;
    let ph = dim.1 as f64;
    let x_per_pix_p = (x_max - x_min) / pw;
    let y_per_pix_p = (y_max - y_min) / ph;
    let mut paths: Vec<Vec<(f64, f64)>> = Vec::with_capacity(ticks.len());
    for &tick in ticks {
        if vertical {
            if tick >= x_min && tick <= x_max {
                paths.push(vec![(tick, y_min), (tick, y_max)]);
            }
        } else {
            if tick >= y_min && tick <= y_max {
                paths.push(vec![(x_min, tick), (x_max, tick)]);
            }
        }
    }

    // dash/gap 长度与线宽像素成正比，2px 主线对应 8/4 像素，1px 副线对应 4/2 像素
    let dash_px = (stroke_w as f64 * 4.0).max(2.0);
    let gap_px = (stroke_w as f64 * 2.0).max(2.0);
    let dot_px = (stroke_w as f64 * 1.0).max(1.0);
    let dash_len = if vertical { dash_px * y_per_pix_p } else { dash_px * x_per_pix_p };
    let gap_len = if vertical { gap_px * y_per_pix_p } else { gap_px * x_per_pix_p };
    let dot_len = if vertical { dot_px * y_per_pix_p } else { dot_px * x_per_pix_p };

    match ls {
        Some("--") => {
            // 虚线网格：按线宽像素成比例，dash=4*stroke_w 像素，gap=2*stroke_w 像素
            for path in &paths {
                if path.len() >= 2 {
                    let dx = path[1].0 - path[0].0;
                    let dy = path[1].1 - path[0].1;
                    let total_len = (dx * dx + dy * dy).sqrt();
                    let unit_x = dx / total_len;
                    let unit_y = dy / total_len;
                    let mut pos = 0.0f64;
                    let mut drawing = true;
                    while pos < total_len {
                        let seg_len = if drawing { dash_len } else { gap_len };
                        let end_pos = (pos + seg_len).min(total_len);
                        if drawing {
                            let p1 = (path[0].0 + unit_x * pos, path[0].1 + unit_y * pos);
                            let p2 = (path[0].0 + unit_x * end_pos, path[0].1 + unit_y * end_pos);
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![p1, p2], style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Dashed grid: {}", e)))?;
                        }
                        pos = end_pos;
                        drawing = !drawing;
                    }
                }
            }
        }
        Some(":") => {
            // 点线网格：按线宽像素成比例
            for path in &paths {
                if path.len() >= 2 {
                    let dx = path[1].0 - path[0].0;
                    let dy = path[1].1 - path[0].1;
                    let total_len = (dx * dx + dy * dy).sqrt();
                    let unit_x = dx / total_len;
                    let unit_y = dy / total_len;
                    let mut pos = 0.0f64;
                    let mut drawing = true;
                    while pos < total_len {
                        let seg_len = if drawing { dot_len } else { gap_len };
                        let end_pos = (pos + seg_len).min(total_len);
                        if drawing {
                            let p1 = (path[0].0 + unit_x * pos, path[0].1 + unit_y * pos);
                            let p2 = (path[0].0 + unit_x * end_pos, path[0].1 + unit_y * end_pos);
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![p1, p2], style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Dotted grid: {}", e)))?;
                        }
                        pos = end_pos;
                        drawing = !drawing;
                    }
                }
            }
        }
        _ => {
            // 实线网格
            for path in paths {
                chart.draw_series(std::iter::once(PathElement::new(path, style)))
                    .map_err(|e| PyRuntimeError::new_err(format!("Grid line: {}", e)))?;
            }
        }
    }
    Ok(())
}

/// 使用填充多边形绘制指定像素宽度的实线（后备方案，当前实线渲染走原生 stroke_width）
pub fn draw_thick_polyline<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    points: &[(f64, f64)],
    width_px: u32,
    color: RgbColor,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if points.len() < 2 || width_px < 1 { return Ok(()); }
    let rgb = to_plotters_color(color);
    // 获取绘图区域的像素尺寸，用于计算像素到数据坐标的换算
    let area = chart.plotting_area();
    let dim = area.dim_in_pixel();
    let pw = dim.0 as f64;
    let ph = dim.1 as f64;
    if pw < 1.0 || ph < 1.0 { return Ok(()); }
    // 数据坐标每像素对应的数据单位
    let x_range = x_max - x_min;
    let y_range = y_max - y_min;
    let x_per_pix = x_range / pw;
    let y_per_pix = y_range / ph;
    let half_w = width_px as f64 / 2.0;
    // 调整半宽：plotters 像素光栅化对半覆盖像素做整行填充。
    // 测试显示 half_w_adj 与渲染像素宽度的关系：
    //   half_w_adj=0.5 → 1 px (lw_px=1)
    //   half_w_adj=1.0 → 2 px (lw_px=2)
    //   half_w_adj=1.25 → 4 px (因整行填充放大)
    //   half_w_adj=1.5 → 4 px
    // 为获得 3 px 视觉效果，需要 half_w_adj ≈ 1.0-1.1
    // 使用 half_w_adj = half_w - 0.5 来精确控制像素宽度
    let half_w_adj = (half_w - 0.5).max(0.0);
    let fill: ShapeStyle = rgb.filled();
    for win in points.windows(2) {
        let (x1, y1) = win[0];
        let (x2, y2) = win[1];
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-10 { continue; }
        // 单位垂直向量在数据坐标空间中的偏移
        let perp_x = -dy / len * half_w_adj * x_per_pix;
        let perp_y = dx / len * half_w_adj * y_per_pix;
        let poly = vec![
            (x1 + perp_x, y1 + perp_y),
            (x1 - perp_x, y1 - perp_y),
            (x2 - perp_x, y2 - perp_y),
            (x2 + perp_x, y2 + perp_y),
        ];
        chart.draw_series(std::iter::once(Polygon::new(poly, fill)))
            .map_err(|e| PyRuntimeError::new_err(format!("Thick line: {}", e)))?;
    }
    Ok(())
}
