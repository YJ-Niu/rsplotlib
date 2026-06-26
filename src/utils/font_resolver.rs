use pyo3::prelude::*;
use std::path::Path;
use std::sync::OnceLock;

/// 字体族名 → 候选文件路径映射（OnceLock 缓存，仅初始化一次）
fn font_name_to_paths() -> &'static Vec<(&'static str, Vec<&'static str>)> {
    static MAPPINGS: OnceLock<Vec<(&'static str, Vec<&'static str>)>> = OnceLock::new();
    MAPPINGS.get_or_init(|| {
        vec![
            // macOS
            #[cfg(target_os = "macos")]
            ("Arial Unicode MS", vec![
                "/Library/Fonts/Arial Unicode.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            ]),
            #[cfg(target_os = "macos")]
            ("Arial", vec![
                "/Library/Fonts/Arial.ttf",
                "/System/Library/Fonts/Supplemental/Arial.ttf",
            ]),
            #[cfg(target_os = "macos")]
            ("Helvetica", vec![
                "/System/Library/Fonts/Helvetica.ttc",
                "/System/Library/Fonts/HelveticaNeue.ttc",
            ]),
            #[cfg(target_os = "macos")]
            ("Helvetica Neue", vec![
                "/System/Library/Fonts/HelveticaNeue.ttc",
            ]),
            #[cfg(target_os = "macos")]
            ("PingFang SC", vec![
                "/System/Library/Fonts/PingFang.ttc",
            ]),
            #[cfg(target_os = "macos")]
            ("Heiti SC", vec![
                "/System/Library/Fonts/STHeiti Light.ttc",
                "/System/Library/Fonts/STHeiti Medium.ttc",
            ]),
            #[cfg(target_os = "macos")]
            ("Hiragino Sans GB", vec![
                "/System/Library/Fonts/Hiragino Sans GB W3.otf",
                "/System/Library/Fonts/Hiragino Sans GB W6.otf",
            ]),
            // Linux
            #[cfg(target_os = "linux")]
            ("DejaVu Sans", vec![
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/dejavu/DejaVuSans.ttf",
            ]),
            #[cfg(target_os = "linux")]
            ("Liberation Sans", vec![
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            ]),
            #[cfg(target_os = "linux")]
            ("Noto Sans CJK SC", vec![
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            ]),
            #[cfg(target_os = "linux")]
            ("WenQuanYi Micro Hei", vec![
                "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            ]),
            // Windows
            #[cfg(target_os = "windows")]
            ("Microsoft YaHei", vec![
                "C:/Windows/Fonts/msyh.ttc",
                "C:/Windows/Fonts/msyh.ttf",
                "C:/Windows/Fonts/msyhbd.ttc",
            ]),
            #[cfg(target_os = "windows")]
            ("SimHei", vec![
                "C:/Windows/Fonts/simhei.ttf",
            ]),
            #[cfg(target_os = "windows")]
            ("SimSun", vec![
                "C:/Windows/Fonts/simsun.ttc",
            ]),
        ]
    })
}

/// 按当前操作系统返回一组"通用全功能字体"候选路径（OnceLock 缓存）
fn system_fallback_paths() -> &'static Vec<&'static str> {
    static FALLBACK: OnceLock<Vec<&'static str>> = OnceLock::new();
    FALLBACK.get_or_init(|| {
        #[cfg(target_os = "macos")]
        {
            vec![
                "/Library/Fonts/Arial Unicode.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            ]
        }
        #[cfg(target_os = "linux")]
        {
            vec![
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/dejavu/DejaVuSans.ttf",
            ]
        }
        #[cfg(target_os = "windows")]
        {
            vec![
                "C:/Windows/Fonts/msyh.ttc",
                "C:/Windows/Fonts/msyh.ttf",
                "C:/Windows/Fonts/msyhbd.ttc",
            ]
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            vec![]
        }
    })
}

/// 根据字体族名查找本地的字体文件路径
///
/// 找不到时返回 None。
fn resolve_font_path_inner(family: &str) -> Option<String> {
    if family.is_empty() {
        return None;
    }

    let mappings = font_name_to_paths();

    // 精确匹配
    for (name, paths) in mappings {
        if *name == family {
            for path in paths {
                if Path::new(path).is_file() {
                    return Some(path.to_string());
                }
            }
        }
    }

    // 大小写不敏感匹配
    let family_lower = family.to_lowercase();
    for (name, paths) in mappings {
        if name.to_lowercase() == family_lower {
            for path in paths {
                if Path::new(path).is_file() {
                    return Some(path.to_string());
                }
            }
        }
    }

    // 平台回退
    for path in system_fallback_paths() {
        if Path::new(path).is_file() {
            return Some(path.to_string());
        }
    }

    None
}

/// 根据字体族名查找本地的字体文件路径（Python 可调用）
#[pyfunction]
pub fn resolve_font_path(family: String) -> Option<String> {
    resolve_font_path_inner(&family)
}

/// 读取 rcParams["font.sans-serif"]，把第一个能解析到本地文件的字体注册到 plotters
#[pyfunction]
pub fn apply_rcparams_font(py: Python) -> PyResult<Option<String>> {
    // 导入 pylab 模块获取 rcParams
    let pylab = match PyModule::import(py, "rsplotlib.pylab") {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };
    let mpl = match pylab.getattr("mpl") {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };
    let rcparams = match mpl.getattr("rcParams") {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let sans_serif = match rcparams.call_method1("get", ("font.sans-serif",)) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    // 获取候选字体列表
    let candidates: Vec<String> = if let Ok(s) = sans_serif.extract::<String>() {
        vec![s]
    } else if let Ok(list) = sans_serif.cast::<pyo3::types::PyList>() {
        let mut result = Vec::with_capacity(list.len());
        for item in list.iter() {
            if let Ok(s) = item.extract::<String>() {
                result.push(s);
            }
        }
        result
    } else {
        // 尝试转换为字符串
        match sans_serif.str() {
            Ok(s) => vec![s.to_string_lossy().to_string()],
            Err(_) => return Ok(None),
        }
    };

    // "sans-serif" 关键字跳过
    let candidates: Vec<&str> = candidates
        .iter()
        .map(|s| s.as_str())
        .filter(|c| !c.is_empty() && c.to_lowercase() != "sans-serif")
        .collect();

    if candidates.is_empty() {
        return Ok(None);
    }

    let rsplotlib = PyModule::import(py, "rsplotlib.rsplotlib")?;

    // 注册列表中 **所有** 能解析到文件的字体，但**反转顺序**注册。
    // fontdb 对同一 family/style 的多个字体，优先返回**最早注册**的那个。
    // 用户习惯把 CJK 字体放列表末尾（如 ["Helvetica", "Arial Unicode MS"]），
    // 反转后 Arial Unicode MS 先注册，fontdb 优先用它渲染（拉丁 + CJK 都能覆盖），
    // 避免中文显示为口。
    let mut last_registered: Option<String> = None;
    for family in candidates.iter().rev() {
        let path = resolve_font_path_inner(family);
        let path = match path {
            Some(p) => p,
            None => {
                // 可能是直接的字体文件路径
                if Path::new(family).is_file() {
                    family.to_string()
                } else {
                    continue;
                }
            }
        };

        if rsplotlib.call_method1("register_sans_serif_font", (path.clone(),)).is_ok() {
            last_registered = Some(path);
        }
    }

    Ok(last_registered)
}

/// 注册 font_resolver 模块中的函数
pub fn register(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(resolve_font_path, m)?)?;
    m.add_function(wrap_pyfunction!(apply_rcparams_font, m)?)?;
    Ok(())
}