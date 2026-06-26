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

use pyo3::prelude::*;

use crate::core::colors::{RgbColor, parse_color};

/// matplotlib 兼容的"漂亮"主刻度算法
pub fn nice_ticks(min: f64, max: f64) -> Vec<f64> {
    if min >= max || !min.is_finite() || !max.is_finite() {
        return vec![min, max];
    }
    let range = max - min;
    if range <= 0.0 { return vec![min]; }
    // 选择合适的步长（matplotlib 的 MaxNLocator 简化版）
    let rough = range / 7.0;
    let mag = 10f64.powf(rough.log10().floor());
    let norm = rough / mag;
    let step = if norm < 1.5 { mag }
               else if norm < 3.0 { 2.0 * mag }
               else if norm < 7.0 { 5.0 * mag }
               else { 10.0 * mag };
    let start = (min / step).ceil() * step;
    let end = (max / step).floor() * step;
    let n = ((end + step * 0.001 - start) / step).ceil() as usize + 1;
    let mut ticks = Vec::with_capacity(n);
    let mut t = start;
    while t <= end + step * 0.001 {
        ticks.push(t);
        t += step;
    }
    if ticks.is_empty() { ticks.push(min); }
    ticks
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
) -> TicksInfo {
    // locator 优先于 xticks_val（与 matplotlib 行为一致）：
    // locator 是动态的（基于数据范围），xticks_val 是固定的。
    // 如果用户同时设置了二者，locator 应当生效。
    let xticks: Vec<f64> = xaxis_major_locator
        .as_ref()
        .and_then(|locator| {
            locator.bind(py).call_method1("tick_values", (x_min, x_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        })
        .or_else(|| xticks_val.clone())
        .unwrap_or_else(|| nice_ticks(x_min, x_max));

    let yticks: Vec<f64> = yaxis_major_locator
        .as_ref()
        .and_then(|locator| {
            locator.bind(py).call_method1("tick_values", (y_min, y_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        })
        .or_else(|| yticks_val.clone())
        .unwrap_or_else(|| nice_ticks(y_min, y_max));

    let x_pw_approx = (plot_pixel_width as f64).max(1.0);
    let y_ph_approx = (plot_pixel_height as f64).max(1.0);

    let should_compute_x_minor = minor_grid_x_visible || !minor_grid_y_visible && minor_grid_visible;
    let xminor = compute_minor_ticks(
        py, xaxis_minor_locator, &xticks, x_min, x_max, x_pw_approx,
        (x_max - x_min) / x_pw_approx, should_compute_x_minor,
    );
    let should_compute_y_minor = minor_grid_y_visible || !minor_grid_x_visible && minor_grid_visible;
    let yminor = compute_minor_ticks(
        py, yaxis_minor_locator, &yticks, y_min, y_max, y_ph_approx,
        (y_max - y_min) / y_ph_approx, should_compute_y_minor,
    );

    TicksInfo { xticks, yticks, xminor, yminor }
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
        loc.bind(py).call_method1("tick_values", (axis_min, axis_max))
            .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
    });

    if let Some(ticks) = ticks_opt {
        if ticks.len() > MAX_MINOR_TICKS { return None; }
        if ticks.len() < 2 { return Some(ticks); }
        // 检查副刻度密度：若像素间距 < MIN_MINOR_TICK_PX_SPACING 则回退
        let min_spacing = ticks.windows(2).map(|w| (w[1] - w[0]).abs())
            .fold(f64::INFINITY, f64::min);
        let min_spacing_px = if units_per_pix > 0.0 { min_spacing / units_per_pix } else { 0.0 };
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
            if spacing <= 0.0 { continue; }
            let step = spacing / 4.0;
            let mut v = major_ticks[i] + step;
            while v < major_ticks[i + 1] - step * 0.5 {
                if v > axis_min && v < axis_max {
                    minor.push(v);
                }
                v += step;
            }
        }
        if minor.is_empty() || minor.len() > MAX_MINOR_TICKS { None } else { Some(minor) }
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
