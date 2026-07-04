use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyTuple};

use plotters::style::{FontStyle, register_font};

use crate::figure::axes::Axes;
use crate::figure::figure::{
    DEFAULT_DPI, DEFAULT_FIGSIZE, Figure, get_current_figure, set_current_figure,
};
use crate::utils::font_stack;

/// 从文件路径注册一个字体到字体系统。
///
/// 该方法执行以下操作：
/// 1. 读取字体文件到内存
/// 2. 如果传入了 family_name，直接使用；否则从字体文件中提取真实家族名称
/// 3. 用家族名称注册到 plotters 字体数据库
/// 4. 将字体数据推入全局 `font_stack`，用于后续 glyph 覆盖检测
///
/// 这样 Python 端 `plt.rcParams["font.sans-serif"] = ["Helvetica", "Arial Unicode MS"]`
/// 设置的多个字体可以形成"字体栈"，渲染时根据文本字符自动选择最佳字体。
///
/// # 参数
/// - `path`: 字体文件路径（.ttf/.otf/.ttc）
/// - `family_name`: 可选的字体族名。如果提供，直接使用；否则从字体文件中提取。
///
/// # 返回
/// - 成功返回 Ok(())
/// - 文件不存在或字体解析失败返回 Err
#[pyfunction]
#[pyo3(signature = (path, family_name=None))]
pub fn register_sans_serif_font(
    py: Python,
    path: String,
    family_name: Option<String>,
) -> PyResult<()> {
    let font_data = std::fs::read(&path)
        .map_err(|e| PyValueError::new_err(format!("Cannot read font file '{}': {}", path, e)))?;

    // 优先使用传入的 family_name，否则从字体文件中提取
    let family = match family_name {
        Some(name) if !name.is_empty() => name,
        _ => {
            font_stack::extract_family_name(&font_data).unwrap_or_else(|| "sans-serif".to_string())
        }
    };

    // 用家族名称注册到 plotters
    let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
    register_font(&family, FontStyle::Normal, font_ref)
        .map_err(|_| PyValueError::new_err(format!("Failed to register font from '{}'", path)))?;

    // 推入字体栈（重新读取，因为 font_data 已被 Box::leak 消耗）
    let font_data2 = std::fs::read(&path)
        .map_err(|e| PyValueError::new_err(format!("Cannot read font file '{}': {}", path, e)))?;
    font_stack::push_font(family, font_data2);

    let _ = py; // suppress unused warning
    Ok(())
}

pub fn get_current_axes(py: Python<'_>) -> PyResult<Py<Axes>> {
    // 已有当前 figure 时的处理
    if let Ok(fig) = get_current_figure(py) {
        {
            let fig_ref = fig.borrow();
            if !fig_ref.axes_list.is_empty() {
                // 返回当前选中的 axes（plt.subplot 选中的子图；否则为最近创建的那个）
                let idx = fig_ref.current_axes_index.min(fig_ref.axes_list.len() - 1);
                return Ok(fig_ref.axes_list[idx].clone_ref(py));
            }
        }
        // 当前 figure 存在但还没有 axes：向其补一个全幅 axes，
        // 保留用户已在 figure 上设置的属性（figsize / dpi 等）。
        let ax_py = Py::new(py, Axes::new())?;
        init_axes_self_py(&ax_py, py);
        let mut fig_mut = fig.borrow_mut();
        fig_mut.axes_list.push(ax_py.clone_ref(py));
        fig_mut.axes_positions.push((0.0, 1.0, 0.0, 1.0));
        fig_mut.current_axes_index = 0;
        return Ok(ax_py);
    }
    // 没有任何当前 figure：按 matplotlib gca() 语义惰性创建 figure + 全幅 axes，
    // 这样 title / xlabel / ylabel 等可在 plot 之前调用而不再报错。
    let (_fig_py, ax_py) = _make_fig_ax(py, Axes::new())?;
    Ok(ax_py)
}

pub fn init_axes_self_py(ax_py: &Py<Axes>, py: Python<'_>) {
    let obj: Py<PyAny> = ax_py.clone_ref(py).into();
    let mut ax_ref = ax_py.borrow_mut(py);
    ax_ref.self_py = Some(obj);
}

fn _make_fig_ax(py: Python<'_>, ax: Axes) -> PyResult<(Py<Figure>, Py<Axes>)> {
    let mut fig = Figure::new();
    fig.axes_list.clear();
    fig.axes_positions.clear();
    let fig_py = Py::new(py, fig)?;
    set_current_figure(fig_py.clone_ref(py));
    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    fig_py.borrow_mut(py).axes_list.push(ax_py.clone_ref(py));
    fig_py
        .borrow_mut(py)
        .axes_positions
        .push((0.0, 1.0, 0.0, 1.0));
    Ok((fig_py, ax_py))
}

/// 宏：消除创建 figure+axes 并返回 PyTuple 的样板代码
macro_rules! make_fig_ax {
    ($py:expr, |$ax:ident| $($body:tt)*) => {{
        let mut $ax = Axes::new();
        $($body)*
        let (fig_py, ax_py) = _make_fig_ax($py, $ax)?;
        let fig_obj = fig_py.bind($py).as_any().clone();
        let ax_obj = ax_py.bind($py).as_any().clone();
        PyTuple::new($py, [fig_obj, ax_obj])
    }};
}

#[pyfunction]
#[pyo3(signature = (text, color=None, fontsize=None, family=None, loc=None))]
pub fn xlabel(
    py: Python,
    text: String,
    color: Option<String>,
    fontsize: Option<f64>,
    family: Option<String>,
    loc: Option<String>,
) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    Axes::set_xlabel(&mut ax_ref, py, text, color, fontsize, family, loc);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (text, color=None, fontsize=None, family=None, loc=None))]
pub fn ylabel(
    py: Python,
    text: String,
    color: Option<String>,
    fontsize: Option<f64>,
    family: Option<String>,
    loc: Option<String>,
) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    Axes::set_ylabel(&mut ax_ref, py, text, color, fontsize, family, loc);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (text, color=None, fontsize=None, family=None, loc=None))]
pub fn title(
    py: Python,
    text: String,
    color: Option<String>,
    fontsize: Option<f64>,
    family: Option<String>,
    loc: Option<String>,
) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    Axes::set_title(&mut ax_ref, py, text, color, fontsize, family, loc);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (visible=None, c=None, ls=None, lw=None, axis=None))]
pub fn grid(
    py: Python,
    visible: Option<bool>,
    c: Option<String>,
    ls: Option<String>,
    lw: Option<f64>,
    axis: Option<String>,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .grid(visible, c, ls, lw, axis);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (loc="best"))]
pub fn legend(py: Python, loc: &str) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).legend(loc);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (left=None, right=None))]
pub fn xlim(py: Python, left: Option<f64>, right: Option<f64>) -> PyResult<()> {
    if let (Some(lo), Some(hi)) = (left, right) {
        get_current_axes(py)?.borrow_mut(py).set_xlim(
            Some(lo),
            Some(hi),
            None,
            None,
            None,
            true,
            None,
        );
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (bottom=None, top=None))]
pub fn ylim(py: Python, bottom: Option<f64>, top: Option<f64>) -> PyResult<()> {
    if let (Some(lo), Some(hi)) = (bottom, top) {
        get_current_axes(py)?
            .borrow_mut(py)
            .set_ylim(Some(lo), Some(hi), true, None);
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x, y, s=100.0, c=None, marker="o", label=None, alpha=1.0))]
pub fn scatter<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    s: f64,
    c: Option<String>,
    marker: &'a str,
    label: Option<String>,
    alpha: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.scatter(py, x, y, s, c, marker, label, alpha)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, s=None, c=None, marker="o", label=None, alpha=1.0))]
#[allow(clippy::too_many_arguments)]
pub fn scatter_multi<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    s: Option<Bound<'a, PyAny>>,
    c: Option<Bound<'a, PyAny>>,
    marker: &'a str,
    label: Option<String>,
    alpha: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.scatter_multi(py, x, y, s, c, marker, label, alpha)?;
    })
}

/// 将一组数值按 colormap 映射为 `#rrggbb` 颜色字符串。
///
/// 用于 scatter 的 `c=数值数组, cmap=...` 场景：Python 层把数值映射为颜色后，
/// 再作为逐点颜色传给 `scatter_multi`。未指定 vmin/vmax 时按数据的 min/max 归一化。
#[pyfunction]
#[pyo3(signature = (values, cmap="viridis", vmin=None, vmax=None))]
pub fn colormap_hex(
    values: Vec<f64>,
    cmap: &str,
    vmin: Option<f64>,
    vmax: Option<f64>,
) -> Vec<String> {
    let vmin = vmin.unwrap_or_else(|| {
        values
            .iter()
            .cloned()
            .filter(|v| v.is_finite())
            .fold(f64::INFINITY, f64::min)
    });
    let vmax = vmax.unwrap_or_else(|| {
        values
            .iter()
            .cloned()
            .filter(|v| v.is_finite())
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let range = if (vmax - vmin).abs() < 1e-12 {
        1.0
    } else {
        vmax - vmin
    };
    values
        .iter()
        .map(|&v| {
            let t = ((v - vmin) / range).clamp(0.0, 1.0);
            let color = crate::core::colormap::colormap_color(cmap, t);
            format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2)
        })
        .collect()
}

#[pyfunction]
#[pyo3(signature = (x, height, width=0.8, color=None, label=None))]
pub fn bar<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    height: Bound<'a, PyAny>,
    width: f64,
    color: Option<Bound<'a, PyAny>>,
    label: Option<String>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.bar(py, x, height, width, color, label)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, bins=None, density=false, label=None, alpha=0.7, color=None, facecolor=None, align=None, histtype=None))]
pub fn hist<'py>(
    py: Python<'py>,
    x: Bound<'py, PyAny>,
    bins: Option<Bound<'py, PyAny>>,
    density: bool,
    label: Option<String>,
    alpha: f64,
    color: Option<Bound<'py, PyAny>>,
    facecolor: Option<Bound<'py, PyAny>>,
    align: Option<String>,
    histtype: Option<String>,
) -> PyResult<Bound<'py, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.hist(
            py, x, bins, density, label, alpha, color, facecolor, align, histtype,
        )?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y1, y2=None, color=None, alpha=0.3, label=None))]
pub fn fill_between<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y1: Bound<'a, PyAny>,
    y2: Option<Bound<'a, PyAny>>,
    color: Option<String>,
    alpha: f64,
    label: Option<String>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.fill_between(py, x, y1, y2, color, alpha, label)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, *args, labels=None, colors=None, alpha=1.0))]
pub fn stackplot<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    args: &Bound<'a, PyTuple>,
    labels: Option<Vec<String>>,
    colors: Option<Vec<String>>,
    alpha: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.stackplot(py, x, args, labels, colors, alpha)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, yerr=None, xerr=None, fmt="o", color=None, label=None, capsize=3.0))]
pub fn errorbar<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    yerr: Option<Py<PyAny>>,
    xerr: Option<Py<PyAny>>,
    fmt: &'a str,
    color: Option<String>,
    label: Option<String>,
    capsize: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.errorbar(py, x, y, yerr, xerr, fmt, color, label, capsize)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, linefmt="-", markerfmt="o", label=None))]
pub fn stem<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    linefmt: &'a str,
    markerfmt: &'a str,
    label: Option<String>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.stem(py, x, y, linefmt, markerfmt, label)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, where_="pre", label=None, color=None, linestyle="-", linewidth=1.5))]
pub fn step<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    where_: &'a str,
    label: Option<String>,
    color: Option<String>,
    linestyle: &'a str,
    linewidth: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.step(py, x, y, where_, label, color, linestyle, linewidth)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, cmap="viridis", aspect="auto"))]
pub fn imshow<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    cmap: &'a str,
    aspect: &'a str,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        let data = if let Ok(v) = x.extract::<Vec<Vec<f64>>>() {
            v
        } else if x.hasattr("tolist")? {
            let list = x.call_method0("tolist")?;
            list.extract::<Vec<Vec<f64>>>()?
        } else {
            x.extract::<Vec<Vec<f64>>>()?
        };
        ax.imshow(data, cmap, aspect);
    })
}

#[pyfunction]
#[pyo3(signature = (x, labels=None, colors=None, autopct=None, startangle=0.0, explode=None))]
pub fn pie<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    labels: Option<Vec<String>>,
    colors: Option<Vec<String>>,
    autopct: Option<String>,
    startangle: f64,
    explode: Option<Vec<f64>>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        let x_vec = if let Ok(v) = x.extract::<Vec<f64>>() {
            v
        } else if x.hasattr("tolist")? {
            let list = x.call_method0("tolist")?;
            list.extract::<Vec<f64>>()?
        } else {
            x.extract::<Vec<f64>>()?
        };
        ax.pie(x_vec, labels, colors, autopct, startangle, explode);
    })
}

#[pyfunction]
#[pyo3(signature = (x, labels=None, vert=true))]
pub fn boxplot<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    labels: Option<Vec<String>>,
    vert: bool,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.boxplot(py, x, labels, vert)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, text, fontsize=None, color=None, c=None, family=None))]
pub fn text(
    py: Python,
    x: f64,
    y: f64,
    text: Bound<'_, PyAny>,
    fontsize: Option<f64>,
    color: Option<String>,
    c: Option<String>,
    family: Option<String>,
) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    Axes::text(&mut ax_ref, py, x, y, text, fontsize, color, c, family);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (y, color=None, linestyle=None, linewidth=None))]
pub fn axhline(
    py: Python,
    y: Option<f64>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .axhline(y, color, linestyle, linewidth);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x, color=None, linestyle=None, linewidth=None))]
pub fn axvline(
    py: Python,
    x: Option<f64>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .axvline(x, color, linestyle, linewidth);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (y, color=None, linestyle=None, linewidth=None))]
pub fn hlines(
    py: Python,
    y: Bound<'_, PyAny>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .hlines(py, y, color, linestyle, linewidth)?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x, color=None, linestyle=None, linewidth=None))]
pub fn vlines(
    py: Python,
    x: Bound<'_, PyAny>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .vlines(py, x, color, linestyle, linewidth)?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (ticks=None, labels=None))]
pub fn xticks(py: Python, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).xticks(ticks, labels);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (ticks=None, labels=None))]
pub fn yticks(py: Python, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).yticks(ticks, labels);
    Ok(())
}

#[pyfunction]
pub fn cla(py: Python) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).cla();
    Ok(())
}

#[pyfunction]
pub fn close(_py: Python) -> PyResult<()> {
    if let Ok(mut current) = crate::figure::figure::CURRENT_FIGURE.lock() {
        *current = None;
    }
    Ok(())
}

#[pyfunction]
pub fn twinx(py: Python) -> PyResult<Py<Axes>> {
    let ax = get_current_axes(py)?;
    let twin = ax.borrow_mut(py).twinx();
    let twin_py = Py::new(py, twin)?;
    init_axes_self_py(&twin_py, py);
    Ok(twin_py)
}

#[pyfunction]
pub fn twiny(py: Python) -> PyResult<Py<Axes>> {
    let ax = get_current_axes(py)?;
    let twin = ax.borrow_mut(py).twiny();
    let twin_py = Py::new(py, twin)?;
    init_axes_self_py(&twin_py, py);
    Ok(twin_py)
}

#[pyfunction]
pub fn tight_layout(py: Python) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method0("tight_layout")?;
    Ok(())
}

#[pyfunction]
pub fn set_size(py: Python, width: u32, height: u32) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method1("set_size", (width, height))?;
    Ok(())
}

#[pyfunction]
pub fn set_dpi(py: Python, dpi: f64) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method1("set_dpi", (dpi,))?;
    Ok(())
}

/// 计算第 `row` 行、第 `col` 列子图在 [0,1] 网格坐标系中的位置 (left, right, bottom, top)。
///
/// 采用与 matplotlib 一致的 wspace/hspace 语义（默认 0.2，即相邻子图间隔为
/// 单个子图宽/高的 20%），为内侧子图的刻度/坐标标签留出间隙，避免相互重叠。
/// 行号 0 在最上方（top=1.0 一侧）。
fn grid_position(row: usize, col: usize, nrows: usize, ncols: usize) -> (f64, f64, f64, f64) {
    const WSPACE: f64 = 0.2;
    const HSPACE: f64 = 0.2;
    let ncols_f = ncols as f64;
    let nrows_f = nrows as f64;
    let cell_w = 1.0 / (ncols_f + (ncols_f - 1.0) * WSPACE);
    let cell_h = 1.0 / (nrows_f + (nrows_f - 1.0) * HSPACE);
    let gap_w = cell_w * WSPACE;
    let gap_h = cell_h * HSPACE;
    let left = col as f64 * (cell_w + gap_w);
    let right = left + cell_w;
    let top = 1.0 - row as f64 * (cell_h + gap_h);
    let bottom = top - cell_h;
    (left, right, bottom, top)
}

#[pyfunction]
#[pyo3(signature = (nrows=1, ncols=1, index=1))]
pub fn subplot(
    py: Python<'_>,
    nrows: usize,
    ncols: usize,
    index: usize,
) -> PyResult<Bound<'_, PyTuple>> {
    let total = nrows * ncols;
    if index == 0 || index > total {
        return Err(PyValueError::new_err("Index out of range"));
    }

    // 复用当前 figure（保留用户已设置的 figsize / dpi 等）；没有则新建一个。
    let fig_bound = match get_current_figure(py) {
        Ok(f) => f,
        Err(_) => {
            let fig_py = Py::new(py, Figure::new())?;
            set_current_figure(fig_py.clone_ref(py));
            fig_py.bind(py).clone()
        }
    };

    // 若现有网格与请求的 nrows×ncols 不一致，则重建为一个空网格，
    // 并为每个格子写入正确的分数坐标位置；一致时直接复用，仅切换选中项。
    {
        let mut fig_ref = fig_bound.borrow_mut();
        let need_rebuild =
            fig_ref.nrows != nrows || fig_ref.ncols != ncols || fig_ref.axes_list.len() != total;
        if need_rebuild {
            fig_ref.axes_list.clear();
            fig_ref.axes_positions.clear();
            fig_ref.nrows = nrows;
            fig_ref.ncols = ncols;
            for k in 0..total {
                let ax_py = Py::new(py, Axes::new())?;
                init_axes_self_py(&ax_py, py);
                let pos = grid_position(k / ncols, k % ncols, nrows, ncols);
                fig_ref.axes_list.push(ax_py.clone_ref(py));
                fig_ref.axes_positions.push(pos);
            }
        }
        fig_ref.current_axes_index = index - 1;
    }

    let ax_py = fig_bound.borrow().axes_list[index - 1].clone_ref(py);
    let fig_obj = fig_bound.as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (nrows=1, ncols=1, figsize=None, dpi=None))]
pub fn subplots(
    py: Python<'_>,
    nrows: usize,
    ncols: usize,
    figsize: Option<(f64, f64)>,
    dpi: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let total = nrows * ncols;
    let dpi_val = dpi.unwrap_or(DEFAULT_DPI);
    let (width, height) = if let Some((w, h)) = figsize {
        ((w * dpi_val).round() as u32, (h * dpi_val).round() as u32)
    } else {
        let w = (DEFAULT_FIGSIZE.0 * dpi_val).round() as u32;
        let h = (DEFAULT_FIGSIZE.1 * dpi_val).round() as u32;
        (w, h)
    };

    let mut fig = Figure::new();
    fig.nrows = nrows;
    fig.ncols = ncols;
    fig.width = width.max(100);
    fig.height = height.max(100);
    fig.dpi = dpi_val;
    fig.axes_list.clear();
    fig.axes_positions.clear();
    let fig_py = Py::new(py, fig)?;
    set_current_figure(fig_py.clone_ref(py));

    if total == 1 {
        let ax_py = Py::new(py, Axes::new())?;
        init_axes_self_py(&ax_py, py);
        {
            let mut fig_ref = fig_py.borrow_mut(py);
            fig_ref.axes_list.push(ax_py.clone_ref(py));
            fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
            fig_ref.current_axes_index = 0;
        }
        let fig_obj = fig_py.bind(py).as_any().clone();
        let ax_obj = ax_py.bind(py).as_any().clone();
        PyTuple::new(py, [fig_obj, ax_obj])
    } else {
        let mut py_axes: Vec<Bound<'_, PyAny>> = Vec::new();
        {
            let mut fig_ref = fig_py.borrow_mut(py);
            for k in 0..total {
                let ax_py = Py::new(py, Axes::new())?;
                init_axes_self_py(&ax_py, py);
                let pos = grid_position(k / ncols, k % ncols, nrows, ncols);
                fig_ref.axes_list.push(ax_py.clone_ref(py));
                fig_ref.axes_positions.push(pos);
                py_axes.push(ax_py.bind(py).as_any().clone());
            }
            fig_ref.current_axes_index = 0;
        }
        let fig_obj = fig_py.bind(py).as_any().clone();
        let axes_list = PyList::new(py, py_axes)?;
        PyTuple::new(py, [fig_obj, axes_list.as_any().clone()])
    }
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, lw=None, c=None, ls=None, markersize=None, markeredgewidth=None, markerfacecolor=None, markeredgecolor=None, solid_capstyle=None))]
#[allow(clippy::too_many_arguments)]
pub fn plot<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
    lw: Option<f64>,
    c: Option<String>,
    ls: Option<String>,
    markersize: Option<f64>,
    markeredgewidth: Option<f64>,
    markerfacecolor: Option<String>,
    markeredgecolor: Option<String>,
    solid_capstyle: Option<String>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.plot(
            py,
            x,
            y,
            label,
            color,
            &linestyle.unwrap_or_else(|| "-".to_string()),
            marker,
            linewidth.unwrap_or(1.5),
            lw,
            c,
            ls,
            markersize,
            markeredgewidth,
            markerfacecolor,
            markeredgecolor,
            solid_capstyle,
        )?;
    })
}

#[pyfunction]
#[pyo3(signature = (filename, dpi=None))]
pub fn savefig(py: Python, filename: &str, dpi: Option<f64>) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    match dpi {
        Some(d) => fig.call_method1("savefig", (filename, d))?,
        None => fig.call_method1("savefig", (filename,))?,
    };
    Ok(())
}

#[pyfunction]
pub fn show(py: Python) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method0("show")?;
    Ok(())
}

#[pyfunction]
pub fn figure(py: Python) -> PyResult<Py<Figure>> {
    let fig = Figure::new();
    let fig_py = Py::new(py, fig)?;
    set_current_figure(fig_py.clone_ref(py));
    Ok(fig_py)
}

#[pyfunction]
pub fn gca(py: Python) -> PyResult<Py<Axes>> {
    let fig = get_current_figure(py)?;
    let fig_ref = fig.borrow();
    if fig_ref.axes_list.is_empty() {
        return Err(PyRuntimeError::new_err(
            "No axes found. Create a figure first.",
        ));
    }
    // 返回当前选中的 axes（plt.subplot 选中的子图；否则为最近创建的那个）
    let idx = fig_ref.current_axes_index.min(fig_ref.axes_list.len() - 1);
    Ok(fig_ref.axes_list[idx].clone_ref(py))
}

#[pyfunction]
pub fn clf(py: Python) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    let mut fig_ref = fig.borrow_mut();
    fig_ref.axes_list.clear();
    fig_ref.axes_positions.clear();
    fig_ref.current_axes_index = 0;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (y, width, height=0.8, color=None, label=None))]
pub fn barh<'a>(
    py: Python<'a>,
    y: Bound<'a, PyAny>,
    width: Bound<'a, PyAny>,
    height: f64,
    color: Option<Bound<'a, PyAny>>,
    label: Option<String>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.barh(py, y, width, height, color, label)?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
pub fn semilogx<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.set_xscale("log");
        let ls = linestyle.as_deref().unwrap_or("-");
        let lw = linewidth.unwrap_or(1.5);
        ax.plot(
            py, x, y, label, color, ls, marker, lw, None, None, None, None, None, None, None, None,
        )?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
pub fn semilogy<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.set_yscale("log");
        let ls = linestyle.as_deref().unwrap_or("-");
        let lw = linewidth.unwrap_or(1.5);
        ax.plot(
            py, x, y, label, color, ls, marker, lw, None, None, None, None, None, None, None, None,
        )?;
    })
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
pub fn loglog<'a>(
    py: Python<'a>,
    x: Bound<'a, PyAny>,
    y: Bound<'a, PyAny>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'a, PyTuple>> {
    make_fig_ax!(py, |ax| {
        ax.set_xscale("log");
        ax.set_yscale("log");
        let ls = linestyle.as_deref().unwrap_or("-");
        let lw = linewidth.unwrap_or(1.5);
        ax.plot(
            py, x, y, label, color, ls, marker, lw, None, None, None, None, None, None, None, None,
        )?;
    })
}

#[pyfunction]
pub fn use_(_backend: String) {}

#[pyfunction]
pub fn gcf(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    get_current_figure(py).map(|f| f.as_any().clone())
}

#[pyfunction]
pub fn xscale(py: Python<'_>, scale: &str) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    ax.borrow_mut(py).set_xscale(scale);
    Ok(())
}

#[pyfunction]
pub fn yscale(py: Python<'_>, scale: &str) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    ax.borrow_mut(py).set_yscale(scale);
    Ok(())
}

#[pyfunction]
pub fn margins(_py: Python<'_>, _x_margin: Option<f64>, _y_margin: Option<f64>) -> PyResult<()> {
    Ok(())
}

#[pyfunction]
pub fn box_(_py: Python<'_>, _on: Option<bool>) -> PyResult<()> {
    Ok(())
}

#[pyfunction]
pub fn minorticks_on(py: Python<'_>) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    ax_ref.minor_grid_visible = true;
    Ok(())
}

#[pyfunction]
pub fn minorticks_off(py: Python<'_>) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    ax_ref.minor_grid_visible = false;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (y1, y2, color=None, alpha=0.3))]
pub fn axhspan(
    py: Python<'_>,
    y1: f64,
    y2: f64,
    color: Option<String>,
    alpha: f64,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .axhspan(y1, y2, color, alpha);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x1, x2, color=None, alpha=0.3))]
pub fn axvspan(
    py: Python<'_>,
    x1: f64,
    x2: f64,
    color: Option<String>,
    alpha: f64,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .axvspan(x1, x2, color, alpha);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (xy1, xy2, color=None, linestyle=None, linewidth=None))]
pub fn axline(
    py: Python<'_>,
    xy1: (f64, f64),
    xy2: (f64, f64),
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?
        .borrow_mut(py)
        .axline(xy1, xy2, color, linestyle, linewidth);
    Ok(())
}
