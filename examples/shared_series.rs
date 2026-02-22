use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    AppContext, Application, AsyncWindowContext, Bounds, Timer, WindowBounds, WindowOptions, div,
    px, size,
};

use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, Plot, PlotViewConfig, Series, SeriesKind, Theme,
    View,
};

struct SharedSeriesDemo {
    top: gpui::Entity<GpuiPlotView>,
    bottom: gpui::Entity<GpuiPlotView>,
}

impl gpui::Render for SharedSeriesDemo {
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
) {
    let mut shared_source = Series::line("shared-stream").with_kind(SeriesKind::Line(LineStyle {
        color: Color::new(0.25, 0.8, 0.95, 1.0),
        width: 2.0,
    }));
    let _ = shared_source.extend_y((0..300).map(|i| {
        let x = i as f64 * 0.03;
        x.sin()
    }));

    let mut top_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Sample").build())
        .y_axis(AxisConfig::builder().title("Plot A").build())
        .view(View::FollowLastN { points: 1000 })
        .build();
    top_plot.add_shared_series(&shared_source);

    let mut bottom_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Sample").build())
        .y_axis(AxisConfig::builder().title("Plot B").build())
        .view(View::FollowLastN { points: 1000 })
        .build();
    bottom_plot.add_shared_series(&shared_source);

    let config = PlotViewConfig {
        show_legend: true,
        show_hover: true,
        ..Default::default()
    };

    let top = cx.new(|_| GpuiPlotView::with_config(top_plot, config.clone()));
    let bottom = cx.new(|_| GpuiPlotView::with_config(bottom_plot, config));
    (top, bottom, shared_source)
}

fn spawn_updates(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    top: gpui::Entity<GpuiPlotView>,
    bottom: gpui::Entity<GpuiPlotView>,
    mut shared_source: Series,
) {
    window
        .spawn(cx, move |cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let mut phase = 0.0_f64;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let _ = shared_source.extend_y((0..120).map(|_| {
                        let y = (phase * 0.9).sin() + 0.25 * (phase * 0.15).cos();
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
                size(px(900.0), px(700.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let (top, bottom, shared_source) = build_views(cx);
            spawn_updates(window, cx, top.clone(), bottom.clone(), shared_source);
            cx.new(|_| SharedSeriesDemo { top, bottom })
        })
        .unwrap();
    });
}
