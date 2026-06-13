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

use crate::elements::PlotElement;

/// 对 log 刻度轴的数据值进行 log10 转换
fn log_transform(val: f64) -> f64 {
    if val > 0.0 { val.log10() } else { f64::NEG_INFINITY }
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

    let tx = |v: f64| if xlog { log_transform(v) } else { v };
    let ty = |v: f64| if ylog { log_transform(v) } else { v };

    for el in elements {
        match el {
            PlotElement::Line { x, y, .. } => {
                for v in x.iter().flatten() {
                    let tv = tx(*v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                for v in y.iter().flatten() {
                    let tv = ty(*v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
            }
            PlotElement::Scatter { x, y, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
            }
            PlotElement::Bar { x, height, width, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                for &v in height {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
                if !x.is_empty() && !height.is_empty() {
                    let last_x = tx(x[x.len() - 1]);
                    let bar_end = last_x + *width;
                    if bar_end > x_max { x_max = bar_end; }
                }
                if !ylog && y_min > 0.0 { y_min = 0.0; }
            }
            PlotElement::BarH { y, width, .. } => {
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
                for &v in width {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                if !width.is_empty() {
                    let last_w = tx(width[width.len() - 1]);
                    if last_w > x_max { x_max = last_w; }
                }
                if !xlog && x_min > 0.0 { x_min = 0.0; }
            }
            PlotElement::Hist { data_all, bins, density, bin_edges, .. } => {
                if data_all.is_empty() { continue; }
                let all_data: Vec<f64> = data_all.iter().flatten().cloned().collect();
                if all_data.is_empty() { continue; }
                let data_min = all_data.iter().cloned().fold(f64::INFINITY, f64::min);
                let data_max = all_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let (x_start, x_end) = if let Some(edges) = bin_edges {
                    (edges[0], edges[edges.len() - 1])
                } else {
                    (data_min, data_max)
                };
                let tx_start = tx(x_start);
                let tx_end = tx(x_end);
                if tx_start > f64::NEG_INFINITY && tx_start < x_min { x_min = tx_start; }
                if tx_end > x_max { x_max = tx_end; }
                let total = all_data.len() as f64;
                let mut max_count = 0.0f64;
                for dataset in data_all {
                    if dataset.is_empty() { continue; }
                    let d_min = dataset.iter().cloned().fold(f64::INFINITY, f64::min);
                    let d_max = dataset.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let d_range = d_max - d_min;
                    if d_range < 1e-10 {
                        // 所有值相同 -> 算一个单柱，计数为 dataset 长度
                        let dc = if *density { dataset.len() as f64 / total } else { dataset.len() as f64 };
                        if dc > max_count { max_count = dc; }
                        continue;
                    }
                    let bw = d_range / *bins as f64;
                    let mut counts = vec![0usize; *bins];
                    for &val in dataset {
                        let mut bin = ((val - d_min) / bw).floor() as usize;
                        if bin >= *bins { bin = *bins - 1; }
                        counts[bin] += 1;
                    }
                    let mc = counts.iter().max().unwrap_or(&0);
                    let dc = if *density { *mc as f64 / (total * bw) } else { *mc as f64 };
                    if dc > max_count { max_count = dc; }
                }
                if !ylog && y_min > 0.0 { y_min = 0.0; }
                let tmax = ty(max_count);
                if tmax > y_max { y_max = tmax; }
            }
            PlotElement::Image { data, .. } => {
                if data.is_empty() || data[0].is_empty() { continue; }
                x_min = 0.0;
                x_max = data[0].len() as f64;
                y_min = 0.0;
                y_max = data.len() as f64;
            }
            PlotElement::Text { x, y, .. } => {
                let tvx = tx(*x);
                let tvy = ty(*y);
                if tvx > f64::NEG_INFINITY && tvx < x_min { x_min = tvx; }
                if tvx > x_max { x_max = tvx; }
                if tvy > f64::NEG_INFINITY && tvy < y_min { y_min = tvy; }
                if tvy > y_max { y_max = tvy; }
            }
            PlotElement::HLine { y, .. } => {
                if x_min == f64::INFINITY { x_min = -1.0; x_max = 1.0; }
                let tvy = ty(*y);
                if tvy > f64::NEG_INFINITY && tvy < y_min { y_min = tvy; }
                if tvy > y_max { y_max = tvy; }
            }
            PlotElement::VLine { x, .. } => {
                if y_min == f64::INFINITY { y_min = -1.0; y_max = 1.0; }
                let tvx = tx(*x);
                if tvx > f64::NEG_INFINITY && tvx < x_min { x_min = tvx; }
                if tvx > x_max { x_max = tvx; }
            }
            PlotElement::Pie { .. } => {
                if x_min > -1.5 { x_min = -1.5; }
                if x_max < 1.5 { x_max = 1.5; }
                if y_min > -1.5 { y_min = -1.5; }
                if y_max < 1.5 { y_max = 1.5; }
            }
            PlotElement::FillBetween { x, y1, y2, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                for &v in y1 {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
                for &v in y2 {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
            }
            PlotElement::ErrorBar { x, y, yerr, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
                if let Some(ye_vec) = yerr.as_ref() {
                    for (i, &yv) in y.iter().enumerate() {
                        let ye = if i < ye_vec.len() { ye_vec[i] } else { 0.0_f64 };
                        let tv_lo = ty(yv - ye);
                        let tv_hi = ty(yv + ye);
                        if tv_lo > f64::NEG_INFINITY && tv_lo < y_min { y_min = tv_lo; }
                        if tv_hi > y_max { y_max = tv_hi; }
                    }
                }
            }
            PlotElement::Stem { x, y, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
                if !ylog && y_min > 0.0 { y_min = 0.0; }
            }
            PlotElement::Step { x, y, .. } => {
                for &v in x {
                    let tv = tx(v);
                    if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                    if tv > x_max { x_max = tv; }
                }
                for &v in y {
                    let tv = ty(v);
                    if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                    if tv > y_max { y_max = tv; }
                }
            }
            PlotElement::BoxPlot { data, .. } => {
                for series in data {
                    for &v in series {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                }
                if !ylog && y_min > 0.0 { y_min = 0.0; }
                if !xlog && x_min > 0.0 { x_min = 0.0; }
                let n = data.len() as f64;
                if n > x_max { x_max = n + 1.0; }
            }
            PlotElement::Annotate { xy, xytext, .. } => {
                let (xv, yv) = *xy;
                let tvx = tx(xv);
                let tvy = ty(yv);
                if tvx > f64::NEG_INFINITY && tvx < x_min { x_min = tvx; }
                if tvx > x_max { x_max = tvx; }
                if tvy > f64::NEG_INFINITY && tvy < y_min { y_min = tvy; }
                if tvy > y_max { y_max = tvy; }
                if let Some((xt, yt)) = xytext {
                    let tvxt = tx(*xt);
                    let tvyt = ty(*yt);
                    if tvxt > f64::NEG_INFINITY && tvxt < x_min { x_min = tvxt; }
                    if tvxt > x_max { x_max = tvxt; }
                    if tvyt > f64::NEG_INFINITY && tvyt < y_min { y_min = tvyt; }
                    if tvyt > y_max { y_max = tvyt; }
                }
            }
        }
    }

    if x_min == f64::INFINITY { x_min = 0.0; x_max = 1.0; }
    if y_min == f64::INFINITY { y_min = 0.0; y_max = 1.0; }

    let x_range = x_max - x_min;
    let y_range = y_max - y_min;
    let x_pad = if x_range.abs() < 1e-10 { 1.0 } else { x_range * 0.05 };
    let y_pad = if y_range.abs() < 1e-10 { 1.0 } else { y_range * 0.05 };

    if let Some((l, r)) = xlim {
        x_min = l;
        x_max = r;
    } else {
        x_min -= x_pad;
        x_max += x_pad;
    }
    if let Some((b, t)) = ylim {
        y_min = b;
        y_max = t;
    } else {
        y_min -= y_pad;
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
