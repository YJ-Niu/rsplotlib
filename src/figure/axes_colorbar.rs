//! 颜色条 (colorbar) 渲染模块
//!
//! 颜色条从父子图数据区「窃取」一条空间后，紧贴数据区绘制渐变色带 + 边框 +
//! 刻度 + 刻度值（+ 可选轴标签 + 越界三角端）。竖直（location right/left）色带
//! 沿 y 方向，水平（location top/bottom）沿 x 方向。直接在 figure 根绘图区
//! (`root`) 上以绝对像素坐标绘制。颜色取自统一入口 `colormap_color`，与
//! scatter / imshow 的取色完全一致。

use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::core::colormap::colormap_color;
use crate::figure::axes::{ColorbarSpec, DEFAULT_FONT_SCALE, scale_font};
use crate::figure::axes_mesh::nice_ticks;
use crate::utils::font_stack;

/// 颜色条刻度短线长度（超采样像素），与图形层预算计算保持一致。
pub fn colorbar_tick_len(ss: f64) -> f64 {
    (8.0 * ss).max(4.0)
}

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

/// 对数刻度值：[vmin, vmax] 内的 10 的整数幂（如 [0.01,100] -> 0.01,0.1,1,10,100）。
/// 值域非正或无整幂落入时退回线性 `nice_ticks`。
fn log_decade_ticks(vmin: f64, vmax: f64) -> Vec<f64> {
    let (lo, hi) = (vmin.min(vmax), vmin.max(vmax));
    if lo <= 0.0 || hi <= 0.0 {
        return nice_ticks(vmin, vmax);
    }
    let e_lo = lo.log10().floor() as i32;
    let e_hi = hi.log10().ceil() as i32;
    let mut out = Vec::new();
    for e in e_lo..=e_hi {
        let v = 10f64.powi(e);
        if v >= lo * (1.0 - 1e-9) && v <= hi * (1.0 + 1e-9) {
            out.push(v);
        }
    }
    if out.is_empty() {
        nice_ticks(vmin, vmax)
    } else {
        out
    }
}

/// 对数刻度标签：10 的整数幂用简洁十进制（0.01 / 0.1 / 1 / 10 / 100），
/// |指数| 过大时用科学计数；非整幂退回通用简洁格式。
fn fmt_log_tick(v: f64) -> String {
    if v <= 0.0 {
        return fmt_tick(v);
    }
    let e = v.log10().round() as i32;
    if (10f64.powi(e) - v).abs() > v.abs() * 1e-6 {
        return fmt_tick(v);
    }
    if (-4..=6).contains(&e) {
        if e >= 0 {
            format!("{}", 10f64.powi(e) as i64)
        } else {
            format!("{:.*}", (-e) as usize, v)
        }
    } else if e >= 0 {
        format!("1e+{:02}", e)
    } else {
        format!("1e-{:02}", -e)
    }
}

/// 值 v 在色带上的归一化位置 [0,1]。线性: (v-vmin)/span；对数: 按 ln 插值。
fn bar_frac(v: f64, vmin: f64, vmax: f64, is_log: bool) -> f64 {
    if is_log && vmin > 0.0 && vmax > 0.0 {
        let d = vmax.ln() - vmin.ln();
        if d.abs() < 1e-12 {
            0.0
        } else {
            (v.ln() - vmin.ln()) / d
        }
    } else {
        let span = vmax - vmin;
        if span.abs() < 1e-12 {
            0.0
        } else {
            (v - vmin) / span
        }
    }
}

/// 颜色条刻度值与其显示标签：对数模式取 10 的幂并用对数标签，否则线性 nice_ticks。
/// 用户显式 ticks / format 优先。
fn colorbar_ticks_labels(spec: &ColorbarSpec) -> Vec<(f64, String)> {
    let is_log = spec.is_log();
    let ticks = spec.ticks.clone().unwrap_or_else(|| {
        if is_log {
            log_decade_ticks(spec.vmin, spec.vmax)
        } else {
            nice_ticks(spec.vmin, spec.vmax)
        }
    });
    ticks
        .into_iter()
        .map(|v| {
            let label = if is_log && spec.format.is_none() {
                fmt_log_tick(v)
            } else {
                fmt_colorbar_tick(v, spec.format.as_deref())
            };
            (v, label)
        })
        .collect()
}

/// 按 matplotlib `format` 参数格式化刻度值。
///
/// 支持 None（缺省简洁格式）、C 风格 `%[.prec][efgEFG]`（如 `%4.2e`、`%.3f`）与
/// 新式 `{x:.2e}` / `{:.2f}`。无法识别的格式回退到缺省简洁格式。
pub fn fmt_colorbar_tick(v: f64, format: Option<&str>) -> String {
    match format {
        None => fmt_tick(v),
        Some(spec) => format_with_spec(v, spec).unwrap_or_else(|| fmt_tick(v)),
    }
}

fn format_with_spec(v: f64, spec: &str) -> Option<String> {
    let ty = spec
        .chars()
        .rev()
        .find(|c| matches!(c, 'e' | 'E' | 'f' | 'F' | 'g' | 'G'))?;
    let prec = spec.find('.').and_then(|dot| {
        let digits: String = spec[dot + 1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse::<usize>().ok()
    });
    let p = prec.unwrap_or(6);
    let s = match ty {
        'f' | 'F' => format!("{:.*}", p, v),
        'e' => format!("{:.*e}", p, v),
        'E' => format!("{:.*E}", p, v),
        'g' | 'G' => {
            let t = format!("{:.*}", p, v);
            t.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        _ => return None,
    };
    Some(s)
}

/// 绘制颜色条：从数据区某侧（location）「窃取」的空间内绘制渐变色带 + 刻度 + 标签。
///
/// # 参数
/// - `root`: figure 根绘图区（绝对像素坐标）
/// - `spec`: 颜色条完整配置（cmap / 值域 / location / shrink / extend / ticks / label 等）
/// - `thickness`: 色带短边厚度（超采样像素，由图形层依 aspect/shrink 预算得出）
/// - `pad_px`: 色带与数据区之间的间隙（超采样像素）
/// - `data_left/right/top/bottom`: 父数据区四边的绝对像素坐标
/// - `font_scale`: 字体缩放系数
/// - `ss`: 超采样系数
#[allow(clippy::too_many_arguments)]
pub fn draw_colorbar<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    spec: &ColorbarSpec,
    thickness: f64,
    pad_px: f64,
    data_left: f64,
    data_right: f64,
    data_top: f64,
    data_bottom: f64,
    font_scale: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if spec.is_horizontal() {
        draw_horizontal(
            root,
            spec,
            thickness,
            pad_px,
            data_left,
            data_right,
            data_top,
            data_bottom,
            font_scale,
            ss,
        )
    } else {
        draw_vertical(
            root,
            spec,
            thickness,
            pad_px,
            data_left,
            data_right,
            data_top,
            data_bottom,
            font_scale,
            ss,
        )
    }
}

/// 竖直色带（location right/left）：长轴沿 y，顶端=vmax，底端=vmin。
#[allow(clippy::too_many_arguments)]
fn draw_vertical<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    spec: &ColorbarSpec,
    thickness: f64,
    pad_px: f64,
    data_left: f64,
    data_right: f64,
    data_top: f64,
    data_bottom: f64,
    font_scale: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let border = RGBColor(60, 60, 60);
    let text_color = RGBColor(0, 0, 0);
    let font_size = scale_font(10.0 * DEFAULT_FONT_SCALE, font_scale);
    let tick_len = colorbar_tick_len(ss);
    let cmap = spec.cmap.as_str();
    let (vmin, vmax) = (spec.vmin, spec.vmax);
    let extend_min = matches!(spec.extend.as_str(), "both" | "min");
    let extend_max = matches!(spec.extend.as_str(), "both" | "max");
    let on_left = spec.location == "left";

    let full_len = (data_bottom - data_top).max(1.0);
    let bar_len = (full_len * spec.shrink).max(1.0);
    let cy = (data_top + data_bottom) / 2.0;
    let bar_top = cy - bar_len / 2.0;
    let bar_bottom = cy + bar_len / 2.0;
    let (bar_left, bar_right) = if on_left {
        let r = data_left - pad_px;
        (r - thickness, r)
    } else {
        let l = data_right + pad_px;
        (l, l + thickness)
    };
    let ext = bar_len * 0.05;

    // 渐变：逐像素行；顶=vmax(t=1)，底=vmin(t=0)。
    let steps = bar_len.ceil().max(1.0) as i32;
    for k in 0..steps {
        let y_a = bar_top + bar_len * (k as f64) / (steps as f64);
        let y_b = bar_top + bar_len * ((k + 1) as f64) / (steps as f64);
        let t = 1.0 - (k as f64 + 0.5) / (steps as f64);
        let c = colormap_color(cmap, t);
        root.draw(&Rectangle::new(
            [
                (bar_left.round() as i32, y_a.round() as i32),
                (bar_right.round() as i32, y_b.round() as i32),
            ],
            RGBColor(c.0, c.1, c.2).filled(),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar bar: {}", e)))?;
    }

    // 越界三角端：max 端在顶（向上），min 端在底（向下）。
    let cx = (bar_left + bar_right) / 2.0;
    if extend_max {
        let c = colormap_color(cmap, 1.0);
        root.draw(&Polygon::new(
            vec![
                (bar_left.round() as i32, bar_top.round() as i32),
                (bar_right.round() as i32, bar_top.round() as i32),
                (cx.round() as i32, (bar_top - ext).round() as i32),
            ],
            RGBColor(c.0, c.1, c.2).filled(),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar extend: {}", e)))?;
    }
    if extend_min {
        let c = colormap_color(cmap, 0.0);
        root.draw(&Polygon::new(
            vec![
                (bar_left.round() as i32, bar_bottom.round() as i32),
                (bar_right.round() as i32, bar_bottom.round() as i32),
                (cx.round() as i32, (bar_bottom + ext).round() as i32),
            ],
            RGBColor(c.0, c.1, c.2).filled(),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar extend: {}", e)))?;
    }

    // 刻度 + 刻度值（色带外侧）
    let ticks = colorbar_ticks_labels(spec);
    let is_log = spec.is_log();
    let (lo, hi) = (vmin.min(vmax), vmin.max(vmax));
    let mut tick_label_w = 0.0f64;
    for (v, label) in &ticks {
        let v = *v;
        if v < lo - 1e-9 || v > hi + 1e-9 {
            continue;
        }
        let frac = bar_frac(v, vmin, vmax, is_log);
        let y = bar_bottom - bar_len * frac;
        let (tick_x0, tick_x1, label_x, hpos) = if on_left {
            (
                bar_left,
                bar_left - tick_len,
                bar_left - tick_len - 3.0 * ss,
                HPos::Right,
            )
        } else {
            (
                bar_right,
                bar_right + tick_len,
                bar_right + tick_len + 3.0 * ss,
                HPos::Left,
            )
        };
        root.draw(&PathElement::new(
            vec![
                (tick_x0.round() as i32, y.round() as i32),
                (tick_x1.round() as i32, y.round() as i32),
            ],
            border.stroke_width(1),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar tick: {}", e)))?;

        let fam = font_stack::select_family(label);
        if let Ok((w, _)) = (fam.as_str(), font_size).into_font().box_size(label) {
            tick_label_w = tick_label_w.max(w as f64);
        }
        let style: TextStyle = (fam.as_str(), font_size)
            .into_font()
            .color(&text_color)
            .pos(Pos::new(hpos, VPos::Center));
        root.draw_text(label, &style, (label_x.round() as i32, y.round() as i32))
            .map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to draw colorbar tick label: {}", e))
            })?;
    }

    // 轴标签（竖排，在刻度值外侧）
    if !spec.label.is_empty() {
        let label_x = if on_left {
            bar_left - tick_len - 3.0 * ss - tick_label_w - 4.0 * ss
        } else {
            bar_right + tick_len + 3.0 * ss + tick_label_w + 4.0 * ss
        };
        let fam = font_stack::select_family(&spec.label);
        // Rotate270：文字自下而上阅读，居中于色带。
        let style: TextStyle = (fam.as_str(), font_size)
            .into_font()
            .color(&text_color)
            .transform(FontTransform::Rotate270)
            .pos(Pos::new(HPos::Center, VPos::Center));
        root.draw_text(
            &spec.label,
            &style,
            (label_x.round() as i32, cy.round() as i32),
        )
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar label: {}", e)))?;
    }

    Ok(())
}

/// 水平色带（location top/bottom）：长轴沿 x，左端=vmin，右端=vmax。
#[allow(clippy::too_many_arguments)]
fn draw_horizontal<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    spec: &ColorbarSpec,
    thickness: f64,
    pad_px: f64,
    data_left: f64,
    data_right: f64,
    data_top: f64,
    data_bottom: f64,
    font_scale: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let border = RGBColor(60, 60, 60);
    let text_color = RGBColor(0, 0, 0);
    let font_size = scale_font(10.0 * DEFAULT_FONT_SCALE, font_scale);
    let tick_len = colorbar_tick_len(ss);
    let cmap = spec.cmap.as_str();
    let (vmin, vmax) = (spec.vmin, spec.vmax);
    let extend_min = matches!(spec.extend.as_str(), "both" | "min");
    let extend_max = matches!(spec.extend.as_str(), "both" | "max");
    let on_top = spec.location == "top";

    let full_len = (data_right - data_left).max(1.0);
    let bar_len = (full_len * spec.shrink).max(1.0);
    let cx = (data_left + data_right) / 2.0;
    let bar_left = cx - bar_len / 2.0;
    let bar_right = cx + bar_len / 2.0;
    let (bar_top, bar_bottom) = if on_top {
        let b = data_top - pad_px;
        (b - thickness, b)
    } else {
        let t = data_bottom + pad_px;
        (t, t + thickness)
    };
    let ext = bar_len * 0.05;

    // 渐变：逐像素列；左=vmin(t=0)，右=vmax(t=1)。
    let steps = bar_len.ceil().max(1.0) as i32;
    for k in 0..steps {
        let x_a = bar_left + bar_len * (k as f64) / (steps as f64);
        let x_b = bar_left + bar_len * ((k + 1) as f64) / (steps as f64);
        let t = (k as f64 + 0.5) / (steps as f64);
        let c = colormap_color(cmap, t);
        root.draw(&Rectangle::new(
            [
                (x_a.round() as i32, bar_top.round() as i32),
                (x_b.round() as i32, bar_bottom.round() as i32),
            ],
            RGBColor(c.0, c.1, c.2).filled(),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar bar: {}", e)))?;
    }

    // 越界三角端：min 端在左（向左），max 端在右（向右）。
    let cyy = (bar_top + bar_bottom) / 2.0;
    if extend_min {
        let c = colormap_color(cmap, 0.0);
        root.draw(&Polygon::new(
            vec![
                (bar_left.round() as i32, bar_top.round() as i32),
                (bar_left.round() as i32, bar_bottom.round() as i32),
                ((bar_left - ext).round() as i32, cyy.round() as i32),
            ],
            RGBColor(c.0, c.1, c.2).filled(),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar extend: {}", e)))?;
    }
    if extend_max {
        let c = colormap_color(cmap, 1.0);
        root.draw(&Polygon::new(
            vec![
                (bar_right.round() as i32, bar_top.round() as i32),
                (bar_right.round() as i32, bar_bottom.round() as i32),
                ((bar_right + ext).round() as i32, cyy.round() as i32),
            ],
            RGBColor(c.0, c.1, c.2).filled(),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar extend: {}", e)))?;
    }

    // 刻度 + 刻度值（色带外侧）
    let ticks = colorbar_ticks_labels(spec);
    let is_log = spec.is_log();
    let (lo, hi) = (vmin.min(vmax), vmin.max(vmax));
    for (v, label) in &ticks {
        let v = *v;
        if v < lo - 1e-9 || v > hi + 1e-9 {
            continue;
        }
        let frac = bar_frac(v, vmin, vmax, is_log);
        let x = bar_left + bar_len * frac;
        let (ty0, ty1, label_y, vpos) = if on_top {
            (
                bar_top,
                bar_top - tick_len,
                bar_top - tick_len - 3.0 * ss,
                VPos::Bottom,
            )
        } else {
            (
                bar_bottom,
                bar_bottom + tick_len,
                bar_bottom + tick_len + 3.0 * ss,
                VPos::Top,
            )
        };
        root.draw(&PathElement::new(
            vec![
                (x.round() as i32, ty0.round() as i32),
                (x.round() as i32, ty1.round() as i32),
            ],
            border.stroke_width(1),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar tick: {}", e)))?;

        let fam = font_stack::select_family(label);
        let style: TextStyle = (fam.as_str(), font_size)
            .into_font()
            .color(&text_color)
            .pos(Pos::new(HPos::Center, vpos));
        root.draw_text(label, &style, (x.round() as i32, label_y.round() as i32))
            .map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to draw colorbar tick label: {}", e))
            })?;
    }

    // 轴标签（水平，在刻度值外侧）
    if !spec.label.is_empty() {
        let label_y = if on_top {
            bar_top - tick_len - 3.0 * ss - font_size - 4.0 * ss
        } else {
            bar_bottom + tick_len + 3.0 * ss + font_size + 4.0 * ss
        };
        let fam = font_stack::select_family(&spec.label);
        let style: TextStyle = (fam.as_str(), font_size)
            .into_font()
            .color(&text_color)
            .pos(Pos::new(HPos::Center, VPos::Center));
        root.draw_text(
            &spec.label,
            &style,
            (cx.round() as i32, label_y.round() as i32),
        )
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar label: {}", e)))?;
    }

    Ok(())
}
