use pyo3::prelude::*;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use owned_ttf_parser::{Face, name_id};

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
            #[cfg(target_os = "macos")]
            ("Courier", vec![
                "/System/Library/Fonts/Supplemental/Courier New.ttf",
                "/System/Library/Fonts/Courier.ttc",
            ]),
            #[cfg(target_os = "macos")]
            ("Courier New", vec![
                "/System/Library/Fonts/Supplemental/Courier New.ttf",
            ]),
            #[cfg(target_os = "macos")]
            ("Menlo", vec![
                "/System/Library/Fonts/Menlo.ttc",
            ]),
            #[cfg(target_os = "macos")]
            ("Monaco", vec![
                "/System/Library/Fonts/Monaco.ttf",
            ]),
            #[cfg(target_os = "macos")]
            ("Times New Roman", vec![
                "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
            ]),
            #[cfg(target_os = "macos")]
            ("Georgia", vec![
                "/System/Library/Fonts/Supplemental/Georgia.ttf",
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

/// 当前操作系统上存放字体的目录列表（含用户字体目录）。
fn system_font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    #[cfg(target_os = "macos")]
    {
        dirs.push(Path::new("/System/Library/Fonts").to_path_buf());
        dirs.push(Path::new("/System/Library/Fonts/Supplemental").to_path_buf());
        dirs.push(Path::new("/Library/Fonts").to_path_buf());
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(Path::new(&home).join("Library/Fonts"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        dirs.push(Path::new("/usr/share/fonts").to_path_buf());
        dirs.push(Path::new("/usr/local/share/fonts").to_path_buf());
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(Path::new(&home).join(".fonts"));
            dirs.push(Path::new(&home).join(".local/share/fonts"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        dirs.push(Path::new("C:/Windows/Fonts").to_path_buf());
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(Path::new(&local).join("Microsoft/Windows/Fonts"));
        }
    }
    dirs
}

/// 从字体二进制数据中收集所有可用于匹配的名称：
/// 家族名(1)、全名(4)、PostScript 名(6)、排版家族名(16)，
/// 以及 "家族 + 子family" 组合（覆盖 "Arial Bold" 这类请求）。
/// TTC 字体集合会遍历其中每个子字体。
fn collect_font_names(data: &[u8]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for i in 0..32u32 {
        let face = match Face::parse(data, i) {
            Ok(f) => f,
            Err(_) => break,
        };
        let mut family: Option<String> = None;
        let mut typo_family: Option<String> = None;
        let mut subfamily: Option<String> = None;
        let mut typo_subfamily: Option<String> = None;
        for name in face.names() {
            let s = match name.to_string() {
                Some(s) if !s.is_empty() => s,
                _ => continue,
            };
            match name.name_id {
                name_id::FAMILY => {
                    if family.is_none() {
                        family = Some(s.clone());
                    }
                    names.push(s);
                }
                name_id::FULL_NAME | name_id::POST_SCRIPT_NAME => names.push(s),
                name_id::TYPOGRAPHIC_FAMILY => {
                    if typo_family.is_none() {
                        typo_family = Some(s.clone());
                    }
                    names.push(s);
                }
                name_id::SUBFAMILY => {
                    if subfamily.is_none() {
                        subfamily = Some(s);
                    }
                }
                name_id::TYPOGRAPHIC_SUBFAMILY => {
                    if typo_subfamily.is_none() {
                        typo_subfamily = Some(s);
                    }
                }
                _ => {}
            }
        }
        let fam = typo_family.or(family);
        let sub = typo_subfamily.or(subfamily);
        if let (Some(f), Some(s)) = (fam, sub)
            && s.to_lowercase() != "regular"
        {
            names.push(format!("{} {}", f, s));
        }
    }
    names
}

/// 递归扫描一个目录，收集其中的字体文件及其元数据 (路径, 修改时间秒, 大小)。
/// 只做 stat，不读取字体内容，因此非常快——用于快速计算"字体集合签名"。
fn collect_font_files(dir: &Path, out: &mut Vec<(PathBuf, u64, u64)>, depth: usize) {
    if depth > 6 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_font_files(&path, out, depth + 1);
            continue;
        }
        let is_font = matches!(
            path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref(),
            Some("ttf") | Some("otf") | Some("ttc") | Some("otc")
        );
        if !is_font {
            continue;
        }
        let (mtime, size) = match entry.metadata() {
            Ok(md) => {
                let mtime = md
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                (mtime, md.len())
            }
            Err(_) => (0, 0),
        };
        out.push((path, mtime, size));
    }
}

/// 列出所有系统字体文件（含元数据），按路径排序以保证签名稳定。
fn list_system_font_files() -> Vec<(PathBuf, u64, u64)> {
    let mut files: Vec<(PathBuf, u64, u64)> = Vec::new();
    for dir in system_font_dirs() {
        collect_font_files(&dir, &mut files, 0);
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// 根据字体文件集合（路径 + 修改时间 + 大小）计算一个签名。
/// 只要没有字体被增删改，签名就保持不变，可据此判断磁盘缓存是否仍然有效。
fn signature_of(files: &[(PathBuf, u64, u64)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (path, mtime, size) in files {
        path.to_string_lossy().hash(&mut hasher);
        mtime.hash(&mut hasher);
        size.hash(&mut hasher);
    }
    hasher.finish()
}

/// 缓存文件路径：优先使用环境变量 `MPLCONFIGDIR`（与 matplotlib 习惯一致），
/// 未设置时回退到用户主目录下的 `.rsplotlib`（Windows 用 `LOCALAPPDATA`）。
fn font_cache_file() -> Option<PathBuf> {
    let dir = if let Some(d) = std::env::var_os("MPLCONFIGDIR").filter(|s| !s.is_empty()) {
        PathBuf::from(d)
    } else {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("LOCALAPPDATA")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(|d| Path::new(&d).join("rsplotlib"))?
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var_os("HOME").map(|d| Path::new(&d).join(".rsplotlib"))?
        }
    };
    Some(dir.join("rsplotlib-fontcache.txt"))
}

const CACHE_HEADER: &str = "RSPLOTLIB-FONTCACHE-V1";

/// 从磁盘缓存加载字体索引。仅当缓存头与签名都匹配时才返回 Some。
fn load_font_cache(path: &Path, signature: u64) -> Option<HashMap<String, String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    if lines.next()? != CACHE_HEADER {
        return None;
    }
    let sig_line = lines.next()?;
    let cached_sig: u64 = sig_line.strip_prefix("SIG=")?.parse().ok()?;
    if cached_sig != signature {
        return None;
    }
    let mut map: HashMap<String, String> = HashMap::new();
    for line in lines {
        if let Some((name, font_path)) = line.split_once('\t') {
            map.insert(name.to_string(), font_path.to_string());
        }
    }
    Some(map)
}

/// 把字体索引写入磁盘缓存（先写临时文件再原子重命名，兼容多进程并发）。
fn save_font_cache(path: &Path, signature: u64, map: &HashMap<String, String>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut out = String::with_capacity(map.len() * 32 + 64);
    out.push_str(CACHE_HEADER);
    out.push('\n');
    out.push_str(&format!("SIG={}\n", signature));
    for (name, font_path) in map {
        // 名称/路径若含制表符或换行则跳过，避免破坏行格式
        if name.contains('\t') || name.contains('\n') || font_path.contains('\t') || font_path.contains('\n') {
            continue;
        }
        out.push_str(name);
        out.push('\t');
        out.push_str(font_path);
        out.push('\n');
    }
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    if std::fs::write(&tmp, out.as_bytes()).is_ok() && std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// 读取一批字体文件，建立 "字体名(小写) → 文件路径" 索引。
fn build_index_from_files(files: &[(PathBuf, u64, u64)]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for (path, _, _) in files {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let path_str = path.to_string_lossy().to_string();
        for name in collect_font_names(&data) {
            // 先注册者优先（系统目录先于用户目录时不会被覆盖）
            map.entry(name.to_lowercase()).or_insert_with(|| path_str.clone());
        }
    }
    map
}

/// 扫描系统字体目录，建立 "字体名(小写) → 文件路径" 索引（OnceLock 进程内缓存）。
///
/// 为避免每次进程启动都重新读取+解析所有系统字体（macOS/Linux 上动辄数百个、
/// 上百 MB），这里额外维护一份磁盘缓存：
/// - 首先对系统字体文件集合做一次轻量 stat，算出签名；
/// - 若磁盘缓存存在且签名一致，直接加载（毫秒级）；
/// - 否则真正读取解析所有字体、重建索引，并写回磁盘缓存。
fn system_font_index() -> &'static HashMap<String, String> {
    static INDEX: OnceLock<(HashMap<String, String>, u64)> = OnceLock::new();
    let (map, signature) = INDEX.get_or_init(|| {
        let files = list_system_font_files();
        let signature = signature_of(&files);

        // 冷/热启动都按"此刻"的缓存目录去加载，命中即用（毫秒级）。
        if let Some(path) = font_cache_file()
            && let Some(map) = load_font_cache(&path, signature)
        {
            return (map, signature);
        }
        (build_index_from_files(&files), signature)
    });

    // 关键：磁盘缓存的**写入目录**在每次解析时按当前 `MPLCONFIGDIR` 重新确定，
    // 而不是绑定在 OnceLock 首次初始化的那一刻。这样即使 `MPLCONFIGDIR` 是在
    // 首次字体扫描之后才设置的（例如脚本先 import、随后才 os.environ[...] = ...），
    // 缓存也会落到用户指定的目录，而不会被永久锁死在兜底目录里。
    sync_disk_cache(map, *signature);
    map
}

/// 把内存中的字体索引持久化到"当前"缓存目录（按 `MPLCONFIGDIR` 实时解析）。
///
/// 通过记录"上次成功处理过的目标路径"来去重：只有当目标目录发生变化、且该目录
/// 下尚无有效缓存时才真正写盘。因此在同一目录稳定之后，后续每次解析的调用都是
/// O(1) 的无副作用早退，不会带来额外的磁盘开销。
fn sync_disk_cache(map: &HashMap<String, String>, signature: u64) {
    let path = match font_cache_file() {
        Some(p) => p,
        None => return,
    };
    static LAST_SYNCED: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    let cell = LAST_SYNCED.get_or_init(|| Mutex::new(None));
    let mut last = match cell.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if last.as_deref() == Some(path.as_path()) {
        return;
    }
    if load_font_cache(&path, signature).is_none() {
        save_font_cache(&path, signature, map);
    }
    *last = Some(path);
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

    // 扫描系统已安装字体：支持任意系统字体（如 "Arial Bold"、"Songti SC"、
    // "Comic Sans MS" 等），按真实家族名/全名/PostScript 名匹配到实际文件。
    if let Some(path) = system_font_index().get(&family_lower)
        && Path::new(path).is_file()
    {
        return Some(path.clone());
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

    // 清空旧的字体栈，然后注册所有新的字体
    crate::utils::font_stack::clear_font_stack();

    // 注册所有能解析到本地文件的字体到 font_stack。
    // register_sans_serif_font 现在会将每个字体以其真实家族名称注册到 plotters，
    // 并推入 font_stack。渲染时通过 font_stack::resolve_font_family(text, ...)
    // 根据文本字符自动选择能覆盖所有字符的第一个字体。
    let mut last_registered: Option<String> = None;
    for family in candidates.iter() {
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

        if rsplotlib.call_method1("register_sans_serif_font", (path.clone(), family.to_string())).is_ok() {
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