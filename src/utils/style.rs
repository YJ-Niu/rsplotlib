use pyo3::prelude::*;

/// 可用样式列表
pub static AVAILABLE_STYLES: &[&str] = &[
    "default",
    "classic",
    "ggplot",
    "seaborn-v0_8",
    "fast",
    "fivethirtyeight",
    "grayscale",
    "dark_background",
    "bmh",
    "tableau-colorblind10",
];

/// Style - 样式管理器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct Style {
    #[pyo3(get)]
    current: String,
}

#[pymethods]
impl Style {
    #[new]
    fn new() -> Self {
        Style {
            current: "default".to_string(),
        }
    }

    /// 应用样式
    fn use_(&mut self, style_name: &str) {
        self.current = style_name.to_string();
    }

    /// 返回可用样式列表
    fn available(&self) -> Vec<String> {
        AVAILABLE_STYLES.iter().map(|s| s.to_string()).collect()
    }

    fn __repr__(&self) -> String {
        format!("<Style: {}>", self.current)
    }
}

/// 注册 style 模块中的类
pub fn register(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Style>()?;
    Ok(())
}