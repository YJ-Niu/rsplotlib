use plotters::prelude::RGBColor;

/// 对单个颜色通道做单调三次 Hermite 插值 (Fritsch–Carlson)。
///
/// 相比分段线性插值，三次 Hermite 在每个锚点保证一阶导数连续 (C1)，
/// 消除了锚点处的斜率突变（肉眼可见的 Mach 带 / 颜色分界），得到平滑渐变；
/// Fritsch–Carlson 的单调性修正确保不产生超出相邻锚点范围的过冲。
fn mono_cubic(t: f64, stops: &[(f64, u8, u8, u8)], ch: impl Fn(&(f64, u8, u8, u8)) -> f64) -> f64 {
    let n = stops.len();
    // 定位包含 t 的区间 [i, i+1]（调用方已保证 stops[0].0 < t < stops[n-1].0）
    let mut i = 0;
    while i + 2 < n && t >= stops[i + 1].0 {
        i += 1;
    }
    let x0 = stops[i].0;
    let x1 = stops[i + 1].0;
    let y0 = ch(&stops[i]);
    let y1 = ch(&stops[i + 1]);
    let h = (x1 - x0).max(1e-12);
    let d = (y1 - y0) / h; // 当前区间割线斜率

    // 端点用单侧斜率，内部用相邻割线的平均值
    let mut m0 = if i == 0 {
        d
    } else {
        let hp = (stops[i].0 - stops[i - 1].0).max(1e-12);
        0.5 * ((y0 - ch(&stops[i - 1])) / hp + d)
    };
    let mut m1 = if i + 2 >= n {
        d
    } else {
        let hn = (stops[i + 2].0 - stops[i + 1].0).max(1e-12);
        0.5 * (d + (ch(&stops[i + 2]) - y1) / hn)
    };

    // Fritsch–Carlson 单调性修正：避免过冲
    if d.abs() < 1e-12 {
        m0 = 0.0;
        m1 = 0.0;
    } else {
        let a = m0 / d;
        let b = m1 / d;
        let s = a * a + b * b;
        if s > 9.0 {
            let tau = 3.0 / s.sqrt();
            m0 = tau * a * d;
            m1 = tau * b * d;
        }
    }

    let s = (t - x0) / h;
    let s2 = s * s;
    let s3 = s2 * s;
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h10 = s3 - 2.0 * s2 + s;
    let h01 = -2.0 * s3 + 3.0 * s2;
    let h11 = s3 - s2;
    y0 * h00 + h * m0 * h10 + y1 * h01 + h * m1 * h11
}

/// 平滑颜色插值：对锚点表做单调三次 Hermite 插值，得到无斜率突变的平滑渐变。
fn interpolate(t: f64, stops: &[(f64, u8, u8, u8)]) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let last = stops.len() - 1;
    if t <= stops[0].0 {
        return RGBColor(stops[0].1, stops[0].2, stops[0].3);
    }
    if t >= stops[last].0 {
        return RGBColor(stops[last].1, stops[last].2, stops[last].3);
    }
    let r = mono_cubic(t, stops, |s| s.1 as f64)
        .round()
        .clamp(0.0, 255.0) as u8;
    let g = mono_cubic(t, stops, |s| s.2 as f64)
        .round()
        .clamp(0.0, 255.0) as u8;
    let b = mono_cubic(t, stops, |s| s.3 as f64)
        .round()
        .clamp(0.0, 255.0) as u8;
    RGBColor(r, g, b)
}

macro_rules! colormap_fn {
    ($name:ident, $stops:expr) => {
        pub fn $name(t: f64) -> RGBColor {
            interpolate(t, &$stops)
        }
    };
}

const VIRIDIS: [(f64, u8, u8, u8); 9] = [
    (0.0, 68, 1, 84),
    (0.125, 71, 44, 122),
    (0.25, 59, 82, 139),
    (0.375, 43, 120, 142),
    (0.5, 33, 145, 140),
    (0.625, 53, 178, 112),
    (0.75, 94, 201, 98),
    (0.875, 172, 229, 62),
    (1.0, 253, 231, 37),
];
colormap_fn!(viridis_color, VIRIDIS);

const PLASMA: [(f64, u8, u8, u8); 9] = [
    (0.0, 13, 8, 135),
    (0.125, 75, 3, 161),
    (0.25, 125, 3, 168),
    (0.375, 168, 34, 157),
    (0.5, 203, 70, 121),
    (0.625, 229, 107, 83),
    (0.75, 248, 150, 45),
    (0.875, 249, 198, 27),
    (1.0, 240, 249, 33),
];
colormap_fn!(plasma_color, PLASMA);

const INFERNO: [(f64, u8, u8, u8); 9] = [
    (0.0, 0, 0, 4),
    (0.125, 31, 12, 72),
    (0.25, 85, 15, 143),
    (0.375, 136, 34, 171),
    (0.5, 180, 55, 155),
    (0.625, 217, 81, 113),
    (0.75, 243, 120, 62),
    (0.875, 249, 170, 23),
    (1.0, 252, 225, 10),
];
colormap_fn!(inferno_color, INFERNO);

const MAGMA: [(f64, u8, u8, u8); 9] = [
    (0.0, 0, 0, 4),
    (0.125, 28, 16, 68),
    (0.25, 79, 18, 123),
    (0.375, 129, 23, 144),
    (0.5, 172, 43, 138),
    (0.625, 209, 69, 111),
    (0.75, 237, 103, 71),
    (0.875, 248, 148, 33),
    (1.0, 252, 196, 7),
];
colormap_fn!(magma_color, MAGMA);

pub fn cool_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor((t * 255.0) as u8, ((1.0 - t) * 255.0) as u8, 255)
}

pub fn spring_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(255, (t * 255.0) as u8, ((1.0 - t) * 255.0) as u8)
}

pub fn summer_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(
        (t * 255.0) as u8,
        (128.0 + t * 127.0) as u8,
        (64.0 * (1.0 - t)) as u8,
    )
}

pub fn autumn_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(255, (t * 255.0) as u8, 0)
}

pub fn winter_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(0, (t * 255.0) as u8, (255.0 * (1.0 - t * 0.5)) as u8)
}

/// afmhot: 黑 -> 红 -> 黄 -> 白 (matplotlib segmentdata 近似)
pub fn afmhot_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let r = (t * 2.0).clamp(0.0, 1.0);
    let g = (t * 2.0 - 0.5).clamp(0.0, 1.0);
    let b = (t * 2.0 - 1.0).clamp(0.0, 1.0);
    RGBColor((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// gist_heat: 黑 -> 红 -> 黄 -> 白
pub fn gist_heat_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let r = (t * 1.5).clamp(0.0, 1.0);
    let g = (t * 2.0 - 1.0).clamp(0.0, 1.0);
    let b = (t * 4.0 - 3.0).clamp(0.0, 1.0);
    RGBColor((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

const JET: [(f64, u8, u8, u8); 5] = [
    (0.0, 0, 0, 128),
    (0.35, 0, 255, 255),
    (0.5, 0, 255, 0),
    (0.65, 255, 255, 0),
    (1.0, 128, 0, 0),
];
colormap_fn!(jet_color, JET);

const RAINBOW: [(f64, u8, u8, u8); 6] = [
    (0.0, 128, 0, 255),
    (0.2, 0, 0, 255),
    (0.4, 0, 255, 255),
    (0.6, 0, 255, 0),
    (0.8, 255, 255, 0),
    (1.0, 255, 0, 0),
];
colormap_fn!(rainbow_color, RAINBOW);

const BWR: [(f64, u8, u8, u8); 3] = [(0.0, 0, 0, 255), (0.5, 255, 255, 255), (1.0, 255, 0, 0)];
colormap_fn!(bwr_color, BWR);

const SEISMIC: [(f64, u8, u8, u8); 5] = [
    (0.0, 0, 0, 76),
    (0.25, 0, 0, 255),
    (0.5, 255, 255, 255),
    (0.75, 255, 0, 0),
    (1.0, 128, 0, 0),
];
colormap_fn!(seismic_color, SEISMIC);

const COOLWARM: [(f64, u8, u8, u8); 3] =
    [(0.0, 59, 76, 192), (0.5, 221, 221, 221), (1.0, 180, 4, 38)];
colormap_fn!(coolwarm_color, COOLWARM);

const BONE: [(f64, u8, u8, u8); 4] = [
    (0.0, 0, 0, 0),
    (0.375, 84, 84, 116),
    (0.75, 169, 200, 200),
    (1.0, 255, 255, 255),
];
colormap_fn!(bone_color, BONE);

const COPPER: [(f64, u8, u8, u8); 2] = [(0.0, 0, 0, 0), (1.0, 255, 199, 127)];
colormap_fn!(copper_color, COPPER);

const PINK: [(f64, u8, u8, u8); 3] = [(0.0, 30, 0, 0), (0.5, 179, 129, 129), (1.0, 255, 255, 255)];
colormap_fn!(pink_color, PINK);

const BLUES: [(f64, u8, u8, u8); 4] = [
    (0.0, 247, 251, 255),
    (0.35, 158, 202, 225),
    (0.7, 49, 130, 189),
    (1.0, 8, 48, 107),
];
colormap_fn!(blues_color, BLUES);

const GREENS: [(f64, u8, u8, u8); 4] = [
    (0.0, 247, 252, 245),
    (0.35, 161, 217, 155),
    (0.7, 49, 163, 84),
    (1.0, 0, 68, 27),
];
colormap_fn!(greens_color, GREENS);

const REDS: [(f64, u8, u8, u8); 4] = [
    (0.0, 255, 245, 240),
    (0.35, 252, 146, 114),
    (0.7, 222, 45, 38),
    (1.0, 103, 0, 13),
];
colormap_fn!(reds_color, REDS);

const ORANGES: [(f64, u8, u8, u8); 4] = [
    (0.0, 255, 245, 235),
    (0.35, 253, 174, 107),
    (0.7, 230, 85, 13),
    (1.0, 127, 39, 4),
];
colormap_fn!(oranges_color, ORANGES);

const PURPLES: [(f64, u8, u8, u8); 4] = [
    (0.0, 252, 251, 253),
    (0.35, 188, 189, 220),
    (0.7, 117, 107, 177),
    (1.0, 63, 0, 125),
];
colormap_fn!(purples_color, PURPLES);

const GREYS: [(f64, u8, u8, u8); 2] = [(0.0, 255, 255, 255), (1.0, 0, 0, 0)];
colormap_fn!(greys_color, GREYS);

const SPECTRAL: [(f64, u8, u8, u8); 6] = [
    (0.0, 158, 1, 66),
    (0.2, 244, 109, 67),
    (0.4, 254, 224, 139),
    (0.6, 230, 245, 152),
    (0.8, 102, 194, 165),
    (1.0, 94, 79, 162),
];
colormap_fn!(spectral_color, SPECTRAL);

const TERRAIN: [(f64, u8, u8, u8); 5] = [
    (0.0, 51, 51, 153),
    (0.15, 0, 153, 255),
    (0.25, 51, 204, 102),
    (0.5, 255, 255, 153),
    (1.0, 255, 255, 255),
];
colormap_fn!(terrain_color, TERRAIN);

const OCEAN: [(f64, u8, u8, u8); 3] = [(0.0, 0, 128, 0), (0.5, 0, 0, 128), (1.0, 255, 255, 255)];
colormap_fn!(ocean_color, OCEAN);

const CIVIDIS: [(f64, u8, u8, u8); 5] = [
    (0.0, 0, 32, 76),
    (0.25, 47, 65, 108),
    (0.5, 124, 123, 120),
    (0.75, 187, 175, 113),
    (1.0, 255, 233, 69),
];
colormap_fn!(cividis_color, CIVIDIS);

const TWILIGHT: [(f64, u8, u8, u8); 5] = [
    (0.0, 226, 217, 226),
    (0.25, 89, 122, 189),
    (0.5, 42, 30, 76),
    (0.75, 152, 78, 92),
    (1.0, 226, 217, 226),
];
colormap_fn!(twilight_color, TWILIGHT);

const CUBEHELIX: [(f64, u8, u8, u8); 5] = [
    (0.0, 0, 0, 0),
    (0.25, 43, 55, 30),
    (0.5, 128, 80, 130),
    (0.75, 166, 173, 190),
    (1.0, 255, 255, 255),
];
colormap_fn!(cubehelix_color, CUBEHELIX);

const GIST_EARTH: [(f64, u8, u8, u8); 5] = [
    (0.0, 0, 0, 0),
    (0.25, 24, 84, 138),
    (0.5, 71, 152, 84),
    (0.75, 173, 165, 90),
    (1.0, 253, 250, 250),
];
colormap_fn!(gist_earth_color, GIST_EARTH);

const NIPY_SPECTRAL: [(f64, u8, u8, u8); 7] = [
    (0.0, 0, 0, 0),
    (0.16, 128, 0, 153),
    (0.33, 0, 0, 217),
    (0.5, 0, 179, 179),
    (0.66, 0, 179, 0),
    (0.83, 255, 153, 0),
    (1.0, 204, 204, 204),
];
colormap_fn!(nipy_spectral_color, NIPY_SPECTRAL);

fn hsv_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let h = t * 6.0;
    let i = h.floor() as i32 % 6;
    let f = h - h.floor();
    let (r, g, b) = match i {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    RGBColor((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// 按名称将归一化值 t (0.0-1.0) 映射为颜色，未知名称回退到 viridis。
/// 供 imshow、scatter (数值 c + cmap) 等统一使用。
///
/// 支持 matplotlib 的 `_r` 反转变体：名称以 `_r` 结尾时，用 `1.0 - t` 取基础
/// colormap 的反向颜色（如 `afmhot_r`、`viridis_r`、`jet_r`）。
pub fn colormap_color(name: &str, t: f64) -> RGBColor {
    if let Some(base) = name.strip_suffix("_r") {
        return colormap_color(base, 1.0 - t);
    }
    match name {
        "gray" | "grey" => {
            let v = (t.clamp(0.0, 1.0) * 255.0) as u8;
            RGBColor(v, v, v)
        }
        "gist_gray" | "gist_yarg" => {
            let v = (t.clamp(0.0, 1.0) * 255.0) as u8;
            RGBColor(v, v, v)
        }
        "binary" => {
            let v = ((1.0 - t.clamp(0.0, 1.0)) * 255.0) as u8;
            RGBColor(v, v, v)
        }
        "Greys" => greys_color(t),
        "hot" => {
            let r = (t * 3.0).clamp(0.0, 1.0);
            let g = (t * 3.0 - 1.0).clamp(0.0, 1.0);
            let b = (t * 3.0 - 2.0).clamp(0.0, 1.0);
            RGBColor((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        }
        "afmhot" => afmhot_color(t),
        "gist_heat" => gist_heat_color(t),
        "plasma" => plasma_color(t),
        "inferno" => inferno_color(t),
        "magma" => magma_color(t),
        "cividis" => cividis_color(t),
        "cool" => cool_color(t),
        "spring" => spring_color(t),
        "summer" => summer_color(t),
        "autumn" => autumn_color(t),
        "winter" => winter_color(t),
        "jet" => jet_color(t),
        "rainbow" | "gist_rainbow" => rainbow_color(t),
        "hsv" => hsv_color(t),
        "bwr" => bwr_color(t),
        "seismic" => seismic_color(t),
        "coolwarm" => coolwarm_color(t),
        "bone" => bone_color(t),
        "copper" => copper_color(t),
        "pink" => pink_color(t),
        "Blues" => blues_color(t),
        "Greens" => greens_color(t),
        "Reds" => reds_color(t),
        "Oranges" => oranges_color(t),
        "Purples" => purples_color(t),
        "Spectral" => spectral_color(t),
        "terrain" => terrain_color(t),
        "ocean" => ocean_color(t),
        "twilight" | "twilight_shifted" => twilight_color(t),
        "cubehelix" => cubehelix_color(t),
        "gist_earth" => gist_earth_color(t),
        "nipy_spectral" => nipy_spectral_color(t),
        _ => viridis_color(t),
    }
}
