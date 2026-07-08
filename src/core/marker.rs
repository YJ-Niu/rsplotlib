use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// 绘制单个 marker。
///
/// `size` 语义：marker 包围盒的「半边长」（= markersize_px / 2），对圆形而言即半径，单位为像素。
/// matplotlib 中 markersize 单位为 points，包围盒边长(像素) = markersize * dpi/72，因此各形状
/// 顶点相对中心的像素偏移取 `s`（半边长），使包围盒边长 = 2s = markersize_px，与 matplotlib 一致。
///
/// `face` 为标记填充色 (markerfacecolor)，`edge` 为标记边框色 (markeredgecolor)。当二者相同时
/// 只填充、不额外描边（与旧行为一致）；不同时先填充再用 edge 描出轮廓。'x' / '+' 为纯描线标记，
/// 始终使用 edge 色。`alpha` 为透明度 (0.0-1.0)，混入填充与描边色（用于 scatter 的透明散点等）。
///
/// `edge_width` 为描边/线宽（像素）。传入 `<= 0.0` 表示沿用历史默认：填充类 marker 轮廓 1px，
/// 'x' / '+' 纯描线 marker 2px。scatter 的 edgecolors/linewidths 会传入换算好的正值以覆盖。
///
/// 关键：多边形 / 折线类 marker 必须在「像素空间」构造——plotters 的 `Polygon` / `Rectangle`
/// / `PathElement` 会把坐标解释为**数据坐标**，直接用像素偏移量作数据坐标会让 marker 尺寸随坐标轴
/// 量程变化（在小量程下甚至撑满整个绘图区）。这里统一用 `EmptyElement::at(数据点) + 形状(像素偏移)`：
/// 锚点用数据坐标，子形状坐标按**后端像素**偏移解释（注意后端 y 轴向下）。`Circle` 的圆心可用数据
/// 坐标而半径本就以像素为单位，故圆 / 点 marker 仍可直接用数据锚点。
#[allow(clippy::too_many_arguments)]
pub fn draw_marker<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    marker: &str,
    x: f64,
    y: f64,
    size: f64,
    face: RGBColor,
    edge: RGBColor,
    alpha: f64,
    edge_width: f64,
) -> PyResult<()> {
    let s = size;
    let si = s.round() as i32;
    let fill: ShapeStyle = face.mix(alpha).filled();
    // edge_width <= 0.0 -> 历史默认；否则按传入像素宽度描边（至少 1px）。
    let ew_fill: u32 = if edge_width <= 0.0 {
        1
    } else {
        edge_width.round().max(1.0) as u32
    };
    let ew_line: u32 = if edge_width <= 0.0 {
        2
    } else {
        edge_width.round().max(1.0) as u32
    };
    let edge_style: ShapeStyle = edge.mix(alpha).stroke_width(ew_fill);
    let need_edge = edge != face;
    let err = |e| PyRuntimeError::new_err(format!("Marker error: {}", e));
    match marker {
        "o" => {
            // 圆：数据坐标锚点 + 像素半径，直径 = 2s = markersize_px
            chart
                .draw_series(std::iter::once(Circle::new((x, y), si, fill)))
                .map_err(err)?;
            if need_edge {
                chart
                    .draw_series(std::iter::once(Circle::new((x, y), si, edge_style)))
                    .map_err(err)?;
            }
        }
        "s" => {
            // 正方形：半边长 = s（像素），边长 = markersize_px
            chart
                .draw_series(std::iter::once(
                    EmptyElement::at((x, y)) + Rectangle::new([(-si, -si), (si, si)], fill),
                ))
                .map_err(err)?;
            if need_edge {
                chart
                    .draw_series(std::iter::once(
                        EmptyElement::at((x, y))
                            + Rectangle::new([(-si, -si), (si, si)], edge_style),
                    ))
                    .map_err(err)?;
            }
        }
        "^" => {
            // 后端 y 轴向下：顶点在上取负 y
            let pts = vec![(0, -si), (-si, si), (si, si)];
            draw_polygon_marker(chart, x, y, &pts, fill, edge_style, need_edge)?;
        }
        "v" => {
            let pts = vec![(0, si), (-si, -si), (si, -si)];
            draw_polygon_marker(chart, x, y, &pts, fill, edge_style, need_edge)?;
        }
        "D" => {
            // 菱形 = 边长为 markersize 的正方形旋转 45°，对角线半长 = s*√2
            let d = (s * std::f64::consts::SQRT_2).round() as i32;
            let pts = vec![(0, -d), (d, 0), (0, d), (-d, 0)];
            draw_polygon_marker(chart, x, y, &pts, fill, edge_style, need_edge)?;
        }
        "*" => {
            // 四角星：外顶点半径 s，内顶点半径 s/3
            let i = (s / 3.0).round() as i32;
            let pts = vec![
                (0, -si),
                (i, -i),
                (si, 0),
                (i, i),
                (0, si),
                (-i, i),
                (-si, 0),
                (-i, -i),
            ];
            draw_polygon_marker(chart, x, y, &pts, fill, edge_style, need_edge)?;
        }
        "p" => {
            let a = (2.0 * s / 3.0).round() as i32;
            let b = (s / 2.0).round() as i32;
            let pts = vec![
                (0, -si),
                (a, -b),
                (si, 0),
                (a, b),
                (0, si),
                (-a, b),
                (-si, 0),
                (-a, -b),
            ];
            draw_polygon_marker(chart, x, y, &pts, fill, edge_style, need_edge)?;
        }
        "h" => {
            // 正六边形（尖顶）：外接圆半径 = s，故高 = 2s = markersize_px，
            // 宽 = √3·s ≈ 0.866·高（与 matplotlib hexagon1 的宽高比一致）。
            let w = (0.8660254_f64 * s).round() as i32;
            let b = (s / 2.0).round() as i32;
            let pts = vec![(0, -si), (w, -b), (w, b), (0, si), (-w, b), (-w, -b)];
            draw_polygon_marker(chart, x, y, &pts, fill, edge_style, need_edge)?;
        }
        "x" => {
            // 纯描线 marker：使用边框色
            let line_style: ShapeStyle = edge.mix(alpha).stroke_width(ew_line);
            chart
                .draw_series(std::iter::once(
                    EmptyElement::at((x, y))
                        + PathElement::new(vec![(-si, -si), (si, si)], line_style)
                        + PathElement::new(vec![(-si, si), (si, -si)], line_style),
                ))
                .map_err(err)?;
        }
        "+" => {
            let line_style: ShapeStyle = edge.mix(alpha).stroke_width(ew_line);
            chart
                .draw_series(std::iter::once(
                    EmptyElement::at((x, y))
                        + PathElement::new(vec![(-si, 0), (si, 0)], line_style)
                        + PathElement::new(vec![(0, -si), (0, si)], line_style),
                ))
                .map_err(err)?;
        }
        _ => {
            // "." / "," 等点 marker：调用方已把半径换算好（"." = 0.25*markersize_px，"," ≈ 1px）
            let r = si.max(1);
            chart
                .draw_series(std::iter::once(Circle::new((x, y), r, fill)))
                .map_err(err)?;
            if need_edge {
                chart
                    .draw_series(std::iter::once(Circle::new((x, y), r, edge_style)))
                    .map_err(err)?;
            }
        }
    }
    Ok(())
}

/// 绘制多边形 marker：先填充，need_edge 时再用闭合折线描出边框。
fn draw_polygon_marker<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    x: f64,
    y: f64,
    pts: &[(i32, i32)],
    fill: ShapeStyle,
    edge_style: ShapeStyle,
    need_edge: bool,
) -> PyResult<()> {
    let err = |e| PyRuntimeError::new_err(format!("Marker error: {}", e));
    chart
        .draw_series(std::iter::once(
            EmptyElement::at((x, y)) + Polygon::new(pts.to_vec(), fill),
        ))
        .map_err(err)?;
    if need_edge {
        let mut outline = pts.to_vec();
        if let Some(&first) = pts.first() {
            outline.push(first);
        }
        chart
            .draw_series(std::iter::once(
                EmptyElement::at((x, y)) + PathElement::new(outline, edge_style),
            ))
            .map_err(err)?;
    }
    Ok(())
}
