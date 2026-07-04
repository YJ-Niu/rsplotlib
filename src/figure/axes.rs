use plotters::coord::ranged1d::{BoldPoints, Ranged};
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use plotters::style::ShapeStyle;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyTuple};

use crate::core::colors::{
    RgbColor, default_color, default_color_str, parse_color, to_plotters_color,
};
use crate::core::elements::PlotElement;
use crate::utils::font_stack;

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
/// 支持 None 值和空字符串 ""（均视为无值）
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
        } else if let Ok(s) = item.extract::<String>() {
            // 空字符串 "" 视为无值
            if s.is_empty() {
                result.push(None);
            } else {
                // 尝试将字符串解析为浮点数
                if let Ok(v) = s.parse::<f64>() {
                    result.push(Some(v));
                } else {
                    result.push(None);
                }
            }
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
use crate::figure::axis::{Axis, Patch, SpineDict};

/// 字体大小缩放并四舍五入到1位小数
/// 补偿 plotters 内部对 font size 的换算（实测比预期小约 30%），
/// 通过 * 14.5 将字号放大到与 matplotlib 一致。
pub fn scale_font(size: f64, font_scale: f64) -> f64 {
    (size * font_scale * 14.5).round() / 10.0
}

/// 解析并注册用户显式指定的字体族名。
///
/// 通过 Python 的 `_font_resolver.resolve_font_path` 找到字体文件路径，读入后
/// 用 `Box::leak` 提升为 'static 并注册到 plotters（同 (family, style) 会覆盖）。
/// 成功时返回该字体族名（供渲染时作为 family 使用），失败或名字为空时返回 None。
fn resolve_and_register_family(py: Python<'_>, family: Option<String>) -> Option<String> {
    family.and_then(|family_name| {
        if family_name.is_empty() {
            return None;
        }
        if let Ok(resolver_mod) = py.import("rsplotlib.utils._font_resolver")
            && let Ok(path_obj) = resolver_mod.call_method1("resolve_font_path", (&family_name,))
            && let Ok(Some(path)) = path_obj.extract::<Option<String>>()
            && let Ok(font_data) = std::fs::read(&path)
        {
            let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
            let _ = plotters::style::register_font(&family_name, FontStyle::Normal, font_ref);
            return Some(family_name);
        }
        None
    })
}

#[pyclass(skip_from_py_object)]
pub struct Axes {
    pub elements: Vec<PlotElement>,
    pub xlabel: String,
    pub ylabel: String,
    pub xlabel_fontsize: f64,
    pub xlabel_color: RgbColor,
    pub xlabel_family: Option<String>,
    pub xlabel_loc: String,
    pub ylabel_fontsize: f64,
    pub ylabel_color: RgbColor,
    pub ylabel_family: Option<String>,
    pub ylabel_loc: String,
    pub title: String,
    pub title_fontsize: f64,
    pub title_color: RgbColor,
    pub title_family: Option<String>,
    pub title_loc: String,
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
    pub axis_off: bool,
    pub self_py: Option<Py<PyAny>>,
    pub xaxis_major_locator: Option<Py<PyAny>>,
    pub xaxis_minor_locator: Option<Py<PyAny>>,
    pub yaxis_major_locator: Option<Py<PyAny>>,
    pub yaxis_minor_locator: Option<Py<PyAny>>,
    pub x_axis_inverted: bool,
    pub y_axis_inverted: bool,
    /// 最近一次可映射绘制 (scatter 数值 c / imshow) 的 (cmap, vmin, vmax)，供 colorbar 使用
    pub mappable: Option<(String, f64, f64)>,
    /// 若为 Some，则渲染时在数据区右侧绘制颜色条 (cmap, vmin, vmax)
    pub colorbar: Option<(String, f64, f64)>,
}

impl Clone for Axes {
    fn clone(&self) -> Self {
        Axes {
            elements: self.elements.clone(),
            xlabel: self.xlabel.clone(),
            ylabel: self.ylabel.clone(),
            xlabel_fontsize: self.xlabel_fontsize,
            xlabel_color: self.xlabel_color,
            xlabel_family: self.xlabel_family.clone(),
            xlabel_loc: self.xlabel_loc.clone(),
            ylabel_fontsize: self.ylabel_fontsize,
            ylabel_color: self.ylabel_color,
            ylabel_family: self.ylabel_family.clone(),
            ylabel_loc: self.ylabel_loc.clone(),
            title: self.title.clone(),
            title_fontsize: self.title_fontsize,
            title_color: self.title_color,
            title_family: self.title_family.clone(),
            title_loc: self.title_loc.clone(),
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
            axis_off: self.axis_off,
            self_py: None,
            xaxis_major_locator: None,
            xaxis_minor_locator: None,
            yaxis_major_locator: None,
            yaxis_minor_locator: None,
            x_axis_inverted: self.x_axis_inverted,
            y_axis_inverted: self.y_axis_inverted,
            mappable: self.mappable.clone(),
            colorbar: self.colorbar.clone(),
        }
    }
}

/// 解析 matplotlib 格式字符串
/// 返回 (marker, linestyle, color) 三元组，如果字符串不是 fmt 格式则返回 None
fn parse_fmt_string(fmt: &str) -> Option<(Option<String>, Option<String>, Option<String>)> {
    // 已知 marker 字符
    const MARKERS: &[&str] = &[
        "o", "s", "^", "v", "D", "d", "*", "+", "x", ".", ",", "|", "_", "h", "H", "p", "P", "<",
        ">", "1", "2", "3", "4",
    ];
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
        let c = &fmt[i..i + 1];
        if COLORS.contains(&c) {
            found_color = Some(c.to_string());
            i += 1;
        }
    }

    // 解析 marker
    if i < fmt.len() {
        let m1 = &fmt[i..i + 1];
        if MARKERS.contains(&m1) {
            found_marker = Some(m1.to_string());
            i += 1;
        }
        // 检查是否还有更多 marker 字符
        while i < fmt.len() {
            let m = &fmt[i..i + 1];
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

impl Default for Axes {
    fn default() -> Self {
        Self::new()
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
            xlabel_fontsize: 0.0,
            xlabel_color: RgbColor(0, 0, 0),
            xlabel_family: None,
            xlabel_loc: "center".to_string(),
            ylabel_fontsize: 0.0,
            ylabel_color: RgbColor(0, 0, 0),
            ylabel_family: None,
            ylabel_loc: "center".to_string(),
            title: String::new(),
            title_fontsize: 12.0,
            title_color: RgbColor(0, 0, 0),
            title_family: None,
            title_loc: "center".to_string(),
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
            axis_off: false,
            self_py: None,
            xaxis_major_locator: None,
            xaxis_minor_locator: None,
            yaxis_major_locator: None,
            yaxis_minor_locator: None,
            x_axis_inverted: false,
            y_axis_inverted: false,
            mappable: None,
            colorbar: None,
        }
    }

    #[pyo3(signature = (x, y, label=None, color=None, linestyle="-", marker=None, linewidth=1.5, lw=None, c=None, ls=None, markersize=None, markeredgewidth=None, markerfacecolor=None, markeredgecolor=None, solid_capstyle=None))]
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
        markerfacecolor: Option<String>,
        markeredgecolor: Option<String>,
        solid_capstyle: Option<String>,
    ) -> PyResult<()> {
        // matplotlib 兼容：解析格式字符串
        // 如果 label 是格式字符串（如 'o', '-', 'r--', 'b-o'），从其中提取 marker/linestyle/color
        let mut actual_label = label;
        let mut actual_marker = marker;
        let mut actual_linestyle = linestyle.to_string();
        let mut actual_color = color;
        if let Some(ref lbl) = actual_label
            && is_format_string(lbl)
            && let Some((fmt_marker, fmt_ls, fmt_color)) = parse_fmt_string(lbl)
        {
            let has_marker = fmt_marker.is_some();
            if actual_marker.is_none() {
                actual_marker = fmt_marker;
            }
            if ls.is_none() && linestyle == "-" {
                if let Some(ls_val) = fmt_ls {
                    actual_linestyle = ls_val;
                } else if has_marker {
                    // 格式字符串只有 marker（如 'o'），无线条
                    actual_linestyle = " ".to_string();
                }
            }
            if actual_color.is_none() {
                actual_color = fmt_color;
            }
            actual_label = None;
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
            || linestyle.eq_ignore_ascii_case("null")
        {
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
            markerfacecolor,
            markeredgecolor,
        });
        if let Some(lbl) = actual_label {
            let c =
                parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels
                .push((lbl, c, linestyle_val, None, linewidth));
        }
        Ok(())
    }

    #[pyo3(signature = (x, y, s=100.0, c=None, marker="o", label=None, alpha=1.0))]
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
            let col =
                parse_color(&c.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels
                .push((lbl, col, "-".to_string(), Some(marker_val), 1.5));
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
        color: Option<Bound<'_, PyAny>>,
        label: Option<String>,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let height_vec = py_to_vec_f64(&height)?;
        let idx = self.element_count;
        self.element_count += 1;
        // color 可为单色字符串或每柱一色的列表；None 时留空，渲染回退到默认色。
        let colors_vec = match &color {
            Some(c) => Self::parse_color_list(c, x_vec.len())?,
            None => Vec::new(),
        };
        self.elements.push(PlotElement::Bar {
            x: x_vec,
            height: height_vec,
            width,
            colors: colors_vec.clone(),
            label: label.clone(),
            color_idx: idx,
        });
        if let Some(lbl) = label {
            let col = colors_vec
                .first()
                .map(|c| parse_color(c, idx).unwrap_or_else(|_| default_color(idx)))
                .unwrap_or_else(|| default_color(idx));
            self.legend_labels
                .push((lbl, col, "-".to_string(), None, 1.5));
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
        color: Option<Bound<'_, PyAny>>,
        label: Option<String>,
    ) -> PyResult<()> {
        let y_vec = py_to_vec_f64(&y)?;
        let width_vec = py_to_vec_f64(&width)?;
        let idx = self.element_count;
        self.element_count += 1;
        // color 可为单色字符串或每柱一色的列表；None 时留空，渲染回退到默认色。
        let colors_vec = match &color {
            Some(c) => Self::parse_color_list(c, y_vec.len())?,
            None => Vec::new(),
        };
        self.elements.push(PlotElement::BarH {
            y: y_vec,
            width: width_vec,
            height,
            colors: colors_vec.clone(),
            label: label.clone(),
            color_idx: idx,
        });
        if let Some(lbl) = label {
            let col = colors_vec
                .first()
                .map(|c| parse_color(c, idx).unwrap_or_else(|_| default_color(idx)))
                .unwrap_or_else(|| default_color(idx));
            self.legend_labels
                .push((lbl, col, "-".to_string(), None, 1.5));
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
        let (num_bins, custom_edges): (usize, Option<Vec<f64>>) =
            if let Ok(n) = bins.extract::<usize>() {
                (n, None)
            } else if let Ok(n) = bins.extract::<i64>() {
                if n <= 0 {
                    return Err(PyValueError::new_err("bins must be positive"));
                }
                (n as usize, None)
            } else if let Ok(edges) = py_to_vec_f64(&bins) {
                if edges.len() < 2 {
                    return Err(PyValueError::new_err(
                        "bin_edges must have at least 2 elements",
                    ));
                }
                (edges.len() - 1, Some(edges))
            } else if let Ok(edges) = bins.extract::<Vec<i64>>() {
                if edges.len() < 2 {
                    return Err(PyValueError::new_err(
                        "bin_edges must have at least 2 elements",
                    ));
                }
                (
                    edges.len() - 1,
                    Some(edges.iter().map(|&x| x as f64).collect()),
                )
            } else if let Ok(edges) = bins.extract::<Vec<usize>>() {
                if edges.len() < 2 {
                    return Err(PyValueError::new_err(
                        "bin_edges must have at least 2 elements",
                    ));
                }
                (
                    edges.len() - 1,
                    Some(edges.iter().map(|&x| x as f64).collect()),
                )
            } else {
                return Err(PyValueError::new_err(
                    "bins must be an integer or a list of bin edges",
                ));
            };
        let colors: Vec<String> = if let Some(fc) = facecolor {
            Self::parse_color_list(&fc, x_parsed.len())?
        } else if let Some(c) = color {
            Self::parse_color_list(&c, x_parsed.len())?
        } else {
            (0..x_parsed.len()).map(default_color_str).collect()
        };
        let histtype_val = histtype.unwrap_or_else(|| "bar".to_string());
        let idx = self.element_count;
        self.element_count += 1;

        // 先计算统计量（使用 &x_parsed），再 move 进 PlotElement
        let (global_min, global_max) = x_parsed
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), &v| {
                (mn.min(v), mx.max(v))
            });
        let global_min = if global_min.is_finite() {
            global_min
        } else {
            0.0
        };
        let global_max = if global_max.is_finite() {
            global_max
        } else {
            1.0
        };
        let global_range = global_max - global_min;
        let bin_width = if global_range < 1e-10 {
            1.0
        } else {
            global_range / num_bins as f64
        };
        let n: Vec<Vec<f64>> = x_parsed
            .iter()
            .map(|dataset| {
                if dataset.is_empty() {
                    return vec![0.0; num_bins];
                }
                let mut counts = vec![0usize; num_bins];
                if let Some(ref edges) = custom_edges {
                    let edge_min = edges[0];
                    let edge_max = edges[edges.len() - 1];
                    for &val in dataset {
                        if val < edge_min || val > edge_max {
                            continue;
                        }
                        let bin = edges.partition_point(|&e| e <= val).saturating_sub(1);
                        if bin < num_bins {
                            counts[bin] += 1;
                        }
                    }
                    let total = dataset.len() as f64;
                    if density {
                        counts
                            .iter()
                            .enumerate()
                            .map(|(i, &c)| {
                                let bw = edges[i + 1] - edges[i];
                                if bw > 0.0 {
                                    c as f64 / (total * bw)
                                } else {
                                    0.0
                                }
                            })
                            .collect()
                    } else {
                        counts.iter().map(|&c| c as f64).collect()
                    }
                } else {
                    for &val in dataset {
                        let mut bin = ((val - global_min) / bin_width).floor() as usize;
                        if bin >= num_bins {
                            bin = num_bins - 1;
                        }
                        counts[bin] += 1;
                    }
                    let total = dataset.len() as f64;
                    counts
                        .iter()
                        .map(|&c| {
                            if density {
                                c as f64 / (total * bin_width)
                            } else {
                                c as f64
                            }
                        })
                        .collect()
                }
            })
            .collect();
        let bin_edges: Vec<f64> = if let Some(ref edges) = custom_edges {
            edges.clone()
        } else {
            (0..=num_bins)
                .map(|i| global_min + i as f64 * bin_width)
                .collect()
        };

        let x_parsed_len = x_parsed.len();
        // 移动 x_parsed 和 custom_edges，避免克隆
        self.elements.push(PlotElement::Hist {
            data_all: x_parsed,
            bins: num_bins,
            density,
            histtype: histtype_val,
            label: label.clone(),
            alpha,
            colors: colors.clone(),
            color_idx: idx,
            bin_edges: custom_edges,
        });
        if let Some(lbl) = label {
            let col = parse_color(colors.first().unwrap_or(&String::new()), idx)
                .unwrap_or_else(|_| default_color(idx));
            self.legend_labels
                .push((lbl, col, "-".to_string(), None, 1.5));
        }
        let n_obj: Py<PyAny> = if x_parsed_len <= 1 {
            let empty: Vec<f64> = Vec::new();
            let data = n.first().unwrap_or(&empty);
            PyList::new(py, data.as_slice())
                .unwrap()
                .into_any()
                .unbind()
        } else {
            let lists: Vec<Bound<'_, PyList>> = n
                .iter()
                .map(|inner| PyList::new(py, inner.as_slice()).unwrap())
                .collect();
            PyList::new(py, lists.as_slice())
                .unwrap()
                .into_any()
                .unbind()
        };
        Ok((n_obj, bin_edges, None))
    }

    #[pyo3(signature = (x, cmap="viridis", aspect="auto"))]
    #[allow(unused_variables)]
    pub fn imshow(&mut self, x: Vec<Vec<f64>>, cmap: &str, aspect: &str) {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for row in &x {
            for &v in row {
                if v.is_finite() {
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
        if lo.is_finite() && hi.is_finite() {
            self.mappable = Some((cmap.to_string(), lo, hi));
        }
        self.elements.push(PlotElement::Image {
            data: x,
            cmap: cmap.to_string(),
        });
    }

    /// 记录最近一次可映射绘制的 (cmap, vmin, vmax)，供随后的 colorbar() 使用。
    pub fn set_mappable(&mut self, cmap: String, vmin: f64, vmax: f64) {
        self.mappable = Some((cmap, vmin, vmax));
    }

    /// 基于当前记录的 mappable 启用颜色条；无 mappable 时按 viridis / [0,1] 兜底。
    pub fn enable_colorbar(&mut self) {
        self.colorbar = Some(
            self.mappable
                .clone()
                .unwrap_or_else(|| ("viridis".to_string(), 0.0, 1.0)),
        );
    }

    #[pyo3(signature = (text, color=None, fontsize=None, family=None, loc=None))]
    pub fn set_xlabel(
        &mut self,
        py: Python<'_>,
        text: String,
        color: Option<String>,
        fontsize: Option<f64>,
        family: Option<String>,
        loc: Option<String>,
    ) {
        self.xlabel = text;
        if let Some(fs) = fontsize {
            self.xlabel_fontsize = fs;
        }
        if let Some(c) = color {
            self.xlabel_color = parse_color(&c, 0).unwrap_or(RgbColor(0, 0, 0));
        }
        self.xlabel_family = resolve_and_register_family(py, family);
        if let Some(l) = loc {
            self.xlabel_loc = l;
        }
    }

    #[pyo3(signature = (text, color=None, fontsize=None, family=None, loc=None))]
    pub fn set_ylabel(
        &mut self,
        py: Python<'_>,
        text: String,
        color: Option<String>,
        fontsize: Option<f64>,
        family: Option<String>,
        loc: Option<String>,
    ) {
        self.ylabel = text;
        if let Some(fs) = fontsize {
            self.ylabel_fontsize = fs;
        }
        if let Some(c) = color {
            self.ylabel_color = parse_color(&c, 0).unwrap_or(RgbColor(0, 0, 0));
        }
        self.ylabel_family = resolve_and_register_family(py, family);
        if let Some(l) = loc {
            self.ylabel_loc = l;
        }
    }

    #[pyo3(signature = (text, color=None, fontsize=None, family=None, loc=None))]
    pub fn set_title(
        &mut self,
        py: Python<'_>,
        text: String,
        color: Option<String>,
        fontsize: Option<f64>,
        family: Option<String>,
        loc: Option<String>,
    ) {
        self.title = text;
        if let Some(fs) = fontsize {
            self.title_fontsize = fs;
        }
        if let Some(c) = color {
            self.title_color = parse_color(&c, 0).unwrap_or(RgbColor(0, 0, 0));
        }
        // 当 family 参数传入时，通过 Python 的 _font_resolver 解析字体路径并注册到
        // plotters，使用实际字体家族名称，确保标题以该字体渲染（与 text() 一致）。
        self.title_family = resolve_and_register_family(py, family);
        if let Some(l) = loc {
            self.title_loc = l;
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
    pub fn grid(
        &mut self,
        visible: Option<bool>,
        c: Option<String>,
        ls: Option<String>,
        lw: Option<f64>,
        axis: Option<String>,
    ) {
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

    #[pyo3(signature = (left=None, right=None, _auto=None, xmin=None, xmax=None, emit=true, auto=None))]
    pub fn set_xlim(
        &mut self,
        left: Option<f64>,
        right: Option<f64>,
        _auto: Option<bool>,
        xmin: Option<f64>,
        xmax: Option<f64>,
        emit: bool,
        auto: Option<bool>,
    ) {
        let lo = left.or(xmin);
        let hi = right.or(xmax);
        if let (Some(lo), Some(hi)) = (lo, hi) {
            self.xlim = Some((lo, hi));
        }
        let _ = (emit, auto);
    }

    /// 反转 x 轴方向（matplotlib 兼容）
    pub fn invert_xaxis(&mut self) {
        self.x_axis_inverted = !self.x_axis_inverted;
    }

    /// 反转 y 轴方向（matplotlib 兼容）
    pub fn invert_yaxis(&mut self) {
        self.y_axis_inverted = !self.y_axis_inverted;
    }

    /// 获取 x 轴范围
    pub fn get_xlim(&self) -> PyResult<(f64, f64)> {
        match self.xlim {
            Some((lo, hi)) => Ok((lo, hi)),
            None => {
                let ((x_min, x_max), _) = self.compute_bounds();
                Ok((x_min, x_max))
            }
        }
    }

    /// 获取 y 轴范围
    pub fn get_ylim(&self) -> PyResult<(f64, f64)> {
        match self.ylim {
            Some((lo, hi)) => Ok((lo, hi)),
            None => {
                let (_, (y_min, y_max)) = self.compute_bounds();
                Ok((y_min, y_max))
            }
        }
    }

    #[pyo3(signature = (bottom=None, top=None, emit=true, auto=None))]
    pub fn set_ylim(
        &mut self,
        bottom: Option<f64>,
        top: Option<f64>,
        emit: bool,
        auto: Option<bool>,
    ) {
        if let (Some(lo), Some(hi)) = (bottom, top) {
            self.ylim = Some((lo, hi));
        }
        let _ = (emit, auto);
    }

    #[pyo3(signature = (x, y, text, fontsize=None, color=None, c=None, family=None))]
    pub fn text(
        &mut self,
        py: Python<'_>,
        x: f64,
        y: f64,
        text: Bound<'_, PyAny>,
        fontsize: Option<f64>,
        color: Option<String>,
        c: Option<String>,
        family: Option<String>,
    ) {
        let color = c.or(color);
        let text_str: String = text
            .extract::<String>()
            .unwrap_or_else(|_| text.str().map(|s| s.to_string()).unwrap_or_default());
        let col = parse_color(&color.unwrap_or_else(|| "black".to_string()), 0)
            .unwrap_or(RgbColor(0, 0, 0));
        // 当 family 参数传入时，通过 Python 的 _font_resolver 解析字体路径并注册到 plotters，
        // 使用实际字体家族名称（而非 "sans-serif"），确保只影响指定文字。
        let font_family = family.and_then(|family_name| {
            if let Ok(resolver_mod) = py.import("rsplotlib.utils._font_resolver")
                && let Ok(path_obj) =
                    resolver_mod.call_method1("resolve_font_path", (&family_name,))
                && let Ok(Some(path)) = path_obj.extract::<Option<String>>()
                && let Ok(font_data) = std::fs::read(&path)
            {
                let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
                let _ = plotters::style::register_font(&family_name, FontStyle::Normal, font_ref);
                return Some(family_name);
            }
            None
        });
        self.elements.push(PlotElement::Text {
            x,
            y,
            text: text_str,
            fontsize: fontsize.unwrap_or(12.0),
            color: col,
            font_family,
        });
    }

    #[doc = "绘制水平参考线\n\n参数:\n    y: y 坐标\n    color: 颜色 (可选)\n    linestyle: 线型 ('-', '--', ':', '-.', 可选)\n    linewidth: 线宽 (可选)"]
    #[pyo3(signature = (y=None, color=None, linestyle=None, linewidth=None))]
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

    #[doc = "绘制垂直参考线\n\n参数:\n    x: x 坐标\n    color: 颜色 (可选)\n    linestyle: 线型 (可选)\n    linewidth: 线宽 (可选)"]
    #[pyo3(signature = (x=None, color=None, linestyle=None, linewidth=None))]
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

    #[doc = "在指定 y 位置绘制多条水平线段 (Rust 层批量实现)"]
    #[pyo3(signature = (y, color=None, linestyle=None, linewidth=None))]
    pub fn hlines(
        &mut self,
        py: Python<'_>,
        y: Bound<'_, PyAny>,
        color: Option<String>,
        linestyle: Option<String>,
        linewidth: Option<f64>,
    ) -> PyResult<()> {
        let ys = py_to_vec_f64(&y)?;
        let color_s = color.unwrap_or_default();
        let ls_s = linestyle.unwrap_or_else(|| "-".to_string());
        let lw = linewidth.unwrap_or(1.0);
        for &yv in &ys {
            let idx = self.element_count;
            self.element_count += 1;
            self.elements.push(PlotElement::HLine {
                y: yv,
                color: color_s.clone(),
                linestyle: ls_s.clone(),
                linewidth: lw,
                color_idx: idx,
            });
        }
        let _ = py;
        Ok(())
    }

    #[doc = "在指定 x 位置绘制多条垂直线段 (Rust 层批量实现)"]
    #[pyo3(signature = (x, color=None, linestyle=None, linewidth=None))]
    pub fn vlines(
        &mut self,
        py: Python<'_>,
        x: Bound<'_, PyAny>,
        color: Option<String>,
        linestyle: Option<String>,
        linewidth: Option<f64>,
    ) -> PyResult<()> {
        let xs = py_to_vec_f64(&x)?;
        let color_s = color.unwrap_or_default();
        let ls_s = linestyle.unwrap_or_else(|| "-".to_string());
        let lw = linewidth.unwrap_or(1.0);
        for &xv in &xs {
            let idx = self.element_count;
            self.element_count += 1;
            self.elements.push(PlotElement::VLine {
                x: xv,
                color: color_s.clone(),
                linestyle: ls_s.clone(),
                linewidth: lw,
                color_idx: idx,
            });
        }
        let _ = py;
        Ok(())
    }

    #[pyo3(signature = (x, labels=None, colors=None, autopct=None, startangle=0.0, explode=None))]
    pub fn pie(
        &mut self,
        x: Vec<f64>,
        labels: Option<Vec<String>>,
        colors: Option<Vec<String>>,
        autopct: Option<String>,
        startangle: f64,
        explode: Option<Vec<f64>>,
    ) {
        self.elements.push(PlotElement::Pie {
            x,
            labels,
            colors,
            autopct,
            startangle,
            explode,
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
            let col =
                parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels
                .push((lbl, col, "-".to_string(), None, 1.5));
        }
        Ok(())
    }

    #[pyo3(signature = (x, *args, labels=None, colors=None, alpha=1.0))]
    pub fn stackplot(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        args: &Bound<'_, PyTuple>,
        labels: Option<Vec<String>>,
        colors: Option<Vec<String>>,
        alpha: f64,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        // 从 *args 收集 y 数据：每个 arg 应该是 Vec<f64>
        let mut y_series: Vec<Vec<f64>> = Vec::new();
        for arg in args.iter() {
            if let Ok(single) = arg.extract::<Vec<f64>>() {
                y_series.push(single);
            } else if let Ok(list_of_lists) = arg.extract::<Vec<Vec<f64>>>() {
                y_series.extend(list_of_lists);
            }
        }

        let idx = self.element_count;
        self.element_count += 1;
        self.elements.push(PlotElement::Stack {
            x: x_vec,
            y_series,
            labels: labels.clone(),
            colors: colors.clone(),
            alpha,
        });
        if let Some(lbls) = labels {
            for (i, lbl) in lbls.into_iter().enumerate() {
                let col = parse_color(
                    colors
                        .as_ref()
                        .and_then(|c| c.get(i))
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                    idx,
                )
                .unwrap_or_else(|_| default_color(idx + i));
                self.legend_labels
                    .push((lbl, col, "-".to_string(), None, 1.5));
            }
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
            let col =
                parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels
                .push((lbl, col, "-".to_string(), Some(fmt.to_string()), 1.5));
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
            self.legend_labels.push((
                lbl,
                col,
                linefmt.to_string(),
                Some(markerfmt.to_string()),
                1.5,
            ));
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
            let col =
                parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels
                .push((lbl, col, linestyle.to_string(), None, linewidth));
        }
        Ok(())
    }

    #[pyo3(signature = (x, labels=None, vert=true))]
    pub fn boxplot(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        labels: Option<Vec<String>>,
        vert: bool,
    ) -> PyResult<()> {
        let data = py_to_vec_vec_f64(&x)?;
        self.elements
            .push(PlotElement::BoxPlot { data, labels, vert });
        Ok(())
    }

    #[doc = "添加带箭头的文本标注 (由 Rust 层实现)\n\n参数:\n    text: 标注文本\n    xy: 被标注点的坐标 (x, y)\n    xytext: 文本放置位置, 若提供则自动绘制箭头到 xy\n    fontsize: 字体大小 (默认 12.0)\n    color: 颜色\n    arrowprops: 箭头属性字典 (可选)\n    arrowstyle: 箭头样式 (可选)\n    arrowsize: 箭头大小 (默认 1.0)"]
    #[pyo3(signature = (text, xy, xytext=None, fontsize=12.0, color="black", arrowprops=None, arrowstyle=None, arrowsize=1.0))]
    pub fn annotate(
        &mut self,
        text: &str,
        xy: (f64, f64),
        xytext: Option<(f64, f64)>,
        fontsize: f64,
        color: &str,
        arrowprops: Option<Bound<'_, PyAny>>,
        arrowstyle: Option<String>,
        arrowsize: f64,
    ) {
        // 判断是否需要绘制箭头
        let needs_arrow = xytext.is_some();
        self.elements.push(PlotElement::Annotate {
            text: text.to_string(),
            xy,
            xytext,
            fontsize,
            color: color.to_string(),
        });
        // 如果有 xytext，绘制一条箭头线从 xy 指向 xytext
        if needs_arrow {
            let text_pos = xytext.unwrap();
            self.elements.push(PlotElement::Arrow {
                x1: text_pos.0,
                y1: text_pos.1,
                x2: xy.0,
                y2: xy.1,
                color: color.to_string(),
                linewidth: 1.0 * arrowsize,
                head_size: 8.0 * arrowsize,
            });
            // 忽略 arrowprops dict（简单实现即可）
            let _ = arrowprops;
            let _ = arrowstyle;
        }
    }

    #[doc = "散点图 (支持每个点独立颜色和大小, Rust 层批量实现)\n\n参数:\n    x: x 坐标列表\n    y: y 坐标列表\n    s: 每个点的大小 (列表), 或 None 用默认\n    c: 每个点的颜色 (列表), 或 None 用默认\n    marker: 标记形状 ('o', 's', '^', 'D', '*', 'x', '+', 'v', '<', '>')\n    label: 图例标签\n    alpha: 透明度 (0.0-1.0)"]
    #[pyo3(signature = (x, y, s=None, c=None, marker="o", label=None, alpha=1.0))]
    #[allow(clippy::too_many_arguments)]
    pub fn scatter_multi(
        &mut self,
        _py: Python<'_>,
        x: Bound<'_, PyAny>,
        y: Bound<'_, PyAny>,
        s: Option<Bound<'_, PyAny>>,
        c: Option<Bound<'_, PyAny>>,
        marker: &str,
        label: Option<String>,
        alpha: f64,
    ) -> PyResult<()> {
        let x_vec = py_to_vec_f64(&x)?;
        let y_vec = py_to_vec_f64(&y)?;
        let idx = self.element_count;
        self.element_count += 1;

        let s_list: Option<Vec<f64>> = match s {
            Some(v) => Some(py_to_vec_f64(&v)?),
            None => None,
        };
        let c_list: Option<Vec<String>> = match c {
            Some(v) => {
                if let Ok(list) = v.extract::<Vec<String>>() {
                    Some(list)
                } else if let Ok(single) = v.extract::<String>() {
                    Some(vec![single])
                } else {
                    None
                }
            }
            None => None,
        };

        self.elements.push(PlotElement::ScatterMulti {
            x: x_vec,
            y: y_vec,
            s_list,
            c_list,
            marker: marker.to_string(),
            label,
            alpha,
            color_idx: idx,
        });
        Ok(())
    }

    #[doc = "绘制水平区间填充 (在 y 方向高亮 y1 到 y2 的水平带)\n\n参数:\n    y1: y 轴下限\n    y2: y 轴上限\n    color: 填充颜色\n    alpha: 透明度 (0.0-1.0, 默认 0.3)"]
    #[pyo3(signature = (y1, y2, color=None, alpha=0.3))]
    pub fn axhspan(&mut self, y1: f64, y2: f64, color: Option<String>, alpha: f64) {
        self.elements.push(PlotElement::HSpan {
            y1,
            y2,
            color: color.unwrap_or_default(),
            alpha,
        });
    }

    #[doc = "绘制垂直区间填充 (在 x 方向高亮 x1 到 x2 的垂直带)\n\n参数:\n    x1: x 轴下限\n    x2: x 轴上限\n    color: 填充颜色\n    alpha: 透明度 (0.0-1.0, 默认 0.3)"]
    #[pyo3(signature = (x1, x2, color=None, alpha=0.3))]
    pub fn axvspan(&mut self, x1: f64, x2: f64, color: Option<String>, alpha: f64) {
        self.elements.push(PlotElement::VSpan {
            x1,
            x2,
            color: color.unwrap_or_default(),
            alpha,
        });
    }

    #[doc = "通过两点绘制任意斜率的直线 (贯穿整张图)\n\n参数:\n    xy1: 起点坐标 (x1, y1)\n    xy2: 终点坐标 (x2, y2)\n    color: 线颜色\n    linestyle: 线型\n    linewidth: 线宽"]
    #[pyo3(signature = (xy1, xy2, color=None, linestyle=None, linewidth=None))]
    pub fn axline(
        &mut self,
        xy1: (f64, f64),
        xy2: (f64, f64),
        color: Option<String>,
        linestyle: Option<String>,
        linewidth: Option<f64>,
    ) {
        self.elements.push(PlotElement::AxLine {
            xy1,
            xy2,
            color: color.unwrap_or_default(),
            linestyle: linestyle.unwrap_or_else(|| "-".to_string()),
            linewidth: linewidth.unwrap_or(1.5),
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
    pub fn set_xticks(&mut self, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) {
        self.xticks_val = ticks;
        self.xtick_labels = labels;
    }

    #[pyo3(signature = (ticks=None, labels=None))]
    pub fn yticks(&mut self, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) {
        self.yticks_val = ticks;
        self.ytick_labels = labels;
    }

    #[pyo3(signature = (ticks=None, labels=None))]
    pub fn set_yticks(&mut self, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) {
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
    pub fn tick_params(
        &mut self,
        axis: &str,
        labelsize: Option<f64>,
        rotation: Option<f64>,
        bottom: Option<bool>,
        top: Option<bool>,
        left: Option<bool>,
        right: Option<bool>,
    ) {
        if let Some(v) = labelsize {
            self.tick_labelsize = v;
        }
        if let Some(v) = bottom {
            self.tick_bottom = v;
        }
        if let Some(v) = top {
            self.tick_top = v;
        }
        if let Some(v) = left {
            self.tick_left = v;
        }
        if let Some(v) = right {
            self.tick_right = v;
        }
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
        self.axis_off = true;
    }

    /// matplotlib 兼容：启用次刻度（major + minor）
    pub fn minorticks_on(&mut self) {
        self.minor_grid_visible = true;
        self.minor_grid_x_visible = true;
        self.minor_grid_y_visible = true;
    }

    /// matplotlib 兼容：禁用次刻度
    pub fn minorticks_off(&mut self) {
        self.minor_grid_visible = false;
        self.minor_grid_x_visible = false;
        self.minor_grid_y_visible = false;
    }

    pub fn set_aspect(&mut self, _aspect: &str) {}

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
        Py::new(py, axis)
    }

    #[getter]
    pub fn get_yaxis(&self, py: Python) -> PyResult<Py<Axis>> {
        let mut axis = Axis::new();
        axis.which = "y".to_string();
        axis.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Py::new(py, axis)
    }

    #[getter]
    pub fn get_patch(&self, py: Python) -> PyResult<Py<Patch>> {
        let mut patch = Patch::new();
        patch.facecolor = self.facecolor.clone();
        patch.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Py::new(py, patch)
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
        Py::new(py, sd)
    }
}

impl Axes {
    pub fn compute_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let xlog = self.xscale == "log";
        let ylog = self.yscale == "log";
        let ((mut x_min, mut x_max), (mut y_min, mut y_max)) =
            crate::figure::axes_bounds::compute_bounds(
                &self.elements,
                self.xlim,
                self.ylim,
                xlog,
                ylog,
            );
        // 应用轴反转
        if self.x_axis_inverted {
            std::mem::swap(&mut x_min, &mut x_max);
        }
        if self.y_axis_inverted {
            std::mem::swap(&mut y_min, &mut y_max);
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
        marker_scale: f64,
        fill_bg: bool,
        bitmap: bool,
        ss: f64,
        _subplot_info: Option<&(f64, f64, f64, f64)>,
    ) -> PyResult<()>
    where
        DB::ErrorType: 'static,
    {
        // 仅主轴填充背景，twin axes 不填充以避免覆盖主轴数据
        // 当 axis("off") 被调用时，子图背景设为透明（不填充）
        if fill_bg && !self.axis_off {
            let bg_color = parse_color(&self.facecolor, 0).unwrap_or(RgbColor(255, 255, 255));
            chart
                .plotting_area()
                .fill(&to_plotters_color(bg_color))
                .map_err(|e| {
                    PyRuntimeError::new_err(format!("Failed to fill background: {}", e))
                })?;
        }

        // 在 chart 进入可变借用前，先取出绘图区像素尺寸（用于判断副刻度密度）
        let (plot_pixel_width, plot_pixel_height) = {
            let dim = chart.plotting_area().dim_in_pixel();
            (dim.0, dim.1)
        };

        let xlog = self.xscale == "log";
        let ylog = self.yscale == "log";

        // 计算主/副刻度
        let ticks_info = crate::figure::axes_mesh::compute_ticks(
            py,
            &self.xticks_val,
            &self.yticks_val,
            &self.xaxis_major_locator,
            &self.yaxis_major_locator,
            &self.xaxis_minor_locator,
            &self.yaxis_minor_locator,
            x_min,
            x_max,
            y_min,
            y_max,
            plot_pixel_width,
            plot_pixel_height,
            self.minor_grid_x_visible,
            self.minor_grid_y_visible,
            self.minor_grid_visible,
        );

        // 计算网格线颜色/线宽/样式
        let grid_style = crate::figure::axes_mesh::compute_grid_style(
            &self.grid_color,
            self.grid_linewidth,
            &self.grid_linestyle,
            &self.minor_grid_color,
            self.minor_grid_linewidth,
            &self.minor_grid_linestyle,
        );

        // 配置并绘制 mesh（与 ChartContext 的借用密切相关，必须内联）
        {
            let frame_color = parse_color(&self.spine_color, 0).unwrap_or(RgbColor(0, 0, 0));
            let frame_lw = self.spine_linewidth.round().max(1.0) as u32;
            let frame_style: ShapeStyle = to_plotters_color(frame_color).stroke_width(frame_lw);
            let label_size: f64 = scale_font(self.tick_labelsize, font_scale);
            // plotters 的 configure_mesh 只有单一 axis_desc_style（x_desc 与 y_desc 共用），
            // 无法给 xlabel/ylabel 各自设样式。这里让二者共用一套：优先采用 xlabel 的
            // fontdict（family/size/color），其次 ylabel，最后回退默认。TextStyle 借用
            // desc_family 字符串，故其必须与 mesh_builder 同作用域。
            let x_has_custom = self.xlabel_family.is_some()
                || self.xlabel_fontsize > 0.0
                || !(self.xlabel_color.0 == 0
                    && self.xlabel_color.1 == 0
                    && self.xlabel_color.2 == 0);
            let y_has_custom = self.ylabel_family.is_some()
                || self.ylabel_fontsize > 0.0
                || !(self.ylabel_color.0 == 0
                    && self.ylabel_color.1 == 0
                    && self.ylabel_color.2 == 0);
            let (desc_family_opt, desc_fontsize, desc_color) = if x_has_custom {
                (
                    self.xlabel_family.clone(),
                    self.xlabel_fontsize,
                    self.xlabel_color,
                )
            } else if y_has_custom {
                (
                    self.ylabel_family.clone(),
                    self.ylabel_fontsize,
                    self.ylabel_color,
                )
            } else {
                (None, 0.0, RgbColor(0, 0, 0))
            };
            // family：显式指定优先，否则按标签文本自动选字（含 CJK 回退）。
            let desc_text = if !self.xlabel.is_empty() {
                self.xlabel.as_str()
            } else {
                self.ylabel.as_str()
            };
            let axis_desc_family =
                font_stack::resolve_font_family(desc_text, desc_family_opt.as_deref());
            let axis_desc_size = if desc_fontsize > 0.0 {
                scale_font(desc_fontsize, font_scale)
            } else {
                label_size
            };
            let axis_desc_rgb = to_plotters_color(desc_color);
            // 类别型 x 轴：同时提供刻度位置 (xticks_val) 与字符串标签 (xtick_labels) 时，
            // 把落在这些位置的刻度渲染成对应字符串（如柱状图的类别名），其余刻度回退为
            // 数值格式。plotters 仅按数量自动布点，故用位置匹配 (容差 1e-6) 做映射。
            // 在 mesh_builder 之前声明，保证其生命周期长于持有其引用的 builder。
            let xtick_label_map: Vec<(f64, String)> = match (&self.xticks_val, &self.xtick_labels) {
                (Some(ticks), Some(labels)) => {
                    ticks.iter().cloned().zip(labels.iter().cloned()).collect()
                }
                _ => Vec::new(),
            };
            let has_xcat = !xtick_label_map.is_empty();
            let x_cat_fmt = move |v: &f64| -> String {
                for (t, l) in &xtick_label_map {
                    if (t - *v).abs() < 1e-6 {
                        return l.clone();
                    }
                }
                crate::figure::axes_mesh::format_linear_tick(*v)
            };
            // 类别型 y 轴：与 x 轴对称（如 barh 的类别名），落在 yticks_val 位置的刻度渲染
            // 为对应字符串标签，其余回退数值格式。
            let ytick_label_map: Vec<(f64, String)> = match (&self.yticks_val, &self.ytick_labels) {
                (Some(ticks), Some(labels)) => {
                    ticks.iter().cloned().zip(labels.iter().cloned()).collect()
                }
                _ => Vec::new(),
            };
            let has_ycat = !ytick_label_map.is_empty();
            let y_cat_fmt = move |v: &f64| -> String {
                for (t, l) in &ytick_label_map {
                    if (t - *v).abs() < 1e-6 {
                        return l.clone();
                    }
                }
                crate::figure::axes_mesh::format_linear_tick(*v)
            };
            // 主刻度线像素长度（matplotlib 风格，向外）。plotters 中 label_dist = 2*tick_px。
            let tick_px = (3.5 * font_scale).round().max(1.0) as i32;
            // 仅主轴（非 twin）的底部 x 轴：抑制 plotters 内置刻度标签文本，改在 mesh
            // 绘制后手动绘制，使标签相对 plotters 默认位置再下移 2 个最终像素（渲染像素 =
            // round(2*ss)）。刻度线仍由 plotters 在相同 key points 处绘制，保证标签与刻度线
            // 水平对齐。twin 轴的 x 标签在顶部，位置不同，故不处理。
            let x_axis_on = self.spine_bottom || self.spine_top;
            let x_labels_on = self.tick_bottom || self.tick_top;
            let do_manual_x = !self.is_twin_x && !self.is_twin_y && x_axis_on && x_labels_on;
            // 取 plotters 实际用于底部 x 标签的 key points（与刻度线位置一致）。
            let x_key_points: Vec<f64> = if do_manual_x {
                let n_x = ticks_info.xticks.len().max(2);
                chart
                    .plotting_area()
                    .as_coord_spec()
                    .x_spec()
                    .key_points(BoldPoints(n_x))
            } else {
                Vec::new()
            };

            let mut mesh_builder = chart.configure_mesh();
            mesh_builder
                .x_labels(ticks_info.xticks.len().max(2))
                .y_labels(ticks_info.yticks.len().max(2))
                .x_label_style(("sans-serif", label_size).into_font().color(&BLACK))
                .y_label_style(("sans-serif", label_size).into_font().color(&BLACK))
                .bold_line_style(frame_style);

            // xlabel/ylabel 用 plotters 内置 x_desc/y_desc 自动定位，共用 axis_desc_style。
            // 但 plotters 只能居中；当 loc 非居中时，此处传空串禁用内置绘制，
            // 改由 figure.rs 在 root 上按绝对像素手动绘制（见 axes_title::draw_{x,y}label_manual）。
            let x_desc_text = if self.xlabel_loc == "center" {
                self.xlabel.clone()
            } else {
                String::new()
            };
            let y_desc_text = if self.ylabel_loc == "center" {
                self.ylabel.clone()
            } else {
                String::new()
            };
            mesh_builder
                .x_desc(x_desc_text)
                .y_desc(y_desc_text)
                .axis_desc_style(
                    (axis_desc_family.as_str(), axis_desc_size)
                        .into_font()
                        .color(&axis_desc_rgb),
                );

            if xlog {
                mesh_builder.x_label_formatter(&|v| format!("{:.1e}", 10.0f64.powf(*v)));
            }
            if ylog {
                mesh_builder.y_label_formatter(&|v| format!("{:.1e}", 10.0f64.powf(*v)));
            } else {
                mesh_builder
                    .y_label_formatter(&|v| crate::figure::axes_mesh::format_linear_tick(*v));
                mesh_builder
                    .x_label_formatter(&|v| crate::figure::axes_mesh::format_linear_tick(*v));
            }
            // 类别标签覆盖 x 轴数值格式（plotters 后一次 x_label_formatter 覆盖前一次）。
            if has_xcat {
                mesh_builder.x_label_formatter(&x_cat_fmt);
            }
            // 类别标签覆盖 y 轴数值格式（barh 等场景）。
            if has_ycat {
                mesh_builder.y_label_formatter(&y_cat_fmt);
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

            // matplotlib 风格刻度线：向外、长度约 3.5pt（正值 = 向外）。
            // plotters 默认刻度长为绘图区的 5%，在本项目自定义布局下渲染极短（~1px），
            // 故显式设为固定像素（tick_px，见上文）。draw_impl 中 label_dist = 2*tick_size。
            mesh_builder
                .set_tick_mark_size(LabelAreaPosition::Bottom, tick_px)
                .set_tick_mark_size(LabelAreaPosition::Left, tick_px);

            // 主轴底部 x 标签改为手动绘制：用空串抑制 plotters 内置标签文本（刻度线保留）。
            if do_manual_x {
                mesh_builder.x_label_formatter(&|_: &f64| String::new());
            }

            // 手动绘制 mesh：禁用内置网格线（由 axes_grid 模块统一绘制）
            mesh_builder
                .disable_x_mesh()
                .disable_y_mesh()
                .draw()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw mesh: {}", e)))?;

            // 手动绘制底部 x 刻度标签：相对 plotters 默认位置（label_dist = 2*tick_px）
            // 再向下偏移 2 个最终像素（渲染像素 = round(2*ss)）。锚点 (t, y_min) 映射到
            // 绘图区底边，Text 的像素偏移 (0, offset_y) 使文字顶端下移，与刻度线对齐。
            if do_manual_x {
                drop(mesh_builder);
                let (x_lo, x_hi) = (x_min.min(x_max), x_min.max(x_max));
                let offset_y = tick_px * 2 + (2.0 * ss).round() as i32;
                let text_style: TextStyle = ("sans-serif", label_size)
                    .into_font()
                    .color(&BLACK)
                    .pos(Pos::new(HPos::Center, VPos::Top));
                for &t in &x_key_points {
                    if t < x_lo || t > x_hi {
                        continue;
                    }
                    let text = if xlog {
                        format!("{:.1e}", 10.0f64.powf(t))
                    } else if has_xcat {
                        x_cat_fmt(&t)
                    } else {
                        crate::figure::axes_mesh::format_linear_tick(t)
                    };
                    chart
                        .draw_series(std::iter::once(
                            plotters::element::EmptyElement::at((t, y_min))
                                + plotters::element::Text::new(
                                    text,
                                    (0, offset_y),
                                    text_style.clone(),
                                ),
                        ))
                        .map_err(|e| {
                            PyRuntimeError::new_err(format!("Failed to draw x tick label: {}", e))
                        })?;
                }
            }
        }

        // 手动绘制顶部和右侧 spine（plotters mesh 只绘制左侧和底部边框）
        {
            let spine_col = parse_color(&self.spine_color, 0).unwrap_or(RgbColor(0, 0, 0));
            let spine_rgb = to_plotters_color(spine_col);
            let spine_lw = self.spine_linewidth.round().max(1.0) as u32;
            let spine_style: ShapeStyle = spine_rgb.stroke_width(spine_lw);
            if self.spine_top {
                chart
                    .draw_series(std::iter::once(PathElement::new(
                        vec![(x_min, y_max), (x_max, y_max)],
                        spine_style,
                    )))
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to draw top spine: {}", e))
                    })?;
            }
            if self.spine_right {
                chart
                    .draw_series(std::iter::once(PathElement::new(
                        vec![(x_max, y_min), (x_max, y_max)],
                        spine_style,
                    )))
                    .map_err(|e| {
                        PyRuntimeError::new_err(format!("Failed to draw right spine: {}", e))
                    })?;
            }
        }

        // 绘制主网格线
        if self.grid_visible {
            let major_ls = grid_style.major_ls.as_deref();
            if self.grid_axis == "both" || self.grid_axis == "x" {
                crate::figure::axes_grid::draw_grid_lines(
                    chart,
                    true,
                    &ticks_info.xticks,
                    grid_style.major_color,
                    grid_style.major_lw,
                    major_ls,
                    font_scale,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                )?;
            }
            if self.grid_axis == "both" || self.grid_axis == "y" {
                crate::figure::axes_grid::draw_grid_lines(
                    chart,
                    false,
                    &ticks_info.yticks,
                    grid_style.major_color,
                    grid_style.major_lw,
                    major_ls,
                    font_scale,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                )?;
            }
        }

        // 绘制副网格线
        if self.minor_grid_visible {
            let minor_ls = grid_style.minor_ls.as_deref();
            // 过滤掉与主刻度位置重叠的副刻度，避免副网格线覆盖主网格线
            let xmin_filtered = ticks_info.xminor.as_ref().map(|minor| {
                crate::figure::axes_grid::filter_minor_ticks(minor, &ticks_info.xticks)
            });
            let ymin_filtered = ticks_info.yminor.as_ref().map(|minor| {
                crate::figure::axes_grid::filter_minor_ticks(minor, &ticks_info.yticks)
            });
            let show_x_minor = self.minor_grid_x_visible || !self.minor_grid_y_visible;
            let show_y_minor = self.minor_grid_y_visible || !self.minor_grid_x_visible;
            if show_x_minor && let Some(ref ticks) = xmin_filtered {
                crate::figure::axes_grid::draw_grid_lines(
                    chart,
                    true,
                    ticks,
                    grid_style.minor_color,
                    grid_style.minor_lw,
                    minor_ls,
                    font_scale,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                )?;
            }
            if show_y_minor && let Some(ref ticks) = ymin_filtered {
                crate::figure::axes_grid::draw_grid_lines(
                    chart,
                    false,
                    ticks,
                    grid_style.minor_color,
                    grid_style.minor_lw,
                    minor_ls,
                    font_scale,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                )?;
            }
        }

        // 渲染所有数据元素（线、散点、柱状图、填充、误差棒、饼图等）
        crate::figure::axes_render_elements::render_elements(
            chart,
            &self.elements,
            font_scale,
            marker_scale,
            xlog,
            ylog,
            x_min,
            x_max,
            y_min,
            y_max,
            bitmap,
        )?;

        if let Some(loc) = &self.legend_loc.clone()
            && !self.legend_labels.is_empty()
        {
            crate::figure::axes_legend::draw_legend(
                chart,
                Some(loc),
                &self.legend_labels,
                font_scale,
                x_min,
                x_max,
                y_min,
                y_max,
            )?;
        }

        // 渲染 axes 标题（在数据区域上方的 margin_top 区域内）
        crate::figure::axes_title::draw_title(
            chart,
            &self.title,
            self.title_fontsize,
            font_scale,
            self.title_color,
            self.title_family.as_deref(),
            &self.title_loc,
            x_min,
            x_max,
            y_min,
            y_max,
        )?;

        Ok(())
    }

    pub fn parse_hist_data(x: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<f64>>> {
        if let Ok(lst) = x.extract::<Vec<Bound<'_, PyAny>>>() {
            if lst.is_empty() {
                return Ok(Vec::new());
            }
            if lst[0].extract::<f64>().is_ok() {
                let flat: Vec<f64> = lst
                    .iter()
                    .map(|item| item.extract::<f64>())
                    .collect::<Result<Vec<f64>, _>>()
                    .map_err(|e| PyValueError::new_err(format!("hist data parse error: {}", e)))?;
                Ok(vec![flat])
            } else {
                let multi: Vec<Vec<f64>> = lst
                    .iter()
                    .map(|item| {
                        item.extract::<Vec<f64>>().map_err(|e| {
                            PyValueError::new_err(format!("hist multi-data parse error: {}", e))
                        })
                    })
                    .collect::<Result<Vec<Vec<f64>>, _>>()?;
                Ok(multi)
            }
        } else {
            Err(PyValueError::new_err(
                "hist data must be a list or list of lists",
            ))
        }
    }

    pub fn parse_color_list(
        color: &Bound<'_, PyAny>,
        expected_len: usize,
    ) -> PyResult<Vec<String>> {
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
            Ok((0..expected_len).map(default_color_str).collect())
        }
    }
}
