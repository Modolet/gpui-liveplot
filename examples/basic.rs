use gpui_kit::{AppContext, Bounds, WindowBounds, WindowOptions, px, size};

use gpui_liveplot::{
    AxisConfig, LineStyle, Plot, PlotView, PlotViewConfig, Rgba, Series, SeriesKind, Theme,
};

fn main() {
    gpui_kit::application().run(|cx| {
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
                    color: Rgba {
                        r: 0.2,
                        g: 0.75,
                        b: 0.95,
                        a: 1.0,
                    },
                    width: 2.0,
                }),
            );

            let mut plot = Plot::builder()
                .theme(Theme::dark())
                .x_axis(AxisConfig::builder().title("Sample").build())
                .y_axis(AxisConfig::builder().title("Amplitude").build())
                .build();
            plot.add_series(&series);

            let config = PlotViewConfig {
                show_legend: true,
                show_hover: true,
                ..Default::default()
            };

            let view = PlotView::with_config(plot, config);
            cx.new(|_| view)
        })
        .unwrap();
    });
}
