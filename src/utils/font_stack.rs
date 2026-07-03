use owned_ttf_parser::{AsFaceRef, OwnedFace};
use pyo3::prelude::*;
use std::sync::OnceLock;
/// 全局字体栈，支持多字体的 glyph 回退（fallback）。
///
/// # 原理
///
/// plotters 的 `register_font` 对同一 (family, style) 使用 HashMap insert，
/// 因此只能保存**一个**字体。为了支持 `font.sans-serif = ["Helvetica", "Arial Unicode MS"]`
/// 这样的配置，我们维护一个额外的字体栈。
///
/// 渲染时，对给定的文本，按注册顺序依次检查栈中的每个字体是否能覆盖该文本
/// 中的**所有**字符（通过 `ttf_parser` 查询 cmap 表），选择第一个能全覆盖的字体。
///
/// 这样：
/// - 纯拉丁文本（"ABC 123"）→ 使用第一个字体（Helvetica），尺寸正确。
/// - 含 CJK 的文本（"中文测试"、"中文 ABC"）→ 使用第二个字体（Arial Unicode MS），
///   该字体同时覆盖拉丁和 CJK 字符。
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 字体栈中的一条记录
struct FontEntry {
    family: String,
    face: OwnedFace,
}

static FONT_STACK: OnceLock<RwLock<Vec<FontEntry>>> = OnceLock::new();
/// 字体栈当前大小（用于快速判断是否非空，避免获取读锁）
static FONT_STACK_LEN: AtomicUsize = AtomicUsize::new(0);

fn stack() -> &'static RwLock<Vec<FontEntry>> {
    FONT_STACK.get_or_init(|| RwLock::new(Vec::new()))
}

/// 判断文本是否全为 ASCII 字符。
/// 纯 ASCII 文本可以走快速路径——直接使用第一个字体（西文字体肯定能覆盖 ASCII）。
#[inline]
fn is_ascii_text(text: &str) -> bool {
    text.is_ascii()
}

/// 从 TrueType/OpenType 字体二进制数据中提取字体家族名称。
///
/// 优先使用 Name ID 16（Typographic Family Name），
/// 回退到 Name ID 1（Font Family Name）。
/// 对于 TTC 字体集合，尝试每个子字体，返回第一个有名称的。
pub fn extract_family_name(data: &[u8]) -> Option<String> {
    for i in 0..6 {
        let face = match owned_ttf_parser::Face::parse(data, i) {
            Ok(f) => f,
            Err(_) => break,
        };
        for name in face.names() {
            if name.name_id == owned_ttf_parser::name_id::TYPOGRAPHIC_FAMILY
                && let Some(s) = name.to_string()
                && !s.is_empty()
            {
                return Some(s);
            }
        }
        for name in face.names() {
            if name.name_id == owned_ttf_parser::name_id::FAMILY
                && let Some(s) = name.to_string()
                && !s.is_empty()
            {
                return Some(s);
            }
        }
    }
    None
}

/// 检查某个字体是否包含指定文本中的**所有**字符。
/// 使用预解析的 OwnedFace，避免重复解析字体文件。
fn can_render_text(face: &OwnedFace, text: &str) -> bool {
    text.chars()
        .all(|c| face.as_face_ref().glyph_index(c).is_some())
}

/// 将一个字体添加到全局字体栈。
///
/// 实际调用方（`pyfuncs::register_sans_serif_font`）应保证：
/// 1. 同一个字体已通过 `plotters::style::register_font(family, ...)` 注册到 plotters。
pub fn push_font(family: String, data: Vec<u8>) {
    if let Ok(face) = OwnedFace::from_vec(data, 0) {
        let mut s = stack().write().unwrap();
        s.push(FontEntry { family, face });
        FONT_STACK_LEN.store(s.len(), Ordering::Relaxed);
    }
}

/// 清空字体栈（主要用于测试 / 重置）。
#[pyfunction]
pub fn clear_font_stack() {
    let mut s = stack().write().unwrap();
    s.clear();
    FONT_STACK_LEN.store(0, Ordering::Relaxed);
}

/// 从字体栈中选择最适合渲染指定文本的字体家族名称。
///
/// 遍历栈中所有字体，返回**第一个**能覆盖文本中所有字符的字体家族名。
/// 如果没有任何字体能完全覆盖，返回 `"sans-serif"` 作为降级。
pub fn select_family(text: &str) -> String {
    let len = FONT_STACK_LEN.load(Ordering::Relaxed);
    if len == 0 {
        return "sans-serif".to_string();
    }

    // 快速路径：纯 ASCII 文本直接使用第一个字体
    // （第一个字体通常是西文字体，必然能覆盖所有 ASCII 字符）
    if is_ascii_text(text) {
        let s = stack().read().unwrap();
        if let Some(first) = s.first() {
            return first.family.clone();
        }
        return "sans-serif".to_string();
    }

    // 慢路径：遍历所有字体，找第一个能覆盖全部字符的
    let s = stack().read().unwrap();
    for entry in s.iter() {
        if can_render_text(&entry.face, text) {
            return entry.family.clone();
        }
    }
    "sans-serif".to_string()
}

/// 根据文本选择最合适的字体家族名称。
///
/// 如果 `explicit_family` 有值且非空，直接返回它（用户显式指定的优先级最高）。
/// 否则调用 `select_family` 从字体栈中按 glyph 覆盖选择最佳匹配的字体。
///
/// 这是渲染端统一的"字体选择入口"，所有硬编码 "sans-serif" 的地方都应替换为
/// 此函数调用。
pub fn resolve_font_family(text: &str, explicit_family: Option<&str>) -> String {
    if let Some(family) = explicit_family
        && !family.is_empty()
        && family != "sans-serif"
    {
        return family.to_string();
    }
    select_family(text)
}

/// Python 可调用：返回当前字体栈中所有字体的家族名称列表（调试用）。
#[pyfunction]
pub fn debug_font_stack() -> Vec<String> {
    let s = stack().read().unwrap();
    s.iter().map(|e| e.family.clone()).collect()
}

/// Python 可调用：测试某段文本会选择哪个字体（调试用）。
#[pyfunction]
pub fn debug_select_family(text: String) -> String {
    select_family(&text)
}
