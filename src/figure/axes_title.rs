//! 标题渲染模块
//!
//! 在数据区域上方的 margin_top 区域内绘制 axes 标题。
//! 字体大小根据 plotters 字体可见字符高度与 matplotlib 的差异做 1.20 倍补偿，
//! 以匹配 matplotlib 的视觉高度。

use plotters::coord::Shift;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::core::colors::RgbColor;
use crate::figure::axes::{DEFAULT_FONT_SCALE, scale_font};
use crate::utils::font_stack;
use crate::utils::mathtext::{self, HAlign, VAlign};

/// 渲染 axes 标题
///
/// # 参数
/// - `chart`: plotters 的 chart 上下文
/// - `title`: 标题文本（为空时不渲染）
/// - `title_fontsize`: 用户指定的字体大小（0 表示使用默认 12pt）
/// - `font_scale`: 字体缩放系数
/// - `title_color`: 标题颜色（默认黑色）
/// - `title_family`: 用户显式指定的字体族名（None 时按文本自动选择字体栈）
/// - `title_loc`: 标题水平位置（"left"/"center"/"right"，默认居中）
/// - `x_min`, `x_max`: X 轴数据范围（用于计算标题水平位置）
/// - `y_min`, `y_max`: Y 轴数据范围（用于计算标题垂直位置）
#[allow(clippy::too_many_arguments)]
pub fn draw_title<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    title: &str,
    title_fontsize: f64,
    font_scale: f64,
    title_color: RgbColor,
    title_family: Option<&str>,
    title_loc: &str,
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
    // 根据 loc 选择水平锚点与对齐方式：left→左端左对齐，right→右端右对齐，其余居中。
    let (title_x, h_align) = match title_loc {
        "left" => (x_min, HAlign::Left),
        "right" => (x_max, HAlign::Right),
        _ => ((x_min + x_max) / 2.0, HAlign::Center),
    };
    // 标题位于数据范围上方的 margin_top 区域，使用数据坐标的微小偏移
    let y_range = y_max - y_min;
    // 标题锚点在数据区顶部，使用 VAlign::Bottom 让文字向上延伸
    // 偏移量设为数据范围的极小比例，确保文字位于 margin_top 区域内
    let title_y = y_max + y_range * 0.01;
    // 使用用户指定的 fontsize；若未指定则取 matplotlib 默认 12pt
    let title_size = if title_fontsize > 0.0 {
        title_fontsize
    } else {
        9.6 * DEFAULT_FONT_SCALE
    };
    let rgb = RGBColor(title_color.0, title_color.1, title_color.2);
    mathtext::draw_math_chart(
        chart,
        title_x,
        title_y,
        title,
        scale_font(title_size, font_scale),
        rgb,
        title_family,
        h_align,
        VAlign::Bottom,
        0.0,
    )?;
    Ok(())
}

/// 在 figure 根绘图区上，用绝对像素坐标手动绘制 **非居中** 的 x 轴标签。
///
/// plotters 的 `x_desc` 只能水平居中，无法实现 matplotlib 的 loc="left"/"right"。
/// 当 loc 非居中时，`Axes::render` 会禁用 plotters 的 x_desc，改由此函数绘制。
/// 位置与 plotters 居中时保持一致（x 标签区底边、向上延伸），仅改变水平锚点。
///
/// # 参数
/// - `root`: figure 根绘图区（坐标为绝对像素）
/// - `text`: 标签文本
/// - `loc`: "left" / "right"（"center" 不应走到此函数）
/// - `fontsize`: 用户指定字号（<=0 时按 `default_size` 回退）
/// - `default_size`: 未显式指定字号时的默认像素字号（通常为 tick 标签像素字号）
/// - `color`: 文本颜色
/// - `family`: 用户显式字体族（None 时按文本自动选择）
/// - `data_left`, `data_right`: 数据区左右边缘的绝对像素 x 坐标
/// - `anchor_y`: x 标签区底边的绝对像素 y 坐标
#[allow(clippy::too_many_arguments)]
pub fn draw_xlabel_manual<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    text: &str,
    loc: &str,
    fontsize: f64,
    default_size: f64,
    color: RgbColor,
    family: Option<&str>,
    data_left: f64,
    data_right: f64,
    anchor_y: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if text.is_empty() {
        return Ok(());
    }
    let (anchor_x, h_align) = match loc {
        "left" => (data_left, HAlign::Left),
        "right" => (data_right, HAlign::Right),
        _ => ((data_left + data_right) / 2.0, HAlign::Center),
    };
    let size = if fontsize > 0.0 {
        fontsize
    } else {
        default_size
    };
    let rgb = RGBColor(color.0, color.1, color.2);
    mathtext::draw_math_area(
        root,
        anchor_x,
        anchor_y,
        text,
        size,
        rgb,
        family,
        h_align,
        VAlign::Bottom,
    )?;
    Ok(())
}

/// 在 figure 根绘图区上，用绝对像素坐标手动绘制 **非居中** 的 y 轴标签。
///
/// plotters 的 `y_desc` 只能垂直居中，无法实现 matplotlib 的 loc="top"/"bottom"。
/// 文字沿用 plotters 相同的 `FontTransform::Rotate270` 旋转（自下而上阅读），
/// 水平位置（贴近 y 标签区左缘、向右延伸）与居中时一致，仅改变垂直锚点：
/// - loc="top"：文字顶端对齐数据区顶边（`HPos::Right` 使旋转后向下延伸）
/// - loc="bottom"：文字底端对齐数据区底边（`HPos::Left` 使旋转后向上延伸）
///
/// # 参数
/// - `anchor_x`: y 标签区左缘的绝对像素 x 坐标
/// - `data_top`, `data_bottom`: 数据区上下边缘的绝对像素 y 坐标
#[allow(clippy::too_many_arguments)]
pub fn draw_ylabel_manual<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    text: &str,
    loc: &str,
    fontsize: f64,
    default_size: f64,
    color: RgbColor,
    family: Option<&str>,
    anchor_x: f64,
    data_top: f64,
    data_bottom: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if text.is_empty() {
        return Ok(());
    }
    // Rotate270 下，文字的水平范围（宽度）映射为屏幕竖直方向，由 HPos 控制：
    //   HPos::Left  → 从锚点向上延伸（底对齐）
    //   HPos::Right → 从锚点向下延伸（顶对齐）
    let (anchor_y, h_pos) = match loc {
        "top" => (data_top, HPos::Right),
        "bottom" => (data_bottom, HPos::Left),
        _ => ((data_top + data_bottom) / 2.0, HPos::Center),
    };
    let size = if fontsize > 0.0 {
        fontsize
    } else {
        default_size
    };
    let rgb = RGBColor(color.0, color.1, color.2);
    // 含数学 IR 时走旋转二维排版引擎（真实上/下标、分式线、根号盖线）；
    // loc 映射为阅读方向对齐：top→Top（块向下延伸）、bottom→Bottom（块向上延伸）、
    // 其余居中。
    if mathtext::contains_ir(text) {
        let valign = match loc {
            "top" => VAlign::Top,
            "bottom" => VAlign::Bottom,
            _ => VAlign::Center,
        };
        return mathtext::draw_math_area_rotated(
            root, anchor_x, anchor_y, text, size, rgb, family, valign,
        );
    }
    // 纯文本快路径：沿用 plotters 的单行 Rotate270 绘制。
    let fam = font_stack::resolve_font_family(text, family);
    let font: FontDesc = (fam.as_str(), size).into();
    // VPos::Top 使旋转后文字向右（朝向坐标轴）延伸，贴近 y 标签区左缘。
    let style: TextStyle = font
        .color(&rgb)
        .transform(FontTransform::Rotate270)
        .pos(Pos::new(h_pos, VPos::Top));
    root.draw_text(
        text,
        &style,
        (anchor_x.round() as i32, anchor_y.round() as i32),
    )
    .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw ylabel: {}", e)))?;
    Ok(())
}
