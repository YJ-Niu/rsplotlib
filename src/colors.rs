use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use plotters::style::ShapeStyle;
use plotters::prelude::*;

#[derive(Clone, Copy)]
pub struct RgbColor(pub u8, pub u8, pub u8);

pub const DEFAULT_COLORS: &[&str] = &[
    "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd",
    "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf",
];

pub fn parse_color(name: &str, color_idx: usize) -> PyResult<RgbColor> {
    let trimmed = name.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
            return Ok(RgbColor(r, g, b));
        }
        return Err(PyValueError::new_err("Hex color must be #RRGGBB"));
    }
    let c = match trimmed.to_lowercase().as_str() {
        "r" | "red" => RgbColor(255, 0, 0),
        "g" | "green" => RgbColor(0, 128, 0),
        "b" | "blue" => RgbColor(0, 0, 255),
        "c" | "cyan" => RgbColor(0, 255, 255),
        "m" | "magenta" => RgbColor(255, 0, 255),
        "y" | "yellow" => RgbColor(255, 255, 0),
        "k" | "black" => RgbColor(0, 0, 0),
        "w" | "white" => RgbColor(255, 255, 255),
        "grey" | "gray" => RgbColor(128, 128, 128),
        "darkgrey" | "darkgray" => RgbColor(169, 169, 169),
        "lightgrey" | "lightgray" => RgbColor(211, 211, 211),
        "dimgrey" | "dimgray" => RgbColor(105, 105, 105),
        "slategrey" | "slategray" => RgbColor(112, 128, 144),
        _ => return Ok(default_color(color_idx)),
    };
    Ok(c)
}

pub fn default_color(idx: usize) -> RgbColor {
    let hex = DEFAULT_COLORS[idx % DEFAULT_COLORS.len()];
    let hex = hex.strip_prefix('#').unwrap();
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    RgbColor(r, g, b)
}

pub fn default_color_str(idx: usize) -> String {
    DEFAULT_COLORS[idx % DEFAULT_COLORS.len()].to_string()
}

pub fn shape_style(color: RgbColor, linewidth: f64, linestyle: &str) -> ShapeStyle {
    let rgb = RGBColor(color.0, color.1, color.2);
    match linestyle {
        "--" => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
        ":" => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
        "-." => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
        _ => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
    }
}

pub fn to_plotters_color(c: RgbColor) -> RGBColor {
    RGBColor(c.0, c.1, c.2)
}

pub fn median(data: &[f64]) -> f64 {
    let n = data.len();
    if n == 0 { return 0.0; }
    if n % 2 == 0 {
        (data[n / 2 - 1] + data[n / 2]) / 2.0
    } else {
        data[n / 2]
    }
}
