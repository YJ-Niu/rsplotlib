//! 文本预处理工具
//!
//! 解决 plotters 文本渲染中"字符宽度/字距视觉不一致 + 空格过窄"的问题。
//!
//! ## 根因
//!
//! plotters 通过 ab_glyph 渲染字符：
//!   1. `font.h_advance(glyph_id)` 决定每个字形的横向推进量（即"字宽"）。
//!   2. `font.kern(prev, curr)` 决定两个相邻字形之间的字距调整。
//!
//! 对于 Arial Unicode 来说：
//!   - ASCII 空格 (U+0020) 的 h_advance ≈ 0.278 em（窄）
//!   - 拉丁字母 h_advance ≈ 0.5-0.6 em（中）
//!   - CJK 字符 h_advance ≈ 1.0 em（宽）
//!   - 拉丁-拉丁对有 kerning（如 "AV" 收紧），但 CJK-CJK / CJK-拉丁 kerning 几乎为 0
//!
//! 当一行中混合 CJK + 拉丁 + 空格时，肉眼很容易感到"字距忽大忽小、空格特别窄"。
//!
//! ## 修复策略
//!
//! 不动 plotters 内部（ab_glyph 不支持禁用 kerning），而是在 **送入 plotters 之前**
//! 对文本做一次归一化预处理：
//!
//!   1. 用更宽的 Unicode 空格替换 ASCII 空格：
//!      - 文本中含 CJK 字符 → 替换为全角空格 (U+3000 IDEOGRAPHIC SPACE，1 em)
//!      - 文本中无 CJK 字符 → 替换为 en 空格 (U+2002 EN SPACE，0.5 em)
//!      - 这样做可让空格与 CJK / 拉丁字符的视觉间距更协调
//!   2. 连续 ASCII 空格统一为更宽的空格 + 占位符，避免 plotters 仍按原宽累加
//!
//! 注意：plotters 的 `font.draw` 在最后会调用 `font.h_advance(glyph_id)`，因此替换
//! 为更宽的字形后，**布局宽度（layout_box）也会变宽**，自然就让后续字符的 x 坐标
//! 推得更远，视觉上等价于"加了字距"。

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

/// 将文本中的 ASCII 空格替换为更宽的 Unicode 空格，使排版视觉更协调。
///
/// **上下文感知**：根据每个空格前后的"非空格"字符选择合适的 Unicode 空格：
///
/// | 前一个字符 | 后一个字符 | 替换为 | 宽度 |
/// |-----------|-----------|--------|------|
/// | CJK       | CJK       | U+3000 全角空格 | 1.0 em |
/// | CJK       | ASCII     | U+3000 全角空格 | 1.0 em |
/// | ASCII     | CJK       | U+3000 全角空格 | 1.0 em |
/// | ASCII     | ASCII     | U+2002 en 空格  | 0.5 em |
/// | 文本开头/结尾 | 任意     | U+2002 en 空格  | 0.5 em |
///
/// 这样 CJK 上下文用全角空格（与汉字等宽），纯拉丁上下文用 en 空格（约为
/// ASCII 空格的两倍），与 matplotlib / InDesign / Word 的"中英文混排空格"行为
/// 一致。
///
/// **连续空格不会被合并**——每个 ASCII 空格都按其上下文独立替换，
/// 保留用户输入的间距。
pub fn normalize_spaces(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 4);
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        let c = chars[i];
        if c == ' ' {
            // 找到当前空格前一个"非空格"字符（向前跳过多余空格）
            let prev = if i == 0 {
                None
            } else {
                let mut j = i as isize - 1;
                while j >= 0 && chars[j as usize] == ' ' {
                    j -= 1;
                }
                if j >= 0 { Some(chars[j as usize]) } else { None }
            };
            // 找到当前空格后一个"非空格"字符
            let next = {
                let mut j = i + 1;
                while j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                if j < chars.len() { Some(chars[j]) } else { None }
            };
            out.push(pick_space(prev, next));
        } else {
            out.push(c);
        }
    }
    out
}

/// 根据前后字符决定空格替换字符
fn pick_space(prev: Option<char>, next: Option<char>) -> char {
    match (prev.map(is_cjk).unwrap_or(false), next.map(is_cjk).unwrap_or(false)) {
        // 任意一侧是 CJK → 用全角空格（CJK 上下文用全角视觉协调）
        (true, _) | (_, true) => '\u{3000}',
        // 两侧都是非 CJK（含开头/结尾的 None）→ 用 en 空格
        (false, false) => '\u{2002}',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cjk_detection() {
        assert!(contains_cjk("请确认"));
        assert!(contains_cjk("あいうえお"));
        assert!(contains_cjk("アイウエオ"));
        assert!(contains_cjk("한글"));
        assert!(!contains_cjk("hello world"));
        assert!(!contains_cjk("ABC123"));
        assert!(!contains_cjk(""));
    }

    #[test]
    fn test_normalize_spaces_latin() {
        // 拉丁文本：空格 → en 空格
        let r = normalize_spaces("hello world");
        assert_eq!(r, "hello\u{2002}world");
    }

    #[test]
    fn test_normalize_spaces_cjk() {
        // CJK 文本：空格 → 全角空格
        let r = normalize_spaces("请确认 关键字");
        assert_eq!(r, "请确认\u{3000}关键字");
    }

    #[test]
    fn test_normalize_spaces_mixed() {
        // 混合：含 CJK 用全角空格
        let r = normalize_spaces("Plot 图表");
        assert_eq!(r, "Plot\u{3000}图表");
    }

    #[test]
    fn test_normalize_spaces_preserve() {
        // 连续空格不再合并——每个空格独立替换为 en 空格（拉丁上下文）
        let r = normalize_spaces("a    b");
        assert_eq!(r, "a\u{2002}\u{2002}\u{2002}\u{2002}b");
    }

    #[test]
    fn test_normalize_spaces_empty() {
        assert_eq!(normalize_spaces(""), "");
    }

    #[test]
    fn test_context_aware_cjk_to_cjk() {
        // CJK 上下文：使用全角空格
        assert_eq!(normalize_spaces("请 确认"), "请\u{3000}确认");
        assert_eq!(normalize_spaces("中 文"), "中\u{3000}文");
    }

    #[test]
    fn test_context_aware_latin_to_latin() {
        // 纯拉丁：使用 en 空格
        assert_eq!(normalize_spaces("hello world"), "hello\u{2002}world");
        assert_eq!(normalize_spaces("foo bar"), "foo\u{2002}bar");
    }

    #[test]
    fn test_context_aware_mixed_cjk_to_latin() {
        // CJK 到拉丁：用全角空格（CJK 一侧）
        assert_eq!(normalize_spaces("中文 abc"), "中文\u{3000}abc");
    }

    #[test]
    fn test_context_aware_mixed_latin_to_cjk() {
        // 拉丁到 CJK：用全角空格（CJK 一侧）
        assert_eq!(normalize_spaces("abc 中文"), "abc\u{3000}中文");
    }

    #[test]
    fn test_context_aware_leading_space() {
        // 文本开头的空格：next 决定
        assert_eq!(normalize_spaces(" hello"), "\u{2002}hello");
        assert_eq!(normalize_spaces(" 中文"), "\u{3000}中文");
    }

    #[test]
    fn test_context_aware_trailing_space() {
        // 文本结尾的空格：prev 决定
        assert_eq!(normalize_spaces("hello "), "hello\u{2002}");
        assert_eq!(normalize_spaces("中文 "), "中文\u{3000}");
    }

    #[test]
    fn test_context_aware_multiple_spaces_preserve() {
        // 多个连续空格不再合并——每个空格都按上下文独立替换
        // 前后都是拉丁字符 → 全部用 en 空格
        assert_eq!(normalize_spaces("a   b"), "a\u{2002}\u{2002}\u{2002}b");
        // 前后都是 CJK → 全部用全角空格
        assert_eq!(normalize_spaces("中   文"), "中\u{3000}\u{3000}\u{3000}文");
    }

    #[test]
    fn test_context_aware_japanese_kana() {
        // 日文假名也算 CJK
        assert_eq!(normalize_spaces("あい うえ"), "あい\u{3000}うえ");
    }

    #[test]
    fn test_context_aware_punctuation() {
        // 标点不是 CJK（不在我们定义的范围），跟拉丁相同
        // U+3000 是 CJK 标点（属于 CJK Symbols and Punctuation）
        // U+FF0C 全角逗号属于 Halfwidth and Fullwidth Forms（也属于 CJK）
        assert_eq!(normalize_spaces("hi, world"), "hi,\u{2002}world");
        // 全角标点算 CJK → 用全角空格
        assert_eq!(normalize_spaces("中文， 测试"), "中文，\u{3000}测试");
    }
}
