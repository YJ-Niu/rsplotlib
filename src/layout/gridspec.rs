//! GridSpec 和 SubplotSpec 的 Rust 实现
//!
//! 提供与 matplotlib 兼容的子图网格布局管理。
//! 所有类通过 PyO3 导出为 Python 类，Python 侧的 gridspec.py 为薄包装层。

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PySlice, PyTuple};

// ==================== GridSpec ====================

/// GridSpec 布局管理器
///
/// 用于在 Figure 中创建子图网格布局。
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct GridSpec {
    pub nrows: i32,
    pub ncols: i32,
    pub left: Option<f64>,
    pub bottom: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub wspace: Option<f64>,
    pub hspace: Option<f64>,
    pub width_ratios: Vec<f64>,
    pub height_ratios: Vec<f64>,
}

#[pymethods]
impl GridSpec {
    #[new]
    #[pyo3(signature = (nrows=1, ncols=1, left=None, bottom=None, right=None, top=None, wspace=None, hspace=None, width_ratios=None, height_ratios=None))]
    fn new(
        nrows: i32,
        ncols: i32,
        left: Option<f64>,
        bottom: Option<f64>,
        right: Option<f64>,
        top: Option<f64>,
        wspace: Option<f64>,
        hspace: Option<f64>,
        width_ratios: Option<Vec<f64>>,
        height_ratios: Option<Vec<f64>>,
    ) -> Self {
        GridSpec {
            nrows,
            ncols,
            left,
            bottom,
            right,
            top,
            wspace,
            hspace,
            width_ratios: width_ratios.unwrap_or_else(|| vec![1.0; ncols as usize]),
            height_ratios: height_ratios.unwrap_or_else(|| vec![1.0; nrows as usize]),
        }
    }

    fn __getitem__(&self, py: Python, key: &Bound<'_, PyAny>) -> PyResult<Py<SubplotSpec>> {
        if let Ok(tuple) = key.cast::<PyTuple>() {
            if tuple.len() != 2 {
                return Err(PyTypeError::new_err(
                    "GridSpec indices must be tuples (row, col)",
                ));
            }
            let row_spec = tuple.get_item(0)?;
            let col_spec = tuple.get_item(1)?;

            let (row_start, row_end) = self.parse_spec(&row_spec, true)?;
            let (col_start, col_end) = self.parse_spec(&col_spec, false)?;

            let spec = SubplotSpec {
                gridspec: Some(self.clone()),
                row_start,
                row_end,
                col_start,
                col_end,
                num_rows: self.nrows,
                num_cols: self.ncols,
            };
            Ok(Py::new(py, spec)?)
        } else {
            Err(PyTypeError::new_err(
                "GridSpec indices must be tuples (row, col)",
            ))
        }
    }

    fn get_subplot_params(&self, _figure: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<(String, Option<f64>)>> {
        Ok(vec![
            ("left".to_string(), self.left),
            ("bottom".to_string(), self.bottom),
            ("right".to_string(), self.right),
            ("top".to_string(), self.top),
            ("wspace".to_string(), self.wspace),
            ("hspace".to_string(), self.hspace),
        ])
    }

    fn tight_layout(&self, _figure: Option<&Bound<'_, PyAny>>, _renderer: Option<&Bound<'_, PyAny>>) {
        // 占位实现
    }
}

impl GridSpec {
    fn parse_spec(&self, spec: &Bound<'_, PyAny>, is_row: bool) -> PyResult<(i32, i32)> {
        // 检查是否为 slice
        if let Ok(slice) = spec.cast::<PySlice>() {
            let start = slice.getattr("start")?;
            let stop = slice.getattr("stop")?;
            let _step = slice.getattr("step")?;

            let s: i32 = if start.is_none() {
                0
            } else {
                start.extract::<i32>()?
            };
            let default_stop = if is_row { self.nrows } else { self.ncols };
            let e: i32 = if stop.is_none() {
                default_stop
            } else {
                stop.extract::<i32>()?
            };
            Ok((s, e))
        } else {
            // 视为整数索引
            let idx: i32 = spec.extract::<i32>()?;
            Ok((idx, idx + 1))
        }
    }
}

// ==================== SubplotSpec ====================

/// SubplotSpec - 子图定位器
///
/// 字段使用 snake_case （Rust 惯例），
/// 通过 `#[pyo3(name = "...")]` 在 Python 侧显示为 camelCase
/// （与 matplotlib API 兼容）。
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct SubplotSpec {
    pub gridspec: Option<GridSpec>,
    #[pyo3(get, name = "rowStart")]
    pub row_start: i32,
    #[pyo3(get, name = "rowStop")]
    pub row_end: i32,
    #[pyo3(get, name = "colStart")]
    pub col_start: i32,
    #[pyo3(get, name = "colStop")]
    pub col_end: i32,
    #[pyo3(get, name = "numRows")]
    pub num_rows: i32,
    #[pyo3(get, name = "numCols")]
    pub num_cols: i32,
}

#[pymethods]
impl SubplotSpec {
    #[new]
    #[pyo3(signature = (gridspec, row_start=0, row_end=1, col_start=0, col_end=1))]
    fn new(gridspec: Option<GridSpec>, row_start: i32, row_end: i32, col_start: i32, col_end: i32) -> Self {
        let (num_rows, num_cols) = if let Some(ref gs) = gridspec {
            (gs.nrows, gs.ncols)
        } else {
            (row_end.max(1), col_end.max(1))
        };
        SubplotSpec {
            gridspec,
            row_start,
            row_end,
            col_start,
            col_end,
            num_rows,
            num_cols,
        }
    }

    fn get_position(&self, _figure: Option<&Bound<'_, PyAny>>) -> (f64, f64, f64, f64) {
        if let Some(ref gs) = self.gridspec {
            let row_heights = &gs.height_ratios;
            let col_widths = &gs.width_ratios;

            let total_h: f64 = row_heights.iter().sum();
            let total_w: f64 = col_widths.iter().sum();

            let x: f64 = col_widths[..self.col_start as usize].iter().sum::<f64>() / total_w;
            let y: f64 = 1.0 - row_heights[..self.row_end as usize].iter().sum::<f64>() / total_h;
            let w: f64 = col_widths[self.col_start as usize..self.col_end as usize]
                .iter()
                .sum::<f64>()
                / total_w;
            let h: f64 = row_heights[self.row_start as usize..self.row_end as usize]
                .iter()
                .sum::<f64>()
                / total_h;

            (x, y, w, h)
        } else {
            let num_rows_f = self.num_rows as f64;
            let num_cols_f = self.num_cols as f64;
            (
                self.col_start as f64 / num_cols_f,
                1.0 - self.row_end as f64 / num_rows_f,
                (self.col_end - self.col_start) as f64 / num_cols_f,
                (self.row_end - self.row_start) as f64 / num_rows_f,
            )
        }
    }

    fn get_grid_span(&self) -> (i32, i32, i32, i32) {
        (self.row_start, self.row_end, self.col_start, self.col_end)
    }
}

// ==================== 便利函数 ====================

/// 从 SubplotSpec 创建 GridSpec
#[pyfunction]
#[pyo3(signature = (nrows, ncols, _subplot_spec=None, left=None, bottom=None, right=None, top=None, wspace=None, hspace=None, width_ratios=None, height_ratios=None))]
pub fn gridspec_from_subplotspec(
    nrows: i32,
    ncols: i32,
    _subplot_spec: Option<&Bound<'_, PyAny>>,
    left: Option<f64>,
    bottom: Option<f64>,
    right: Option<f64>,
    top: Option<f64>,
    wspace: Option<f64>,
    hspace: Option<f64>,
    width_ratios: Option<Vec<f64>>,
    height_ratios: Option<Vec<f64>>,
) -> GridSpec {
    GridSpec {
        nrows,
        ncols,
        left,
        bottom,
        right,
        top,
        wspace,
        hspace,
        width_ratios: width_ratios.unwrap_or_else(|| vec![1.0; ncols as usize]),
        height_ratios: height_ratios.unwrap_or_else(|| vec![1.0; nrows as usize]),
    }
}

/// 模块注册函数
pub fn register(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GridSpec>()?;
    m.add_class::<SubplotSpec>()?;
    m.add_function(wrap_pyfunction!(gridspec_from_subplotspec, m)?)?;
    Ok(())
}