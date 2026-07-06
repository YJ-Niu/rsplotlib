use plotters::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use std::fs::File;
use std::sync::Mutex;

use crate::figure::axes::Axes;
use crate::utils::font_stack;
use crate::utils::pyfuncs::{BASE_HSPACE, BASE_WSPACE, grid_position};
// colors not needed directly in this module

/// 默认图形尺寸（英寸），与 matplotlib 默认一致 (12.0, 9.0)
pub const DEFAULT_FIGSIZE: (f64, f64) = (12.0, 9.0);
/// 默认 DPI
pub const DEFAULT_DPI: f64 = 100.0;

/// savefig 位图输出的超采样倍数：先按此倍数放大渲染（2× 边长 = 4× 像素），
/// 再用盒式滤波（box filter）平均缩回目标尺寸。等效于 2×2 超采样抗锯齿（SSAA），
/// 让文字、marker、曲线边缘平滑，明显优于无抗锯齿；相比 4×4 只渲染 1/4 像素，
/// savefig 速度约为 4×4 的 2.7 倍，是画质与速度的平衡点。
pub const SUPERSAMPLE: u32 = 2;

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
    /// 当前选中的子图下标（plt.subplot 选中后，plt.* 路由到此 axes）
    pub current_axes_index: usize,
    pub facecolor: String,
    pub subplot_left: f64,
    pub subplot_right: f64,
    pub subplot_bottom: f64,
    pub subplot_top: f64,
}

impl Default for Figure {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl Figure {
    #[new]
    pub fn new() -> Self {
        let w = (DEFAULT_FIGSIZE.0 * DEFAULT_DPI).round() as u32;
        let h = (DEFAULT_FIGSIZE.1 * DEFAULT_DPI).round() as u32;
        // matplotlib 兼容的默认 subplots_adjust 边距
        // matplotlib 默认: left=0.125, right=0.9, bottom=0.11, top=0.88
        Figure {
            axes_list: Vec::new(),
            nrows: 1,
            ncols: 1,
            suptitle: String::new(),
            width: w,
            height: h,
            dpi: DEFAULT_DPI,
            axes_positions: Vec::new(),
            current_axes_index: 0,
            facecolor: "white".to_string(),
            subplot_left: 0.125,
            subplot_right: 0.9,
            subplot_bottom: 0.11,
            subplot_top: 0.88,
        }
    }

    #[doc = "设置图形像素尺寸"]
    fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    #[doc = "设置图形分辨率 (每英寸点数)"]
    fn set_dpi(&mut self, dpi: f64) {
        self.dpi = dpi;
    }

    fn suptitle(&mut self, text: String) {
        self.suptitle = text;
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (left=None, right=None, bottom=None, top=None, wspace=None, hspace=None))]
    fn subplots_adjust(
        &mut self,
        left: Option<f64>,
        right: Option<f64>,
        bottom: Option<f64>,
        top: Option<f64>,
        wspace: Option<f64>,
        hspace: Option<f64>,
    ) {
        if let Some(v) = left {
            self.subplot_left = v;
        }
        if let Some(v) = right {
            self.subplot_right = v;
        }
        if let Some(v) = bottom {
            self.subplot_bottom = v;
        }
        if let Some(v) = top {
            self.subplot_top = v;
        }
    }

    #[doc = "调整子图间距"]
    fn tight_layout(&mut self) {}

    #[doc = "设置图形背景颜色"]
    fn set_facecolor(&mut self, color: &str) {
        self.facecolor = color.to_string();
    }

    #[doc = "清除所有子图"]
    fn clear(&mut self) {
        self.axes_list.clear();
        self.axes_positions.clear();
        self.current_axes_index = 0;
    }

    #[doc = "清除所有子图"]
    fn clf(&mut self) {
        self.axes_list.clear();
        self.axes_positions.clear();
        self.current_axes_index = 0;
    }

    #[doc = "添加子图"]
    #[pyo3(signature = (spec))]
    #[allow(unused_variables)]
    fn add_subplot(&mut self, py: Python, spec: &Bound<'_, PyAny>) -> PyResult<Py<Axes>> {
        let (left, right, bottom, top) = if spec.getattr("rowStart").is_ok() {
            let num_rows: f64 = spec
                .getattr("numRows")?
                .extract::<i32>()
                .map(|v| v as f64)
                .unwrap_or(100.0);
            let num_cols: f64 = spec
                .getattr("numCols")?
                .extract::<i32>()
                .map(|v| v as f64)
                .unwrap_or(100.0);
            let row_start: f64 = spec
                .getattr("rowStart")?
                .extract::<i32>()
                .map(|v| v as f64)
                .unwrap_or(0.0);
            let row_stop: f64 = spec
                .getattr("rowStop")?
                .extract::<i32>()
                .map(|v| v as f64)
                .unwrap_or(num_rows);
            let col_start: f64 = spec
                .getattr("colStart")?
                .extract::<i32>()
                .map(|v| v as f64)
                .unwrap_or(0.0);
            let col_stop: f64 = spec
                .getattr("colStop")?
                .extract::<i32>()
                .map(|v| v as f64)
                .unwrap_or(num_cols);

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
        crate::utils::pyfuncs::init_axes_self_py(&ax_py, py);
        self.axes_list.push(ax_py.clone_ref(py));
        self.axes_positions.push((left, right, bottom, top));
        self.current_axes_index = self.axes_list.len() - 1;
        Ok(ax_py)
    }

    #[doc = "保存图形到文件\n\n参数:\n    filename: 文件名, 支持 .png/.jpg/.svg\n    dpi: 可选分辨率, 默认与创建时一致"]
    #[pyo3(signature = (filename, dpi=None))]
    fn savefig(&self, py: Python, filename: &str, dpi: Option<f64>) -> PyResult<()> {
        let used_dpi = dpi.unwrap_or(self.dpi);
        let font_scale = used_dpi / 72.0;
        if filename.ends_with(".png") {
            // SSAA 超采样：先按 SUPERSAMPLE 倍边长渲染，再盒式滤波缩回目标尺寸。
            // 含平滑渐变（colorbar / imshow）时颜色数远超 256，量化会产生明显色带，改写
            // 真彩 RGB PNG 保留平滑渐变；其余（折线/散点等）量化到 256 色近乎无损且体积仅
            // 真彩 1/3~1/4。
            let rgb = self.render_downsampled_rgb(py, font_scale)?;
            if self.has_gradient_content(py) {
                self.write_rgb_png_truecolor(filename, &rgb, used_dpi)
            } else {
                self.write_rgb_png_indexed(filename, &rgb, used_dpi)
            }
        } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            let rgb = self.render_downsampled_rgb(py, font_scale)?;
            self.write_rgb_jpg(filename, &rgb)
        } else {
            // 使用完整像素尺寸作为SVG坐标空间，确保字体大小正确
            let mut content = self.render_svg_string(py, font_scale)?;
            // 后处理：设置SVG物理尺寸为英寸单位，与matplotlib一致
            let width_in = self.width as f64 / used_dpi;
            let height_in = self.height as f64 / used_dpi;
            // plotters SVGBackend 输出 width="pixel_width" height="pixel_height"，替换为英寸单位
            content = content
                .replacen(
                    &format!("width=\"{}\"", self.width),
                    &format!("width=\"{:.4}in\"", width_in),
                    1,
                )
                .replacen(
                    &format!("height=\"{}\"", self.height),
                    &format!("height=\"{:.4}in\"", height_in),
                    1,
                );
            std::fs::write(filename, content)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to write SVG: {}", e)))?;
            Ok(())
        }
    }

    fn show(&self, py: Python) -> PyResult<()> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let path = cwd.join("rsplot_output.png");
        let filename = path.to_str().unwrap_or("rsplot_output.png").to_string();
        let font_scale = self.dpi / 72.0;
        let backend = BitMapBackend::new(&filename, (self.width, self.height));
        self.render_to_backend(py, backend, self.width, self.height, true, font_scale)?;

        if cfg!(target_os = "macos") {
            let _ = std::process::Command::new("open").arg(&filename).spawn();
        } else if cfg!(target_os = "linux") {
            let _ = std::process::Command::new("xdg-open")
                .arg(&filename)
                .spawn();
        }

        println!("Figure saved to: {}", filename);
        Ok(())
    }
}

impl Figure {
    /// 用 SVG 矢量后端把整张图渲染为 SVG 字符串（坐标空间为像素尺寸）。
    ///
    /// savefig(".svg") 用此入口，再把 width/height 改成英寸后落盘。
    ///
    /// 只注入 stroke-linejoin="round"（让折线拐角平滑，与位图 AA 的 round join 一致），
    /// **不注入** stroke-linecap="round"：dash 段用 matplotlib 默认的 butt 平头端点。
    /// 若强制 round 端点，每段短划 / 点会被两端半圆撑大且半圆朝向随切线变化，
    /// 视觉上呈现"方向杂乱、不沿整体方向"——正是要避免的。默认 butt 端点使 dash
    /// 段严格沿线方向、点保持短促，与位图路径 (draw_thick_polyline_aa, "butt") 一致。
    fn render_svg_string(&self, py: Python, font_scale: f64) -> PyResult<String> {
        use crate::figure::axes_render_elements::{clear_svg_dash_injects, take_svg_dash_injects};
        // 渲染前清空收集表，避免上一次渲染残留的注入信息。
        clear_svg_dash_injects();
        let mut svg = String::new();
        {
            let backend = SVGBackend::with_string(&mut svg, (self.width, self.height));
            self.render_to_backend(py, backend, self.width, self.height, false, font_scale)?;
        }
        // 虚线在 SVG 分支被画成整条连续 polyline；此处按收集到的 (颜色, 首点, dasharray)
        // 精确定位每条并注入原生 stroke-dasharray + butt 端点，使各段 dash 相位连续、
        // 像素形状规律一致。必须在下面 linejoin 替换之前做（那次替换会在 `<polyline `
        // 之后再插入 linejoin，两属性并存、互不影响）。
        let injects = take_svg_dash_injects();
        Self::inject_dash(&mut svg, &injects);
        let svg = svg.replace("<polyline ", "<polyline stroke-linejoin=\"round\" ");
        Ok(svg)
    }

    /// 给 SVG 中的虚线 polyline 注入原生 `stroke-dasharray`（及 butt 端点）。
    ///
    /// plotters 的 SVG 后端不支持 dasharray，虚线在渲染阶段被画成一条**完整连续**的
    /// polyline，同时把 (stroke 颜色 hex, 首点整数像素坐标, dasharray 字符串) 记录到线程
    /// 本地收集表。这里用「颜色 + 首点坐标」在生成的 SVG 里唯一定位对应 polyline，
    /// 在 `<polyline ` 之后插入 `stroke-dasharray="..." stroke-linecap="butt" `。
    ///
    /// plotters draw_path 输出的属性顺序固定为
    /// `<polyline fill="none" opacity="1" stroke="#RRGGBB" stroke-width="N" points="X0,Y0 ..."/>`，
    /// 故用 `stroke="{color}" stroke-width="` 作为锚点定位标签，再核对 `points="X0,Y0 `
    /// 前缀确认是目标虚线。已注入过 dasharray 的标签会被跳过，避免重复注入。
    fn inject_dash(svg: &mut String, injects: &[(String, i32, i32, String)]) {
        for (color, x0, y0, darr) in injects {
            let color_needle = format!("stroke=\"{}\" stroke-width=\"", color);
            let pts_prefix = format!("{},{} ", x0, y0);
            let mut from = 0usize;
            while let Some(rel) = svg[from..].find(&color_needle) {
                let cpos = from + rel;
                let tag_start = match svg[..cpos].rfind('<') {
                    Some(p) => p,
                    None => {
                        from = cpos + color_needle.len();
                        continue;
                    }
                };
                if svg[tag_start..].starts_with("<polyline")
                    && let Some(prel) = svg[cpos..].find("points=\"")
                {
                    let ppos = cpos + prel + "points=\"".len();
                    if svg[ppos..].starts_with(&pts_prefix) {
                        if !svg[tag_start..ppos].contains("stroke-dasharray") {
                            let attr =
                                format!("stroke-dasharray=\"{}\" stroke-linecap=\"butt\" ", darr);
                            svg.insert_str(tag_start + "<polyline ".len(), &attr);
                        }
                        break;
                    }
                }
                from = cpos + color_needle.len();
            }
        }
    }

    /// 以 SUPERSAMPLE 倍边长渲染整张图到 RGB 缓冲，再盒式滤波缩回目标尺寸。
    ///
    /// 等效 SUPERSAMPLE×SUPERSAMPLE 超采样抗锯齿：在 (width*ss, height*ss) 的大画布上光栅化，
    /// 每 ss×ss 个源像素取平均得到一个目标像素，使文字、marker、曲线边缘更平滑。
    /// 返回长度 width*height*3 的 RGB 缓冲（行主序，每像素 R,G,B）。
    fn render_downsampled_rgb(&self, py: Python, font_scale: f64) -> PyResult<Vec<u8>> {
        let ss = SUPERSAMPLE;
        let sw = self.width * ss;
        let sh = self.height * ss;
        let mut hi = vec![0u8; (sw as usize) * (sh as usize) * 3];
        {
            let backend: BitMapBackend<'_, plotters::backend::RGBPixel> =
                BitMapBackend::with_buffer_and_format(&mut hi, (sw, sh)).map_err(|e| {
                    PyRuntimeError::new_err(format!("Failed to create bitmap backend: {}", e))
                })?;
            // 传 actual_w/h = 超采样尺寸（render_to_backend 据此算出 ss 并放大各布局常量），
            // font_scale 也乘以 ss，让字体/线宽在放大画布上同比放大。
            self.render_to_backend(py, backend, sw, sh, true, font_scale * ss as f64)?;
        }
        Ok(downsample_box(&hi, sw, sh, ss))
    }

    /// 图中是否含平滑渐变内容（colorbar 或 imshow 图像）——决定 PNG 是否用真彩输出。
    ///
    /// 折线/散点等即便因抗锯齿产生上万种混合色，量化到 256 色仍近乎无损；只有 colorbar
    /// 渐变色带与 imshow 栅格图才需要真彩以避免可见色带。
    fn has_gradient_content(&self, py: Python) -> bool {
        for ax_py in &self.axes_list {
            let ax = ax_py.borrow(py);
            if ax.colorbar.is_some() {
                return true;
            }
            if ax
                .elements
                .iter()
                .any(|e| matches!(e, crate::core::elements::PlotElement::Image { .. }))
            {
                return true;
            }
        }
        false
    }

    /// 将 RGB 像素缓冲量化为至多 256 色调色板后写入索引（8-bit indexed）PNG 文件, 内嵌 DPI 元数据。
    ///
    /// 超采样降采样后的图含上万种颜色（多为抗锯齿混合色），真彩 PNG 每像素 3 字节、
    /// 文件较大。用八叉树（octree）量化出至多 256 色调色板、每像素只存 1 字节索引，
    /// 文件体积约为真彩的 1/3~1/4，且绘图内容（少数纯色 + 边缘混合色）用 256 色足以
    /// 近乎无损重现。八叉树映射只需 O(树深) 遍历，比 NeuQuant 快一个量级。
    fn write_rgb_png_indexed(&self, filename: &str, rgb: &[u8], dpi: f64) -> PyResult<()> {
        // palette: 长度 = 3*色数 的 RGB 连续调色板；indices: 每像素调色板下标。
        let (palette, indices) = quantize_octree(rgb, 256);

        let ppm = (dpi / 0.0254).round() as u32;
        let dims = png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        };
        let file = File::create(filename)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create file: {}", e)))?;
        let mut encoder = png::Encoder::new(file, self.width, self.height);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(palette);
        encoder.set_pixel_dims(Some(dims));
        // PNG 无损：Fast(fdeflate) 编码极快，索引数据本就小，压缩比也很好。
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder
            .write_header()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to write PNG header: {}", e)))?;
        writer
            .write_image_data(&indices)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to write PNG data: {}", e)))?;
        Ok(())
    }

    /// 将 RGB 像素缓冲写入真彩（24-bit RGB）PNG 文件, 内嵌 DPI 元数据。
    ///
    /// 用于含平滑渐变（colorbar / imshow）的图：颜色数远超 256, 索引量化会产生色带,
    /// 真彩输出可无损保留渐变。
    fn write_rgb_png_truecolor(&self, filename: &str, rgb: &[u8], dpi: f64) -> PyResult<()> {
        let ppm = (dpi / 0.0254).round() as u32;
        let dims = png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        };
        let file = File::create(filename)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create file: {}", e)))?;
        let mut encoder = png::Encoder::new(file, self.width, self.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_pixel_dims(Some(dims));
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder
            .write_header()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to write PNG header: {}", e)))?;
        writer
            .write_image_data(rgb)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to write PNG data: {}", e)))?;
        Ok(())
    }

    /// 将 RGB 像素缓冲编码为 JPEG（质量 90）写入文件。
    fn write_rgb_jpg(&self, filename: &str, rgb: &[u8]) -> PyResult<()> {
        use jpeg_encoder::{ColorType, Encoder};
        let encoder = Encoder::new_file(filename, 90).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to create JPEG encoder: {}", e))
        })?;
        encoder
            .encode(rgb, self.width as u16, self.height as u16, ColorType::Rgb)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to encode JPEG: {}", e)))?;
        Ok(())
    }

    fn render_to_backend<B: DrawingBackend>(
        &self,
        py: Python,
        backend: B,
        actual_w: u32,
        actual_h: u32,
        fill_bg: bool,
        font_scale: f64,
    ) -> PyResult<()>
    where
        B::ErrorType: 'static,
    {
        let root = backend.into_drawing_area();

        // 超采样倍数：位图 savefig 会以 self.width*ss × self.height*ss 的画布渲染
        // （actual_w = self.width*ss），再盒式滤波缩回原尺寸。ss>1 时所有尺寸
        // （字体/线宽已由 font_scale 体现、marker、以及下面的固定像素常量）都要
        // 同步放大，否则超采样画布上的布局比例会失真。SVG/普通位图 ss=1。
        let ss = (actual_w as f64 / (self.width.max(1) as f64)).max(1.0);

        // marker 尺寸单位是 points，其像素大小取决于图形真实分辨率 (self.dpi)，
        // 与 savefig 传入的 font_scale (影响字体/线宽) 解耦：markersize 只调整
        // marker 大小，不随字体/线宽缩放变化。超采样时需乘以 ss 才能在放大画布上
        // 保持正确的相对大小。
        let marker_scale = (self.dpi / 72.0) * ss;

        // tick/label 区域里除字体外的固定像素 padding，超采样时按 ss 放大。
        let pad6 = (6.0 * ss).round() as u32;
        let pad2 = (2.0 * ss).round() as u32;
        let title_gap = (4.0 * ss).round() as u32;

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
            let sup_family = font_stack::select_family(&self.suptitle);
            let sup_size = 21.0 * 1.30 * font_scale;
            // plotters 的 titled() 会把标题带贴着画布顶边（起始 y=0），显得太靠上。
            // 先给顶部留一段内边距（约半个字号），使总标题下移，接近 matplotlib
            // suptitle 默认 y≈0.98 的观感。返回的子区域丢弃，子图仍绘制在原 root 上，
            // 布局不受影响。
            let sup_top_pad = (sup_size * 0.5).round() as i32;
            let _ = root
                .margin(sup_top_pad, 0, 0, 0)
                .titled(&self.suptitle, (sup_family.as_str(), sup_size));
        }

        let total_w = actual_w as f64;
        let total_h = actual_h as f64;

        // 规则网格（由 subplot/subplots 创建，格子数 == nrows×ncols 且多于 1 个）在渲染阶段
        // 依据坐标轴标签动态调整间距：只要有子图设置了 Y 轴标签，水平间距翻倍；只要有子图
        // 设置了 X 轴标签，垂直间距翻倍——为标签腾出空间，避免与相邻子图重叠。
        // 通过 add_subplot/gridspec 等自定义布局（nrows×ncols 与格子数不符）保持原位置不变。
        let is_regular_grid =
            self.axes_list.len() == self.nrows * self.ncols && self.axes_list.len() > 1;
        let (grid_wspace, grid_hspace) = if is_regular_grid {
            let any_ylabel = self
                .axes_list
                .iter()
                .any(|a| !a.borrow(py).ylabel.is_empty());
            let any_xlabel = self
                .axes_list
                .iter()
                .any(|a| !a.borrow(py).xlabel.is_empty());
            let w = if any_ylabel {
                BASE_WSPACE * 2.0
            } else {
                BASE_WSPACE
            };
            let h = if any_xlabel {
                BASE_HSPACE * 2.0
            } else {
                BASE_HSPACE
            };
            (w, h)
        } else {
            (BASE_WSPACE, BASE_HSPACE)
        };

        for (i, ax_py) in self.axes_list.iter().enumerate() {
            let ax = ax_py.borrow(py);

            let ((x_min, x_max), (y_min, y_max)) = ax.compute_bounds();

            let (left, right, bottom, top) = if is_regular_grid {
                grid_position(
                    i / self.ncols,
                    i % self.ncols,
                    self.nrows,
                    self.ncols,
                    grid_wspace,
                    grid_hspace,
                )
            } else if i < self.axes_positions.len() {
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

            let x0 = plot_left * total_w;
            let y0 = (1.0 - plot_top_frac) * total_h;
            let sub_w = (plot_right - plot_left) * total_w;
            let sub_h = (plot_top_frac - plot_bottom_frac) * total_h;

            if sub_w <= 0.0 || sub_h <= 0.0 {
                drop(ax);
                continue;
            }

            // —— 刻度值 / 坐标轴标签区域尺寸（随刻度值位数、字号自动调整）——
            // tick 标签渲染字体：与 axes.rs 的 label_size 完全一致，用于精确测量刻度值像素宽度。
            let tick_font_size = crate::figure::axes::scale_font(ax.tick_labelsize, font_scale);
            // 字体高度：x 刻度值竖向占位、坐标轴标签旋转后横向占位的近似值。
            let tick_label_size = tick_font_size.ceil() as u32;
            // plotters 刻度线长度 tick_px；刻度值离轴距离 label_dist = 2*tick_px（见 draw_impl）。
            let tick_px = (3.5 * font_scale).round().max(1.0) as u32;
            let label_dist = tick_px * 2;

            // 是否真正显示刻度值：刻度开启 + spine 存在 + 未被 plt.{x,y}ticks([]) 显式清空。
            let y_ticklabels_shown = (ax.tick_left || ax.tick_right)
                && (ax.spine_left || ax.spine_right)
                && !matches!(ax.yticks_val, Some(ref v) if v.is_empty());
            let x_ticklabels_shown = (ax.tick_bottom || ax.tick_top)
                && (ax.spine_bottom || ax.spine_top)
                && !matches!(ax.xticks_val, Some(ref v) if v.is_empty());

            // y 刻度值从轴线向外占用的空间：最长刻度值的实际渲染宽度（位数越多 / 字号越大越宽）
            // + 离轴距离；无刻度值时为 0，使坐标轴标签紧贴坐标轴。
            let y_tick_area = if y_ticklabels_shown {
                let labels = y_tick_label_strings(py, &ax, y_min, y_max);
                measure_max_text_width(&labels, tick_font_size) + label_dist
            } else {
                0
            };
            // x 刻度值为水平文本，竖向占用 = 字体高度 + 离轴距离；无刻度值时为 0。
            let x_tick_area = if x_ticklabels_shown {
                tick_label_size + label_dist
            } else {
                0
            };

            // 坐标轴标签（ylabel/xlabel）在刻度值之外：额外留 pad6 间隙 + 标签自身占位（字体高度）。
            // plotters 把 y_desc 贴 y_label_area 左边缘、刻度值贴右边缘（近轴），故加宽 y_tick_area
            // 会自动把 ylabel 左移、远离刻度值；刻度值为空时 y_tick_area=0，ylabel 紧贴坐标轴。
            // 无标签也无刻度值时最小保留 pad2，确保 plotters 正确绘制边界 spine。
            let y_label_area = if ax.ylabel.is_empty() {
                if y_ticklabels_shown { y_tick_area + pad2 } else { pad2 }
            } else {
                y_tick_area + pad6 + tick_label_size
            };
            let x_label_area = if ax.xlabel.is_empty() {
                if x_ticklabels_shown { x_tick_area + pad2 } else { pad2 }
            } else {
                x_tick_area + pad6 + tick_label_size
            };

            // 顶部边距：ax.title 是通过 chart.draw_series(Text) 渲染的，
            // 文字在数据区顶部 y_max 处向上延伸 (VPos::Bottom)，所以不需要 plotters margin_top
            // 保留少量 margin_top 作为 title 与数据区之间的视觉间距
            let margin_top_internal = if ax.title.is_empty() { 0u32 } else { title_gap };

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
            let chart_w = sub_w + y_label_actual as f64;
            let chart_h = sub_h + x_label_actual as f64;

            // 防止超出 figure 右/下边界
            let chart_w = chart_w.min(total_w - chart_x0).max(1.0);
            let chart_h = chart_h.min(total_h - chart_y0).max(1.0);

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
            ax.render(
                py,
                &mut chart,
                (x_min, x_max),
                (y_min, y_max),
                font_scale,
                marker_scale,
                true,
                fill_bg,
                ss,
                Some(&fig_subplot_info),
            )?;

            // 非居中的 xlabel/ylabel：plotters 的 x_desc/y_desc 只能居中，
            // Axes::render 已在 loc 非居中时禁用内置 desc，这里用绝对像素在 root 上手动绘制。
            // 数据区四边的绝对像素坐标（与 plotters 布局一致）：
            //   data_left = chart_x0 + y_label_area, data_right = chart_x0 + chart_w
            //   data_top  = chart_y0 + margin_top,   data_bottom = chart_y0 + chart_h - x_label_area
            if !ax.xlabel.is_empty() && ax.xlabel_loc != "center" {
                let tick_px = crate::figure::axes::scale_font(ax.tick_labelsize, font_scale);
                let data_left = chart_x0 + y_label_actual as f64;
                let data_right = chart_x0 + chart_w;
                let xsize = if ax.xlabel_fontsize > 0.0 {
                    crate::figure::axes::scale_font(ax.xlabel_fontsize, font_scale)
                } else {
                    0.0
                };
                crate::figure::axes_title::draw_xlabel_manual(
                    &root,
                    &ax.xlabel,
                    &ax.xlabel_loc,
                    xsize,
                    tick_px,
                    ax.xlabel_color,
                    ax.xlabel_family.as_deref(),
                    data_left,
                    data_right,
                    chart_y0 + chart_h,
                )?;
            }
            if !ax.ylabel.is_empty() && ax.ylabel_loc != "center" {
                let tick_px = crate::figure::axes::scale_font(ax.tick_labelsize, font_scale);
                let data_top = chart_y0 + margin_top as f64;
                let data_bottom = chart_y0 + chart_h - x_label_actual as f64;
                let ysize = if ax.ylabel_fontsize > 0.0 {
                    crate::figure::axes::scale_font(ax.ylabel_fontsize, font_scale)
                } else {
                    0.0
                };
                crate::figure::axes_title::draw_ylabel_manual(
                    &root,
                    &ax.ylabel,
                    &ax.ylabel_loc,
                    ysize,
                    tick_px,
                    ax.ylabel_color,
                    ax.ylabel_family.as_deref(),
                    chart_x0,
                    data_top,
                    data_bottom,
                )?;
            }

            // 颜色条：在数据区右侧空白 margin 内绘制渐变色带 + 刻度标签。
            // 需在 drop(ax) 之前读取 ax.colorbar。数据区四边像素坐标与 plotters
            // 布局一致（见上文 xlabel/ylabel 手动绘制处的推导）。
            if let Some((cb_cmap, cb_vmin, cb_vmax)) = &ax.colorbar {
                let data_right = chart_x0 + chart_w;
                let data_top = chart_y0 + margin_top as f64;
                let data_bottom = chart_y0 + chart_h - x_label_actual as f64;
                crate::figure::axes_colorbar::draw_colorbar(
                    &root,
                    cb_cmap,
                    *cb_vmin,
                    *cb_vmax,
                    data_right,
                    data_top,
                    data_bottom,
                    total_w,
                    font_scale,
                    ss,
                )?;
            }

            let twin_axes = ax.twin_axes.clone();
            drop(ax);
            for twin in &twin_axes {
                let ((tx_min, tx_max), (ty_min, ty_max)) = twin.compute_bounds();
                let (ux_min, ux_max) = if twin.is_twin_x {
                    (tx_min, tx_max)
                } else {
                    (x_min, x_max)
                };
                let (uy_min, uy_max) = if twin.is_twin_y {
                    (ty_min, ty_max)
                } else {
                    (y_min, y_max)
                };
                // twin axes 使用与主轴相同的 chart_area，但 label area 在右侧/顶部
                let twin_tick_size =
                    crate::figure::axes::scale_font(twin.tick_labelsize, font_scale).ceil() as u32;
                let twin_y_label_area = twin_tick_size + pad6;
                let twin_x_label_area = twin_tick_size + pad6;
                let mut twin_chart = ChartBuilder::on(&chart_area)
                    .margin_top(0)
                    .margin_right(0)
                    .margin_bottom(0)
                    .margin_left(0)
                    .right_y_label_area_size(twin_y_label_area)
                    .top_x_label_area_size(twin_x_label_area)
                    .build_cartesian_2d(ux_min..ux_max, uy_min..uy_max)
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to build twin chart: {}", e))
                    })?;
                // twin axes 不填充背景，避免覆盖主轴数据
                twin.render(
                    py,
                    &mut twin_chart,
                    (ux_min, ux_max),
                    (uy_min, uy_max),
                    font_scale,
                    marker_scale,
                    false,
                    fill_bg,
                    ss,
                    None,
                )?;
            }
        }

        root.present()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to present: {}", e)))?;

        Ok(())
    }
}

/// 返回默认图形尺寸 (width, height)
#[pyfunction]
pub fn get_default_figsize() -> (f64, f64) {
    DEFAULT_FIGSIZE
}

/// 返回默认 DPI
#[pyfunction]
pub fn get_default_dpi() -> f64 {
    DEFAULT_DPI
}

/// 用与 axes.rs 渲染刻度值一致的字体（"sans-serif" + 指定字号）测量一组标签的最大
/// 渲染像素宽度。据此按刻度值实际宽度（随位数、字号变化）预留 y 轴刻度值区域，使
/// 坐标轴标签与刻度值不重叠。空标签集或非法字号返回 0。
fn measure_max_text_width(labels: &[String], font_size: f64) -> u32 {
    if font_size <= 0.0 {
        return 0;
    }
    let font = ("sans-serif", font_size).into_font();
    labels
        .iter()
        .filter(|s| !s.is_empty())
        .filter_map(|s| font.box_size(s).ok().map(|(w, _)| w))
        .max()
        .unwrap_or(0)
}

/// 计算 y 轴刻度值的显示字符串，与 axes.rs 的渲染逻辑保持一致：
/// - 类别型（同时设置 yticks + ytick_labels）直接用字符串标签；
/// - 否则按 locator > yticks_val > nice_ticks 得到主刻度值，log 轴用科学计数、
///   线性轴用 format_linear_tick 格式化。
///
/// 仅用于测量刻度值宽度，plotters 实际渲染的 key points 与此同量级（位数一致），
/// 足以准确预留区域。
fn y_tick_label_strings(py: Python<'_>, ax: &Axes, y_min: f64, y_max: f64) -> Vec<String> {
    if let (Some(ticks), Some(labels)) = (&ax.yticks_val, &ax.ytick_labels)
        && !ticks.is_empty()
        && !labels.is_empty()
    {
        return labels.clone();
    }
    let yticks: Vec<f64> = ax
        .yaxis_major_locator
        .as_ref()
        .and_then(|loc| {
            loc.bind(py)
                .call_method1("tick_values", (y_min, y_max))
                .ok()
                .and_then(|r| r.extract::<Vec<f64>>().ok())
        })
        .or_else(|| ax.yticks_val.clone())
        .unwrap_or_else(|| crate::figure::axes_mesh::nice_ticks(y_min, y_max));
    let ylog = ax.yscale == "log";
    yticks
        .iter()
        .map(|v| {
            if ylog {
                format!("{:.1e}", 10.0f64.powf(*v))
            } else {
                crate::figure::axes_mesh::format_linear_tick(*v)
            }
        })
        .collect()
}

/// 盒式滤波下采样：把 (sw, sh) 的 RGB 缓冲按 factor×factor 块求平均，
/// 缩到 (sw/factor, sh/factor)。用于 savefig 的超采样抗锯齿。
///
/// 每个目标像素 = 对应 factor×factor 源像素各通道的算术平均（带 +area/2 四舍五入）。
/// 要求 sw、sh 均为 factor 的整数倍（render_downsampled_rgb 保证：sw=width*ss）。
///
/// 输出按目标行水平切块，用 std::thread::scope 并行填充：各线程读取源缓冲的
/// 只读切片、写各自独立的输出行段，无数据竞争。大图（16× 像素）下明显提速。
fn downsample_box(src: &[u8], sw: u32, sh: u32, factor: u32) -> Vec<u8> {
    let dw = (sw / factor) as usize;
    let dh = (sh / factor) as usize;
    let sw = sw as usize;
    let mut out = vec![0u8; dw * dh * 3];
    if dh == 0 || dw == 0 {
        return out;
    }
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(dh)
        .min(8);
    if nthreads <= 1 {
        downsample_rows(src, &mut out, 0, dh, dw, sw, factor as usize);
        return out;
    }
    let rows_per = dh.div_ceil(nthreads);
    std::thread::scope(|scope| {
        let mut rest = out.as_mut_slice();
        let mut start = 0usize;
        while start < dh {
            let end = (start + rows_per).min(dh);
            let take = (end - start) * dw * 3;
            let (chunk, tail) = rest.split_at_mut(take);
            rest = tail;
            scope.spawn(move || {
                downsample_rows(src, chunk, start, end, dw, sw, factor as usize);
            });
            start = end;
        }
    });
    out
}

/// 对目标行区间 [dy_start, dy_end) 做盒式平均，结果写入 `dst`（dst 从该区间起始行开始，
/// 局部下标从 0 计）。供 downsample_box 单线程或并行分块调用。
fn downsample_rows(
    src: &[u8],
    dst: &mut [u8],
    dy_start: usize,
    dy_end: usize,
    dw: usize,
    sw: usize,
    f: usize,
) {
    let area = (f * f) as u32;
    let half = area / 2;
    let stride = sw * 3;
    for dy in dy_start..dy_end {
        let row_base = dy * f * stride;
        let dst_row = (dy - dy_start) * dw * 3;
        for dx in 0..dw {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut base = row_base + dx * f * 3;
            for _ in 0..f {
                let mut idx = base;
                for _ in 0..f {
                    // SAFETY: 调用方保证 sw、sh 均为 f 的整数倍，故 dy*f+j < sh、dx*f+k < sw，
                    // 因此 idx+2 < src.len()、o+2 < dst.len()，全部索引恒在界内。用 get_unchecked
                    // 消除逐像素边界检查，让这段热循环可被自动向量化。
                    unsafe {
                        r += *src.get_unchecked(idx) as u32;
                        g += *src.get_unchecked(idx + 1) as u32;
                        b += *src.get_unchecked(idx + 2) as u32;
                    }
                    idx += 3;
                }
                base += stride;
            }
            let o = dst_row + dx * 3;
            unsafe {
                *dst.get_unchecked_mut(o) = ((r + half) / area) as u8;
                *dst.get_unchecked_mut(o + 1) = ((g + half) / area) as u8;
                *dst.get_unchecked_mut(o + 2) = ((b + half) / area) as u8;
            }
        }
    }
}

/// 八叉树深度：RGB 各 8 bit，最多 8 层细分。
const OCTREE_LEVELS: usize = 8;
/// children 中"无子节点"的哨兵值（节点存于 arena 的下标，u32::MAX 表示空）。
const OCTREE_NONE: u32 = u32::MAX;

/// 八叉树节点。内部节点只做路由（children 指向子节点，累加量为 0）；
/// 叶子节点累计落入该颜色区域的像素数与各通道分量之和，用于算平均色。
struct OctreeNode {
    is_leaf: bool,
    pixel_count: u64,
    r_sum: u64,
    g_sum: u64,
    b_sum: u64,
    children: [u32; 8],
    palette_index: u16,
}

impl OctreeNode {
    fn new(is_leaf: bool) -> Self {
        OctreeNode {
            is_leaf,
            pixel_count: 0,
            r_sum: 0,
            g_sum: 0,
            b_sum: 0,
            children: [OCTREE_NONE; 8],
            palette_index: 0,
        }
    }
}

/// Gervautz–Purgathofer 八叉树颜色量化器。
///
/// 每种颜色按 RGB 比特位从高到低逐层选子节点插入，到第 8 层落成叶子。叶子数超过
/// 上限时，从最深可归约层取一个内部节点，把它的（此时必为叶子的）子节点合并进它、
/// 自身变叶子，从而减少颜色数。映射时同样按比特位下行到叶子取调色板下标，O(树深)。
struct Octree {
    nodes: Vec<OctreeNode>,
    /// reducible[level] = 该层所有"内部节点"的 arena 下标；归约时优先取最深层。
    reducible: Vec<Vec<u32>>,
    leaf_count: usize,
    max_colors: usize,
}

impl Octree {
    fn new(max_colors: usize) -> Self {
        let mut nodes = Vec::with_capacity(2048);
        nodes.push(OctreeNode::new(false)); // 根节点（内部节点），下标 0
        Octree {
            nodes,
            reducible: vec![Vec::new(); OCTREE_LEVELS],
            leaf_count: 0,
            max_colors,
        }
    }

    /// 第 `level` 层用哪个 bit 选子节点（0 层用最高位 bit7）。
    #[inline(always)]
    fn child_index(r: u32, g: u32, b: u32, level: usize) -> usize {
        let bit = 7 - level;
        ((((r >> bit) & 1) << 2) | (((g >> bit) & 1) << 1) | ((b >> bit) & 1)) as usize
    }

    /// 插入一种颜色（`weight` = 该颜色的像素数）：下行到叶子并按权累加，
    /// 之后按需归约到颜色上限内。加权累加使叶子平均色为像素加权质心，
    /// 画质与"逐像素插入"完全等价，但只需按唯一色调用一次。
    fn add_color(&mut self, r: u8, g: u8, b: u8, weight: u64) {
        let (r, g, b) = (r as u32, g as u32, b as u32);
        let mut nid = 0usize;
        for level in 0..OCTREE_LEVELS {
            if self.nodes[nid].is_leaf {
                break;
            }
            let ci = Self::child_index(r, g, b, level);
            let child = self.nodes[nid].children[ci];
            nid = if child == OCTREE_NONE {
                let new_id = self.nodes.len();
                let make_leaf = level + 1 >= OCTREE_LEVELS;
                self.nodes.push(OctreeNode::new(make_leaf));
                self.nodes[nid].children[ci] = new_id as u32;
                if make_leaf {
                    self.leaf_count += 1;
                } else {
                    self.reducible[level + 1].push(new_id as u32);
                }
                new_id
            } else {
                child as usize
            };
        }
        let node = &mut self.nodes[nid];
        node.pixel_count += weight;
        node.r_sum += r as u64 * weight;
        node.g_sum += g as u64 * weight;
        node.b_sum += b as u64 * weight;

        while self.leaf_count > self.max_colors {
            self.reduce();
        }
    }

    /// 归约一次：取最深可归约层的一个内部节点，合并其子叶子、自身变叶子。
    ///
    /// 因为总是取最深非空层，被归约节点的子节点必然都已是叶子（若有内部子节点，
    /// 它会登记在更深层，与"最深非空"矛盾），故可直接累加子节点的分量和。
    fn reduce(&mut self) {
        let mut level = OCTREE_LEVELS - 1;
        while level > 0 && self.reducible[level].is_empty() {
            level -= 1;
        }
        let nid = match self.reducible[level].pop() {
            Some(id) => id as usize,
            None => return,
        };
        let children = self.nodes[nid].children;
        let mut r = 0u64;
        let mut g = 0u64;
        let mut b = 0u64;
        let mut cnt = 0u64;
        let mut merged = 0usize;
        for c in children {
            if c != OCTREE_NONE {
                let child = &self.nodes[c as usize];
                r += child.r_sum;
                g += child.g_sum;
                b += child.b_sum;
                cnt += child.pixel_count;
                merged += 1;
            }
        }
        let node = &mut self.nodes[nid];
        node.is_leaf = true;
        node.r_sum = r;
        node.g_sum = g;
        node.b_sum = b;
        node.pixel_count = cnt;
        node.children = [OCTREE_NONE; 8];
        self.leaf_count -= merged;
        self.leaf_count += 1;
    }

    /// 遍历所有叶子生成 RGB 调色板（叶子平均色），同时把调色板下标写回叶子节点。
    fn build_palette(&mut self) -> Vec<u8> {
        let mut palette = Vec::with_capacity(self.leaf_count * 3);
        let mut idx: u16 = 0;
        let mut stack = vec![0u32];
        while let Some(nid) = stack.pop() {
            let nid = nid as usize;
            if self.nodes[nid].is_leaf {
                let cnt = self.nodes[nid].pixel_count.max(1);
                palette.push((self.nodes[nid].r_sum / cnt) as u8);
                palette.push((self.nodes[nid].g_sum / cnt) as u8);
                palette.push((self.nodes[nid].b_sum / cnt) as u8);
                self.nodes[nid].palette_index = idx;
                idx += 1;
            } else {
                let children = self.nodes[nid].children;
                for c in children {
                    if c != OCTREE_NONE {
                        stack.push(c);
                    }
                }
            }
        }
        palette
    }

    /// 把一种颜色映射到调色板下标：按比特位下行到叶子取 palette_index。
    #[inline]
    fn index_of(&self, r: u8, g: u8, b: u8) -> u8 {
        let (r, g, b) = (r as u32, g as u32, b as u32);
        let mut nid = 0usize;
        for level in 0..OCTREE_LEVELS {
            let node = &self.nodes[nid];
            if node.is_leaf {
                return node.palette_index as u8;
            }
            let ci = Self::child_index(r, g, b, level);
            let child = node.children[ci];
            if child == OCTREE_NONE {
                return node.palette_index as u8; // 防御：映射的颜色均已插入过，理论上不会走到
            }
            nid = child as usize;
        }
        self.nodes[nid].palette_index as u8
    }
}

/// Fibonacci 散列常数（2^32 / 黄金比例），用于去重哈希表。
const DEDUP_HASH_MUL: u32 = 0x9E37_79B1;

/// 单个像素条带（band）去重结果。
struct BandDedup {
    /// local id -> 24-bit 颜色（首次出现顺序）
    keys: Vec<u32>,
    /// local id -> 该颜色在本条带内的像素数
    cnt: Vec<u32>,
    /// 本条带每像素的 local id（长度 = 条带像素数）
    ids: Vec<u32>,
}

/// 对一段连续 RGB 像素做颜色去重：返回条带内唯一色（首次出现顺序）、各色像素数、
/// 及每像素的条带内 local id。开放寻址哈希表（2 的幂容量、Fibonacci 散列、线性探测）
/// + "上一像素颜色"缓存吃掉大片同色区。供 quantize_octree 单线程或并行分块调用。
fn dedup_band(rgb: &[u8]) -> BandDedup {
    let npix = rgb.len() / 3;
    let mut bits = 14u32; // 每条带初始 16384 槽，驻留 CPU 缓存
    let mut slots = vec![u32::MAX; 1usize << bits];
    let mut mask = slots.len() - 1;
    let mut keys: Vec<u32> = Vec::new();
    let mut cnt: Vec<u32> = Vec::new();
    let mut ids = vec![0u32; npix];
    let mut prev_key = u32::MAX;
    let mut prev_id = 0u32;
    for i in 0..npix {
        let key =
            ((rgb[i * 3] as u32) << 16) | ((rgb[i * 3 + 1] as u32) << 8) | (rgb[i * 3 + 2] as u32);
        let id = if key == prev_key {
            prev_id
        } else {
            let mut slot = (key.wrapping_mul(DEDUP_HASH_MUL) >> (32 - bits)) as usize;
            loop {
                let s = slots[slot];
                if s == u32::MAX {
                    let id = keys.len() as u32;
                    slots[slot] = id;
                    keys.push(key);
                    cnt.push(0);
                    // 装载率 > 3/4 时翻倍重散列（极少触发）。
                    if keys.len() * 4 >= slots.len() * 3 {
                        bits += 1;
                        slots = vec![u32::MAX; 1usize << bits];
                        mask = slots.len() - 1;
                        for (uid, &k) in keys.iter().enumerate() {
                            let mut sl = (k.wrapping_mul(DEDUP_HASH_MUL) >> (32 - bits)) as usize;
                            while slots[sl] != u32::MAX {
                                sl = (sl + 1) & mask;
                            }
                            slots[sl] = uid as u32;
                        }
                    }
                    break id;
                } else if keys[s as usize] == key {
                    break s;
                }
                slot = (slot + 1) & mask;
            }
        };
        prev_key = key;
        prev_id = id;
        ids[i] = id;
        cnt[id as usize] += 1;
    }
    BandDedup { keys, cnt, ids }
}

/// 八叉树颜色量化：把 RGB 缓冲量化到至多 `max_colors` 种颜色。
///
/// 返回 `(palette, indices)`：palette 为 RGB 三元组连续排列（长度 = 3 * 色数），
/// indices 为每像素的调色板下标（长度 = 像素数）。用于把真彩缓冲写成索引 PNG，
/// 文件体积约降到真彩的 1/3~1/4，且映射为 O(树深)、确定性、对纯色保留极好。
///
/// 去重是最重的一步（要扫描全部像素）。将像素按条带切分、多线程各自建局部去重表，
/// 再按**固定的条带顺序**合并为全局唯一色（保证结果确定、可复现），最后按条带并行
/// 回填每像素索引。建树/求调色板只处理去重后的少量颜色（典型上万），成本很小。
fn quantize_octree(rgb: &[u8], max_colors: usize) -> (Vec<u8>, Vec<u8>) {
    let npix = rgb.len() / 3;

    // ---- 并行去重 ----
    // 绘图图像颜色种类通常只有上万种（远少于像素数），但抗锯齿边缘会产生大量混合色。
    // 各线程处理一段像素、产出局部唯一色 + 每像素 local id，避免对上百万像素串行走哈希。
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(8);
    let bands: Vec<BandDedup> = if nthreads <= 1 || npix < 200_000 {
        vec![dedup_band(rgb)]
    } else {
        let pix_per = npix.div_ceil(nthreads);
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            let mut start = 0usize;
            while start < npix {
                let end = (start + pix_per).min(npix);
                let slice = &rgb[start * 3..end * 3];
                handles.push(scope.spawn(move || dedup_band(slice)));
                start = end;
            }
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        })
    };

    // ---- 合并各条带唯一色为全局唯一色 ----
    // 按 band 顺序、band 内首次出现顺序插入全局哈希表，结果确定、可复现。
    // 全局表按各条带唯一色总数一次性开够（装载率 < 3/4），合并中不再重散列。
    let total_local: usize = bands.iter().map(|b| b.keys.len()).sum();
    let mut gbits = 15u32;
    while (1usize << gbits) * 3 < total_local.saturating_mul(4).max(1) {
        gbits += 1;
    }
    let mut gslots = vec![u32::MAX; 1usize << gbits];
    let gmask = gslots.len() - 1;
    let mut uniq_key: Vec<u32> = Vec::new();
    let mut uniq_cnt: Vec<u32> = Vec::new();
    // 每条带 local id -> 全局 id 的重映射表
    let mut remaps: Vec<Vec<u32>> = Vec::with_capacity(bands.len());
    for b in &bands {
        let mut remap = vec![0u32; b.keys.len()];
        for (lid, &key) in b.keys.iter().enumerate() {
            let mut slot = (key.wrapping_mul(DEDUP_HASH_MUL) >> (32 - gbits)) as usize;
            let gid = loop {
                let s = gslots[slot];
                if s == u32::MAX {
                    let gid = uniq_key.len() as u32;
                    gslots[slot] = gid;
                    uniq_key.push(key);
                    uniq_cnt.push(0);
                    break gid;
                } else if uniq_key[s as usize] == key {
                    break s;
                }
                slot = (slot + 1) & gmask;
            };
            remap[lid] = gid;
            uniq_cnt[gid as usize] += b.cnt[lid];
        }
        remaps.push(remap);
    }

    // ---- 建树 ----
    // 只对每种全局唯一色调用一次 add_color，按其像素数加权，叶子平均色即像素加权质心。
    let mut tree = Octree::new(max_colors);
    for (id, &key) in uniq_key.iter().enumerate() {
        tree.add_color(
            (key >> 16) as u8,
            (key >> 8) as u8,
            key as u8,
            uniq_cnt[id] as u64,
        );
    }
    let palette = tree.build_palette();

    // ---- 映射 ----
    // 每种全局唯一色求一次调色板下标，再按条带并行回填每像素：
    // 条带内 local id --remap--> 全局 id --uniq_pidx--> 调色板下标（顺序访问、查小表，缓存友好）。
    let uniq_pidx: Vec<u8> = uniq_key
        .iter()
        .map(|&key| tree.index_of((key >> 16) as u8, (key >> 8) as u8, key as u8))
        .collect();
    let mut indices = vec![0u8; npix];
    if bands.len() == 1 {
        let b = &bands[0];
        let remap = &remaps[0];
        for (i, &lid) in b.ids.iter().enumerate() {
            indices[i] = uniq_pidx[remap[lid as usize] as usize];
        }
    } else {
        std::thread::scope(|scope| {
            let mut rest = indices.as_mut_slice();
            for (b, remap) in bands.iter().zip(remaps.iter()) {
                let (chunk, tail) = rest.split_at_mut(b.ids.len());
                rest = tail;
                let ids = &b.ids;
                let up = &uniq_pidx;
                scope.spawn(move || {
                    for (j, &lid) in ids.iter().enumerate() {
                        chunk[j] = up[remap[lid as usize] as usize];
                    }
                });
            }
        });
    }
    (palette, indices)
}
