use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};

pub fn draw_marker<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    marker: &str,
    x: f64,
    y: f64,
    size: f64,
    color: RGBColor,
) -> PyResult<()> {
    let s = size.max(0.5);
    let style: ShapeStyle = color.filled().into();
    let line_style: ShapeStyle = color.stroke_width(2).into();
    let x_range = chart.x_range();
    let y_range = chart.y_range();
    let x_diff = x_range.end - x_range.start;
    let y_diff = y_range.end - y_range.start;
    let x_span = x_diff.max(1e-10) * s;
    let y_span = y_diff.max(1e-10) * s;

    // 统一使用 x_span/400.0 和 y_span/500.0 作为基础单位

    match marker {
        // --- filled circle (already existed, explicit for ".") ---
        "." | "o" => {
            let r = s.max(1.0);
            chart
                .draw_series(std::iter::once(Circle::new((x, y), r as i32, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- pixel (tiny dot) ---
        "," => {
            chart
                .draw_series(std::iter::once(Circle::new((x, y), 1.max((s / 2.0) as i32), style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- square ---
        "s" => {
            chart
                .draw_series(std::iter::once(Rectangle::new(
                    [
                        (x - x_span / 100.0, y - y_span / 100.0),
                        (x + x_span / 100.0, y + y_span / 100.0),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- triangle down ---
        "v" => {
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x, y - y_span / 500.0),
                        (x - x_span / 400.0, y + y_span / 500.0),
                        (x + x_span / 400.0, y + y_span / 500.0),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- triangle up ---
        "^" => {
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x, y + y_span / 500.0),
                        (x - x_span / 400.0, y - y_span / 500.0),
                        (x + x_span / 400.0, y - y_span / 500.0),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- triangle left ---
        "<" => {
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x - x_span / 500.0, y),
                        (x + x_span / 500.0, y - y_span / 400.0),
                        (x + x_span / 500.0, y + y_span / 400.0),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- triangle right ---
        ">" => {
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x + x_span / 500.0, y),
                        (x - x_span / 500.0, y - y_span / 400.0),
                        (x - x_span / 500.0, y + y_span / 400.0),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- diamond ---
        "D" => {
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x, y - y_span / 500.0),
                        (x + x_span / 400.0, y),
                        (x, y + y_span / 500.0),
                        (x - x_span / 400.0, y),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- thin diamond ---
        "d" => {
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x, y - y_span / 400.0),
                        (x + x_span / 600.0, y),
                        (x, y + y_span / 400.0),
                        (x - x_span / 600.0, y),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- star (5-point) ---
        "*" => {
            let r_inner = 0.4;
            let angles: Vec<f64> = (0..10)
                .map(|i| std::f64::consts::PI * 2.0 * i as f64 / 10.0 - std::f64::consts::PI / 2.0)
                .collect();
            let pts: Vec<(f64, f64)> = angles
                .iter()
                .enumerate()
                .map(|(i, &a)| {
                    let r = if i % 2 == 0 { 1.0 } else { r_inner };
                    (
                        x + x_span / 400.0 * r * a.cos(),
                        y + y_span / 500.0 * r * a.sin(),
                    )
                })
                .collect();
            chart
                .draw_series(std::iter::once(Polygon::new(pts, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- pentagon ---
        "p" => {
            let pts: Vec<(f64, f64)> = (0..5)
                .map(|i| {
                    let a = std::f64::consts::PI * 2.0 * i as f64 / 5.0 - std::f64::consts::PI / 2.0;
                    (
                        x + x_span / 400.0 * a.cos(),
                        y + y_span / 500.0 * a.sin(),
                    )
                })
                .collect();
            chart
                .draw_series(std::iter::once(Polygon::new(pts, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- plus ---
        "+" => {
            chart
                .draw_series(std::iter::once(PathElement::new(
                    vec![(x - x_span / 400.0 * 200.0, y), (x + x_span / 400.0 * 200.0, y)],
                    line_style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart
                .draw_series(std::iter::once(PathElement::new(
                    vec![(x, y - y_span / 500.0 * 250.0), (x, y + y_span / 500.0 * 250.0)],
                    line_style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- filled plus (plus sign with filled center area) ---
        "P" => {
            // Draw a filled shape formed by the plus arms
            let arm_x = x_span / 400.0 * 120.0;
            let arm_y = y_span / 500.0 * 150.0;
            let w = x_span / 400.0 * 2.0; // arm width
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x - arm_x, y - w),
                        (x + arm_x, y - w),
                        (x + arm_x, y + w),
                        (x - arm_x, y + w),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x - w, y - arm_y),
                        (x + w, y - arm_y),
                        (x + w, y + arm_y),
                        (x - w, y + arm_y),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- x (unfilled) ---
        "x" => {
            let arm_x = x_span / 400.0 * 160.0;
            let arm_y = y_span / 500.0 * 200.0;
            chart
                .draw_series(std::iter::once(PathElement::new(
                    vec![(x - arm_x, y - arm_y), (x + arm_x, y + arm_y)],
                    line_style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart
                .draw_series(std::iter::once(PathElement::new(
                    vec![(x - arm_x, y + arm_y), (x + arm_x, y - arm_y)],
                    line_style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- x (filled) ---
        "X" => {
            let arm_x = x_span / 400.0 * 160.0;
            let arm_y = y_span / 500.0 * 200.0;
            let w = x_span / 400.0 * 3.0;
            // Draw filled rotated square
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x - arm_x, y - arm_y + w),
                        (x - arm_x + w, y - arm_y),
                        (x + arm_x - w, y + arm_y),
                        (x + arm_x, y + arm_y - w),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart
                .draw_series(std::iter::once(Polygon::new(
                    vec![
                        (x - arm_x, y + arm_y - w),
                        (x - arm_x + w, y + arm_y),
                        (x + arm_x - w, y - arm_y),
                        (x + arm_x, y - arm_y + w),
                    ],
                    style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- vertical line ---
        "|" => {
            chart
                .draw_series(std::iter::once(PathElement::new(
                    vec![(x, y - y_span / 200.0), (x, y + y_span / 200.0)],
                    line_style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- horizontal line ---
        "_" => {
            chart
                .draw_series(std::iter::once(PathElement::new(
                    vec![(x - x_span / 300.0, y), (x + x_span / 400.0, y)],
                    line_style,
                )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- None / empty / space: no marker ---
        "" | "None" | " " => {
            // do nothing
        }
        // --- $...$ rendered character marker ---
        _ if marker.starts_with('$') && marker.ends_with('$') && marker.len() > 2 => {
            let ch = &marker[1..marker.len() - 1];
            let text_style: TextStyle = TextStyle::from(("sans-serif", s.max(6.0)))
                .color(&color)
                .pos(Pos::new(HPos::Center, VPos::Center));
            chart
                .draw_series(std::iter::once(Text::new(ch.to_string(), (x, y), text_style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        // --- fallback: small filled circle ---
        _ => {
            let r = (s as i32).max(1);
            chart
                .draw_series(std::iter::once(Circle::new((x, y), r, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
    }
    Ok(())
}