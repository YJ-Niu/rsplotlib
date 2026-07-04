//! 颜色条 (colorbar) 渲染模块
//!
//! 在数据区右侧的空白 margin 内绘制一条竖直渐变色带 + 边框 + 刻度标签，
//! 直接在 figure 根绘图区 (`root`) 上以绝对像素坐标绘制（与 axes_title 中的
//! `draw_ylabel_manual` 同一套坐标约定）。颜色取自统一入口 `colormap_color`，
//! 因此与 scatter / imshow 的取色完全一致。

use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::core::colormap::colormap_color;
use crate::figure::axes::scale_font;
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

/// 在数据区右侧绘制颜色条。
///
/// # 参数
/// - `root`: figure 根绘图区（绝对像素坐标）
/// - `cmap`: colormap 名称（支持 `_r` 反转变体）
/// - `vmin` / `vmax`: 颜色条数值范围（下端 / 上端）
/// - `data_right`: 数据区右边缘的像素 x 坐标
/// - `data_top` / `data_bottom`: 数据区上下边缘的像素 y 坐标
/// - `total_w`: figure 总宽度（像素），作为颜色条 + 标签的右边界
/// - `font_scale`: 字体缩放系数
/// - `ss`: 超采样系数（用于把固定像素尺寸放大到超采样画布）
#[allow(clippy::too_many_arguments)]
pub fn draw_colorbar<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    cmap: &str,
    vmin: f64,
    vmax: f64,
    data_right: f64,
    data_top: f64,
    data_bottom: f64,
    total_w: f64,
    font_scale: f64,
    ss: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    let height = (data_bottom - data_top).max(1.0);
    let gap = (total_w - data_right).max(1.0);

    // 色带几何：与数据区留一段间距，色带宽度取 figure 右侧空白的一部分并夹在
    // [6px, 28px*ss] 之间；其余空间留给刻度标签。
    let pad = gap * 0.10;
    let bar_width = (gap * 0.28).clamp(6.0, 28.0 * ss);
    let bar_left = data_right + pad;
    let bar_right = bar_left + bar_width;

    // 竖直渐变：顶部对应 vmax (t=1)，底部对应 vmin (t=0)。逐像素行绘制细矩形。
    let steps = height.ceil() as i32;
    for k in 0..steps {
        let y_a = data_top + height * (k as f64) / (steps as f64);
        let y_b = data_top + height * ((k + 1) as f64) / (steps as f64);
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

    // 边框
    root.draw(&Rectangle::new(
        [
            (bar_left.round() as i32, data_top.round() as i32),
            (bar_right.round() as i32, data_bottom.round() as i32),
        ],
        RGBColor(60, 60, 60).stroke_width(1),
    ))
    .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar border: {}", e)))?;

    // 刻度：vmin..vmax 之间等距取 6 个刻度，画短横线 + 数值标签
    let n_ticks = 6i32;
    let tick_len = (4.0 * ss).max(2.0);
    let label_x = bar_right + tick_len + 3.0 * ss;
    let font_size = scale_font(10.0, font_scale);

    for i in 0..n_ticks {
        let frac = i as f64 / (n_ticks - 1) as f64; // 0 (底) -> 1 (顶)
        let v = vmin + (vmax - vmin) * frac;
        let y = data_bottom - height * frac;

        // 刻度短线（色带右侧向外）
        root.draw(&PathElement::new(
            vec![
                (bar_right.round() as i32, y.round() as i32),
                ((bar_right + tick_len).round() as i32, y.round() as i32),
            ],
            RGBColor(60, 60, 60).stroke_width(1),
        ))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw colorbar tick: {}", e)))?;

        let label = fmt_tick(v);
        let fam = font_stack::select_family(&label);
        let style: TextStyle = (fam.as_str(), font_size)
            .into_font()
            .color(&RGBColor(0, 0, 0))
            .pos(Pos::new(HPos::Left, VPos::Center));
        root.draw_text(&label, &style, (label_x.round() as i32, y.round() as i32))
            .map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to draw colorbar label: {}", e))
            })?;
    }

    Ok(())
}
