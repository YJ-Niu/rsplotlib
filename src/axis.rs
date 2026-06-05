use pyo3::prelude::*;
use crate::axes::Axes;

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
    pub parent: Option<Py<PyAny>>,
    pub which: String,
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
            parent: None,
            which: "x".to_string(),
        }
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (visible=None, which="major", ls=None, c=None, lw=None))]
    fn grid(&mut self, py: Python<'_>, visible: Option<bool>, which: &str, ls: Option<&str>, c: Option<&str>, lw: Option<f64>) {
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
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
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
    }

    fn set_major_locator(&mut self, py: Python<'_>, locator: &Bound<'_, PyAny>) {
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                if self.which == "x" {
                    ax.xaxis_major_locator = Some(locator.clone().unbind());
                } else {
                    ax.yaxis_major_locator = Some(locator.clone().unbind());
                }
            }
        }
    }

    fn set_minor_locator(&mut self, py: Python<'_>, locator: &Bound<'_, PyAny>) {
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                if self.which == "x" {
                    ax.xaxis_minor_locator = Some(locator.clone().unbind());
                } else {
                    ax.yaxis_minor_locator = Some(locator.clone().unbind());
                }
            }
        }
    }
}

#[pyclass]
pub struct Patch {
    pub facecolor: String,
    pub edgecolor: String,
    pub parent: Option<Py<PyAny>>,
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
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                ax.facecolor = color.to_string();
            }
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

#[pymethods]
impl SpineDict {
    #[new]
    pub fn new() -> Self {
        let names = vec!["top", "bottom", "left", "right"];
        SpineDict {
            spines: names.iter().map(|n| Spine { name: n.to_string(), visible: true, parent: None }).collect(),
            parent: None,
        }
    }

    fn __getitem__(&mut self, py: Python<'_>, key: &str) -> Option<Spine> {
        self.spines.iter().position(|s| s.name == key).map(|i| {
            let mut spine = self.spines[i].clone();
            spine.parent = self.parent.as_ref().map(|p| p.clone_ref(py));
            spine
        })
    }

    fn items(&self, _py: Python<'_>) -> Vec<(String, Spine)> {
        self.spines.iter().map(|s| (s.name.clone(), s.clone())).collect()
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
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
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
    }

    fn get_visible(&self) -> bool {
        self.visible
    }

    fn get_color(&self) -> String {
        "black".to_string()
    }

    fn set_color(&mut self, color: &str, py: Python<'_>) {
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                ax.spine_color = color.to_string();
            }
        }
    }

    fn set_linewidth(&mut self, lw: f64, py: Python<'_>) {
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                ax.spine_linewidth = lw;
            }
        }
    }
}
