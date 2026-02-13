use std::time::Duration;

use gpui::{
    AppContext, Application, AsyncWindowContext, Bounds, Timer, WindowBounds, WindowOptions, px,
    size,
};

use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, Plot, PlotHandle, PlotViewConfig, Series,
    SeriesKind, Theme, View,
};

struct LiveDemo {
    view: gpui::Entity<GpuiPlotView>,
    handle: PlotHandle,
}

impl LiveDemo {
    fn new(cx: &mut gpui::Context<Self>) -> Self {
        let (view, handle) = build_view(cx);
        Self { view, handle }
    }
}

impl gpui::Render for LiveDemo {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.view.clone()
    }
}

fn build_view(cx: &mut gpui::Context<LiveDemo>) -> (gpui::Entity<GpuiPlotView>, PlotHandle) {
    let mut series = Series::line("stream").with_kind(SeriesKind::Line(LineStyle {
        color: Color::new(0.2, 0.8, 0.95, 1.0),
        width: 2.0,
    }));

    for i in 0..200 {
        let phase = i as f64 * 0.03;
        let _ = series.push_y(phase.sin());
    }

    let mut plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::linear().with_title("Sample"))
        .y_axis(AxisConfig::linear().with_title("Value"))
        .view(View::FollowLastN { points: 800 })
        .build();
    plot.add_series(series);

    let config = PlotViewConfig {
        show_legend: false,
        show_hover: false,
        ..Default::default()
    };

    let view = GpuiPlotView::with_config(plot, config);
    let handle = view.plot_handle();
    let entity = cx.new(|_| view);
    (entity, handle)
}

fn spawn_updates(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    view: gpui::Entity<GpuiPlotView>,
    handle: PlotHandle,
) {
    window
        .spawn(cx, move |cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let mut phase = 0.0_f64;
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    handle.write(|plot| {
                        if let Some(series) = plot.series_mut().get_mut(0) {
                            for _ in 0..120 {
                                let y = phase.sin();
                                let _ = series.push_y(y);
                                phase += 0.02;
                            }
                        }
                    });
                    let _ = cx.update(|_, cx| {
                        view.update(cx, |_view, view_cx| view_cx.notify());
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
                size(px(900.0), px(520.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let demo = cx.new(LiveDemo::new);
            let handle = demo.read(cx).handle.clone();
            let view = demo.read(cx).view.clone();
            spawn_updates(window, cx, view, handle);
            demo
        })
        .unwrap();
    });
}
