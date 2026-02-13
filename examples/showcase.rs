use std::time::Duration;

use gpui::{
    AppContext, Application, AsyncWindowContext, Bounds, FontWeight, ParentElement, Styled, Timer,
    WindowBounds, WindowOptions, div, px, size,
};

use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, MarkerShape, MarkerStyle, Plot, PlotHandle, Point,
    Series, SeriesKind, Theme, View,
};

struct Demo {
    live_view: gpui::Entity<GpuiPlotView>,
    live_handle: PlotHandle,
    time_view: gpui::Entity<GpuiPlotView>,
    points: usize,
}

impl Demo {
    fn new(cx: &mut gpui::Context<Self>) -> Self {
        let (live_view, live_handle) = build_live_view(cx);
        let time_view = build_time_view(cx);
        Self {
            live_view,
            live_handle,
            time_view,
            points: 0,
        }
    }
}

impl gpui::Render for Demo {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let header = div()
            .text_xl()
            .font_weight(FontWeight::SEMIBOLD)
            .child("gpui_plot 简洁示例");

        let subtitle = div()
            .text_sm()
            .text_color(gpui::rgb(0xA6A6A6))
            .child("拖拽平移，滚轮缩放，右键框选，点击点位 Pin");

        let live_label = div()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .child(format!("实时流式（当前点数：{}）", self.points));

        let time_label = div()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .child("时间轴静态序列");

        let live_panel = div()
            .w_full()
            .h(px(280.0))
            .border_1()
            .border_color(gpui::rgb(0x333333))
            .child(self.live_view.clone());

        let time_panel = div()
            .w_full()
            .h(px(240.0))
            .border_1()
            .border_color(gpui::rgb(0x333333))
            .child(self.time_view.clone());

        div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(16.0))
            .child(header)
            .child(subtitle)
            .child(div().h(px(12.0)))
            .child(live_label)
            .child(div().h(px(8.0)))
            .child(live_panel)
            .child(div().h(px(16.0)))
            .child(time_label)
            .child(div().h(px(8.0)))
            .child(time_panel)
    }
}

fn build_live_view(cx: &mut gpui::Context<Demo>) -> (gpui::Entity<GpuiPlotView>, PlotHandle) {
    let line_style = LineStyle {
        color: Color::new(0.2, 0.75, 0.95, 1.0),
        width: 2.0,
    };
    let marker_style = MarkerStyle {
        color: Color::new(0.95, 0.55, 0.2, 1.0),
        size: 5.0,
        shape: MarkerShape::Circle,
    };

    let mut line = Series::line("sensor-a").with_kind(SeriesKind::Line(line_style));
    let mut events = Series::scatter("events").with_kind(SeriesKind::Scatter(marker_style));

    for i in 0..200 {
        let x = i as f64 * 0.03;
        let _ = line.push_y(x.sin());
        if i % 15 == 0 {
            let _ = events.push_y((x * 0.4).cos());
        }
    }

    let mut plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::linear().with_title("Sample"))
        .y_axis(AxisConfig::linear().with_title("Amplitude"))
        .view(View::FollowLastN { points: 1500 })
        .build();

    plot.add_series(line);
    plot.add_series(events);

    let view = GpuiPlotView::new(plot);
    let handle = view.plot_handle();
    let entity = cx.new(|_| view);
    (entity, handle)
}

fn build_time_view(cx: &mut gpui::Context<Demo>) -> gpui::Entity<GpuiPlotView> {
    let start = 1_700_000_000.0_f64;
    let points = (0..900).map(|i| {
        let x = start + i as f64 * 60.0;
        let y = (i as f64 * 0.01).sin();
        Point::new(x, y)
    });

    let line = Series::from_iter_points(
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

    plot.add_series(line);

    let view = GpuiPlotView::new(plot);
    cx.new(|_| view)
}

fn spawn_live_updates(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    demo: gpui::Entity<Demo>,
    handle: PlotHandle,
) {
    window
        .spawn(cx, move |cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let mut phase = 0.0_f64;
                let mut count = 0_usize;
                let mut event_tick = 0_usize;

                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let samples = 200;
                    handle.write(|plot| {
                        if let Some(series) = plot.series_mut().get_mut(0) {
                            for _ in 0..samples {
                                let y = phase.sin();
                                let _ = series.push_y(y);
                                phase += 0.01;
                                count += 1;
                            }
                        }
                        if let Some(series) = plot.series_mut().get_mut(1) {
                            event_tick = event_tick.wrapping_add(1);
                            if event_tick.is_multiple_of(10) {
                                let _ = series.push_y((phase * 0.4).cos());
                            }
                        }
                    });

                    cx.update(|_, cx| {
                        demo.update(cx, |this, cx| {
                            this.points = count;
                            this.live_view.update(cx, |_view, view_cx| view_cx.notify());
                            cx.notify();
                        });
                    })
                    .ok();
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
                size(px(980.0), px(720.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let demo = cx.new(Demo::new);
            let live_handle = demo.read(cx).live_handle.clone();
            spawn_live_updates(window, cx, demo.clone(), live_handle);
            demo
        })
        .unwrap();
    });
}
