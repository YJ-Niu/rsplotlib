use std::sync::Mutex;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use plotters::prelude::*;

use crate::axes::Axes;
// colors not needed directly in this module

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
            width: 640,
            height: 480,
            dpi: 100.0,
            axes_positions: Vec::new(),
            facecolor: "white".to_string(),
            subplot_left: 0.0,
            subplot_right: 1.0,
            subplot_bottom: 0.0,
            subplot_top: 1.0,
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
        let font_scale = self.dpi / 72.0;
        if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            let backend = BitMapBackend::new(filename, (self.width, self.height));
            self.render_to_backend(py, backend, self.width, self.height, true, font_scale)
        } else {
            // 使用完整像素尺寸作为SVG坐标空间，确保字体大小正确
            let backend = SVGBackend::new(filename, (self.width, self.height));
            self.render_to_backend(py, backend, self.width, self.height, false, font_scale)?;
            // 后处理：设置SVG物理尺寸为英寸单位，与matplotlib一致
            if let Ok(content) = std::fs::read_to_string(filename) {
                let width_in = self.width as f64 / self.dpi;
                let height_in = self.height as f64 / self.dpi;
                // plotters SVGBackend 输出 width="pixel_width" height="pixel_height"
                // 替换为英寸单位
                let content = content
                    .replacen(&format!("width=\"{}\"", self.width), &format!("width=\"{}in\"", format!("{:.4}", width_in)), 1)
                    .replacen(&format!("height=\"{}\"", self.height), &format!("height=\"{}in\"", format!("{:.4}", height_in)), 1);
                let _ = std::fs::write(filename, content);
            }
            Ok(())
        }
    }

    fn show(&self, py: Python) -> PyResult<()> {
        let tmpdir = std::env::temp_dir();
        let path = tmpdir.join("rsplot_output.png");
        let filename = path.to_str().unwrap_or("/tmp/rsplot_output.png").to_string();
        let font_scale = self.dpi / 72.0;
        let backend = BitMapBackend::new(&filename, (self.width, self.height));
        self.render_to_backend(py, backend, self.width, self.height, true, font_scale)?;

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
    fn render_to_backend<B: DrawingBackend>(&self, py: Python, backend: B, actual_w: u32, actual_h: u32, fill_bg: bool, font_scale: f64) -> PyResult<()>
    where
        B::ErrorType: 'static,
    {
        let root = backend.into_drawing_area();

        // 仅位图后端填充白色背景，避免在SVG中产生额外的背景rect，
        // 保持与matplotlib的SVG输出一致（matplotlib SVG仅在axes区域内有白色背景）
        if fill_bg {
            let _ = root.fill(&WHITE);
        }

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

            // 内部边距应基于子图自身的宽高（与 matplotlib 一致），
            // 而不是整个 figure 的尺寸。否则子图越大，data area 越小。
            let tick_label_size = (ax.tick_labelsize * font_scale).ceil() as u32;
            let axis_label_size = (12.0 * font_scale).ceil() as u32;  // 轴标签字体大小
            let title_size = (14.0 * font_scale).ceil() as u32;

            // Y 轴标签区域: tick 标签 + 轴标签 + 间距
            let y_label_area = tick_label_size + axis_label_size + 8;
            // X 轴标签区域: tick 标签 + 轴标签 + 间距
            let x_label_area = tick_label_size + axis_label_size + 8;

            // 模拟 matplotlib 的 subplot 边距（占子图宽/高的比例）
            // matplotlib 默认: left=12.5%, right=10%, bottom=10%, top=10%
            // 但需要为 tick/axis label 预留空间，剩余空间用 margin 补足
            let target_left = (sub_w * 0.125) as u32;
            let target_right = (sub_w * 0.10) as u32;
            let target_bottom = (sub_h * 0.10) as u32;
            let target_top = if ax.title.is_empty() {
                (sub_h * 0.10) as u32
            } else {
                (sub_h * 0.10) as u32 + title_size + 5
            };

            let margin_left = target_left.saturating_sub(y_label_area);
            let margin_right = target_right.saturating_sub(5);
            let margin_bottom = target_bottom.saturating_sub(x_label_area);
            let margin_top = target_top.saturating_sub(5);

            let mut chart = ChartBuilder::on(&chart_area)
                .margin_top(margin_top.max(5))
                .margin_right(margin_right.max(5))
                .margin_bottom(margin_bottom.max(5))
                .margin_left(margin_left.max(5))
                .x_label_area_size(x_label_area)
                .y_label_area_size(y_label_area)
                .build_cartesian_2d(x_min..x_max, y_min..y_max)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to build chart: {}", e)))?;

            ax.render(py, &mut chart, (x_min, x_max), (y_min, y_max), font_scale, true)?;

            let twin_axes = ax.twin_axes.clone();
            drop(ax);
            for twin in &twin_axes {
                let ((tx_min, tx_max), (ty_min, ty_max)) = twin.compute_bounds();
                let (ux_min, ux_max) = if twin.is_twin_x { (tx_min, tx_max) } else { (x_min, x_max) };
                let (uy_min, uy_max) = if twin.is_twin_y { (ty_min, ty_max) } else { (y_min, y_max) };
                // twin axes 使用与主轴相同的布局参数
                let twin_tick_size = (twin.tick_labelsize * font_scale).ceil() as u32;
                let twin_axis_label_size = (12.0 * font_scale).ceil() as u32;
                let twin_y_label_area = twin_tick_size + twin_axis_label_size + 8;
                let twin_x_label_area = twin_tick_size + twin_axis_label_size + 8;
                let mut twin_chart = ChartBuilder::on(&chart_area)
                    .margin_top(margin_top.max(5))
                    .margin_right(margin_right.max(5))
                    .margin_bottom(margin_bottom.max(5))
                    .margin_left(margin_left.max(5))
                    .right_y_label_area_size(twin_y_label_area)
                    .top_x_label_area_size(twin_x_label_area)
                    .build_cartesian_2d(ux_min..ux_max, uy_min..uy_max)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to build twin chart: {}", e)))?;
                // twin axes 不填充背景，避免覆盖主轴数据
                twin.render(py, &mut twin_chart, (ux_min, ux_max), (uy_min, uy_max), font_scale, false)?;
            }
        }

        root.present()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to present: {}", e)))?;

        Ok(())
    }
}