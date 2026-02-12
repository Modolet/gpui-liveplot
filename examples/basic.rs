use gpui_plot::{
    AxisConfig, DecimationScratch, LineStyle, MarkerStyle, Plot, Point, ScreenPoint, ScreenRect,
    Series, SeriesKind, Theme, Transform,
};

fn main() {
    let mut plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::linear())
        .y_axis(AxisConfig::linear())
        .build();

    let line = Series::from_iter_y(
        "line",
        (0..2000).map(|i| (i as f64 * 0.01).sin()),
        SeriesKind::Line(LineStyle::default()),
    );
    let scatter = Series::from_iter_points(
        "scatter",
        (0..200).map(|i| Point::new(i as f64 * 0.1, (i as f64 * 0.1).cos())),
        SeriesKind::Scatter(MarkerStyle::default()),
    );

    plot.add_series(line);
    plot.add_series(scatter);
    plot.refresh_viewport(0.05, 1e-6);

    let viewport = plot.viewport().expect("viewport available");
    let rect = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(800.0, 600.0));
    let transform =
        Transform::new(viewport, rect, plot.x_axis().scale(), plot.y_axis().scale()).unwrap();
    let mut scratch = DecimationScratch::new();

    for series in plot.series() {
        let decimated = series.data().decimate(viewport.x, 800, &mut scratch);
        println!(
            "{}: {} points -> {} decimated points",
            series.name(),
            series.data().data().len(),
            decimated.len()
        );
        let _ = transform;
    }

    println!("basic example complete");
}
