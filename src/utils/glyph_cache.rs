use ab_glyph::{Font, FontRef, ScaleFont};
use plotters::style::FontStyle;
use plotters_backend::{
    BackendColor, BackendCoord, BackendStyle, BackendTextStyle, DrawingBackend, DrawingErrorKind,
    FontTransform,
    text_anchor::{HPos, VPos},
};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// 镜像 plotters 私有字体表：family -> (style_str -> (font_id, FontRef))。
/// 与 6 处 `register_font` 一一配对写入，保证同一 (family,style) 指向同一字体。
struct FontMirror {
    by_name: HashMap<String, HashMap<String, (u32, FontRef<'static>)>>,
    next_id: u32,
}

static MIRROR: OnceLock<RwLock<FontMirror>> = OnceLock::new();

fn mirror() -> &'static RwLock<FontMirror> {
    MIRROR.get_or_init(|| {
        RwLock::new(FontMirror {
            by_name: HashMap::new(),
            next_id: 0,
        })
    })
}

/// 与 `plotters::style::register_font` 配对调用：用同一 `&'static [u8]` 建 ab_glyph
/// FontRef 记入镜像表。解析失败则跳过（该 family 走 inner.draw_text 回退，仍正确）。
pub fn register_ab_glyph(name: &str, style: FontStyle, bytes: &'static [u8]) {
    let Ok(font) = FontRef::try_from_slice(bytes) else {
        return;
    };
    let mut m = mirror().write().unwrap();
    let id = m.next_id;
    m.next_id += 1;
    m.by_name
        .entry(name.to_string())
        .or_default()
        .insert(style.as_str().to_string(), (id, font));
}

/// 查 family+style，缺 style 时回退 Normal（与 plotters get_fallback 一致）。
fn lookup(family: &str, style_key: &str) -> Option<(u32, FontRef<'static>)> {
    let m = mirror().read().unwrap();
    let fam = m.by_name.get(family)?;
    fam.get(style_key)
        .or_else(|| fam.get(FontStyle::Normal.as_str()))
        .cloned()
}

/// 单个字形的缓存：覆盖率像素（局部坐标, cov）+ px_bounds().min.y。
struct CachedGlyph {
    px: Vec<(i32, i32, f32)>,
    rect_min_y: f32,
}

/// 包裹任意后端；仅 draw_text 自绘并缓存字形，其余全部转发内层后端。
pub struct GlyphCacheBackend<B: DrawingBackend> {
    inner: B,
    cache: HashMap<(u32, char, u32), Option<CachedGlyph>>,
}

impl<B: DrawingBackend> GlyphCacheBackend<B> {
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            cache: HashMap::new(),
        }
    }
}

impl<B: DrawingBackend> DrawingBackend for GlyphCacheBackend<B> {
    type ErrorType = B::ErrorType;

    fn get_size(&self) -> (u32, u32) {
        self.inner.get_size()
    }
    fn ensure_prepared(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.ensure_prepared()
    }
    fn present(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.present()
    }
    fn draw_pixel(
        &mut self,
        point: BackendCoord,
        color: BackendColor,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.draw_pixel(point, color)
    }
    fn draw_line<S: BackendStyle>(
        &mut self,
        from: BackendCoord,
        to: BackendCoord,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.draw_line(from, to, style)
    }
    fn draw_rect<S: BackendStyle>(
        &mut self,
        upper_left: BackendCoord,
        bottom_right: BackendCoord,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.draw_rect(upper_left, bottom_right, style, fill)
    }
    fn draw_path<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        path: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.draw_path(path, style)
    }
    fn draw_circle<S: BackendStyle>(
        &mut self,
        center: BackendCoord,
        radius: u32,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.draw_circle(center, radius, style, fill)
    }
    fn fill_polygon<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        vert: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.fill_polygon(vert, style)
    }
    fn blit_bitmap(
        &mut self,
        pos: BackendCoord,
        dim: (u32, u32),
        src: &[u8],
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.inner.blit_bitmap(pos, dim, src)
    }
    fn estimate_text_size<TStyle: BackendTextStyle>(
        &self,
        text: &str,
        style: &TStyle,
    ) -> Result<(u32, u32), DrawingErrorKind<Self::ErrorType>> {
        self.inner.estimate_text_size(text, style)
    }

    fn draw_text<TStyle: BackendTextStyle>(
        &mut self,
        text: &str,
        style: &TStyle,
        pos: BackendCoord,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let color = style.color();
        if color.alpha == 0.0 {
            return Ok(());
        }
        // 仅处理无旋转文本；有旋转/其它变换交回默认实现，保证正确。
        if !matches!(style.transform(), FontTransform::None) {
            return self.inner.draw_text(text, style, pos);
        }
        // 镜像表缺该 family/style → 回退默认实现。
        let Some((font_id, font_ref)) = lookup(style.family().as_str(), style.style().as_str())
        else {
            return self.inner.draw_text(text, style, pos);
        };
        // layout_box 出错 → 回退默认实现（与默认对错误的处理一致地退让）。
        let Ok(layout) = style.layout_box(text) else {
            return self.inner.draw_text(text, style, pos);
        };
        let ((min_x, min_y), (max_x, max_y)) = layout;
        let width = max_x - min_x;
        let height = max_y - min_y;
        let dx = match style.anchor().h_pos {
            HPos::Left => 0,
            HPos::Right => -width,
            HPos::Center => -width / 2,
        };
        let dy = match style.anchor().v_pos {
            VPos::Top => 0,
            VPos::Center => -height / 2,
            VPos::Bottom => -height,
        };

        let size = style.size();
        let sbits = (size as f32).to_bits();
        let scaled = font_ref.as_scaled(size as f32);
        let (w, h) = self.inner.get_size();

        let mut x_shift = 0f32;
        let mut prev: Option<char> = None;
        for c in text.chars() {
            let gid = scaled.glyph_id(c);
            if let Some(pc) = prev {
                x_shift += scaled.kern(scaled.glyph_id(pc), gid);
            }
            prev = Some(c);

            let key = (font_id, c, sbits);
            let cached = self.cache.entry(key).or_insert_with(|| {
                let glyph = scaled.scaled_glyph(c);
                scaled.outline_glyph(glyph).map(|q| {
                    let rect = q.px_bounds();
                    let mut px = Vec::new();
                    q.draw(|x, y, cov| px.push((x as i32, y as i32, cov)));
                    CachedGlyph {
                        px,
                        rect_min_y: rect.min.y,
                    }
                })
            });

            if let Some(cg) = cached.as_ref() {
                let y_shift = ((size as f32) / 2.0 + cg.rect_min_y) as i32;
                let x_shift_i = x_shift as i32;
                for &(gx, gy, cov) in &cg.px {
                    let fx = pos.0 + gx + x_shift_i + dx - min_x;
                    let fy = pos.1 + gy + y_shift + dy - min_y;
                    if fx >= 0 && fx < w as i32 && fy >= 0 && fy < h as i32 {
                        self.inner.draw_pixel((fx, fy), color.mix(cov as f64))?;
                    }
                }
            }
            x_shift += scaled.h_advance(gid);
        }
        Ok(())
    }
}
