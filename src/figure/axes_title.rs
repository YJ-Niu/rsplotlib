//! 标题渲染模块
//!
//! 在数据区域上方的 margin_top 区域内绘制 axes 标题。
//! 字体大小根据 plotters 字体可见字符高度与 matplotlib 的差异做 1.20 倍补偿，
//! 以匹配 matplotlib 的视觉高度。

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, VPos, Pos};

use crate::figure::axes::scale_font;
use crate::utils::font_stack;

/// 渲染 axes 标题
///
/// # 参数
/// - `chart`: plotters 的 chart 上下文
/// - `title`: 标题文本（为空时不渲染）
/// - `title_fontsize`: 用户指定的字体大小（0 表示使用默认 12pt）
/// - `font_scale`: 字体缩放系数
/// - `x_min`, `x_max`: X 轴数据范围（用于计算标题水平居中位置）
/// - `y_min`, `y_max`: Y 轴数据范围（用于计算标题垂直位置）
pub fn draw_title<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    title: &str,
    title_fontsize: f64,
    font_scale: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if title.is_empty() {
        return Ok(());
    }
    let title_x = (x_min + x_max) / 2.0;
    // 标题位于数据范围上方的 margin_top 区域，使用数据坐标的微小偏移
    let y_range = y_max - y_min;
    // 标题锚点在数据区顶部，使用 VPos::Bottom 让文字向上延伸
    // 偏移量设为数据范围的极小比例，确保文字位于 margin_top 区域内
    let title_y = y_max + y_range * 0.01;
    // 使用用户指定的 fontsize；若未指定则取 matplotlib 默认 12pt
    let title_size = if title_fontsize > 0.0 { title_fontsize } else { 12.0 };
    // plotters 的 ab_glyph 字体可见字符高度约 0.94em，而 matplotlib DejaVu Sans 约 1.13em。
    // 为匹配 matplotlib 视觉高度，乘以 1.40 补偿。
    let title_family = font_stack::select_family(title);
    let font: FontDesc = (title_family.as_str(), scale_font(title_size * 1.25, font_scale)).into();
    let colored_font = font.color(&BLACK);
    let text_style: TextStyle = colored_font.pos(Pos::new(HPos::Center, VPos::Bottom));
    chart.draw_series(std::iter::once(plotters::element::Text::new(
        title.to_string(),
        (title_x, title_y),
        text_style,
    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw title: {}", e)))?;
    Ok(())
}
