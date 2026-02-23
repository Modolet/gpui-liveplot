use gpui::prelude::*;
use gpui::{AppContext, Application, Bounds, WindowBounds, WindowOptions, div, px, size};

use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, Plot, PlotLinkGroup, PlotLinkOptions,
    PlotViewConfig, Series, SeriesKind, Theme,
};

struct LinkedP1Demo {
    overview: gpui::Entity<GpuiPlotView>,
    detail: gpui::Entity<GpuiPlotView>,
}

impl gpui::Render for LinkedP1Demo {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().child(self.overview.clone()))
            .child(div().flex_1().child(self.detail.clone()))
    }
}

fn build_views(cx: &mut gpui::App) -> (gpui::Entity<GpuiPlotView>, gpui::Entity<GpuiPlotView>) {
    let overview_series = Series::from_iter_points(
        "overview",
        (0..24_000).map(|i| {
            let x = i as f64 * 0.005;
            let y = (x * 0.7).sin() + 0.35 * (x * 0.11).cos();
            gpui_plot::Point::new(x, y)
        }),
        SeriesKind::Line(LineStyle {
            color: Color::new(0.25, 0.78, 0.95, 1.0),
            width: 1.8,
        }),
    );

    let detail_series = Series::from_iter_points(
        "detail",
        (0..24_000).map(|i| {
            let x = i as f64 * 0.005;
            let y = (x * 0.7).sin() + 0.35 * (x * 0.11).cos() + 0.1 * (x * 2.5).sin();
            gpui_plot::Point::new(x, y)
        }),
        SeriesKind::Line(LineStyle {
            color: Color::new(0.95, 0.64, 0.28, 1.0),
            width: 1.8,
        }),
    );

    let mut overview_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Global X").build())
        .y_axis(AxisConfig::builder().title("Overview").build())
        .build();
    overview_plot.add_series(&overview_series);

    let mut detail_plot = Plot::builder()
        .theme(Theme::dark())
        .x_axis(AxisConfig::builder().title("Global X").build())
        .y_axis(AxisConfig::builder().title("Detail").build())
        .build();
    detail_plot.add_series(&detail_series);

    let config = PlotViewConfig {
        show_legend: true,
        show_hover: true,
        ..Default::default()
    };

    let group = PlotLinkGroup::new();
    let options = PlotLinkOptions {
        link_x: true,
        link_y: false,
        link_cursor: true,
        link_brush: true,
        link_reset: true,
    };

    let overview = cx.new(|_| {
        GpuiPlotView::with_config(overview_plot, config.clone()).with_link_group(group.clone(), options)
    });
    let detail =
        cx.new(|_| GpuiPlotView::with_config(detail_plot, config).with_link_group(group, options));

    (overview, detail)
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

        cx.open_window(options, |_window, cx| {
            let (overview, detail) = build_views(cx);
            cx.new(|_| LinkedP1Demo { overview, detail })
        })
        .unwrap();
    });
}
