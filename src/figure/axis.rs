use crate::figure::axes::Axes;
use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PySlice;

#[pyclass]
pub struct Axis {
    pub grid_visible: bool,
    pub grid_color: Option<String>,
    pub grid_linewidth: Option<f64>,
    pub grid_linestyle: Option<String>,
    pub minor_grid_color: Option<String>,
    pub minor_grid_linewidth: Option<f64>,
    pub minor_grid_linestyle: Option<String>,
    #[allow(dead_code)]
    pub major_locator: String,
    #[allow(dead_code)]
    pub minor_locator: String,
    /// 存储 set_major_locator 传入的 Python locator 对象（用于反射）
    pub major_locator_py: Option<Py<PyAny>>,
    /// 存储 set_minor_locator 传入的 Python locator 对象（用于反射）
    pub minor_locator_py: Option<Py<PyAny>>,
    pub parent: Option<Py<PyAny>>,
    pub which: String,
}

impl Default for Axis {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl Axis {
    #[new]
    pub fn new() -> Self {
        Axis {
            grid_visible: false,
            grid_color: None,
            grid_linewidth: None,
            grid_linestyle: None,
            minor_grid_color: None,
            minor_grid_linewidth: None,
            minor_grid_linestyle: None,
            major_locator: "auto".to_string(),
            minor_locator: "auto".to_string(),
            major_locator_py: None,
            minor_locator_py: None,
            parent: None,
            which: "x".to_string(),
        }
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (visible=None, which="major", ls=None, c=None, lw=None))]
    fn grid(
        &mut self,
        py: Python<'_>,
        visible: Option<bool>,
        which: &str,
        ls: Option<&str>,
        c: Option<&str>,
        lw: Option<f64>,
    ) {
        self.grid_visible = visible.unwrap_or(true);
        if "minor".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
            self.minor_grid_color = c.map(|s| s.to_string());
            self.minor_grid_linewidth = lw;
            self.minor_grid_linestyle = ls.map(|s| s.to_string());
        }
        if "major".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
            self.grid_color = c.map(|s| s.to_string());
            self.grid_linewidth = lw;
            self.grid_linestyle = ls.map(|s| s.to_string());
        }
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            if "minor".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
                ax.minor_grid_visible = true;
                ax.minor_grid_color = c.map(|s| s.to_string());
                ax.minor_grid_linewidth = lw;
                ax.minor_grid_linestyle = ls.map(|s| s.to_string());
                if self.which == "x" {
                    ax.minor_grid_x_visible = true;
                } else {
                    ax.minor_grid_y_visible = true;
                }
            }
            if "major".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
                ax.grid_color = c.map(|s| s.to_string());
                ax.grid_linewidth = lw;
                ax.grid_linestyle = ls.map(|s| s.to_string());
                if visible.unwrap_or(true) {
                    ax.grid_visible = true;
                }
            }
        }
    }

    fn set_major_locator(&mut self, py: Python<'_>, locator: &Bound<'_, PyAny>) {
        // 保存 locator 引用（用于 Python 端反射以及 Axes 端 tick 计算）
        self.major_locator_py = Some(locator.clone().unbind());
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            if self.which == "x" {
                ax.xaxis_major_locator = Some(locator.clone().unbind());
            } else {
                ax.yaxis_major_locator = Some(locator.clone().unbind());
            }
        }
    }

    fn set_minor_locator(&mut self, py: Python<'_>, locator: &Bound<'_, PyAny>) {
        // 保存 locator 引用
        self.minor_locator_py = Some(locator.clone().unbind());
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            if self.which == "x" {
                ax.xaxis_minor_locator = Some(locator.clone().unbind());
            } else {
                ax.yaxis_minor_locator = Some(locator.clone().unbind());
            }
        }
    }

    fn get_major_locator<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyAny>> {
        // 优先使用本对象缓存的 locator
        if let Some(loc) = self.major_locator_py.as_ref() {
            return Some(loc.bind(py).clone());
        }
        // 回退到 parent Axes 上的 locator
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let ax = ax_bound.borrow();
            let stored = if self.which == "x" {
                ax.xaxis_major_locator.as_ref()
            } else {
                ax.yaxis_major_locator.as_ref()
            };
            if let Some(loc) = stored {
                return Some(loc.bind(py).clone());
            }
        }
        None
    }

    fn get_minor_locator<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyAny>> {
        // 优先使用本对象缓存的 locator
        if let Some(loc) = self.minor_locator_py.as_ref() {
            return Some(loc.bind(py).clone());
        }
        // 回退到 parent Axes 上的 locator
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let ax = ax_bound.borrow();
            let stored = if self.which == "x" {
                ax.xaxis_minor_locator.as_ref()
            } else {
                ax.yaxis_minor_locator.as_ref()
            };
            if let Some(loc) = stored {
                return Some(loc.bind(py).clone());
            }
        }
        None
    }

    fn set_major_formatter(&mut self, py: Python<'_>, formatter: &Bound<'_, PyAny>) {
        // 保存 formatter 引用（如 ConciseDateFormatter），渲染刻度标签时调用其
        // format_ticks 生成文本。存到 parent Axes 上，供 render 读取。
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            if self.which == "x" {
                ax.xaxis_major_formatter = Some(formatter.clone().unbind());
            } else {
                ax.yaxis_major_formatter = Some(formatter.clone().unbind());
            }
        }
    }

    fn get_major_formatter<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyAny>> {
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let ax = ax_bound.borrow();
            let stored = if self.which == "x" {
                ax.xaxis_major_formatter.as_ref()
            } else {
                ax.yaxis_major_formatter.as_ref()
            };
            if let Some(fmt) = stored {
                return Some(fmt.bind(py).clone());
            }
        }
        None
    }
}

#[pyclass]
pub struct Patch {
    pub facecolor: String,
    pub edgecolor: String,
    pub parent: Option<Py<PyAny>>,
}

impl Default for Patch {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl Patch {
    #[new]
    pub fn new() -> Self {
        Patch {
            facecolor: "white".to_string(),
            edgecolor: "black".to_string(),
            parent: None,
        }
    }

    fn set_facecolor(&mut self, color: &str, py: Python<'_>) {
        self.facecolor = color.to_string();
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            ax.facecolor = color.to_string();
        }
    }

    fn get_facecolor(&self) -> &str {
        &self.facecolor
    }

    fn set_edgecolor(&mut self, color: &str) {
        self.edgecolor = color.to_string();
    }
}

#[pyclass]
pub struct SpineDict {
    pub spines: Vec<Spine>,
    pub parent: Option<Py<PyAny>>,
}

impl Default for SpineDict {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl SpineDict {
    #[new]
    pub fn new() -> Self {
        let names = ["top", "bottom", "left", "right"];
        SpineDict {
            spines: names
                .iter()
                .map(|n| Spine {
                    name: n.to_string(),
                    visible: true,
                    parent: None,
                })
                .collect(),
            parent: None,
        }
    }

    /// 索引取 spine：字符串键返回单个 Spine；切片（如 `ax.spines[:]`）或名称列表返回
    /// 广播代理 SpineProxy，其方法（set_visible/set_color/set_linewidth）作用于所选全部
    /// spine，对齐 matplotlib 的 `SpinesProxy` 行为。
    fn __getitem__(&mut self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if let Ok(name) = key.extract::<String>() {
            return match self.spines.iter().position(|s| s.name == name) {
                Some(i) => {
                    let mut spine = self.spines[i].clone();
                    spine.parent = self.parent.as_ref().map(|p| p.clone_ref(py));
                    Ok(Py::new(py, spine)?.into_any())
                }
                None => Err(PyKeyError::new_err(name)),
            };
        }
        let names: Vec<String> = if key.is_instance_of::<PySlice>() {
            self.spines.iter().map(|s| s.name.clone()).collect()
        } else if let Ok(list) = key.extract::<Vec<String>>() {
            list
        } else {
            return Err(PyTypeError::new_err(
                "spines index must be a str, slice, or list of str",
            ));
        };
        let proxy = SpineProxy {
            names,
            parent: self.parent.as_ref().map(|p| p.clone_ref(py)),
        };
        Ok(Py::new(py, proxy)?.into_any())
    }

    fn items(&self, _py: Python<'_>) -> Vec<(String, Spine)> {
        self.spines
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect()
    }
}

#[pyclass(skip_from_py_object)]
pub struct Spine {
    pub name: String,
    pub visible: bool,
    pub parent: Option<Py<PyAny>>,
}

impl Clone for Spine {
    fn clone(&self) -> Self {
        Spine {
            name: self.name.clone(),
            visible: self.visible,
            parent: None,
        }
    }
}

#[pymethods]
impl Spine {
    fn set_visible(&mut self, visible: bool, py: Python<'_>) {
        self.visible = visible;
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            match self.name.as_str() {
                "top" => ax.spine_top = visible,
                "bottom" => ax.spine_bottom = visible,
                "left" => ax.spine_left = visible,
                "right" => ax.spine_right = visible,
                _ => {}
            }
        }
    }

    fn get_visible(&self) -> bool {
        self.visible
    }

    fn get_color(&self) -> String {
        "black".to_string()
    }

    fn set_color(&mut self, color: &str, py: Python<'_>) {
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            ax.spine_color = color.to_string();
        }
    }

    fn set_linewidth(&mut self, lw: f64, py: Python<'_>) {
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            ax.spine_linewidth = lw;
        }
    }
}

/// `ax.spines[:]` / `ax.spines[['left', 'bottom']]` 返回的广播代理：方法调用作用于
/// 选中的全部 spine（matplotlib 中为 `SpinesProxy`）。
#[pyclass]
pub struct SpineProxy {
    pub names: Vec<String>,
    pub parent: Option<Py<PyAny>>,
}

#[pymethods]
impl SpineProxy {
    fn set_visible(&self, visible: bool, py: Python<'_>) {
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            for name in &self.names {
                match name.as_str() {
                    "top" => ax.spine_top = visible,
                    "bottom" => ax.spine_bottom = visible,
                    "left" => ax.spine_left = visible,
                    "right" => ax.spine_right = visible,
                    _ => {}
                }
            }
        }
    }

    fn set_color(&self, color: &str, py: Python<'_>) {
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            ax.spine_color = color.to_string();
        }
    }

    fn set_linewidth(&self, lw: f64, py: Python<'_>) {
        if let Some(parent) = &self.parent
            && let Ok(ax_bound) = parent.bind(py).cast::<Axes>()
        {
            let mut ax = ax_bound.borrow_mut();
            ax.spine_linewidth = lw;
        }
    }
}
