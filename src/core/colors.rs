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
        // 其余 CSS/HTML 命名颜色（与 matplotlib / HTML 颜色值一致）
        "aliceblue" => Some(RgbColor(240, 248, 255)),
        "antiquewhite" => Some(RgbColor(250, 235, 215)),
        "aqua" => Some(RgbColor(0, 255, 255)),
        "aquamarine" => Some(RgbColor(127, 255, 212)),
        "azure" => Some(RgbColor(240, 255, 255)),
        "beige" => Some(RgbColor(245, 245, 220)),
        "bisque" => Some(RgbColor(255, 228, 196)),
        "blanchedalmond" => Some(RgbColor(255, 235, 205)),
        "blueviolet" => Some(RgbColor(138, 43, 226)),
        "brown" => Some(RgbColor(165, 42, 42)),
        "burlywood" => Some(RgbColor(222, 184, 135)),
        "cadetblue" => Some(RgbColor(95, 158, 160)),
        "chartreuse" => Some(RgbColor(127, 255, 0)),
        "chocolate" => Some(RgbColor(210, 105, 30)),
        "coral" => Some(RgbColor(255, 127, 80)),
        "cornflowerblue" => Some(RgbColor(100, 149, 237)),
        "cornsilk" => Some(RgbColor(255, 248, 220)),
        "crimson" => Some(RgbColor(220, 20, 60)),
        "darkblue" => Some(RgbColor(0, 0, 139)),
        "darkcyan" => Some(RgbColor(0, 139, 139)),
        "darkgoldenrod" => Some(RgbColor(184, 134, 11)),
        "darkgreen" => Some(RgbColor(0, 100, 0)),
        "darkkhaki" => Some(RgbColor(189, 183, 107)),
        "darkmagenta" => Some(RgbColor(139, 0, 139)),
        "darkolivegreen" => Some(RgbColor(85, 107, 47)),
        "darkorange" => Some(RgbColor(255, 140, 0)),
        "darkorchid" => Some(RgbColor(153, 50, 204)),
        "darkred" => Some(RgbColor(139, 0, 0)),
        "darksalmon" => Some(RgbColor(233, 150, 122)),
        "darkseagreen" => Some(RgbColor(143, 188, 143)),
        "darkslateblue" => Some(RgbColor(72, 61, 139)),
        "darkslategrey" | "darkslategray" => Some(RgbColor(47, 79, 79)),
        "darkturquoise" => Some(RgbColor(0, 206, 209)),
        "darkviolet" => Some(RgbColor(148, 0, 211)),
        "deeppink" => Some(RgbColor(255, 20, 147)),
        "deepskyblue" => Some(RgbColor(0, 191, 255)),
        "dodgerblue" => Some(RgbColor(30, 144, 255)),
        "firebrick" => Some(RgbColor(178, 34, 34)),
        "floralwhite" => Some(RgbColor(255, 250, 240)),
        "forestgreen" => Some(RgbColor(34, 139, 34)),
        "fuchsia" => Some(RgbColor(255, 0, 255)),
        "gainsboro" => Some(RgbColor(220, 220, 220)),
        "ghostwhite" => Some(RgbColor(248, 248, 255)),
        "gold" => Some(RgbColor(255, 215, 0)),
        "goldenrod" => Some(RgbColor(218, 165, 32)),
        "greenyellow" => Some(RgbColor(173, 255, 47)),
        "honeydew" => Some(RgbColor(240, 255, 240)),
        "hotpink" => Some(RgbColor(255, 105, 180)),
        "indianred" => Some(RgbColor(205, 92, 92)),
        "indigo" => Some(RgbColor(75, 0, 130)),
        "ivory" => Some(RgbColor(255, 255, 240)),
        "khaki" => Some(RgbColor(240, 230, 140)),
        "lavender" => Some(RgbColor(230, 230, 250)),
        "lavenderblush" => Some(RgbColor(255, 240, 245)),
        "lawngreen" => Some(RgbColor(124, 252, 0)),
        "lemonchiffon" => Some(RgbColor(255, 250, 205)),
        "lightblue" => Some(RgbColor(173, 216, 230)),
        "lightcoral" => Some(RgbColor(240, 128, 128)),
        "lightcyan" => Some(RgbColor(224, 255, 255)),
        "lightgoldenrodyellow" => Some(RgbColor(250, 250, 210)),
        "lightgreen" => Some(RgbColor(144, 238, 144)),
        "lightpink" => Some(RgbColor(255, 182, 193)),
        "lightsalmon" => Some(RgbColor(255, 160, 122)),
        "lightseagreen" => Some(RgbColor(32, 178, 170)),
        "lightskyblue" => Some(RgbColor(135, 206, 250)),
        "lightslategrey" | "lightslategray" => Some(RgbColor(119, 136, 153)),
        "lightsteelblue" => Some(RgbColor(176, 196, 222)),
        "lightyellow" => Some(RgbColor(255, 255, 224)),
        "lime" => Some(RgbColor(0, 255, 0)),
        "limegreen" => Some(RgbColor(50, 205, 50)),
        "linen" => Some(RgbColor(250, 240, 230)),
        "maroon" => Some(RgbColor(128, 0, 0)),
        "mediumaquamarine" => Some(RgbColor(102, 205, 170)),
        "mediumblue" => Some(RgbColor(0, 0, 205)),
        "mediumorchid" => Some(RgbColor(186, 85, 211)),
        "mediumpurple" => Some(RgbColor(147, 112, 219)),
        "mediumseagreen" => Some(RgbColor(60, 179, 113)),
        "mediumslateblue" => Some(RgbColor(123, 104, 238)),
        "mediumspringgreen" => Some(RgbColor(0, 250, 154)),
        "mediumturquoise" => Some(RgbColor(72, 209, 204)),
        "mediumvioletred" => Some(RgbColor(199, 21, 133)),
        "midnightblue" => Some(RgbColor(25, 25, 112)),
        "mintcream" => Some(RgbColor(245, 255, 250)),
        "mistyrose" => Some(RgbColor(255, 228, 225)),
        "moccasin" => Some(RgbColor(255, 228, 181)),
        "navajowhite" => Some(RgbColor(255, 222, 173)),
        "navy" => Some(RgbColor(0, 0, 128)),
        "oldlace" => Some(RgbColor(253, 245, 230)),
        "olive" => Some(RgbColor(128, 128, 0)),
        "olivedrab" => Some(RgbColor(107, 142, 35)),
        "orange" => Some(RgbColor(255, 165, 0)),
        "orangered" => Some(RgbColor(255, 69, 0)),
        "orchid" => Some(RgbColor(218, 112, 214)),
        "palegoldenrod" => Some(RgbColor(238, 232, 170)),
        "palegreen" => Some(RgbColor(152, 251, 152)),
        "paleturquoise" => Some(RgbColor(175, 238, 238)),
        "palevioletred" => Some(RgbColor(219, 112, 147)),
        "papayawhip" => Some(RgbColor(255, 239, 213)),
        "peachpuff" => Some(RgbColor(255, 218, 185)),
        "peru" => Some(RgbColor(205, 133, 63)),
        "pink" => Some(RgbColor(255, 192, 203)),
        "plum" => Some(RgbColor(221, 160, 221)),
        "powderblue" => Some(RgbColor(176, 224, 230)),
        "purple" => Some(RgbColor(128, 0, 128)),
        "rebeccapurple" => Some(RgbColor(102, 51, 153)),
        "rosybrown" => Some(RgbColor(188, 143, 143)),
        "royalblue" => Some(RgbColor(65, 105, 225)),
        "saddlebrown" => Some(RgbColor(139, 69, 19)),
        "salmon" => Some(RgbColor(250, 128, 114)),
        "sandybrown" => Some(RgbColor(244, 164, 96)),
        "seagreen" => Some(RgbColor(46, 139, 87)),
        "seashell" => Some(RgbColor(255, 245, 238)),
        "sienna" => Some(RgbColor(160, 82, 45)),
        "silver" => Some(RgbColor(192, 192, 192)),
        "skyblue" => Some(RgbColor(135, 206, 235)),
        "slateblue" => Some(RgbColor(106, 90, 205)),
        "snow" => Some(RgbColor(255, 250, 250)),
        "springgreen" => Some(RgbColor(0, 255, 127)),
        "steelblue" => Some(RgbColor(70, 130, 180)),
        "tan" => Some(RgbColor(210, 180, 140)),
        "teal" => Some(RgbColor(0, 128, 128)),
        "thistle" => Some(RgbColor(216, 191, 216)),
        "tomato" => Some(RgbColor(255, 99, 71)),
        "turquoise" => Some(RgbColor(64, 224, 208)),
        "violet" => Some(RgbColor(238, 130, 238)),
        "wheat" => Some(RgbColor(245, 222, 179)),
        "whitesmoke" => Some(RgbColor(245, 245, 245)),
        "yellowgreen" => Some(RgbColor(154, 205, 50)),
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