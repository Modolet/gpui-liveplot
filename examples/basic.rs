use gpui::{AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};

use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, Plot, PlotViewConfig, Series, SeriesKind, Theme,
};

fn main() {
    Application::new().run(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(720.0), px(480.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |_window, cx| {
            let series = Series::from_iter_y(
                "signal",
                (0..400).map(|i| {
                    let x = i as f64 * 0.03;
                    x.sin()
                }),
                SeriesKind::Line(LineStyle {
                    color: Color::new(0.2, 0.75, 0.95, 1.0),
                    width: 2.0,
                }),
            );

            let mut plot = Plot::builder()
                .theme(Theme::dark())
                .x_axis(AxisConfig::linear().with_title("Sample"))
                .y_axis(AxisConfig::linear().with_title("Amplitude"))
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
