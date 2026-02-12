use gpui_plot::{
    AxisConfig, DecimationScratch, LineStyle, MarkerStyle, Plot, Point, ScreenPoint, ScreenRect,
    Series, SeriesKind, Transform,
};

fn main() {
    let mut plot = Plot::builder()
        .x_axis(AxisConfig::linear())
        .y_axis(AxisConfig::log10())
        .build();

    let line = Series::from_iter_points(
        "sensor-a",
        (1..=1000).map(|i| Point::new(i as f64, (i as f64 * 0.01).exp())),
        SeriesKind::Line(LineStyle::default()),
    );
    let scatter = Series::from_iter_points(
        "sensor-b",
        (1..=200).map(|i| Point::new(i as f64 * 5.0, (i as f64 * 0.02).exp())),
        SeriesKind::Scatter(MarkerStyle::default()),
    );

    plot.add_series(line);
    plot.add_series(scatter);
    plot.refresh_viewport(0.05, 1e-6);

    let viewport = plot.viewport().expect("viewport available");
    let rect = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(1024.0, 640.0));
    let transform =
        Transform::new(viewport, rect, plot.x_axis().scale(), plot.y_axis().scale()).unwrap();

    let mut scratch = DecimationScratch::new();
    for series in plot.series() {
        let decimated = series.data().decimate(viewport.x, 1024, &mut scratch);
        println!(
            "{}: {} raw -> {} decimated",
            series.name(),
            series.data().data().len(),
            decimated.len()
        );
        let _ = transform;
    }

    println!("multi-series example complete");
}
