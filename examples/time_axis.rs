#[cfg(feature = "time")]
fn main() {
    use gpui_plot::{AxisConfig, LineStyle, Plot, Point, Series, SeriesKind};
    use time::{Duration, OffsetDateTime};

    let start = OffsetDateTime::now_utc();
    let points = (0..3600).map(|i| {
        let dt = start + Duration::seconds(i);
        let x = dt.unix_timestamp() as f64;
        let y = (i as f64 * 0.01).sin();
        Point::new(x, y)
    });

    let series = Series::from_iter_points("time", points, SeriesKind::Line(LineStyle::default()));
    let mut plot = Plot::builder().x_axis(AxisConfig::time()).build();
    plot.add_series(series);
    plot.refresh_viewport(0.02, 1e-6);

    println!("time-axis example ready: {} series", plot.series().len());
}

#[cfg(not(feature = "time"))]
fn main() {
    println!("Enable the time feature to run this example:");
    println!("cargo run --example time_axis --features time");
}
