//! 刻度定位器 (Locator) 和格式化器 (Formatter) 的 Rust 实现
//!
//! 提供与 matplotlib 兼容的刻度计算和格式化功能。
//! 所有类通过 PyO3 导出为 Python 类，Python 侧的 ticker.py 为薄包装层。

use pyo3::prelude::*;

// ==================== 定位器 ====================

/// MultipleLocator - 倍数定位器，刻度位置是基数的整数倍
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct MultipleLocator {
    pub base: f64,
}

#[pymethods]
impl MultipleLocator {
    #[new]
    fn new(base: f64) -> Self {
        MultipleLocator { base }
    }

    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        if self.base == 0.0 {
            return vec![];
        }
        let vmin = (vmin / self.base).floor() * self.base;
        let vmax = (vmax / self.base).ceil() * self.base;
        let n = ((vmax + self.base * 0.5 - vmin) / self.base).ceil() as usize;
        let mut ticks = Vec::with_capacity(n);
        let mut v = vmin;
        while v <= vmax + self.base * 0.5 {
            ticks.push(v);
            v += self.base;
        }
        ticks
    }

    fn __repr__(&self) -> String {
        format!("MultipleLocator(base={})", self.base)
    }
}

/// MaxNLocator - 最大数量定位器，最多 nbins+1 个刻度
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct MaxNLocator {
    pub nbins: i32,
    pub integer: bool,
}

fn nice_step(step: f64) -> f64 {
    if step <= 0.0 {
        return 1.0;
    }
    let exponent = step.log10().floor();
    let fraction = step / (10.0_f64).powf(exponent);
    let nice = if fraction < 1.5 {
        1.0
    } else if fraction < 3.5 {
        2.0
    } else if fraction < 7.5 {
        5.0
    } else {
        10.0
    };
    nice * (10.0_f64).powf(exponent)
}

#[pymethods]
impl MaxNLocator {
    #[new]
    fn new(nbins: i32, integer: bool) -> Self {
        MaxNLocator { nbins, integer }
    }

    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        if vmax <= vmin {
            return vec![vmin];
        }
        let range_val = vmax - vmin;
        if range_val == 0.0 {
            return vec![vmin];
        }
        let raw_step = range_val / self.nbins as f64;
        let step = if self.integer {
            (1.0_f64).max(raw_step)
        } else {
            nice_step(raw_step)
        };
        let vmin = (vmin / step).floor() * step;
        let estimated = ((vmax + step * 0.5 - vmin) / step).ceil() as usize;
        let mut ticks = Vec::with_capacity(estimated.min(50));
        let mut v = vmin;
        while v <= vmax + step * 0.5 {
            if !self.integer || (v - v.round()).abs() < 1e-10 {
                let val: f64 = if self.integer { v.round() } else { v };
                if ticks.is_empty() || (f64::abs(val - ticks[ticks.len() - 1]) > 1e-10) {
                    ticks.push(val);
                }
            }
            v += step;
        }
        while ticks.len() > (self.nbins + 1) as usize {
            ticks = ticks.iter().step_by(2).copied().collect();
        }
        ticks
    }

    fn __repr__(&self) -> String {
        format!("MaxNLocator(nbins={}, integer={})", self.nbins, self.integer)
    }
}

/// AutoMinorLocator - 自动次要刻度定位器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct AutoMinorLocator {
    pub n: i32,
}

#[pymethods]
impl AutoMinorLocator {
    #[new]
    fn new(n: i32) -> Self {
        AutoMinorLocator { n }
    }

    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        let major_step = (vmax - vmin) / 10.0;
        if major_step <= 0.0 {
            return vec![];
        }
        let minor_step = major_step / self.n as f64;
        let n = ((vmax + minor_step * 0.5 - vmin) / minor_step).ceil() as usize;
        let mut ticks = Vec::with_capacity(n);
        let mut v = vmin;
        while v <= vmax + minor_step * 0.5 {
            ticks.push(v);
            v += minor_step;
        }
        ticks
    }

    fn __repr__(&self) -> String {
        format!("AutoMinorLocator(n={})", self.n)
    }
}

/// FixedLocator - 固定位置定位器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct FixedLocator {
    pub locs: Vec<f64>,
}

#[pymethods]
impl FixedLocator {
    #[new]
    fn new(locs: Vec<f64>) -> Self {
        FixedLocator { locs }
    }

    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        self.locs
            .iter()
            .filter(|&&l| l >= vmin && l <= vmax)
            .copied()
            .collect()
    }
}

/// LinearLocator - 线性定位器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct LinearLocator {
    pub numticks: i32,
}

#[pymethods]
impl LinearLocator {
    #[new]
    fn new(numticks: i32) -> Self {
        LinearLocator { numticks }
    }

    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        if vmax <= vmin {
            return vec![vmin];
        }
        let step = (vmax - vmin) / (self.numticks - 1) as f64;
        (0..self.numticks)
            .map(|i| vmin + i as f64 * step)
            .collect()
    }
}

/// LogLocator - 对数定位器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct LogLocator {
    pub base: f64,
    pub numticks: i32,
}

#[pymethods]
impl LogLocator {
    #[new]
    fn new(base: f64, numticks: i32) -> Self {
        LogLocator { base, numticks }
    }

    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        let vmin = if vmin <= 0.0 { 1e-10 } else { vmin };
        if self.numticks <= 1 {
            return vec![vmin];
        }
        let log_min = vmin.log(self.base);
        let log_max = vmax.log(self.base);
        let step = (log_max - log_min) / (self.numticks - 1) as f64;
        (0..self.numticks)
            .map(|i| self.base.powf(log_min + i as f64 * step))
            .collect()
    }
}

/// NullLocator - 空定位器，不显示刻度
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct NullLocator;

#[pymethods]
impl NullLocator {
    #[new]
    fn new() -> Self {
        NullLocator
    }

    fn tick_values(&self, _vmin: f64, _vmax: f64) -> Vec<f64> {
        vec![]
    }
}

// ==================== 格式化器 ====================

/// NullFormatter - 不显示标签
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct NullFormatter;

#[pymethods]
impl NullFormatter {
    #[new]
    fn new() -> Self {
        NullFormatter
    }

    fn __call__(&self, _value: f64) -> String {
        String::new()
    }

    fn format_ticks(&self, values: Vec<f64>) -> Vec<String> {
        values.iter().map(|_| String::new()).collect()
    }
}

/// FixedFormatter - 固定标签格式化器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct FixedFormatter {
    pub seq: Vec<String>,
}

#[pymethods]
impl FixedFormatter {
    #[new]
    fn new(seq: Vec<String>) -> Self {
        FixedFormatter { seq }
    }

    fn __call__(&self, value: f64) -> String {
        let idx = value.round() as isize;
        if idx >= 0 && (idx as usize) < self.seq.len() {
            self.seq[idx as usize].clone()
        } else {
            String::new()
        }
    }

    fn format_ticks(&self, values: Vec<f64>) -> Vec<String> {
        values.iter().map(|&v| self.__call__(v)).collect()
    }
}

/// FormatStrFormatter - 格式化字符串
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct FormatStrFormatter {
    pub fmt: String,
}

#[pymethods]
impl FormatStrFormatter {
    #[new]
    fn new(fmt: String) -> Self {
        FormatStrFormatter { fmt }
    }

    fn __call__(&self, value: f64) -> String {
        // 支持 % 格式化
        format!("{}", self.fmt.replace("%d", &(value as i64).to_string())
            .replace("%f", &format!("{:.6}", value))
            .replace("%g", &format!("{}", value))
            .replace("%e", &format!("{:e}", value))
            .replace("%.2e", &format!("{:.2e}", value))
            .replace("%.1f", &format!("{:.1}", value))
            .replace("%.2f", &format!("{:.2}", value)))
    }

    fn format_ticks(&self, values: Vec<f64>) -> Vec<String> {
        values.iter().map(|&v| self.__call__(v)).collect()
    }
}

fn format_g_value(value: f64) -> String {
    if (value - value.round()).abs() < 1e-10 {
        format!("{:.0}", value)
    } else {
        let s = format!("{:.6}", value);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

/// ScalarFormatter - 标量格式化器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct ScalarFormatter;

#[pymethods]
impl ScalarFormatter {
    #[new]
    fn new() -> Self {
        ScalarFormatter
    }

    fn __call__(&self, value: f64) -> String {
        let abs_val = value.abs();
        if abs_val >= 1e4 || (abs_val < 1e-3 && abs_val > 0.0) {
            format!("{:.2e}", value)
        } else {
            format_g_value(value)
        }
    }

    fn format_ticks(&self, values: Vec<f64>) -> Vec<String> {
        values.iter().map(|&v| self.__call__(v)).collect()
    }
}

/// LogFormatterSciNotation - 科学计数法格式化器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct LogFormatterSciNotation;

#[pymethods]
impl LogFormatterSciNotation {
    #[new]
    fn new() -> Self {
        LogFormatterSciNotation
    }

    fn __call__(&self, value: f64) -> String {
        if value <= 0.0 {
            return "0".to_string();
        }
        let exp = value.log10();
        // 四舍五入到整数指数
        let int_exp = if exp >= 0.0 {
            (exp + 0.5).floor() as i32
        } else {
            (exp - 0.5).ceil() as i32
        };
        format!("$10^{{{}}}$", int_exp)
    }

    fn format_ticks(&self, values: Vec<f64>) -> Vec<String> {
        values.iter().map(|&v| self.__call__(v)).collect()
    }
}

/// FuncFormatter - 函数格式化器，包装一个 Python 可调用对象
#[pyclass(skip_from_py_object)]
pub struct FuncFormatter {
    pub func: Py<PyAny>,
}

#[pymethods]
impl FuncFormatter {
    #[new]
    fn new(func: Py<PyAny>) -> Self {
        FuncFormatter { func }
    }

    fn __call__(&self, py: Python, value: f64) -> PyResult<String> {
        let result = self.func.bind(py).call1((value,))?;
        result.extract::<String>()
    }

    fn format_ticks(&self, py: Python, values: Vec<f64>) -> PyResult<Vec<String>> {
        values.iter().map(|&v| self.__call__(py, v)).collect()
    }
}

/// StrMethodFormatter - 字符串方法格式化器
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct StrMethodFormatter {
    pub fmt: String,
}

#[pymethods]
impl StrMethodFormatter {
    #[new]
    fn new(fmt: String) -> Self {
        StrMethodFormatter { fmt }
    }

    fn __call__(&self, value: f64) -> String {
        self.fmt.replace("{}", &format!("{}", value))
    }

    fn format_ticks(&self, values: Vec<f64>) -> Vec<String> {
        values.iter().map(|&v| self.__call__(v)).collect()
    }
}

/// Tick - 刻度对象
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct Tick {
    pub loc: f64,
    pub label: String,
}

#[pymethods]
impl Tick {
    #[new]
    fn new(loc: f64, label: String) -> Self {
        Tick { loc, label }
    }
}

/// 便捷函数：创建自动定位器
#[pyfunction]
pub fn auto_locator() -> MaxNLocator {
    MaxNLocator {
        nbins: 10,
        integer: false,
    }
}

/// 模块注册函数
pub fn register(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MultipleLocator>()?;
    m.add_class::<MaxNLocator>()?;
    m.add_class::<AutoMinorLocator>()?;
    m.add_class::<FixedLocator>()?;
    m.add_class::<LinearLocator>()?;
    m.add_class::<LogLocator>()?;
    m.add_class::<NullLocator>()?;
    m.add_class::<NullFormatter>()?;
    m.add_class::<FixedFormatter>()?;
    m.add_class::<FormatStrFormatter>()?;
    m.add_class::<ScalarFormatter>()?;
    m.add_class::<LogFormatterSciNotation>()?;
    m.add_class::<FuncFormatter>()?;
    m.add_class::<StrMethodFormatter>()?;
    m.add_class::<Tick>()?;
    m.add_function(wrap_pyfunction!(auto_locator, m)?)?;
    Ok(())
}