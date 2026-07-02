use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;

/// 绘制单个 marker。
///
/// `size` 语义：marker 包围盒的「半边长」（= markersize_px / 2），对圆形而言即半径，单位为像素。
/// matplotlib 中 markersize 单位为 points，包围盒边长(像素) = markersize * dpi/72，因此各形状
/// 顶点相对中心的像素偏移取 `s`（半边长），使包围盒边长 = 2s = markersize_px，与 matplotlib 一致。
///
/// 关键：多边形 / 折线类 marker 必须在「像素空间」构造——plotters 的 `Polygon` / `Rectangle`
/// / `PathElement` 会把坐标解释为**数据坐标**，直接用像素偏移量作数据坐标会让 marker 尺寸随坐标轴
/// 量程变化（在小量程下甚至撑满整个绘图区）。这里统一用 `EmptyElement::at(数据点) + 形状(像素偏移)`：
/// 锚点用数据坐标，子形状坐标按**后端像素**偏移解释（注意后端 y 轴向下）。`Circle` 的圆心可用数据
/// 坐标而半径本就以像素为单位，故圆 / 点 marker 仍可直接用数据锚点。
pub fn draw_marker<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    marker: &str,
    x: f64,
    y: f64,
    size: f64,
    color: RGBColor,
) -> PyResult<()> {
    let s = size;
    let si = s.round() as i32;
    let style: ShapeStyle = color.filled();
    let err = |e| PyRuntimeError::new_err(format!("Marker error: {}", e));
    match marker {
        "o" => {
            // 圆：数据坐标锚点 + 像素半径，直径 = 2s = markersize_px
            chart.draw_series(std::iter::once(Circle::new((x, y), si, style)))
                .map_err(err)?;
        }
        "s" => {
            // 正方形：半边长 = s（像素），边长 = markersize_px
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y)) + Rectangle::new([(-si, -si), (si, si)], style),
            )).map_err(err)?;
        }
        "^" => {
            // 后端 y 轴向下：顶点在上取负 y
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y)) + Polygon::new(
                    vec![(0, -si), (-si, si), (si, si)], style,
                ),
            )).map_err(err)?;
        }
        "v" => {
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y)) + Polygon::new(
                    vec![(0, si), (-si, -si), (si, -si)], style,
                ),
            )).map_err(err)?;
        }
        "D" => {
            // 菱形 = 边长为 markersize 的正方形旋转 45°，对角线半长 = s*√2
            let d = (s * std::f64::consts::SQRT_2).round() as i32;
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y)) + Polygon::new(
                    vec![(0, -d), (d, 0), (0, d), (-d, 0)], style,
                ),
            )).map_err(err)?;
        }
        "*" => {
            // 四角星：外顶点半径 s，内顶点半径 s/3
            let i = (s / 3.0).round() as i32;
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y)) + Polygon::new(
                    vec![
                        (0, -si), (i, -i), (si, 0), (i, i),
                        (0, si), (-i, i), (-si, 0), (-i, -i),
                    ],
                    style,
                ),
            )).map_err(err)?;
        }
        "p" => {
            let a = (2.0 * s / 3.0).round() as i32;
            let b = (s / 2.0).round() as i32;
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y)) + Polygon::new(
                    vec![
                        (0, -si), (a, -b), (si, 0), (a, b),
                        (0, si), (-a, b), (-si, 0), (-a, -b),
                    ],
                    style,
                ),
            )).map_err(err)?;
        }
        "h" => {
            // 正六边形（尖顶）：外接圆半径 = s，故高 = 2s = markersize_px，
            // 宽 = √3·s ≈ 0.866·高（与 matplotlib hexagon1 的宽高比一致）。
            let w = (0.8660254_f64 * s).round() as i32;
            let b = (s / 2.0).round() as i32;
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y)) + Polygon::new(
                    vec![
                        (0, -si), (w, -b), (w, b),
                        (0, si), (-w, b), (-w, -b),
                    ],
                    style,
                ),
            )).map_err(err)?;
        }
        "x" => {
            let line_style: ShapeStyle = color.stroke_width(2);
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y))
                    + PathElement::new(vec![(-si, -si), (si, si)], line_style)
                    + PathElement::new(vec![(-si, si), (si, -si)], line_style),
            )).map_err(err)?;
        }
        "+" => {
            let line_style: ShapeStyle = color.stroke_width(2);
            chart.draw_series(std::iter::once(
                EmptyElement::at((x, y))
                    + PathElement::new(vec![(-si, 0), (si, 0)], line_style)
                    + PathElement::new(vec![(0, -si), (0, si)], line_style),
            )).map_err(err)?;
        }
        _ => {
            // "." / "," 等点 marker：调用方已把半径换算好（"." = 0.25*markersize_px，"," ≈ 1px）
            let r = si.max(1);
            chart.draw_series(std::iter::once(Circle::new((x, y), r, style)))
                .map_err(err)?;
        }
    }
    Ok(())
}
