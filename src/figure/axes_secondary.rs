//! 二级坐标轴 (secondary_xaxis / secondary_yaxis) 与 twin 轴 (twinx / twiny) 的手动渲染。
//!
//! 不新建坐标系，只在数据区对侧（x 轴顶部 / y 轴右侧）按刻度值绘制刻度线、刻度值与
//! 轴标签，直接在 figure 根绘图区 (`root`) 上以绝对像素坐标绘制（与 `axes_colorbar`、
//! `axes_title` 同一套坐标约定）。
//!
//! - 二级轴：`forward` 把主轴数据坐标映射到二级刻度值（如弧度 → 角度），刻度取二级
//!   刻度空间的"漂亮"刻度，再用 `inverse` 反解回主轴数据坐标以精确定位（缺省时按线性
//!   比例插值）。二级轴**不画轴线(spine)**，只保留刻度线与刻度值。
//! - twin 轴：与主轴共享一条坐标（twinx 共享 x、twiny 共享 y），对侧坐标轴按自身数据
//!   范围的线性 nice_ticks 绘制，**不画轴线(spine)**，只保留刻度线与刻度值。

use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::figure::axes::SecondaryAxisSpec;
use crate::utils::font_stack;

/// 将刻度数值格式化为简洁字符串（整数不带小数，否则保留至多两位小数）。
fn fmt_tick(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{:.2}", v);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    }
}

/// 在 GIL 下调用 Python 变换函数（forward / inverse）计算标量映射，失败返回 None。
fn call_transform(py: Python<'_>, func: &Py<PyAny>, v: f64) -> Option<f64> {
    func.bind(py)
        .call1((v,))
        .ok()?
        .extract::<f64>()
        .ok()
        .filter(|r| r.is_finite())
}

/// 沿水平方向绘制一条 x 轴的刻度线、刻度值与可选轴标签。
///
/// `marks` 为 `(frac, label)`：`frac` 是自左(0)到右(1)沿数据区宽度的比例。
/// `top=true` 时轴位于数据区顶部，刻度线与刻度值朝上；否则朝下。
/// `draw_spine=true` 时额外沿 `axis_y` 画一条水平轴线（twin 用）；二级轴传 false。
#[allow(clippy::too_many_arguments)]
fn draw_x_marks<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    top: bool,
    draw_spine: bool,
    data_left: f64,
    data_right: f64,
    axis_y: f64,
    marks: &[(f64, String)],
    label: &str,
    tick_font_px: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let width = (data_right - data_left).max(1.0);
    let tick_len = (4.0 * ss).max(2.0);
    let color = RGBColor(0, 0, 0);
    let dir = if top { -1.0 } else { 1.0 };
    let vpos = if top { VPos::Bottom } else { VPos::Top };

    if draw_spine {
        root.draw(&PathElement::new(
            vec![
                (data_left.round() as i32, axis_y.round() as i32),
                (data_right.round() as i32, axis_y.round() as i32),
            ],
            color.stroke_width(1),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw x axis spine: {}", e)))?;
    }

    for (frac, lbl) in marks {
        if !(-0.001..=1.001).contains(frac) {
            continue;
        }
        let px = data_left + frac * width;
        root.draw(&PathElement::new(
            vec![
                (px.round() as i32, axis_y.round() as i32),
                (px.round() as i32, (axis_y + dir * tick_len).round() as i32),
            ],
            color.stroke_width(1),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw x tick: {}", e)))?;

        // 刻度值用字面 "sans-serif"（默认注册字体），与主轴 mesh 刻度标签
        // (axes.rs x/y_label_style) 完全一致；不走 select_family（其返回字体栈首个
        // 字体如 Helvetica，与主轴字体度量不同，同号字渲染偏大）。
        let style: TextStyle = ("sans-serif", tick_font_px)
            .into_font()
            .color(&color)
            .pos(Pos::new(HPos::Center, vpos));
        let ly = axis_y + dir * (tick_len + 3.0 * ss);
        root.draw_text(lbl, &style, (px.round() as i32, ly.round() as i32))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw x tick label: {}", e)))?;
    }

    if !label.is_empty() {
        let lx = (data_left + data_right) / 2.0;
        let ly = axis_y + dir * (tick_len + 3.0 * ss + tick_font_px + 6.0 * ss);
        let fam = font_stack::select_family(label);
        let style: TextStyle = (fam.as_str(), tick_font_px)
            .into_font()
            .color(&color)
            .pos(Pos::new(HPos::Center, vpos));
        root.draw_text(label, &style, (lx.round() as i32, ly.round() as i32))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw x axis label: {}", e)))?;
    }
    Ok(())
}

/// 沿竖直方向绘制一条 y 轴的刻度线、刻度值与可选轴标签。
///
/// `marks` 为 `(frac, label)`：`frac` 是自底(0)到顶(1)沿数据区高度的比例。
/// `right=true` 时轴位于数据区右侧，刻度线与刻度值朝右；否则朝左。
/// `draw_spine=true` 时额外沿 `axis_x` 画一条竖直轴线（twin 用）；二级轴传 false。
#[allow(clippy::too_many_arguments)]
fn draw_y_marks<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    right: bool,
    draw_spine: bool,
    axis_x: f64,
    data_top: f64,
    data_bottom: f64,
    marks: &[(f64, String)],
    label: &str,
    tick_font_px: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let height = (data_bottom - data_top).max(1.0);
    let tick_len = (4.0 * ss).max(2.0);
    let color = RGBColor(0, 0, 0);
    let dir = if right { 1.0 } else { -1.0 };
    let hpos = if right { HPos::Left } else { HPos::Right };

    if draw_spine {
        root.draw(&PathElement::new(
            vec![
                (axis_x.round() as i32, data_top.round() as i32),
                (axis_x.round() as i32, data_bottom.round() as i32),
            ],
            color.stroke_width(1),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw y axis spine: {}", e)))?;
    }

    for (frac, lbl) in marks {
        if !(-0.001..=1.001).contains(frac) {
            continue;
        }
        // 数据 y 向上增大，像素 y 向下增大：底边对应 frac=0。
        let py_pix = data_bottom - frac * height;
        root.draw(&PathElement::new(
            vec![
                (axis_x.round() as i32, py_pix.round() as i32),
                (
                    (axis_x + dir * tick_len).round() as i32,
                    py_pix.round() as i32,
                ),
            ],
            color.stroke_width(1),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw y tick: {}", e)))?;

        // 刻度值用字面 "sans-serif"，与主轴 mesh 刻度标签一致（见 draw_x_marks 注释）。
        let style: TextStyle = ("sans-serif", tick_font_px)
            .into_font()
            .color(&color)
            .pos(Pos::new(hpos, VPos::Center));
        let lx = axis_x + dir * (tick_len + 3.0 * ss);
        root.draw_text(lbl, &style, (lx.round() as i32, py_pix.round() as i32))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw y tick label: {}", e)))?;
    }

    if !label.is_empty() {
        let ly = (data_top + data_bottom) / 2.0;
        let lx = axis_x + dir * (tick_len + 3.0 * ss + tick_font_px + 6.0 * ss);
        let fam = font_stack::select_family(label);
        let font: FontDesc = (fam.as_str(), tick_font_px).into();
        // 竖直排布（自下而上阅读），与主 y 轴标签一致。
        let style: TextStyle = font
            .color(&color)
            .transform(FontTransform::Rotate270)
            .pos(Pos::new(HPos::Center, VPos::Top));
        root.draw_text(label, &style, (lx.round() as i32, ly.round() as i32))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw y axis label: {}", e)))?;
    }
    Ok(())
}

/// 用 `forward`/`inverse` 计算二级刻度在主轴数据坐标上的比例位置，得到 `(frac, label)`。
fn secondary_marks(
    py: Python<'_>,
    spec: &SecondaryAxisSpec,
    v_min: f64,
    v_max: f64,
) -> Option<Vec<(f64, String)>> {
    let sec_lo = call_transform(py, &spec.forward, v_min)?;
    let sec_hi = call_transform(py, &spec.forward, v_max)?;
    let span = v_max - v_min;
    if span.abs() < 1e-12 {
        return None;
    }
    let (lo, hi) = (sec_lo.min(sec_hi), sec_lo.max(sec_hi));
    let ticks = crate::figure::axes_mesh::nice_ticks(lo, hi);
    let mut marks = Vec::with_capacity(ticks.len());
    for &v in &ticks {
        let data_pos = match &spec.inverse {
            Some(inv) => match call_transform(py, inv, v) {
                Some(p) => p,
                None => continue,
            },
            None => {
                let f = if (sec_hi - sec_lo).abs() < 1e-12 {
                    0.0
                } else {
                    (v - sec_lo) / (sec_hi - sec_lo)
                };
                v_min + f * span
            }
        };
        let frac = (data_pos - v_min) / span;
        marks.push((frac, fmt_tick(v)));
    }
    Some(marks)
}

/// 线性刻度（twin 用）：在 `[v_min, v_max]` 上取 nice_ticks，得到 `(frac, label)`。
fn linear_marks(v_min: f64, v_max: f64) -> Vec<(f64, String)> {
    let span = v_max - v_min;
    if span.abs() < 1e-12 {
        return Vec::new();
    }
    crate::figure::axes_mesh::nice_ticks(v_min.min(v_max), v_min.max(v_max))
        .into_iter()
        .map(|v| ((v - v_min) / span, fmt_tick(v)))
        .collect()
}

/// 绘制二级 x 轴（默认位于数据区顶部；location=="bottom" 则位于底部）。不画轴线。
#[allow(clippy::too_many_arguments)]
pub fn draw_secondary_xaxis<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    py: Python<'_>,
    spec: &SecondaryAxisSpec,
    data_left: f64,
    data_right: f64,
    data_top: f64,
    data_bottom: f64,
    x_min: f64,
    x_max: f64,
    tick_font_px: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let bottom = spec.location.eq_ignore_ascii_case("bottom");
    let marks = match secondary_marks(py, spec, x_min, x_max) {
        Some(m) => m,
        None => return Ok(()),
    };
    let axis_y = if bottom { data_bottom } else { data_top };
    draw_x_marks(
        root,
        !bottom,
        false,
        data_left,
        data_right,
        axis_y,
        &marks,
        &spec.label,
        tick_font_px,
        ss,
    )
}

/// 绘制二级 y 轴（默认位于数据区右侧；location=="left" 则位于左侧）。不画轴线。
#[allow(clippy::too_many_arguments)]
pub fn draw_secondary_yaxis<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    py: Python<'_>,
    spec: &SecondaryAxisSpec,
    data_left: f64,
    data_right: f64,
    data_top: f64,
    data_bottom: f64,
    y_min: f64,
    y_max: f64,
    tick_font_px: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let left = spec.location.eq_ignore_ascii_case("left");
    let marks = match secondary_marks(py, spec, y_min, y_max) {
        Some(m) => m,
        None => return Ok(()),
    };
    let axis_x = if left { data_left } else { data_right };
    draw_y_marks(
        root,
        !left,
        false,
        axis_x,
        data_top,
        data_bottom,
        &marks,
        &spec.label,
        tick_font_px,
        ss,
    )
}

/// 绘制 twinx 的右侧 y 轴（刻度 + 刻度值 + 可选竖排轴标签，不画轴线）。
#[allow(clippy::too_many_arguments)]
pub fn draw_twin_right_yaxis<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    data_right: f64,
    data_top: f64,
    data_bottom: f64,
    y_min: f64,
    y_max: f64,
    label: &str,
    tick_font_px: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let marks = linear_marks(y_min, y_max);
    draw_y_marks(
        root,
        true,
        false,
        data_right,
        data_top,
        data_bottom,
        &marks,
        label,
        tick_font_px,
        ss,
    )
}

/// 绘制 twiny 的顶部 x 轴（刻度 + 刻度值 + 可选轴标签，不画轴线）。
#[allow(clippy::too_many_arguments)]
pub fn draw_twin_top_xaxis<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    data_left: f64,
    data_right: f64,
    data_top: f64,
    x_min: f64,
    x_max: f64,
    label: &str,
    tick_font_px: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let marks = linear_marks(x_min, x_max);
    draw_x_marks(
        root,
        true,
        false,
        data_left,
        data_right,
        data_top,
        &marks,
        label,
        tick_font_px,
        ss,
    )
}
