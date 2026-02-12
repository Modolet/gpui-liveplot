#[cfg(feature = "gpui")]
use std::time::Duration;

#[cfg(feature = "gpui")]
use gpui::{
    AppContext, Application, AsyncWindowContext, Bounds, Timer, WindowBounds, WindowOptions, px,
    size,
};

#[cfg(feature = "gpui")]
use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, MarkerShape, MarkerStyle, Plot, Series, SeriesKind,
    Theme,
};

#[cfg(feature = "gpui")]
fn main() {
    Application::new().run(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(900.0), px(600.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let mut plot = Plot::builder()
                .theme(Theme::dark())
                .x_axis(AxisConfig::linear())
                .y_axis(AxisConfig::linear())
                // .view(View::FollowLastN { points: 2000 })
                .build();

            let line_style = LineStyle {
                color: Color::new(0.2, 0.8, 0.9, 1.0),
                width: 2.0,
            };
            let scatter_style = MarkerStyle {
                color: Color::new(0.95, 0.55, 0.2, 1.0),
                size: 6.0,
                shape: MarkerShape::Circle,
            };

            let line = Series::from_iter_y(
                "sensor",
                (0..1000).map(|i| (i as f64 * 0.01).sin()),
                SeriesKind::Line(line_style),
            );
            let scatter = Series::scatter("events").with_kind(SeriesKind::Scatter(scatter_style));
            plot.add_series(line);
            plot.add_series(scatter);

            let view = GpuiPlotView::new(plot);
            let plot_handle = view.plot_handle();
            let view_handle = cx.new(|_| view);

            let view_for_task = view_handle.clone();
            window
                .spawn(cx, move |cx: &mut AsyncWindowContext| {
                    let mut cx = cx.clone();
                    async move {
                        let mut phase = 0.0_f64;
                        loop {
                            Timer::after(Duration::from_millis(16)).await;
                            let phase_step = 0.01;
                            let samples = 500;
                            cx.update(|_, cx| {
                                view_for_task.update(cx, |_view, view_cx| {
                                    plot_handle.write(|plot| {
                                        if let Some(series) = plot.series_mut().get_mut(0) {
                                            for _ in 0..samples {
                                                let _ = series.push_y(phase.sin());
                                                phase += phase_step;
                                            }
                                        }
                                        if let Some(series) = plot.series_mut().get_mut(1) {
                                            let _ = series.push_y((phase * 0.5).cos());
                                        }
                                    });
                                    view_cx.notify();
                                });
                            })
                            .ok();
                        }
                    }
                })
                .detach();

            view_handle
        })
        .unwrap();
    });
}

#[cfg(not(feature = "gpui"))]
fn main() {
    eprintln!("Enable the gpui feature to run this example.");
}
