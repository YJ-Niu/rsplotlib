use std::sync::Mutex;
use std::io::BufWriter;
use std::fs::File;

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
        // matplotlib 兼容的默认 subplots_adjust 边距
        // matplotlib 默认: left=0.125, right=0.9, bottom=0.11, top=0.88
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
            subplot_left: 0.125,
            subplot_right: 0.9,
            subplot_bottom: 0.11,
            subplot_top: 0.88,
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
        if filename.ends_with(".png") {
            // 渲染到 RGB 缓冲区
            let buf_size = (self.width as usize) * (self.height as usize) * 3;
            let mut buffer = vec![0u8; buf_size];
            let backend: BitMapBackend<'_, plotters::backend::RGBPixel> = BitMapBackend::with_buffer_and_format(
                &mut buffer,
                (self.width, self.height),
            )
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create bitmap backend: {}", e)))?;
            self.render_to_backend(py, backend, self.width, self.height, true, font_scale)?;

            // 写入 PNG 并嵌入 DPI 信息（pHYs chunk）
            let file = File::create(filename)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to create file: {}", e)))?;
            let ref mut w = BufWriter::new(file);
            let mut encoder = png::Encoder::new(w, self.width, self.height);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            // 最高压缩级别 + 自适应滤波，使文件大小与 matplotlib 一致
            encoder.set_compression(png::Compression::Best);
            encoder.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
            // 设置 DPI：png 使用 像素/米（1 英寸 = 0.0254 米）
            let ppm = (self.dpi / 0.0254).round() as u32;
            encoder.set_pixel_dims(Some(png::PixelDimensions {
                xppu: ppm,
                yppu: ppm,
                unit: png::Unit::Meter,
            }));
            let mut writer = encoder.write_header()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to write PNG header: {}", e)))?;
            writer.write_image_data(&buffer)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to write PNG data: {}", e)))?;
            Ok(())
        } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
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
            let _ = root.titled(&self.suptitle, ("sans-serif", 21.0 * font_scale));
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

            // 计算 tick/label 区域大小
            let tick_label_size = (ax.tick_labelsize * font_scale).ceil() as u32;

            // 估算 y/x tick label 区域大小（容纳最长的 tick 数字 + 少量 padding）
            let y_tick_area = tick_label_size + 6;
            let x_tick_area = tick_label_size + 6;

            // 检测 y 轴是否有可见的 tick 标签或 ylabel
            // 条件：ylabel 非空 或 (yticks_val 未被设为空 且 tick_left 或 tick_right 为真)
            let y_has_labels = !ax.ylabel.is_empty()
                || !matches!(ax.yticks_val, Some(ref v) if v.is_empty());
            let y_has_ticks = ax.tick_left || ax.tick_right;

            // 检测 x 轴是否有可见的 tick 标签或 xlabel
            let x_has_labels = !ax.xlabel.is_empty()
                || !matches!(ax.xticks_val, Some(ref v) if v.is_empty());
            let x_has_ticks = ax.tick_bottom || ax.tick_top;

            // axis label (y_desc/x_desc) 在 tick label 之外，需要额外空间
            // plotters 会把 y_desc 放在 y_label_area 中心，tick label 放在 y_label_area 右边缘
            // 如果没有 label 和 tick，最小保留 2px 以确保 plotters 正确绘制边界 spine
            let y_label_area = if !y_has_labels {
                2u32
            } else if ax.ylabel.is_empty() {
                y_tick_area
            } else {
                y_tick_area + tick_label_size + 6
            };
            let x_label_area = if !x_has_labels {
                2u32
            } else if ax.xlabel.is_empty() {
                x_tick_area
            } else {
                x_tick_area + tick_label_size + 6
            };
            // 抑制未使用变量警告
            let _ = y_has_ticks;
            let _ = x_has_ticks;

            // 顶部边距：ax.title 是通过 chart.draw_series(Text) 渲染的，
            // 文字在数据区顶部 y_max 处向上延伸 (VPos::Bottom)，所以不需要 plotters margin_top
            // 保留少量 margin_top 作为 title 与数据区之间的视觉间距
            let margin_top_internal = if ax.title.is_empty() { 0u32 } else { 4u32 };

            // 关键修复：chart 区域向左侧/上扩展，使 plotters 的 y_label_area 和 margin_top
            // 容纳在子图外部，最终 data area 正好等于 subplot 区域（与 matplotlib 一致）
            // plotters 内部布局：
            //   drawing_area = chart_area - margin(top, bottom, left, right)
            //   然后按 label_areas 切分出 plotting area (data area)
            // 因此：
            //   data_area.top    = chart_y0 + margin_top
            //   data_area.bottom = chart_y0 + chart_h - x_label_area
            //   data_area.left   = chart_x0 + y_label_area
            //   data_area.right  = chart_x0 + chart_w
            // 要使 data_area = subplot：
            //   chart_x0 = subplot_x - y_label_area
            //   chart_w  = subplot_w + y_label_area
            //   chart_y0 = subplot_y - margin_top
            //   chart_h  = subplot_h + x_label_area
            let y_label_actual = y_label_area;
            let x_label_actual = x_label_area;
            let margin_top_actual = margin_top_internal;

            // 限制扩展不超过 figure 边界（最左侧/最上侧子图可扩展到边）
            let chart_x0 = (x0 - y_label_actual as f64).max(0.0);
            let chart_y0 = (y0 - margin_top_actual as f64).max(0.0);
            let chart_w = (sub_w + y_label_actual as f64) as f64;
            let chart_h = (sub_h + x_label_actual as f64) as f64;

            // 防止超出 figure 右/下边界
            let chart_w = chart_w.min((total_w - chart_x0) as f64).max(1.0);
            let chart_h = chart_h.min((total_h - chart_y0) as f64).max(1.0);

            let chart_area = root.clone().shrink(
                (chart_x0 as i32, chart_y0 as i32),
                (chart_w as u32, chart_h as u32),
            );

            // 子图内部边距：y_label_area / x_label_area 已在 chart_area 尺寸中体现
            // margin_top 取最小值(4)用于 title 与数据区视觉间距
            let margin_left = 0u32;
            let margin_right = 0u32;
            let margin_bottom = 0u32;
            let margin_top = margin_top_actual;

            let mut chart = ChartBuilder::on(&chart_area)
                .margin_top(margin_top)
                .margin_right(margin_right)
                .margin_bottom(margin_bottom)
                .margin_left(margin_left)
                .x_label_area_size(x_label_actual)
                .y_label_area_size(y_label_actual)
                .build_cartesian_2d(x_min..x_max, y_min..y_max)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to build chart: {}", e)))?;

            // 将标题信息存到 axes 之外用：传入 subplot 在 figure 中的位置，用于在 figure root 上绘制
            let fig_subplot_info = (x0, y0, sub_w, sub_h);
            ax.render(py, &mut chart, (x_min, x_max), (y_min, y_max), font_scale, true, Some(&fig_subplot_info))?;

            let twin_axes = ax.twin_axes.clone();
            drop(ax);
            for twin in &twin_axes {
                let ((tx_min, tx_max), (ty_min, ty_max)) = twin.compute_bounds();
                let (ux_min, ux_max) = if twin.is_twin_x { (tx_min, tx_max) } else { (x_min, x_max) };
                let (uy_min, uy_max) = if twin.is_twin_y { (ty_min, ty_max) } else { (y_min, y_max) };
                // twin axes 使用与主轴相同的 chart_area，但 label area 在右侧/顶部
                let twin_tick_size = (twin.tick_labelsize * font_scale).ceil() as u32;
                let twin_y_label_area = twin_tick_size + 6;
                let twin_x_label_area = twin_tick_size + 6;
                let mut twin_chart = ChartBuilder::on(&chart_area)
                    .margin_top(0)
                    .margin_right(0)
                    .margin_bottom(0)
                    .margin_left(0)
                    .right_y_label_area_size(twin_y_label_area)
                    .top_x_label_area_size(twin_x_label_area)
                    .build_cartesian_2d(ux_min..ux_max, uy_min..uy_max)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to build twin chart: {}", e)))?;
                // twin axes 不填充背景，避免覆盖主轴数据
                twin.render(py, &mut twin_chart, (ux_min, ux_max), (uy_min, uy_max), font_scale, false, None)?;
            }
        }

        root.present()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to present: {}", e)))?;

        Ok(())
    }
}