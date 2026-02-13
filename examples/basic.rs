use gpui_plot::{AxisConfig, LineStyle, MarkerStyle, Plot, Point, Series, SeriesKind, Theme};

fn main() {
    let mut plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::linear())
        .y_axis(AxisConfig::linear())
        .build();

    let line_values: Vec<f64> = (0..2000).map(|i| (i as f64 * 0.01).sin()).collect();
    let scatter_points: Vec<Point> = (0..200)
        .map(|i| Point::new(i as f64 * 0.1, (i as f64 * 0.1).cos()))
        .collect();

    let line = Series::from_iter_y(
        "line",
        line_values.iter().copied(),
        SeriesKind::Line(LineStyle::default()),
    );
    let scatter = Series::from_iter_points(
        "scatter",
        scatter_points.iter().copied(),
        SeriesKind::Scatter(MarkerStyle::default()),
    );

    plot.add_series(line);
    plot.add_series(scatter);
    plot.refresh_viewport(0.05, 1e-6);

    let _ = plot.viewport().expect("viewport available");

    println!(
        "basic example ready: line={} scatter={}",
        line_values.len(),
        scatter_points.len()
    );
}
