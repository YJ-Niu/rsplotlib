pub mod core;
pub mod figure;
pub mod layout;
pub mod ticks;
pub mod utils;

use plotters::style::register_font;
use pyo3::prelude::*;

use crate::figure::axes::{Axes, Line2D};
use crate::figure::axis::{Axis, Patch, Spine, SpineDict};
use crate::figure::figure::Figure;

/// 把一份字体二进制注册为 plotters 的 "sans-serif" family，
/// 并同时记录其 face 供降级路径的 glyph 覆盖查询（见 `font_stack::char_supported`）。
/// 返回是否注册成功。
fn install_default_sans(font_data: Vec<u8>) -> bool {
    let face_copy = font_data.clone();
    let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
    if register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref).is_ok() {
        crate::utils::glyph_cache::register_ab_glyph(
            "sans-serif",
            plotters::style::FontStyle::Normal,
            font_ref,
        );
        crate::utils::font_stack::set_default_face(face_copy);
        true
    } else {
        false
    }
}

/// 注册数学字母回退字体（覆盖 SMP「Mathematical Alphanumeric Symbols」块），
/// 供 `\mathcal`/`\mathbb`/`\mathfrak`/`\mathsf`/`\mathtt` 等花体/黑板体字母渲染。
///
/// 用字体自身的家族名注册到 plotters，并记录到 `font_stack::MATH_FACE` 作为**最后
/// 回退**——仅当默认 sans（如 Arial Unicode MS，只覆盖 BMP）无法覆盖含 SMP 数学
/// 字母的文本时才被选用，不影响普通文本的字体选择。
/// 依次尝试候选路径，注册第一个可读取且能解析出家族名的字体。
fn install_math_fallback(candidates: &[&str]) -> bool {
    for path in candidates {
        let Ok(font_data) = std::fs::read(path) else {
            continue;
        };
        let Some(family) = crate::utils::font_stack::extract_family_name(&font_data) else {
            continue;
        };
        let face_copy = font_data.clone();
        let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
        // family 名需在两侧一致：plotters 绘制时按此名查表，选择器也返回此名。
        let leaked_family: &'static str = Box::leak(family.clone().into_boxed_str());
        if register_font(leaked_family, plotters::style::FontStyle::Normal, font_ref).is_ok() {
            crate::utils::glyph_cache::register_ab_glyph(
                leaked_family,
                plotters::style::FontStyle::Normal,
                font_ref,
            );
            crate::utils::font_stack::set_math_face(family, face_copy);
            return true;
        }
    }
    false
}

/// 以给定的通用族名（如 "monospace"/"serif"）注册系统/matplotlib 字体到 plotters。
///
/// matplotlib 的 `family="monospace"`、`"serif"` 等是**通用族关键字**；plotters 必须
/// 先以该名 `register_font` 才能绘制它，否则渲染时抛 `FontUnavailable`。依次尝试候选
/// 路径，注册第一个可读取的字体，并将其 face 记入 `font_stack::register_named_family`
/// 供选择/覆盖查询。返回是否成功。
fn install_named_font(family: &str, candidates: &[&str]) -> bool {
    for path in candidates {
        let Ok(font_data) = std::fs::read(path) else {
            continue;
        };
        let face_copy = font_data.clone();
        let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
        if register_font(family, plotters::style::FontStyle::Normal, font_ref).is_ok() {
            crate::utils::glyph_cache::register_ab_glyph(
                family,
                plotters::style::FontStyle::Normal,
                font_ref,
            );
            crate::utils::font_stack::register_named_family(family, face_copy);
            return true;
        }
    }
    false
}

/// 返回虚拟环境中 matplotlib 自带字体的候选路径（用于与 matplotlib 外观一致的
/// monospace/serif 回退）。`file` 形如 "DejaVuSansMono.ttf"。
fn mpl_font_paths(file: &str) -> Vec<String> {
    let mut out = Vec::new();
    for prefix in [
        std::env::var("VIRTUAL_ENV").ok(),
        Some(
            std::env::current_dir()
                .map(|p| p.join(".venv").to_string_lossy().to_string())
                .unwrap_or_default(),
        )
        .filter(|p| !p.is_empty()),
    ]
    .iter()
    .flatten()
    {
        out.push(
            std::path::Path::new(prefix)
                .join("lib/python3.13/site-packages/matplotlib/mpl-data/fonts/ttf")
                .join(file)
                .to_string_lossy()
                .to_string(),
        );
    }
    out
}

/// 注册 matplotlib 通用族 monospace / serif。
///
/// 优先使用 matplotlib 自带的 DejaVu Sans Mono / DejaVu Serif（与 matplotlib 默认
/// 外观一致），再回退到平台自带字体。仅接受单一字体的 TTF/OTF（ab_glyph 无法解析
/// TTC 集合，故避免 .ttc）。
fn install_generic_families(platform_mono: &[&str], platform_serif: &[&str]) {
    let mono_mpl = mpl_font_paths("DejaVuSansMono.ttf");
    let mut mono: Vec<&str> = mono_mpl.iter().map(|s| s.as_str()).collect();
    mono.extend_from_slice(platform_mono);
    install_named_font("monospace", &mono);

    let serif_mpl = mpl_font_paths("DejaVuSerif.ttf");
    let mut serif: Vec<&str> = serif_mpl.iter().map(|s| s.as_str()).collect();
    serif.extend_from_slice(platform_serif);
    install_named_font("serif", &serif);
}

#[pymodule]
fn rsplotlib(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // 字体注册策略说明：
    //
    // 用户反馈"字符宽度/字距视觉不一致 + 空格宽度窄"。
    //
    // 根因：plotters 内部用 `fontdb` 管理字体。若同一 family 名下注册了多个字体
    // （例如 "sans-serif" 既注册了 Arial 又注册了 Arial Unicode），fontdb 在
    // 排版一行文本时，可能：
    //   - 拉丁字符用 Arial 渲染
    //   - 中文用 Arial Unicode 渲染
    //   - 空格 / 标点 / 数字 又可能用回某个字体
    // 不同字体的 advance width（字宽）不同，混排后视觉上就是"间距忽大忽小、
    // 空格比预期的窄"。
    //
    // 修复：仅注册 **一个** 覆盖范围最广的字体到 "sans-serif" family，
    // 整行文本由同一个字形集排版，字符宽度/空格宽度都来自同一字体，视觉一致。
    // 优先选择 Arial Unicode MS（macOS 自带，几乎覆盖所有 Unicode 字符，
    // 中英文字形与 Arial 同一家族，混排视觉自然）。
    //
    // 同时在渲染端（axes_render_elements / axes_title / axes_legend）
    // 给所有文字调用传入 `transform`, 修正 plotters 默认 baseline 偏移。

    #[cfg(target_os = "macos")]
    {
        // macOS 优先 Arial Unicode（含 CJK + 拉丁 + 空格，宽度继承自 Arial，
        // 与 matplotlib 默认字体外观接近）；若不存在再退回 DejaVu Sans
        // （仅拉丁可读，CJK 会变方块，但至少保证拉丁字符宽度一致）。
        let font_candidates: Vec<String> = vec![
            "/Library/Fonts/Arial Unicode.ttf".to_string(),
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf".to_string(),
        ];
        let mut registered = false;
        for path in &font_candidates {
            if let Ok(font_data) = std::fs::read(path)
                && install_default_sans(font_data)
            {
                registered = true;
                break; // 找到第一个能用的就停，保证只用单一字体
            }
        }
        if !registered {
            // 退回 matplotlib 自带 DejaVu Sans（仅英文可用）
            for prefix in [
                std::env::var("VIRTUAL_ENV").ok(),
                Some(
                    std::env::current_dir()
                        .map(|p| p.join(".venv").to_string_lossy().to_string())
                        .unwrap_or_default(),
                )
                .filter(|p| !p.is_empty()),
            ]
            .iter()
            .flatten()
            {
                let p = std::path::Path::new(&prefix).join(
                    "lib/python3.13/site-packages/matplotlib/mpl-data/fonts/ttf/DejaVuSans.ttf",
                );
                if let Ok(font_data) = std::fs::read(&p) {
                    install_default_sans(font_data);
                    break;
                }
            }
        }
        // 数学字母回退：Arial Unicode MS 只覆盖 BMP，缺 SMP 数学字母块；挂 STIX 让
        // \mathcal/\mathbb 等花体/黑板体字母能完整渲染 26 个字母。
        install_math_fallback(&[
            "/System/Library/Fonts/Supplemental/STIXTwoMath.otf",
            "/System/Library/Fonts/Supplemental/STIXGeneral.otf",
            "/Library/Fonts/STIXTwoMath.otf",
        ]);
        // 通用族 monospace / serif：优先 matplotlib 自带 DejaVu，再回退系统 TTF
        // （避免 .ttc 集合——ab_glyph 不支持）。
        install_generic_families(
            &[
                "/System/Library/Fonts/Supplemental/Andale Mono.ttf",
                "/System/Library/Fonts/Supplemental/Courier New.ttf",
            ],
            &[
                "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
                "/System/Library/Fonts/Supplemental/Georgia.ttf",
            ],
        );
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: 优先注册 Noto Sans CJK（多数发行版自带，覆盖 CJK + 拉丁），
        // 若系统未安装 Noto，则退回到常见的 DejaVu Sans 或 Liberation Sans。
        let font_candidates: Vec<String> = vec![
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string(),
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc".to_string(),
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc".to_string(),
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string(),
            "/usr/share/fonts/dejavu/DejaVuSans.ttf".to_string(),
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf".to_string(),
        ];
        let mut registered = false;
        for path in &font_candidates {
            if let Ok(font_data) = std::fs::read(path)
                && install_default_sans(font_data)
            {
                registered = true;
                break;
            }
        }
        if !registered {
            // 退回 DejaVu Sans (可能来自虚拟环境中的 matplotlib 数据)
            for prefix in [
                std::env::var("VIRTUAL_ENV").ok(),
                Some(
                    std::env::current_dir()
                        .map(|p| p.join(".venv").to_string_lossy().to_string())
                        .unwrap_or_default(),
                )
                .filter(|p| !p.is_empty()),
            ]
            .iter()
            .flatten()
            {
                let p = std::path::Path::new(&prefix).join(
                    "lib/python3.13/site-packages/matplotlib/mpl-data/fonts/ttf/DejaVuSans.ttf",
                );
                if let Ok(font_data) = std::fs::read(&p) {
                    install_default_sans(font_data);
                    break;
                }
            }
        }
        // 数学字母回退：STIX（多数发行版随 matplotlib/texlive 提供）覆盖 SMP 数学字母块。
        install_math_fallback(&[
            "/usr/share/fonts/truetype/stix-word/STIXMath-Regular.otf",
            "/usr/share/fonts/opentype/stix/STIXTwoMath-Regular.otf",
            "/usr/share/fonts/stix/STIXTwoMath-Regular.otf",
            "/usr/share/fonts/OTF/STIXTwoMath-Regular.otf",
        ]);
        // 通用族 monospace / serif：优先 matplotlib 自带 DejaVu，再回退系统 TTF。
        install_generic_families(
            &[
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
                "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
            ],
            &[
                "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
                "/usr/share/fonts/dejavu/DejaVuSerif.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
                "/usr/share/fonts/liberation/LiberationSerif-Regular.ttf",
            ],
        );
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: 优先 Microsoft YaHei（自带，CJK + 拉丁）
        let font_candidates: Vec<String> = vec![
            "C:/Windows/Fonts/msyh.ttc".to_string(),
            "C:/Windows/Fonts/msyh.ttf".to_string(),
        ];
        let mut registered = false;
        for path in &font_candidates {
            if let Ok(font_data) = std::fs::read(path)
                && install_default_sans(font_data)
            {
                registered = true;
                break;
            }
        }
        if !registered {
            // 退回 Arial
            let p = "C:/Windows/Fonts/arial.ttf".to_string();
            if let Ok(font_data) = std::fs::read(&p) {
                install_default_sans(font_data);
            }
        }
        // 数学字母回退：Cambria Math（Windows 自带）覆盖 SMP 数学字母块。
        install_math_fallback(&[
            "C:/Windows/Fonts/cambria.ttc",
            "C:/Windows/Fonts/STIXTwoMath-Regular.otf",
        ]);
        // 通用族 monospace / serif：优先 matplotlib 自带 DejaVu，再回退系统 TTF。
        install_generic_families(
            &["C:/Windows/Fonts/consola.ttf", "C:/Windows/Fonts/cour.ttf"],
            &["C:/Windows/Fonts/times.ttf", "C:/Windows/Fonts/georgia.ttf"],
        );
    }

    m.add_class::<Figure>()?;
    m.add_class::<Axes>()?;
    m.add_class::<Line2D>()?;
    m.add_class::<Axis>()?;
    m.add_class::<Patch>()?;
    m.add_class::<SpineDict>()?;
    m.add_class::<Spine>()?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::subplots, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::subplot, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::plot, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::savefig, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::show, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::figure, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::semilogx, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::semilogy, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::loglog, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::gca, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::clf, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::barh, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::xlabel, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::ylabel, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::title, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::grid, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::legend, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::xlim, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::ylim, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::scatter, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::scatter_multi, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::colormap_hex, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::bar, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::hist, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::fill_between, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::stackplot, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::errorbar, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::stem, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::step, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::imshow, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::imsave, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::imread, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::pie, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::boxplot, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::text, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::axhline, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::axvline, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::hlines, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::vlines, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::xticks, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::yticks, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::cla, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::close, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::twinx, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::twiny, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::tight_layout, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::set_size, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::set_dpi, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::use_, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::gcf, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::xscale, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::yscale, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::margins, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::box_, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::minorticks_on, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::minorticks_off, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::axhspan, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::axvspan, m)?)?;
    m.add_function(wrap_pyfunction!(utils::pyfuncs::axline, m)?)?;
    m.add_function(wrap_pyfunction!(
        utils::pyfuncs::register_sans_serif_font,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(utils::font_stack::clear_font_stack, m)?)?;
    m.add_function(wrap_pyfunction!(utils::font_stack::glyph_supported, m)?)?;
    m.add_function(wrap_pyfunction!(utils::font_stack::debug_font_stack, m)?)?;
    m.add_function(wrap_pyfunction!(utils::font_stack::debug_select_family, m)?)?;
    m.add_function(wrap_pyfunction!(figure::figure::get_default_figsize, m)?)?;
    m.add_function(wrap_pyfunction!(figure::figure::get_default_dpi, m)?)?;

    ticks::ticker::register(py, m)?;
    layout::gridspec::register(py, m)?;
    utils::style::register(py, m)?;
    utils::font_resolver::register(py, m)?;
    Ok(())
}
