use gpui::{AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};

use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, Plot, PlotViewConfig, Point, Series, SeriesKind,
    Theme,
};

fn main() {
    Application::new().run(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(900.0), px(520.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |_window, cx| {
            let start = 1_700_000_000.0_f64;
            let points = (0..240).map(|i| {
                let x = start + i as f64 * 60.0;
                let y = (i as f64 * 0.05).sin();
                Point::new(x, y)
            });

            let series = Series::from_iter_points(
                "timeline",
                points,
                SeriesKind::Line(LineStyle {
                    color: Color::new(0.55, 0.9, 0.65, 1.0),
                    width: 2.0,
                }),
            );

            let mut plot = Plot::builder()
                .theme(Theme::dark())
                .x_axis(AxisConfig::time().with_title("Timestamp"))
                .y_axis(AxisConfig::linear().with_title("Value"))
                .build();
            plot.add_series(series);

            let config = PlotViewConfig {
                show_legend: false,
                show_hover: false,
                ..Default::default()
            };

            let view = GpuiPlotView::with_config(plot, config);
            cx.new(|_| view)
        })
        .unwrap();
    });
}
