use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use plotters::style::ShapeStyle;
use plotters::prelude::*;

#[derive(Clone, Copy)]
pub struct RgbColor(pub u8, pub u8, pub u8);

/// 预计算的 10 种默认颜色
const DEFAULT_COLOR_VALUES: [RgbColor; 10] = [
    RgbColor(31, 119, 180),  // #1f77b4
    RgbColor(255, 127, 14),  // #ff7f0e
    RgbColor(44, 160, 44),   // #2ca02c
    RgbColor(214, 39, 40),   // #d62728
    RgbColor(148, 103, 189), // #9467bd
    RgbColor(140, 86, 75),   // #8c564b
    RgbColor(227, 119, 194), // #e377c2
    RgbColor(127, 127, 127), // #7f7f7f
    RgbColor(188, 189, 34),  // #bcbd22
    RgbColor(23, 190, 207),  // #17becf
];

pub const DEFAULT_COLORS: &[&str] = &[
    "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd",
    "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf",
];

/// 颜色名到 RgbColor 的静态映射，直接匹配全名避免首字母冲突（如 "black" 以 b 开头但不是 blue）
fn named_color(name: &str) -> Option<RgbColor> {
    match name {
        "r" | "red" => Some(RgbColor(255, 0, 0)),
        "g" | "green" => Some(RgbColor(0, 128, 0)),
        "b" | "blue" => Some(RgbColor(0, 0, 255)),
        "c" | "cyan" => Some(RgbColor(0, 255, 255)),
        "m" | "magenta" => Some(RgbColor(255, 0, 255)),
        "y" | "yellow" => Some(RgbColor(255, 255, 0)),
        "k" | "black" => Some(RgbColor(0, 0, 0)),
        "w" | "white" => Some(RgbColor(255, 255, 255)),
        "grey" | "gray" => Some(RgbColor(128, 128, 128)),
        "darkgrey" | "darkgray" => Some(RgbColor(169, 169, 169)),
        "lightgrey" | "lightgray" => Some(RgbColor(211, 211, 211)),
        "dimgrey" | "dimgray" => Some(RgbColor(105, 105, 105)),
        "slategrey" | "slategray" => Some(RgbColor(112, 128, 144)),
        _ => None,
    }
}

fn parse_hex(hex: &str) -> PyResult<RgbColor> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return Err(PyValueError::new_err("Hex color must be #RRGGBB"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
    Ok(RgbColor(r, g, b))
}

pub fn parse_color(name: &str, color_idx: usize) -> PyResult<RgbColor> {
    let trimmed = name.trim();
    if trimmed.starts_with('#') {
        return parse_hex(trimmed);
    }
    // 精确匹配命名颜色（所有命名颜色都是小写）
    named_color(trimmed)
        .or_else(|| named_color(&trimmed.to_lowercase()))
        .map(Ok)
        .unwrap_or_else(|| Ok(default_color(color_idx)))
}

/// 预计算默认颜色，0 分配
pub fn default_color(idx: usize) -> RgbColor {
    DEFAULT_COLOR_VALUES[idx % DEFAULT_COLOR_VALUES.len()]
}

pub fn default_color_str(idx: usize) -> String {
    DEFAULT_COLORS[idx % DEFAULT_COLORS.len()].to_string()
}

pub fn shape_style(color: RgbColor, linewidth: f64, _linestyle: &str) -> ShapeStyle {
    let rgb = RGBColor(color.0, color.1, color.2);
    let lw_px = (linewidth).round().max(1.0) as u32;
    rgb.mix(1.0).stroke_width(lw_px)
}

pub fn to_plotters_color(c: RgbColor) -> RGBColor {
    RGBColor(c.0, c.1, c.2)
}

pub fn median(data: &[f64]) -> f64 {
    let n = data.len();
    if n == 0 { return 0.0; }
    if n.is_multiple_of(2) {
        (data[n / 2 - 1] + data[n / 2]) / 2.0
    } else {
        data[n / 2]
    }
}