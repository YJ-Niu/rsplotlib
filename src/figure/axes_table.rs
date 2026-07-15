use plotters::coord::Shift;
use plotters::element::PathElement;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::core::colors::{RgbColor, parse_color, to_plotters_color};
use crate::figure::axes::TableSpec;

pub fn draw_table<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    table: &TableSpec,
    font_scale: f64,
    subplot_x: f64,
    subplot_y: f64,
    subplot_w: f64,
    subplot_h: f64,
) -> PyResult<()>
where
    DB::ErrorType: 'static,
{
    if table.cell_text.is_empty() {
        return Ok(());
    }

    let num_cols = table.cell_text[0].len();
    let col_widths = if !table.col_widths.is_empty() && table.col_widths.len() >= num_cols {
        &table.col_widths[0..num_cols]
    } else {
        &vec![1.0 / num_cols as f64; num_cols]
    };

    let font_size = table.fontsize * font_scale;
    let line_height = font_size * 1.5;

    let row_label_width = if !table.row_labels.is_empty() {
        font_size * 3.0
    } else {
        0.0
    };

    let num_header_rows = if !table.col_labels.is_empty() { 1 } else { 0 };
    let num_data_rows = table.cell_text.len();
    let total_rows = num_header_rows + num_data_rows;

    let total_table_height = total_rows as f64 * line_height;
    let col_area_width = subplot_w - row_label_width;

    let table_x = subplot_x;
    let table_y = match table.loc.as_str() {
        "top" => subplot_y + 5.0 * font_scale,
        "center" => subplot_y + subplot_h / 2.0 - total_table_height / 2.0,
        _ => subplot_y + subplot_h + 2.0 * font_scale,
    };

    let col_start_x = table_x + row_label_width;
    let mut y_pos = table_y;

    if !table.col_labels.is_empty() {
        let mut x_pos = col_start_x;
        for (col_idx, width) in col_widths.iter().enumerate() {
            let col_width = col_area_width * width;
            if col_idx < table.col_labels.len() {
                let label = &table.col_labels[col_idx];
                let bg_style = to_plotters_color(RgbColor(240, 240, 240)).filled();
                let rect_pts = vec![
                    (x_pos.round() as i32, y_pos.round() as i32),
                    ((x_pos + col_width).round() as i32, y_pos.round() as i32),
                    (
                        (x_pos + col_width).round() as i32,
                        (y_pos + line_height).round() as i32,
                    ),
                    (x_pos.round() as i32, (y_pos + line_height).round() as i32),
                ];
                root.draw(&PathElement::new(rect_pts, bg_style))
                    .map_err(|e| PyRuntimeError::new_err(format!("Table col label rect: {}", e)))?;

                let style = FontDesc::from(("sans-serif", font_size))
                    .color(&BLACK)
                    .pos(Pos::new(HPos::Center, VPos::Center));
                let text_x = (x_pos + col_width / 2.0).round() as i32;
                let text_y = (y_pos + line_height / 2.0).round() as i32;
                root.draw_text(label, &style, (text_x, text_y))
                    .map_err(|e| PyRuntimeError::new_err(format!("Table col label text: {}", e)))?;
            }
            x_pos += col_width;
        }
        y_pos += line_height;
    }

    for (row_idx, row) in table.cell_text.iter().enumerate() {
        let x_pos = table_x;

        if !table.row_labels.is_empty() && row_idx < table.row_labels.len() {
            let label = &table.row_labels[row_idx];
            let bg_color = if !table.row_colors.is_empty() && row_idx < table.row_colors.len() {
                parse_color(&table.row_colors[row_idx], 0).unwrap_or(RgbColor(255, 255, 255))
            } else {
                RgbColor(255, 255, 255)
            };
            let bg_style = to_plotters_color(bg_color).filled();
            let rect_pts = vec![
                (x_pos.round() as i32, y_pos.round() as i32),
                (
                    (x_pos + row_label_width).round() as i32,
                    y_pos.round() as i32,
                ),
                (
                    (x_pos + row_label_width).round() as i32,
                    (y_pos + line_height).round() as i32,
                ),
                (x_pos.round() as i32, (y_pos + line_height).round() as i32),
            ];
            root.draw(&PathElement::new(rect_pts, bg_style))
                .map_err(|e| PyRuntimeError::new_err(format!("Table row label rect: {}", e)))?;

            let style = FontDesc::from(("sans-serif", font_size))
                .color(&BLACK)
                .pos(Pos::new(HPos::Center, VPos::Center));
            let text_x = (x_pos + row_label_width / 2.0).round() as i32;
            let text_y = (y_pos + line_height / 2.0).round() as i32;
            root.draw_text(label, &style, (text_x, text_y))
                .map_err(|e| PyRuntimeError::new_err(format!("Table row label text: {}", e)))?;
        }

        let mut col_x = col_start_x;
        for (col_idx, text) in row.iter().enumerate() {
            let col_width = if col_idx < col_widths.len() {
                col_area_width * col_widths[col_idx]
            } else {
                col_area_width / num_cols as f64
            };

            let bg_color = if !table.row_colors.is_empty() && row_idx < table.row_colors.len() {
                parse_color(&table.row_colors[row_idx], 0).unwrap_or(RgbColor(255, 255, 255))
            } else {
                RgbColor(255, 255, 255)
            };
            let bg_style = to_plotters_color(bg_color).filled();
            let rect_pts = vec![
                (col_x.round() as i32, y_pos.round() as i32),
                ((col_x + col_width).round() as i32, y_pos.round() as i32),
                (
                    (col_x + col_width).round() as i32,
                    (y_pos + line_height).round() as i32,
                ),
                (col_x.round() as i32, (y_pos + line_height).round() as i32),
            ];
            root.draw(&PathElement::new(rect_pts, bg_style))
                .map_err(|e| PyRuntimeError::new_err(format!("Table cell rect: {}", e)))?;

            let style = FontDesc::from(("sans-serif", font_size))
                .color(&BLACK)
                .pos(Pos::new(HPos::Center, VPos::Center));
            let text_x = (col_x + col_width / 2.0).round() as i32;
            let text_y = (y_pos + line_height / 2.0).round() as i32;
            root.draw_text(text, &style, (text_x, text_y))
                .map_err(|e| PyRuntimeError::new_err(format!("Table cell text: {}", e)))?;
            col_x += col_width;
        }

        y_pos += line_height;
    }

    Ok(())
}
