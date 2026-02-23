use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    AppContext, Application, AsyncWindowContext, Bounds, Timer, WindowBounds, WindowOptions, div,
    px, size,
};

use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, Plot, PlotLinkGroup, PlotLinkOptions,
    PlotViewConfig, Series, SeriesKind, Theme, View,
};

struct LinkedP0Demo {
    top: gpui::Entity<GpuiPlotView>,
    bottom: gpui::Entity<GpuiPlotView>,
}

impl gpui::Render for LinkedP0Demo {
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
    let mut top_source = Series::line("sensor-A").with_kind(SeriesKind::Line(LineStyle {
        color: Color::new(0.2, 0.8, 0.95, 1.0),
        width: 2.0,
    }));
    let mut bottom_source = Series::line("sensor-B").with_kind(SeriesKind::Line(LineStyle {
        color: Color::new(0.95, 0.65, 0.25, 1.0),
        width: 2.0,
    }));

    for i in 0..600 {
        let phase = i as f64 * 0.02;
        let _ = top_source.push_y((phase * 0.8).sin() + 0.15 * (phase * 0.12).cos());
        let _ = bottom_source.push_y((phase * 0.45).cos() * 1.2);
    }

    let mut top_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Sample").build())
        .y_axis(AxisConfig::builder().title("Channel A").build())
        .view(View::FollowLastN { points: 1_200 })
        .build();
    top_plot.add_series(&top_source);

    let mut bottom_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Sample").build())
        .y_axis(AxisConfig::builder().title("Channel B").build())
        .view(View::FollowLastN { points: 1_200 })
        .build();
    bottom_plot.add_series(&bottom_source);

    let config = PlotViewConfig {
        show_legend: true,
        show_hover: true,
        ..Default::default()
    };

    let link_group = PlotLinkGroup::new();
    let link_options = PlotLinkOptions {
        link_x: true,
        link_y: false,
        link_cursor: false,
        link_brush: false,
        link_reset: true,
    };

    let top = cx.new(|_| {
        GpuiPlotView::with_config(top_plot, config.clone())
            .with_link_group(link_group.clone(), link_options)
    });
    let bottom = cx.new(|_| {
        GpuiPlotView::with_config(bottom_plot, config)
            .with_link_group(link_group, link_options)
    });

    (top, bottom, top_source, bottom_source)
}

fn spawn_updates(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    top: gpui::Entity<GpuiPlotView>,
    bottom: gpui::Entity<GpuiPlotView>,
    mut top_source: Series,
    mut bottom_source: Series,
) {
    window
        .spawn(cx, move |cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let mut phase = 0.0_f64;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let _ = top_source.extend_y((0..120).map(|_| {
                        let y = (phase * 0.8).sin() + 0.15 * (phase * 0.12).cos();
                        phase += 0.02;
                        y
                    }));
                    let _ = bottom_source.extend_y((0..120).map(|_| {
                        let y = (phase * 0.45).cos() * 1.2 + 0.2 * (phase * 0.08).sin();
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
                size(px(960.0), px(720.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let (top, bottom, top_source, bottom_source) = build_views(cx);
            spawn_updates(
                window,
                cx,
                top.clone(),
                bottom.clone(),
                top_source,
                bottom_source,
            );
            cx.new(|_| LinkedP0Demo { top, bottom })
        })
        .unwrap();
    });
}
