use gpui_plot::{AxisConfig, LineStyle, MarkerStyle, Plot, Point, Series, SeriesKind, Theme};

fn main() {
    let mut plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::linear())
        .y_axis(AxisConfig::log10())
        .build();

    let line_points: Vec<Point> = (1..=1000)
        .map(|i| Point::new(i as f64, (i as f64 * 0.01).exp()))
        .collect();
    let scatter_points: Vec<Point> = (1..=200)
        .map(|i| Point::new(i as f64 * 5.0, (i as f64 * 0.02).exp()))
        .collect();

    let line = Series::from_iter_points(
        "sensor-a",
        line_points.iter().copied(),
        SeriesKind::Line(LineStyle::default()),
    );
    let scatter = Series::from_iter_points(
        "sensor-b",
        scatter_points.iter().copied(),
        SeriesKind::Scatter(MarkerStyle::default()),
    );

    plot.add_series(line);
    plot.add_series(scatter);
    plot.refresh_viewport(0.05, 1e-6);

    let _ = plot.viewport().expect("viewport available");

    println!(
        "multi-series example ready: line={} scatter={}",
        line_points.len(),
        scatter_points.len()
    );
}
