use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    AppContext, Application, AsyncWindowContext, Bounds, Timer, WindowBounds, WindowOptions, div,
    px, size,
};

use gpui_liveplot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, MarkerShape, MarkerStyle, Plot, PlotLinkGroup,
    PlotLinkOptions, PlotViewConfig, Range, Series, SeriesKind, Theme, View,
};

struct AdvancedDemo {
    top: gpui::Entity<GpuiPlotView>,
    bottom: gpui::Entity<GpuiPlotView>,
}

impl gpui::Render for AdvancedDemo {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().child(self.top.clone()))
            .child(div().flex_1().child(self.bottom.clone()))
    }
}

fn build_views(
    cx: &mut gpui::App,
) -> (
    gpui::Entity<GpuiPlotView>,
    gpui::Entity<GpuiPlotView>,
    Series,
    Series,
) {
    let mut stream_a = Series::line("stream-A").with_kind(SeriesKind::Line(LineStyle {
        color: Color::new(0.2, 0.82, 0.95, 1.0),
        width: 2.0,
    }));
    let mut stream_b = Series::line("stream-B").with_kind(SeriesKind::Line(LineStyle {
        color: Color::new(0.95, 0.64, 0.28, 1.0),
        width: 2.0,
    }));

    for i in 0..1_000 {
        let phase = i as f64 * 0.02;
        let _ = stream_a.push_y((phase * 0.9).sin() + 0.2 * (phase * 0.13).cos());
        let _ = stream_b.push_y((phase * 0.45).cos() * 1.15 + 0.15 * (phase * 0.09).sin());
    }

    let events = Series::from_iter_points(
        "events(scatter)",
        (0..200).map(|i| {
            let x = i as f64 * 80.0 + 40.0;
            let y = (x * 0.02).sin() * 0.9;
            gpui_liveplot::Point::new(x, y)
        }),
        SeriesKind::Scatter(MarkerStyle {
            color: Color::new(0.95, 0.25, 0.55, 1.0),
            size: 5.0,
            shape: MarkerShape::Circle,
        }),
    );

    let baseline = Series::from_explicit_callback(
        "baseline(callback)",
        |x| (x * 0.015).sin() * 0.4,
        Range::new(0.0, 25_000.0),
        5_000,
        SeriesKind::Line(LineStyle {
            color: Color::new(0.45, 0.45, 0.5, 0.8),
            width: 1.0,
        }),
    );

    let mut top_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Sample").build())
        .y_axis(AxisConfig::builder().title("Top: stream + events").build())
        .view(View::FollowLastN { points: 2_000 })
        .build();
    top_plot.add_series(&stream_a);
    top_plot.add_series(&events);

    let mut bottom_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Sample").build())
        .y_axis(
            AxisConfig::builder()
                .title("Bottom: stream + baseline")
                .build(),
        )
        .view(View::FollowLastNXY { points: 2_000 })
        .build();
    bottom_plot.add_series(&stream_b);
    bottom_plot.add_series(&baseline);

    let config = PlotViewConfig {
        show_legend: true,
        show_hover: true,
        ..Default::default()
    };

    let link_group = PlotLinkGroup::new();
    let options = PlotLinkOptions {
        link_x: true,
        link_y: false,
        link_cursor: true,
        link_brush: true,
        link_reset: true,
    };

    let top = cx.new(|_| {
        GpuiPlotView::with_config(top_plot, config.clone())
            .with_link_group(link_group.clone(), options)
    });
    let bottom = cx.new(|_| {
        GpuiPlotView::with_config(bottom_plot, config).with_link_group(link_group, options)
    });

    (top, bottom, stream_a, stream_b)
}

fn spawn_updates(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    top: gpui::Entity<GpuiPlotView>,
    bottom: gpui::Entity<GpuiPlotView>,
    mut stream_a: Series,
    mut stream_b: Series,
) {
    window
        .spawn(cx, move |cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let mut phase = 0.0_f64;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let _ = stream_a.extend_y((0..120).map(|_| {
                        let y = (phase * 0.9).sin() + 0.2 * (phase * 0.13).cos();
                        phase += 0.02;
                        y
                    }));
                    let _ = stream_b.extend_y((0..120).map(|_| {
                        let y = (phase * 0.45).cos() * 1.15 + 0.15 * (phase * 0.09).sin();
                        phase += 0.02;
                        y
                    }));

                    let _ = cx.update(|_, cx| {
                        top.update(cx, |_view, view_cx| view_cx.notify());
                        bottom.update(cx, |_view, view_cx| view_cx.notify());
                    });
                }
            }
        })
        .detach();
}

fn main() {
    Application::new().run(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1_000.0), px(740.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let (top, bottom, stream_a, stream_b) = build_views(cx);
            spawn_updates(window, cx, top.clone(), bottom.clone(), stream_a, stream_b);
            cx.new(|_| AdvancedDemo { top, bottom })
        })
        .unwrap();
    });
}
