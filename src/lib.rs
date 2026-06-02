pub mod axes;
pub mod axis;
pub mod colormap;
pub mod colors;
pub mod elements;
pub mod figure;
pub mod marker;
pub mod pyfuncs;

use pyo3::prelude::*;
use plotters::style::register_font;

use crate::axis::{Axis, Patch, SpineDict, Spine};
use crate::axes::Axes;
use crate::figure::Figure;

#[pymodule]
fn rsplotlib(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
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