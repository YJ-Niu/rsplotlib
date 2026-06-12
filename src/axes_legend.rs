//! 图例渲染模块
//!
//! 在指定位置绘制图例框，包含每个 plot 调用对应的标签、线段、marker。

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use plotters::style::text_anchor::{HPos, VPos};

use crate::axes::scale_font;
use crate::colors::{RgbColor, to_plotters_color};
use crate::marker::draw_marker;
use crate::text_utils::normalize_spaces;

/// 渲染图例（如果设置了 `legend_loc` 且 `legend_labels` 非空）
///
/// # 参数
/// - `chart`: plotters 的 chart 上下文
/// - `legend_loc`: 图例位置字符串（如 "upper right"、"lower left" 等）
/// - `legend_labels`: 标签列表，每项为 (label, color, linestyle, marker, linewidth)
/// - `font_scale`: 字体缩放系数
/// - `x_min`, `x_max`, `y_min`, `y_max`: 数据范围
pub fn draw_legend<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    legend_loc: Option<&String>,
    legend_labels: &[(String, RgbColor, String, Option<String>, f64)],
    font_scale: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if let Some(loc) = legend_loc {
        if legend_labels.is_empty() {
            return Ok(());
        }
        let (x_anchor, y_anchor, h_pos, v_pos) = match loc.as_str() {
            "upper right" => (x_max, y_max, HPos::Right, VPos::Top),
            "upper left" => (x_min, y_max, HPos::Left, VPos::Top),
            "lower right" => (x_max, y_min, HPos::Right, VPos::Bottom),
            "lower left" => (x_min, y_min, HPos::Left, VPos::Bottom),
            "center" => {
                let cx = (x_min + x_max) / 2.0;
                let cy = (y_min + y_max) / 2.0;
                (cx, cy, HPos::Center, VPos::Center)
            }
            "right" => {
                (x_max, (y_min + y_max) / 2.0, HPos::Right, VPos::Center)
            }
            "center left" => {
                (x_min, (y_min + y_max) / 2.0, HPos::Left, VPos::Center)
            }
            "center right" => {
                (x_max, (y_min + y_max) / 2.0, HPos::Right, VPos::Center)
            }
            "lower center" => {
                ((x_min + x_max) / 2.0, y_min, HPos::Center, VPos::Bottom)
            }
            "upper center" => {
                ((x_min + x_max) / 2.0, y_max, HPos::Center, VPos::Top)
            }
            _ => {
                let try_x = x_max - (x_max - x_min) * 0.3;
                let try_y = y_max - (y_max - y_min) * 0.1;
                (try_x, try_y, HPos::Right, VPos::Top)
            }
        };

        let entry_count = legend_labels.len();
        let x_range = (x_max - x_min).abs();
        let y_range = (y_max - y_min).abs();
        let entry_height = y_range * 0.04;
        let legend_height = entry_height * entry_count as f64 + y_range * 0.02;
        let legend_width = x_range * 0.25;

        let (box_x1, box_x2) = match h_pos {
            HPos::Right => (x_anchor - legend_width, x_anchor),
            HPos::Left => (x_anchor, x_anchor + legend_width),
            HPos::Center => (x_anchor - legend_width / 2.0, x_anchor + legend_width / 2.0),
        };
        let (box_y1, box_y2) = match v_pos {
            VPos::Top => (y_anchor - legend_height, y_anchor),
            VPos::Bottom => (y_anchor, y_anchor + legend_height),
            VPos::Center => (y_anchor - legend_height / 2.0, y_anchor + legend_height / 2.0),
        };

        let bg_fill: ShapeStyle = RGBColor(255, 255, 255).mix(0.85).filled().into();
        let bg_border: ShapeStyle = RGBColor(180, 180, 180).stroke_width(1).into();

        let bg_rect = Rectangle::new(
            [(box_x1, box_y1), (box_x2, box_y2)],
            bg_fill,
        );
        let bg_elements = vec![
            bg_rect,
            Rectangle::new(
                [(box_x1, box_y1), (box_x2, box_y2)],
                bg_border,
            ),
        ];
        for elem in bg_elements {
            chart.draw_series(std::iter::once(elem))
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend bg: {}", e)))?;
        }

        for (i, (label, color, ls, marker_opt, lw)) in legend_labels.iter().enumerate() {
            let y_pos = box_y1 + entry_height * 0.75 + i as f64 * entry_height;
            let x_line_start = box_x1 + x_range * 0.015;
            let x_line_end = box_x1 + x_range * 0.06;
            let x_text = box_x1 + x_range * 0.07;

            let rgb = to_plotters_color(*color);
            // 使用实际的 linewidth（与数据线保持一致），将 points 转换为像素
            // plotters stroke_width(n) 实际渲染为 2n-1 像素，使用 stroke = max(1, width_px - 1) 接近 mpl
            let lw_px = ((*lw) * font_scale).max(1.0).round() as u32;
            let legend_stroke = (lw_px as i32 - 1).max(1) as u32;
            let line_style: ShapeStyle = rgb.stroke_width(legend_stroke).into();

            // 根据线型绘制图例线段
            match ls.as_str() {
                "--" => {
                    let dash_len = 8.0;
                    let gap_len = 4.0;
                    let mut pos = x_line_start;
                    let mut drawing = true;
                    while pos < x_line_end {
                        let seg_end = if drawing { (pos + dash_len).min(x_line_end) } else { (pos + gap_len).min(x_line_end) };
                        if drawing {
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(pos, y_pos), (seg_end, y_pos)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Legend dashed: {}", e)))?;
                        }
                        pos = seg_end;
                        drawing = !drawing;
                    }
                }
                ":" => {
                    let dot_len = 2.0;
                    let gap_len = 4.0;
                    let mut pos = x_line_start;
                    let mut drawing = true;
                    while pos < x_line_end {
                        let seg_end = if drawing { (pos + dot_len).min(x_line_end) } else { (pos + gap_len).min(x_line_end) };
                        if drawing {
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(pos, y_pos), (seg_end, y_pos)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Legend dotted: {}", e)))?;
                        }
                        pos = seg_end;
                        drawing = !drawing;
                    }
                }
                "-." => {
                    let dash_len = 8.0;
                    let dot_len = 2.0;
                    let gap_len = 3.0;
                    let mut pos = x_line_start;
                    let mut is_dash = true;
                    while pos < x_line_end {
                        let mark_len = if is_dash { dash_len } else { dot_len };
                        let seg_end = (pos + mark_len).min(x_line_end);
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(pos, y_pos), (seg_end, y_pos)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Legend dash-dot: {}", e)))?;
                        pos = seg_end;
                        let gap_end = (pos + gap_len).min(x_line_end);
                        pos = gap_end;
                        is_dash = !is_dash;
                    }
                }
                _ => {
                    // 实线
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(x_line_start, y_pos), (x_line_end, y_pos)], line_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend line: {}", e)))?;
                }
            }

            if let Some(mkr) = marker_opt {
                if !mkr.is_empty() {
                    let mid_x = (x_line_start + x_line_end) / 2.0;
                    draw_marker(chart, mkr, mid_x, y_pos, x_range * 0.01, rgb)
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend marker: {}", e)))?;
                }
            }

            chart.draw_series(std::iter::once(plotters::element::Text::new(
                normalize_spaces(label),
                (x_text, y_pos),
                ("sans-serif", scale_font(11.0, font_scale)),
            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend text: {}", e)))?;
        }
    }
    Ok(())
}
