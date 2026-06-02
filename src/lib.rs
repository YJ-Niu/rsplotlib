use std::sync::Mutex;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use plotters::coord::types::RangedCoordf64;
use plotters::style::ShapeStyle;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, VPos};
use plotters::style::register_font;

// ==================== Color System ====================

#[derive(Clone, Copy)]
struct RgbColor(u8, u8, u8);

const DEFAULT_COLORS: &[&str] = &[
    "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd",
    "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf",
];

fn parse_color(name: &str, color_idx: usize) -> PyResult<RgbColor> {
    let trimmed = name.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
            return Ok(RgbColor(r, g, b));
        }
        return Err(PyValueError::new_err("Hex color must be #RRGGBB"));
    }
    let c = match trimmed.to_lowercase().as_str() {
        "r" | "red" => RgbColor(255, 0, 0),
        "g" | "green" => RgbColor(0, 128, 0),
        "b" | "blue" => RgbColor(0, 0, 255),
        "c" | "cyan" => RgbColor(0, 255, 255),
        "m" | "magenta" => RgbColor(255, 0, 255),
        "y" | "yellow" => RgbColor(255, 255, 0),
        "k" | "black" => RgbColor(0, 0, 0),
        "w" | "white" => RgbColor(255, 255, 255),
        "grey" | "gray" => RgbColor(128, 128, 128),
        "darkgrey" | "darkgray" => RgbColor(169, 169, 169),
        "lightgrey" | "lightgray" => RgbColor(211, 211, 211),
        "dimgrey" | "dimgray" => RgbColor(105, 105, 105),
        "slategrey" | "slategray" => RgbColor(112, 128, 144),
        _ => return Ok(default_color(color_idx)),
    };
    Ok(c)
}

fn default_color(idx: usize) -> RgbColor {
    let hex = DEFAULT_COLORS[idx % DEFAULT_COLORS.len()];
    let hex = hex.strip_prefix('#').unwrap();
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    RgbColor(r, g, b)
}

fn default_color_str(idx: usize) -> String {
    DEFAULT_COLORS[idx % DEFAULT_COLORS.len()].to_string()
}

fn shape_style(color: RgbColor, linewidth: f64, linestyle: &str) -> ShapeStyle {
    let rgb = RGBColor(color.0, color.1, color.2);
    match linestyle {
        "--" => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
        ":" => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
        "-." => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
        _ => rgb.mix(1.0).stroke_width(linewidth as u32).into(),
    }
}

fn to_plotters_color(c: RgbColor) -> RGBColor {
    RGBColor(c.0, c.1, c.2)
}

// ==================== Plot Elements ====================

#[derive(Clone)]
#[allow(dead_code)]
enum PlotElement {
    Line {
        x: Vec<Option<f64>>,
        y: Vec<Option<f64>>,
        label: Option<String>,
        color: String,
        linestyle: String,
        marker: Option<String>,
        linewidth: f64,
        color_idx: usize,
        solid_capstyle: String,
    },
    Scatter {
        x: Vec<f64>,
        y: Vec<f64>,
        s: f64,
        c: String,
        marker: String,
        label: Option<String>,
        alpha: f64,
        color_idx: usize,
    },
    Bar {
        x: Vec<f64>,
        height: Vec<f64>,
        width: f64,
        color: String,
        label: Option<String>,
        color_idx: usize,
    },
    BarH {
        y: Vec<f64>,
        width: Vec<f64>,
        height: f64,
        color: String,
        label: Option<String>,
        color_idx: usize,
    },
    Hist {
        data_all: Vec<Vec<f64>>,
        bins: usize,
        density: bool,
        histtype: String,
        label: Option<String>,
        alpha: f64,
        colors: Vec<String>,
        color_idx: usize,
        bin_edges: Option<Vec<f64>>,
    },
    Image {
        data: Vec<Vec<f64>>,
        cmap: String,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        fontsize: i32,
        color: RgbColor,
    },
    HLine {
        y: f64,
        color: String,
        linestyle: String,
        linewidth: f64,
        color_idx: usize,
    },
    VLine {
        x: f64,
        color: String,
        linestyle: String,
        linewidth: f64,
        color_idx: usize,
    },
    Pie {
        x: Vec<f64>,
        labels: Option<Vec<String>>,
        colors: Option<Vec<String>>,
        autopct: Option<String>,
        startangle: f64,
    },
    FillBetween {
        x: Vec<f64>,
        y1: Vec<f64>,
        y2: f64,
        color: String,
        alpha: f64,
        label: Option<String>,
    },
    ErrorBar {
        x: Vec<f64>,
        y: Vec<f64>,
        yerr: Option<f64>,
        xerr: Option<f64>,
        fmt: String,
        color: String,
        label: Option<String>,
        capsize: f64,
    },
    Stem {
        x: Vec<f64>,
        y: Vec<f64>,
        linefmt: String,
        markerfmt: String,
        label: Option<String>,
    },
    Step {
        x: Vec<f64>,
        y: Vec<f64>,
        where_: String,
        label: Option<String>,
        color: String,
        linestyle: String,
        linewidth: f64,
    },
    BoxPlot {
        data: Vec<Vec<f64>>,
        labels: Option<Vec<String>>,
        vert: bool,
    },
    Annotate {
        text: String,
        xy: (f64, f64),
        xytext: Option<(f64, f64)>,
        fontsize: f64,
        color: String,
    },
}

// ==================== Marker Rendering ====================

fn draw_marker<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
    marker: &str,
    x: f64,
    y: f64,
    size: f64,
    color: RGBColor,
) -> PyResult<()> {
    let s = size.max(2.0);
    let style: ShapeStyle = color.filled().into();
    match marker {
        "o" => {
            let r = s;
            let n = 20;
            let mut points = Vec::with_capacity(n + 1);
            for i in 0..=n {
                let angle = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
                points.push((x + r * angle.cos(), y + r * angle.sin()));
            }
            chart.draw_series(std::iter::once(PathElement::new(points, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "s" => {
            chart.draw_series(std::iter::once(Rectangle::new(
                [(x - s / 2.0, y - s / 2.0), (x + s / 2.0, y + s / 2.0)],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "^" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x - s / 2.0, y + s / 2.0),
                    (x + s / 2.0, y + s / 2.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "D" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x + s / 2.0, y),
                    (x, y + s / 2.0),
                    (x - s / 2.0, y),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "v" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y + s / 2.0),
                    (x - s / 2.0, y - s / 2.0),
                    (x + s / 2.0, y - s / 2.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "*" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x + s / 6.0, y - s / 6.0),
                    (x + s / 2.0, y),
                    (x + s / 6.0, y + s / 6.0),
                    (x, y + s / 2.0),
                    (x - s / 6.0, y + s / 6.0),
                    (x - s / 2.0, y),
                    (x - s / 6.0, y - s / 6.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "p" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x, y - s / 2.0),
                    (x + s / 3.0, y - s / 4.0),
                    (x + s / 2.0, y),
                    (x + s / 3.0, y + s / 4.0),
                    (x, y + s / 2.0),
                    (x - s / 3.0, y + s / 4.0),
                    (x - s / 2.0, y),
                    (x - s / 3.0, y - s / 4.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "h" => {
            chart.draw_series(std::iter::once(Polygon::new(
                vec![
                    (x - s / 2.0, y - s / 4.0),
                    (x, y - s / 2.0),
                    (x + s / 2.0, y - s / 4.0),
                    (x + s / 2.0, y + s / 4.0),
                    (x, y + s / 2.0),
                    (x - s / 2.0, y + s / 4.0),
                ],
                style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "x" => {
            let line_style: ShapeStyle = color.stroke_width(2).into();
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - s / 3.0, y - s / 3.0), (x + s / 3.0, y + s / 3.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - s / 3.0, y + s / 3.0), (x + s / 3.0, y - s / 3.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        "+" => {
            let line_style: ShapeStyle = color.stroke_width(2).into();
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x - s / 3.0, y), (x + s / 3.0, y)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(x, y - s / 3.0), (x, y + s / 3.0)],
                line_style,
            )))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
        _ => {
            let r = 3.0;
            let n = 16;
            let mut points = Vec::with_capacity(n + 1);
            for i in 0..=n {
                let angle = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
                points.push((x + r * angle.cos(), y + r * angle.sin()));
            }
            chart.draw_series(std::iter::once(PathElement::new(points, style)))
                .map_err(|e| PyRuntimeError::new_err(format!("Marker error: {}", e)))?;
        }
    }
    Ok(())
}

// ==================== Axes Class ====================

#[pyclass(skip_from_py_object)]
pub struct Axes {
    elements: Vec<PlotElement>,
    xlabel: String,
    ylabel: String,
    title: String,
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
    grid_visible: bool,
    legend_loc: Option<String>,
    element_count: usize,
    legend_labels: Vec<(String, RgbColor, String, Option<String>)>,
    xscale: String,
    yscale: String,
    xticks_val: Option<Vec<f64>>,
    xtick_labels: Option<Vec<String>>,
    yticks_val: Option<Vec<f64>>,
    ytick_labels: Option<Vec<String>>,
    is_twin_x: bool,
    is_twin_y: bool,
    twin_axes: Vec<Axes>,
    facecolor: String,
    spine_top: bool,
    spine_bottom: bool,
    spine_left: bool,
    spine_right: bool,
    grid_color: Option<String>,
    grid_linewidth: Option<f64>,
    grid_linestyle: Option<String>,
    grid_axis: String,
    minor_grid_visible: bool,
    minor_grid_color: Option<String>,
    minor_grid_linewidth: Option<f64>,
    minor_grid_linestyle: Option<String>,
    tick_bottom: bool,
    tick_top: bool,
    tick_left: bool,
    tick_right: bool,
    tick_labelsize: f64,
    self_py: Option<Py<PyAny>>,
    xaxis_major_locator: Option<Py<PyAny>>,
    xaxis_minor_locator: Option<Py<PyAny>>,
    yaxis_major_locator: Option<Py<PyAny>>,
    yaxis_minor_locator: Option<Py<PyAny>>,
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
    fn new() -> Self {
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
    fn plot(
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
    fn scatter(
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
    fn bar(
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
    fn barh(
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
    fn hist(
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
    fn imshow(&mut self, x: Vec<Vec<f64>>, cmap: &str, aspect: &str) {
        self.elements.push(PlotElement::Image {
            data: x,
            cmap: cmap.to_string(),
        });
    }

    fn set_xlabel(&mut self, text: String) {
        self.xlabel = text;
    }

    fn set_ylabel(&mut self, text: String) {
        self.ylabel = text;
    }

    fn set_title(&mut self, text: String) {
        self.title = text;
    }

    #[pyo3(signature = (loc="best"))]
    fn legend(&mut self, loc: &str) {
        self.legend_loc = Some(loc.to_string());
    }

    #[pyo3(signature = (_v=None))]
    fn axis(&mut self, _v: Option<String>) {
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
    fn grid(&mut self, visible: Option<bool>, c: Option<String>, lw: Option<f64>, ls: Option<String>, axis: Option<String>) {
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

    fn set_xlim(&mut self, left: f64, right: f64) {
        self.xlim = Some((left, right));
    }

    fn set_ylim(&mut self, bottom: f64, top: f64) {
        self.ylim = Some((bottom, top));
    }

    #[pyo3(signature = (x, y, text, fontsize=None, color=None, c=None, _family=None))]
    fn text(
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

    fn axhline(
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

    fn axvline(
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
    fn pie(
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
    fn fill_between(
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
    fn errorbar(
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
    fn stem(
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
    fn step(
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
    fn boxplot(&mut self, x: Vec<Vec<f64>>, labels: Option<Vec<String>>, vert: bool) {
        self.elements.push(PlotElement::BoxPlot {
            data: x,
            labels,
            vert,
        });
    }

    #[pyo3(signature = (text, xy, xytext=None, fontsize=12.0, color="black"))]
    fn annotate(
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

    fn set_xscale(&mut self, scale: &str) {
        self.xscale = scale.to_string();
    }

    fn set_yscale(&mut self, scale: &str) {
        self.yscale = scale.to_string();
    }

    #[pyo3(signature = (ticks=None, labels=None))]
    fn xticks(&mut self, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) {
        self.xticks_val = ticks;
        self.xtick_labels = labels;
    }

    #[pyo3(signature = (ticks=None, labels=None))]
    fn yticks(&mut self, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) {
        self.yticks_val = ticks;
        self.ytick_labels = labels;
    }

    fn twinx(&mut self) -> Axes {
        let mut twin = Axes::new();
        twin.xlim = self.xlim;
        twin.is_twin_x = true;
        self.twin_axes.push(twin.clone());
        twin
    }

    fn twiny(&mut self) -> Axes {
        let mut twin = Axes::new();
        twin.ylim = self.ylim;
        twin.is_twin_y = true;
        self.twin_axes.push(twin.clone());
        twin
    }

    fn cla(&mut self) {
        self.elements.clear();
        self.legend_labels.clear();
        self.element_count = 0;
    }

    #[pyo3(signature = (axis="both", labelsize=None, rotation=None, bottom=None, top=None, left=None, right=None))]
    #[allow(unused_variables)]
    fn tick_params(&mut self, axis: &str, labelsize: Option<f64>, rotation: Option<f64>, bottom: Option<bool>, top: Option<bool>, left: Option<bool>, right: Option<bool>) {
        if let Some(v) = labelsize { self.tick_labelsize = v; }
        if let Some(v) = bottom { self.tick_bottom = v; }
        if let Some(v) = top { self.tick_top = v; }
        if let Some(v) = left { self.tick_left = v; }
        if let Some(v) = right { self.tick_right = v; }
    }

    fn _axis_off(&mut self) {
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

    fn set_aspect(&mut self, _aspect: &str) {
    }

    fn set_xaxis_major_locator(&mut self, locator: Py<PyAny>) {
        self.xaxis_major_locator = Some(locator);
    }

    fn set_xaxis_minor_locator(&mut self, locator: Py<PyAny>) {
        self.xaxis_minor_locator = Some(locator);
    }

    fn set_yaxis_major_locator(&mut self, locator: Py<PyAny>) {
        self.yaxis_major_locator = Some(locator);
    }

    fn set_yaxis_minor_locator(&mut self, locator: Py<PyAny>) {
        self.yaxis_minor_locator = Some(locator);
    }

    fn set_facecolor(&mut self, color: &str) {
        self.facecolor = color.to_string();
    }

    #[getter]
    fn get_xaxis(&self, py: Python) -> PyResult<Py<Axis>> {
        let mut axis = Axis::new();
        axis.which = "x".to_string();
        axis.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Ok(Py::new(py, axis)?)
    }

    #[getter]
    fn get_yaxis(&self, py: Python) -> PyResult<Py<Axis>> {
        let mut axis = Axis::new();
        axis.which = "y".to_string();
        axis.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Ok(Py::new(py, axis)?)
    }

    #[getter]
    fn get_patch(&self, py: Python) -> PyResult<Py<Patch>> {
        let mut patch = Patch::new();
        patch.facecolor = self.facecolor.clone();
        patch.parent = self.self_py.as_ref().map(|p| p.clone_ref(py));
        Ok(Py::new(py, patch)?)
    }

    #[getter]
    fn get_spines(&self, py: Python) -> PyResult<Py<SpineDict>> {
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

// ==================== Axes Rendering ====================

impl Axes {
    fn compute_bounds(&self) -> ((f64, f64), (f64, f64)) {
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

    fn render<DB: DrawingBackend>(
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

        // Compute tick values from locators if available
        let computed_xticks: Option<Vec<f64>> = self.xaxis_major_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (x_min, x_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| self.xticks_val.clone());

        let computed_yticks: Option<Vec<f64>> = self.yaxis_major_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (y_min, y_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| self.yticks_val.clone());

        let computed_xminor: Option<Vec<f64>> = self.xaxis_minor_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (x_min, x_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| {
            if self.minor_grid_visible {
                computed_xticks.as_ref().and_then(|major_ticks| {
                    if major_ticks.len() < 2 { return None; }
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
                    if minor.is_empty() { None } else { Some(minor) }
                })
            } else {
                None
            }
        });

        let computed_yminor: Option<Vec<f64>> = self.yaxis_minor_locator.as_ref().map(|locator| {
            locator.bind(py).call_method1("tick_values", (y_min, y_max))
                .ok().and_then(|r| r.extract::<Vec<f64>>().ok())
        }).flatten().or_else(|| {
            if self.minor_grid_visible {
                computed_yticks.as_ref().and_then(|major_ticks| {
                    if major_ticks.len() < 2 { return None; }
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
                    if minor.is_empty() { None } else { Some(minor) }
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
            RgbColor(200, 200, 200)
        };
        let major_lw_f64 = self.grid_linewidth.unwrap_or(0.8);
        let major_lw_u32 = (major_lw_f64.max(0.1)).ceil() as u32;

        let minor_color = if let Some(ref c) = self.minor_grid_color {
            parse_color(c, 0).unwrap_or(RgbColor(230, 230, 230))
        } else {
            RgbColor(230, 230, 230)
        };
        let minor_lw_f64 = self.minor_grid_linewidth.unwrap_or(0.4);
        let minor_lw_u32 = (minor_lw_f64.max(0.1)).ceil() as u32;

        let grid_ls = self.grid_linestyle.as_deref().unwrap_or("-");
        let minor_ls = self.minor_grid_linestyle.as_deref().unwrap_or("--");

        // Configure mesh - disable axis elements that we'll draw manually
        let label_size: i32 = self.tick_labelsize as i32;
        mesh_builder
            .x_labels(x_label_count.max(2))
            .y_labels(y_label_count.max(2))
            .x_label_style(("sans-serif", label_size))
            .y_label_style(("sans-serif", label_size))
            .x_desc(self.xlabel.clone())
            .y_desc(self.ylabel.clone())
            .bold_line_style(frame_style)
            .light_line_style(frame_style);

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

        // Draw grid lines manually for proper dashed style support
        let draw_grid_line = |chart: &mut ChartContext<DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>,
                              x1: f64, y1: f64, x2: f64, y2: f64,
                              color: RgbColor, lw: f64, ls: &str| -> PyResult<()> {
            let rgb = to_plotters_color(color);
            let lw_u32 = (lw.max(0.1)).ceil() as u32;
            let total_dx = x2 - x1;
            let total_dy = y2 - y1;
            let total_len = (total_dx * total_dx + total_dy * total_dy).sqrt();
            if total_len < 0.5 {
                return Ok(());
            }
            let (seg_len, gap_len) = match ls {
                "--" => (6.0, 3.0),
                ":" => (2.0, 4.0),
                "-." => (6.0, 2.0),
                _ => (total_len, 0.0),
            };
            if gap_len <= 0.0 || seg_len >= total_len {
                chart.draw_series(std::iter::once(PathElement::new(
                    vec![(x1, y1), (x2, y2)],
                    rgb.stroke_width(lw_u32),
                ))).map_err(|e| PyRuntimeError::new_err(format!("Grid line: {}", e)))?;
            } else {
                let nx = total_dx / total_len;
                let ny = total_dy / total_len;
                let cycle_len = seg_len + gap_len;
                let num_cycles = (total_len / cycle_len) as usize;
                let mut all_points = Vec::with_capacity(num_cycles * 2 + 2);
                for i in 0..num_cycles {
                    let s0 = i as f64 * cycle_len;
                    let s1 = (s0 + seg_len).min(total_len);
                    let px1 = x1 + nx * s0;
                    let py1 = y1 + ny * s0;
                    let px2 = x1 + nx * s1;
                    let py2 = y1 + ny * s1;
                    all_points.push((px1, py1));
                    all_points.push((px2, py2));
                }
                let remaining = total_len - num_cycles as f64 * cycle_len;
                if remaining > seg_len * 0.5 {
                    let s0 = num_cycles as f64 * cycle_len;
                    let s1 = (s0 + seg_len).min(total_len);
                    let px1 = x1 + nx * s0;
                    let py1 = y1 + ny * s0;
                    let px2 = x1 + nx * s1;
                    let py2 = y1 + ny * s1;
                    all_points.push((px1, py1));
                    all_points.push((px2, py2));
                }
                if !all_points.is_empty() {
                    chart.draw_series(std::iter::once(PathElement::new(all_points, rgb.stroke_width(lw_u32))))
                        .map_err(|e| PyRuntimeError::new_err(format!("Grid dashed: {}", e)))?;
                }
            }
            Ok(())
        };

        // Draw vertical grid lines at computed x tick positions
        if self.grid_visible {
            if self.grid_axis == "both" || self.grid_axis == "x" {
                if let Some(ref ticks) = computed_xticks {
                    for &tx in ticks {
                        if tx >= x_min && tx <= x_max {
                            draw_grid_line(chart, tx, y_min, tx, y_max, major_color, major_lw_f64, grid_ls)?;
                        }
                    }
                }
            }
            if self.grid_axis == "both" || self.grid_axis == "y" {
                if let Some(ref ticks) = computed_yticks {
                    for &ty in ticks {
                        if ty >= y_min && ty <= y_max {
                            draw_grid_line(chart, x_min, ty, x_max, ty, major_color, major_lw_f64, grid_ls)?;
                        }
                    }
                }
            }
        }

        // Draw minor grid lines
        if self.minor_grid_visible {
            if let Some(ref ticks) = computed_xminor {
                for &tx in ticks {
                    if tx > x_min && tx < x_max {
                        draw_grid_line(chart, tx, y_min, tx, y_max, minor_color, minor_lw_f64, minor_ls)?;
                    }
                }
            }
            if let Some(ref ticks) = computed_yminor {
                for &ty in ticks {
                    if ty > y_min && ty < y_max {
                        draw_grid_line(chart, x_min, ty, x_max, ty, minor_color, minor_lw_f64, minor_ls)?;
                    }
                }
            }
        }

        // If no computed ticks, use existing grid styling via standard mesh behavior
        // This is handled by the mesh builder already

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
                            if linestyle == "-" {
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
                            } else {
                                for i in 0..points.len() - 1 {
                                    let (x1, y1) = points[i];
                                    let (x2, y2) = points[i + 1];
                                    draw_grid_line(chart, x1, y1, x2, y2, col, *linewidth, linestyle)?;
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
                    if linestyle == "-" || linestyle.is_empty() {
                        let style = shape_style(col, *linewidth, linestyle);
                        chart.draw_series(LineSeries::new(points, style))
                            .map_err(|e| PyRuntimeError::new_err(format!("Step draw: {}", e)))?;
                    } else {
                        for i in 0..points.len() - 1 {
                            let (x1, y1) = points[i];
                            let (x2, y2) = points[i + 1];
                            draw_grid_line(chart, x1, y1, x2, y2, col, *linewidth, linestyle)?;
                        }
                    }
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

    fn parse_hist_data(x: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<f64>>> {
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

    fn parse_color_list(color: &Bound<'_, PyAny>, expected_len: usize) -> PyResult<Vec<String>> {
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

// ==================== Viridis Colormap ====================

fn viridis_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, u8, u8, u8); 9] = [
        (0.0, 68, 1, 84),
        (0.125, 71, 44, 122),
        (0.25, 59, 82, 139),
        (0.375, 43, 120, 142),
        (0.5, 33, 145, 140),
        (0.625, 53, 178, 112),
        (0.75, 94, 201, 98),
        (0.875, 172, 229, 62),
        (1.0, 253, 231, 37),
    ];
    if t <= 0.0 { return RGBColor(stops[0].1, stops[0].2, stops[0].3); }
    if t >= 1.0 { return RGBColor(stops[8].1, stops[8].2, stops[8].3); }
    for i in 0..stops.len() - 1 {
        let (t1, r1, g1, b1) = stops[i];
        let (t2, r2, g2, b2) = stops[i + 1];
        if t >= t1 && t <= t2 {
            let frac = if (t2 - t1).abs() < 1e-10 { 0.0 } else { (t - t1) / (t2 - t1) };
            return RGBColor(
                (r1 as f64 + (r2 as f64 - r1 as f64) * frac) as u8,
                (g1 as f64 + (g2 as f64 - g1 as f64) * frac) as u8,
                (b1 as f64 + (b2 as f64 - b1 as f64) * frac) as u8,
            );
        }
    }
    RGBColor(stops[8].1, stops[8].2, stops[8].3)
}

fn plasma_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, u8, u8, u8); 9] = [
        (0.0, 13, 8, 135),
        (0.125, 75, 3, 161),
        (0.25, 125, 3, 168),
        (0.375, 168, 34, 157),
        (0.5, 203, 70, 121),
        (0.625, 229, 107, 83),
        (0.75, 248, 150, 45),
        (0.875, 249, 198, 27),
        (1.0, 240, 249, 33),
    ];
    if t <= 0.0 { return RGBColor(stops[0].1, stops[0].2, stops[0].3); }
    if t >= 1.0 { return RGBColor(stops[8].1, stops[8].2, stops[8].3); }
    for i in 0..stops.len() - 1 {
        let (t1, r1, g1, b1) = stops[i];
        let (t2, r2, g2, b2) = stops[i + 1];
        if t >= t1 && t <= t2 {
            let frac = if (t2 - t1).abs() < 1e-10 { 0.0 } else { (t - t1) / (t2 - t1) };
            return RGBColor(
                (r1 as f64 + (r2 as f64 - r1 as f64) * frac) as u8,
                (g1 as f64 + (g2 as f64 - g1 as f64) * frac) as u8,
                (b1 as f64 + (b2 as f64 - b1 as f64) * frac) as u8,
            );
        }
    }
    RGBColor(stops[8].1, stops[8].2, stops[8].3)
}

fn inferno_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, u8, u8, u8); 9] = [
        (0.0, 0, 0, 4),
        (0.125, 31, 12, 72),
        (0.25, 85, 15, 143),
        (0.375, 136, 34, 171),
        (0.5, 180, 55, 155),
        (0.625, 217, 81, 113),
        (0.75, 243, 120, 62),
        (0.875, 249, 170, 23),
        (1.0, 252, 225, 10),
    ];
    if t <= 0.0 { return RGBColor(stops[0].1, stops[0].2, stops[0].3); }
    if t >= 1.0 { return RGBColor(stops[8].1, stops[8].2, stops[8].3); }
    for i in 0..stops.len() - 1 {
        let (t1, r1, g1, b1) = stops[i];
        let (t2, r2, g2, b2) = stops[i + 1];
        if t >= t1 && t <= t2 {
            let frac = if (t2 - t1).abs() < 1e-10 { 0.0 } else { (t - t1) / (t2 - t1) };
            return RGBColor(
                (r1 as f64 + (r2 as f64 - r1 as f64) * frac) as u8,
                (g1 as f64 + (g2 as f64 - g1 as f64) * frac) as u8,
                (b1 as f64 + (b2 as f64 - b1 as f64) * frac) as u8,
            );
        }
    }
    RGBColor(stops[8].1, stops[8].2, stops[8].3)
}

fn magma_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, u8, u8, u8); 9] = [
        (0.0, 0, 0, 4),
        (0.125, 28, 16, 68),
        (0.25, 79, 18, 123),
        (0.375, 129, 23, 144),
        (0.5, 172, 43, 138),
        (0.625, 209, 69, 111),
        (0.75, 237, 103, 71),
        (0.875, 248, 148, 33),
        (1.0, 252, 196, 7),
    ];
    if t <= 0.0 { return RGBColor(stops[0].1, stops[0].2, stops[0].3); }
    if t >= 1.0 { return RGBColor(stops[8].1, stops[8].2, stops[8].3); }
    for i in 0..stops.len() - 1 {
        let (t1, r1, g1, b1) = stops[i];
        let (t2, r2, g2, b2) = stops[i + 1];
        if t >= t1 && t <= t2 {
            let frac = if (t2 - t1).abs() < 1e-10 { 0.0 } else { (t - t1) / (t2 - t1) };
            return RGBColor(
                (r1 as f64 + (r2 as f64 - r1 as f64) * frac) as u8,
                (g1 as f64 + (g2 as f64 - g1 as f64) * frac) as u8,
                (b1 as f64 + (b2 as f64 - b1 as f64) * frac) as u8,
            );
        }
    }
    RGBColor(stops[8].1, stops[8].2, stops[8].3)
}

fn cool_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(
        (t * 255.0) as u8,
        ((1.0 - t) * 255.0) as u8,
        255,
    )
}

fn spring_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(
        255,
        (t * 255.0) as u8,
        ((1.0 - t) * 255.0) as u8,
    )
}

fn summer_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(
        (t * 255.0) as u8,
        (128.0 + t * 127.0) as u8,
        (64.0 * (1.0 - t)) as u8,
    )
}

fn autumn_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(
        255,
        (t * 255.0) as u8,
        0,
    )
}

fn winter_color(t: f64) -> RGBColor {
    let t = t.clamp(0.0, 1.0);
    RGBColor(
        0,
        (t * 255.0) as u8,
        (255.0 * (1.0 - t * 0.5)) as u8,
    )
}

fn median(data: &[f64]) -> f64 {
    let n = data.len();
    if n == 0 { return 0.0; }
    if n % 2 == 0 {
        (data[n / 2 - 1] + data[n / 2]) / 2.0
    } else {
        data[n / 2]
    }
}

// ==================== Helper Classes for Matplotlib Compatibility ====================

#[pyclass]
pub struct Axis {
    grid_visible: bool,
    grid_color: Option<String>,
    grid_linewidth: Option<f64>,
    grid_linestyle: Option<String>,
    minor_grid_color: Option<String>,
    minor_grid_linewidth: Option<f64>,
    minor_grid_linestyle: Option<String>,
    #[allow(dead_code)]
    major_locator: String,
    #[allow(dead_code)]
    minor_locator: String,
    parent: Option<Py<PyAny>>,
    which: String,
}

#[pymethods]
impl Axis {
    #[new]
    fn new() -> Self {
        Axis {
            grid_visible: false,
            grid_color: None,
            grid_linewidth: None,
            grid_linestyle: None,
            minor_grid_color: None,
            minor_grid_linewidth: None,
            minor_grid_linestyle: None,
            major_locator: "auto".to_string(),
            minor_locator: "auto".to_string(),
            parent: None,
            which: "x".to_string(),
        }
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (visible=None, which="major", ls=None, c=None, lw=None))]
    fn grid(&mut self, py: Python<'_>, visible: Option<bool>, which: &str, ls: Option<&str>, c: Option<&str>, lw: Option<f64>) {
        self.grid_visible = visible.unwrap_or(true);
        if "minor".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
            self.minor_grid_color = c.map(|s| s.to_string());
            self.minor_grid_linewidth = lw;
            self.minor_grid_linestyle = ls.map(|s| s.to_string());
        }
        if "major".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
            self.grid_color = c.map(|s| s.to_string());
            self.grid_linewidth = lw;
            self.grid_linestyle = ls.map(|s| s.to_string());
        }
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                if "minor".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
                    ax.minor_grid_visible = true;
                    ax.minor_grid_color = c.map(|s| s.to_string());
                    ax.minor_grid_linewidth = lw;
                    ax.minor_grid_linestyle = ls.map(|s| s.to_string());
                }
                if "major".eq_ignore_ascii_case(which) || "both".eq_ignore_ascii_case(which) {
                    ax.grid_color = c.map(|s| s.to_string());
                    ax.grid_linewidth = lw;
                    ax.grid_linestyle = ls.map(|s| s.to_string());
                    if visible.unwrap_or(true) {
                        ax.grid_visible = true;
                    }
                }
            }
        }
    }

    fn set_major_locator(&mut self, py: Python<'_>, locator: &Bound<'_, PyAny>) {
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                if self.which == "x" {
                    ax.xaxis_major_locator = Some(locator.clone().unbind());
                } else {
                    ax.yaxis_major_locator = Some(locator.clone().unbind());
                }
            }
        }
    }

    fn set_minor_locator(&mut self, py: Python<'_>, locator: &Bound<'_, PyAny>) {
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                if self.which == "x" {
                    ax.xaxis_minor_locator = Some(locator.clone().unbind());
                } else {
                    ax.yaxis_minor_locator = Some(locator.clone().unbind());
                }
            }
        }
    }
}

#[pyclass]
pub struct Patch {
    facecolor: String,
    edgecolor: String,
    parent: Option<Py<PyAny>>,
}

#[pymethods]
impl Patch {
    #[new]
    fn new() -> Self {
        Patch {
            facecolor: "white".to_string(),
            edgecolor: "black".to_string(),
            parent: None,
        }
    }

    fn set_facecolor(&mut self, color: &str, py: Python<'_>) {
        self.facecolor = color.to_string();
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                ax.facecolor = color.to_string();
            }
        }
    }

    fn get_facecolor(&self) -> &str {
        &self.facecolor
    }

    fn set_edgecolor(&mut self, color: &str) {
        self.edgecolor = color.to_string();
    }
}

#[pyclass]
pub struct SpineDict {
    spines: Vec<Spine>,
    parent: Option<Py<PyAny>>,
}

#[pymethods]
impl SpineDict {
    #[new]
    fn new() -> Self {
        let names = vec!["top", "bottom", "left", "right"];
        SpineDict {
            spines: names.iter().map(|n| Spine { name: n.to_string(), visible: true, parent: None }).collect(),
            parent: None,
        }
    }

    fn __getitem__(&mut self, py: Python<'_>, key: &str) -> Option<Spine> {
        self.spines.iter().position(|s| s.name == key).map(|i| {
            let mut spine = self.spines[i].clone();
            spine.parent = self.parent.as_ref().map(|p| p.clone_ref(py));
            spine
        })
    }

    fn items(&self, _py: Python<'_>) -> Vec<(String, Spine)> {
        self.spines.iter().map(|s| (s.name.clone(), s.clone())).collect()
    }
}

#[pyclass(skip_from_py_object)]
pub struct Spine {
    name: String,
    visible: bool,
    parent: Option<Py<PyAny>>,
}

impl Clone for Spine {
    fn clone(&self) -> Self {
        Spine {
            name: self.name.clone(),
            visible: self.visible,
            parent: None,
        }
    }
}

#[pymethods]
impl Spine {
    fn set_visible(&mut self, visible: bool, py: Python<'_>) {
        self.visible = visible;
        if let Some(parent) = &self.parent {
            if let Ok(ax_bound) = parent.bind(py).cast::<Axes>() {
                let mut ax = ax_bound.borrow_mut();
                match self.name.as_str() {
                    "top" => ax.spine_top = visible,
                    "bottom" => ax.spine_bottom = visible,
                    "left" => ax.spine_left = visible,
                    "right" => ax.spine_right = visible,
                    _ => {}
                }
            }
        }
    }

    fn get_visible(&self) -> bool {
        self.visible
    }

    fn get_color(&self) -> String {
        "black".to_string()
    }

    fn set_color(&mut self, _color: &str) {
    }

    fn set_linewidth(&mut self, _lw: f64) {
    }
}

// ==================== Figure Class ====================

static CURRENT_FIGURE: Mutex<Option<Py<Figure>>> = Mutex::new(None);

fn set_current_figure(fig: Py<Figure>) {
    if let Ok(mut current) = CURRENT_FIGURE.lock() {
        *current = Some(fig);
    }
}

fn get_current_figure(py: Python<'_>) -> PyResult<Bound<'_, Figure>> {
    let guard = CURRENT_FIGURE
        .lock()
        .map_err(|_| PyRuntimeError::new_err("Mutex poisoned"))?;
    match guard.as_ref() {
        Some(fig) => Ok(fig.bind(py).clone()),
        None => Err(PyRuntimeError::new_err(
            "No current figure. Create one with figure() or subplots() first.",
        )),
    }
}

#[pyclass]
pub struct Figure {
    axes_list: Vec<Py<Axes>>,
    nrows: usize,
    ncols: usize,
    suptitle: String,
    width: u32,
    height: u32,
    dpi: f64,
    axes_positions: Vec<(f64, f64, f64, f64)>,
    facecolor: String,
    subplot_left: f64,
    subplot_right: f64,
    subplot_bottom: f64,
    subplot_top: f64,
}

#[pymethods]
impl Figure {
    #[new]
    fn new() -> Self {
        Figure {
            axes_list: Vec::new(),
            nrows: 1,
            ncols: 1,
            suptitle: String::new(),
            width: 800,
            height: 600,
            dpi: 100.0,
            axes_positions: Vec::new(),
            facecolor: "white".to_string(),
            subplot_left: 0.125,
            subplot_right: 0.9,
            subplot_bottom: 0.1,
            subplot_top: 0.9,
        }
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn set_dpi(&mut self, dpi: f64) {
        self.dpi = dpi;
    }

    fn suptitle(&mut self, text: String) {
        self.suptitle = text;
    }

    #[pyo3(signature = (left=None, right=None, bottom=None, top=None, _wspace=None, _hspace=None))]
    fn subplots_adjust(&mut self, left: Option<f64>, right: Option<f64>, bottom: Option<f64>, top: Option<f64>, _wspace: Option<f64>, _hspace: Option<f64>) {
        if let Some(v) = left { self.subplot_left = v; }
        if let Some(v) = right { self.subplot_right = v; }
        if let Some(v) = bottom { self.subplot_bottom = v; }
        if let Some(v) = top { self.subplot_top = v; }
    }

    fn tight_layout(&mut self) {
    }

    fn set_facecolor(&mut self, color: &str) {
        self.facecolor = color.to_string();
    }

    fn clear(&mut self) {
        self.axes_list.clear();
        self.axes_positions.clear();
    }

    fn clf(&mut self) {
        self.axes_list.clear();
        self.axes_positions.clear();
    }

    fn add_subplot(&mut self, py: Python, spec: &Bound<'_, PyAny>) -> PyResult<Py<Axes>> {
        let (left, right, bottom, top) = if let Ok(_) = spec.getattr("rowStart") {
            let num_rows: f64 = spec.getattr("numRows")?.extract::<i32>().map(|v| v as f64).unwrap_or(100.0);
            let num_cols: f64 = spec.getattr("numCols")?.extract::<i32>().map(|v| v as f64).unwrap_or(100.0);
            let row_start: f64 = spec.getattr("rowStart")?.extract::<i32>().map(|v| v as f64).unwrap_or(0.0);
            let row_stop: f64 = spec.getattr("rowStop")?.extract::<i32>().map(|v| v as f64).unwrap_or(num_rows);
            let col_start: f64 = spec.getattr("colStart")?.extract::<i32>().map(|v| v as f64).unwrap_or(0.0);
            let col_stop: f64 = spec.getattr("colStop")?.extract::<i32>().map(|v| v as f64).unwrap_or(num_cols);
            let left = col_start / num_cols;
            let right = col_stop / num_cols;
            let bottom = 1.0 - row_stop / num_rows;
            let top = 1.0 - row_start / num_rows;
            (left, right, bottom, top)
        } else {
            (0.0, 1.0, 0.0, 1.0)
        };
        let ax = Axes::new();
        let ax_py = Py::new(py, ax)?;
        init_axes_self_py(&ax_py, py);
        self.axes_list.push(ax_py.clone_ref(py));
        self.axes_positions.push((left, right, bottom, top));
        Ok(ax_py)
    }

    fn savefig(&self, py: Python, filename: &str) -> PyResult<()> {
        if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            let backend = BitMapBackend::new(filename, (self.width, self.height));
            self.render_to_backend(py, backend, self.width, self.height)
        } else {
            let backend = SVGBackend::new(filename, (self.width, self.height));
            self.render_to_backend(py, backend, self.width, self.height)?;
            let w_in = self.width as f64 / self.dpi;
            let h_in = self.height as f64 / self.dpi;
            if let Ok(content) = std::fs::read_to_string(filename) {
                let content = content
                    .replacen(&format!("width=\"{}\"", self.width), &format!("width=\"{:.4}in\"", w_in), 1)
                    .replacen(&format!("height=\"{}\"", self.height), &format!("height=\"{:.4}in\"", h_in), 1);
                let _ = std::fs::write(filename, content);
            }
            Ok(())
        }
    }

    fn show(&self, py: Python) -> PyResult<()> {
        let tmpdir = std::env::temp_dir();
        let path = tmpdir.join("rsplot_output.png");
        let filename = path.to_str().unwrap_or("/tmp/rsplot_output.png").to_string();
        let backend = BitMapBackend::new(&filename, (self.width, self.height));
        self.render_to_backend(py, backend, self.width, self.height)?;

        if cfg!(target_os = "macos") {
            let _ = std::process::Command::new("open").arg(&filename).spawn();
        } else if cfg!(target_os = "linux") {
            let _ = std::process::Command::new("xdg-open").arg(&filename).spawn();
        }

        println!("Figure saved to: {}", filename);
        Ok(())
    }
}

impl Figure {
    fn render_to_backend<B: DrawingBackend>(&self, py: Python, backend: B, actual_w: u32, actual_h: u32) -> PyResult<()>
    where
        B::ErrorType: 'static,
    {
        let root = backend.into_drawing_area();

        let fig_bg = parse_color(&self.facecolor, 0).unwrap_or(RgbColor(255, 255, 255));
        root.fill(&to_plotters_color(fig_bg))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to fill background: {}", e)))?;

        if self.axes_list.is_empty() {
            root.present()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to present: {}", e)))?;
            return Ok(());
        }

        let _nrows = self.nrows;
        let _ncols = self.ncols;

        if !self.suptitle.is_empty() {
            let _ = root.titled(&self.suptitle, ("sans-serif", 24));
        }

        let total_w = actual_w as f64;
        let total_h = actual_h as f64;

        for (i, ax_py) in self.axes_list.iter().enumerate() {
            let ax = ax_py.borrow(py);

            let ((x_min, x_max), (y_min, y_max)) = ax.compute_bounds();

            let (left, right, bottom, top) = if i < self.axes_positions.len() {
                self.axes_positions[i]
            } else {
                (0.0, 1.0, 0.0, 1.0)
            };

            let margin_l = self.subplot_left;
            let margin_r = 1.0 - self.subplot_right;
            let margin_b = self.subplot_bottom;
            let margin_t = 1.0 - self.subplot_top;

            let usable_w = 1.0 - margin_l - margin_r;
            let usable_h = 1.0 - margin_b - margin_t;

            let plot_left = left * usable_w + margin_l;
            let plot_right = right * usable_w + margin_l;
            let plot_bottom_frac = bottom * usable_h + margin_b;
            let plot_top_frac = top * usable_h + margin_b;

            let x0 = (plot_left * total_w) as i32;
            let y0 = ((1.0 - plot_top_frac) * total_h) as i32;
            let sub_w = ((plot_right - plot_left) * total_w) as u32;
            let sub_h = ((plot_top_frac - plot_bottom_frac) * total_h) as u32;

            if sub_w <= 0 || sub_h <= 0 {
                drop(ax);
                continue;
            }

            let chart_area = root.clone().shrink(
                (x0, y0),
                (sub_w, sub_h),
            );

            let margin_top = if ax.title.is_empty() { 5 } else { 25 };
            let margin_right = 10;
            let margin_bottom = if ax.xlabel.is_empty() { 5 } else { 25 };
            let margin_left = if ax.ylabel.is_empty() { 5 } else { 40 };

            let mut chart = ChartBuilder::on(&chart_area)
                .margin_top(margin_top)
                .margin_right(margin_right)
                .margin_bottom(margin_bottom)
                .margin_left(margin_left)
                .caption(ax.title.clone(), ("sans-serif", 18))
                .build_cartesian_2d(x_min..x_max, y_min..y_max)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to build chart: {}", e)))?;

            ax.render(py, &mut chart, (x_min, x_max), (y_min, y_max))?;

            let twin_axes = ax.twin_axes.clone();
            drop(ax);
            for twin in &twin_axes {
                let ((tx_min, tx_max), (ty_min, ty_max)) = twin.compute_bounds();
                let (ux_min, ux_max) = if twin.is_twin_x { (tx_min, tx_max) } else { (x_min, x_max) };
                let (uy_min, uy_max) = if twin.is_twin_y { (ty_min, ty_max) } else { (y_min, y_max) };
                let mut twin_chart = ChartBuilder::on(&chart_area)
                    .margin_top(if twin.title.is_empty() { 5 } else { 25 })
                    .margin_right(10)
                    .margin_bottom(if twin.xlabel.is_empty() { 5 } else { 25 })
                    .margin_left(if twin.ylabel.is_empty() { 5 } else { 40 })
                    .caption(twin.title.clone(), ("sans-serif", 18))
                    .build_cartesian_2d(ux_min..ux_max, uy_min..uy_max)
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to build twin chart: {}", e)))?;
                twin.render(py, &mut twin_chart, (ux_min, ux_max), (uy_min, uy_max))?;
            }
        }

        root.present()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to present: {}", e)))?;
        Ok(())
    }
}

// ==================== Module-level Functions ====================

fn get_current_axes(py: Python<'_>) -> PyResult<Py<Axes>> {
    let fig = get_current_figure(py)?;
    let fig_ref = fig.borrow();
    if fig_ref.axes_list.is_empty() {
        return Err(PyRuntimeError::new_err("No axes found in current figure."));
    }
    Ok(fig_ref.axes_list[0].clone_ref(py))
}

fn init_axes_self_py(ax_py: &Py<Axes>, py: Python<'_>) {
    let obj: Py<PyAny> = ax_py.clone_ref(py).into();
    let mut ax_ref = ax_py.borrow_mut(py);
    ax_ref.self_py = Some(obj);
}

fn _make_fig_ax(py: Python<'_>, ax: Axes) -> PyResult<(Py<Figure>, Py<Axes>)> {
    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));
    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    fig_py.borrow_mut(py).axes_list.push(ax_py.clone_ref(py));
    fig_py.borrow_mut(py).axes_positions.push((0.0, 1.0, 0.0, 1.0));
    Ok((fig_py, ax_py))
}

#[pyfunction]
fn xlabel(py: Python, text: String) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_xlabel(text);
    Ok(())
}

#[pyfunction]
fn ylabel(py: Python, text: String) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_ylabel(text);
    Ok(())
}

#[pyfunction]
fn title(py: Python, text: String) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_title(text);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (visible=None, c=None, ls=None, lw=None, axis=None))]
fn grid(py: Python, visible: Option<bool>, c: Option<String>, ls: Option<String>, lw: Option<f64>, axis: Option<String>) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).grid(visible, c, lw, ls, axis);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (loc="best"))]
fn legend(py: Python, loc: &str) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).legend(loc);
    Ok(())
}

#[pyfunction]
fn xlim(py: Python, left: f64, right: f64) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_xlim(left, right);
    Ok(())
}

#[pyfunction]
fn ylim(py: Python, bottom: f64, top: f64) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).set_ylim(bottom, top);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x, y, s=20.0, c=None, marker="o", label=None, alpha=1.0))]
fn scatter<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    s: f64,
    c: Option<String>,
    marker: &'a str,
    label: Option<String>,
    alpha: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.scatter(x, y, s, c, marker, label, alpha);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, height, width=0.8, color=None, label=None))]
fn bar(
    py: Python<'_>,
    x: Vec<f64>,
    height: Vec<f64>,
    width: f64,
    color: Option<String>,
    label: Option<String>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.bar(x, height, width, color, label);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, bins=10, density=false, label=None, alpha=0.7, color=None, facecolor=None))]
fn hist<'py>(
    py: Python<'py>,
    x: Bound<'py, PyAny>,
    bins: usize,
    density: bool,
    label: Option<String>,
    alpha: f64,
    color: Option<Bound<'py, PyAny>>,
    facecolor: Option<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyTuple>> {
    let mut ax = Axes::new();
    let bins_any = pyo3::types::PyInt::new(py, bins as i64).as_any().clone();
    ax.hist(py, x, Some(bins_any), density, label, alpha, color, facecolor, None, None)?;
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y1, y2=0.0, color=None, alpha=0.3, label=None))]
fn fill_between(
    py: Python<'_>,
    x: Vec<f64>,
    y1: Vec<f64>,
    y2: f64,
    color: Option<String>,
    alpha: f64,
    label: Option<String>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.fill_between(x, y1, y2, color, alpha, label);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, yerr=None, xerr=None, fmt="o", color=None, label=None, capsize=3.0))]
fn errorbar<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    yerr: Option<f64>,
    xerr: Option<f64>,
    fmt: &'a str,
    color: Option<String>,
    label: Option<String>,
    capsize: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.errorbar(x, y, yerr, xerr, fmt, color, label, capsize);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, linefmt="-", markerfmt="o", label=None))]
fn stem<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    linefmt: &'a str,
    markerfmt: &'a str,
    label: Option<String>,
) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.stem(x, y, linefmt, markerfmt, label);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, where_="pre", label=None, color=None, linestyle="-", linewidth=1.5))]
fn step<'a>(
    py: Python<'a>,
    x: Vec<f64>,
    y: Vec<f64>,
    where_: &'a str,
    label: Option<String>,
    color: Option<String>,
    linestyle: &'a str,
    linewidth: f64,
) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.step(x, y, where_, label, color, linestyle, linewidth);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, cmap="viridis", aspect="auto"))]
fn imshow<'a>(py: Python<'a>, x: Vec<Vec<f64>>, cmap: &'a str, aspect: &'a str) -> PyResult<Bound<'a, PyTuple>> {
    let mut ax = Axes::new();
    ax.imshow(x, cmap, aspect);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, labels=None, colors=None, autopct=None, startangle=0.0))]
fn pie(
    py: Python<'_>,
    x: Vec<f64>,
    labels: Option<Vec<String>>,
    colors: Option<Vec<String>>,
    autopct: Option<String>,
    startangle: f64,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.pie(x, labels, colors, autopct, startangle);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, labels=None, vert=true))]
fn boxplot(
    py: Python<'_>,
    x: Vec<Vec<f64>>,
    labels: Option<Vec<String>>,
    vert: bool,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.boxplot(x, labels, vert);
    let (fig_py, ax_py) = _make_fig_ax(py, ax)?;
    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, text, fontsize=None, color=None, c=None, family=None))]
fn text(
    py: Python,
    x: f64,
    y: f64,
    text: Bound<'_, PyAny>,
    fontsize: Option<i32>,
    color: Option<String>,
    c: Option<String>,
    family: Option<String>,
) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    Axes::text(&mut ax_ref, py, x, y, text, fontsize, color, c, family);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (y=None, color=None, linestyle=None, linewidth=None))]
fn axhline(
    py: Python,
    y: Option<f64>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).axhline(y, color, linestyle, linewidth);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (x=None, color=None, linestyle=None, linewidth=None))]
fn axvline(
    py: Python,
    x: Option<f64>,
    color: Option<String>,
    linestyle: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).axvline(x, color, linestyle, linewidth);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (ticks=None, labels=None))]
fn xticks(py: Python, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).xticks(ticks, labels);
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (ticks=None, labels=None))]
fn yticks(py: Python, ticks: Option<Vec<f64>>, labels: Option<Vec<String>>) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).yticks(ticks, labels);
    Ok(())
}

#[pyfunction]
fn cla(py: Python) -> PyResult<()> {
    get_current_axes(py)?.borrow_mut(py).cla();
    Ok(())
}

#[pyfunction]
fn close(_py: Python) -> PyResult<()> {
    if let Ok(mut current) = CURRENT_FIGURE.lock() {
        *current = None;
    }
    Ok(())
}

#[pyfunction]
fn twinx(py: Python) -> PyResult<Py<Axes>> {
    let ax = get_current_axes(py)?;
    let twin = ax.borrow_mut(py).twinx();
    let twin_py = Py::new(py, twin)?;
    init_axes_self_py(&twin_py, py);
    Ok(twin_py)
}

#[pyfunction]
fn twiny(py: Python) -> PyResult<Py<Axes>> {
    let ax = get_current_axes(py)?;
    let twin = ax.borrow_mut(py).twiny();
    let twin_py = Py::new(py, twin)?;
    init_axes_self_py(&twin_py, py);
    Ok(twin_py)
}

#[pyfunction]
fn tight_layout(py: Python) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method0("tight_layout")?;
    Ok(())
}

#[pyfunction]
fn set_size(py: Python, width: u32, height: u32) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method1("set_size", (width, height))?;
    Ok(())
}

#[pyfunction]
fn set_dpi(py: Python, dpi: f64) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method1("set_dpi", (dpi,))?;
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (nrows=1, ncols=1, index=1))]
fn subplot(py: Python<'_>, nrows: usize, ncols: usize, index: usize) -> PyResult<Bound<'_, PyTuple>> {
    if index == 0 || index > nrows * ncols {
        return Err(PyValueError::new_err("Index out of range"));
    }
    let result = subplots(py, nrows, ncols)?;
    let fig = result.get_item(0)?;
    let axes_all = result.get_item(1)?;
    let ax = if nrows * ncols == 1 {
        axes_all.clone()
    } else {
        let lst = axes_all.cast::<PyList>()?;
        lst.get_item(index - 1)?
    };
    PyTuple::new(py, [fig, ax])
}

#[pyfunction]
#[pyo3(signature = (nrows=1, ncols=1))]
fn subplots(
    py: Python<'_>,
    nrows: usize,
    ncols: usize,
) -> PyResult<Bound<'_, PyTuple>> {
    let total = nrows * ncols;

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows,
        ncols,
        suptitle: String::new(),
        width: (ncols as u32 * 400).max(600),
        height: (nrows as u32 * 300).max(400),
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    if total == 1 {
        let ax = Axes::new();
        let ax_py = Py::new(py, ax)?;
        init_axes_self_py(&ax_py, py);
        {
            let mut fig_ref = fig_py.borrow_mut(py);
            fig_ref.axes_list.push(ax_py.clone_ref(py));
        }
        let fig_obj = fig_py.bind(py).as_any().clone();
        let ax_obj = ax_py.bind(py).as_any().clone();
        PyTuple::new(py, [fig_obj, ax_obj])
    } else {
        let mut py_axes: Vec<Bound<'_, PyAny>> = Vec::new();
        {
            let mut fig_ref = fig_py.borrow_mut(py);
            for _ in 0..total {
                let ax = Axes::new();
                let ax_py = Py::new(py, ax)?;
                init_axes_self_py(&ax_py, py);
                fig_ref.axes_list.push(ax_py.clone_ref(py));
                py_axes.push(ax_py.bind(py).as_any().clone());
            }
        }
        let fig_obj = fig_py.bind(py).as_any().clone();
        let axes_list = PyList::new(py, py_axes)?;
        PyTuple::new(py, [fig_obj, axes_list.as_any().clone()])
    }
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
fn plot(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, &linestyle.unwrap_or_else(|| "-".to_string()), marker, linewidth.unwrap_or(1.5), None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
fn savefig(py: Python, filename: &str) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method1("savefig", (filename,))?;
    Ok(())
}

#[pyfunction]
fn show(py: Python) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    fig.call_method0("show")?;
    Ok(())
}

#[pyfunction]
fn figure(py: Python) -> PyResult<Py<Figure>> {
    let fig = Figure::new();
    let fig_py = Py::new(py, fig)?;
    set_current_figure(fig_py.clone_ref(py));
    Ok(fig_py)
}

#[pyfunction]
fn gca(py: Python) -> PyResult<Py<Axes>> {
    let fig = get_current_figure(py)?;
    let fig_ref = fig.borrow();
    if fig_ref.axes_list.is_empty() {
        return Err(PyRuntimeError::new_err("No axes found. Create a figure first."));
    }
    Ok(fig_ref.axes_list[0].clone_ref(py))
}

#[pyfunction]
fn clf(py: Python) -> PyResult<()> {
    let fig = get_current_figure(py)?;
    let mut fig_ref = fig.borrow_mut();
    fig_ref.axes_list.clear();
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (y, width, height=0.8, color=None, label=None))]
fn barh(py: Python<'_>, y: Vec<f64>, width: Vec<f64>, height: f64, color: Option<String>, label: Option<String>) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.barh(y, width, height, color, label);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
fn semilogx(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.set_xscale("log");
    let ls = linestyle.as_deref().unwrap_or("-");
    let lw = linewidth.unwrap_or(1.5);
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, ls, marker, lw, None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
fn semilogy(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.set_yscale("log");
    let ls = linestyle.as_deref().unwrap_or("-");
    let lw = linewidth.unwrap_or(1.5);
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, ls, marker, lw, None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));

    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
#[pyo3(signature = (x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None))]
#[allow(clippy::too_many_arguments)]
fn loglog(
    py: Python<'_>,
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: Option<String>,
    linestyle: Option<String>,
    marker: Option<String>,
    linewidth: Option<f64>,
) -> PyResult<Bound<'_, PyTuple>> {
    let mut ax = Axes::new();
    ax.set_xscale("log");
    ax.set_yscale("log");
    let ls = linestyle.as_deref().unwrap_or("-");
    let lw = linewidth.unwrap_or(1.5);
    ax.plot(x.into_iter().map(Some).collect(), y.into_iter().map(Some).collect(), label, color, ls, marker, lw, None, None, None, None, None, None);

    let fig_py = Py::new(py, Figure {
        axes_list: Vec::new(),
        nrows: 1,
        ncols: 1,
        suptitle: String::new(),
        width: 800,
        height: 600,
        dpi: 100.0,
        axes_positions: Vec::new(),
        facecolor: "white".to_string(),
        subplot_left: 0.125,
        subplot_right: 0.9,
        subplot_bottom: 0.1,
        subplot_top: 0.9,
    })?;
    set_current_figure(fig_py.clone_ref(py));
    let ax_py = Py::new(py, ax)?;
    init_axes_self_py(&ax_py, py);
    {
        let mut fig_ref = fig_py.borrow_mut(py);
        fig_ref.axes_list.push(ax_py.clone_ref(py));
        fig_ref.axes_positions.push((0.0, 1.0, 0.0, 1.0));
    }

    let fig_obj = fig_py.bind(py).as_any().clone();
    let ax_obj = ax_py.bind(py).as_any().clone();
    PyTuple::new(py, [fig_obj, ax_obj])
}

#[pyfunction]
fn use_(_backend: String) {
    // matplotlib compatibility: backend selection is not applicable
}

#[pyfunction]
fn gcf(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    get_current_figure(py).map(|f| f.as_any().clone())
}

#[pyfunction]
fn xscale(py: Python<'_>, scale: &str) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    ax.borrow_mut(py).set_xscale(scale);
    Ok(())
}

#[pyfunction]
fn yscale(py: Python<'_>, scale: &str) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    ax.borrow_mut(py).set_yscale(scale);
    Ok(())
}

#[pyfunction]
fn margins(_py: Python<'_>, _x_margin: Option<f64>, _y_margin: Option<f64>) -> PyResult<()> {
    Ok(())
}

#[pyfunction]
fn box_(_py: Python<'_>, _on: Option<bool>) -> PyResult<()> {
    Ok(())
}

#[pyfunction]
fn minorticks_on(py: Python<'_>) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    ax_ref.minor_grid_visible = true;
    Ok(())
}

#[pyfunction]
fn minorticks_off(py: Python<'_>) -> PyResult<()> {
    let ax = get_current_axes(py)?;
    let mut ax_ref = ax.borrow_mut(py);
    ax_ref.minor_grid_visible = false;
    Ok(())
}

// ==================== Module Definition ====================

#[pymodule]
fn rsplot(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register system font for bitmap rendering
    #[cfg(target_os = "macos")]
    {
        if let Ok(font_data) = std::fs::read("/System/Library/Fonts/Supplemental/Andale Mono.ttf") {
            let font_ref: &'static [u8] = Box::leak(font_data.into_boxed_slice());
            let _ = register_font("sans-serif", plotters::style::FontStyle::Normal, font_ref);
        }
    }

    m.add_class::<Figure>()?;
    m.add_class::<Axes>()?;
    m.add_class::<Axis>()?;
    m.add_class::<Patch>()?;
    m.add_class::<SpineDict>()?;
    m.add_class::<Spine>()?;
    m.add_function(wrap_pyfunction!(subplots, m)?)?;
    m.add_function(wrap_pyfunction!(subplot, m)?)?;
    m.add_function(wrap_pyfunction!(plot, m)?)?;
    m.add_function(wrap_pyfunction!(savefig, m)?)?;
    m.add_function(wrap_pyfunction!(show, m)?)?;
    m.add_function(wrap_pyfunction!(figure, m)?)?;
    m.add_function(wrap_pyfunction!(semilogx, m)?)?;
    m.add_function(wrap_pyfunction!(semilogy, m)?)?;
    m.add_function(wrap_pyfunction!(loglog, m)?)?;
    m.add_function(wrap_pyfunction!(gca, m)?)?;
    m.add_function(wrap_pyfunction!(clf, m)?)?;
    m.add_function(wrap_pyfunction!(barh, m)?)?;
    m.add_function(wrap_pyfunction!(xlabel, m)?)?;
    m.add_function(wrap_pyfunction!(ylabel, m)?)?;
    m.add_function(wrap_pyfunction!(title, m)?)?;
    m.add_function(wrap_pyfunction!(grid, m)?)?;
    m.add_function(wrap_pyfunction!(legend, m)?)?;
    m.add_function(wrap_pyfunction!(xlim, m)?)?;
    m.add_function(wrap_pyfunction!(ylim, m)?)?;
    m.add_function(wrap_pyfunction!(scatter, m)?)?;
    m.add_function(wrap_pyfunction!(bar, m)?)?;
    m.add_function(wrap_pyfunction!(hist, m)?)?;
    m.add_function(wrap_pyfunction!(fill_between, m)?)?;
    m.add_function(wrap_pyfunction!(errorbar, m)?)?;
    m.add_function(wrap_pyfunction!(stem, m)?)?;
    m.add_function(wrap_pyfunction!(step, m)?)?;
    m.add_function(wrap_pyfunction!(imshow, m)?)?;
    m.add_function(wrap_pyfunction!(pie, m)?)?;
    m.add_function(wrap_pyfunction!(boxplot, m)?)?;
    m.add_function(wrap_pyfunction!(text, m)?)?;
    m.add_function(wrap_pyfunction!(axhline, m)?)?;
    m.add_function(wrap_pyfunction!(axvline, m)?)?;
    m.add_function(wrap_pyfunction!(xticks, m)?)?;
    m.add_function(wrap_pyfunction!(yticks, m)?)?;
    m.add_function(wrap_pyfunction!(cla, m)?)?;
    m.add_function(wrap_pyfunction!(close, m)?)?;
    m.add_function(wrap_pyfunction!(twinx, m)?)?;
    m.add_function(wrap_pyfunction!(twiny, m)?)?;
    m.add_function(wrap_pyfunction!(tight_layout, m)?)?;
    m.add_function(wrap_pyfunction!(set_size, m)?)?;
    m.add_function(wrap_pyfunction!(set_dpi, m)?)?;
    m.add_function(wrap_pyfunction!(use_, m)?)?;
    m.add_function(wrap_pyfunction!(gcf, m)?)?;
    m.add_function(wrap_pyfunction!(xscale, m)?)?;
    m.add_function(wrap_pyfunction!(yscale, m)?)?;
    m.add_function(wrap_pyfunction!(margins, m)?)?;
    m.add_function(wrap_pyfunction!(box_, m)?)?;
    m.add_function(wrap_pyfunction!(minorticks_on, m)?)?;
    m.add_function(wrap_pyfunction!(minorticks_off, m)?)?;
    m.setattr("__version__", "0.1.3")?;
    Ok(())
}