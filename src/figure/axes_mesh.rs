//! 轴网格（mesh）辅助模块
//!
//! 提供主刻度/副刻度位置计算、坐标轴标签/刻度文本格式化等辅助函数。
//! 注意：mesh builder 的具体配置（label style、formatter 等）和 spine 绘制
//! 与 plotters ChartContext/MeshStyle 的借用密切相关，仍在 axes.rs::render
//! 中内联完成。本模块只提供纯计算/格式化辅助函数。
//!
//! 主要 API：
//! - `nice_ticks()`: matplotlib 兼容的"漂亮"主刻度算法
//! - `compute_ticks()`: 计算 X/Y 主刻度与副刻度（考虑用户自定义 locator、密度回退）
//! - `compute_grid_style()`: 解析主/副网格线颜色/线宽/样式
//! - `format_linear_tick()`: 线性刻度文本格式化（整数去 .0，小数最多 2 位）

use plotters::style::IntoFont;
use pyo3::prelude::*;

use crate::core::colors::{RgbColor, parse_color};

/// matplotlib 兼容的"漂亮"主刻度算法（默认约 7 个区间）
pub fn nice_ticks(min: f64, max: f64) -> Vec<f64> {
    nice_ticks_with_intervals(min, max, 7.0)
}

/// 按目标区间数生成"漂亮"主刻度。`target_intervals` 越小步长越大、刻度越少。
/// 供 X 轴按可用像素宽度稀释刻度使用（见 `thin_x_ticks`）。
fn nice_ticks_with_intervals(min: f64, max: f64, target_intervals: f64) -> Vec<f64> {
    if min >= max || !min.is_finite() || !max.is_finite() {
        return vec![min, max];
    }
    let range = max - min;
    if range <= 0.0 {
        return vec![min];
    }
    // 选择合适的步长（matplotlib 的 MaxNLocator 简化版）
    let rough = range / target_intervals.max(1.0);
    let mag = 10f64.powf(rough.log10().floor());
    let norm = rough / mag;
    let step = if norm < 1.5 {
        mag
    } else if norm < 3.0 {
        2.0 * mag
    } else if norm < 7.0 {
        5.0 * mag
    } else {
        10.0 * mag
    };
    let start = (min / step).ceil() * step;
    let end = (max / step).floor() * step;
    let n = ((end + step * 0.001 - start) / step).ceil() as usize + 1;
    let mut ticks = Vec::with_capacity(n);
    let mut t = start;
    while t <= end + step * 0.001 {
        ticks.push(t);
        t += step;
    }
    if ticks.is_empty() {
        ticks.push(min);
    }
    ticks
}

/// 生成"漂亮"主刻度并保证刻度数量不超过 `max_ticks`：逐步减少目标区间数（增大步长）
/// 直到刻度数达标，用于在子图较窄、刻度值较长时稀释 X 轴刻度以避免标签水平重叠。
fn nice_ticks_capped(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let mut intervals = max_ticks.saturating_sub(1).max(1);
    loop {
        let ticks = nice_ticks_with_intervals(min, max, intervals as f64);
        if ticks.len() <= max_ticks || intervals <= 1 {
            return ticks;
        }
        intervals -= 1;
    }
}

/// 用与刻度值渲染一致的字体（"sans-serif" + 指定字号）测量单个标签的像素宽度。
/// 度量失败时按字符数粗略估算。字号 ≤ 0 或空串返回 0。
fn measure_label_px(label: &str, font_size: f64) -> f64 {
    if font_size <= 0.0 || label.is_empty() {
        return 0.0;
    }
    let plain = crate::utils::mathtext::to_plain(label);
    ("sans-serif", font_size)
        .into_font()
        .box_size(&plain)
        .map(|(w, _)| w as f64)
        .unwrap_or_else(|_| plain.chars().count() as f64 * font_size * 0.6)
}

/// 当 X 轴自动刻度的标签较长而绘图区较窄时，按「绘图区像素宽 / 单个标签所需间距」上限
/// 减少刻度数量（增大间距），避免刻度值水平重叠。
///
/// `plot_px_w` 为绘图区像素宽，`font_size` 为刻度值渲染像素字号，`xlog` 决定标签格式
/// （log 轴用科学计数 `{:.1e}`）。返回稀释后的刻度；无需稀释时原样返回。
fn thin_x_ticks(
    ticks: Vec<f64>,
    x_min: f64,
    x_max: f64,
    plot_px_w: f64,
    font_size: f64,
    xlog: bool,
) -> Vec<f64> {
    if ticks.len() <= 2 || plot_px_w < 1.0 || font_size <= 0.0 {
        return ticks;
    }
    let fmt = |v: f64| -> String {
        if xlog {
            format!("{:.1e}", 10f64.powf(v))
        } else {
            format_linear_tick(v)
        }
    };
    let max_w = ticks
        .iter()
        .map(|&v| measure_label_px(&fmt(v), font_size))
        .fold(0.0_f64, f64::max);
    if max_w <= 0.0 {
        return ticks;
    }
    // 相邻刻度值中心间距至少为「标签宽 + 约一个字符空隙」，保证水平不重叠。
    let min_center = max_w + font_size * 0.6;
    let max_labels = ((plot_px_w / min_center).floor() as usize).max(2);
    if ticks.len() <= max_labels {
        return ticks;
    }
    nice_ticks_capped(x_min, x_max, max_labels)
}

const MAX_MAJOR_TICKS_FOR_MINOR: usize = 30;
const MAX_MINOR_TICKS: usize = 2000;
/// 副刻度的最小像素间距：小于这个值说明副刻度太密，应该用回退策略
const MIN_MINOR_TICK_PX_SPACING: f64 = 3.0;

/// 主/副刻度计算结果
pub struct TicksInfo {
    pub xticks: Vec<f64>,
    pub yticks: Vec<f64>,
    pub xminor: Option<Vec<f64>>,
    pub yminor: Option<Vec<f64>>,
}

/// 计算 X/Y 主刻度与副刻度
///
/// 优先级（与 matplotlib 保持一致）：
/// 1. 用户通过 `set_xaxis_major_locator`/`set_yaxis_major_locator` 设置的 Python locator
/// 2. 用户通过 `xticks`/`yticks` 设置的位置
/// 3. `nice_ticks` 自动计算的"漂亮"刻度
///
/// 副刻度：
/// 1. 用户通过 `set_xaxis_minor_locator`/`set_yaxis_minor_locator` 设置的 Python locator
/// 2. 自动按主刻度等分（4 等分）
#[allow(clippy::too_many_arguments)]
pub fn compute_ticks(
    py: Python<'_>,
    xticks_val: &Option<Vec<f64>>,
    yticks_val: &Option<Vec<f64>>,
    xaxis_major_locator: &Option<Py<PyAny>>,
    yaxis_major_locator: &Option<Py<PyAny>>,
    xaxis_minor_locator: &Option<Py<PyAny>>,
    yaxis_minor_locator: &Option<Py<PyAny>>,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_pixel_width: u32,
    plot_pixel_height: u32,
    minor_grid_x_visible: bool,
    minor_grid_y_visible: bool,
    minor_grid_visible: bool,
    x_tick_font_size: f64,
    xlog: bool,
) -> TicksInfo {
    // locator 优先于 xticks_val（与 matplotlib 行为一致）：
    // locator 是动态的（基于数据范围），xticks_val 是固定的。
    // 如果用户同时设置了二者，locator 应当生效。
    // 自动刻度（无 locator 也无显式 xticks）在标签较长、子图较窄时按像素宽度稀释，
    // 避免刻度值水平重叠；用户显式指定的刻度 / locator 保持原样。
    let x_auto = xaxis_major_locator.is_none() && xticks_val.is_none();
    let xticks: Vec<f64> = xaxis_major_locator
        .as_ref()
        .and_then(|locator| {
            locator
                .bind(py)
                .call_method1("tick_values", (x_min, x_max))
                .ok()
                .and_then(|r| r.extract::<Vec<f64>>().ok())
        })
        .or_else(|| xticks_val.clone())
        .unwrap_or_else(|| nice_ticks(x_min, x_max));
    let xticks = if x_auto {
        thin_x_ticks(
            xticks,
            x_min,
            x_max,
            plot_pixel_width as f64,
            x_tick_font_size,
            xlog,
        )
    } else {
        xticks
    };

    let yticks: Vec<f64> = yaxis_major_locator
        .as_ref()
        .and_then(|locator| {
            locator
                .bind(py)
                .call_method1("tick_values", (y_min, y_max))
                .ok()
                .and_then(|r| r.extract::<Vec<f64>>().ok())
        })
        .or_else(|| yticks_val.clone())
        .unwrap_or_else(|| nice_ticks(y_min, y_max));

    let x_pw_approx = (plot_pixel_width as f64).max(1.0);
    let y_ph_approx = (plot_pixel_height as f64).max(1.0);

    let should_compute_x_minor =
        minor_grid_x_visible || !minor_grid_y_visible && minor_grid_visible;
    let xminor = compute_minor_ticks(
        py,
        xaxis_minor_locator,
        &xticks,
        x_min,
        x_max,
        x_pw_approx,
        (x_max - x_min) / x_pw_approx,
        should_compute_x_minor,
    );
    let should_compute_y_minor =
        minor_grid_y_visible || !minor_grid_x_visible && minor_grid_visible;
    let yminor = compute_minor_ticks(
        py,
        yaxis_minor_locator,
        &yticks,
        y_min,
        y_max,
        y_ph_approx,
        (y_max - y_min) / y_ph_approx,
        should_compute_y_minor,
    );

    TicksInfo {
        xticks,
        yticks,
        xminor,
        yminor,
    }
}

fn compute_minor_ticks(
    py: Python<'_>,
    locator: &Option<Py<PyAny>>,
    major_ticks: &[f64],
    axis_min: f64,
    axis_max: f64,
    _plot_pixels: f64,
    units_per_pix: f64,
    should_compute: bool,
) -> Option<Vec<f64>> {
    // 优先使用用户设置的 Python locator
    let ticks_opt = locator.as_ref().and_then(|loc| {
        loc.bind(py)
            .call_method1("tick_values", (axis_min, axis_max))
            .ok()
            .and_then(|r| r.extract::<Vec<f64>>().ok())
    });

    if let Some(ticks) = ticks_opt {
        if ticks.len() > MAX_MINOR_TICKS {
            return None;
        }
        if ticks.len() < 2 {
            return Some(ticks);
        }
        // 检查副刻度密度：若像素间距 < MIN_MINOR_TICK_PX_SPACING 则回退
        let min_spacing = ticks
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(f64::INFINITY, f64::min);
        let min_spacing_px = if units_per_pix > 0.0 {
            min_spacing / units_per_pix
        } else {
            0.0
        };
        if min_spacing_px < MIN_MINOR_TICK_PX_SPACING {
            None
        } else {
            Some(ticks)
        }
    } else if should_compute {
        if major_ticks.len() < 2 || major_ticks.len() > MAX_MAJOR_TICKS_FOR_MINOR {
            return None;
        }
        let mut minor = Vec::with_capacity(major_ticks.len().saturating_sub(1) * 4);
        for i in 0..major_ticks.len().saturating_sub(1) {
            let spacing = major_ticks[i + 1] - major_ticks[i];
            if spacing <= 0.0 {
                continue;
            }
            let step = spacing / 4.0;
            let mut v = major_ticks[i] + step;
            while v < major_ticks[i + 1] - step * 0.5 {
                if v > axis_min && v < axis_max {
                    minor.push(v);
                }
                v += step;
            }
        }
        if minor.is_empty() || minor.len() > MAX_MINOR_TICKS {
            None
        } else {
            Some(minor)
        }
    } else {
        None
    }
}

/// 网格/刻度绘制所需的网格颜色与线宽配置
pub struct GridStyle {
    pub major_color: RgbColor,
    pub major_lw: f64,
    pub major_ls: Option<String>,
    pub minor_color: RgbColor,
    pub minor_lw: f64,
    pub minor_ls: Option<String>,
}

/// 计算主/副网格线的颜色与线宽
pub fn compute_grid_style(
    grid_color: &Option<String>,
    grid_linewidth: Option<f64>,
    grid_linestyle: &Option<String>,
    minor_grid_color: &Option<String>,
    minor_grid_linewidth: Option<f64>,
    minor_grid_linestyle: &Option<String>,
) -> GridStyle {
    // matplotlib 默认主网格颜色（约 0.6 alpha 后的 153,153,153）
    let major_color = if let Some(c) = grid_color {
        parse_color(c, 0).unwrap_or(RgbColor(153, 153, 153))
    } else {
        RgbColor(153, 153, 153)
    };
    let major_lw = grid_linewidth.unwrap_or(0.8);

    // matplotlib minor grid 默认使用比主网格更浅的灰色（约 200,200,200）
    let minor_color = if let Some(c) = minor_grid_color {
        parse_color(c, 0).unwrap_or(RgbColor(200, 200, 200))
    } else {
        RgbColor(200, 200, 200)
    };
    let minor_lw = minor_grid_linewidth.unwrap_or(0.4);

    GridStyle {
        major_color,
        major_lw,
        major_ls: grid_linestyle.clone(),
        minor_color,
        minor_lw,
        minor_ls: minor_grid_linestyle.clone(),
    }
}

/// 格式化线性刻度标签：整数显示为不带 ".0"，小数保留最多两位有效数字
pub fn format_linear_tick(val: f64) -> String {
    if (val - val.round()).abs() < 1e-9 {
        format!("{}", val.round() as i64)
    } else {
        let s = format!("{:.2}", val);
        // 去掉末尾的 0 和可能的 .
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_owned()
    }
}
