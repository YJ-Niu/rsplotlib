pub mod axes;
pub mod axes_bounds;
pub mod axes_grid;
pub mod axes_legend;
pub mod axes_mesh;
pub mod axes_render_elements;
pub mod axes_title;
pub mod axis;
pub mod colormap;
pub mod colors;
pub mod elements;
pub mod figure;
pub mod marker;
pub mod pyfuncs;
pub mod text_utils;

use pyo3::prelude::*;
use plotters::style::register_font;

use crate::axis::{Axis, Patch, SpineDict, Spine};
use crate::axes::Axes;
use crate::figure::Figure;

#[pymodule]
fn rsplotlib(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
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
            if let Ok(font_data) = std::fs::read(path) {
                let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
                if register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref).is_ok() {
                    registered = true;
                    break; // 找到第一个能用的就停，保证只用单一字体
                }
            }
        }
        if !registered {
            // 退回 matplotlib 自带 DejaVu Sans（仅英文可用）
            for base in &[
                std::env::var("VIRTUAL_ENV").ok(),
                Some(std::env::current_dir()
                    .map(|p| p.join(".venv").to_string_lossy().to_string())
                    .unwrap_or_default())
                    .filter(|p| !p.is_empty()),
            ] {
                if let Some(prefix) = base {
                    let p = std::path::Path::new(&prefix)
                        .join("lib/python3.13/site-packages/matplotlib/mpl-data/fonts/ttf/DejaVuSans.ttf");
                    if let Ok(font_data) = std::fs::read(&p) {
                        let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
                        let _ = register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref);
                        break;
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: 优先注册 Noto Sans CJK（多数发行版自带，覆盖 CJK + 拉丁）
        let font_candidates: Vec<String> = vec![
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string(),
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc".to_string(),
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc".to_string(),
        ];
        let mut registered = false;
        for path in &font_candidates {
            if let Ok(font_data) = std::fs::read(path) {
                let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
                if register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref).is_ok() {
                    registered = true;
                    break;
                }
            }
        }
        if !registered {
            // 退回 DejaVu Sans
            for base in &[
                std::env::var("VIRTUAL_ENV").ok(),
                Some(std::env::current_dir()
                    .map(|p| p.join(".venv").to_string_lossy().to_string())
                    .unwrap_or_default())
                    .filter(|p| !p.is_empty()),
            ] {
                if let Some(prefix) = base {
                    let p = std::path::Path::new(&prefix)
                        .join("lib/python3.13/site-packages/matplotlib/mpl-data/fonts/ttf/DejaVuSans.ttf");
                    if let Ok(font_data) = std::fs::read(&p) {
                        let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
                        let _ = register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref);
                        break;
                    }
                }
            }
        }
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
            if let Ok(font_data) = std::fs::read(path) {
                let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
                if register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref).is_ok() {
                    registered = true;
                    break;
                }
            }
        }
        if !registered {
            // 退回 Arial
            let p = "C:/Windows/Fonts/arial.ttf".to_string();
            if let Ok(font_data) = std::fs::read(&p) {
                let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
                let _ = register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref);
            }
        }
    }

    m.add_class::<Figure>()?;
    m.add_class::<Axes>()?;
    m.add_class::<Axis>()?;
    m.add_class::<Patch>()?;
    m.add_class::<SpineDict>()?;
    m.add_class::<Spine>()?;
    m.add_function(wrap_pyfunction!(pyfuncs::subplots, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::subplot, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::plot, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::savefig, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::show, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::figure, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::semilogx, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::semilogy, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::loglog, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::gca, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::clf, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::barh, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::xlabel, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::ylabel, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::title, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::grid, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::legend, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::xlim, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::ylim, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::scatter, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::bar, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::hist, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::fill_between, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::errorbar, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::stem, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::step, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::imshow, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::pie, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::boxplot, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::text, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::axhline, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::axvline, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::xticks, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::yticks, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::cla, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::close, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::twinx, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::twiny, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::tight_layout, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::set_size, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::set_dpi, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::use_, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::gcf, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::xscale, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::yscale, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::margins, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::box_, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::minorticks_on, m)?)?;
    m.add_function(wrap_pyfunction!(pyfuncs::minorticks_off, m)?)?;
    m.setattr("__version__", "0.1.3")?;
    Ok(())
}