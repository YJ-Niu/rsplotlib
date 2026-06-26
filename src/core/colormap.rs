use plotters::prelude::RGBColor;

/// 分段线性颜色插值，消除 8 个 colormap 函数的代码重复
fn interpolate(t: f64, stops: &[(f64, u8, u8, u8)]) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    if t <= stops[0].0 {
        return RGBColor(stops[0].1, stops[0].2, stops[0].3);
    }
    let last = stops.len() - 1;
    if t >= stops[last].0 {
        return RGBColor(stops[last].1, stops[last].2, stops[last].3);
    }
    for i in 0..last {
        let (t1, r1, g1, b1) = stops[i];
        let (t2, r2, g2, b2) = stops[i + 1];
        if t >= t1 && t <= t2 {
            let frac = if (t2 - t1).abs() < 1e-10 { 0.0 } else { (t - t1) / (t2 - t1) };
            return RGBColor(
                (r1 as f64 + (r2 as f64 - r1 as f64) * frac) as u8,
                (g1 as f64 + (g2 as f64 - g1 as f64) * frac) as u8,
                (b1 as f64 + (b2 as f64 - b1 as f64) * frac) as u8,
            );
        }
    }
    RGBColor(stops[last].1, stops[last].2, stops[last].3)
}

macro_rules! colormap_fn {
    ($name:ident, $stops:expr) => {
        pub fn $name(t: f64) -> RGBColor {
            interpolate(t, &$stops)
        }
    };
}

const VIRIDIS: [(f64, u8, u8, u8); 9] = [
    (0.0, 68, 1, 84), (0.125, 71, 44, 122), (0.25, 59, 82, 139),
    (0.375, 43, 120, 142), (0.5, 33, 145, 140), (0.625, 53, 178, 112),
    (0.75, 94, 201, 98), (0.875, 172, 229, 62), (1.0, 253, 231, 37),
];
colormap_fn!(viridis_color, VIRIDIS);

const PLASMA: [(f64, u8, u8, u8); 9] = [
    (0.0, 13, 8, 135), (0.125, 75, 3, 161), (0.25, 125, 3, 168),
    (0.375, 168, 34, 157), (0.5, 203, 70, 121), (0.625, 229, 107, 83),
    (0.75, 248, 150, 45), (0.875, 249, 198, 27), (1.0, 240, 249, 33),
];
colormap_fn!(plasma_color, PLASMA);

const INFERNO: [(f64, u8, u8, u8); 9] = [
    (0.0, 0, 0, 4), (0.125, 31, 12, 72), (0.25, 85, 15, 143),
    (0.375, 136, 34, 171), (0.5, 180, 55, 155), (0.625, 217, 81, 113),
    (0.75, 243, 120, 62), (0.875, 249, 170, 23), (1.0, 252, 225, 10),
];
colormap_fn!(inferno_color, INFERNO);

const MAGMA: [(f64, u8, u8, u8); 9] = [
    (0.0, 0, 0, 4), (0.125, 28, 16, 68), (0.25, 79, 18, 123),
    (0.375, 129, 23, 144), (0.5, 172, 43, 138), (0.625, 209, 69, 111),
    (0.75, 237, 103, 71), (0.875, 248, 148, 33), (1.0, 252, 196, 7),
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
    RGBColor((t * 255.0) as u8, (128.0 + t * 127.0) as u8, (64.0 * (1.0 - t)) as u8)
}

pub fn autumn_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(255, (t * 255.0) as u8, 0)
}

pub fn winter_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(0, (t * 255.0) as u8, (255.0 * (1.0 - t * 0.5)) as u8)
}