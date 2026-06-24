use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;

pub fn draw_marker<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    marker: &str,
    x: f64,
    y: f64,
    size: f64,
    color: RGBColor,
) -> PyResult<()> {
    let s = size;
    let style: ShapeStyle = color.filled().into();
    
    let x_range = chart.x_range();
    let y_range = chart.y_range();
    let x_diff = x_range.end - x_range.start;
    let y_diff = y_range.end - y_range.start;
    let x_span = x_diff.max(1e-10) * s;
    let y_span = y_diff.max(1e-10) * s;

    match marker {
        "o" => {
            // Use Circle element for filled circle markers
            let r = s;
            chart.draw_series(std::iter::once(Circle::new((x, y), r as i32, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "s" => {
            chart.draw_series(std::iter::once(Rectangle::new(
                [(x - x_span / 100.0, y - y_span / 100.0), (x + x_span / 100.0, y + y_span / 100.0)],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "v" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - y_span / 500.0),
                    (x - x_span / 400.0, y + y_span / 500.0),
                    (x + x_span / 400.0, y + y_span / 500.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "D" => {
            chart.draw_series(std::iter::once(Polygon::new(
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
        "^" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y + y_span / 500.0),
                    (x - x_span / 400.0, y - x_span / 400.0),
                    (x + x_span / 400.0, y - x_span / 400.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "*" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - y_span / 500.0),
                    (x + x_span / 400.0, y - y_span / 500.0),
                    (x + x_span / 400.0, y),
                    (x + x_span / 400.0, y + y_span / 500.0),
                    (x, y + y_span / 500.0),
                    (x - x_span / 400.0, y + y_span / 500.0),
                    (x - x_span / 400.0, y),
                    (x - x_span / 400.0, y - y_span / 500.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "p" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - y_span / 500.0),
                    (x + x_span / 400.0, y - y_span / 500.0),
                    (x + x_span / 400.0, y),
                    (x + x_span / 400.0, y + y_span / 500.0),
                    (x, y + y_span / 500.0),
                    (x - x_span / 400.0, y + y_span / 500.0),
                    (x - x_span / 400.0, y),
                    (x - x_span / 400.0, y - y_span / 500.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "h" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x - x_span / 400.0, y - y_span / 500.0),
                    (x, y - y_span / 500.0),
                    (x + x_span / 400.0, y - y_span / 500.0),
                    (x + x_span / 400.0, y + y_span / 500.0),
                    (x, y + y_span / 500.0),
                    (x - x_span / 400.0, y + y_span / 500.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "x" => {
            let line_style: ShapeStyle = color.stroke_width(2).into();
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - x_span / 400.0, y - y_span / 500.0), (x + x_span / 400.0, y + y_span / 500.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - x_span / 400.0, y + y_span / 500.0), (x + x_span / 400.0, y - y_span / 500.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "+" => {
            let arm_len_x = s / 400.0 * x_span;
            let arm_len_y = s / 400.0 * y_span;
            let line_style: ShapeStyle = color.stroke_width(2).filled().into();
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - arm_len_x + x_span * 0.002, y), (x + arm_len_x - x_span * 0.001, y)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x, y - arm_len_y - y_span * 0.002), (x, y + arm_len_y + y_span * 0.001)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        _ => {
            // "." / "," 等像素点 marker：直接画一个小圆（半径 <= 1）
            let r = (s as i32).max(1);
            chart.draw_series(std::iter::once(Circle::new((x, y), r, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
    }
    Ok(())
}
