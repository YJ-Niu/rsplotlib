use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyAny};
use plotters::coord::types::RangedCoordf64;
use plotters::style::{ShapeStyle, text_anchor::{HPos, VPos}};
use plotters::prelude::*;

use crate::colors::{RgbColor, parse_color, default_color, default_color_str, shape_style, to_plotters_color};
use crate::elements::PlotElement;
use crate::marker::draw_marker;
use crate::colormap::{viridis_color, plasma_color, inferno_color, magma_color, cool_color, spring_color, summer_color, autumn_color, winter_color};
use crate::colors::median;
use crate::axis::{Axis, Patch, SpineDict};

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
        x: Vec<Option<f64>>,
        y: Vec<Option<f64>>,
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
    ) {
        let color = c.or(color);
        let linewidth = lw.unwrap_or(linewidth);
        let linestyle = ls.as_deref().unwrap_or(linestyle);
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        let linestyle_val = linestyle.to_string();
        self.elements.push(PlotElement::Line {
            x,
            y,
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
    }

    #[pyo3(signature = (x, y, s=20.0, c=None, marker="o", label=None, alpha=1.0))]
    #[allow(clippy::too_many_arguments)]
    pub fn scatter(
        &mut self,
        x: Vec<f64>,
        y: Vec<f64>,
        s: f64,
        c: Option<String>,
        marker: &str,
        label: Option<String>,
        alpha: f64,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        let c_val = c.clone().unwrap_or_default();
        let marker_val = marker.to_string();
        self.elements.push(PlotElement::Scatter {
            x,
            y,
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
    }

    #[pyo3(signature = (x, height, width=0.8, color=None, label=None))]
    pub fn bar(
        &mut self,
        x: Vec<f64>,
        height: Vec<f64>,
        width: f64,
        color: Option<String>,
        label: Option<String>,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::Bar {
            x,
            height,
            width,
            color: color_val.clone(),
            label: label.clone(),
            color_idx: idx,
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), None));
        }
    }

    #[pyo3(signature = (y, width, height=0.8, color=None, label=None))]
    pub fn barh(
        &mut self,
        y: Vec<f64>,
        width: Vec<f64>,
        height: f64,
        color: Option<String>,
        label: Option<String>,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::BarH {
            y,
            width,
            height,
            color: color_val.clone(),
            label: label.clone(),
            color_idx: idx,
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), None));
        }
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

    pub fn set_xlabel(&mut self, text: String) {
        self.xlabel = text;
    }

    pub fn set_ylabel(&mut self, text: String) {
        self.ylabel = text;
    }

    pub fn set_title(&mut self, text: String) {
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

    #[pyo3(signature = (visible=None, c=None, lw=None, ls=None, axis=None))]
    pub fn grid(&mut self, visible: Option<bool>, c: Option<String>, lw: Option<f64>, ls: Option<String>, axis: Option<String>) {
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

    #[pyo3(signature = (x, y1, y2=0.0, color=None, alpha=0.3, label=None))]
    pub fn fill_between(
        &mut self,
        x: Vec<f64>,
        y1: Vec<f64>,
        y2: f64,
        color: Option<String>,
        alpha: f64,
        label: Option<String>,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::FillBetween {
            x,
            y1,
            y2,
            color: color_val.clone(),
            alpha,
            label: label.clone(),
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), None));
        }
    }

    #[pyo3(signature = (x, y, yerr=None, xerr=None, fmt="o", color=None, label=None, capsize=3.0))]
    #[allow(clippy::too_many_arguments)]
    pub fn errorbar(
        &mut self,
        x: Vec<f64>,
        y: Vec<f64>,
        yerr: Option<f64>,
        xerr: Option<f64>,
        fmt: &str,
        color: Option<String>,
        label: Option<String>,
        capsize: f64,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::ErrorBar {
            x,
            y,
            yerr,
            xerr,
            fmt: fmt.to_string(),
            color: color_val.clone(),
            label: label.clone(),
            capsize,
        });
        if let Some(lbl) = label {
            let col = parse_color(&color.unwrap_or_default(), idx).unwrap_or_else(|_| default_color(idx));
            self.legend_labels.push((lbl, col, "-".to_string(), Some(fmt.to_string())));
        }
    }

    #[pyo3(signature = (x, y, linefmt="-", markerfmt="o", label=None))]
    pub fn stem(
        &mut self,
        x: Vec<f64>,
        y: Vec<f64>,
        linefmt: &str,
        markerfmt: &str,
        label: Option<String>,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        self.elements.push(PlotElement::Stem {
            x,
            y,
            linefmt: linefmt.to_string(),
            markerfmt: markerfmt.to_string(),
            label: label.clone(),
        });
        if let Some(lbl) = label {
            let col = default_color(idx);
            self.legend_labels.push((lbl, col, linefmt.to_string(), Some(markerfmt.to_string())));
        }
    }

    #[pyo3(signature = (x, y, where_="pre", label=None, color=None, linestyle="-", linewidth=1.5))]
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        x: Vec<f64>,
        y: Vec<f64>,
        where_: &str,
        label: Option<String>,
        color: Option<String>,
        linestyle: &str,
        linewidth: f64,
    ) {
        let idx = self.element_count;
        self.element_count += 1;
        let color_val = color.clone().unwrap_or_default();
        self.elements.push(PlotElement::Step {
            x,
            y,
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
    }

    #[pyo3(signature = (x, labels=None, vert=true))]
    pub fn boxplot(&mut self, x: Vec<Vec<f64>>, labels: Option<Vec<String>>, vert: bool) {
        self.elements.push(PlotElement::BoxPlot {
            data: x,
            labels,
            vert,
        });
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
    pub fn compute_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for el in &self.elements {
            match el {
                PlotElement::Line { x, y, .. } => {
                    for v in x.iter().flatten() {
                        if *v < x_min { x_min = *v; }
                        if *v > x_max { x_max = *v; }
                    }
                    for v in y.iter().flatten() {
                        if *v < y_min { y_min = *v; }
                        if *v > y_max { y_max = *v; }
                    }
                }
                PlotElement::Scatter { x, y, .. } => {
                    for &v in x {
                        if v < x_min { x_min = v; }
                        if v > x_max { x_max = v; }
                    }
                    for &v in y {
                        if v < y_min { y_min = v; }
                        if v > y_max { y_max = v; }
                    }
                }
                PlotElement::Bar { x, height, width, .. } => {
                    for &v in x {
                        if v < x_min { x_min = v; }
                        if v > x_max { x_max = v; }
                    }
                    for &v in height {
                        if v < y_min { y_min = v; }
                        if v > y_max { y_max = v; }
                    }
                    if !x.is_empty() && !height.is_empty() {
                        let last_x = x[x.len() - 1];
                        if last_x + width > x_max { x_max = last_x + width; }
                    }
                    if y_min > 0.0 { y_min = 0.0; }
                }
                PlotElement::BarH { y, width, .. } => {
                    for &v in y {
                        if v < y_min { y_min = v; }
                        if v > y_max { y_max = v; }
                    }
                    for &v in width {
                        if v < x_min { x_min = v; }
                        if v > x_max { x_max = v; }
                    }
                    if !width.is_empty() {
                        let last_w = width[width.len() - 1];
                        if last_w > x_max { x_max = last_w; }
                    }
                    if x_min > 0.0 { x_min = 0.0; }
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
                    if x_start < x_min { x_min = x_start; }
                    if x_end > x_max { x_max = x_end; }
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
                    if y_min > 0.0 { y_min = 0.0; }
                    if max_count > y_max { y_max = max_count; }
                }
                PlotElement::Image { data, .. } => {
                    if data.is_empty() || data[0].is_empty() { continue; }
                    x_min = 0.0;
                    x_max = data[0].len() as f64;
                    y_min = 0.0;
                    y_max = data.len() as f64;
                }
                PlotElement::Text { x, y, .. } => {
                    if *x < x_min { x_min = *x; }
                    if *x > x_max { x_max = *x; }
                    if *y < y_min { y_min = *y; }
                    if *y > y_max { y_max = *y; }
                }
                PlotElement::HLine { y, .. } => {
                    if x_min == f64::INFINITY { x_min = -1.0; x_max = 1.0; }
                    if *y < y_min { y_min = *y; }
                    if *y > y_max { y_max = *y; }
                }
                PlotElement::VLine { x, .. } => {
                    if y_min == f64::INFINITY { y_min = -1.0; y_max = 1.0; }
                    if *x < x_min { x_min = *x; }
                    if *x > x_max { x_max = *x; }
                }
                PlotElement::Pie { .. } => {
                    if x_min > -1.5 { x_min = -1.5; }
                    if x_max < 1.5 { x_max = 1.5; }
                    if y_min > -1.5 { y_min = -1.5; }
                    if y_max < 1.5 { y_max = 1.5; }
                }
                PlotElement::FillBetween { x, y1, y2, .. } => {
                    for &v in x {
                        if v < x_min { x_min = v; }
                        if v > x_max { x_max = v; }
                    }
                    for &v in y1 {
                        if v < y_min { y_min = v; }
                        if v > y_max { y_max = v; }
                    }
                    if *y2 < y_min { y_min = *y2; }
                    if *y2 > y_max { y_max = *y2; }
                }
                PlotElement::ErrorBar { x, y, yerr, .. } => {
                    for &v in x {
                        if v < x_min { x_min = v; }
                        if v > x_max { x_max = v; }
                    }
                    for &v in y {
                        if v < y_min { y_min = v; }
                        if v > y_max { y_max = v; }
                    }
                    if let Some(ye) = yerr {
                        for &yv in y {
                            if yv - ye < y_min { y_min = yv - ye; }
                            if yv + ye > y_max { y_max = yv + ye; }
                        }
                    }
                }
                PlotElement::Stem { x, y, .. } => {
                    for &v in x {
                        if v < x_min { x_min = v; }
                        if v > x_max { x_max = v; }
                    }
                    for &v in y {
                        if v < y_min { y_min = v; }
                        if v > y_max { y_max = v; }
                    }
                    if y_min > 0.0 { y_min = 0.0; }
                }
                PlotElement::Step { x, y, .. } => {
                    for &v in x {
                        if v < x_min { x_min = v; }
                        if v > x_max { x_max = v; }
                    }
                    for &v in y {
                        if v < y_min { y_min = v; }
                        if v > y_max { y_max = v; }
                    }
                }
                PlotElement::BoxPlot { data, .. } => {
                    for series in data {
                        for &v in series {
                            if v < y_min { y_min = v; }
                            if v > y_max { y_max = v; }
                        }
                    }
                    if y_min > 0.0 { y_min = 0.0; }
                    if x_min > 0.0 { x_min = 0.0; }
                    let n = data.len() as f64;
                    if n > x_max { x_max = n + 1.0; }
                }
                PlotElement::Annotate { xy, xytext, .. } => {
                    let (xv, yv) = *xy;
                    if xv < x_min { x_min = xv; }
                    if xv > x_max { x_max = xv; }
                    if yv < y_min { y_min = yv; }
                    if yv > y_max { y_max = yv; }
                    if let Some((xt, yt)) = xytext {
                        if *xt < x_min { x_min = *xt; }
                        if *xt > x_max { x_max = *xt; }
                        if *yt < y_min { y_min = *yt; }
                        if *yt > y_max { y_max = *yt; }
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
    ) -> PyResult<()>
    where
        DB::ErrorType: 'static,
    {
        let bg_color = parse_color(&self.facecolor, 0).unwrap_or(RgbColor(255, 255, 255));
        chart.plotting_area().fill(&to_plotters_color(bg_color))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to fill background: {}", e)))?;

        let mut mesh_builder = chart.configure_mesh();

        let frame_color = RgbColor(50, 50, 50);
        let frame_lw = ((0.6f64).ceil().max(1.0)) as u32;
        let frame_style: ShapeStyle = to_plotters_color(frame_color).stroke_width(frame_lw).into();

        let computed_xticks: Option<Vec<f64>> = self.xaxis_major_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (x_min, x_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| self.xticks_val.clone());

        let computed_yticks: Option<Vec<f64>> = self.yaxis_major_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (y_min, y_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| self.yticks_val.clone());

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

        let label_size: i32 = self.tick_labelsize as i32;
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

        mesh_builder.draw()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw mesh: {}", e)))?;

        let draw_grid_line = |chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
                              x1: f64, y1: f64, x2: f64, y2: f64,
                              color: RgbColor, lw: f64, _ls: &str| -> PyResult<()> {
            let rgb = to_plotters_color(color);
            let lw_u32 = (lw.max(0.1)).ceil() as u32;
            let style = rgb.stroke_width(lw_u32);
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x1, y1), (x2, y2)], style,
            ))).map_err(|e| PyRuntimeError::new_err(format!("Grid line: {}", e)))?;
            Ok(())
        };

        if self.grid_visible {
            if self.grid_axis == "both" || self.grid_axis == "x" {
                if let Some(ref ticks) = computed_xticks {
                    for &tx in ticks {
                        if tx >= x_min && tx <= x_max {
                            draw_grid_line(chart, tx, y_min, tx, y_max, major_color, major_lw_f64, "-")?;
                        }
                    }
                }
            }
            if self.grid_axis == "both" || self.grid_axis == "y" {
                if let Some(ref ticks) = computed_yticks {
                    for &ty in ticks {
                        if ty >= y_min && ty <= y_max {
                            draw_grid_line(chart, x_min, ty, x_max, ty, major_color, major_lw_f64, "-")?;
                        }
                    }
                }
            }
        }

        if self.minor_grid_visible {
            if self.minor_grid_x_visible || (!self.minor_grid_x_visible && !self.minor_grid_y_visible) {
                if let Some(ref ticks) = computed_xminor {
                    for &tx in ticks {
                        if tx > x_min && tx < x_max {
                            draw_grid_line(chart, tx, y_min, tx, y_max, minor_color, minor_lw_f64, "-")?;
                        }
                    }
                }
            }
            if self.minor_grid_y_visible || (!self.minor_grid_x_visible && !self.minor_grid_y_visible) {
                if let Some(ref ticks) = computed_yminor {
                    for &ty in ticks {
                        if ty > y_min && ty < y_max {
                            draw_grid_line(chart, x_min, ty, x_max, ty, minor_color, minor_lw_f64, "-")?;
                        }
                    }
                }
            }
        }

        for el in &self.elements {
            match el {
                PlotElement::Line { x, y, color, linestyle, marker, linewidth, color_idx, solid_capstyle, .. } => {
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| default_color(*color_idx));
                    if x.len() >= 2 && x.len() == y.len() {
                        let points: Vec<(f64, f64)> = x.iter().zip(y.iter())
                            .filter_map(|(xv, yv)| match (xv, yv) {
                                (Some(xv), Some(yv)) => Some((*xv, *yv)),
                                _ => None,
                            })
                            .collect();
                        if points.len() >= 2 {
                            let style = shape_style(col, *linewidth, linestyle);
                            chart.draw_series(LineSeries::new(points.clone(), style))
                                .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw line: {}", e)))?;
                            if solid_capstyle == "round" && *linewidth > 1.0 && marker.as_ref().map_or(true, |m| m.is_empty()) {
                                let rgb = to_plotters_color(col);
                                let circle_r = *linewidth / 2.0;
                                let n_seg = 16;
                                let cap_points = [points.first().unwrap().clone(), points.last().unwrap().clone()];
                                for pt in cap_points.iter() {
                                    let mut pts = Vec::with_capacity(n_seg + 1);
                                    for i in 0..=n_seg {
                                        let angle = i as f64 * 2.0 * std::f64::consts::PI / n_seg as f64;
                                        pts.push((pt.0 + circle_r * angle.cos(), pt.1 + circle_r * angle.sin()));
                                    }
                                    chart.draw_series(std::iter::once(PathElement::new(pts, rgb.filled())))
                                        .map_err(|e| PyRuntimeError::new_err(format!("Cap path: {}", e)))?;
                                }
                            }
                        }
                    }
                    if let Some(marker_name) = marker {
                        if !marker_name.is_empty() && x.len() == y.len() {
                            let col2 = parse_color(color, *color_idx).unwrap_or_else(|_| default_color(*color_idx));
                            let rgb = to_plotters_color(col2);
                            let marker_size = *linewidth * 3.0 + 3.0;
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
                        draw_marker(chart, marker, xv, yv, size.max(2.0), rgb)
                            .map_err(|e| PyRuntimeError::new_err(format!("Failed to draw scatter: {}", e)))?;
                    }
                }
                PlotElement::Bar { x, height, width, color, color_idx, .. } => {
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| default_color(*color_idx));
                    let rgb = to_plotters_color(col);
                    let fill_style: ShapeStyle = rgb.filled().into();
                    for (&xv, &h) in x.iter().zip(height.iter()) {
                        let y0 = 0.0f64.max(y_min);
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(xv - width / 2.0, y0), (xv + width / 2.0, h)],
                            fill_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw bar: {}", e)))?;
                    }
                }
                PlotElement::BarH { y, width, height, color, color_idx, .. } => {
                    let c = if color.is_empty() { default_color(*color_idx) } else { parse_color(color, *color_idx)? };
                    let rgb = to_plotters_color(c);
                    let fill_style: ShapeStyle = rgb.filled().into();
                    for (&yv, &wv) in y.iter().zip(width.iter()) {
                        let bar_y0 = yv - height / 2.0;
                        let bar_y1 = yv + height / 2.0;
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(0.0, bar_y0), (wv, bar_y1)],
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
                    let font: FontDesc = ("sans-serif", *fontsize as f64).into();
                    let colored_font = font.color(&to_plotters_color(*color));
                    let text_style: TextStyle = colored_font.into();
                    chart.draw_series(std::iter::once(plotters::element::Text::new(
                        text.clone(),
                        (*x, *y),
                        text_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw text: {}", e)))?;
                }
                PlotElement::HLine { y, color, linestyle, linewidth, color_idx } => {
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| RgbColor(0, 0, 0));
                    draw_grid_line(chart, x_min, *y, x_max, *y, col, *linewidth, linestyle)?;
                }
                PlotElement::VLine { x, color, linestyle, linewidth, color_idx } => {
                    let col = parse_color(color, *color_idx).unwrap_or_else(|_| RgbColor(0, 0, 0));
                    let x_val = if self.xscale == "log" { x.max(1e-10).log10() } else { *x };
                    draw_grid_line(chart, x_val, y_min, x_val, y_max, col, *linewidth, linestyle)?;
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
                                    l.clone(), (lx, ly), ("sans-serif", 12),
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
                                text, (tx, ty), ("sans-serif", 11),
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
                        points.push((xv, yv));
                    }
                    for (&xv, _) in x.iter().rev().zip(y1.iter().rev()) {
                        points.push((xv, *y2));
                    }
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
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        if let Some(ye) = yerr {
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(xv, yv - ye), (xv, yv + ye)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar line: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(xv - cap_half, yv - ye), (xv + cap_half, yv - ye)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar cap: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(xv - cap_half, yv + ye), (xv + cap_half, yv + ye)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar cap: {}", e)))?;
                        }
                        if let Some(xe) = xerr {
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(xv - xe, yv), (xv + xe, yv)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xline: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(xv - xe, yv - cap_half), (xv - xe, yv + cap_half)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e)))?;
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(xv + xe, yv - cap_half), (xv + xe, yv + cap_half)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("ErrorBar xcap: {}", e)))?;
                        }
                        if !fmt.is_empty() {
                            let marker_name = fmt;
                            draw_marker(chart, marker_name, xv, yv, 5.0, rgb)
                                .map_err(|e| PyRuntimeError::new_err(format!("ErrorBar marker: {}", e)))?;
                        }
                    }
                }
                PlotElement::Stem { x, y, linefmt, markerfmt, .. } => {
                    let col = RgbColor(0, 0, 200);
                    let rgb = to_plotters_color(col);
                    if linefmt == "-" || linefmt.is_empty() {
                        let line_style = shape_style(col, 1.0, linefmt);
                        for (&xv, &yv) in x.iter().zip(y.iter()) {
                            chart.draw_series(std::iter::once(PathElement::new(
                                vec![(xv, 0.0), (xv, yv)], line_style,
                            ))).map_err(|e| PyRuntimeError::new_err(format!("Stem line: {}", e)))?;
                        }
                    } else {
                        for (&xv, &yv) in x.iter().zip(y.iter()) {
                            draw_grid_line(chart, xv, 0.0, xv, yv, col, 1.0, linefmt)?;
                        }
                    }
                    for (&xv, &yv) in x.iter().zip(y.iter()) {
                        draw_marker(chart, markerfmt, xv, yv, 5.0, rgb)
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
                            points.push((x[0], y[0]));
                            for i in 1..x.len() {
                                points.push((x[i], y[i - 1]));
                                points.push((x[i], y[i]));
                            }
                        }
                        "post" => {
                            for i in 0..x.len() - 1 {
                                points.push((x[i], y[i]));
                                points.push((x[i], y[i + 1]));
                            }
                            points.push((x[x.len() - 1], y[y.len() - 1]));
                        }
                        _ => {
                            points.push((x[0], y[0]));
                            for i in 1..x.len() {
                                let mid = (x[i - 1] + x[i]) / 2.0;
                                points.push((mid, y[i - 1]));
                                points.push((mid, y[i]));
                            }
                            points.push((x[x.len() - 1], y[y.len() - 1]));
                        }
                    }
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
                        let cx = (i + 1) as f64;
                        let col = to_plotters_color(default_color(i));
                        let fill_style: ShapeStyle = col.mix(0.3).filled().into();
                        let line_style: ShapeStyle = col.stroke_width(2).into();
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx, lower_whisker), (cx, upper_whisker)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot whisker: {}", e)))?;
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(cx - box_width / 2.0, q1), (cx + box_width / 2.0, q3)], fill_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot box: {}", e)))?;
                        chart.draw_series(std::iter::once(Rectangle::new(
                            [(cx - box_width / 2.0, q1), (cx + box_width / 2.0, q3)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot border: {}", e)))?;
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx - box_width / 2.0, med), (cx + box_width / 2.0, med)],
                            col.stroke_width(2).filled(),
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot median: {}", e)))?;
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx - box_width / 4.0, lower_whisker), (cx + box_width / 4.0, lower_whisker)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                        chart.draw_series(std::iter::once(PathElement::new(
                            vec![(cx - box_width / 4.0, upper_whisker), (cx + box_width / 4.0, upper_whisker)], line_style,
                        ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot cap: {}", e)))?;
                        if let Some(lbls) = labels {
                            if let Some(l) = lbls.get(i) {
                                chart.draw_series(std::iter::once(plotters::element::Text::new(
                                    l.clone(), (cx, -0.3), ("sans-serif", 11),
                                ))).map_err(|e| PyRuntimeError::new_err(format!("BoxPlot label: {}", e)))?;
                            }
                        }
                    }
                }
                PlotElement::Annotate { text, xy, xytext, fontsize, color } => {
                    let col = parse_color(color, 0).unwrap_or_else(|_| RgbColor(0, 0, 0));
                    let rgb = to_plotters_color(col);
                    let (tx, ty) = xytext.unwrap_or((xy.0 + 0.2, xy.1 + 0.2));
                    let arrow_style: ShapeStyle = rgb.stroke_width(1).into();
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(tx, ty), (xy.0, xy.1)], arrow_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Annotate arrow: {}", e)))?;
                    chart.draw_series(std::iter::once(plotters::element::Text::new(
                        text.clone(), (tx, ty), ("sans-serif", *fontsize),
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

                for (i, (label, color, _ls, marker_opt)) in self.legend_labels.iter().enumerate() {
                    let y_pos = box_y1 + entry_height * 0.75 + i as f64 * entry_height;
                    let x_line_start = box_x1 + x_range * 0.015;
                    let x_line_end = box_x1 + x_range * 0.06;
                    let x_text = box_x1 + x_range * 0.07;

                    let rgb = to_plotters_color(*color);
                    let line_style: ShapeStyle = rgb.stroke_width(2).into();
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(x_line_start, y_pos), (x_line_end, y_pos)], line_style,
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend line: {}", e)))?;

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
                        ("sans-serif", 11),
                    ))).map_err(|e| PyRuntimeError::new_err(format!("Failed to draw legend text: {}", e)))?;
                }
            }
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