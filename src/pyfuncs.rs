use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple, PyAny, PyInt};

use crate::axes::Axes;
use crate::figure::{get_current_figure, set_current_figure, Figure};

pub fn get_current_axes(py: Python<'_>) -> PyResult<Py<Axes>> {
    let fig = get_current_figure(py)?;
    let fig_ref = fig.borrow();
    if fig_ref.axes_list.is_empty() {
        return Err(PyRuntimeError::new_err("No axes found in current figure."));
    }
    // 返回最后创建的axes（更符合matplotlib行为，plt.*应该作用于最近操作的axes）
    let last_idx = fig_ref.axes_list.len() - 1;
    Ok(fig_ref.axes_list[last_idx].clone_ref(py))
}

pub fn init_axes_self_py(ax_py: &Py<Axes>, py: Python<'_>) {
    let obj: Py<PyAny> = ax_py.clone_ref(py).into();
    let mut ax_ref = ax_py.borrow_mut(py);
    ax_ref.self_py = Some(obj);
}

fn _make_fig_ax(py: Python<'_>, ax: Axes) -> PyResult<(Py<Figure>, Py<Axes>)> {
    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));
    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    fig_py.borrow_mut(py).axes_list.push(ax_py.clone_ref(py));
    fig_py.borrow_mut(py).axes_positions.push((0.0, 1.0, 0.0, 1.0));
    Ok((fig_py, ax_py))
}

#[pyfunction]
pub fn xlabel(py: Python, text: String) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_xlabel(text);
    Ok(())
}

#[pyfunction]
pub fn ylabel(py: Python, text: String) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_ylabel(text);
    Ok(())
}

#[pyfunction]
pub fn title(py: Python, text: String) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_title(text);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (visible=None, c=None, ls=None, lw=None, axis=None))]
pub fn grid(py: Python, visible: Option<bool>, c: Option<String>, ls: Option<String>, lw: Option<f64>, axis: Option<String>) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).grid(visible, c, ls, lw, axis);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (loc="best"))]
pub fn legend(py: Python, loc: &str) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).legend(loc);
    Ok(())
}

#[pyfunction]
pub fn xlim(py: Python, left: f64, right: f64) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_xlim(left, right);
    Ok(())
}

#[pyfunction]
pub fn ylim(py: Python, bottom: f64, top: f64) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_ylim(bottom, top);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x, y, s=20.0, c=None, marker="o", label=None, alpha=1.0))]
pub fn scatter<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    s: f64,
    c: Option<String>,
    marker: &'a str,
    label: Option<String>,
    alpha: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.scatter(x, y, s, c, marker, label, alpha);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, height, width=0.8, color=None, label=None))]
pub fn bar(
    py: Python<'_>,
    x: Vec<f64>,
    height: Vec<f64>,
    width: f64,
    color: Option<String>,
    label: Option<String>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.bar(x, height, width, color, label);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, bins=10, density=false, label=None, alpha=0.7, color=None, facecolor=None))]
pub fn hist<'py>(
    py: Python<'py>,
    x: Bound<'py, PyAny>,
    bins: usize,
    density: bool,
    label: Option<String>,
    alpha: f64,
    color: Option<Bound<'py, PyAny>>,
    facecolor: Option<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyTuple>> {
    let mut ax = Axes::new();
    let bins_any = PyInt::new(py, bins as i64).as_any().clone();
    ax.hist(py, x, Some(bins_any), density, label, alpha, color, facecolor, None, None)?;
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y1, y2=0.0, color=None, alpha=0.3, label=None))]
pub fn fill_between(
    py: Python<'_>,
    x: Vec<f64>,
    y1: Vec<f64>,
    y2: f64,
    color: Option<String>,
    alpha: f64,
    label: Option<String>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.fill_between(x, y1, y2, color, alpha, label);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, yerr=None, xerr=None, fmt="o", color=None, label=None, capsize=3.0))]
pub fn errorbar<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    yerr: Option<Py<PyAny>>,
    xerr: Option<Py<PyAny>>,
    fmt: &'a str,
    color: Option<String>,
    label: Option<String>,
    capsize: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    // Convert possible scalar or sequence yerr/xerr into Vec<f64>
    let make_vec = |maybe: Option<Py<PyAny>>, n: usize| -> Option<Vec<f64>> {
        if let Some(obj) = maybe {
            if let Ok(v) = obj.extract::<Vec<f64>>(py) {
                return Some(v);
            }
            if let Ok(v) = obj.extract::<f64>(py) {
                return Some(vec![v; n]);
            }
        }
        None
    };

    let yerr_vec = make_vec(yerr, x.len());
    let xerr_vec = make_vec(xerr, x.len());

    let mut ax = Axes::new();
    ax.errorbar(x, y, yerr_vec, xerr_vec, fmt, color, label, capsize);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, linefmt="-", markerfmt="o", label=None))]
pub fn stem<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    linefmt: &'a str,
    markerfmt: &'a str,
    label: Option<String>,
) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.stem(x, y, linefmt, markerfmt, label);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, where_="pre", label=None, color=None, linestyle="-", linewidth=1.5))]
pub fn step<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    where_: &'a str,
    label: Option<String>,
    color: Option<String>,
    linestyle: &'a str,
    linewidth: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.step(x, y, where_, label, color, linestyle, linewidth);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, cmap="viridis", aspect="auto"))]
pub fn imshow<'a>(py: Python<'a>, x: Vec<Vec<f64>>, cmap: &'a str, aspect: &'a str) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.imshow(x, cmap, aspect);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, labels=None, colors=None, autopct=None, startangle=0.0))]
pub fn pie(
    py: Python<'_>,
    x: Vec<f64>,
    labels: Option<Vec<String>>,
    colors: Option<Vec<String>>,
    autopct: Option<String>,
    startangle: f64,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.pie(x, labels, colors, autopct, startangle);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, labels=None, vert=true))]
pub fn boxplot(
    py: Python<'_>,
    x: Vec<Vec<f64>>,
    labels: Option<Vec<String>>,
    vert: bool,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.boxplot(x, labels, vert);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, text, fontsize=None, color=None, c=None, family=None))]
pub fn text(
    py: Python,
    x: f64,
    y: f64,
    text: Bound<'_, PyAny>,
    fontsize: Option<i32>,
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
#[pyo3(signature = (y=None, color=None, linestyle=None, linewidth=None))]
pub fn axhline(
    py: Python,
    y: Option<f64>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).axhline(y, color, linestyle, linewidth);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x=None, color=None, linestyle=None, linewidth=None))]
pub fn axvline(
    py: Python,
    x: Option<f64>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).axvline(x, color, linestyle, linewidth);
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
    if let Ok(mut current) = crate::figure::CURRENT_FIGURE.lock() {
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

#[pyfunction]
#[pyo3(signature = (nrows=1, ncols=1, index=1))]
pub fn subplot(py: Python<'_>, nrows: usize, ncols: usize, index: usize) -> PyResult<Bound<'_, PyTuple>> {
    if index == 0 || index > nrows * ncols {
        return Err(PyValueError::new_err("Index out of range"));
    }
    let result = subplots(py, nrows, ncols)?;
    let fig = result.get_item(0)?;
    let axes_all = result.get_item(1)?;
    let ax = if nrows * ncols == 1 {
        axes_all.clone()
    } else {
        let lst = axes_all.cast::<PyList>()?;
        lst.get_item(index - 1)?
    };
    PyTuple::new(py, [fig, ax])
}

#[pyfunction]
#[pyo3(signature = (nrows=1, ncols=1))]
pub fn subplots(
    py: Python<'_>,
    nrows: usize,
    ncols: usize,
) -> PyResult<Bound<'_, PyTuple>> {
    let total = nrows * ncols;

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows,
        ncols,
        suptitle: String::new(),
        width: (ncols as u32 * 400).max(600),
        height: (nrows as u32 * 300).max(400),
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    if total == 1 {
        let ax = Axes::new();
        let ax_py = Py::new(py, ax)?;
        init_axes_self_py(&ax_py, py);
        {
            let mut fig_ref = fig_py.borrow_mut(py);
            fig_ref.axes_list.push(ax_py.clone_ref(py));
        }
        let fig_obj = fig_py.bind(py).as_any().clone();
        let ax_obj = ax_py.bind(py).as_any().clone();
        PyTuple::new(py, [fig_obj, ax_obj])
    } else {
        let mut py_axes: Vec<Bound<'_, PyAny>> = Vec::new();
        {
            let mut fig_ref = fig_py.borrow_mut(py);
            for _ in 0..total {
                let ax = Axes::new();
                let ax_py = Py::new(py, ax)?;
                init_axes_self_py(&ax_py, py);
                fig_ref.axes_list.push(ax_py.clone_ref(py));
                py_axes.push(ax_py.bind(py).as_any().clone());
            }
        }
        let fig_obj = fig_py.bind(py).as_any().clone();
        let axes_list = PyList::new(py, py_axes)?;
        PyTuple::new(py, [fig_obj, axes_list.as_any().clone()])
    }
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
pub fn plot(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, &linestyle.unwrap_or_else(|| "-".to_string()), marker, linewidth.unwrap_or(1.5), None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
pub fn savefig(py: Python, filename: &str) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method1("savefig", (filename,))?;
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
        return Err(PyRuntimeError::new_err("No axes found. Create a figure first."));
    }
    // 返回最后创建的axes（更符合matplotlib行为）
    let last_idx = fig_ref.axes_list.len() - 1;
    Ok(fig_ref.axes_list[last_idx].clone_ref(py))
}

#[pyfunction]
pub fn clf(py: Python) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    let mut fig_ref = fig.borrow_mut();
    fig_ref.axes_list.clear();
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (y, width, height=0.8, color=None, label=None))]
pub fn barh(py: Python<'_>, y: Vec<f64>, width: Vec<f64>, height: f64, color: Option<String>, label: Option<String>) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.barh(y, width, height, color, label);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
pub fn semilogx(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.set_xscale("log");
    let ls = linestyle.as_deref().unwrap_or("-");
    let lw = linewidth.unwrap_or(1.5);
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, ls, marker, lw, None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
pub fn semilogy(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.set_yscale("log");
    let ls = linestyle.as_deref().unwrap_or("-");
    let lw = linewidth.unwrap_or(1.5);
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, ls, marker, lw, None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
pub fn loglog(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.set_xscale("log");
    ax.set_yscale("log");
    let ls = linestyle.as_deref().unwrap_or("-");
    let lw = linewidth.unwrap_or(1.5);
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, ls, marker, lw, None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));
    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
pub fn use_(_backend: String) {
}

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