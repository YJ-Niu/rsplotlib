use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyAny};
use plotters::coord::types::RangedCoordf64;
use plotters::style::{ShapeStyle, text_anchor::{HPos, VPos, Pos}};
use plotters::prelude::*;

use crate::colors::{RgbColor, parse_color, default_color, default_color_str, shape_style, to_plotters_color};
use crate::elements::PlotElement;
use crate::marker::draw_marker;

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
use crate::colormap::{viridis_color, plasma_color, inferno_color, magma_color, cool_color, spring_color, summer_color, autumn_color, winter_color};
use crate::colors::median;
use crate::axis::{Axis, Patch, SpineDict};

/// 字体大小缩放并四舍五入到1位小数
fn scale_font(size: f64, font_scale: f64) -> f64 {
    (size * font_scale * 10.0).round() / 10.0
}

#[pyclass(skip_from_py_object)]
pub struct Axes {
    pub elements: Vec<PlotElement>,
    pub xlabel: String,
    pub ylabel: String,
    pub title: String,
    pub xlim: Option<(f64, f64)>,
    pub ylim: Option<(f64, f64)>,
    pub grid_visible: bool,
    pub legend_loc: Option<String>,
    pub element_count: usize,
    pub legend_labels: Vec<(String, RgbColor, String, Option<String>)>,
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

#[pymethods]
impl Axes {
    #[new]
    pub fn new() -> Self {
        Axes {
            elements: Vec::new(),
            xlabel: String::new(),
            ylabel: String::new(),
            title: String::new(),
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
        let x_vec = py_to_vec_option_f64(&x)?;
        let y_vec = py_to_vec_option_f64(&y)?;
        let color = c.or(color);
        let linewidth = lw.unwrap_or(linewidth);
        let linestyle = ls.as_deref().unwrap_or(linestyle);
        let idx = self.element_count;
        self.element_count += 1;
        // consume optional params to avoid unused variable warnings while preserving Python API
        let _ = markersize;
        let _ = markeredgewidth;
        let color_val = color.clone().unwrap_or_default();
        let linestyle_val = linestyle.to_string();
        self.elements.push(PlotElement::Line {
            x: x_vec,
            y: y_vec,
            label: label.clone(),
            color: color_val,
            linestyle: linestyle_val.clone(),
            marker,
            linewidth,
            color_idx: idx,
            solid_capstyle: solid_capstyle.unwrap_or_else(|| "butt".to_string()),
        });
        if let Some(lbl) = label {
            let c = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, c, linestyle_val, None));
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
            self.legend_labels.push((lbl, col, "-".to_string(), Some(marker_val)));
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
            self.legend_labels.push((lbl, col, "-".to_string(), None));
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
            self.legend_labels.push((lbl, col, "-".to_string(), None));
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
            self.legend_labels.push((lbl, col, "-".to_string(), None));
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

    #[pyo3(signature = (text, color=None))]
    pub fn set_title(&mut self, text: String, color: Option<String>) {
        let _ = color;
        self.title = text;
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

    #[pyo3(signature = (x, y, text, fontsize=None, color=None, c=None, _family=None))]
    pub fn text(
        &mut self,
        _py: Python<'_>,
        x: f64,
        y: f64,
        text: Bound<'_, PyAny>,
        fontsize: Option<i32>,
        color: Option<String>,
        c: Option<String>,
        _family: Option<String>,
    ) {
        let color = c.or(color);
        let text_str: String = text.extract::<String>().unwrap_or_else(|_| {
            text.str().map(|s| s.to_string()).unwrap_or_default()
        });
        let col = parse_color(&color.unwrap_or_else(|| "black".to_string()), 0).unwrap_or(RgbColor(0, 0, 0));
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
            self.legend_labels.push((lbl, col, "-".to_string(), None));
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
            self.legend_labels.push((lbl, col, "-".to_string(), Some(fmt.to_string())));
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
            self.legend_labels.push((lbl, col, linefmt.to_string(), Some(markerfmt.to_string())));
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
            self.legend_labels.push((lbl, col, linestyle.to_string(), None));
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
    /// 对 log 刻度轴的数据值进行 log10 转换
    fn log_transform(val: f64) -> f64 {
        if val > 0.0 { val.log10() } else { f64::NEG_INFINITY }
    }

    pub fn compute_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        let xlog = self.xscale == "log";
        let ylog = self.yscale == "log";

        let tx = |v: f64| if xlog { Self::log_transform(v) } else { v };
        let ty = |v: f64| if ylog { Self::log_transform(v) } else { v };

        for el in &self.elements {
            match el {
                PlotElement::Line { x, y, .. } => {
                    for v in x.iter().flatten() {
                        let tv = tx(*v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    for v in y.iter().flatten() {
                        let tv = ty(*v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                }
                PlotElement::Scatter { x, y, .. } => {
                    for &v in x {
                        let tv = tx(v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    for &v in y {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                }
                PlotElement::Bar { x, height, width, .. } => {
                    for &v in x {
                        let tv = tx(v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    for &v in height {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                    if !x.is_empty() && !height.is_empty() {
                        let last_x = tx(x[x.len() - 1]);
                        let bar_end = last_x + *width;
                        if bar_end > x_max { x_max = bar_end; }
                    }
                    if !ylog && y_min > 0.0 { y_min = 0.0; }
                }
                PlotElement::BarH { y, width, .. } => {
                    for &v in y {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                    for &v in width {
                        let tv = tx(v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    if !width.is_empty() {
                        let last_w = tx(width[width.len() - 1]);
                        if last_w > x_max { x_max = last_w; }
                    }
                    if !xlog && x_min > 0.0 { x_min = 0.0; }
                }
                PlotElement::Hist { data_all, bins, density, bin_edges, .. } => {
                    if data_all.is_empty() { continue; }
                    let all_data: Vec<f64> = data_all.iter().flatten().cloned().collect();
                    if all_data.is_empty() { continue; }
                    let data_min = all_data.iter().cloned().fold(f64::INFINITY, f64::min);
                    let data_max = all_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let (x_start, x_end) = if let Some(edges) = bin_edges {
                        (edges[0], edges[edges.len() - 1])
                    } else {
                        (data_min, data_max)
                    };
                    let tx_start = tx(x_start);
                    let tx_end = tx(x_end);
                    if tx_start > f64::NEG_INFINITY && tx_start < x_min { x_min = tx_start; }
                    if tx_end > x_max { x_max = tx_end; }
                    let total = all_data.len() as f64;
                    let mut max_count = 0.0f64;
                    for dataset in data_all {
                        if dataset.is_empty() { continue; }
                        let d_min = dataset.iter().cloned().fold(f64::INFINITY, f64::min);
                        let d_max = dataset.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                        let d_range = d_max - d_min;
                        if d_range < 1e-10 { continue; }
                        let bw = d_range / *bins as f64;
                        let mut counts = vec![0usize; *bins];
                        for &val in dataset {
                            let mut bin = ((val - d_min) / bw).floor() as usize;
                            if bin >= *bins { bin = *bins - 1; }
                            counts[bin] += 1;
                        }
                        let mc = counts.iter().max().unwrap_or(&0);
                        let dc = if *density { *mc as f64 / (total * bw) } else { *mc as f64 };
                        if dc > max_count { max_count = dc; }
                    }
                    if !ylog && y_min > 0.0 { y_min = 0.0; }
                    let tmax = ty(max_count);
                    if tmax > y_max { y_max = tmax; }
                }
                PlotElement::Image { data, .. } => {
                    if data.is_empty() || data[0].is_empty() { continue; }
                    x_min = 0.0;
                    x_max = data[0].len() as f64;
                    y_min = 0.0;
                    y_max = data.len() as f64;
                }
                PlotElement::Text { x, y, .. } => {
                    let tvx = tx(*x);
                    let tvy = ty(*y);
                    if tvx > f64::NEG_INFINITY && tvx < x_min { x_min = tvx; }
                    if tvx > x_max { x_max = tvx; }
                    if tvy > f64::NEG_INFINITY && tvy < y_min { y_min = tvy; }
                    if tvy > y_max { y_max = tvy; }
                }
                PlotElement::HLine { y, .. } => {
                    if x_min == f64::INFINITY { x_min = -1.0; x_max = 1.0; }
                    let tvy = ty(*y);
                    if tvy > f64::NEG_INFINITY && tvy < y_min { y_min = tvy; }
                    if tvy > y_max { y_max = tvy; }
                }
                PlotElement::VLine { x, .. } => {
                    if y_min == f64::INFINITY { y_min = -1.0; y_max = 1.0; }
                    let tvx = tx(*x);
                    if tvx > f64::NEG_INFINITY && tvx < x_min { x_min = tvx; }
                    if tvx > x_max { x_max = tvx; }
                }
                PlotElement::Pie { .. } => {
                    if x_min > -1.5 { x_min = -1.5; }
                    if x_max < 1.5 { x_max = 1.5; }
                    if y_min > -1.5 { y_min = -1.5; }
                    if y_max < 1.5 { y_max = 1.5; }
                }
                PlotElement::FillBetween { x, y1, y2, .. } => {
                    for &v in x {
                        let tv = tx(v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    for &v in y1 {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                    for &v in y2 {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                }
                PlotElement::ErrorBar { x, y, yerr, .. } => {
                    for &v in x {
                        let tv = tx(v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    for &v in y {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                    if let Some(ye_vec) = yerr.as_ref() {
                        for (i, &yv) in y.iter().enumerate() {
                            let ye = if i < ye_vec.len() { ye_vec[i] } else { 0.0_f64 };
                            let tv_lo = ty(yv - ye);
                            let tv_hi = ty(yv + ye);
                            if tv_lo > f64::NEG_INFINITY && tv_lo < y_min { y_min = tv_lo; }
                            if tv_hi > y_max { y_max = tv_hi; }
                        }
                    }
                }
                PlotElement::Stem { x, y, .. } => {
                    for &v in x {
                        let tv = tx(v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    for &v in y {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                    if !ylog && y_min > 0.0 { y_min = 0.0; }
                }
                PlotElement::Step { x, y, .. } => {
                    for &v in x {
                        let tv = tx(v);
                        if tv > f64::NEG_INFINITY && tv < x_min { x_min = tv; }
                        if tv > x_max { x_max = tv; }
                    }
                    for &v in y {
                        let tv = ty(v);
                        if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                        if tv > y_max { y_max = tv; }
                    }
                }
                PlotElement::BoxPlot { data, .. } => {
                    for series in data {
                        for &v in series {
                            let tv = ty(v);
                            if tv > f64::NEG_INFINITY && tv < y_min { y_min = tv; }
                            if tv > y_max { y_max = tv; }
                        }
                    }
                    if !ylog && y_min > 0.0 { y_min = 0.0; }
                    if !xlog && x_min > 0.0 { x_min = 0.0; }
                    let n = data.len() as f64;
                    if n > x_max { x_max = n + 1.0; }
                }
                PlotElement::Annotate { xy, xytext, .. } => {
                    let (xv, yv) = *xy;
                    let tvx = tx(xv);
                    let tvy = ty(yv);
                    if tvx > f64::NEG_INFINITY && tvx < x_min { x_min = tvx; }
                    if tvx > x_max { x_max = tvx; }
                    if tvy > f64::NEG_INFINITY && tvy < y_min { y_min = tvy; }
                    if tvy > y_max { y_max = tvy; }
                    if let Some((xt, yt)) = xytext {
                        let tvxt = tx(*xt);
                        let tvyt = ty(*yt);
                        if tvxt > f64::NEG_INFINITY && tvxt < x_min { x_min = tvxt; }
                        if tvxt > x_max { x_max = tvxt; }
                        if tvyt > f64::NEG_INFINITY && tvyt < y_min { y_min = tvyt; }
                        if tvyt > y_max { y_max = tvyt; }
                    }
                }
            }
        }

        if x_min == f64::INFINITY { x_min = 0.0; x_max = 1.0; }
        if y_min == f64::INFINITY { y_min = 0.0; y_max = 1.0; }

        let x_range = x_max - x_min;
        let y_range = y_max - y_min;
        let x_pad = if x_range.abs() < 1e-10 { 1.0 } else { x_range * 0.05 };
        let y_pad = if y_range.abs() < 1e-10 { 1.0 } else { y_range * 0.05 };

        if let Some((l, r)) = self.xlim {
            x_min = l;
            x_max = r;
        } else {
            x_min -= x_pad;
            x_max += x_pad;
        }
        if let Some((b, t)) = self.ylim {
            y_min = b;
            y_max = t;
        } else {
            y_min -= y_pad;
            y_max += y_pad;
        }

        ((x_min, x_max), (y_min, y_max))
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

        let mut mesh_builder = chart.configure_mesh();

        let frame_color = parse_color(&self.spine_color, 0).unwrap_or(RgbColor(0, 0, 0));
        let frame_lw = self.spine_linewidth.round().max(1.0) as u32;
        let frame_style: ShapeStyle = to_plotters_color(frame_color).stroke_width(frame_lw).into();

        // 自动计算主tick位置（matplotlib兼容）：使用MaxNLocator的"漂亮"算法
        fn nice_ticks(min: f64, max: f64) -> Vec<f64> {
            if min >= max || !min.is_finite() || !max.is_finite() {
                return vec![min, max];
            }
            let range = max - min;
            if range <= 0.0 { return vec![min]; }
            // 选择合适的步长（matplotlib的MaxNLocator简化版）
            let rough = range / 7.0;
            let mag = 10f64.powf(rough.log10().floor());
            let norm = rough / mag;
            let step = if norm < 1.5 { mag } else if norm < 3.0 { 2.0 * mag } else if norm < 7.0 { 5.0 * mag } else { 10.0 * mag };
            let start = (min / step).ceil() * step;
            let end = (max / step).floor() * step;
            let mut ticks = Vec::new();
            let mut t = start;
            while t <= end + step * 0.001 {
                ticks.push(t);
                t += step;
            }
            if ticks.is_empty() { ticks.push(min); }
            ticks
        }

        let computed_xticks: Option<Vec<f64>> = self.xaxis_major_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (x_min, x_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| self.xticks_val.clone()).or_else(|| Some(nice_ticks(x_min, x_max)));

        let computed_yticks: Option<Vec<f64>> = self.yaxis_major_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (y_min, y_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| self.yticks_val.clone()).or_else(|| Some(nice_ticks(y_min, y_max)));

        const MAX_MAJOR_TICKS_FOR_MINOR: usize = 30;
        const MAX_MINOR_TICKS: usize = 100;

        let should_compute_x_minor = self.minor_grid_x_visible || (!self.minor_grid_x_visible && !self.minor_grid_y_visible && self.minor_grid_visible);
        let computed_xminor: Option<Vec<f64>> = self.xaxis_minor_locator.as_ref().and_then(|locator| {
            locator.bind(py).call_method1("tick_values", (x_min, x_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).filter(|ticks| ticks.len() <= MAX_MINOR_TICKS).or_else(|| {
            if should_compute_x_minor {
                computed_xticks.as_ref().and_then(|major_ticks| {
                    if major_ticks.len() < 2 || major_ticks.len() > MAX_MAJOR_TICKS_FOR_MINOR { return None; }
                    let mut minor = Vec::new();
                    for i in 0..major_ticks.len().saturating_sub(1) {
                        let spacing = major_ticks[i + 1] - major_ticks[i];
                        if spacing <= 0.0 { continue; }
                        let step = spacing / 4.0;
                        let mut v = major_ticks[i] + step;
                        while v < major_ticks[i + 1] - step * 0.5 {
                            if v > x_min && v < x_max {
                                minor.push(v);
                            }
                            v += step;
                        }
                    }
                    if minor.is_empty() || minor.len() > MAX_MINOR_TICKS { None } else { Some(minor) }
                })
            } else {
                None
            }
        });

        let should_compute_y_minor = self.minor_grid_y_visible || (!self.minor_grid_x_visible && !self.minor_grid_y_visible && self.minor_grid_visible);
        let computed_yminor: Option<Vec<f64>> = self.yaxis_minor_locator.as_ref().and_then(|locator| {
            locator.bind(py).call_method1("tick_values", (y_min, y_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).filter(|ticks| ticks.len() <= MAX_MINOR_TICKS).or_else(|| {
            if should_compute_y_minor {
                computed_yticks.as_ref().and_then(|major_ticks| {
                    if major_ticks.len() < 2 || major_ticks.len() > MAX_MAJOR_TICKS_FOR_MINOR { return None; }
                    let mut minor = Vec::new();
                    for i in 0..major_ticks.len().saturating_sub(1) {
                        let spacing = major_ticks[i + 1] - major_ticks[i];
                        if spacing <= 0.0 { continue; }
                        let step = spacing / 4.0;
                        let mut v = major_ticks[i] + step;
                        while v < major_ticks[i + 1] - step * 0.5 {
                            if v > y_min && v < y_max {
                                minor.push(v);
                            }
                            v += step;
                        }
                    }
                    if minor.is_empty() || minor.len() > MAX_MINOR_TICKS { None } else { Some(minor) }
                })
            } else {
                None
            }
        });

        let x_label_count = computed_xticks.as_ref().map(|t| t.len()).unwrap_or(10).max(1);
        let y_label_count = computed_yticks.as_ref().map(|t| t.len()).unwrap_or(10).max(1);

        let major_color = if let Some(ref c) = self.grid_color {
            parse_color(c, 0).unwrap_or(RgbColor(200, 200, 200))
        } else {
            RgbColor(128, 128, 128)
        };
        let major_lw_f64 = self.grid_linewidth.unwrap_or(0.8);
        
        let minor_color = if let Some(ref c) = self.minor_grid_color {
            parse_color(c, 0).unwrap_or(RgbColor(230, 230, 230))
        } else {
            RgbColor(120, 122, 120)
        };
        let minor_lw_f64 = self.minor_grid_linewidth.unwrap_or(0.4);

        let label_size: f64 = scale_font(self.tick_labelsize, font_scale);
        mesh_builder
            .x_labels(x_label_count.max(2))
            .y_labels(y_label_count.max(2))
            .x_label_style(("sans-serif", label_size))
            .y_label_style(("sans-serif", label_size))
            .x_desc(self.xlabel.clone())
            .y_desc(self.ylabel.clone())
            .bold_line_style(frame_style);

        if self.xscale == "log" {
            mesh_builder
                .x_label_formatter(&|v| format!("{:.1e}", 10.0f64.powf(*v)));
        }
        if self.yscale == "log" {
            mesh_builder
                .y_label_formatter(&|v| format!("{:.1e}", 10.0f64.powf(*v)));
        } else {
            // 对线性刻度使用与 matplotlib 兼容的格式：
            // 整数显示为不带 ".0"，小数保留最多两位有效数字。
            mesh_builder.y_label_formatter(&|v| {
                let val = *v;
                if (val - val.round()).abs() < 1e-9 {
                    format!("{}", val.round() as i64)
                } else {
                    let s = format!("{:.2}", val);
                    // 去掉末尾的 0 和可能的 .
                    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                    trimmed.to_string()
                }
            });
            mesh_builder.x_label_formatter(&|v| {
                let val = *v;
                if (val - val.round()).abs() < 1e-9 {
                    format!("{}", val.round() as i64)
                } else {
                    let s = format!("{:.2}", val);
                    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                    trimmed.to_string()
                }
            });
        }

        if let Some(ref ticks) = computed_xticks {
            mesh_builder.x_labels(ticks.len().max(1));
        }
        if let Some(ref ticks) = computed_yticks {
            mesh_builder.y_labels(ticks.len().max(1));
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

        // 绘制 mesh（刻度标签、轴标签、边框）
        // disable_x_mesh/disable_y_mesh 禁用内置网格线，保留 tick 标签和轴边框
        // 我们手动绘制网格线以支持虚线/点线等样式
        mesh_builder
            .disable_x_mesh()
            .disable_y_mesh()
            .draw()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw mesh: {}", e)))?;

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

        let draw_grid_lines = |chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
                              vertical: bool, ticks: &[f64],
                              color: RgbColor, lw: f64, ls: Option<&str>| -> PyResult<()> {
            let rgb = to_plotters_color(color);
            let lw_u32 = if lw < 0.5 { 1 } else { lw.round() as u32 };
            let style = rgb.stroke_width(lw_u32);

            let mut paths: Vec<Vec<(f64, f64)>> = Vec::new();
            for &tick in ticks {
                if vertical {
                    if tick >= x_min && tick <= x_max {
                        paths.push(vec![(tick, y_min), (tick, y_max)]);
                    }
                } else {
                    if tick >= y_min && tick <= y_max {
                        paths.push(vec![(x_min, tick), (x_max, tick)]);
                    }
                }
            }

            match ls {
                Some("--") => {
                    // 虚线网格：每条网格线用dash模式绘制
                    let dash_len = lw_u32 as f64 * 4.0;
                    let gap_len = lw_u32 as f64 * 2.0;
                    for path in &paths {
                        if path.len() >= 2 {
                            let dx = path[1].0 - path[0].0;
                            let dy = path[1].1 - path[0].1;
                            let total_len = (dx * dx + dy * dy).sqrt();
                            let unit_x = dx / total_len;
                            let unit_y = dy / total_len;
                            let mut pos = 0.0f64;
                            let mut drawing = true;
                            while pos < total_len {
                                let seg_len = if drawing { dash_len } else { gap_len };
                                let end_pos = (pos + seg_len).min(total_len);
                                if drawing {
                                    let p1 = (path[0].0 + unit_x * pos, path[0].1 + unit_y * pos);
                                    let p2 = (path[0].0 + unit_x * end_pos, path[0].1 + unit_y * end_pos);
                                    chart.draw_series(std::iter::once(PathElement::new(
                                        vec![p1, p2], style,
                                    ))).map_err(|e| PyRuntimeError::new_err(format!("Dashed grid: {}", e)))?;
                                }
                                pos = end_pos;
                                drawing = !drawing;
                            }
                        }
                    }
                }
                Some(":") => {
                    // 点线网格
                    let dot_len = lw_u32 as f64 * 1.0;
                    let gap_len = lw_u32 as f64 * 2.0;
                    for path in &paths {
                        if path.len() >= 2 {
                            let dx = path[1].0 - path[0].0;
                            let dy = path[1].1 - path[0].1;
                            let total_len = (dx * dx + dy * dy).sqrt();
                            let unit_x = dx / total_len;
                            let unit_y = dy / total_len;
                            let mut pos = 0.0f64;
                            let mut drawing = true;
                            while pos < total_len {
                                let seg_len = if drawing { dot_len } else { gap_len };
                                let end_pos = (pos + seg_len).min(total_len);
                                if drawing {
                                    let p1 = (path[0].0 + unit_x * pos, path[0].1 + unit_y * pos);
                                    let p2 = (path[0].0 + unit_x * end_pos, path[0].1 + unit_y * end_pos);
                                    chart.draw_series(std::iter::once(PathElement::new(
                                        vec![p1, p2], style,
                                    ))).map_err(|e| PyRuntimeError::new_err(format!("Dotted grid: {}", e)))?;
                                }
                                pos = end_pos;
                                drawing = !drawing;
                            }
                        }
                    }
                }
                _ => {
                    // 实线网格
                    for path in paths {
                        chart.draw_series(std::iter::once(PathElement::new(path, style)))
                            .map_err(|e| PyRuntimeError::new_err(format!("Grid line: {}", e)))?;
                    }
                }
            }
            Ok(())
        };

        let draw_single_line = |chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
                               x1: f64, y1: f64, x2: f64, y2: f64,
                               color: RgbColor, lw: f64| -> PyResult<()> {
            let rgb = to_plotters_color(color);
            let lw_u32 = if lw < 0.5 { 1 } else { lw.round() as u32 };
            let style = rgb.stroke_width(lw_u32);
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x1, y1), (x2, y2)], style,
            ))).map_err(|e| PyRuntimeError::new_err(format!("Line: {}", e)))?;
            Ok(())
        };

        if self.grid_visible {
            let major_ls = self.grid_linestyle.as_deref();
            if self.grid_axis == "both" || self.grid_axis == "x" {
                if let Some(ref ticks) = computed_xticks {
                    draw_grid_lines(chart, true, ticks, major_color, major_lw_f64, major_ls)?;
                }
            }
            if self.grid_axis == "both" || self.grid_axis == "y" {
                if let Some(ref ticks) = computed_yticks {
                    draw_grid_lines(chart, false, ticks, major_color, major_lw_f64, major_ls)?;
                }
            }
        }

        if self.minor_grid_visible {
            let minor_ls = self.minor_grid_linestyle.as_deref();
            if self.minor_grid_x_visible || (!self.minor_grid_x_visible && !self.minor_grid_y_visible) {
                if let Some(ref ticks) = computed_xminor {
                    draw_grid_lines(chart, true, ticks, minor_color, minor_lw_f64, minor_ls)?;
                }
            }
            if self.minor_grid_y_visible || (!self.minor_grid_x_visible && !self.minor_grid_y_visible) {
                if let Some(ref ticks) = computed_yminor {
                    draw_grid_lines(chart, false, ticks, minor_color, minor_lw_f64, minor_ls)?;
                }
            }
        }

        // log 刻度坐标转换闭包
        let xlog = self.xscale == "log";
        let ylog = self.yscale == "log";
        let tx = |v: f64| if xlog { if v > 0.0 { v.log10() } else { f64::NEG_INFINITY } } else { v };
        let ty = |v: f64| if ylog { if v > 0.0 { v.log10() } else { f64::NEG_INFINITY } } else { v };

        for el in &self.elements {
            match el {
                PlotElement::Line { x, y, color, linestyle, marker, linewidth, color_idx, solid_capstyle, .. } => {
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| default_color(*color_idx));
                    if x.len() >= 2 && x.len() == y.len() {
                        let points: Vec<(f64, f64)> = x.iter().zip(y.iter())
                            .filter_map(|(xv, yv)| match (xv, yv) {
                                (Some(xv), Some(yv)) => {
                                    let txv = tx(*xv);
                                    let tyv = ty(*yv);
                                    if txv.is_finite() && tyv.is_finite() { Some((txv, tyv)) } else { None }
                                }
                                _ => None,
                            })
                            .collect();
                        if points.len() >= 2 {
                            let rgb = to_plotters_color(col);
                            let style = shape_style(col, *linewidth, linestyle);
                            // 对于虚线样式，使用分段绘制模拟
                            if linestyle == "--" {
                                let dash_len = *linewidth * 4.0;
                                let gap_len = *linewidth * 2.0;
                                let mut seg_start = 0usize;
                                while seg_start < points.len() - 1 {
                                    let mut seg_end = seg_start + 1;
                                    let mut acc_dist = 0.0;
                                    while seg_end < points.len() {
                                        let dx = points[seg_end].0 - points[seg_end - 1].0;
                                        let dy = points[seg_end].1 - points[seg_end - 1].1;
                                        acc_dist += (dx * dx + dy * dy).sqrt();
                                        if acc_dist >= dash_len + gap_len { break; }
                                        seg_end += 1;
                                    }
                                    // 绘制dash段（前dash_len长度）
                                    let mut dash_points = Vec::new();
                                    dash_points.push(points[seg_start]);
                                    let mut dist = 0.0;
                                    for i in seg_start..seg_end.min(points.len() - 1) {
                                        let dx = points[i + 1].0 - points[i].0;
                                        let dy = points[i + 1].1 - points[i].1;
                                        let seg_len = (dx * dx + dy * dy).sqrt();
                                        if dist + seg_len <= dash_len {
                                            dash_points.push(points[i + 1]);
                                            dist += seg_len;
                                        } else {
                                            let remain = dash_len - dist;
                                            let t = remain / seg_len;
                                            dash_points.push((points[i].0 + dx * t, points[i].1 + dy * t));
                                            break;
                                        }
                                    }
                                    if dash_points.len() >= 2 {
                                        let lw_px = ((*linewidth) * font_scale).round().max(1.0) as u32;
                                        chart.draw_series(std::iter::once(PathElement::new(dash_points, rgb.stroke_width(lw_px))))
                                            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw dashed line: {}", e)))?;
                                    }
                                    seg_start = seg_end.max(seg_start + 1);
                                }
                            } else if linestyle == ":" {
                                // 点线：沿路径绘制短点段
                                let dot_len = *linewidth * 1.0;
                                let gap_len = *linewidth * 2.0;
                                let mut seg_idx = 0usize;
                                let mut pos_in_seg = 0.0f64;
                                while seg_idx < points.len() - 1 {
                                    let dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                    let dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                    let seg_len = (dx * dx + dy * dy).sqrt();
                                    if seg_len < 1e-10 {
                                        seg_idx += 1;
                                        pos_in_seg = 0.0;
                                        continue;
                                    }
                                    let unit_x = dx / seg_len;
                                    let unit_y = dy / seg_len;
                                    // 绘制一个点
                                    let dot_start = pos_in_seg;
                                    let dot_end = (pos_in_seg + dot_len).min(seg_len);
                                    let p1 = (points[seg_idx].0 + unit_x * dot_start,
                                              points[seg_idx].1 + unit_y * dot_start);
                                    let p2 = (points[seg_idx].0 + unit_x * dot_end,
                                              points[seg_idx].1 + unit_y * dot_end);
                                    chart.draw_series(std::iter::once(PathElement::new(
                                        vec![p1, p2], rgb.stroke_width(*linewidth as u32))))
                                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw dotted line: {}", e)))?;
                                    // 跳过间隙
                                    let gap_end = dot_end + gap_len;
                                    if gap_end < seg_len {
                                        pos_in_seg = gap_end;
                                    } else {
                                        // 间隙跨越到下一段
                                        let mut remaining_gap = gap_end - seg_len;
                                        seg_idx += 1;
                                        pos_in_seg = 0.0;
                                        while seg_idx < points.len() - 1 && remaining_gap > 0.0 {
                                            let next_dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                            let next_dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                            let next_len = (next_dx * next_dx + next_dy * next_dy).sqrt();
                                            if remaining_gap < next_len {
                                                pos_in_seg = remaining_gap;
                                                remaining_gap = 0.0;
                                            } else {
                                                remaining_gap -= next_len;
                                                seg_idx += 1;
                                                pos_in_seg = 0.0;
                                            }
                                        }
                                    }
                                }
                            } else if linestyle == "-." {
                                // 点划线：交替绘制长划和短点
                                let dash_len = *linewidth * 6.0;
                                let dot_len = *linewidth * 1.0;
                                let gap_len = *linewidth * 2.0;
                                let mut seg_idx = 0usize;
                                let mut pos_in_seg = 0.0f64;
                                let mut is_dash = true; // 交替 dash/dot
                                while seg_idx < points.len() - 1 {
                                    let dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                    let dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                    let seg_len = (dx * dx + dy * dy).sqrt();
                                    if seg_len < 1e-10 {
                                        seg_idx += 1;
                                        pos_in_seg = 0.0;
                                        continue;
                                    }
                                    let unit_x = dx / seg_len;
                                    let unit_y = dy / seg_len;
                                    let mark_len = if is_dash { dash_len } else { dot_len };
                                    let mark_start = pos_in_seg;
                                    let mark_end = (pos_in_seg + mark_len).min(seg_len);
                                    let p1 = (points[seg_idx].0 + unit_x * mark_start,
                                              points[seg_idx].1 + unit_y * mark_start);
                                    let p2 = (points[seg_idx].0 + unit_x * mark_end,
                                              points[seg_idx].1 + unit_y * mark_end);
                                    chart.draw_series(std::iter::once(PathElement::new(
                                        vec![p1, p2], rgb.stroke_width(*linewidth as u32))))
                                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw dash-dot line: {}", e)))?;
                                    // 跳过间隙
                                    let gap_end = mark_end + gap_len;
                                    is_dash = !is_dash;
                                    if gap_end < seg_len {
                                        pos_in_seg = gap_end;
                                    } else {
                                        let mut remaining_gap = gap_end - seg_len;
                                        seg_idx += 1;
                                        pos_in_seg = 0.0;
                                        while seg_idx < points.len() - 1 && remaining_gap > 0.0 {
                                            let next_dx = points[seg_idx + 1].0 - points[seg_idx].0;
                                            let next_dy = points[seg_idx + 1].1 - points[seg_idx].1;
                                            let next_len = (next_dx * next_dx + next_dy * next_dy).sqrt();
                                            if remaining_gap < next_len {
                                                pos_in_seg = remaining_gap;
                                                remaining_gap = 0.0;
                                            } else {
                                                remaining_gap -= next_len;
                                                seg_idx += 1;
                                                pos_in_seg = 0.0;
                                            }
                                        }
                                    }
                                }
                            } else {
                                // 实线或其他样式
                                let path = PathElement::new(points.clone(), style);
                                chart.draw_series(std::iter::once(path))
                                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw line: {}", e)))?;
                            }
                            if solid_capstyle == "round" && *linewidth > 1.0 && marker.as_ref().map_or(true, |m| m.is_empty()) {
                                // 使用屏幕像素半径（参考 marker "o" 的实现），避免在数据坐标下变成巨大椭圆
                                let cap_r = ((*linewidth / 2.0) as i32).max(1);
                                let cap_points = [points.first().unwrap().clone(), points.last().unwrap().clone()];
                                for pt in cap_points.iter() {
                                    chart.draw_series(std::iter::once(Circle::new(*pt, cap_r, rgb.filled())))
                                        .map_err(|e| PyRuntimeError::new_err(format!("Cap circle: {}", e)))?;
                                }
                            }
                        }
                    }
                    if let Some(marker_name) = marker {
                        if !marker_name.is_empty() && x.len() == y.len() {
                            let col2 = parse_color(color, *color_idx).unwrap_or_else(|_| default_color(*color_idx));
                            let rgb = to_plotters_color(col2);
                            // matplotlib 中 markersize 的单位是 "points^2"，半径约为 sqrt(s)/2 像素（@72dpi）
                            // 我们的 line plot 入口没有暴露 markersize，所以按 markersize=6（matplotlib 默认）
                            // 在 144dpi 下半径约 6 像素。
                            // 对于 "." 这种像素点 marker，matplotlib 实际只画 1 个像素，需要更小。
                            let marker_size = if marker_name == "." || marker_name == "," {
                                2.0_f64.max(((*linewidth) * 1.5).round())
                            } else {
                                // 半径 = sqrt(6) * dpi/72 ≈ 6 在 144dpi 下
                                let ms_points = 6.0_f64;
                                ms_points * (font_scale * 144.0 / 72.0) / 2.0
                            };
                            for (xv, yv) in x.iter().zip(y.iter()) {
                                if let (Some(xv), Some(yv)) = (xv, yv) {
                                    draw_marker(chart, marker_name, *xv, *yv, marker_size, rgb)
                                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw marker: {}", e)))?;
                                }
                            }
                        }
                    }
                }
                PlotElement::Scatter { x, y, s, c, marker, color_idx, .. } => {
                    let col = parse_color(c, *color_idx).unwrap_or_else(|_| default_color(*color_idx));
                    let rgb = to_plotters_color(col);
                    let size = s.sqrt() * 0.4;
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if txv.is_finite() && tyv.is_finite() {
                            draw_marker(chart, marker, txv, tyv, size.max(2.0), rgb)
                                .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw scatter: {}", e)))?;
                        }
                    }
                }
                PlotElement::Bar { x, height, width, color, color_idx, .. } => {
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| default_color(*color_idx));
                    let rgb = to_plotters_color(col);
                    let fill_style: ShapeStyle = rgb.filled().into();
                    for (&xv, &h) in x.iter().zip(height.iter()) {
                        let txv = tx(xv);
                        let th = ty(h);
                        let y0 = if ylog { f64::NEG_INFINITY } else { 0.0f64.max(y_min) };
                        if txv.is_finite() && th.is_finite() {
                            chart.draw_series(std::iter::once(Rectangle::new(
                                [(txv - width / 2.0, y0), (txv + width / 2.0, th)],
                                fill_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw bar: {}", e)))?;
                        }
                    }
                }
                PlotElement::BarH { y, width, height, color, color_idx, .. } => {
                    let c = if color.is_empty() { default_color(*color_idx) } else { parse_color(color, *color_idx)? };
                    let rgb = to_plotters_color(c);
                    let fill_style: ShapeStyle = rgb.filled().into();
                    for (&yv, &wv) in y.iter().zip(width.iter()) {
                        let tyv = ty(yv);
                        let twv = tx(wv);
                        let bar_y0 = tyv - height / 2.0;
                        let bar_y1 = tyv + height / 2.0;
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(0.0, bar_y0), (twv, bar_y1)],
                            fill_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw barh: {}", e)))?;
                    }
                }
                PlotElement::Hist { data_all, bins, density, histtype, alpha, colors, color_idx, bin_edges, label: _ } => {
                    if data_all.is_empty() { continue; }
                    let all_data: Vec<f64> = data_all.iter().flatten().cloned().collect();
                    if all_data.is_empty() { continue; }
                    let (global_min, global_max) = if let Some(edges) = bin_edges {
                        (edges[0], edges[edges.len() - 1])
                    } else {
                        let mn = all_data.iter().cloned().fold(f64::INFINITY, f64::min);
                        let mx = all_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                        (mn, mx)
                    };
                    let global_range = global_max - global_min;
                    if global_range < 1e-10 { continue; }
                    let bin_edges_list: Vec<f64> = if let Some(edges) = bin_edges {
                        edges.clone()
                    } else {
                        let bw = global_range / *bins as f64;
                        (0..=*bins).map(|i| global_min + i as f64 * bw).collect()
                    };
                    let total_all = all_data.len() as f64;
                    for (di, dataset) in data_all.iter().enumerate() {
                        if dataset.is_empty() { continue; }
                        let col_str = colors.get(di).map(|s| s.as_str()).unwrap_or("");
                        let col = parse_color(col_str, *color_idx + di).unwrap_or_else(|_| default_color(*color_idx + di));
                        let rgb = to_plotters_color(col);
                        let fill_style: ShapeStyle = rgb.mix(*alpha).filled().into();
                        let outline_style: ShapeStyle = rgb.mix(*alpha).stroke_width(1).into();
                        let mut counts = vec![0usize; *bins];
                        for &val in dataset {
                            if val < global_min || val > global_max { continue; }
                            let bin = bin_edges_list.partition_point(|&e| e <= val) - 1;
                            if bin < *bins {
                                counts[bin] += 1;
                            }
                        }
                        for (i, &count) in counts.iter().enumerate() {
                            let bin_left = bin_edges_list[i];
                            let bin_right = bin_edges_list[i + 1];
                            let h = if *density { count as f64 / (total_all * (bin_right - bin_left)) } else { count as f64 };
                            if h <= 0.0 { continue; }
                            if histtype == "stepfilled" {
                                chart.draw_series(std::iter::once(Rectangle::new(
                                    [(bin_left, 0.0), (bin_right, h)],
                                    fill_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw hist fill: {}", e)))?;
                                chart.draw_series(std::iter::once(Rectangle::new(
                                    [(bin_left, 0.0), (bin_right, h)],
                                    outline_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw hist outline: {}", e)))?;
                            } else {
                                chart.draw_series(std::iter::once(Rectangle::new(
                                    [(bin_left, 0.0), (bin_right, h)],
                                    fill_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw hist: {}", e)))?;
                            }
                        }
                    }
                }
                PlotElement::Image { data, cmap } => {
                    if data.is_empty() || data[0].is_empty() { continue; }
                    let d_min = data.iter().flatten().cloned().fold(f64::INFINITY, f64::min);
                    let d_max = data.iter().flatten().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let d_range = if (d_max - d_min).abs() < 1e-10 { 1.0 } else { d_max - d_min };
                    for (r, row) in data.iter().enumerate() {
                        for (c, &val) in row.iter().enumerate() {
                            let normalized = (val - d_min) / d_range;
                            let rgb = match cmap.as_str() {
                                "gray" | "grey" => { let v = (normalized * 255.0) as u8; RGBColor(v, v, v) }
                                "hot" => {
                                    let r = (normalized * 3.0).min(1.0).max(0.0);
                                    let g = (normalized * 3.0 - 1.0).min(1.0).max(0.0);
                                    let b = (normalized * 3.0 - 2.0).min(1.0).max(0.0);
                                    RGBColor((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
                                }
                                "plasma" => plasma_color(normalized),
                                "inferno" => inferno_color(normalized),
                                "magma" => magma_color(normalized),
                                "cool" => cool_color(normalized),
                                "spring" => spring_color(normalized),
                                "summer" => summer_color(normalized),
                                "autumn" => autumn_color(normalized),
                                "winter" => winter_color(normalized),
                                _ => viridis_color(normalized),
                            };
                            chart.draw_series(std::iter::once(Rectangle::new(
                                [(c as f64, r as f64), ((c + 1) as f64, (r + 1) as f64)],
                                rgb.filled(),
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw image: {}", e)))?;
                        }
                    }
                }
                PlotElement::Text { x, y, text, fontsize, color } => {
                    let txv = tx(*x);
                    let tyv = ty(*y);
                    if !txv.is_finite() || !tyv.is_finite() { continue; }
                    let fs = scale_font(*fontsize as f64, font_scale);
                    let font: FontDesc = ("sans-serif", fs).into();
                    let colored_font = font.color(&to_plotters_color(*color));
                    let text_style: TextStyle = colored_font.into();
                    chart.draw_series(std::iter::once(plotters::element::Text::new(
                        text.clone(),
                        (txv, tyv),
                        text_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw text: {}", e)))?;
                }
                PlotElement::HLine { y, color, linewidth, color_idx, .. } => {
                    let tyv = ty(*y);
                    if !tyv.is_finite() { continue; }
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| RgbColor(0, 0, 0));
                    draw_single_line(chart, x_min, tyv, x_max, tyv, col, *linewidth)?;
                }
                PlotElement::VLine { x, color, linewidth, color_idx, .. } => {
                    let txv = tx(*x);
                    if !txv.is_finite() { continue; }
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| RgbColor(0, 0, 0));
                    draw_single_line(chart, txv, y_min, txv, y_max, col, *linewidth)?;
                }
                PlotElement::Pie { x, labels, colors, autopct, startangle } => {
                    let total: f64 = x.iter().sum();
                    if total <= 0.0 { continue; }
                    let mut current_angle = startangle.to_radians();
                    let pie_colors = colors.as_ref().map(|c| c.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                    for (i, &val) in x.iter().enumerate() {
                        if val <= 0.0 { continue; }
                        let angle = (val / total) * 360.0_f64;
                        let angle_rad = angle.to_radians();
                        let end_angle = current_angle + angle_rad;
                        let col = if let Some(ref pc) = pie_colors {
                            let ci = parse_color(pc.get(i).unwrap_or(&""), i).unwrap_or_else(|_| default_color(i));
                            to_plotters_color(ci)
                        } else {
                            to_plotters_color(default_color(i))
                        };
                        let steps = ((angle_rad / 0.05).ceil() as usize).max(3);
                        let mut vertices = vec![(0.0, 0.0)];
                        for j in 0..=steps {
                            let a = current_angle + (j as f64 / steps as f64) * angle_rad;
                            vertices.push((a.cos(), a.sin()));
                        }
                        chart.draw_series(std::iter::once(Polygon::new(
                            vertices, col.mix(0.85).filled(),
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw pie: {}", e)))?;
                        let mid_angle = current_angle + angle_rad / 2.0;
                        if let Some(lbls) = labels {
                            if let Some(l) = lbls.get(i) {
                                let label_r = 1.3;
                                let lx = mid_angle.cos() * label_r;
                                let ly = mid_angle.sin() * label_r;
                                chart.draw_series(std::iter::once(plotters::element::Text::new(
                                    l.clone(), (lx, ly), ("sans-serif", scale_font(12.0, font_scale)),
                                ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw pie label: {}", e)))?;
                            }
                        }
                        if let Some(fmt) = autopct {
                            let pct = val / total * 100.0;
                            let text = if fmt == "%1.1f%%" || fmt.contains("%%") {
                                format!("{:.1}%", pct)
                            } else if fmt == "%d%%" {
                                format!("{}%", pct as i32)
                            } else {
                                format!("{:.1}%", pct)
                            };
                            let text_r = 0.7;
                            let tx = mid_angle.cos() * text_r;
                            let ty = mid_angle.sin() * text_r;
                            chart.draw_series(std::iter::once(plotters::element::Text::new(
                                text, (tx, ty), ("sans-serif", scale_font(11.0, font_scale)),
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw autopct: {}", e)))?;
                        }
                        current_angle = end_angle;
                    }
                }
                PlotElement::FillBetween { x, y1, y2, color, alpha, .. } => {
                    let col = parse_color(color, 0).unwrap_or_else(|_| RgbColor(0, 128, 0));
                    let rgb = to_plotters_color(col);
                    if x.len() != y1.len() || x.is_empty() { continue; }
                    let mut points: Vec<(f64, f64)> = Vec::with_capacity(x.len() * 2);
                    for (&xv, &yv) in x.iter().zip(y1.iter()) {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                    }
                    for i in (0..x.len()).rev() {
                        let y2v = if i < y2.len() { y2[i] } else { 0.0 };
                        let txv = tx(x[i]);
                        let tyv = ty(y2v);
                        if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                    }
                    if points.len() < 3 { continue; }
                    chart.draw_series(std::iter::once(Polygon::new(
                        points, rgb.mix(*alpha).filled(),
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw fill_between: {}", e)))?;
                }
                PlotElement::ErrorBar { x, y, yerr, xerr, fmt, color, capsize, .. } => {
                    let idx = 0;
                    let col = parse_color(color, idx).unwrap_or_else(|_| default_color(idx));
                    let rgb = to_plotters_color(col);
                    let line_style: ShapeStyle = rgb.stroke_width(1).into();
                    let cap_half = capsize / 2.0;
                    for (i, (&xv, &yv)) in x.iter().zip(y.iter()).enumerate() {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if !txv.is_finite() || !tyv.is_finite() { continue; }
                        let ye = if let Some(vec) = yerr.as_ref() { if i < vec.len() { vec[i] } else { 0.0_f64 } } else { 0.0 };
                        let xe = if let Some(vec) = xerr.as_ref() { if i < vec.len() { vec[i] } else { 0.0_f64 } } else { 0.0 };
                        if ye != 0.0 {
                            let ty_lo = ty(yv - ye);
                            let ty_hi = ty(yv + ye);
                            if ty_lo.is_finite() && ty_hi.is_finite() {
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![(txv, ty_lo), (txv, ty_hi)], line_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar line: {}", e)))?;
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![(txv - cap_half, ty_lo), (txv + cap_half, ty_lo)], line_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar cap: {}", e)))?;
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![(txv - cap_half, ty_hi), (txv + cap_half, ty_hi)], line_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar cap: {}", e)))?;
                            }
                        }
                        if xe != 0.0 {
                            let tx_lo = tx(xv - xe);
                            let tx_hi = tx(xv + xe);
                            if tx_lo.is_finite() && tx_hi.is_finite() {
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![(tx_lo, tyv), (tx_hi, tyv)], line_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xline: {}", e)))?;
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![(tx_lo, tyv - cap_half), (tx_lo, tyv + cap_half)], line_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e)))?;
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![(tx_hi, tyv - cap_half), (tx_hi, tyv + cap_half)], line_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e)))?;
                            }
                        }
                        if !fmt.is_empty() {
                            let marker_name = fmt;
                            draw_marker(chart, marker_name, txv, tyv, 3.0, rgb)
                                .map_err(|e| PyRuntimeError::new_err(format!("ErrorBar marker: {}", e)))?;
                        }
                    }
                }
                PlotElement::Stem { x, y, linefmt, markerfmt, .. } => {
                    let col = RgbColor(0, 0, 200);
                    let rgb = to_plotters_color(col);
                    let baseline = ty(0.0);
                    if linefmt == "-" || linefmt.is_empty() {
                        let line_style = shape_style(col, 1.0, linefmt);
                        for (&xv, &yv) in x.iter().zip(y.iter()) {
                            let txv = tx(xv);
                            let tyv = ty(yv);
                            if !txv.is_finite() || !tyv.is_finite() || !baseline.is_finite() { continue; }
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(txv, baseline), (txv, tyv)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Stem line: {}", e)))?;
                        }
                    } else {
                        for (&xv, &yv) in x.iter().zip(y.iter()) {
                            let txv = tx(xv);
                            let tyv = ty(yv);
                            if !txv.is_finite() || !tyv.is_finite() || !baseline.is_finite() { continue; }
                            draw_single_line(chart, txv, baseline, txv, tyv, col, 1.0)?;
                        }
                    }
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        let txv = tx(xv);
                        let tyv = ty(yv);
                        if !txv.is_finite() || !tyv.is_finite() { continue; }
                        draw_marker(chart, markerfmt, txv, tyv, 5.0, rgb)
                            .map_err(|e| PyRuntimeError::new_err(format!("Stem marker: {}", e)))?;
                    }
                }
                PlotElement::Step { x, y, where_, color, linestyle, linewidth, .. } => {
                    let idx = 0;
                    let col = parse_color(color, idx).unwrap_or_else(|_| default_color(idx));
                    if x.len() < 2 || x.len() != y.len() { continue; }
                    let mut points = Vec::new();
                    match where_.as_str() {
                        "pre" => {
                            let txv = tx(x[0]);
                            let tyv = ty(y[0]);
                            if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                            for i in 1..x.len() {
                                let txv = tx(x[i]);
                                let tyv_prev = ty(y[i - 1]);
                                let tyv = ty(y[i]);
                                if txv.is_finite() && tyv_prev.is_finite() { points.push((txv, tyv_prev)); }
                                if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                            }
                        }
                        "post" => {
                            for i in 0..x.len() - 1 {
                                let txv = tx(x[i]);
                                let tyv = ty(y[i]);
                                let tyv_next = ty(y[i + 1]);
                                if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                                if txv.is_finite() && tyv_next.is_finite() { points.push((txv, tyv_next)); }
                            }
                            let txv = tx(x[x.len() - 1]);
                            let tyv = ty(y[y.len() - 1]);
                            if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                        }
                        _ => {
                            let txv = tx(x[0]);
                            let tyv = ty(y[0]);
                            if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                            for i in 1..x.len() {
                                let mid = (x[i - 1] + x[i]) / 2.0;
                                let tmid = tx(mid);
                                let tyv_prev = ty(y[i - 1]);
                                let tyv = ty(y[i]);
                                if tmid.is_finite() && tyv_prev.is_finite() { points.push((tmid, tyv_prev)); }
                                if tmid.is_finite() && tyv.is_finite() { points.push((tmid, tyv)); }
                            }
                            let txv = tx(x[x.len() - 1]);
                            let tyv = ty(y[y.len() - 1]);
                            if txv.is_finite() && tyv.is_finite() { points.push((txv, tyv)); }
                        }
                    }
                    if points.len() < 2 { continue; }
                    let style = shape_style(col, *linewidth, linestyle);
                    chart.draw_series(LineSeries::new(points, style))
                        .map_err(|e| PyRuntimeError::new_err(format!("Step draw: {}", e)))?;
                }
                PlotElement::BoxPlot { data, labels, .. } => {
                    let box_width = 0.6;
                    for (i, series) in data.iter().enumerate() {
                        if series.is_empty() { continue; }
                        let mut sorted = series.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let n = sorted.len();
                        let min_val = sorted[0];
                        let max_val = sorted[n - 1];
                        let q1 = if n % 2 == 0 {
                            let mid = n / 2;
                            median(&sorted[0..mid])
                        } else {
                            median(&sorted[0..n / 2])
                        };
                        let q3 = if n % 2 == 0 {
                            let mid = n / 2;
                            median(&sorted[mid..])
                        } else {
                            median(&sorted[n / 2 + 1..])
                        };
                        let med = median(&sorted);
                        let iqr = q3 - q1;
                        let lower_whisker = (min_val).max(q1 - 1.5 * iqr);
                        let upper_whisker = (max_val).min(q3 + 1.5 * iqr);
                        let tq1 = ty(q1);
                        let tq3 = ty(q3);
                        let tmed = ty(med);
                        let tlower = ty(lower_whisker);
                        let tupper = ty(upper_whisker);
                        if !tq1.is_finite() || !tq3.is_finite() || !tmed.is_finite() || !tlower.is_finite() || !tupper.is_finite() { continue; }
                        let cx = (i + 1) as f64;
                        let col = to_plotters_color(default_color(i));
                        let fill_style: ShapeStyle = col.mix(0.3).filled().into();
                        let line_style: ShapeStyle = col.stroke_width(2).into();
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx, tlower), (cx, tupper)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot whisker: {}", e)))?;
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(cx - box_width / 2.0, tq1), (cx + box_width / 2.0, tq3)], fill_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot box: {}", e)))?;
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(cx - box_width / 2.0, tq1), (cx + box_width / 2.0, tq3)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot border: {}", e)))?;
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx - box_width / 2.0, tmed), (cx + box_width / 2.0, tmed)],
                            col.stroke_width(2).filled(),
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot median: {}", e)))?;
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx - box_width / 4.0, tlower), (cx + box_width / 4.0, tlower)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx - box_width / 4.0, tupper), (cx + box_width / 4.0, tupper)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                        if let Some(lbls) = labels {
                            if let Some(l) = lbls.get(i) {
                                chart.draw_series(std::iter::once(plotters::element::Text::new(
                                    l.clone(), (cx, -0.3), ("sans-serif", 11.0 * font_scale),
                                ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot label: {}", e)))?;
                            }
                        }
                    }
                }
                PlotElement::Annotate { text, xy, xytext, fontsize, color } => {
                    let col = parse_color(color, 0).unwrap_or_else(|_| RgbColor(0, 0, 0));
                    let rgb = to_plotters_color(col);
                    let (txy_x, txy_y) = xytext.unwrap_or((xy.0 + 0.2, xy.1 + 0.2));
                    let txy_x = tx(txy_x);
                    let txy_y = ty(txy_y);
                    let txy_xy_x = tx(xy.0);
                    let txy_xy_y = ty(xy.1);
                    if !txy_x.is_finite() || !txy_y.is_finite() || !txy_xy_x.is_finite() || !txy_xy_y.is_finite() { continue; }
                    let arrow_style: ShapeStyle = rgb.stroke_width(1).into();
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(txy_x, txy_y), (txy_xy_x, txy_xy_y)], arrow_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Annotate arrow: {}", e)))?;
                    chart.draw_series(std::iter::once(plotters::element::Text::new(
                        text.clone(), (txy_x, txy_y), ("sans-serif", scale_font(*fontsize, font_scale)),
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Annotate text: {}", e)))?;
                }
            }
        }

        if let Some(loc) = &self.legend_loc {
            if !self.legend_labels.is_empty() {
                let (x_anchor, y_anchor, h_pos, v_pos) = match loc.as_str() {
                    "upper right" => (x_max, y_max, HPos::Right, VPos::Top),
                    "upper left" => (x_min, y_max, HPos::Left, VPos::Top),
                    "lower right" => (x_max, y_min, HPos::Right, VPos::Bottom),
                    "lower left" => (x_min, y_min, HPos::Left, VPos::Bottom),
                    "center" => {
                        let cx = (x_min + x_max) / 2.0;
                        let cy = (y_min + y_max) / 2.0;
                        (cx, cy, HPos::Center, VPos::Center)
                    }
                    "right" => {
                        (x_max, (y_min + y_max) / 2.0, HPos::Right, VPos::Center)
                    }
                    "center left" => {
                        (x_min, (y_min + y_max) / 2.0, HPos::Left, VPos::Center)
                    }
                    "center right" => {
                        (x_max, (y_min + y_max) / 2.0, HPos::Right, VPos::Center)
                    }
                    "lower center" => {
                        ((x_min + x_max) / 2.0, y_min, HPos::Center, VPos::Bottom)
                    }
                    "upper center" => {
                        ((x_min + x_max) / 2.0, y_max, HPos::Center, VPos::Top)
                    }
                    _ => {
                        let try_x = x_max - (x_max - x_min) * 0.3;
                        let try_y = y_max - (y_max - y_min) * 0.1;
                        (try_x, try_y, HPos::Right, VPos::Top)
                    }
                };

                let entry_count = self.legend_labels.len();
                let x_range = (x_max - x_min).abs();
                let y_range = (y_max - y_min).abs();
                let entry_height = y_range * 0.04;
                let legend_height = entry_height * entry_count as f64 + y_range * 0.02;
                let legend_width = x_range * 0.25;

                let (box_x1, box_x2) = match h_pos {
                    HPos::Right => (x_anchor - legend_width, x_anchor),
                    HPos::Left => (x_anchor, x_anchor + legend_width),
                    HPos::Center => (x_anchor - legend_width / 2.0, x_anchor + legend_width / 2.0),
                };
                let (box_y1, box_y2) = match v_pos {
                    VPos::Top => (y_anchor - legend_height, y_anchor),
                    VPos::Bottom => (y_anchor, y_anchor + legend_height),
                    VPos::Center => (y_anchor - legend_height / 2.0, y_anchor + legend_height / 2.0),
                };

                let bg_fill: ShapeStyle = RGBColor(255, 255, 255).mix(0.85).filled().into();
                let bg_border: ShapeStyle = RGBColor(180, 180, 180).stroke_width(1).into();

                let bg_rect = Rectangle::new(
                    [(box_x1, box_y1), (box_x2, box_y2)],
                    bg_fill,
                );
                let bg_elements = vec![
                    bg_rect,
                    Rectangle::new(
                        [(box_x1, box_y1), (box_x2, box_y2)],
                        bg_border,
                    ),
                ];
                for elem in bg_elements {
                    chart.draw_series(std::iter::once(elem))
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend bg: {}", e)))?;
                }

                for (i, (label, color, ls, marker_opt)) in self.legend_labels.iter().enumerate() {
                    let y_pos = box_y1 + entry_height * 0.75 + i as f64 * entry_height;
                    let x_line_start = box_x1 + x_range * 0.015;
                    let x_line_end = box_x1 + x_range * 0.06;
                    let x_text = box_x1 + x_range * 0.07;

                    let rgb = to_plotters_color(*color);
                    let line_style: ShapeStyle = rgb.stroke_width(2).into();

                    // 根据线型绘制图例线段
                    match ls.as_str() {
                        "--" => {
                            let dash_len = 8.0;
                            let gap_len = 4.0;
                            let mut pos = x_line_start;
                            let mut drawing = true;
                            while pos < x_line_end {
                                let seg_end = if drawing { (pos + dash_len).min(x_line_end) } else { (pos + gap_len).min(x_line_end) };
                                if drawing {
                                    chart.draw_series(std::iter::once(PathElement::new(
                                        vec![(pos, y_pos), (seg_end, y_pos)], line_style,
                                    ))).map_err(|e| PyRuntimeError::new_err(format!("Legend dashed: {}", e)))?;
                                }
                                pos = seg_end;
                                drawing = !drawing;
                            }
                        }
                        ":" => {
                            let dot_len = 2.0;
                            let gap_len = 4.0;
                            let mut pos = x_line_start;
                            let mut drawing = true;
                            while pos < x_line_end {
                                let seg_end = if drawing { (pos + dot_len).min(x_line_end) } else { (pos + gap_len).min(x_line_end) };
                                if drawing {
                                    chart.draw_series(std::iter::once(PathElement::new(
                                        vec![(pos, y_pos), (seg_end, y_pos)], line_style,
                                    ))).map_err(|e| PyRuntimeError::new_err(format!("Legend dotted: {}", e)))?;
                                }
                                pos = seg_end;
                                drawing = !drawing;
                            }
                        }
                        "-." => {
                            let dash_len = 8.0;
                            let dot_len = 2.0;
                            let gap_len = 3.0;
                            let mut pos = x_line_start;
                            let mut is_dash = true;
                            while pos < x_line_end {
                                let mark_len = if is_dash { dash_len } else { dot_len };
                                let seg_end = (pos + mark_len).min(x_line_end);
                                chart.draw_series(std::iter::once(PathElement::new(
                                    vec![(pos, y_pos), (seg_end, y_pos)], line_style,
                                ))).map_err(|e| PyRuntimeError::new_err(format!("Legend dash-dot: {}", e)))?;
                                pos = seg_end;
                                let gap_end = (pos + gap_len).min(x_line_end);
                                pos = gap_end;
                                is_dash = !is_dash;
                            }
                        }
                        _ => {
                            // 实线
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(x_line_start, y_pos), (x_line_end, y_pos)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend line: {}", e)))?;
                        }
                    }

                    if let Some(mkr) = marker_opt {
                        if !mkr.is_empty() {
                            let mid_x = (x_line_start + x_line_end) / 2.0;
                            draw_marker(chart, mkr, mid_x, y_pos, x_range * 0.01, rgb)
                                .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend marker: {}", e)))?;
                        }
                    }

                    chart.draw_series(std::iter::once(plotters::element::Text::new(
                        label.clone(),
                        (x_text, y_pos),
                        ("sans-serif", scale_font(11.0, font_scale)),
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend text: {}", e)))?;
                }
            }
        }

        // 渲染 axes 标题（在数据区域上方的 margin_top 区域内）
        if !self.title.is_empty() {
            let title_x = (x_min + x_max) / 2.0;
            // 标题位于数据范围上方的 margin_top 区域，使用数据坐标的微小偏移
            let y_range = y_max - y_min;
            // 标题锚点在数据区顶部，使用 VPos::Bottom 让文字向上延伸
            // 偏移量设为数据范围的极小比例，确保文字位于 margin_top 区域内
            let title_y = y_max + y_range * 0.01;
            let font: FontDesc = ("sans-serif", scale_font(14.0, font_scale)).into();
            let colored_font = font.color(&BLACK);
            let text_style: TextStyle = colored_font.pos(Pos::new(HPos::Center, VPos::Bottom)).into();
            chart.draw_series(std::iter::once(plotters::element::Text::new(
                self.title.clone(),
                (title_x, title_y),
                text_style,
            ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw title: {}", e)))?;
        }

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