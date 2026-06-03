use std::sync::Mutex;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use plotters::prelude::*;

use crate::axes::Axes;
use crate::colors::{parse_color, to_plotters_color, RgbColor};

pub(crate) static CURRENT_FIGURE: Mutex<Option<Py<Figure>>> = Mutex::new(None);

pub fn set_current_figure(fig: Py<Figure>) {
    if let Ok(mut current) = CURRENT_FIGURE.lock() {
        *current = Some(fig);
    }
}

pub fn get_current_figure(py: Python<'_>) -> PyResult<Bound<'_, Figure>> {
    let guard = CURRENT_FIGURE
        .lock()
        .map_err(|_| PyRuntimeError::new_err("Mutex poisoned"))?;
    match guard.as_ref() {
        Some(fig) => Ok(fig.bind(py).clone()),
        None => Err(PyRuntimeError::new_err(
            "No current figure. Create one with figure() or subplots() first.",
        )),
    }
}

#[pyclass]
pub struct Figure {
    pub axes_list: Vec<Py<Axes>>,
    pub nrows: usize,
    pub ncols: usize,
    pub suptitle: String,
    pub width: u32,
    pub height: u32,
    pub dpi: f64,
    pub axes_positions: Vec<(f64, f64, f64, f64)>,
    pub facecolor: String,
    pub subplot_left: f64,
    pub subplot_right: f64,
    pub subplot_bottom: f64,
    pub subplot_top: f64,
}

#[pymethods]
impl Figure {
    #[new]
    pub fn new() -> Self {
        Figure {
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
        }
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn set_dpi(&mut self, dpi: f64) {
        self.dpi = dpi;
    }

    fn suptitle(&mut self, text: String) {
        self.suptitle = text;
    }

    #[pyo3(signature = (left=None, right=None, bottom=None, top=None, _wspace=None, _hspace=None))]
    fn subplots_adjust(&mut self, left: Option<f64>, right: Option<f64>, bottom: Option<f64>, top: Option<f64>, _wspace: Option<f64>, _hspace: Option<f64>) {
        if let Some(v) = left { self.subplot_left = v; }
        if let Some(v) = right { self.subplot_right = v; }
        if let Some(v) = bottom { self.subplot_bottom = v; }
        if let Some(v) = top { self.subplot_top = v; }
    }

    fn tight_layout(&mut self) {
    }

    fn set_facecolor(&mut self, color: &str) {
        self.facecolor = color.to_string();
    }

    fn clear(&mut self) {
        self.axes_list.clear();
        self.axes_positions.clear();
    }

    fn clf(&mut self) {
        self.axes_list.clear();
        self.axes_positions.clear();
    }

    fn add_subplot(&mut self, py: Python, spec: &Bound<'_, PyAny>) -> PyResult<Py<Axes>> {
        let (left, right, bottom, top) = if let Ok(_) = spec.getattr("rowStart") {
            let num_rows: f64 = spec.getattr("numRows")?.extract::<i32>().map(|v| v as f64).unwrap_or(100.0);
            let num_cols: f64 = spec.getattr("numCols")?.extract::<i32>().map(|v| v as f64).unwrap_or(100.0);
            let row_start: f64 = spec.getattr("rowStart")?.extract::<i32>().map(|v| v as f64).unwrap_or(0.0);
            let row_stop: f64 = spec.getattr("rowStop")?.extract::<i32>().map(|v| v as f64).unwrap_or(num_rows);
            let col_start: f64 = spec.getattr("colStart")?.extract::<i32>().map(|v| v as f64).unwrap_or(0.0);
            let col_stop: f64 = spec.getattr("colStop")?.extract::<i32>().map(|v| v as f64).unwrap_or(num_cols);
            
            let left = col_start / num_cols;
            let right = col_stop / num_cols;
            let bottom = 1.0 - row_stop / num_rows;
            let top = 1.0 - row_start / num_rows;
            
            (left, right, bottom, top)
        } else {
            (0.0, 1.0, 0.0, 1.0)
        };
        let ax = Axes::new();
        let ax_py = Py::new(py, ax)?;
        crate::pyfuncs::init_axes_self_py(&ax_py, py);
        self.axes_list.push(ax_py.clone_ref(py));
        self.axes_positions.push((left, right, bottom, top));
        Ok(ax_py)
    }

    fn savefig(&self, py: Python, filename: &str) -> PyResult<()> {
        if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            let backend = BitMapBackend::new(filename, (self.width, self.height));
            self.render_to_backend(py, backend, self.width, self.height)
        } else {
            let svg_w = (self.width as f64 * 72.0 / self.dpi).round() as u32;
            let svg_h = (self.height as f64 * 72.0 / self.dpi).round() as u32;
            let backend = SVGBackend::new(filename, (svg_w, svg_h));
            self.render_to_backend(py, backend, svg_w, svg_h)?;
            // 与matplotlib一致，使用pt单位
            if let Ok(content) = std::fs::read_to_string(filename) {
                let content = content
                    .replacen(&format!("width=\"{}\"", svg_w), &format!("width=\"{}pt\"", svg_w), 1)
                    .replacen(&format!("height=\"{}\"", svg_h), &format!("height=\"{}pt\"", svg_h), 1);
                let _ = std::fs::write(filename, content);
            }
            Ok(())
        }
    }

    fn show(&self, py: Python) -> PyResult<()> {
        let tmpdir = std::env::temp_dir();
        let path = tmpdir.join("rsplot_output.png");
        let filename = path.to_str().unwrap_or("/tmp/rsplot_output.png").to_string();
        let backend = BitMapBackend::new(&filename, (self.width, self.height));
        self.render_to_backend(py, backend, self.width, self.height)?;

        if cfg!(target_os = "macos") {
            let _ = std::process::Command::new("open").arg(&filename).spawn();
        } else if cfg!(target_os = "linux") {
            let _ = std::process::Command::new("xdg-open").arg(&filename).spawn();
        }

        println!("Figure saved to: {}", filename);
        Ok(())
    }
}

impl Figure {
    fn render_to_backend<B: DrawingBackend>(&self, py: Python, backend: B, actual_w: u32, actual_h: u32) -> PyResult<()>
    where
        B::ErrorType: 'static,
    {
        let root = backend.into_drawing_area();

        // 不填充全屏背景，与matplotlib一致（matplotlib的SVG不包含全屏背景rect）

        if self.axes_list.is_empty() {
            root.present()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to present: {}", e)))?;
            return Ok(());
        }

        let _nrows = self.nrows;
        let _ncols = self.ncols;

        if !self.suptitle.is_empty() {
            let _ = root.titled(&self.suptitle, ("sans-serif", 24));
        }

        let total_w = actual_w as f64;
        let total_h = actual_h as f64;

        for (i, ax_py) in self.axes_list.iter().enumerate() {
            let ax = ax_py.borrow(py);

            let ((x_min, x_max), (y_min, y_max)) = ax.compute_bounds();

            let (left, right, bottom, top) = if i < self.axes_positions.len() {
                self.axes_positions[i]
            } else {
                (0.0, 1.0, 0.0, 1.0)
            };

            let margin_l = self.subplot_left;
            let margin_r = 1.0 - self.subplot_right;
            let margin_b = self.subplot_bottom;
            let margin_t = 1.0 - self.subplot_top;
            let usable_w = 1.0 - margin_l - margin_r;
            let usable_h = 1.0 - margin_b - margin_t;
            let plot_left = left * usable_w + margin_l;
            let plot_right = right * usable_w + margin_l;
            let plot_bottom_frac = bottom * usable_h + margin_b;
            let plot_top_frac = top * usable_h + margin_b;

            let x0 = (plot_left * total_w) as f64;
            let y0 = ((1.0 - plot_top_frac) * total_h) as f64;
            let sub_w = ((plot_right - plot_left) * total_w) as f64;
            let sub_h = ((plot_top_frac - plot_bottom_frac) * total_h) as f64;

            if sub_w <= 0.0 || sub_h <= 0.0 {
                drop(ax);
                continue;
            }

            let chart_area = root.clone().shrink(
                (x0 as i32, y0 as i32),
                (sub_w as u32, sub_h as u32),
            );

            // GridSpec已定义了包含标签的完整区域，ChartBuilder不需要额外margin
            // 与matplotlib一致：GridSpec位置就是axes的完整边界框
            let mut chart = ChartBuilder::on(&chart_area)
                .margin_top(0)
                .margin_right(0)
                .margin_bottom(0)
                .margin_left(0)
                .build_cartesian_2d(x_min..x_max, y_min..y_max)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to build chart: {}", e)))?;

            ax.render(py, &mut chart, (x_min, x_max), (y_min, y_max))?;

            let twin_axes = ax.twin_axes.clone();
            drop(ax);
            for twin in &twin_axes {
                let ((tx_min, tx_max), (ty_min, ty_max)) = twin.compute_bounds();
                let (ux_min, ux_max) = if twin.is_twin_x { (tx_min, tx_max) } else { (x_min, x_max) };
                let (uy_min, uy_max) = if twin.is_twin_y { (ty_min, ty_max) } else { (y_min, y_max) };
                let mut twin_chart = ChartBuilder::on(&chart_area)
                    .margin_top(0)
                    .margin_right(0)
                    .margin_bottom(0)
                    .margin_left(0)
                    .build_cartesian_2d(ux_min..ux_max, uy_min..uy_max)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to build twin chart: {}", e)))?;
                twin.render(py, &mut twin_chart, (ux_min, ux_max), (uy_min, uy_max))?;
            }
        }

        root.present()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to present: {}", e)))?;

        Ok(())
    }
}