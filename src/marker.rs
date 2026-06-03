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
    match marker {
        "o" => {
            // Use Circle element for filled circle markers
            let r = s;
            chart.draw_series(std::iter::once(Circle::new((x, y), r as i32, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "s" => {
            chart.draw_series(std::iter::once(Rectangle::new(
                [(x - s / 2.0, y - s / 2.0), (x + s / 2.0, y + s / 2.0)],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "^" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x - s / 2.0, y + s / 2.0),
                    (x + s / 2.0, y + s / 2.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "D" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x + s / 2.0, y),
                    (x, y + s / 2.0),
                    (x - s / 2.0, y),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "v" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y + s / 2.0),
                    (x - s / 2.0, y - s / 2.0),
                    (x + s / 2.0, y - s / 2.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "*" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x + s / 6.0, y - s / 6.0),
                    (x + s / 2.0, y),
                    (x + s / 6.0, y + s / 6.0),
                    (x, y + s / 2.0),
                    (x - s / 6.0, y + s / 6.0),
                    (x - s / 2.0, y),
                    (x - s / 6.0, y - s / 6.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "p" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x + s / 3.0, y - s / 4.0),
                    (x + s / 2.0, y),
                    (x + s / 3.0, y + s / 4.0),
                    (x, y + s / 2.0),
                    (x - s / 3.0, y + s / 4.0),
                    (x - s / 2.0, y),
                    (x - s / 3.0, y - s / 4.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "h" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x - s / 2.0, y - s / 4.0),
                    (x, y - s / 2.0),
                    (x + s / 2.0, y - s / 4.0),
                    (x + s / 2.0, y + s / 4.0),
                    (x, y + s / 2.0),
                    (x - s / 2.0, y + s / 4.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "x" => {
            let line_style: ShapeStyle = color.stroke_width(2).into();
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - s / 3.0, y - s / 3.0), (x + s / 3.0, y + s / 3.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - s / 3.0, y + s / 3.0), (x + s / 3.0, y - s / 3.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "+" => {
            let line_style: ShapeStyle = color.stroke_width(2).into();
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - s / 3.0, y), (x + s / 3.0, y)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x, y - s / 3.0), (x, y + s / 3.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        _ => {
            let r = 3.0;
            let n = 16;
            let mut points = Vec::with_capacity(n + 1);
            for i in 0..=n {
                let angle = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
                points.push((x + r * angle.cos(), y + r * angle.sin()));
            }
            chart.draw_series(std::iter::once(PathElement::new(points, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
    }
    Ok(())
}
