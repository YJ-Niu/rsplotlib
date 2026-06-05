use plotters::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = SVGBackend::new("/tmp/plotters_test.svg", (1400, 750)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .build_cartesian_2d(0..10, 0..10)?;

    chart
        .configure_mesh()
        .x_labels(10)
        .y_labels(10)
        .x_label_style(("sans-serif", 20))
        .y_label_style(("sans-serif", 20))
        .draw()?;

    chart.draw_series(LineSeries::new(
        (0..=10).map(|x| (x, x)),
        &RED,
    ))?;

    // Also try drawing text directly
    chart.draw_series(std::iter::once(plotters::element::Text::new(
        "Direct Text",
        (5, 5),
        ("sans-serif", 20),
    )))?;

    root.present()?;

    Ok(())
}
