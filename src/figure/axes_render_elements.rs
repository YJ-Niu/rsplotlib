//! 数据元素渲染模块
//!
//! 处理所有 PlotElement 的绘制逻辑：线、散点、柱状图、填充、误差棒、饼图等。
//!
//! 主要 API：
//! - `render_elements()`: 遍历并渲染所有元素
//! - `draw_single_line()`: 绘制单条线段（用于 axhline/axvline/stem 等）

use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use std::cell::RefCell;

use crate::core::colors::{RgbColor, default_color, median, parse_color, to_plotters_color};
use crate::core::elements::PlotElement;
use crate::core::marker::draw_marker;
use crate::figure::axes::scale_font;
use crate::utils::font_stack;

thread_local! {
    /// SVG 后端虚线注入表。位图后端无原生 dash，需把折线切成一段段独立描边；
    /// 但 SVG/光栅化路径下这样做会让每段 dash 的亚像素相位、跨顶点点数各异，
    /// 光栅化后形状"随机不一致"。改为：SVG 分支把虚线画成**整条连续 polyline**，
    /// 再由 `render_svg_string` 给它注入原生 `stroke-dasharray`——连续描边使各段
    /// dash 相位连续、端点统一，像素形状规律一致（与 matplotlib SVG 输出一致）。
    ///
    /// 每条记录为 (stroke 颜色 hex, 首点像素坐标 x, y, dasharray 字符串)，用于在
    /// plotters 生成的 `<polyline .../>` 中精确定位对应虚线并注入属性。
    static SVG_DASH_INJECTS: RefCell<Vec<(String, i32, i32, String)>> = const { RefCell::new(Vec::new()) };
}

/// 清空 SVG 虚线注入表（每次 SVG 渲染前调用，避免跨次渲染残留）。
pub fn clear_svg_dash_injects() {
    SVG_DASH_INJECTS.with(|c| c.borrow_mut().clear());
}

/// 取出并清空 SVG 虚线注入表（SVG 渲染完成后调用）。
pub fn take_svg_dash_injects() -> Vec<(String, i32, i32, String)> {
    SVG_DASH_INJECTS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn push_svg_dash_inject(color_hex: String, x0: i32, y0: i32, dasharray: String) {
    SVG_DASH_INJECTS.with(|c| c.borrow_mut().push((color_hex, x0, y0, dasharray)));
}

/// 绘制单条线段（用于 axhline/axvline/stem 等）
pub fn draw_single_line<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: RgbColor,
    lw: f64,
    font_scale: f64,
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
    chart
        .draw_series(std::iter::once(PathElement::new(
            vec![(x1, y1), (x2, y2)],
            style,
        )))
        .map_err(|e| PyRuntimeError::new_err(format!("Line: {}", e)))?;
    Ok(())
}

/// 绘制等宽折线（实线）。
///
/// 逐段绘制四边形 + 顶点处圆填充，保证任意斜率下线宽一致。
/// 每个线段在数据坐标中构造精确法线方向的四边形，顶点处用圆形成 round join，
/// 避免 plotters 原生粗线对法线偏移取整导致的粗细不均问题。
#[allow(dead_code)]
fn draw_thick_polyline<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    points: &[(f64, f64)],
    rgb: &plotters::style::RGBColor,
    width_px: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if points.len() < 2 {
        return Ok(());
    }
    let (pw, ph) = {
        let dim = chart.plotting_area().dim_in_pixel();
        (dim.0 as f64, dim.1 as f64)
    };
    let x_per_pix = if pw > 0.0 { (x_max - x_min) / pw } else { 1.0 };
    let y_per_pix = if ph > 0.0 { (y_max - y_min) / ph } else { 1.0 };
    let half = width_px / 2.0;
    let style: ShapeStyle = rgb.filled();

    let n = points.len();

    // 计算每个线段的单位方向（屏幕空间）和法线
    let mut seg_dirs: Vec<(f64, f64)> = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let dx_data = points[i + 1].0 - points[i].0;
        let dy_data = points[i + 1].1 - points[i].1;
        let dx_px = if x_per_pix != 0.0 {
            dx_data / x_per_pix
        } else {
            0.0
        };
        let dy_px = if y_per_pix != 0.0 {
            dy_data / y_per_pix
        } else {
            0.0
        };
        let len = (dx_px * dx_px + dy_px * dy_px).sqrt();
        if len < 1e-9 {
            seg_dirs.push((1.0, 0.0));
        } else {
            seg_dirs.push((dx_px / len, dy_px / len));
        }
    }

    // 计算每个线段的四个顶点（四边形），在数据坐标中构造
    // 法线方向：(-ty, tx)，即左侧法线
    let mut quads: Vec<Vec<(f64, f64)>> = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let (tx, ty) = seg_dirs[i];
        let nx = -ty;
        let ny = tx;
        let (x0, y0) = points[i];
        let (x1, y1) = points[i + 1];

        let dx_n = nx * half * x_per_pix;
        let dy_n = ny * half * y_per_pix;

        quads.push(vec![
            (x0 + dx_n, y0 + dy_n),
            (x1 + dx_n, y1 + dy_n),
            (x1 - dx_n, y1 - dy_n),
            (x0 - dx_n, y0 - dy_n),
        ]);
    }

    // 绘制所有线段四边形
    chart
        .draw_series(quads.iter().map(|v| Polygon::new(v.clone(), style)))
        .map_err(|e| PyRuntimeError::new_err(format!("Thick polyline segs: {}", e)))?;

    // 在每个顶点处绘制填充圆，形成 round join 并填补相邻四边形之间的缝隙。
    // plotters 的 Circle 半径以「像素」为单位（与 marker 一致），因此直接用 half，
    // 半径正好等于线的半宽，圆不会超出 ribbon 宽度，保证线宽处处一致。
    chart
        .draw_series(
            points
                .iter()
                .map(|&(x, y)| Circle::new((x, y), half, style)),
        )
        .map_err(|e| PyRuntimeError::new_err(format!("Thick polyline joins: {}", e)))?;

    Ok(())
}

thread_local! {
    /// 复用的 AA "到折线笔画最小距离" 缓冲 + 本次写入过的像素下标列表。
    ///
    /// dist 缓冲长度覆盖整块绘图区、且始终保持全 +∞（哨兵，表示"未被触及"）；
    /// 每条折线只写入被覆盖的像素，并把这些像素下标记录到 touched，绘制后逐一
    /// 复位为 +∞。这样避免了「每个折线元素都重新分配并清零整块绘图区」的巨大
    /// 开销——单张主图有数百条折线时，这正是渲染的主要瓶颈。
    static AA_SCRATCH: std::cell::RefCell<(Vec<f32>, Vec<usize>)> =
        const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

/// 抗锯齿等宽折线渲染（仅位图后端）。
///
/// 对折线包围盒内的每个像素，计算像素中心到线段的距离，用覆盖率（coverage）
/// 做 alpha 混合。这样任意斜率下线宽都严格一致、边缘平滑，从根本上消除
/// 无抗锯齿多边形填充导致的"阶梯状粗细不均"。相邻线段用 clamped 距离自然
/// 形成圆角连接（round join）；两端按 capstyle 处理 butt/round/projecting。
#[allow(clippy::too_many_arguments)]
fn draw_thick_polyline_aa<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    points: &[(f64, f64)],
    rgb: &plotters::style::RGBColor,
    width_px: f64,
    capstyle: &str,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if points.len() < 2 {
        return Ok(());
    }
    let (pw, ph) = {
        let dim = chart.plotting_area().dim_in_pixel();
        (dim.0 as f64, dim.1 as f64)
    };
    if pw < 1.0 || ph < 1.0 {
        return Ok(());
    }
    let x_per_pix = (x_max - x_min) / pw;
    let y_per_pix = (y_max - y_min) / ph;
    if x_per_pix == 0.0 || y_per_pix == 0.0 {
        return Ok(());
    }
    // 数据坐标 -> 绘图区局部连续像素坐标（y 轴翻转：y_max 对应 v=0）
    let to_px =
        |p: (f64, f64)| -> (f64, f64) { ((p.0 - x_min) / x_per_pix, (y_max - p.1) / y_per_pix) };
    let pts: Vec<(f64, f64)> = points.iter().map(|&p| to_px(p)).collect();

    let half = width_px / 2.0;
    // 覆盖率 = 到折线笔画的距离经过 1px 线性斜坡：cov = clamp(half + 0.5 - dist, 0, 1)。
    // 对直线段而言，这与"每像素 8×8 子采样求面积占比"在数学上等价（面积覆盖对直边
    // 就是该线性斜坡），但每像素只需一次距离计算，省去 64× 子采样开销。
    // 为消除折线顶点处"一侧 50% + 另一侧 50% 被错误合成为 50%"的掐细拐折，
    // 逐段写入的是「像素中心到该段的距离」，并对所有段取 **最小距离**（= 到整条
    // 折线笔画的真实距离），拐点自然形成 round join、线宽处处一致。
    let reach = half + 1.0;
    let cov_reach = half + 0.5; // 距离 >= 此值覆盖率为 0

    let pw_u = pw as usize;
    let ph_u = ph as usize;
    // dist_buf 从线程本地 scratch 借出复用（见 AA_SCRATCH），语义为"到折线笔画的
    // 最小距离"，哨兵值 +∞ 表示未被本条折线触及。避免每个折线元素都重新分配/清零
    // 整块绘图区。
    let (mut dist_buf, mut touched) = AA_SCRATCH.with(|c| {
        let mut g = c.borrow_mut();
        (std::mem::take(&mut g.0), std::mem::take(&mut g.1))
    });
    if dist_buf.len() < pw_u * ph_u {
        dist_buf.resize(pw_u * ph_u, f32::INFINITY);
    }
    touched.clear();

    let cap_round = capstyle.eq_ignore_ascii_case("round");
    let cap_proj = capstyle.eq_ignore_ascii_case("projecting");

    let n = pts.len();
    for i in 0..n - 1 {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[i + 1];
        let dx = bx - ax;
        let dy = by - ay;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            continue;
        }
        let ux = dx / len;
        let uy = dy / len;
        let start_cap = i == 0;
        let end_cap = i == n - 2;

        let min_x = (ax.min(bx) - reach).floor().max(0.0) as usize;
        let max_x = (ax.max(bx) + reach).ceil().min(pw - 1.0) as usize;
        let min_y = (ay.min(by) - reach).floor().max(0.0) as usize;
        let max_y = (ay.max(by) + reach).ceil().min(ph - 1.0) as usize;

        // 像素中心到本段的距离（含端点 cap / 内部 round join 处理）
        let dist_at = |px: f64, py: f64| -> f64 {
            let s = (px - ax) * ux + (py - ay) * uy;
            let perp = ((px - ax) * (-uy) + (py - ay) * ux).abs();
            if s < 0.0 {
                if start_cap {
                    if cap_round {
                        (s * s + perp * perp).sqrt()
                    } else if cap_proj {
                        if s >= -half { perp } else { f64::INFINITY }
                    } else {
                        f64::INFINITY // butt：垂直切断
                    }
                } else {
                    (s * s + perp * perp).sqrt() // 内部连接：round join
                }
            } else if s > len {
                let over = s - len;
                if end_cap {
                    if cap_round {
                        (over * over + perp * perp).sqrt()
                    } else if cap_proj {
                        if over <= half { perp } else { f64::INFINITY }
                    } else {
                        f64::INFINITY
                    }
                } else {
                    (over * over + perp * perp).sqrt()
                }
            } else {
                perp
            }
        };

        for yy in min_y..=max_y {
            let cy = yy as f64 + 0.5;
            let row = yy * pw_u;
            for xx in min_x..=max_x {
                let cx = xx as f64 + 0.5;
                let d = dist_at(cx, cy);
                if d >= cov_reach {
                    continue;
                }
                let idx = row + xx;
                let df = d as f32;
                let cur = dist_buf[idx];
                if cur.is_infinite() {
                    touched.push(idx);
                    dist_buf[idx] = df;
                } else if df < cur {
                    dist_buf[idx] = df;
                }
            }
        }
    }

    // 直接以「后端整数像素」坐标绘制，绕过 plotters 的数据->像素映射
    // （其 floor(actual*logic+1e-3) 会偶发把两个相邻局部像素映射到同一图像
    // 列、留空相邻列，造成"偶尔 1 像素错位"）。strip_coord_spec 给出的
    // Shift 坐标是「加上绘图区左上角原点」的纯平移，局部像素 (xx,yy) 精确
    // 对应后端像素，与覆盖率所用的局部网格一一对应。
    //
    // 仅遍历本次写入过的像素（touched），既完成绘制又顺便把 dist_buf 复位为 +∞，
    // 开销与被覆盖像素数成正比，而非与整块绘图区成正比。复位对每个 touched 下标
    // 都执行（即使绘制中途出错），以保证 dist_buf 归还 scratch 时仍为全 +∞。
    let area = chart.plotting_area().strip_coord_spec();
    let mut draw_res: PyResult<()> = Ok(());
    let cov_reach_f = cov_reach as f32;
    for &idx in touched.iter() {
        let d = dist_buf[idx];
        dist_buf[idx] = f32::INFINITY;
        if draw_res.is_ok() {
            let cov = (cov_reach_f - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                let xx = (idx % pw_u) as i32;
                let yy = (idx / pw_u) as i32;
                let color = rgb.mix(cov as f64);
                if let Err(e) = area.draw_pixel((xx, yy), &color) {
                    draw_res = Err(PyRuntimeError::new_err(format!("AA polyline: {}", e)));
                }
            }
        }
    }
    touched.clear();
    AA_SCRATCH.with(|c| {
        let mut g = c.borrow_mut();
        g.0 = std::mem::take(&mut dist_buf);
        g.1 = std::mem::take(&mut touched);
    });
    draw_res
}

/// 沿折线按「显示像素」度量的图案切分出各段 dash 折线（纯几何，不做绘制）。
///
/// `pattern` 是 (段长像素, 是否绘制) 的循环序列，例如 dashed = [(dash,true),(gap,false)]。
/// 关键点：段长以**像素**度量（用 `x_per_pix` / `y_per_pix` 把数据坐标增量换算为像素），
/// 因此划线长度与坐标轴量程无关，和 matplotlib 一致。返回的每个 `Vec` 是一段 dash 的
/// 折点（数据坐标），一段 dash 可跨越多段折线：在顶点处继续累积当前 dash 的折点。
///
/// 切分与渲染解耦后，每段 dash 都能走与实线相同的高质量渲染路径（位图 AA / SVG 原生描边），
/// 从而线宽处处一致、沿线方向绘制——消除此前把每段 dash 交给 plotters 原生粗线渲染时，
/// 短斜线出现的 Z 形 / 星形 / 粗细不均。
fn compute_dash_segments(
    points: &[(f64, f64)],
    pattern: &[(f64, bool)],
    x_per_pix: f64,
    y_per_pix: f64,
) -> Vec<Vec<(f64, f64)>> {
    let mut out: Vec<Vec<(f64, f64)>> = Vec::new();
    if points.len() < 2 || pattern.is_empty() {
        return out;
    }

    let mut pat_idx = 0usize;
    let mut pat_remain = pattern[0].0.max(1e-6);
    let mut drawing = pattern[0].1;
    // 当前正在累积的一段 dash 折点（数据坐标），可跨多段折线累积
    let mut cur: Vec<(f64, f64)> = Vec::new();

    for i in 0..points.len() - 1 {
        let (ax, ay) = points[i];
        let (bx, by) = points[i + 1];
        let ddx = bx - ax;
        let ddy = by - ay;
        let seg_px = {
            let px = ddx / x_per_pix;
            let py = ddy / y_per_pix;
            (px * px + py * py).sqrt()
        };
        if seg_px < 1e-12 {
            continue;
        }
        let mut consumed = 0.0f64; // 本段已消耗像素
        while seg_px - consumed > 1e-9 {
            let step = (seg_px - consumed).min(pat_remain);
            if drawing {
                if cur.is_empty() {
                    let t0 = consumed / seg_px;
                    cur.push((ax + ddx * t0, ay + ddy * t0));
                }
                let t1 = (consumed + step) / seg_px;
                cur.push((ax + ddx * t1, ay + ddy * t1));
            }
            consumed += step;
            pat_remain -= step;
            if pat_remain <= 1e-9 {
                if drawing && cur.len() >= 2 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
                pat_idx = (pat_idx + 1) % pattern.len();
                pat_remain = pattern[pat_idx].0.max(1e-6);
                drawing = pattern[pat_idx].1;
            }
        }
    }
    if drawing && cur.len() >= 2 {
        out.push(cur);
    }
    out
}

/// 统一的折线渲染入口：实线与每一段 dash 都经此绘制，保证任意斜率线宽一致、沿线方向。
///
/// - 位图后端：走逐像素覆盖率抗锯齿 `draw_thick_polyline_aa`（内部含 round join 与端点处理）；
/// - SVG 后端：走原生连续描边 `PathElement`（描边端点 / 连接在导出时统一注入为 round）。
///
/// 相比直接用 plotters 原生 `stroke_width` 渲染短斜线（法线偏移取整导致 Z 形 / 星形 /
/// 粗细不均），此入口对短 dash 同样保持均匀线宽。
#[allow(clippy::too_many_arguments)]
fn render_polyline<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    points: &[(f64, f64)],
    rgb: &plotters::style::RGBColor,
    width_px: f64,
    capstyle: &str,
    bitmap: bool,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if points.len() < 2 {
        return Ok(());
    }
    if bitmap {
        draw_thick_polyline_aa(
            chart,
            points,
            rgb,
            width_px.max(1.0),
            capstyle,
            x_min,
            x_max,
            y_min,
            y_max,
        )
    } else {
        let sw = width_px.round().max(1.0) as u32;
        let style_native: ShapeStyle = rgb.stroke_width(sw);
        chart
            .draw_series(std::iter::once(PathElement::new(
                points.to_vec(),
                style_native,
            )))
            .map_err(|e| PyRuntimeError::new_err(format!("Native line: {}", e)))?;
        Ok(())
    }
}

/// 网格分层。
///
/// matplotlib 默认 `axes.axisbelow = 'line'`：网格线（zorder≈1.5）绘制在填充
/// patch/collection（zorder≈1，如 bar/hist 柱、fill_between、stackplot、scatter、
/// axhspan/axvspan）之上，但在折线（Line2D，zorder=2，如 plot、hist 的 step 轮廓）之下。
/// 因此渲染分两趟：先画 `BelowGrid` 元素 → 画网格 → 再画 `AboveGrid` 元素。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GridLayer {
    BelowGrid,
    AboveGrid,
}

/// 判断某元素（Hist 除外）应绘制在网格下方（true）还是上方（false）。
///
/// 与 matplotlib 各 artist 默认 zorder 对齐：填充 patch/collection（zorder=1）在网格下，
/// 其余（Line2D、Text 等，zorder≥2）在网格上。Hist 需跨两趟（柱在下、step 轮廓在上），
/// 由其分支内部按 layer 分别处理，不经过此函数。
fn element_below_grid(el: &PlotElement) -> bool {
    matches!(
        el,
        PlotElement::Bar { .. }
            | PlotElement::BarH { .. }
            | PlotElement::FillBetween { .. }
            | PlotElement::Stack { .. }
            | PlotElement::HSpan { .. }
            | PlotElement::VSpan { .. }
            | PlotElement::Scatter { .. }
            | PlotElement::ScatterMulti { .. }
    )
}

/// 渲染所有 PlotElement（按网格分层 `layer` 只绘制归属该层的元素）
///
/// # 参数
/// - `chart`: plotters 的 chart 上下文
/// - `elements`: 所有 plot 调用收集的元素
/// - `layer`: 当前绘制的网格分层（BelowGrid/AboveGrid）
/// - `font_scale`: 字体缩放系数
/// - `xlog`, `ylog`: 是否对数刻度
/// - `x_min`, `x_max`, `y_min`, `y_max`: 数据范围
#[allow(clippy::too_many_arguments)]
pub fn render_elements<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    elements: &[PlotElement],
    layer: GridLayer,
    font_scale: f64,
    marker_scale: f64,
    xlog: bool,
    ylog: bool,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    bitmap: bool,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    // log 刻度坐标转换闭包
    let tx = |v: f64| {
        if xlog {
            if v > 0.0 {
                v.log10()
            } else {
                f64::NEG_INFINITY
            }
        } else {
            v
        }
    };
    let ty = |v: f64| {
        if ylog {
            if v > 0.0 {
                v.log10()
            } else {
                f64::NEG_INFINITY
            }
        } else {
            v
        }
    };

    for el in elements {
        // matplotlib 默认 axisbelow='line'：按元素归属的网格分层跳过不属于本趟的元素。
        // Hist 例外——柱在网格下、step 轮廓在网格上，其分支内部各自按 layer 判断。
        if !matches!(el, PlotElement::Hist { .. })
            && element_below_grid(el) != (layer == GridLayer::BelowGrid)
        {
            continue;
        }
        match el {
            PlotElement::Line {
                x,
                y,
                color,
                linestyle,
                marker,
                linewidth,
                color_idx,
                solid_capstyle,
                markersize,
                markerfacecolor,
                markeredgecolor,
                ..
            } => {
                let col = parse_color(color, *color_idx).unwrap_or(default_color(*color_idx));
                let rgb = to_plotters_color(col);
                // 折线线宽增加 50%
                let lw_scaled = *linewidth * 1.5;
                let linewidth = &lw_scaled;
                // plotters 的坐标映射对屏幕像素取 floor，会让点整体偏高约 0.5 像素。
                // 向下偏移半个像素，等效为四舍五入，使线/marker 的中心落在坐标点上。
                let y_half_px = {
                    let dim = chart.plotting_area().dim_in_pixel();
                    let ph = dim.1 as f64;
                    if ph > 0.0 {
                        (y_max - y_min) / ph * 0.5
                    } else {
                        0.0
                    }
                };
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
                                    current.push((txv, tyv - y_half_px));
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
                            let lw_px = ((*linewidth) * font_scale).max(1.0).round() as u32;
                            // 虚线图案按「显示像素」度量：matplotlib 图案(points) × 名义线宽 × (dpi/72)。
                            let (pw_dash, ph_dash) = {
                                let dim = chart.plotting_area().dim_in_pixel();
                                (dim.0 as f64, dim.1 as f64)
                            };
                            let x_per_pix = if pw_dash > 0.0 {
                                (x_max - x_min) / pw_dash
                            } else {
                                1.0
                            };
                            let y_per_pix = if ph_dash > 0.0 {
                                (y_max - y_min) / ph_dash
                            } else {
                                1.0
                            };
                            // 撤销前面 1.5x 线宽膨胀，dash 图案按 matplotlib 名义线宽缩放；font_scale = dpi/72
                            let lw_nominal = (*linewidth / 1.5).max(0.1);
                            let ds = lw_nominal * font_scale; // 1 图案单位(point) -> 像素
                            let width_px = (lw_px as i32 - 1).max(1) as f64;
                            // matplotlib 默认 dash 图案 (rcParams)，None 表示实线
                            let dash_pattern: Option<Vec<(f64, bool)>> = match linestyle.as_str() {
                                // dashed (lines.dashed_pattern): 划 3.7, 隙 1.6
                                "--" => Some(vec![(3.7 * ds, true), (1.6 * ds, false)]),
                                // dotted (lines.dotted_pattern): 点 1, 隙 1.65
                                ":" => Some(vec![(1.0 * ds, true), (1.65 * ds, false)]),
                                // dashdot (lines.dashdot_pattern): 划 6.4, 隙 1.6, 点 1, 隙 1.6
                                "-." => Some(vec![
                                    (6.4 * ds, true),
                                    (1.6 * ds, false),
                                    (1.0 * ds, true),
                                    (1.6 * ds, false),
                                ]),
                                _ => None,
                            };
                            if let Some(pattern) = dash_pattern {
                                if bitmap {
                                    // 位图后端无原生 dash：按像素图案把折线切成一段段 dash（沿线方向），
                                    // 每段走与实线相同的 AA 渲染入口，线宽处处一致、无 Z 形 / 星形。
                                    // dash 端点用 "butt"（matplotlib 默认 dash_capstyle）。
                                    let dashes = compute_dash_segments(
                                        points, &pattern, x_per_pix, y_per_pix,
                                    );
                                    for dash in &dashes {
                                        render_polyline(
                                            chart, dash, &rgb, width_px, "butt", bitmap, x_min,
                                            x_max, y_min, y_max,
                                        )?;
                                    }
                                } else {
                                    // SVG 后端：画**整条连续** polyline（butt 端点），再由
                                    // render_svg_string 注入原生 stroke-dasharray。连续描边让每段
                                    // dash 相位连续、端点统一，像素形状规律一致（不再"随机"）。
                                    render_polyline(
                                        chart, points, &rgb, width_px, "butt", bitmap, x_min,
                                        x_max, y_min, y_max,
                                    )?;
                                    // 记录注入信息：首点像素坐标（= plotters 写入 polyline 的整数坐标）
                                    // + stroke 颜色 hex + dasharray（图案长度序列，单位为显示像素）。
                                    let (x0, y0) = chart.backend_coord(&points[0]);
                                    let color_hex =
                                        format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2);
                                    let dasharray = pattern
                                        .iter()
                                        .map(|(len, _)| format!("{:.2}", len))
                                        .collect::<Vec<_>>()
                                        .join(",");
                                    push_svg_dash_inject(color_hex, x0, y0, dasharray);
                                }
                            } else {
                                // 实线：中心已对齐坐标点（见 y_half_px）。
                                render_polyline(
                                    chart,
                                    points,
                                    &rgb,
                                    width_px,
                                    solid_capstyle,
                                    bitmap,
                                    x_min,
                                    x_max,
                                    y_min,
                                    y_max,
                                )?;
                            }
                        }
                    }
                }
                if let Some(marker_name) = marker
                    && !marker_name.is_empty()
                    && x.len() == y.len()
                {
                    let col2 = parse_color(color, *color_idx).unwrap_or(default_color(*color_idx));
                    let line_rgb = to_plotters_color(col2);
                    // markerfacecolor / markeredgecolor 缺省时都跟随线条颜色 (matplotlib 'auto')
                    let face_rgb = markerfacecolor
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .and_then(|s| parse_color(s, *color_idx).ok())
                        .map(to_plotters_color)
                        .unwrap_or(line_rgb);
                    let edge_rgb = markeredgecolor
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .and_then(|s| parse_color(s, *color_idx).ok())
                        .map(to_plotters_color)
                        .unwrap_or(line_rgb);
                    // matplotlib markersize 单位是 points；marker 包围盒边长(像素) = markersize * dpi/72。
                    // 使用 marker_scale (= 真实 dpi/72) 计算，与字体/线宽的 font_scale 解耦，
                    // 保证 markersize 只影响 marker 大小。
                    // draw_marker 收到的 `size` 是「半边长 / 半径」= 包围盒边长 / 2。
                    let marker_size = if marker_name == "," {
                        // matplotlib 像素点 marker：约 1 设备像素
                        1.0
                    } else if marker_name == "." {
                        // matplotlib 点 marker：直径 = 0.5 * markersize（point_size_reduction=0.5），
                        // 故半径 = 0.25 * markersize_px
                        let ms = markersize.unwrap_or(6.0_f64).max(0.01);
                        0.25 * ms * marker_scale
                    } else {
                        // markersize: None => matplotlib 默认 6
                        let ms = markersize.unwrap_or(6.0_f64).max(0.01);
                        let diameter_px = ms * marker_scale;
                        diameter_px / 2.0
                    };
                    for (xv, yv) in x.iter().zip(y.iter()) {
                        if let (Some(xv), Some(yv)) = (xv, yv) {
                            let txv = tx(*xv);
                            let tyv = ty(*yv) - y_half_px;
                            if txv.is_finite() && tyv.is_finite() {
                                draw_marker(
                                    chart,
                                    marker_name,
                                    txv,
                                    tyv,
                                    marker_size,
                                    face_rgb,
                                    edge_rgb,
                                    1.0,
                                )
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("Failed to draw marker: {}", e))
                                })?;
                            }
                        }
                    }
                }
            }
            PlotElement::Scatter {
                x,
                y,
                s,
                c,
                marker,
                alpha,
                color_idx,
                ..
            } => {
                let col = parse_color(c, *color_idx).unwrap_or(default_color(*color_idx));
                let rgb = to_plotters_color(col);
                // matplotlib: s 是 marker 面积 (points²)，故直径 = sqrt(s) points，
                // 像素直径 = sqrt(s) * marker_scale；draw_marker 的 size 是半径。
                let size = (s.sqrt() * marker_scale / 2.0).max(1.0);
                for (&xv, &yv) in x.iter().zip(y.iter()) {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if txv.is_finite() && tyv.is_finite() {
                        draw_marker(chart, marker, txv, tyv, size, rgb, rgb, *alpha).map_err(
                            |e| PyRuntimeError::new_err(format!("Failed to draw scatter: {}", e)),
                        )?;
                    }
                }
            }
            PlotElement::ScatterMulti {
                x,
                y,
                s_list,
                c_list,
                marker,
                alpha,
                color_idx,
                ..
            } => {
                if x.is_empty() || y.is_empty() {
                    continue;
                }
                for (i, (&xv, &yv)) in x.iter().zip(y.iter()).enumerate() {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if !txv.is_finite() || !tyv.is_finite() {
                        continue;
                    }
                    let c_str = c_list
                        .as_ref()
                        .and_then(|c| c.get(i).cloned())
                        .unwrap_or_default();
                    let col = if c_str.is_empty() {
                        parse_color("", *color_idx).unwrap_or(default_color(*color_idx + i))
                    } else {
                        parse_color(&c_str, *color_idx + i).unwrap_or(default_color(*color_idx + i))
                    };
                    let rgb = to_plotters_color(col);
                    let size = (s_list
                        .as_ref()
                        .and_then(|s| s.get(i).cloned())
                        .unwrap_or(100.0)
                        .sqrt()
                        * marker_scale
                        / 2.0)
                        .max(1.0);
                    draw_marker(chart, marker, txv, tyv, size, rgb, rgb, *alpha).map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to draw scatter_multi: {}", e))
                    })?;
                }
            }
            PlotElement::Bar {
                x,
                height,
                width,
                colors,
                color_idx,
                ..
            } => {
                for (i, (&xv, &h)) in x.iter().zip(height.iter()).enumerate() {
                    let col = match colors.get(i) {
                        Some(s) if !s.is_empty() => {
                            parse_color(s, *color_idx + i).unwrap_or(default_color(*color_idx + i))
                        }
                        _ => default_color(*color_idx),
                    };
                    let fill_style: ShapeStyle = to_plotters_color(col).filled();
                    let txv = tx(xv);
                    let th = ty(h);
                    let y0 = if ylog {
                        f64::NEG_INFINITY
                    } else {
                        0.0f64.max(y_min)
                    };
                    if txv.is_finite() && th.is_finite() {
                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [(txv - width / 2.0, y0), (txv + width / 2.0, th)],
                                fill_style,
                            )))
                            .map_err(|e| {
                                PyRuntimeError::new_err(format!("Failed to draw bar: {}", e))
                            })?;
                    }
                }
            }
            PlotElement::BarH {
                y,
                width,
                height,
                colors,
                color_idx,
                ..
            } => {
                for (i, (&yv, &wv)) in y.iter().zip(width.iter()).enumerate() {
                    let col = match colors.get(i) {
                        Some(s) if !s.is_empty() => {
                            parse_color(s, *color_idx + i).unwrap_or(default_color(*color_idx + i))
                        }
                        _ => default_color(*color_idx),
                    };
                    let fill_style: ShapeStyle = to_plotters_color(col).filled();
                    let tyv = ty(yv);
                    let twv = tx(wv);
                    let bar_y0 = tyv - height / 2.0;
                    let bar_y1 = tyv + height / 2.0;
                    chart
                        .draw_series(std::iter::once(Rectangle::new(
                            [(0.0, bar_y0), (twv, bar_y1)],
                            fill_style,
                        )))
                        .map_err(|e| {
                            PyRuntimeError::new_err(format!("Failed to draw barh: {}", e))
                        })?;
                }
            }
            PlotElement::Hist {
                bars,
                outlines,
                histtype,
                orientation,
                alpha,
                colors,
                color_idx,
                label: _,
            } => {
                let is_horizontal = orientation == "horizontal";
                // 将 (分箱位置轴, 计数轴) 映射到数据坐标 (x, y)，竖直时 pos->x,val->y，
                // 水平时交换；并套用各轴的 log 变换。
                let to_xy = |pos: f64, val: f64| -> (f64, f64) {
                    if is_horizontal {
                        (tx(val), ty(pos))
                    } else {
                        (tx(pos), ty(val))
                    }
                };
                let is_step = histtype == "step" || histtype == "stepfilled";
                let draw_fill = histtype != "step";
                let n_datasets = bars.len().max(outlines.len());
                for di in 0..n_datasets {
                    let col_str = colors.get(di).map(|s| s.as_str()).unwrap_or("");
                    let col = parse_color(col_str, *color_idx + di)
                        .unwrap_or(default_color(*color_idx + di));
                    let rgb = to_plotters_color(col);
                    let fill_style: ShapeStyle = rgb.mix(*alpha).filled();

                    // 柱（填充 patch，zorder=1）→ 网格下方
                    if layer == GridLayer::BelowGrid
                        && draw_fill
                        && let Some(ds_bars) = bars.get(di)
                    {
                        for &(pl, pr, vb, vt) in ds_bars {
                            if (vt - vb).abs() < 1e-12 {
                                continue;
                            }
                            let c1 = to_xy(pl, vb);
                            let c2 = to_xy(pr, vt);
                            chart
                                .draw_series(std::iter::once(Rectangle::new([c1, c2], fill_style)))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("Failed to draw hist: {}", e))
                                })?;
                        }
                    }

                    // step 轮廓（Line2D，zorder=2）→ 网格上方
                    if layer == GridLayer::AboveGrid
                        && is_step
                        && let Some(pts) = outlines.get(di)
                    {
                        let mapped: Vec<(f64, f64)> =
                            pts.iter().map(|&(p, v)| to_xy(p, v)).collect();
                        let lw_px = (1.5 * font_scale).max(1.0).round() as u32;
                        let stroke_w = (lw_px as i32 - 1).max(1) as u32;
                        let outline_style = rgb.stroke_width(stroke_w);
                        chart
                            .draw_series(std::iter::once(PathElement::new(mapped, outline_style)))
                            .map_err(|e| {
                                PyRuntimeError::new_err(format!(
                                    "Failed to draw hist outline: {}",
                                    e
                                ))
                            })?;
                    }
                }
            }
            PlotElement::Image { pixels, alpha } => {
                if pixels.is_empty() || pixels[0].is_empty() {
                    continue;
                }
                for (r, row) in pixels.iter().enumerate() {
                    for (c, &(pr, pg, pb)) in row.iter().enumerate() {
                        let style = plotters::style::RGBAColor(pr, pg, pb, *alpha).filled();
                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [(c as f64, r as f64), ((c + 1) as f64, (r + 1) as f64)],
                                style,
                            )))
                            .map_err(|e| {
                                PyRuntimeError::new_err(format!("Failed to draw image: {}", e))
                            })?;
                    }
                }
            }
            PlotElement::Text {
                x,
                y,
                text,
                fontsize,
                color,
                font_family,
            } => {
                let txv = tx(*x);
                let tyv = ty(*y);
                if !txv.is_finite() || !tyv.is_finite() {
                    continue;
                }
                let fs = scale_font(*fontsize, font_scale);
                let family_name = font_stack::resolve_font_family(text, font_family.as_deref());
                let font: FontDesc = (family_name.as_str(), fs).into();
                let colored_font = font.color(&to_plotters_color(*color));
                // 垂直居中对齐：让 (x, y) 落在文字的几何中心，
                // 与 axhline/axvline 在同一坐标时的视觉位置一致。
                let text_style = colored_font.pos(Pos::new(HPos::Left, VPos::Center));
                chart
                    .draw_series(std::iter::once(plotters::element::Text::new(
                        text.to_string(),
                        (txv, tyv),
                        text_style,
                    )))
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw text: {}", e)))?;
            }
            PlotElement::HLine {
                y,
                color,
                linewidth,
                color_idx,
                ..
            } => {
                let tyv = ty(*y);
                if !tyv.is_finite() {
                    continue;
                }
                let col = parse_color(color, *color_idx).unwrap_or(RgbColor(0, 0, 0));
                draw_single_line(chart, x_min, tyv, x_max, tyv, col, *linewidth, font_scale)?;
            }
            PlotElement::VLine {
                x,
                color,
                linewidth,
                color_idx,
                ..
            } => {
                let txv = tx(*x);
                if !txv.is_finite() {
                    continue;
                }
                let col = parse_color(color, *color_idx).unwrap_or(RgbColor(0, 0, 0));
                draw_single_line(chart, txv, y_min, txv, y_max, col, *linewidth, font_scale)?;
            }
            PlotElement::HSpan {
                y1,
                y2,
                color,
                alpha,
            } => {
                let ty1 = ty(*y1);
                let ty2 = ty(*y2);
                if !ty1.is_finite() || !ty2.is_finite() {
                    continue;
                }
                let col = parse_color(color, 0).unwrap_or(RgbColor(128, 128, 128));
                let rgb = to_plotters_color(col);
                let top = ty1.max(ty2);
                let bottom = ty1.min(ty2);
                chart
                    .draw_series(std::iter::once(Rectangle::new(
                        [(x_min, bottom), (x_max, top)],
                        rgb.mix(*alpha).filled(),
                    )))
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to draw axhspan: {}", e))
                    })?;
            }
            PlotElement::VSpan {
                x1,
                x2,
                color,
                alpha,
            } => {
                let tx1 = tx(*x1);
                let tx2 = tx(*x2);
                if !tx1.is_finite() || !tx2.is_finite() {
                    continue;
                }
                let col = parse_color(color, 0).unwrap_or(RgbColor(128, 128, 128));
                let rgb = to_plotters_color(col);
                let left = tx1.min(tx2);
                let right = tx1.max(tx2);
                chart
                    .draw_series(std::iter::once(Rectangle::new(
                        [(left, y_min), (right, y_max)],
                        rgb.mix(*alpha).filled(),
                    )))
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to draw axvspan: {}", e))
                    })?;
            }
            PlotElement::AxLine {
                xy1,
                xy2,
                color,
                linestyle,
                linewidth,
            } => {
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
            PlotElement::Arrow {
                x1,
                y1,
                x2,
                y2,
                color,
                linewidth,
                head_size,
            } => {
                let col = parse_color(color, 0).unwrap_or(RgbColor(0, 0, 0));
                let tx1 = tx(*x1);
                let ty1 = ty(*y1);
                let tx2 = tx(*x2);
                let ty2 = ty(*y2);
                if !(tx1.is_finite() && ty1.is_finite() && tx2.is_finite() && ty2.is_finite()) {
                    continue;
                }
                draw_single_line(chart, tx1, ty1, tx2, ty2, col, *linewidth, font_scale)?;
                // 箭头头部：简单三角形
                let dx = tx2 - tx1;
                let dy = ty2 - ty1;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-10 {
                    continue;
                }
                let nx = dx / len;
                let ny = dy / len;
                let head = *head_size;
                let p1 = (tx2, ty2);
                let p2 = (
                    tx2 - head * nx - head * 0.5 * ny,
                    ty2 - head * ny + head * 0.5 * nx,
                );
                let p3 = (
                    tx2 - head * nx + head * 0.5 * ny,
                    ty2 - head * ny - head * 0.5 * nx,
                );
                let rgb = to_plotters_color(col);
                chart
                    .draw_series(std::iter::once(Polygon::new(
                        vec![p1, p2, p3],
                        rgb.filled(),
                    )))
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw arrow: {}", e)))?;
            }
            PlotElement::Pie {
                x,
                labels,
                colors,
                autopct,
                startangle,
                explode,
            } => {
                let total: f64 = x.iter().sum();
                if total <= 0.0 {
                    continue;
                }
                // 使饼图呈正圆：按绘图区像素宽高比压缩 x/y 数据半径，
                // 让单位圆在两个方向上映射到相同的像素半径（等效 matplotlib 的 aspect='equal'）。
                let (pw, ph) = chart.plotting_area().dim_in_pixel();
                let px_per_x = if x_max > x_min {
                    pw as f64 / (x_max - x_min)
                } else {
                    1.0
                };
                let px_per_y = if y_max > y_min {
                    ph as f64 / (y_max - y_min)
                } else {
                    1.0
                };
                let s = px_per_x.min(px_per_y);
                let sx = if px_per_x > 0.0 { s / px_per_x } else { 1.0 };
                let sy = if px_per_y > 0.0 { s / px_per_y } else { 1.0 };
                let mut current_angle = startangle.to_radians();
                let pie_colors = colors
                    .as_ref()
                    .map(|c| c.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                for (i, &val) in x.iter().enumerate() {
                    if val <= 0.0 {
                        continue;
                    }
                    let angle = (val / total) * 360.0_f64;
                    let angle_rad = angle.to_radians();
                    let end_angle = current_angle + angle_rad;
                    let mid_angle = current_angle + angle_rad / 2.0;
                    // explode：沿扇形中线方向向外偏移 explode[i] 倍半径
                    let exp = explode
                        .as_ref()
                        .and_then(|e| e.get(i))
                        .copied()
                        .unwrap_or(0.0);
                    let ox = mid_angle.cos() * exp * sx;
                    let oy = mid_angle.sin() * exp * sy;
                    let col = if let Some(ref pc) = pie_colors {
                        let ci =
                            parse_color(pc.get(i).unwrap_or(&""), i).unwrap_or(default_color(i));
                        to_plotters_color(ci)
                    } else {
                        to_plotters_color(default_color(i))
                    };
                    let steps = ((angle_rad / 0.05).ceil() as usize).max(3);
                    let mut vertices = vec![(ox, oy)];
                    for j in 0..=steps {
                        let a = current_angle + (j as f64 / steps as f64) * angle_rad;
                        vertices.push((a.cos() * sx + ox, a.sin() * sy + oy));
                    }
                    chart
                        .draw_series(std::iter::once(Polygon::new(
                            vertices,
                            col.mix(0.85).filled(),
                        )))
                        .map_err(|e| {
                            PyRuntimeError::new_err(format!("Failed to draw pie: {}", e))
                        })?;
                    if let Some(lbls) = labels
                        && let Some(l) = lbls.get(i)
                    {
                        let label_r = 1.3;
                        let lx = mid_angle.cos() * label_r * sx + ox;
                        let ly = mid_angle.sin() * label_r * sy + oy;
                        // 使用 BLACK 让 font.color() 返回 TextStyle，再 .pos() 调整锚点
                        let pie_family = font_stack::select_family(l);
                        let pie_label_style: TextStyle =
                            FontDesc::from((pie_family.as_str(), scale_font(12.0, font_scale)))
                                .color(&BLACK)
                                .pos(Pos::new(HPos::Center, VPos::Center));
                        chart
                            .draw_series(std::iter::once(plotters::element::Text::new(
                                l.to_string(),
                                (lx, ly),
                                pie_label_style,
                            )))
                            .map_err(|e| {
                                PyRuntimeError::new_err(format!("Failed to draw pie label: {}", e))
                            })?;
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
                        let tx = mid_angle.cos() * text_r * sx + ox;
                        let ty = mid_angle.sin() * text_r * sy + oy;
                        let autopct_family = font_stack::select_family(&text);
                        let autopct_style: TextStyle =
                            FontDesc::from((autopct_family.as_str(), scale_font(11.0, font_scale)))
                                .color(&BLACK)
                                .pos(Pos::new(HPos::Center, VPos::Center));
                        chart
                            .draw_series(std::iter::once(plotters::element::Text::new(
                                text,
                                (tx, ty),
                                autopct_style,
                            )))
                            .map_err(|e| {
                                PyRuntimeError::new_err(format!("Failed to draw autopct: {}", e))
                            })?;
                    }
                    current_angle = end_angle;
                }
            }
            PlotElement::FillBetween {
                x,
                y1,
                y2,
                color,
                alpha,
                ..
            } => {
                let col = parse_color(color, 0).unwrap_or(RgbColor(0, 128, 0));
                let rgb = to_plotters_color(col);
                if x.len() != y1.len() || x.is_empty() {
                    continue;
                }
                let mut points: Vec<(f64, f64)> = Vec::with_capacity(x.len() * 2);
                for (&xv, &yv) in x.iter().zip(y1.iter()) {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if txv.is_finite() && tyv.is_finite() {
                        points.push((txv, tyv));
                    }
                }
                for i in (0..x.len()).rev() {
                    let y2v = if i < y2.len() { y2[i] } else { 0.0 };
                    let txv = tx(x[i]);
                    let tyv = ty(y2v);
                    if txv.is_finite() && tyv.is_finite() {
                        points.push((txv, tyv));
                    }
                }
                if points.len() < 3 {
                    continue;
                }
                chart
                    .draw_series(std::iter::once(Polygon::new(
                        points,
                        rgb.mix(*alpha).filled(),
                    )))
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to draw fill_between: {}", e))
                    })?;
            }
            PlotElement::Stack {
                x,
                y_series,
                colors,
                alpha,
                ..
            } => {
                if x.is_empty() || y_series.is_empty() {
                    continue;
                }
                // 计算累加值：从最底层开始绘制
                let mut cumulative: Vec<f64> = vec![0.0; x.len()];
                for (si, series) in y_series.iter().enumerate() {
                    let color_str = colors
                        .as_ref()
                        .and_then(|c| c.get(si).cloned())
                        .unwrap_or_default();
                    let col = parse_color(&color_str, 0).unwrap_or(default_color(si));
                    let rgb = to_plotters_color(col);
                    // 构造当前层的上下边界点
                    let mut points: Vec<(f64, f64)> = Vec::with_capacity(x.len() * 2);
                    for (i, &xv) in x.iter().enumerate() {
                        let upper = if i < series.len() {
                            cumulative[i] + series[i]
                        } else {
                            cumulative[i]
                        };
                        let txv = tx(xv);
                        let tyv = ty(upper);
                        if txv.is_finite() && tyv.is_finite() {
                            points.push((txv, tyv));
                        }
                    }
                    for i in (0..x.len()).rev() {
                        let txv = tx(x[i]);
                        let tyv = ty(cumulative[i]);
                        if txv.is_finite() && tyv.is_finite() {
                            points.push((txv, tyv));
                        }
                    }
                    // 累加
                    for (i, v) in series.iter().enumerate() {
                        if i < cumulative.len() {
                            cumulative[i] += v;
                        }
                    }
                    if points.len() < 3 {
                        continue;
                    }
                    chart
                        .draw_series(std::iter::once(Polygon::new(
                            points,
                            rgb.mix(*alpha).filled(),
                        )))
                        .map_err(|e| {
                            PyRuntimeError::new_err(format!("Failed to draw stack: {}", e))
                        })?;
                }
            }
            PlotElement::ErrorBar {
                x,
                y,
                yerr,
                xerr,
                fmt,
                color,
                capsize,
                ..
            } => {
                let idx = 0;
                let col = parse_color(color, idx).unwrap_or(default_color(idx));
                let rgb = to_plotters_color(col);
                let line_style: ShapeStyle = rgb.stroke_width(1);
                let cap_half = capsize / 2.0;
                for (i, (&xv, &yv)) in x.iter().zip(y.iter()).enumerate() {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if !txv.is_finite() || !tyv.is_finite() {
                        continue;
                    }
                    let ye = if let Some(vec) = yerr.as_ref() {
                        if i < vec.len() { vec[i] } else { 0.0_f64 }
                    } else {
                        0.0
                    };
                    let xe = if let Some(vec) = xerr.as_ref() {
                        if i < vec.len() { vec[i] } else { 0.0_f64 }
                    } else {
                        0.0
                    };
                    if ye != 0.0 {
                        let ty_lo = ty(yv - ye);
                        let ty_hi = ty(yv + ye);
                        if ty_lo.is_finite() && ty_hi.is_finite() {
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(txv, ty_lo), (txv, ty_hi)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("ErrorBar line: {}", e))
                                })?;
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(txv - cap_half, ty_lo), (txv + cap_half, ty_lo)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("ErrorBar cap: {}", e))
                                })?;
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(txv - cap_half, ty_hi), (txv + cap_half, ty_hi)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("ErrorBar cap: {}", e))
                                })?;
                        }
                    }
                    if xe != 0.0 {
                        let tx_lo = tx(xv - xe);
                        let tx_hi = tx(xv + xe);
                        if tx_lo.is_finite() && tx_hi.is_finite() {
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(tx_lo, tyv), (tx_hi, tyv)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("ErrorBar xline: {}", e))
                                })?;
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(tx_lo, tyv - cap_half), (tx_lo, tyv + cap_half)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e))
                                })?;
                            chart
                                .draw_series(std::iter::once(PathElement::new(
                                    vec![(tx_hi, tyv - cap_half), (tx_hi, tyv + cap_half)],
                                    line_style,
                                )))
                                .map_err(|e| {
                                    PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e))
                                })?;
                        }
                    }
                    if !fmt.is_empty() {
                        let marker_name = fmt;
                        draw_marker(chart, marker_name, txv, tyv, 3.0, rgb, rgb, 1.0).map_err(
                            |e| PyRuntimeError::new_err(format!("ErrorBar marker: {}", e)),
                        )?;
                    }
                }
            }
            PlotElement::Stem {
                x,
                y,
                linefmt,
                markerfmt,
                ..
            } => {
                let col = RgbColor(0, 0, 200);
                let rgb = to_plotters_color(col);
                let baseline = ty(0.0);
                if linefmt == "-" || linefmt.is_empty() {
                    let lw_px = (1.0 * font_scale).round().max(1.0) as u32;
                    let line_style: ShapeStyle = rgb.stroke_width(lw_px);
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if !txv.is_finite() || !tyv.is_finite() || !baseline.is_finite() {
                            continue;
                        }
                        chart
                            .draw_series(std::iter::once(PathElement::new(
                                vec![(txv, baseline), (txv, tyv)],
                                line_style,
                            )))
                            .map_err(|e| PyRuntimeError::new_err(format!("Stem line: {}", e)))?;
                    }
                } else {
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if !txv.is_finite() || !tyv.is_finite() || !baseline.is_finite() {
                            continue;
                        }
                        draw_single_line(chart, txv, baseline, txv, tyv, col, 1.0, font_scale)?;
                    }
                }
                for (&xv, &yv) in x.iter().zip(y.iter()) {
                    let txv = tx(xv);
                    let tyv = ty(yv);
                    if !txv.is_finite() || !tyv.is_finite() {
                        continue;
                    }
                    draw_marker(chart, markerfmt, txv, tyv, 5.0, rgb, rgb, 1.0)
                        .map_err(|e| PyRuntimeError::new_err(format!("Stem marker: {}", e)))?;
                }
            }
            PlotElement::Step {
                x,
                y,
                where_,
                color,
                linestyle: _,
                linewidth,
                ..
            } => {
                let idx = 0;
                let col = parse_color(color, idx).unwrap_or(default_color(idx));
                if x.len() < 2 || x.len() != y.len() {
                    continue;
                }
                let mut points = Vec::new();
                match where_.as_str() {
                    "pre" => {
                        let txv = tx(x[0]);
                        let tyv = ty(y[0]);
                        if txv.is_finite() && tyv.is_finite() {
                            points.push((txv, tyv));
                        }
                        for i in 1..x.len() {
                            let txv = tx(x[i]);
                            let tyv_prev = ty(y[i - 1]);
                            let tyv = ty(y[i]);
                            if txv.is_finite() && tyv_prev.is_finite() {
                                points.push((txv, tyv_prev));
                            }
                            if txv.is_finite() && tyv.is_finite() {
                                points.push((txv, tyv));
                            }
                        }
                    }
                    "post" => {
                        for i in 0..x.len() - 1 {
                            let txv = tx(x[i]);
                            let tyv = ty(y[i]);
                            let tyv_next = ty(y[i + 1]);
                            if txv.is_finite() && tyv.is_finite() {
                                points.push((txv, tyv));
                            }
                            if txv.is_finite() && tyv_next.is_finite() {
                                points.push((txv, tyv_next));
                            }
                        }
                        let txv = tx(x[x.len() - 1]);
                        let tyv = ty(y[y.len() - 1]);
                        if txv.is_finite() && tyv.is_finite() {
                            points.push((txv, tyv));
                        }
                    }
                    _ => {
                        let txv = tx(x[0]);
                        let tyv = ty(y[0]);
                        if txv.is_finite() && tyv.is_finite() {
                            points.push((txv, tyv));
                        }
                        for i in 1..x.len() {
                            let mid = (x[i - 1] + x[i]) / 2.0;
                            let tmid = tx(mid);
                            let tyv_prev = ty(y[i - 1]);
                            let tyv = ty(y[i]);
                            if tmid.is_finite() && tyv_prev.is_finite() {
                                points.push((tmid, tyv_prev));
                            }
                            if tmid.is_finite() && tyv.is_finite() {
                                points.push((tmid, tyv));
                            }
                        }
                        let txv = tx(x[x.len() - 1]);
                        let tyv = ty(y[y.len() - 1]);
                        if txv.is_finite() && tyv.is_finite() {
                            points.push((txv, tyv));
                        }
                    }
                }
                if points.len() < 2 {
                    continue;
                }
                let lw_px = ((*linewidth) * font_scale).round().max(1.0) as u32;
                let style: ShapeStyle = to_plotters_color(col).stroke_width(lw_px);
                chart
                    .draw_series(LineSeries::new(points, style))
                    .map_err(|e| PyRuntimeError::new_err(format!("Step draw: {}", e)))?;
            }
            PlotElement::BoxPlot { data, labels, .. } => {
                let box_width = 0.6;
                for (i, series) in data.iter().enumerate() {
                    if series.is_empty() {
                        continue;
                    }
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
                    if !tq1.is_finite()
                        || !tq3.is_finite()
                        || !tmed.is_finite()
                        || !tlower.is_finite()
                        || !tupper.is_finite()
                    {
                        continue;
                    }
                    let cx = (i + 1) as f64;
                    let col = to_plotters_color(default_color(i));
                    let fill_style: ShapeStyle = col.mix(0.3).filled();
                    let line_style: ShapeStyle = col.stroke_width(2);
                    chart
                        .draw_series(std::iter::once(PathElement::new(
                            vec![(cx, tlower), (cx, tupper)],
                            line_style,
                        )))
                        .map_err(|e| PyRuntimeError::new_err(format!("BoxPlot whisker: {}", e)))?;
                    chart
                        .draw_series(std::iter::once(Rectangle::new(
                            [(cx - box_width / 2.0, tq1), (cx + box_width / 2.0, tq3)],
                            fill_style,
                        )))
                        .map_err(|e| PyRuntimeError::new_err(format!("BoxPlot box: {}", e)))?;
                    chart
                        .draw_series(std::iter::once(Rectangle::new(
                            [(cx - box_width / 2.0, tq1), (cx + box_width / 2.0, tq3)],
                            line_style,
                        )))
                        .map_err(|e| PyRuntimeError::new_err(format!("BoxPlot border: {}", e)))?;
                    chart
                        .draw_series(std::iter::once(PathElement::new(
                            vec![(cx - box_width / 2.0, tmed), (cx + box_width / 2.0, tmed)],
                            col.stroke_width(2).filled(),
                        )))
                        .map_err(|e| PyRuntimeError::new_err(format!("BoxPlot median: {}", e)))?;
                    chart
                        .draw_series(std::iter::once(PathElement::new(
                            vec![
                                (cx - box_width / 4.0, tlower),
                                (cx + box_width / 4.0, tlower),
                            ],
                            line_style,
                        )))
                        .map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                    chart
                        .draw_series(std::iter::once(PathElement::new(
                            vec![
                                (cx - box_width / 4.0, tupper),
                                (cx + box_width / 4.0, tupper),
                            ],
                            line_style,
                        )))
                        .map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                    if let Some(lbls) = labels
                        && let Some(l) = lbls.get(i)
                    {
                        let box_family = font_stack::select_family(l);
                        let box_label_style: TextStyle =
                            FontDesc::from((box_family.as_str(), scale_font(11.0, font_scale)))
                                .color(&BLACK)
                                .pos(Pos::new(HPos::Center, VPos::Center));
                        chart
                            .draw_series(std::iter::once(plotters::element::Text::new(
                                l.to_string(),
                                (cx, -0.3),
                                box_label_style,
                            )))
                            .map_err(|e| {
                                PyRuntimeError::new_err(format!("BoxPlot label: {}", e))
                            })?;
                    }
                }
            }
            PlotElement::Annotate {
                text,
                xy,
                xytext,
                fontsize,
                color,
            } => {
                let col = parse_color(color, 0).unwrap_or(RgbColor(0, 0, 0));
                let rgb = to_plotters_color(col);
                let (txy_x, txy_y) = xytext.unwrap_or((xy.0, xy.1));
                let txy_x = tx(txy_x);
                let txy_y = ty(txy_y);
                let txy_xy_x = tx(xy.0);
                let txy_xy_y = ty(xy.1);
                if !txy_x.is_finite()
                    || !txy_y.is_finite()
                    || !txy_xy_x.is_finite()
                    || !txy_xy_y.is_finite()
                {
                    continue;
                }
                let arrow_style: ShapeStyle = rgb.stroke_width(1);
                chart
                    .draw_series(std::iter::once(PathElement::new(
                        vec![(txy_x, txy_y), (txy_xy_x, txy_xy_y)],
                        arrow_style,
                    )))
                    .map_err(|e| PyRuntimeError::new_err(format!("Annotate arrow: {}", e)))?;
                let anno_family = font_stack::select_family(text);
                let anno_style: TextStyle =
                    FontDesc::from((anno_family.as_str(), scale_font(*fontsize, font_scale)))
                        .color(&rgb)
                        .pos(Pos::new(HPos::Center, VPos::Center));
                chart
                    .draw_series(std::iter::once(plotters::element::Text::new(
                        text.to_string(),
                        (txy_x, txy_y),
                        anno_style,
                    )))
                    .map_err(|e| PyRuntimeError::new_err(format!("Annotate text: {}", e)))?;
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
#[allow(dead_code)]
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
    let p1 = (
        points[points.len() - 1].0,
        points[points.len() - 1].1 - cap_y_shift,
    );
    // 内点（用于计算切线方向）也要应用相同 y_shift，否则 cap 的切线方向会偏
    let next1 = (points[1].0, points[1].1 - cap_y_shift);
    let prev_n = (
        points[points.len() - 2].0,
        points[points.len() - 2].1 - cap_y_shift,
    );
    if capstyle.eq_ignore_ascii_case("round") {
        // 圆头：直径 = cap_lw_px（与线段 stroke_w 严格相等），构造 SVG 路径半圆。
        // 为保证光滑，使用 64 段直线逼近整个半圆，肉眼完全无折线感。
        draw_round_cap(
            chart, p0, next1, cap_lw_px, x_min, x_max, y_min, y_max, rgb, true,
        )?;
        draw_round_cap(
            chart, p1, prev_n, cap_lw_px, x_min, x_max, y_min, y_max, rgb, false,
        )?;
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
        let sa = (
            start_pt.0 + s_tan_x - s_nor_x,
            start_pt.1 + s_tan_y - s_nor_y,
        );
        let sb = (
            start_pt.0 + s_tan_x + s_nor_x,
            start_pt.1 + s_tan_y + s_nor_y,
        );
        let sc = (start_pt.0 + s_nor_x, start_pt.1 + s_nor_y);
        let sd = (start_pt.0 - s_nor_x, start_pt.1 - s_nor_y);
        chart
            .draw_series(std::iter::once(Polygon::new(
                vec![sa, sb, sc, sd],
                rgb.filled(),
            )))
            .map_err(|e| PyRuntimeError::new_err(format!("Cap square (start): {}", e)))?;
        // 结束端矩形（沿切线反方向延伸）
        let e_tan_x = cap_px * eux * x_per_pix;
        let e_tan_y = cap_px * euy * y_per_pix;
        let e_nor_x = cap_px * (-euy) * x_per_pix;
        let e_nor_y = cap_px * (eux) * y_per_pix;
        let ea = (end_pt.0 + e_tan_x - e_nor_x, end_pt.1 + e_tan_y - e_nor_y);
        let eb = (end_pt.0 + e_tan_x + e_nor_x, end_pt.1 + e_tan_y + e_nor_y);
        let ec = (end_pt.0 + e_nor_x, end_pt.1 + e_nor_y);
        let ed = (end_pt.0 - e_nor_x, end_pt.1 - e_nor_y);
        chart
            .draw_series(std::iter::once(Polygon::new(
                vec![ea, eb, ec, ed],
                rgb.filled(),
            )))
            .map_err(|e| PyRuntimeError::new_err(format!("Cap square (end): {}", e)))?;
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
    _x_min: f64,
    _x_max: f64,
    _y_min: f64,
    _y_max: f64,
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
    chart
        .draw_series(std::iter::once(circle_elem))
        .map_err(|e| PyRuntimeError::new_err(format!("Cap round circle: {}", e)))?;
    Ok(())
}
