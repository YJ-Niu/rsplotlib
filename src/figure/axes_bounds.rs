//! 数据范围计算模块
//!
//! 计算 axes 的 X/Y 数据范围（min/max），考虑：
//! - 所有 PlotElement 的数据范围
//! - 用户通过 `set_xlim`/`set_ylim` 设置的显式范围
//! - log 刻度下的 log10 变换
//! - 自动添加 5% 边距
//!
//! 主要 API：
//! - `compute_bounds()`: 计算 X/Y 数据范围

use crate::core::elements::PlotElement;

/// 对 log 刻度轴的数据值进行 log10 转换
fn log_transform(val: f64) -> f64 {
    if val > 0.0 {
        val.log10()
    } else {
        f64::NEG_INFINITY
    }
}

/// 计算 axes 的 X/Y 数据范围
///
/// # 参数
/// - `elements`: 所有 plot 调用收集的元素
/// - `xlim`: 用户显式设置的 X 范围（None 表示自动计算）
/// - `ylim`: 用户显式设置的 Y 范围（None 表示自动计算）
/// - `xlog`/`ylog`: 是否对数刻度
///
/// # 返回
/// `((x_min, x_max), (y_min, y_max))`
pub fn compute_bounds(
    elements: &[PlotElement],
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
    xlog: bool,
    ylog: bool,
) -> ((f64, f64), (f64, f64)) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    // 柱状图基线粘附 0（matplotlib sticky edges）：柱子全为正值时，对应轴的起点固定到
    // 0，且该侧不再追加 5% 留白，使柱子从坐标轴上"长出来"而非悬空。
    let mut y_sticky_min = false;
    let mut x_sticky_min = false;
    // imshow：图像应紧贴坐标轴，四周不留 5% 空白（与 matplotlib 一致）。
    let mut tight_image = false;

    let tx = |v: f64| if xlog { log_transform(v) } else { v };
    let ty = |v: f64| if ylog { log_transform(v) } else { v };

    for el in elements {
        match el {
            PlotElement::Line { x, y, .. } => {
                for v in x.iter().flatten() {
                    let tv = tx(*v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                for v in y.iter().flatten() {
                    let tv = ty(*v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
            }
            PlotElement::Scatter { x, y, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
            }
            PlotElement::Bar {
                x, height, width, ..
            } => {
                // 柱子居中于 x（align='center'），左右各延伸 width/2；用其真实边缘参与
                // 自动缩放，使首/末柱不被裁切，x 轴范围也对称贴合定义的 x 位置。
                let hw = *width / 2.0;
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv - hw < x_min {
                        x_min = tv - hw;
                    }
                    if tv + hw > x_max {
                        x_max = tv + hw;
                    }
                }
                for &v in height {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
                if !ylog && y_min > 0.0 {
                    y_min = 0.0;
                    y_sticky_min = true;
                }
            }
            PlotElement::BarH {
                y, width, height, ..
            } => {
                let hh = *height / 2.0;
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv - hh < y_min {
                        y_min = tv - hh;
                    }
                    if tv + hh > y_max {
                        y_max = tv + hh;
                    }
                }
                for &v in width {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                if !width.is_empty() {
                    let last_w = tx(width[width.len() - 1]);
                    if last_w > x_max {
                        x_max = last_w;
                    }
                }
                if !xlog && x_min > 0.0 {
                    x_min = 0.0;
                    x_sticky_min = true;
                }
            }
            PlotElement::Hist {
                bars,
                outlines,
                orientation,
                ..
            } => {
                let is_horizontal = orientation == "horizontal";
                // 收集所有 (pos, val) 顶点：柱子四角 + 轮廓折线点
                let mut pos_vals: Vec<(f64, f64)> = Vec::new();
                for ds in bars {
                    for &(pl, pr, vb, vt) in ds {
                        pos_vals.push((pl, vb));
                        pos_vals.push((pr, vt));
                    }
                }
                for ds in outlines {
                    for &(p, v) in ds {
                        pos_vals.push((p, v));
                    }
                }
                if pos_vals.is_empty() {
                    continue;
                }
                // 竖直: pos->x, val->y；水平: pos->y, val->x
                for &(pos, val) in &pos_vals {
                    let (dx, dy) = if is_horizontal {
                        (val, pos)
                    } else {
                        (pos, val)
                    };
                    let tvx = tx(dx);
                    let tvy = ty(dy);
                    if tvx > f64::NEG_INFINITY && tvx < x_min {
                        x_min = tvx;
                    }
                    if tvx > x_max {
                        x_max = tvx;
                    }
                    if tvy > f64::NEG_INFINITY && tvy < y_min {
                        y_min = tvy;
                    }
                    if tvy > y_max {
                        y_max = tvy;
                    }
                }
                // 计数轴基线粘附 0（数值全为正时，计数轴从 0 起始、下方不留白）：
                // 竖直方向作用于 y 轴，水平方向作用于 x 轴。
                if is_horizontal {
                    if !xlog && x_min >= 0.0 {
                        x_min = 0.0;
                        x_sticky_min = true;
                    }
                } else if !ylog && y_min >= 0.0 {
                    y_min = 0.0;
                    y_sticky_min = true;
                }
            }
            PlotElement::Image { pixels, .. } => {
                if pixels.is_empty() || pixels[0].is_empty() {
                    continue;
                }
                x_min = 0.0;
                x_max = pixels[0].len() as f64;
                y_min = 0.0;
                y_max = pixels.len() as f64;
                tight_image = true;
            }
            PlotElement::Text { x, y, .. } => {
                let tvx = tx(*x);
                let tvy = ty(*y);
                if tvx > f64::NEG_INFINITY && tvx < x_min {
                    x_min = tvx;
                }
                if tvx > x_max {
                    x_max = tvx;
                }
                if tvy > f64::NEG_INFINITY && tvy < y_min {
                    y_min = tvy;
                }
                if tvy > y_max {
                    y_max = tvy;
                }
            }
            PlotElement::HLine { y, .. } => {
                if x_min == f64::INFINITY {
                    x_min = -1.0;
                    x_max = 1.0;
                }
                let tvy = ty(*y);
                if tvy > f64::NEG_INFINITY && tvy < y_min {
                    y_min = tvy;
                }
                if tvy > y_max {
                    y_max = tvy;
                }
            }
            PlotElement::VLine { x, .. } => {
                if y_min == f64::INFINITY {
                    y_min = -1.0;
                    y_max = 1.0;
                }
                let tvx = tx(*x);
                if tvx > f64::NEG_INFINITY && tvx < x_min {
                    x_min = tvx;
                }
                if tvx > x_max {
                    x_max = tvx;
                }
            }
            PlotElement::Pie { .. } => {
                if x_min > -1.5 {
                    x_min = -1.5;
                }
                if x_max < 1.5 {
                    x_max = 1.5;
                }
                if y_min > -1.5 {
                    y_min = -1.5;
                }
                if y_max < 1.5 {
                    y_max = 1.5;
                }
            }
            PlotElement::FillBetween { x, y1, y2, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                for &v in y1 {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
                for &v in y2 {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
            }
            PlotElement::Stack { x, y_series, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                // 累加 y 以考虑堆叠后的最大值
                let n = x.len();
                let mut cumulative = vec![0.0; n];
                for series in y_series {
                    for (i, &v) in series.iter().enumerate() {
                        if i < n {
                            cumulative[i] += v;
                        }
                    }
                }
                for &v in &cumulative {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
            }
            PlotElement::ErrorBar { x, y, yerr, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
                if let Some(ye_vec) = yerr.as_ref() {
                    for (i, &yv) in y.iter().enumerate() {
                        let ye = if i < ye_vec.len() { ye_vec[i] } else { 0.0_f64 };
                        let tv_lo = ty(yv - ye);
                        let tv_hi = ty(yv + ye);
                        if tv_lo > f64::NEG_INFINITY && tv_lo < y_min {
                            y_min = tv_lo;
                        }
                        if tv_hi > y_max {
                            y_max = tv_hi;
                        }
                    }
                }
            }
            PlotElement::Stem { x, y, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
                if !ylog && y_min > 0.0 {
                    y_min = 0.0;
                }
            }
            PlotElement::Step { x, y, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
            }
            PlotElement::BoxPlot { data, .. } => {
                for series in data {
                    for &v in series {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min {
                            y_min = tv;
                        }
                        if tv > y_max {
                            y_max = tv;
                        }
                    }
                }
                if !ylog && y_min > 0.0 {
                    y_min = 0.0;
                }
                if !xlog && x_min > 0.0 {
                    x_min = 0.0;
                }
                let n = data.len() as f64;
                if n > x_max {
                    x_max = n + 1.0;
                }
            }
            PlotElement::Annotate { xy, xytext, .. } => {
                let (xv, yv) = *xy;
                let tvx = tx(xv);
                let tvy = ty(yv);
                if tvx > f64::NEG_INFINITY && tvx < x_min {
                    x_min = tvx;
                }
                if tvx > x_max {
                    x_max = tvx;
                }
                if tvy > f64::NEG_INFINITY && tvy < y_min {
                    y_min = tvy;
                }
                if tvy > y_max {
                    y_max = tvy;
                }
                if let Some((xt, yt)) = xytext {
                    let tvxt = tx(*xt);
                    let tvyt = ty(*yt);
                    if tvxt > f64::NEG_INFINITY && tvxt < x_min {
                        x_min = tvxt;
                    }
                    if tvxt > x_max {
                        x_max = tvxt;
                    }
                    if tvyt > f64::NEG_INFINITY && tvyt < y_min {
                        y_min = tvyt;
                    }
                    if tvyt > y_max {
                        y_max = tvyt;
                    }
                }
            }
            PlotElement::HSpan { y1, y2, .. } => {
                let tv1 = ty(*y1);
                let tv2 = ty(*y2);
                if tv1.min(tv2) > f64::NEG_INFINITY && tv1.min(tv2) < y_min {
                    y_min = tv1.min(tv2);
                }
                if tv1.max(tv2) > y_max {
                    y_max = tv1.max(tv2);
                }
                if x_min == f64::INFINITY {
                    x_min = -1.0;
                    x_max = 1.0;
                }
            }
            PlotElement::VSpan { x1, x2, .. } => {
                let tv1 = tx(*x1);
                let tv2 = tx(*x2);
                if tv1.min(tv2) > f64::NEG_INFINITY && tv1.min(tv2) < x_min {
                    x_min = tv1.min(tv2);
                }
                if tv1.max(tv2) > x_max {
                    x_max = tv1.max(tv2);
                }
                if y_min == f64::INFINITY {
                    y_min = -1.0;
                    y_max = 1.0;
                }
            }
            PlotElement::AxLine { xy1, xy2, .. } => {
                let (xv1, yv1) = *xy1;
                let (xv2, yv2) = *xy2;
                for (xv, yv) in [(xv1, yv1), (xv2, yv2)] {
                    let tvx = tx(xv);
                    let tvy = ty(yv);
                    if tvx > f64::NEG_INFINITY && tvx < x_min {
                        x_min = tvx;
                    }
                    if tvx > x_max {
                        x_max = tvx;
                    }
                    if tvy > f64::NEG_INFINITY && tvy < y_min {
                        y_min = tvy;
                    }
                    if tvy > y_max {
                        y_max = tvy;
                    }
                }
            }
            PlotElement::ScatterMulti { x, y, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min {
                        x_min = tv;
                    }
                    if tv > x_max {
                        x_max = tv;
                    }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min {
                        y_min = tv;
                    }
                    if tv > y_max {
                        y_max = tv;
                    }
                }
            }
            PlotElement::Arrow { x1, y1, x2, y2, .. } => {
                for (xv, yv) in [(*x1, *y1), (*x2, *y2)] {
                    let tvx = tx(xv);
                    let tvy = ty(yv);
                    if tvx > f64::NEG_INFINITY && tvx < x_min {
                        x_min = tvx;
                    }
                    if tvx > x_max {
                        x_max = tvx;
                    }
                    if tvy > f64::NEG_INFINITY && tvy < y_min {
                        y_min = tvy;
                    }
                    if tvy > y_max {
                        y_max = tvy;
                    }
                }
            }
        }
    }

    if x_min == f64::INFINITY {
        x_min = 0.0;
        x_max = 1.0;
    }
    if y_min == f64::INFINITY {
        y_min = 0.0;
        y_max = 1.0;
    }

    let x_range = x_max - x_min;
    let y_range = y_max - y_min;
    let x_pad = if x_range.abs() < 1e-10 {
        1.0
    } else {
        x_range * 0.05
    };
    let y_pad = if y_range.abs() < 1e-10 {
        1.0
    } else {
        y_range * 0.05
    };

    if let Some((l, r)) = xlim {
        x_min = l;
        x_max = r;
    } else if !tight_image {
        if !x_sticky_min {
            x_min -= x_pad;
        }
        x_max += x_pad;
    }
    if let Some((b, t)) = ylim {
        y_min = b;
        y_max = t;
    } else if !tight_image {
        if !y_sticky_min {
            y_min -= y_pad;
        }
        y_max += y_pad;
    }

    // 安全防护：确保 min <= max，避免 plotters 因反转范围而卡死
    if x_min > x_max {
        std::mem::swap(&mut x_min, &mut x_max);
    }
    if y_min > y_max {
        std::mem::swap(&mut y_min, &mut y_max);
    }

    ((x_min, x_max), (y_min, y_max))
}
