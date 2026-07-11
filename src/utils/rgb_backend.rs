use plotters::backend::RGBPixel;
use plotters::prelude::BitMapBackend;
use plotters_backend::{
    BackendColor, BackendCoord, BackendStyle, DrawingBackend, DrawingErrorKind,
};
use std::cell::RefCell;
use std::rc::Rc;

/// 自定义后端错误类型。`DrawingBackend::ErrorType` 要求 `Error + Send + Sync`，
/// 用 String 承载临时 BitMapBackend 的错误文本即可满足。
#[derive(Debug)]
pub struct CanvasError(pub String);

impl std::fmt::Display for CanvasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canvas backend error: {}", self.0)
    }
}

impl std::error::Error for CanvasError {}

/// 把临时 BitMapBackend 的错误映射为 `CanvasError`：绘制错误取文本，字体错误原样透传。
fn map_err<E: std::error::Error + Send + Sync + 'static>(
    e: DrawingErrorKind<E>,
) -> DrawingErrorKind<CanvasError> {
    match e {
        DrawingErrorKind::DrawingError(e) => {
            DrawingErrorKind::DrawingError(CanvasError(e.to_string()))
        }
        DrawingErrorKind::FontError(e) => DrawingErrorKind::FontError(e),
    }
}

/// 拥有共享 RGB 缓冲（`Rc<RefCell<Vec<u8>>>`）的位图后端。
///
/// 位图渲染时它与热路径（`draw_thick_polyline_aa` / imshow）共享同一块缓冲：plotters
/// 常规绘制（坐标轴 / 文字 / 柱等）经此后端落盘，热路径则通过线程本地 canvas 借出同一
/// 缓冲直接 blit，绕过逐像素 `area.draw_pixel` 的坐标变换与 RefCell 借用开销。
///
/// 为与 `BitMapBackend<RGBPixel>` 保持字节级一致，所有绘制方法都在一个临时
/// `BitMapBackend` 上转发——即复用 plotters 完全相同的光栅化 / alpha 混合代码；且只
/// override `BitMapBackend` 所 override 的那组基础方法（get_size / ensure_prepared /
/// present / draw_pixel / draw_line / draw_rect / blit_bitmap），其余复合方法沿用
/// trait 默认实现，与 `BitMapBackend` 逐字节相同。
pub struct RgbBufferBackend {
    buf: Rc<RefCell<Vec<u8>>>,
    size: (u32, u32),
}

impl RgbBufferBackend {
    pub fn new(buf: Rc<RefCell<Vec<u8>>>, size: (u32, u32)) -> Self {
        Self { buf, size }
    }
}

impl DrawingBackend for RgbBufferBackend {
    type ErrorType = CanvasError;

    fn get_size(&self) -> (u32, u32) {
        self.size
    }

    fn ensure_prepared(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn present(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn draw_pixel(
        &mut self,
        point: BackendCoord,
        color: BackendColor,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut buf = self.buf.borrow_mut();
        let mut bm: BitMapBackend<'_, RGBPixel> =
            BitMapBackend::with_buffer(buf.as_mut_slice(), self.size);
        bm.draw_pixel(point, color).map_err(map_err)
    }

    fn draw_line<S: BackendStyle>(
        &mut self,
        from: BackendCoord,
        to: BackendCoord,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut buf = self.buf.borrow_mut();
        let mut bm: BitMapBackend<'_, RGBPixel> =
            BitMapBackend::with_buffer(buf.as_mut_slice(), self.size);
        bm.draw_line(from, to, style).map_err(map_err)
    }

    fn draw_rect<S: BackendStyle>(
        &mut self,
        upper_left: BackendCoord,
        bottom_right: BackendCoord,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut buf = self.buf.borrow_mut();
        let mut bm: BitMapBackend<'_, RGBPixel> =
            BitMapBackend::with_buffer(buf.as_mut_slice(), self.size);
        bm.draw_rect(upper_left, bottom_right, style, fill)
            .map_err(map_err)
    }

    fn blit_bitmap(
        &mut self,
        pos: BackendCoord,
        dim: (u32, u32),
        src: &[u8],
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut buf = self.buf.borrow_mut();
        let mut bm: BitMapBackend<'_, RGBPixel> =
            BitMapBackend::with_buffer(buf.as_mut_slice(), self.size);
        bm.blit_bitmap(pos, dim, src).map_err(map_err)
    }
}

thread_local! {
    /// 位图渲染期间共享的目标缓冲 + 尺寸 (w, h)。热路径据此直接写像素；非位图渲染
    /// （SVG / show 直落文件）时为 None，热路径回退到 `area.draw_pixel`。
    static RGB_CANVAS: RefCell<Option<(Rc<RefCell<Vec<u8>>>, u32, u32)>> =
        const { RefCell::new(None) };
}

/// 渲染前登记线程本地 canvas（`render_downsampled_rgb` 调用）。
pub fn set_canvas(buf: Rc<RefCell<Vec<u8>>>, w: u32, h: u32) {
    RGB_CANVAS.with(|c| *c.borrow_mut() = Some((buf, w, h)));
}

/// 渲染结束后清空线程本地 canvas（务必在取回缓冲前调用，以释放这一份 Rc 引用）。
pub fn clear_canvas() {
    RGB_CANVAS.with(|c| *c.borrow_mut() = None);
}

/// 取回当前线程本地 canvas 的 (共享缓冲, w, h) 克隆；未设置时返回 None。
pub fn canvas() -> Option<(Rc<RefCell<Vec<u8>>>, u32, u32)> {
    RGB_CANVAS.with(|c| c.borrow().clone())
}

/// 单通道整数 alpha 混合，逐位复制 plotters `bitmap_pixel::pixel_format::blend`：
/// `a` 为 `(alpha*256).floor()`，`new>prev` 时加权上调、否则下调，除法向零取整。
#[inline(always)]
fn blend_channel(prev: &mut u8, new: u8, a: u64) {
    if new > *prev {
        *prev += ((u64::from(new - *prev) * a) / 256) as u8;
    } else {
        *prev -= ((u64::from(*prev - new) * a) / 256) as u8;
    }
}

/// 直接向 RGB 缓冲写单像素，语义与 `BitMapBackend<RGBPixel>::draw_pixel` 完全一致：
/// 越界忽略；`alpha >= 1 - 1/256` 直接覆盖；`alpha <= 0` 跳过；否则按整数 alpha 混合
/// （`a = (alpha*256).floor()`）。因此走此路径与经 `area.draw_pixel` 逐字节相同。
#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn put_rgb_pixel(
    buf: &mut [u8],
    w: u32,
    h: u32,
    x: i32,
    y: i32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f64,
) {
    if x < 0 || y < 0 || x as u32 >= w || y as u32 >= h {
        return;
    }
    let base = (y as usize * w as usize + x as usize) * 3;
    if base + 2 >= buf.len() {
        return;
    }
    if alpha >= 1.0 - 1.0 / 256.0 {
        buf[base] = r;
        buf[base + 1] = g;
        buf[base + 2] = b;
    } else if alpha > 0.0 {
        let a = (alpha * 256.0).floor() as u64;
        blend_channel(&mut buf[base], r, a);
        blend_channel(&mut buf[base + 1], g, a);
        blend_channel(&mut buf[base + 2], b, a);
    }
    // alpha <= 0：完全透明，不写入（与 plotters draw_pixel 一致）。
}
