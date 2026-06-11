use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyAny};
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::ShapeStyle;

use crate::colors::{RgbColor, parse_color, default_color, default_color_str, to_plotters_color};
use crate::elements::PlotElement;

/// 将 Python 对象（list、numpy 数组等）转换为 Vec<f64>
fn py_to_vec_f64(obj: &Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
    // 先尝试直接 extract（Python list）
    if let Ok(v) = obj.extract::<Vec<f64>>() {
        return Ok(v);
    }
    // 尝试调用 .tolist()（numpy 数组）
    if obj.hasattr("tolist")? {
        let list = obj.call_method0("tolist")?;
        return list.extract::<Vec<f64>>();
    }
    // 尝试转换为 list
    let items: Vec<Bound<'_, PyAny>> = obj.try_iter()?.collect::<PyResult<Vec<_>>>()?;
    let list = PyList::new(obj.py(), items)?;
    list.extract::<Vec<f64>>()
}

/// 将 Python 对象（list、numpy 数组等）转换为 Vec<Option<f64>>
/// 支持 None 值
fn py_to_vec_option_f64(obj: &Bound<'_, PyAny>) -> PyResult<Vec<Option<f64>>> {
    // 先尝试直接 extract
    if let Ok(v) = obj.extract::<Vec<Option<f64>>>() {
        return Ok(v);
    }
    // 尝试调用 .tolist()（numpy 数组）
    if obj.hasattr("tolist")? {
        let list = obj.call_method0("tolist")?;
        return list.extract::<Vec<Option<f64>>>();
    }
    // 尝试逐元素转换
    let mut result = Vec::new();
    for item in obj.try_iter()? {
        let item = item?;
        if item.is_none() {
            result.push(None);
        } else if let Ok(v) = item.extract::<f64>() {
            result.push(Some(v));
        } else {
            return Err(PyValueError::new_err("Cannot convert element to f64"));
        }
    }
    Ok(result)
}

/// 将 Python 对象转换为 Vec<Vec<f64>>（用于 boxplot、hist 等）
fn py_to_vec_vec_f64(obj: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<f64>>> {
    if let Ok(v) = obj.extract::<Vec<Vec<f64>>>() {
        return Ok(v);
    }
    if obj.hasattr("tolist")? {
        let list = obj.call_method0("tolist")?;
        if let Ok(v) = list.extract::<Vec<Vec<f64>>>() {
            return Ok(v);
        }
        // 可能是 1D 数组
        if let Ok(v) = list.extract::<Vec<f64>>() {
            return Ok(vec![v]);
        }
    }
    // 尝试作为 1D 数组
    if let Ok(v) = obj.extract::<Vec<f64>>() {
        return Ok(vec![v]);
    }
    Err(PyValueError::new_err("Cannot convert to Vec<Vec<f64>>"))
}
use crate::axis::{Axis, Patch, SpineDict};

/// 字体大小缩放并四舍五入到1位小数
pub fn scale_font(size: f64, font_scale: f64) -> f64 {
    (size * font_scale * 10.0).round() / 10.0
}

#[pyclass(skip_from_py_object)]
pub struct Axes {
    pub elements: Vec<PlotElement>,
    pub xlabel: String,
    pub ylabel: String,
    pub title: String,
    pub title_fontsize: f64,
    pub xlim: Option<(f64, f64)>,
    pub ylim: Option<(f64, f64)>,
    pub grid_visible: bool,
    pub legend_loc: Option<String>,
    pub element_count: usize,
    pub legend_labels: Vec<(String, RgbColor, String, Option<String>, f64)>,
    pub xscale: String,
    pub yscale: String,
    pub xticks_val: Option<Vec<f64>>,
    pub xtick_labels: Option<Vec<String>>,
    pub yticks_val: Option<Vec<f64>>,
    pub ytick_labels: Option<Vec<String>>,
    pub is_twin_x: bool,
    pub is_twin_y: bool,
    pub twin_axes: Vec<Axes>,
    pub facecolor: String,
    pub spine_top: bool,
    pub spine_bottom: bool,
    pub spine_left: bool,
    pub spine_right: bool,
    pub spine_color: String,
    pub spine_linewidth: f64,
    pub grid_color: Option<String>,
    pub grid_linewidth: Option<f64>,
    pub grid_linestyle: Option<String>,
    pub grid_axis: String,
    pub minor_grid_visible: bool,
    pub minor_grid_x_visible: bool,
    pub minor_grid_y_visible: bool,
    pub minor_grid_color: Option<String>,
    pub minor_grid_linewidth: Option<f64>,
    pub minor_grid_linestyle: Option<String>,
    pub tick_bottom: bool,
    pub tick_top: bool,
    pub tick_left: bool,
    pub tick_right: bool,
    pub tick_labelsize: f64,
    pub self_py: Option<Py<PyAny>>,
    pub xaxis_major_locator: Option<Py<PyAny>>,
    pub xaxis_minor_locator: Option<Py<PyAny>>,
    pub yaxis_major_locator: Option<Py<PyAny>>,
    pub yaxis_minor_locator: Option<Py<PyAny>>,
}

impl Clone for Axes {
    fn clone(&self) -> Self {
        Axes {
            elements: self.elements.clone(),
            xlabel: self.xlabel.clone(),
            ylabel: self.ylabel.clone(),
            title: self.title.clone(),
            title_fontsize: self.title_fontsize,
            xlim: self.xlim,
            ylim: self.ylim,
            grid_visible: self.grid_visible,
            legend_loc: self.legend_loc.clone(),
            element_count: self.element_count,
            legend_labels: self.legend_labels.clone(),
            xscale: self.xscale.clone(),
            yscale: self.yscale.clone(),
            xticks_val: self.xticks_val.clone(),
            xtick_labels: self.xtick_labels.clone(),
            yticks_val: self.yticks_val.clone(),
            ytick_labels: self.ytick_labels.clone(),
            is_twin_x: self.is_twin_x,
            is_twin_y: self.is_twin_y,
            twin_axes: self.twin_axes.clone(),
            facecolor: self.facecolor.clone(),
            spine_top: self.spine_top,
            spine_bottom: self.spine_bottom,
            spine_left: self.spine_left,
            spine_right: self.spine_right,
            spine_color: self.spine_color.clone(),
            spine_linewidth: self.spine_linewidth,
            grid_color: self.grid_color.clone(),
            grid_linewidth: self.grid_linewidth,
            grid_linestyle: self.grid_linestyle.clone(),
            grid_axis: self.grid_axis.clone(),
            minor_grid_visible: self.minor_grid_visible,
            minor_grid_x_visible: self.minor_grid_x_visible,
            minor_grid_y_visible: self.minor_grid_y_visible,
            minor_grid_color: self.minor_grid_color.clone(),
            minor_grid_linewidth: self.minor_grid_linewidth,
            minor_grid_linestyle: self.minor_grid_linestyle.clone(),
            tick_bottom: self.tick_bottom,
            tick_top: self.tick_top,
            tick_left: self.tick_left,
            tick_right: self.tick_right,
            tick_labelsize: self.tick_labelsize,
            self_py: None,
            xaxis_major_locator: None,
            xaxis_minor_locator: None,
            yaxis_major_locator: None,
            yaxis_minor_locator: None,
        }
    }
}

/// 解析 matplotlib 格式字符串
/// 返回 (marker, linestyle, color) 三元组，如果字符串不是 fmt 格式则返回 None
fn parse_fmt_string(fmt: &str) -> Option<(Option<String>, Option<String>, Option<String>)> {
    // 已知 marker 字符
    const MARKERS: &[&str] = &["o", "s", "^", "v", "D", "d", "*", "+", "x", ".", ",", "|", "_", "h", "H", "p", "P", "<", ">", "1", "2", "3", "4"];
    // 已知 color
    const COLORS: &[&str] = &["b", "g", "r", "c", "m", "y", "k", "w"];

    let mut found_marker: Option<String> = None;
    let mut found_ls: Option<String> = None;
    let mut found_color: Option<String> = None;
    let mut i: usize = 0;

    // 尝试解析 linestyle（在前缀位置时优先）
    if fmt.starts_with("--") {
        found_ls = Some("--".to_string());
        i = 2;
    } else if fmt.starts_with("-.") {
        found_ls = Some("-.".to_string());
        i = 2;
    } else if fmt.starts_with('-') {
        found_ls = Some("-".to_string());
        i = 1;
    } else if fmt.starts_with(':') {
        found_ls = Some(":".to_string());
        i = 1;
    }

    // 解析 color（单字符）
    if i < fmt.len() {
        let c = &fmt[i..i+1];
        if COLORS.contains(&c) {
            found_color = Some(c.to_string());
            i += 1;
        }
    }

    // 解析 marker
    if i < fmt.len() {
        let m1 = &fmt[i..i+1];
        if MARKERS.contains(&m1) {
            found_marker = Some(m1.to_string());
            i += 1;
        }
        // 检查是否还有更多 marker 字符
        while i < fmt.len() {
            let m = &fmt[i..i+1];
            if MARKERS.contains(&m) {
                found_marker = Some(m.to_string());
                i += 1;
            } else {
                break;
            }
        }
    }

    // 如果还有剩余字符，说明不是 fmt 字符串
    if i < fmt.len() {
        return None;
    }

    // 必须至少解析出 marker 或 linestyle 才算 fmt 字符串
    if found_marker.is_none() && found_ls.is_none() && found_color.is_none() {
        return None;
    }

    Some((found_marker, found_ls, found_color))
}

fn is_format_string(s: &str) -> bool {
    parse_fmt_string(s).is_some()
}

#[pymethods]
impl Axes {
    #[new]
    pub fn new() -> Self {
        Axes {
            elements: Vec::new(),
            xlabel: String::new(),
            ylabel: String::new(),
            title: String::new(),
            title_fontsize: 12.0,
            xlim: None,
            ylim: None,
            grid_visible: false,
            legend_loc: None,
            element_count: 0,
            legend_labels: Vec::new(),
            xscale: "linear".to_string(),
            yscale: "linear".to_string(),
            xticks_val: None,
            xtick_labels: None,
            yticks_val: None,
            ytick_labels: None,
            is_twin_x: false,
            is_twin_y: false,
            twin_axes: Vec::new(),
            facecolor: "white".to_string(),
            spine_top: true,
            spine_bottom: true,
            spine_left: true,
            spine_right: true,
            spine_color: "black".to_string(),
            spine_linewidth: 0.8,
            grid_color: None,
            grid_linewidth: None,
            grid_linestyle: None,
            grid_axis: "both".to_string(),
            minor_grid_visible: false,
            minor_grid_x_visible: false,
            minor_grid_y_visible: false,
            minor_grid_color: None,
            minor_grid_linewidth: None,
            minor_grid_linestyle: None,
            tick_bottom: true,
            tick_top: true,
            tick_left: true,
            tick_right: true,
            tick_labelsize: 12.0,
            self_py: None,
            xaxis_major_locator: None,
            xaxis_minor_locator: None,
            yaxis_major_locator: None,
            yaxis_minor_locator: None,
        }
    }

    #[pyo3(signature = (x, y, label=None, color=None, linestyle="-", marker=None, linewidth=1.5, lw=None, c=None, ls=None, markersize=None, markeredgewidth=None, solid_capstyle=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn plot(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        y: Bound<'_, PyAny>,
        label: Option<String>,
        color: Option<String>,
        linestyle: &str,
        marker: Option<String>,
        linewidth: f64,
        lw: Option<f64>,
        c: Option<String>,
        ls: Option<String>,
        markersize: Option<f64>,
        markeredgewidth: Option<f64>,
        solid_capstyle: Option<String>,
    ) -> PyResult<()> {
        // matplotlib 兼容：解析格式字符串
        // 如果 label 是格式字符串（如 'o', '-', 'r--', 'b-o'），从其中提取 marker/linestyle/color
        let mut actual_label = label;
        let mut actual_marker = marker;
        let mut actual_linestyle = linestyle.to_string();
        let mut actual_color = color;
        if let Some(ref lbl) = actual_label {
            if is_format_string(lbl) {
                if let Some((fmt_marker, fmt_ls, fmt_color)) = parse_fmt_string(lbl) {
                    if actual_marker.is_none() {
                        actual_marker = fmt_marker;
                    }
                    if ls.is_none() && linestyle == "-" {
                        if let Some(ls_val) = fmt_ls {
                            actual_linestyle = ls_val;
                        }
                    }
                    if actual_color.is_none() {
                        actual_color = fmt_color;
                    }
                    actual_label = None;
                }
            }
        }

        let x_vec = py_to_vec_option_f64(&x)?;
        let y_vec = py_to_vec_option_f64(&y)?;
        let color = c.or(actual_color);
        let linewidth = lw.unwrap_or(linewidth);
        let linestyle = ls.as_deref().unwrap_or(&actual_linestyle);
        let idx = self.element_count;
        self.element_count += 1;
        // consume optional params to avoid unused variable warnings while preserving Python API
        let _ = markeredgewidth;
        let color_val = color.clone().unwrap_or_default();
        let linestyle_val = linestyle.to_string();
        // matplotlib 兼容：linestyle='' 或 'None'/'none' 都表示无线条
        let linestyle_eff = if linestyle.is_empty()
            || linestyle.eq_ignore_ascii_case("none")
            || linestyle.eq_ignore_ascii_case("null") {
            " ".to_string()
        } else {
            linestyle_val.clone()
        };
        self.elements.push(PlotElement::Line {
            x: x_vec,
            y: y_vec,
            label: actual_label.clone(),
            color: color_val,
            linestyle: linestyle_eff,
            marker: actual_marker,
            linewidth,
            color_idx: idx,
            solid_capstyle: solid_capstyle.unwrap_or_else(|| "butt".to_string()),
            markersize,
        });
        if let Some(lbl) = actual_label {
            let c = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, c, linestyle_val, None, linewidth));
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, s=20.0, c=None, marker="o", label=None, alpha=1.0))]
    #[allow(clippy::too_many_arguments)]
    pub fn scatter(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        y: Bound<'_, PyAny>,
        s: f64,
        c: Option<String>,
        marker: &str,
        label: Option<String>,
        alpha: f64,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let y_vec = py_to_vec_f64(&y)?;
        let idx = self.element_count;
        self.element_count += 1;
        let c_val = c.clone().unwrap_or_default();
        let marker_val = marker.to_string();
        self.elements.push(PlotElement::Scatter {
            x: x_vec,
            y: y_vec,
            s,
            c: c_val.clone(),
            marker: marker_val.clone(),
            label: label.clone(),
            alpha,
            color_idx: idx,
        });
        if let Some(lbl) = label {
            let col = parse_color(&c.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), Some(marker_val), 1.5));
        }
        Ok(())
    }

    #[pyo3(signature = (x, height, width=0.8, color=None, label=None))]
    pub fn bar(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        height: Bound<'_, PyAny>,
        width: f64,
        color: Option<String>,
        label: Option<String>,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let height_vec = py_to_vec_f64(&height)?;
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::Bar {
            x: x_vec,
            height: height_vec,
            width,
            color: color_val.clone(),
            label: label.clone(),
            color_idx: idx,
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), None, 1.5));
        }
        Ok(())
    }

    #[pyo3(signature = (y, width, height=0.8, color=None, label=None))]
    pub fn barh(
        &mut self,
        _py: Python<'_>,
        y: Bound<'_, PyAny>,
        width: Bound<'_, PyAny>,
        height: f64,
        color: Option<String>,
        label: Option<String>,
    ) -> PyResult<()> {
        let y_vec = py_to_vec_f64(&y)?;
        let width_vec = py_to_vec_f64(&width)?;
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::BarH {
            y: y_vec,
            width: width_vec,
            height,
            color: color_val.clone(),
            label: label.clone(),
            color_idx: idx,
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), None, 1.5));
        }
        Ok(())
    }

    #[pyo3(signature = (x, bins=None, density=false, label=None, alpha=0.7, color=None, facecolor=None, align=None, histtype=None))]
    #[allow(clippy::too_many_arguments)]
    pub fn hist(
        &mut self,
        py: Python<'_>,
        x: Bound<'_, PyAny>,
        bins: Option<Bound<'_, PyAny>>,
        density: bool,
        label: Option<String>,
        alpha: f64,
        color: Option<Bound<'_, PyAny>>,
        facecolor: Option<Bound<'_, PyAny>>,
        #[allow(unused_variables)] align: Option<String>,
        #[allow(unused_variables)] histtype: Option<String>,
    ) -> PyResult<(Py<PyAny>, Vec<f64>, Option<Vec<Vec<f64>>>)> {
        let x_parsed: Vec<Vec<f64>> = Self::parse_hist_data(&x)?;
        let bins = bins.unwrap_or_else(|| pyo3::types::PyInt::new(py, 10).as_any().clone());
        let (num_bins, custom_edges): (usize, Option<Vec<f64>>) = if let Ok(n) = bins.extract::<usize>() {
            (n, None)
        } else if let Ok(edges) = bins.extract::<Vec<f64>>() {
            if edges.len() < 2 {
                return Err(PyValueError::new_err("bin_edges must have at least 2 elements"));
            }
            (edges.len() - 1, Some(edges))
        } else {
            return Err(PyValueError::new_err("bins must be an integer or a list of bin edges"));
        };
        let colors: Vec<String> = if let Some(fc) = facecolor {
            Self::parse_color_list(&fc, x_parsed.len())?
        } else if let Some(c) = color {
            Self::parse_color_list(&c, x_parsed.len())?
        } else {
            (0..x_parsed.len()).map(|i| default_color_str(i)).collect()
        };
        let histtype_val = histtype.unwrap_or_else(|| "bar".to_string());
        let idx = self.element_count;
        self.element_count += 1;
        self.elements.push(PlotElement::Hist {
            data_all: x_parsed.clone(),
            bins: num_bins,
            density,
            histtype: histtype_val,
            label: label.clone(),
            alpha,
            colors: colors.clone(),
            color_idx: idx,
            bin_edges: custom_edges.clone(),
        });
        if let Some(lbl) = label {
            let col = parse_color(colors.first().unwrap_or(&String::new()), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), None, 1.5));
        }
        let all_data: Vec<f64> = x_parsed.iter().flatten().cloned().collect();
        let global_min = if all_data.is_empty() { 0.0 } else { all_data.iter().cloned().fold(f64::INFINITY, f64::min) };
        let global_max = if all_data.is_empty() { 1.0 } else { all_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max) };
        let global_range = global_max - global_min;
        let bin_width = if global_range < 1e-10 { 1.0 } else { global_range / num_bins as f64 };
        let n: Vec<Vec<f64>> = x_parsed.iter().map(|dataset| {
            if dataset.is_empty() {
                return vec![0.0; num_bins];
            }
            let mut counts = vec![0usize; num_bins];
            for &val in dataset {
                let mut bin = ((val - global_min) / bin_width).floor() as usize;
                if bin >= num_bins { bin = num_bins - 1; }
                counts[bin] += 1;
            }
            let total = dataset.len() as f64;
            counts.iter().map(|&c| if density { c as f64 / (total * bin_width) } else { c as f64 }).collect()
        }).collect();
        let bin_edges: Vec<f64> = if let Some(ref edges) = custom_edges {
            edges.clone()
        } else {
            (0..=num_bins).map(|i| global_min + i as f64 * bin_width).collect()
        };
        let n_obj: Py<PyAny> = if x_parsed.len() <= 1 {
            let empty: Vec<f64> = Vec::new();
            let data = n.first().unwrap_or(&empty);
            PyList::new(py, data.as_slice()).unwrap().into_any().unbind()
        } else {
            let lists: Vec<Bound<'_, PyList>> = n.iter()
                .map(|inner| PyList::new(py, inner.as_slice()).unwrap())
                .collect();
            PyList::new(py, lists.as_slice()).unwrap().into_any().unbind()
        };
        Ok((n_obj, bin_edges, None))
    }

    #[pyo3(signature = (x, cmap="viridis", aspect="auto"))]
    #[allow(unused_variables)]
    pub fn imshow(&mut self, x: Vec<Vec<f64>>, cmap: &str, aspect: &str) {
        self.elements.push(PlotElement::Image {
            data: x,
            cmap: cmap.to_string(),
        });
    }

    #[pyo3(signature = (text, color=None))]
    pub fn set_xlabel(&mut self, text: String, color: Option<String>) {
        let _ = color;
        self.xlabel = text;
    }

    #[pyo3(signature = (text, color=None))]
    pub fn set_ylabel(&mut self, text: String, color: Option<String>) {
        let _ = color;
        self.ylabel = text;
    }

    #[pyo3(signature = (text, color=None, fontsize=None))]
    pub fn set_title(&mut self, text: String, color: Option<String>, fontsize: Option<f64>) {
        let _ = color;
        self.title = text;
        if let Some(fs) = fontsize {
            self.title_fontsize = fs;
        }
    }

    #[pyo3(signature = (loc="best"))]
    pub fn legend(&mut self, loc: &str) {
        self.legend_loc = Some(loc.to_string());
    }

    #[pyo3(signature = (_v=None))]
    pub fn axis(&mut self, _v: Option<String>) {
        if let Some(v) = _v {
            match v.as_str() {
                "off" => self._axis_off(),
                "on" => {
                    self.spine_top = true;
                    self.spine_bottom = true;
                    self.spine_left = true;
                    self.spine_right = true;
                    self.tick_bottom = true;
                    self.tick_top = true;
                    self.tick_left = true;
                    self.tick_right = true;
                }
                _ => {}
            }
        }
    }

    #[pyo3(signature = (visible=None, c=None, ls=None, lw=None, axis=None))]
    pub fn grid(&mut self, visible: Option<bool>, c: Option<String>, ls: Option<String>, lw: Option<f64>, axis: Option<String>) {
        self.grid_visible = visible.unwrap_or(true);
        if let Some(a) = axis {
            self.grid_axis = a;
        }
        if c.is_some() || lw.is_some() || ls.is_some() {
            self.grid_color = c;
            self.grid_linewidth = lw;
            self.grid_linestyle = ls;
        }
    }

    pub fn set_xlim(&mut self, left: f64, right: f64) {
        self.xlim = Some((left, right));
    }

    pub fn set_ylim(&mut self, bottom: f64, top: f64) {
        self.ylim = Some((bottom, top));
    }

    #[pyo3(signature = (x, y, text, fontsize=None, color=None, c=None, family=None))]
    pub fn text(
        &mut self,
        _py: Python<'_>,
        x: f64,
        y: f64,
        text: Bound<'_, PyAny>,
        fontsize: Option<i32>,
        color: Option<String>,
        c: Option<String>,
        family: Option<String>,
    ) {
        let color = c.or(color);
        let text_str: String = text.extract::<String>().unwrap_or_else(|_| {
            text.str().map(|s| s.to_string()).unwrap_or_default()
        });
        let col = parse_color(&color.unwrap_or_else(|| "black".to_string()), 0).unwrap_or(RgbColor(0, 0, 0));
        // family 参数目前用于通过 Python 端的 _font_resolver 注册字体到 plotters。
        // Rust 端不再做额外处理（因为 plotters 的 sans-serif family 已被全局注册）。
        let _ = family;
        self.elements.push(PlotElement::Text {
            x,
            y,
            text: text_str,
            fontsize: fontsize.unwrap_or(12),
            color: col,
        });
    }

    pub fn axhline(
        &mut self,
        y: Option<f64>,
        color: Option<String>,
        linestyle: Option<String>,
        linewidth: Option<f64>,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        self.elements.push(PlotElement::HLine {
            y: y.unwrap_or(0.0),
            color: color.unwrap_or_default(),
            linestyle: linestyle.unwrap_or_else(|| "-".to_string()),
            linewidth: linewidth.unwrap_or(1.0),
            color_idx: idx,
        });
    }

    pub fn axvline(
        &mut self,
        x: Option<f64>,
        color: Option<String>,
        linestyle: Option<String>,
        linewidth: Option<f64>,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        self.elements.push(PlotElement::VLine {
            x: x.unwrap_or(0.0),
            color: color.unwrap_or_default(),
            linestyle: linestyle.unwrap_or_else(|| "-".to_string()),
            linewidth: linewidth.unwrap_or(1.0),
            color_idx: idx,
        });
    }

    #[pyo3(signature = (x, labels=None, colors=None, autopct=None, startangle=0.0))]
    pub fn pie(
        &mut self,
        x: Vec<f64>,
        labels: Option<Vec<String>>,
        colors: Option<Vec<String>>,
        autopct: Option<String>,
        startangle: f64,
    ) {
        self.elements.push(PlotElement::Pie {
            x,
            labels,
            colors,
            autopct,
            startangle,
        });
    }

    #[pyo3(signature = (x, y1, y2=None, color=None, alpha=0.3, label=None))]
    pub fn fill_between(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        y1: Bound<'_, PyAny>,
        y2: Option<Bound<'_, PyAny>>,
        color: Option<String>,
        alpha: f64,
        label: Option<String>,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let y1_vec = py_to_vec_f64(&y1)?;
        let idx = self.element_count;
        self.element_count += 1;
        // y2 可以是标量或向量，默认为 0.0
        let y2_vec: Vec<f64> = if let Some(y2_val) = y2 {
            if let Ok(scalar) = y2_val.extract::<f64>() {
                vec![scalar; x_vec.len()]
            } else if let Ok(vec) = py_to_vec_f64(&y2_val) {
                vec
            } else {
                vec![0.0; x_vec.len()]
            }
        } else {
            vec![0.0; x_vec.len()]
        };
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::FillBetween {
            x: x_vec,
            y1: y1_vec,
            y2: y2_vec,
            color: color_val.clone(),
            alpha,
            label: label.clone(),
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), None, 1.5));
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, yerr=None, xerr=None, fmt="o", color=None, label=None, capsize=3.0))]
    #[allow(clippy::too_many_arguments)]
    pub fn errorbar(
        &mut self,
        py: Python<'_>,
        x: Bound<'_, PyAny>,
        y: Bound<'_, PyAny>,
        yerr: Option<Py<PyAny>>,
        xerr: Option<Py<PyAny>>,
        fmt: &str,
        color: Option<String>,
        label: Option<String>,
        capsize: f64,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let y_vec = py_to_vec_f64(&y)?;
        // Convert possible scalar or sequence yerr/xerr into Vec<f64>
        let make_vec = |maybe: Option<Py<PyAny>>, n: usize| -> Option<Vec<f64>> {
            if let Some(obj) = maybe {
                if let Ok(v) = obj.extract::<Vec<f64>>(py) {
                    return Some(v);
                }
                if let Ok(v) = obj.extract::<f64>(py) {
                    return Some(vec![v; n]);
                }
            }
            None
        };

        let yerr_vec = make_vec(yerr, x_vec.len());
        let xerr_vec = make_vec(xerr, x_vec.len());

        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::ErrorBar {
            x: x_vec,
            y: y_vec,
            yerr: yerr_vec,
            xerr: xerr_vec,
            fmt: fmt.to_string(),
            color: color_val.clone(),
            label: label.clone(),
            capsize,
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), Some(fmt.to_string()), 1.5));
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, linefmt="-", markerfmt="o", label=None))]
    pub fn stem(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        y: Bound<'_, PyAny>,
        linefmt: &str,
        markerfmt: &str,
        label: Option<String>,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let y_vec = py_to_vec_f64(&y)?;
        let idx = self.element_count;
        self.element_count += 1;
        self.elements.push(PlotElement::Stem {
            x: x_vec,
            y: y_vec,
            linefmt: linefmt.to_string(),
            markerfmt: markerfmt.to_string(),
            label: label.clone(),
        });
        if let Some(lbl) = label {
            let col = default_color(idx);
            self.legend_labels.push((lbl, col, linefmt.to_string(), Some(markerfmt.to_string()), 1.5));
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, where_="pre", label=None, color=None, linestyle="-", linewidth=1.5))]
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        y: Bound<'_, PyAny>,
        where_: &str,
        label: Option<String>,
        color: Option<String>,
        linestyle: &str,
        linewidth: f64,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let y_vec = py_to_vec_f64(&y)?;
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::Step {
            x: x_vec,
            y: y_vec,
            where_: where_.to_string(),
            label: label.clone(),
            color: color_val,
            linestyle: linestyle.to_string(),
            linewidth,
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, linestyle.to_string(), None, linewidth));
        }
        Ok(())
    }

    #[pyo3(signature = (x, labels=None, vert=true))]
    pub fn boxplot(&mut self, _py: Python<'_>, x: Bound<'_, PyAny>, labels: Option<Vec<String>>, vert: bool) -> PyResult<()> {
        let data = py_to_vec_vec_f64(&x)?;
        self.elements.push(PlotElement::BoxPlot {
            data,
            labels,
            vert,
        });
        Ok(())
    }

    #[pyo3(signature = (text, xy, xytext=None, fontsize=12.0, color="black"))]
    pub fn annotate(
        &mut self,
        text: &str,
        xy: (f64, f64),
        xytext: Option<(f64, f64)>,
        fontsize: f64,
        color: &str,
    ) {
        self.elements.push(PlotElement::Annotate {
            text: text.to_string(),
            xy,
            xytext,
            fontsize,
            color: color.to_string(),
        });
    }

    pub fn set_xscale(&mut self, scale: &str) {
        self.xscale = scale.to_string();
    }

    pub fn set_yscale(&mut self, scale: &str) {
        self.yscale = scale.to_string();
    }

    #[pyo3(signature = (ticks=None, labels=None))]
    pub fn xticks(&mut self, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) {
        self.xticks_val = ticks;
        self.xtick_labels = labels;
    }

    #[pyo3(signature = (ticks=None, labels=None))]
    pub fn yticks(&mut self, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) {
        self.yticks_val = ticks;
        self.ytick_labels = labels;
    }

    pub fn twinx(&mut self) -> Axes {
        let mut twin = Axes::new();
        twin.xlim = self.xlim;
        twin.is_twin_x = true;
        self.twin_axes.push(twin.clone());
        twin
    }

    pub fn twiny(&mut self) -> Axes {
        let mut twin = Axes::new();
        twin.ylim = self.ylim;
        twin.is_twin_y = true;
        self.twin_axes.push(twin.clone());
        twin
    }

    pub fn cla(&mut self) {
        self.elements.clear();
        self.legend_labels.clear();
        self.element_count = 0;
    }

    #[pyo3(signature = (axis="both", labelsize=None, rotation=None, bottom=None, top=None, left=None, right=None))]
    #[allow(unused_variables)]
    pub fn tick_params(&mut self, axis: &str, labelsize: Option<f64>, rotation: Option<f64>, bottom: Option<bool>, top: Option<bool>, left: Option<bool>, right: Option<bool>) {
        if let Some(v) = labelsize { self.tick_labelsize = v; }
        if let Some(v) = bottom { self.tick_bottom = v; }
        if let Some(v) = top { self.tick_top = v; }
        if let Some(v) = left { self.tick_left = v; }
        if let Some(v) = right { self.tick_right = v; }
    }

    pub fn _axis_off(&mut self) {
        self.grid_visible = false;
        self.spine_top = false;
        self.spine_bottom = false;
        self.spine_left = false;
        self.spine_right = false;
        self.tick_bottom = false;
        self.tick_top = false;
        self.tick_left = false;
        self.tick_right = false;
    }

    /// matplotlib 兼容：启用次刻度（major + minor）
    pub fn minorticks_on(&mut self) {
        self.minor_grid_visible = true;
        self.minor_grid_x_visible = true;
        self.minor_grid_y_visible = true;
    }

    pub fn set_aspect(&mut self, _aspect: &str) {
    }

    pub fn set_xaxis_major_locator(&mut self, locator: Py<PyAny>) {
        self.xaxis_major_locator = Some(locator);
    }

    pub fn set_xaxis_minor_locator(&mut self, locator: Py<PyAny>) {
        self.xaxis_minor_locator = Some(locator);
    }

    pub fn set_yaxis_major_locator(&mut self, locator: Py<PyAny>) {
        self.yaxis_major_locator = Some(locator);
    }

    pub fn set_yaxis_minor_locator(&mut self, locator: Py<PyAny>) {
        self.yaxis_minor_locator = Some(locator);
    }

    pub fn set_facecolor(&mut self, color: &str) {
        self.facecolor = color.to_string();
    }

    #[getter]
    pub fn get_xaxis(&self, py: Python) -> PyResult<Py<Axis>> {
        let mut axis = Axis::new();
        axis.which = "x".to_string();
        axis.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Ok(Py::new(py, axis)?)
    }

    #[getter]
    pub fn get_yaxis(&self, py: Python) -> PyResult<Py<Axis>> {
        let mut axis = Axis::new();
        axis.which = "y".to_string();
        axis.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Ok(Py::new(py, axis)?)
    }

    #[getter]
    pub fn get_patch(&self, py: Python) -> PyResult<Py<Patch>> {
        let mut patch = Patch::new();
        patch.facecolor = self.facecolor.clone();
        patch.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Ok(Py::new(py, patch)?)
    }

    #[getter]
    pub fn get_spines(&self, py: Python) -> PyResult<Py<SpineDict>> {
        let mut sd = SpineDict::new();
        for spine in &mut sd.spines {
            match spine.name.as_str() {
                "top" => spine.visible = self.spine_top,
                "bottom" => spine.visible = self.spine_bottom,
                "left" => spine.visible = self.spine_left,
                "right" => spine.visible = self.spine_right,
                _ => {}
            }
        }
        sd.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Ok(Py::new(py, sd)?)
    }
}

impl Axes {
    pub fn compute_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let xlog = self.xscale == "log";
        let ylog = self.yscale == "log";
        crate::axes_bounds::compute_bounds(
            &self.elements, self.xlim, self.ylim, xlog, ylog,
        )
    }

    pub fn render<DB: DrawingBackend>(
        &self,
        py: Python<'_>,
        chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
        (x_min, x_max): (f64, f64),
        (y_min, y_max): (f64, f64),
        font_scale: f64,
        fill_bg: bool,
        _subplot_info: Option<&(f64, f64, f64, f64)>,
    ) -> PyResult<()>
    where
        DB::ErrorType: 'static,
    {
        // 仅主轴填充背景，twin axes 不填充以避免覆盖主轴数据
        if fill_bg {
            let bg_color = parse_color(&self.facecolor, 0).unwrap_or(RgbColor(255, 255, 255));
            chart.plotting_area().fill(&to_plotters_color(bg_color))
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to fill background: {}", e)))?;
        }

        // 在 chart 进入可变借用前，先取出绘图区像素尺寸（用于判断副刻度密度）
        let (plot_pixel_width, plot_pixel_height) = {
            let dim = chart.plotting_area().dim_in_pixel();
            (dim.0, dim.1)
        };

        let xlog = self.xscale == "log";
        let ylog = self.yscale == "log";

        // 计算主/副刻度
        let ticks_info = crate::axes_mesh::compute_ticks(
            py,
            &self.xticks_val,
            &self.yticks_val,
            &self.xaxis_major_locator,
            &self.yaxis_major_locator,
            &self.xaxis_minor_locator,
            &self.yaxis_minor_locator,
            x_min, x_max, y_min, y_max,
            plot_pixel_width, plot_pixel_height,
            self.minor_grid_x_visible, self.minor_grid_y_visible, self.minor_grid_visible,
        );

        // 计算网格线颜色/线宽/样式
        let grid_style = crate::axes_mesh::compute_grid_style(
            &self.grid_color, self.grid_linewidth, &self.grid_linestyle,
            &self.minor_grid_color, self.minor_grid_linewidth, &self.minor_grid_linestyle,
        );

        // 配置并绘制 mesh（与 ChartContext 的借用密切相关，必须内联）
        {
            let frame_color = parse_color(&self.spine_color, 0).unwrap_or(RgbColor(0, 0, 0));
            let frame_lw = self.spine_linewidth.round().max(1.0) as u32;
            let frame_style: ShapeStyle = to_plotters_color(frame_color).stroke_width(frame_lw).into();
            let label_size: f64 = scale_font(self.tick_labelsize, font_scale);
            let mut mesh_builder = chart.configure_mesh();
            mesh_builder
                .x_labels(ticks_info.xticks.len().max(2))
                .y_labels(ticks_info.yticks.len().max(2))
                .x_label_style(("sans-serif", label_size))
                .y_label_style(("sans-serif", label_size))
                .x_desc(self.xlabel.clone())
                .y_desc(self.ylabel.clone())
                .bold_line_style(frame_style);

            if xlog {
                mesh_builder.x_label_formatter(&|v| format!("{:.1e}", 10.0f64.powf(*v)));
            }
            if ylog {
                mesh_builder.y_label_formatter(&|v| format!("{:.1e}", 10.0f64.powf(*v)));
            } else {
                mesh_builder.y_label_formatter(&|v| crate::axes_mesh::format_linear_tick(*v));
                mesh_builder.x_label_formatter(&|v| crate::axes_mesh::format_linear_tick(*v));
            }

            if !self.spine_bottom && !self.spine_top {
                mesh_builder.disable_x_axis();
            }
            if !self.spine_left && !self.spine_right {
                mesh_builder.disable_y_axis();
            }
            if !self.tick_bottom && !self.tick_top {
                mesh_builder.x_labels(0);
            }
            if !self.tick_left && !self.tick_right {
                mesh_builder.y_labels(0);
            }

            // 手动绘制 mesh：禁用内置网格线（由 axes_grid 模块统一绘制）
            mesh_builder
                .disable_x_mesh()
                .disable_y_mesh()
                .draw()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw mesh: {}", e)))?;
        }

        // 手动绘制顶部和右侧 spine（plotters mesh 只绘制左侧和底部边框）
        {
            let spine_col = parse_color(&self.spine_color, 0).unwrap_or(RgbColor(0, 0, 0));
            let spine_rgb = to_plotters_color(spine_col);
            let spine_lw = self.spine_linewidth.round().max(1.0) as u32;
            let spine_style: ShapeStyle = spine_rgb.stroke_width(spine_lw).into();
            if self.spine_top {
                chart.draw_series(std::iter::once(PathElement::new(
                    vec![(x_min, y_max), (x_max, y_max)], spine_style,
                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw top spine: {}", e)))?;
            }
            if self.spine_right {
                chart.draw_series(std::iter::once(PathElement::new(
                    vec![(x_max, y_min), (x_max, y_max)], spine_style,
                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw right spine: {}", e)))?;
            }
        }

        // 绘制主网格线
        if self.grid_visible {
            let major_ls = grid_style.major_ls.as_deref();
            if self.grid_axis == "both" || self.grid_axis == "x" {
                crate::axes_grid::draw_grid_lines(
                    chart, true, &ticks_info.xticks,
                    grid_style.major_color, grid_style.major_lw, major_ls,
                    font_scale, x_min, x_max, y_min, y_max,
                )?;
            }
            if self.grid_axis == "both" || self.grid_axis == "y" {
                crate::axes_grid::draw_grid_lines(
                    chart, false, &ticks_info.yticks,
                    grid_style.major_color, grid_style.major_lw, major_ls,
                    font_scale, x_min, x_max, y_min, y_max,
                )?;
            }
        }

        // 绘制副网格线
        if self.minor_grid_visible {
            let minor_ls = grid_style.minor_ls.as_deref();
            // 过滤掉与主刻度位置重叠的副刻度，避免副网格线覆盖主网格线
            let xmin_filtered = ticks_info.xminor.as_ref().map(|minor| {
                crate::axes_grid::filter_minor_ticks(minor, &ticks_info.xticks)
            });
            let ymin_filtered = ticks_info.yminor.as_ref().map(|minor| {
                crate::axes_grid::filter_minor_ticks(minor, &ticks_info.yticks)
            });
            let show_x_minor = self.minor_grid_x_visible
                || (!self.minor_grid_x_visible && !self.minor_grid_y_visible);
            let show_y_minor = self.minor_grid_y_visible
                || (!self.minor_grid_x_visible && !self.minor_grid_y_visible);
            if show_x_minor {
                if let Some(ref ticks) = xmin_filtered {
                    crate::axes_grid::draw_grid_lines(
                        chart, true, ticks,
                        grid_style.minor_color, grid_style.minor_lw, minor_ls,
                        font_scale, x_min, x_max, y_min, y_max,
                    )?;
                }
            }
            if show_y_minor {
                if let Some(ref ticks) = ymin_filtered {
                    crate::axes_grid::draw_grid_lines(
                        chart, false, ticks,
                        grid_style.minor_color, grid_style.minor_lw, minor_ls,
                        font_scale, x_min, x_max, y_min, y_max,
                    )?;
                }
            }
        }

        // 渲染所有数据元素（线、散点、柱状图、填充、误差棒、饼图等）
        crate::axes_render_elements::render_elements(
            chart, &self.elements, font_scale, xlog, ylog,
            x_min, x_max, y_min, y_max,
        )?;

        if let Some(loc) = &self.legend_loc.clone() {
            if !self.legend_labels.is_empty() {
                crate::axes_legend::draw_legend(
                    chart, Some(loc), &self.legend_labels, font_scale,
                    x_min, x_max, y_min, y_max,
                )?;
            }
        }

        // 渲染 axes 标题（在数据区域上方的 margin_top 区域内）
        crate::axes_title::draw_title(
            chart, &self.title, self.title_fontsize, font_scale,
            x_min, x_max, y_min, y_max,
        )?;

        Ok(())
    }

    pub fn parse_hist_data(x: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<f64>>> {
        if let Ok(lst) = x.extract::<Vec<Bound<'_, PyAny>>>() {
            if lst.is_empty() {
                return Ok(Vec::new());
            }
            if let Ok(_) = lst[0].extract::<f64>() {
                let flat: Vec<f64> = lst.iter().map(|item| item.extract::<f64>())
                    .collect::<Result<Vec<f64>, _>>()
                    .map_err(|e| PyValueError::new_err(format!("hist data parse error: {}", e)))?;
                Ok(vec![flat])
            } else {
                let multi: Vec<Vec<f64>> = lst.iter().map(|item| {
                    item.extract::<Vec<f64>>()
                        .map_err(|e| PyValueError::new_err(format!("hist multi-data parse error: {}", e)))
                }).collect::<Result<Vec<Vec<f64>>, _>>()?;
                Ok(multi)
            }
        } else {
            Err(PyValueError::new_err("hist data must be a list or list of lists"))
        }
    }

    pub fn parse_color_list(color: &Bound<'_, PyAny>, expected_len: usize) -> PyResult<Vec<String>> {
        if let Ok(single) = color.extract::<String>() {
            Ok(vec![single; expected_len])
        } else if let Ok(lst) = color.extract::<Vec<String>>() {
            if lst.len() >= expected_len {
                Ok(lst[..expected_len].to_vec())
            } else {
                let mut result = lst.clone();
                while result.len() < expected_len {
                    result.push(default_color_str(result.len()));
                }
                Ok(result)
            }
        } else {
            Ok((0..expected_len).map(|i| default_color_str(i)).collect())
        }
    }
}