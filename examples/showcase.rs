use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AppContext, Application, AsyncWindowContext, Bounds, IntoElement, ParentElement, Styled, Timer,
    WindowBounds, WindowOptions, div, px, size,
};
use gpui_component::{
    ActiveTheme, Root, StyledExt, Theme as ComponentTheme, ThemeMode,
    badge::Badge,
    button::{Button, ButtonVariants},
    divider::Divider,
    group_box::{GroupBox, GroupBoxVariants},
    h_flex,
    slider::{Slider, SliderEvent, SliderState},
    switch::Switch,
    tab::TabBar,
    v_flex,
};
use gpui_plot::{
    AxisConfig, Color, GpuiPlotView, LineStyle, MarkerShape, MarkerStyle, Pin, Plot, PlotHandle,
    PlotViewConfig, Point, Range, Series, SeriesId, SeriesKind, Theme as PlotTheme, View,
};

#[derive(Clone)]
struct PlotPanel {
    view: gpui::Entity<GpuiPlotView>,
    plot: PlotHandle,
}

#[derive(Clone)]
struct LivePanel {
    panel: PlotPanel,
    line_id: SeriesId,
}

#[derive(Clone)]
struct MultiPanel {
    panel: PlotPanel,
    series_ids: [SeriesId; 2],
}

#[derive(Clone)]
struct LiveRuntime {
    running: Arc<AtomicBool>,
    rate: Arc<AtomicUsize>,
    points: Arc<AtomicUsize>,
    last_index: Arc<AtomicUsize>,
}

impl LiveRuntime {
    fn new(default_rate: usize) -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
            rate: Arc::new(AtomicUsize::new(default_rate)),
            points: Arc::new(AtomicUsize::new(0)),
            last_index: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[derive(Clone, Copy)]
enum TabKind {
    Live,
    Multi,
    Sampled,
    Time,
}

struct TabSpec {
    label: &'static str,
    kind: TabKind,
}

struct Showcase {
    tabs: Vec<TabSpec>,
    selected: usize,
    theme_light: bool,
    slider: gpui::Entity<SliderState>,
    live_panel: LivePanel,
    live_runtime: LiveRuntime,
    multi_panel: MultiPanel,
    multi_visible: [bool; 2],
    sampled_panel: PlotPanel,
    time_panel: PlotPanel,
}

impl Showcase {
    fn new(
        cx: &mut gpui::Context<Self>,
        live_panel: LivePanel,
        multi_panel: MultiPanel,
        sampled_panel: PlotPanel,
        time_panel: PlotPanel,
        live_runtime: LiveRuntime,
    ) -> Self {
        let slider = cx.new(|_| {
            SliderState::new()
                .min(50.0)
                .max(2000.0)
                .step(50.0)
                .default_value(live_runtime.rate.load(Ordering::Relaxed) as f32)
        });

        let rate_handle = live_runtime.rate.clone();
        let _ = cx.subscribe(&slider, move |_, _, event, cx| match event {
            SliderEvent::Change(value) => {
                let next = value.end().round().max(1.0) as usize;
                rate_handle.store(next, Ordering::Relaxed);
                cx.notify();
            }
        });

        let tabs = vec![
            TabSpec {
                label: "Live Stream",
                kind: TabKind::Live,
            },
            TabSpec {
                label: "Multi Series",
                kind: TabKind::Multi,
            },
            TabSpec {
                label: "Sampled + Dense",
                kind: TabKind::Sampled,
            },
            TabSpec {
                label: "Time Axis",
                kind: TabKind::Time,
            },
        ];

        let this = Self {
            tabs,
            selected: 0,
            theme_light: !cx.theme().is_dark(),
            slider,
            live_panel,
            live_runtime,
            multi_panel,
            multi_visible: [true, true],
            sampled_panel,
            time_panel,
        };
        this.apply_theme();
        this
    }

    fn apply_theme(&self) {
        let theme = if self.theme_light {
            PlotTheme::light()
        } else {
            PlotTheme::dark()
        };

        self.live_panel
            .panel
            .plot
            .write(|plot| plot.set_theme(theme.clone()));
        self.multi_panel
            .panel
            .plot
            .write(|plot| plot.set_theme(theme.clone()));
        self.sampled_panel
            .plot
            .write(|plot| plot.set_theme(theme.clone()));
        self.time_panel.plot.write(|plot| plot.set_theme(theme));
    }

    fn plot_for_kind(&self, kind: TabKind) -> PlotHandle {
        match kind {
            TabKind::Live => self.live_panel.panel.plot.clone(),
            TabKind::Multi => self.multi_panel.panel.plot.clone(),
            TabKind::Sampled => self.sampled_panel.plot.clone(),
            TabKind::Time => self.time_panel.plot.clone(),
        }
    }

    fn view_for_kind(&self, kind: TabKind) -> gpui::AnyElement {
        match kind {
            TabKind::Live => self.live_panel.panel.view.clone().into_any_element(),
            TabKind::Multi => self.multi_panel.panel.view.clone().into_any_element(),
            TabKind::Sampled => self.sampled_panel.view.clone().into_any_element(),
            TabKind::Time => self.time_panel.view.clone().into_any_element(),
        }
    }
}

impl gpui::Render for Showcase {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let theme_light = !cx.theme().is_dark();
        if self.theme_light != theme_light {
            self.theme_light = theme_light;
            self.apply_theme();
        }
        let running = self.live_runtime.running.load(Ordering::Relaxed);
        let rate = self.live_runtime.rate.load(Ordering::Relaxed);
        let points = self.live_runtime.points.load(Ordering::Relaxed);
        let selected_kind = self
            .tabs
            .get(self.selected)
            .map(|tab| tab.kind)
            .unwrap_or(TabKind::Live);

        let entity = cx.entity();

        let tab_bar = TabBar::new("plot-tabs")
            .segmented()
            .selected_index(self.selected)
            .on_click({
                let entity = entity.clone();
                move |index, _, cx| {
                    entity.update(cx, |this, cx| {
                        this.selected = *index;
                        cx.notify();
                    });
                }
            })
            .children(self.tabs.iter().map(|tab| tab.label));

        let plot_view = self.view_for_kind(selected_kind);

        let plot_container = v_flex()
            .flex_1()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(plot_view);

        let reset_plot = self.plot_for_kind(selected_kind);
        let clear_plot = self.plot_for_kind(selected_kind);
        let reset_button = Button::new("reset-view").label("Reset View").on_click({
            let entity = entity.clone();
            move |_, _, cx| {
                reset_plot.write(|plot| plot.reset_view());
                entity.update(cx, |_, cx| cx.notify());
            }
        });

        let clear_button = Button::new("clear-pins").label("Clear Pins").on_click({
            let entity = entity.clone();
            move |_, _, cx| {
                clear_plot.write(|plot| plot.pins_mut().clear());
                entity.update(cx, |_, cx| cx.notify());
            }
        });

        let pin_button = if matches!(selected_kind, TabKind::Live) {
            let plot = self.live_panel.panel.plot.clone();
            let line_id = self.live_panel.line_id;
            let last_index = self.live_runtime.last_index.clone();
            let entity = entity.clone();
            Some(
                Button::new("pin-latest")
                    .label("Pin Latest")
                    .primary()
                    .on_click(move |_, _, cx| {
                        let index = last_index.load(Ordering::Relaxed);
                        plot.write(|plot| {
                            let pin = Pin {
                                series_id: line_id,
                                point_index: index,
                            };
                            let exists = plot.pins().contains(&pin);
                            if !exists {
                                plot.pins_mut().push(pin);
                            }
                        });
                        entity.update(cx, |_, cx| cx.notify());
                    }),
            )
        } else {
            None
        };

        let view_controls = v_flex()
            .gap_2()
            .child(reset_button)
            .child(clear_button)
            .when_some(pin_button, |this, button| this.child(button));

        let running_switch = Switch::new("live-running")
            .checked(running)
            .label("Streaming")
            .on_click({
                let running = self.live_runtime.running.clone();
                let entity = entity.clone();
                move |next, _, cx| {
                    running.store(*next, Ordering::Relaxed);
                    entity.update(cx, |_, cx| cx.notify());
                }
            });

        let theme_switch = Switch::new("theme-light")
            .checked(theme_light)
            .label("Light Theme")
            .on_click({
                let entity = entity.clone();
                move |next, window, cx| {
                    let mode = if *next {
                        ThemeMode::Light
                    } else {
                        ThemeMode::Dark
                    };
                    ComponentTheme::change(mode, Some(window), cx);
                    entity.update(cx, |this, cx| {
                        this.theme_light = *next;
                        this.apply_theme();
                        cx.notify();
                    });
                }
            });

        let live_group = GroupBox::new().title("Live Stream").fill().child(
            v_flex().gap_3().child(running_switch).child(
                v_flex()
                    .gap_1()
                    .child("Samples / frame")
                    .child(Slider::new(&self.slider).horizontal())
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Badge::new().count(points).child("Points"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Rate: {rate}")),
                            ),
                    ),
            ),
        );

        let view_group = GroupBox::new()
            .title("View & Pins")
            .outline()
            .child(view_controls);

        let multi_group = if matches!(selected_kind, TabKind::Multi) {
            let plot = self.multi_panel.panel.plot.clone();
            let entity = entity.clone();
            let series_a_id = self.multi_panel.series_ids[0];
            let series_b_id = self.multi_panel.series_ids[1];
            let series_a_visible = self.multi_visible[0];
            let series_b_visible = self.multi_visible[1];

            let series_a = Switch::new("series-a")
                .checked(series_a_visible)
                .label("Sensor A")
                .on_click({
                    let plot = plot.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        let visible = *next;
                        plot.write(|plot| {
                            if let Some(series) = plot
                                .series_mut()
                                .iter_mut()
                                .find(|series| series.id() == series_a_id)
                            {
                                series.set_visible(visible);
                            }
                        });
                        entity.update(cx, |this, cx| {
                            this.multi_visible[0] = visible;
                            cx.notify();
                        });
                    }
                });

            let series_b = Switch::new("series-b")
                .checked(series_b_visible)
                .label("Sensor B")
                .on_click({
                    let plot = plot.clone();
                    move |next, _, cx| {
                        let visible = *next;
                        plot.write(|plot| {
                            if let Some(series) = plot
                                .series_mut()
                                .iter_mut()
                                .find(|series| series.id() == series_b_id)
                            {
                                series.set_visible(visible);
                            }
                        });
                        entity.update(cx, |this, cx| {
                            this.multi_visible[1] = visible;
                            cx.notify();
                        });
                    }
                });

            GroupBox::new()
                .title("Multi Series")
                .outline()
                .child(v_flex().gap_2().child(series_a).child(series_b))
        } else {
            GroupBox::new().title("Multi Series").outline().child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Open the Multi Series tab to toggle visibility."),
            )
        };

        let tips_group = GroupBox::new().title("Interaction Tips").outline().child(
            v_flex()
                .gap_1()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Left drag: pan")
                .child("Right drag: zoom to rectangle")
                .child("Wheel: zoom at cursor")
                .child("Double click: reset view")
                .child("Click point: pin/unpin"),
        );

        let header = h_flex()
            .w_full()
            .p_4()
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().font_semibold().child("gpui_plot Showcase"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("GPUI Component UI + gpui_plot features"),
                    ),
            )
            .child(
                h_flex().gap_3().items_center().child(theme_switch).child(
                    Badge::new()
                        .count(points)
                        .child(div().text_sm().child("Live Points")),
                ),
            );

        let sidebar = v_flex()
            .w(px(320.0))
            .gap_4()
            .p_4()
            .child(live_group)
            .child(view_group)
            .child(multi_group)
            .child(tips_group);

        let main = v_flex()
            .size_full()
            .gap_3()
            .p_4()
            .child(tab_bar)
            .child(plot_container);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(header)
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .size_full()
                    .child(sidebar)
                    .child(Divider::vertical())
                    .child(main),
            )
    }
}

fn build_panel(cx: &mut gpui::App, plot: Plot, config: PlotViewConfig) -> PlotPanel {
    let view = GpuiPlotView::with_config(plot, config);
    let plot_handle = view.plot_handle();
    let view_handle = cx.new(|_| view);
    PlotPanel {
        view: view_handle,
        plot: plot_handle,
    }
}

fn build_live_panel(cx: &mut gpui::App) -> LivePanel {
    let line_style = LineStyle {
        color: Color::new(0.2, 0.75, 0.95, 1.0),
        width: 2.0,
    };
    let marker_style = MarkerStyle {
        color: Color::new(0.95, 0.55, 0.2, 1.0),
        size: 6.0,
        shape: MarkerShape::Circle,
    };

    let mut line = Series::line("sensor-a").with_kind(SeriesKind::Line(line_style));
    let mut events = Series::scatter("events").with_kind(SeriesKind::Scatter(marker_style));

    for i in 0..400 {
        let x = i as f64 * 0.02;
        let _ = line.push_y(x.sin());
        if i % 20 == 0 {
            let _ = events.push_y((x * 0.4).cos());
        }
    }

    let line_id = line.id();

    let mut plot = Plot::builder()
        .theme(PlotTheme::dark())
        .x_axis(
            AxisConfig::linear()
                .with_title("Sample")
                .with_units("idx")
                .with_minor_grid(true),
        )
        .y_axis(AxisConfig::linear().with_title("Amplitude"))
        .view(View::FollowLastN { points: 2000 })
        .build();

    plot.add_series(line);
    plot.add_series(events);

    let panel = build_panel(cx, plot, PlotViewConfig::default());
    LivePanel { panel, line_id }
}

fn build_multi_panel(cx: &mut gpui::App) -> MultiPanel {
    let line_values = (0..6000).map(|i| (i as f64 * 0.004).sin());
    let scatter_points = (0..320).map(|i| Point::new(i as f64 * 0.3, (i as f64 * 0.2).cos()));

    let line = Series::from_iter_y(
        "sensor-a",
        line_values,
        SeriesKind::Line(LineStyle {
            color: Color::new(0.3, 0.9, 0.6, 1.0),
            width: 2.0,
        }),
    );
    let scatter = Series::from_iter_points(
        "sensor-b",
        scatter_points,
        SeriesKind::Scatter(MarkerStyle {
            color: Color::new(0.9, 0.35, 0.45, 1.0),
            size: 5.0,
            shape: MarkerShape::Square,
        }),
    );

    let series_ids = [line.id(), scatter.id()];

    let mut plot = Plot::builder()
        .theme(PlotTheme::dark())
        .x_axis(
            AxisConfig::linear()
                .with_title("Time")
                .with_units("s")
                .with_minor_grid(true),
        )
        .y_axis(AxisConfig::linear().with_title("Value"))
        .build();

    plot.add_series(line);
    plot.add_series(scatter);

    let panel = build_panel(cx, plot, PlotViewConfig::default());
    MultiPanel { panel, series_ids }
}

fn build_sampled_panel(cx: &mut gpui::App) -> PlotPanel {
    let range = Range::new(0.0, 240.0);
    let sampled = Series::from_explicit_callback(
        "sampled",
        |x| (x * 0.15).sin() + (x * 0.03).cos() * 0.6,
        range,
        60000,
        SeriesKind::Line(LineStyle {
            color: Color::new(0.4, 0.7, 1.0, 1.0),
            width: 1.5,
        }),
    );

    let markers = (0..240).map(|i| Point::new(i as f64, (i as f64 * 0.18).sin() * 0.8));
    let scatter = Series::from_iter_points(
        "markers",
        markers,
        SeriesKind::Scatter(MarkerStyle {
            color: Color::new(0.95, 0.5, 0.3, 1.0),
            size: 4.0,
            shape: MarkerShape::Circle,
        }),
    );

    let mut plot = Plot::builder()
        .theme(PlotTheme::dark())
        .x_axis(AxisConfig::linear().with_title("x"))
        .y_axis(AxisConfig::linear().with_title("f(x)"))
        .build();

    plot.add_series(sampled);
    plot.add_series(scatter);

    build_panel(cx, plot, PlotViewConfig::default())
}

fn build_time_panel(cx: &mut gpui::App) -> PlotPanel {
    let start = 1_700_000_000.0_f64;
    let points = (0..1800).map(|i| {
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
        .theme(PlotTheme::dark())
        .x_axis(AxisConfig::time().with_title("Timestamp"))
        .y_axis(AxisConfig::linear().with_title("Value"))
        .build();

    plot.add_series(line);

    build_panel(cx, plot, PlotViewConfig::default())
}

fn spawn_live_updates(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    live_panel: LivePanel,
    live_runtime: LiveRuntime,
    showcase: gpui::Entity<Showcase>,
) {
    let plot_handle = live_panel.panel.plot.clone();
    let view_handle = live_panel.panel.view.clone();
    let running = live_runtime.running.clone();
    let rate = live_runtime.rate.clone();
    let points = live_runtime.points.clone();
    let last_index = live_runtime.last_index.clone();

    window
        .spawn(cx, move |cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let mut phase = 0.0_f64;
                let mut count = 0_usize;
                let mut event_tick = 0_usize;

                loop {
                    Timer::after(Duration::from_millis(16)).await;

                    if !running.load(Ordering::Relaxed) {
                        continue;
                    }

                    let samples = rate.load(Ordering::Relaxed).max(1);
                    let mut last = None;

                    plot_handle.write(|plot| {
                        if let Some(series) = plot.series_mut().get_mut(0) {
                            for _ in 0..samples {
                                let y = (phase).sin();
                                last = series.push_y(y).ok();
                                phase += 0.01;
                                count += 1;
                            }
                        }

                        if let Some(series) = plot.series_mut().get_mut(1) {
                            event_tick = event_tick.wrapping_add(1);
                            if event_tick.is_multiple_of(8) {
                                let _ = series.push_y((phase * 0.4).cos());
                            }
                        }
                    });

                    if let Some(index) = last {
                        last_index.store(index, Ordering::Relaxed);
                    }
                    points.store(count, Ordering::Relaxed);

                    cx.update(|_, cx| {
                        view_handle.update(cx, |_view, view_cx| {
                            view_cx.notify();
                        });
                        showcase.update(cx, |_, cx| {
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
        gpui_component::init(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1280.0), px(820.0)),
                cx,
            ))),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let live_panel = build_live_panel(cx);
            let multi_panel = build_multi_panel(cx);
            let sampled_panel = build_sampled_panel(cx);
            let time_panel = build_time_panel(cx);

            let live_runtime = LiveRuntime::new(400);

            let showcase = cx.new(|cx| {
                Showcase::new(
                    cx,
                    live_panel.clone(),
                    multi_panel.clone(),
                    sampled_panel.clone(),
                    time_panel.clone(),
                    live_runtime.clone(),
                )
            });

            spawn_live_updates(window, cx, live_panel, live_runtime, showcase.clone());

            cx.new(|cx| Root::new(showcase, window, cx))
        })
        .unwrap();
    });
}
