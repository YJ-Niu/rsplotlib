//! 文本预处理工具
//!
//! 解决 plotters 文本渲染中"空格过窄 + 特殊符号尺寸/间距不均"的问题。
//!
//! ## 根因
//!
//! plotters 通过 ab_glyph 渲染字符：
//!   1. `font.h_advance(glyph_id)` 决定每个字形的横向推进量（即"字宽"）。
//!   2. `font.kern(prev, curr)` 决定两个相邻字形之间的字距调整。
//!
//! 常见问题：
//!   - ASCII 空格 (U+0020) 太窄（约 0.278 em），视觉上间距不足
//!   - 特殊符号（|、-、#、* 等）左右边距不对称，且视觉上偏小
//!   - 拉丁字母之间有 kerning，导致某些字符对间距忽大忽小
//!
//! ## 修复策略
//!
//! 不动 plotters 内部（ab_glyph 不支持禁用 kerning），而是在 **送入 plotters 之前**
//! 对文本做一次预处理：
//!
//!   1. **空格加宽**：ASCII 空格 → EM 空格 (U+2003, 宽度 1em)
//!      - 这样空格宽度与典型字符宽度协调，视觉更舒适
//!
//!   2. **特殊符号间距优化**：在常用分隔符号（|、-、#、*、+、= 等）两侧
//!      添加细空格 (THIN SPACE, U+2009, 约 0.2em)
//!      - 使符号左右间距更均匀
//!      - 增加符号的视觉存在感（相当于"放大"了符号的视觉宽度）
//!      - 连续相同符号之间不添加（如 "---" 保持连续）
//!      - 符号与空格之间不重复添加
//!
//! **重要原则**：不做任何"字符替换"（如半角转全角），保持原字符不变，
//! 只通过添加空格来调整间距和视觉效果。

/// 判断字符是否为 CJK（中日韩）字符。
///
/// 覆盖范围：CJK 统一表意文字、CJK 统一表意文字扩展 A/B/C/D/E/F、
///          CJK 兼容表意文字、假名（平假名/片假名）、谚文、全角 ASCII。
fn is_cjk(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        // CJK Unified Ideographs
        0x4E00..=0x9FFF
        // CJK Unified Ideographs Extension A
        | 0x3400..=0x4DBF
        // CJK Unified Ideographs Extension B
        | 0x20000..=0x2A6DF
        // CJK Unified Ideographs Extension C
        | 0x2A700..=0x2B73F
        // CJK Unified Ideographs Extension D
        | 0x2B740..=0x2B81F
        // CJK Unified Ideographs Extension E
        | 0x2B820..=0x2CEAF
        // CJK Unified Ideographs Extension F
        | 0x2CEB0..=0x2EBEF
        // CJK Unified Ideographs Extension G
        | 0x30000..=0x3134F
        // CJK Compatibility Ideographs
        | 0xF900..=0xFAFF
        // Hiragana
        | 0x3040..=0x309F
        // Katakana
        | 0x30A0..=0x30FF
        // Katakana Phonetic Extensions
        | 0x31F0..=0x31FF
        // Hangul Syllables
        | 0xAC00..=0xD7AF
        // Hangul Jamo
        | 0x1100..=0x11FF
        // Hangul Compatibility Jamo
        | 0x3130..=0x318F
        // Hangul Jamo Extended A
        | 0xA960..=0xA97F
        // Hangul Jamo Extended B
        | 0xD7B0..=0xD7FF
        // CJK Symbols and Punctuation
        | 0x3000..=0x303F
        // CJK Strokes
        | 0x31C0..=0x31EF
        // Halfwidth and Fullwidth Forms
        | 0xFF00..=0xFFEF
    )
}

/// 判断文本是否含 CJK 字符。
pub fn contains_cjk(text: &str) -> bool {
    text.chars().any(is_cjk)
}

/// 判断字符是否为常用分隔符号（需要优化间距的符号）。
fn is_separator_symbol(c: char) -> bool {
    matches!(c, '|' | '-' | '#' | '+' | ':' | ';' | '(' | ')' | ' ')
}

/// 判断字符是否为空格类字符。
fn is_any_space(c: char) -> bool {
    match c as u32 {
        0x0020 |        // ASCII space
        0x00A0 |        // NBSP
        0x2000..=0x200B | // EN quad..zero width space
        0x202F |        // Narrow NBSP
        0x3000 => true, // Ideographic space
        _ => false,
    }
}

/// 在特殊符号两侧添加细空格，使符号左右间距更均匀，同时增加视觉存在感。
///
/// **规则**：
/// - 对常用分隔符号生效：| - # * + = : ; ( )
/// - 正常情况（符号与文字/数字相邻）：加 THIN SPACE (~0.2em)
/// - 连续相同符号之间：加 HAIR SPACE (~0.1em)，更细
/// - 符号旁边已有空格：加 HAIR SPACE (~0.1em)，更细
/// - 文本开头/结尾的符号外侧：加 HAIR SPACE (~0.1em)，更细
/// - 保持原符号不变，不做任何字符替换
pub fn adjust_symbol_spacing(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + text.len() / 4);
    let thin = '\u{2009}';  // THIN SPACE, ~0.2em （正常宽度）
    let hair = '\u{200A}';  // HAIR SPACE, ~0.1em （更细，用于例外情况）

    for i in 0..chars.len() {
        let c = chars[i];
        if !is_separator_symbol(c) {
            out.push(c);
            continue;
        }

        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = if i + 1 < chars.len() { Some(chars[i + 1]) } else { None };

        // 左侧：例外情况（开头、空格、相同符号）加 hair，否则加 thin
        let left_space = match prev {
            None => hair,
            Some(p) => if is_any_space(p) || p == c { hair } else { thin },
        };

        // 右侧：例外情况（结尾、空格、相同符号）加 hair，否则加 thin
        let right_space = match next {
            None => hair,
            Some(n) => if is_any_space(n) || n == c { hair } else { thin },
        };

        out.push(left_space);
        out.push(c);
        out.push(right_space);
    }

    out
}

/// 将文本中的 ASCII 空格替换为 EM 空格（U+2003, 宽度 1em）。
///
/// ASCII 空格通常只有约 0.278em 宽，视觉上太窄。
/// EM 空格宽度为 1em，与典型字符宽度协调。
pub fn normalize_spaces(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c == ' ' {
            out.push('\u{2003}');  // EM SPACE
        } else {
            out.push(c);
        }
    }
    out
}

/// 统一的文本预处理入口：
///   1. 先调整特殊符号间距（添加细空格）
///   2. 再将 ASCII 空格替换为 EM 空格
pub fn normalize_text(text: &str) -> String {
    let after_symbols = adjust_symbol_spacing(text);
    normalize_spaces(&after_symbols)
}
