//! GPUI integration for gpui_plot.
//!
//! This module provides a GPUI view that renders a [`Plot`] and handles
//! mouse interactions (pan, zoom, box zoom, and pinning).

#![allow(clippy::collapsible_if)]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    App, BorderStyle, Bounds, ContentMask, Corners, Edges, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Point, ScrollWheelEvent, TextRun, Window,
    canvas, div, font, point, px, quad,
};

use crate::axis::{AxisConfig, AxisLayout, AxisLayoutCache, TextMeasurer};
use crate::datasource::DecimationScratch;
use crate::geom::{Point as DataPoint, ScreenPoint, ScreenRect};
use crate::interaction::{
    HitRegion, PlotRegions, pan_viewport, toggle_pin, zoom_factor_from_drag, zoom_to_rect,
    zoom_viewport,
};
use crate::plot::Plot;
use crate::render::{
    Color, LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCacheKey,
    RenderCommand, RenderList, TextStyle, build_line_segments, build_scatter_points,
};
use crate::series::{Series, SeriesId, SeriesKind};
use crate::style::Theme;
use crate::transform::Transform;
use crate::view::{Range, Viewport};

const AXIS_PADDING: f32 = 6.0;
const TICK_LENGTH_MAJOR: f32 = 6.0;
const TICK_LENGTH_MINOR: f32 = 3.0;
const DOUBLE_CLICK_PIN_GRACE_MS: u64 = 1200;
const PIN_RING_INNER_PAD: f32 = 4.0;
const PIN_RING_OUTER_PAD: f32 = 8.0;
const PIN_UNPIN_HIGHLIGHT: Color = Color::new(0.95, 0.25, 0.25, 1.0);
const PIN_LABEL_OFFSET: f32 = 10.0;
const MAX_PIN_LABELS: usize = 12;
const MAX_PIN_LABEL_COVERAGE: f32 = 0.35;
const PIN_CLUSTER_RADIUS: f32 = 40.0;
const LEGEND_FONT_SIZE: f32 = 12.0;
const LEGEND_LINE_HEIGHT: f32 = 16.0;
const LEGEND_PADDING: f32 = 6.0;
const LEGEND_TOGGLE_DIAMETER: f32 = 12.0;
const LEGEND_TOGGLE_INNER_DIAMETER: f32 = 8.0;
const LEGEND_TOGGLE_GAP: f32 = 6.0;
const LEGEND_SWATCH_WIDTH: f32 = 16.0;
const LEGEND_SWATCH_GAP: f32 = 6.0;
const LEGEND_HIDDEN_ALPHA: f32 = 0.35;
const LEGEND_TEXT_HIDDEN_ALPHA: f32 = 0.45;

/// Configuration for the GPUI plot view.
#[derive(Debug, Clone)]
pub struct PlotViewConfig {
    /// Pixel threshold for starting a drag.
    pub drag_threshold_px: f32,
    /// Pixel threshold for pin hit testing.
    pub pin_threshold_px: f32,
    /// Pixel threshold for unpin hit testing.
    pub unpin_threshold_px: f32,
    /// Padding fraction applied when auto-fitting data.
    pub padding_frac: f64,
    /// Minimum padding applied when auto-fitting data.
    pub min_padding: f64,
    /// Show legend overlay.
    pub show_legend: bool,
    /// Show hover coordinate readout.
    pub show_hover: bool,
}

impl Default for PlotViewConfig {
    fn default() -> Self {
        Self {
            drag_threshold_px: 4.0,
            pin_threshold_px: 12.0,
            unpin_threshold_px: 18.0,
            padding_frac: 0.05,
            min_padding: 1e-6,
            show_legend: true,
            show_hover: true,
        }
    }
}

/// A GPUI view that renders a [`Plot`] with interactive controls.
#[derive(Clone)]
pub struct GpuiPlotView {
    plot: Arc<RwLock<Plot>>,
    state: Arc<RwLock<PlotUiState>>,
    config: PlotViewConfig,
}

impl GpuiPlotView {
    /// Create a new GPUI plot view for the given plot.
    pub fn new(plot: Plot) -> Self {
        Self {
            plot: Arc::new(RwLock::new(plot)),
            state: Arc::new(RwLock::new(PlotUiState::default())),
            config: PlotViewConfig::default(),
        }
    }

    /// Create a new GPUI plot view with a custom configuration.
    pub fn with_config(plot: Plot, config: PlotViewConfig) -> Self {
        Self {
            plot: Arc::new(RwLock::new(plot)),
            state: Arc::new(RwLock::new(PlotUiState::default())),
            config,
        }
    }

    /// Get a handle for mutating the underlying plot.
    pub fn plot_handle(&self) -> PlotHandle {
        PlotHandle {
            plot: Arc::clone(&self.plot),
        }
    }

    fn on_mouse_down(&mut self, ev: &MouseDownEvent, cx: &mut Context<Self>) {
        let pos = screen_point(ev.position);
        let mut state = self.state.write().expect("plot state lock");
        state.last_cursor = Some(pos);

        if let Some(series_id) = state.legend_hit(pos) {
            if ev.button == MouseButton::Left && ev.click_count == 1 {
                if let Ok(mut plot) = self.plot.write() {
                    if let Some(series) = plot
                        .series_mut()
                        .iter_mut()
                        .find(|series| series.id() == series_id)
                    {
                        series.set_visible(!series.is_visible());
                    }
                }
            }
            state.clear_interaction();
            state.hover = None;
            state.hover_target = None;
            cx.notify();
            return;
        }

        let region = state.regions.hit_test(pos);
        if ev.button == MouseButton::Left && ev.click_count >= 2 && region == HitRegion::Plot {
            let last_toggle = state.last_pin_toggle.take();
            if let Ok(mut plot) = self.plot.write() {
                if let Some(last_toggle) = last_toggle {
                    if last_toggle.at.elapsed() <= Duration::from_millis(DOUBLE_CLICK_PIN_GRACE_MS)
                        && distance_sq(last_toggle.screen_pos, pos)
                            <= self.config.pin_threshold_px.powi(2)
                    {
                        revert_pin_toggle(&mut plot, last_toggle);
                    }
                }
                plot.reset_view();
            }
            state.clear_interaction();
            cx.notify();
            return;
        }

        state.pending_click = Some(ClickState {
            region,
            button: ev.button,
        });

        match (ev.button, region) {
            (MouseButton::Left, HitRegion::XAxis) => {
                state.drag = Some(DragState::new(DragMode::ZoomX, pos, true));
            }
            (MouseButton::Left, HitRegion::YAxis) => {
                state.drag = Some(DragState::new(DragMode::ZoomY, pos, true));
            }
            (MouseButton::Left, HitRegion::Plot) => {
                state.drag = Some(DragState::new(DragMode::Pan, pos, false));
            }
            (MouseButton::Right, HitRegion::Plot) => {
                state.drag = Some(DragState::new(DragMode::ZoomRect, pos, true));
                state.selection_rect = Some(ScreenRect::new(pos, pos));
            }
            _ => {}
        }

        cx.notify();
    }

    fn on_mouse_move(&mut self, ev: &MouseMoveEvent, cx: &mut Context<Self>) {
        let pos = screen_point(ev.position);
        let mut state = self.state.write().expect("plot state lock");
        state.last_cursor = Some(pos);

        if state.legend_hit(pos).is_some() {
            state.hover = None;
        } else if state.regions.hit_test(pos) == HitRegion::Plot {
            state.hover = Some(pos);
        } else {
            state.hover = None;
        }

        let Some(mut drag) = state.drag.clone() else {
            cx.notify();
            return;
        };

        let moved_sq = distance_sq(drag.start, pos);
        if !drag.active && moved_sq > self.config.drag_threshold_px.powi(2) {
            drag.active = true;
        }

        if !drag.active {
            state.drag = Some(drag);
            cx.notify();
            return;
        }

        let delta = ScreenPoint::new(pos.x - drag.last.x, pos.y - drag.last.y);
        let plot_rect = state.plot_rect;
        let transform = state.transform.clone();

        match drag.mode {
            DragMode::Pan => {
                if let (Some(rect), Some(transform)) = (plot_rect, transform) {
                    if let Ok(mut plot) = self.plot.write() {
                        if let Some(viewport) = plot.viewport() {
                            if let Some(next) = pan_viewport(viewport, delta, &transform) {
                                apply_manual_view(&mut plot, &mut state, rect, next);
                            }
                        }
                    }
                }
            }
            DragMode::ZoomRect => {
                state.selection_rect = Some(ScreenRect::new(drag.start, pos));
            }
            DragMode::ZoomX => {
                if let (Some(rect), Some(transform)) = (plot_rect, transform) {
                    let axis_pixels = rect.width().max(1.0);
                    let factor = zoom_factor_from_drag(delta.x, axis_pixels);
                    if let Ok(mut plot) = self.plot.write() {
                        if let Some(viewport) = plot.viewport() {
                            let center = transform
                                .screen_to_data(pos)
                                .unwrap_or_else(|| viewport.x_center());
                            let next = zoom_viewport(viewport, center, factor, 1.0);
                            apply_manual_view(&mut plot, &mut state, rect, next);
                        }
                    }
                }
            }
            DragMode::ZoomY => {
                if let (Some(rect), Some(transform)) = (plot_rect, transform) {
                    let axis_pixels = rect.height().max(1.0);
                    let factor = zoom_factor_from_drag(-delta.y, axis_pixels);
                    if let Ok(mut plot) = self.plot.write() {
                        if let Some(viewport) = plot.viewport() {
                            let center = transform
                                .screen_to_data(pos)
                                .unwrap_or_else(|| viewport.y_center());
                            let next = zoom_viewport(viewport, center, 1.0, factor);
                            apply_manual_view(&mut plot, &mut state, rect, next);
                        }
                    }
                }
            }
        }

        drag.last = pos;
        state.drag = Some(drag);
        state.pending_click = None;
        cx.notify();
    }

    fn on_mouse_up(&mut self, ev: &MouseUpEvent, cx: &mut Context<Self>) {
        let pos = screen_point(ev.position);
        let mut state = self.state.write().expect("plot state lock");
        let drag = state.drag.clone();

        if let Some(drag_state) = drag.as_ref() {
            if drag_state.active && drag_state.mode == DragMode::ZoomRect {
                if let (Some(rect), Some(transform)) =
                    (state.selection_rect.take(), state.transform.clone())
                {
                    let rect = normalized_rect(rect);
                    if let Ok(mut plot) = self.plot.write() {
                        if let Some(viewport) = plot.viewport() {
                            if let Some(next) = zoom_to_rect(viewport, rect, &transform) {
                                apply_manual_view(&mut plot, &mut state, transform.screen(), next);
                            }
                        }
                    }
                }
            }
        }

        let click = state.pending_click.take();
        let should_toggle = click.as_ref().is_some_and(|click| {
            click.button == MouseButton::Left && click.region == HitRegion::Plot
        }) && drag.as_ref().is_none_or(|drag| !drag.active)
            && ev.click_count == 1;

        if should_toggle {
            if let Some(transform) = state.transform.clone() {
                if let Ok(mut plot) = self.plot.write() {
                    let target = state
                        .hover_target
                        .filter(|target| hover_target_within_threshold(target, pos, &self.config))
                        .or_else(|| {
                            compute_hover_target(
                                &plot,
                                &transform,
                                pos,
                                state.plot_rect,
                                self.config.pin_threshold_px,
                                self.config.unpin_threshold_px,
                            )
                        });

                    if let Some(target) = target {
                        let added = toggle_pin(plot.pins_mut(), target.pin);
                        let now = Instant::now();
                        state.last_pin_toggle = Some(PinToggle {
                            pin: target.pin,
                            added,
                            at: now,
                            screen_pos: target.screen,
                        });
                    }
                }
            }
        } else if ev.click_count > 1 {
            state.last_pin_toggle = None;
        }

        state.drag = None;
        state.selection_rect = None;
        cx.notify();
    }

    fn on_scroll(&mut self, ev: &ScrollWheelEvent, _window: &Window, cx: &mut Context<Self>) {
        let pos = screen_point(ev.position);
        let mut state = self.state.write().expect("plot state lock");
        if state.legend_hit(pos).is_some() {
            return;
        }
        let region = state.regions.hit_test(pos);
        let Some(transform) = state.transform.clone() else {
            return;
        };

        let line_height = px(16.0);
        let delta = ev.delta.pixel_delta(line_height);
        let zoom_delta = -f32::from(delta.y);
        if zoom_delta.abs() < 0.01 {
            return;
        }
        let factor = (1.0 - (zoom_delta as f64 * 0.002)).clamp(0.1, 10.0);

        if let Ok(mut plot) = self.plot.write() {
            if let Some(viewport) = plot.viewport() {
                let center = transform
                    .screen_to_data(pos)
                    .unwrap_or_else(|| viewport.center());
                let (factor_x, factor_y) = match region {
                    HitRegion::XAxis => (factor, 1.0),
                    HitRegion::YAxis => (1.0, factor),
                    HitRegion::Plot => (factor, factor),
                    HitRegion::Outside => (1.0, 1.0),
                };
                if factor_x != 1.0 || factor_y != 1.0 {
                    let next = zoom_viewport(viewport, center, factor_x, factor_y);
                    if let Some(rect) = state.plot_rect {
                        apply_manual_view(&mut plot, &mut state, rect, next);
                    }
                }
            }
        }

        cx.notify();
    }
}

impl Render for GpuiPlotView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let plot = Arc::clone(&self.plot);
        let state = Arc::clone(&self.state);
        let config = self.config.clone();
        let theme = plot.read().expect("plot lock").theme().clone();

        div()
            .size_full()
            .bg(to_hsla(theme.background))
            .child(
                canvas(
                    move |bounds, window, _| {
                        let mut plot = plot.write().expect("plot lock");
                        let mut state = state.write().expect("plot state lock");
                        build_frame(&mut plot, &mut state, &config, bounds, window)
                    },
                    move |_, frame, window, cx| {
                        paint_frame(&frame, window, cx);
                    },
                )
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev, _, cx| {
                    this.on_mouse_down(ev, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, ev, _, cx| {
                    this.on_mouse_down(ev, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, ev, _, cx| {
                this.on_mouse_move(ev, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev, _, cx| {
                    this.on_mouse_up(ev, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, ev, _, cx| {
                    this.on_mouse_up(ev, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev, window, cx| {
                this.on_scroll(ev, window, cx);
            }))
    }
}

/// A handle for mutating a [`Plot`] held inside a `GpuiPlotView`.
#[derive(Clone)]
pub struct PlotHandle {
    plot: Arc<RwLock<Plot>>,
}

impl PlotHandle {
    /// Read the plot state.
    pub fn read<R>(&self, f: impl FnOnce(&Plot) -> R) -> R {
        let plot = self.plot.read().expect("plot lock");
        f(&plot)
    }

    /// Mutate the plot state.
    pub fn write<R>(&self, f: impl FnOnce(&mut Plot) -> R) -> R {
        let mut plot = self.plot.write().expect("plot lock");
        f(&mut plot)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragMode {
    Pan,
    ZoomRect,
    ZoomX,
    ZoomY,
}

#[derive(Debug, Clone)]
struct DragState {
    mode: DragMode,
    start: ScreenPoint,
    last: ScreenPoint,
    active: bool,
}

impl DragState {
    fn new(mode: DragMode, start: ScreenPoint, active: bool) -> Self {
        Self {
            mode,
            start,
            last: start,
            active,
        }
    }
}

#[derive(Debug, Clone)]
struct ClickState {
    region: HitRegion,
    button: MouseButton,
}

#[derive(Debug, Clone, Copy)]
struct PinToggle {
    pin: crate::interaction::Pin,
    added: bool,
    at: Instant,
    screen_pos: ScreenPoint,
}

#[derive(Debug, Clone, Copy)]
struct HoverTarget {
    pin: crate::interaction::Pin,
    screen: ScreenPoint,
    is_pinned: bool,
}

#[derive(Debug, Clone)]
struct PinLabel {
    screen: ScreenPoint,
    label: String,
    size: (f32, f32),
}

#[derive(Debug, Clone, Default)]
struct SeriesCache {
    key: Option<RenderCacheKey>,
    points: Vec<DataPoint>,
}

#[derive(Debug, Clone)]
struct LegendEntry {
    series_id: SeriesId,
    row_rect: ScreenRect,
}

#[derive(Debug, Clone)]
struct LegendLayout {
    rect: ScreenRect,
    entries: Vec<LegendEntry>,
}

#[derive(Debug, Clone)]
struct PlotUiState {
    x_layout: AxisLayoutCache,
    y_layout: AxisLayoutCache,
    regions: PlotRegions,
    plot_rect: Option<ScreenRect>,
    transform: Option<Transform>,
    viewport: Option<Viewport>,
    drag: Option<DragState>,
    pending_click: Option<ClickState>,
    last_pin_toggle: Option<PinToggle>,
    hover_target: Option<HoverTarget>,
    selection_rect: Option<ScreenRect>,
    hover: Option<ScreenPoint>,
    last_cursor: Option<ScreenPoint>,
    decimation_scratch: DecimationScratch,
    series_cache: HashMap<SeriesId, SeriesCache>,
    legend_layout: Option<LegendLayout>,
}

impl Default for PlotUiState {
    fn default() -> Self {
        Self {
            x_layout: AxisLayoutCache::default(),
            y_layout: AxisLayoutCache::default(),
            regions: PlotRegions {
                plot: ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(0.0, 0.0)),
                x_axis: ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(0.0, 0.0)),
                y_axis: ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(0.0, 0.0)),
            },
            plot_rect: None,
            transform: None,
            viewport: None,
            drag: None,
            pending_click: None,
            last_pin_toggle: None,
            hover_target: None,
            selection_rect: None,
            hover: None,
            last_cursor: None,
            decimation_scratch: DecimationScratch::new(),
            series_cache: HashMap::new(),
            legend_layout: None,
        }
    }
}

impl PlotUiState {
    fn clear_interaction(&mut self) {
        self.drag = None;
        self.pending_click = None;
        self.selection_rect = None;
    }

    fn legend_hit(&self, point: ScreenPoint) -> Option<SeriesId> {
        let layout = self.legend_layout.as_ref()?;
        if !rect_contains(layout.rect, point) {
            return None;
        }
        for entry in &layout.entries {
            if rect_contains(entry.row_rect, point) {
                return Some(entry.series_id);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
struct PlotFrame {
    render: RenderList,
}

fn build_frame(
    plot: &mut Plot,
    state: &mut PlotUiState,
    config: &PlotViewConfig,
    bounds: Bounds<Pixels>,
    window: &Window,
) -> PlotFrame {
    let mut render = RenderList::new();

    let full_width = f32::from(bounds.size.width);
    let full_height = f32::from(bounds.size.height);
    if full_width <= 1.0 || full_height <= 1.0 {
        return PlotFrame { render };
    }

    let viewport = plot
        .refresh_viewport(config.padding_frac, config.min_padding)
        .unwrap_or_else(|| Viewport::new(Range::new(0.0, 1.0), Range::new(0.0, 1.0)));

    state.viewport = Some(viewport);

    let measurer = GpuiTextMeasurer::new(window);

    let mut plot_width = full_width;
    let mut plot_height = full_height;

    let x_layout = state
        .x_layout
        .update(plot.x_axis(), viewport.x, plot_width as u32, &measurer)
        .clone();
    let y_layout = state
        .y_layout
        .update(plot.y_axis(), viewport.y, plot_height as u32, &measurer)
        .clone();

    let x_title = axis_title_text(plot.x_axis());
    let x_title_size = x_title
        .as_ref()
        .map(|title| measurer.measure(title, plot.x_axis().label_size()))
        .unwrap_or((0.0, 0.0));

    let x_axis_height =
        x_layout.max_label_size.1 + TICK_LENGTH_MAJOR + AXIS_PADDING * 2.0 + x_title_size.1;
    let y_axis_width = y_layout.max_label_size.0 + TICK_LENGTH_MAJOR + AXIS_PADDING * 2.0;

    let x_axis_height = x_axis_height.clamp(0.0, full_height - 1.0);
    let y_axis_width = y_axis_width.clamp(0.0, full_width - 1.0);

    plot_width = (full_width - y_axis_width).max(1.0);
    plot_height = (full_height - x_axis_height).max(1.0);

    let x_layout = state
        .x_layout
        .update(plot.x_axis(), viewport.x, plot_width as u32, &measurer)
        .clone();
    let y_layout = state
        .y_layout
        .update(plot.y_axis(), viewport.y, plot_height as u32, &measurer)
        .clone();

    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let full_max_x = origin_x + full_width;
    let full_max_y = origin_y + full_height;

    let plot_rect = ScreenRect::new(
        ScreenPoint::new(origin_x + y_axis_width, origin_y),
        ScreenPoint::new(full_max_x, full_max_y - x_axis_height),
    );
    let x_axis_rect = ScreenRect::new(
        ScreenPoint::new(plot_rect.min.x, plot_rect.max.y),
        ScreenPoint::new(plot_rect.max.x, full_max_y),
    );
    let y_axis_rect = ScreenRect::new(
        ScreenPoint::new(origin_x, plot_rect.min.y),
        ScreenPoint::new(plot_rect.min.x, plot_rect.max.y),
    );

    state.regions = PlotRegions {
        plot: plot_rect,
        x_axis: x_axis_rect,
        y_axis: y_axis_rect,
    };
    state.plot_rect = Some(plot_rect);

    let transform = Transform::new(
        viewport,
        plot_rect,
        plot.x_axis().scale(),
        plot.y_axis().scale(),
    );
    state.transform = transform.clone();

    if let Some(transform) = transform {
        build_grid(
            &mut render,
            plot,
            &x_layout,
            &y_layout,
            &transform,
            plot_rect,
        );
        build_series(&mut render, plot, state, &transform, plot_rect);
        build_selection(&mut render, plot, state);
        update_hover_target(
            plot,
            state,
            &transform,
            plot_rect,
            config.pin_threshold_px,
            config.unpin_threshold_px,
        );
        build_pins(&mut render, plot, &transform, plot_rect, &measurer);
        build_axes(
            &mut render,
            plot,
            &x_layout,
            &y_layout,
            plot_rect,
            &transform,
            x_axis_rect,
            y_axis_rect,
            &measurer,
        );
        if config.show_hover {
            build_hover(&mut render, plot, state, &transform, plot_rect, &measurer);
        }
        if config.show_legend {
            build_legend(&mut render, plot, state, plot_rect, &measurer);
        } else {
            state.legend_layout = None;
        }
        build_axis_titles(
            &mut render,
            plot,
            plot_rect,
            x_axis_rect,
            y_axis_rect,
            &measurer,
        );
    } else {
        state.legend_layout = None;
        let message = "Invalid axis range";
        let size = measurer.measure(message, 14.0);
        let pos = ScreenPoint::new(
            plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
            plot_rect.min.y + (plot_rect.height() - size.1) * 0.5,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: message.to_string(),
            style: TextStyle {
                color: plot.theme().axis,
                size: 14.0,
            },
        });
    }

    PlotFrame { render }
}

fn build_grid(
    render: &mut RenderList,
    plot: &Plot,
    x_layout: &AxisLayout,
    y_layout: &AxisLayout,
    transform: &Transform,
    plot_rect: ScreenRect,
) {
    let theme = plot.theme();
    let mut major = Vec::new();
    let mut minor = Vec::new();

    if plot.x_axis().show_grid() {
        for tick in &x_layout.ticks {
            let x = transform
                .data_to_screen(DataPoint::new(tick.value, transform.viewport().y.min))
                .map(|p| p.x);
            let Some(x) = x else { continue };
            let segment = LineSegment::new(
                ScreenPoint::new(x, plot_rect.min.y),
                ScreenPoint::new(x, plot_rect.max.y),
            );
            if tick.is_major {
                major.push(segment);
            } else if plot.x_axis().show_minor_grid() {
                minor.push(segment);
            }
        }
    }

    if plot.y_axis().show_grid() {
        for tick in &y_layout.ticks {
            let y = transform
                .data_to_screen(DataPoint::new(transform.viewport().x.min, tick.value))
                .map(|p| p.y);
            let Some(y) = y else { continue };
            let segment = LineSegment::new(
                ScreenPoint::new(plot_rect.min.x, y),
                ScreenPoint::new(plot_rect.max.x, y),
            );
            if tick.is_major {
                major.push(segment);
            } else if plot.y_axis().show_minor_grid() {
                minor.push(segment);
            }
        }
    }

    render.push(RenderCommand::ClipRect(plot_rect));
    if !minor.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: minor,
            style: LineStyle {
                color: theme.grid_minor,
                width: 1.0,
            },
        });
    }
    if !major.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: major,
            style: LineStyle {
                color: theme.grid_major,
                width: 1.0,
            },
        });
    }

    if plot.x_axis().show_zero_line() {
        if transform.viewport().y.min <= 0.0 && transform.viewport().y.max >= 0.0 {
            if let Some(y) = transform
                .data_to_screen(DataPoint::new(transform.viewport().x.min, 0.0))
                .map(|p| p.y)
            {
                render.push(RenderCommand::LineSegments {
                    segments: vec![LineSegment::new(
                        ScreenPoint::new(plot_rect.min.x, y),
                        ScreenPoint::new(plot_rect.max.x, y),
                    )],
                    style: LineStyle {
                        color: theme.axis,
                        width: 1.0,
                    },
                });
            }
        }
    }

    if plot.y_axis().show_zero_line() {
        if transform.viewport().x.min <= 0.0 && transform.viewport().x.max >= 0.0 {
            if let Some(x) = transform
                .data_to_screen(DataPoint::new(0.0, transform.viewport().y.min))
                .map(|p| p.x)
            {
                render.push(RenderCommand::LineSegments {
                    segments: vec![LineSegment::new(
                        ScreenPoint::new(x, plot_rect.min.y),
                        ScreenPoint::new(x, plot_rect.max.y),
                    )],
                    style: LineStyle {
                        color: theme.axis,
                        width: 1.0,
                    },
                });
            }
        }
    }

    render.push(RenderCommand::ClipEnd);
}

fn build_series(
    render: &mut RenderList,
    plot: &Plot,
    state: &mut PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
) {
    let plot_width = plot_rect.width().max(1.0) as usize;
    let size = (
        plot_rect.width().round() as u32,
        plot_rect.height().round() as u32,
    );

    render.push(RenderCommand::ClipRect(plot_rect));

    for series in plot.series() {
        if !series.is_visible() {
            continue;
        }
        let cache = state.series_cache.entry(series.id()).or_default();
        let key = RenderCacheKey {
            viewport: transform.viewport(),
            size,
            x_scale: plot.x_axis().scale(),
            y_scale: plot.y_axis().scale(),
            generation: series.generation(),
        };
        if cache.key.as_ref() != Some(&key) {
            let decimated = series.data().decimate(
                transform.viewport().x,
                plot_width,
                &mut state.decimation_scratch,
            );
            cache.points.clear();
            cache.points.extend_from_slice(decimated);
            cache.key = Some(key.clone());
        }

        match series.kind() {
            SeriesKind::Line(style) => {
                let mut segments = Vec::new();
                build_line_segments(&cache.points, transform, plot_rect, &mut segments);
                if !segments.is_empty() {
                    render.push(RenderCommand::LineSegments {
                        segments,
                        style: *style,
                    });
                }
            }
            SeriesKind::Scatter(style) => {
                let mut points = Vec::new();
                build_scatter_points(&cache.points, transform, plot_rect, &mut points);
                if !points.is_empty() {
                    render.push(RenderCommand::Points {
                        points,
                        style: *style,
                    });
                }
            }
        }
    }

    render.push(RenderCommand::ClipEnd);
}

fn build_selection(render: &mut RenderList, plot: &Plot, state: &PlotUiState) {
    if let Some(rect) = state.selection_rect {
        let rect = normalized_rect(rect);
        render.push(RenderCommand::Rect {
            rect,
            style: RectStyle {
                fill: plot.theme().selection_fill,
                stroke: plot.theme().selection_border,
                stroke_width: 1.0,
            },
        });
    }
}

fn build_pins(
    render: &mut RenderList,
    plot: &Plot,
    transform: &Transform,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    if plot.pins().is_empty() {
        return;
    }

    let theme = plot.theme();
    let font_size = 12.0;
    let line_height = 14.0;
    let mut labels: Vec<PinLabel> = Vec::new();
    render.push(RenderCommand::ClipRect(plot_rect));

    for pin in plot.pins() {
        let Some(series) = plot
            .series()
            .iter()
            .find(|series| series.id() == pin.series_id)
        else {
            continue;
        };
        if !series.is_visible() {
            continue;
        }
        let Some(point) = series.data().data().point(pin.point_index) else {
            continue;
        };
        let Some(screen) = transform.data_to_screen(point) else {
            continue;
        };

        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            continue;
        }

        let (marker_style, base_size) = marker_style_and_size(series);

        let ring_outer = base_size + PIN_RING_OUTER_PAD;
        let ring_inner = base_size + PIN_RING_INNER_PAD;
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.axis,
                size: ring_outer,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.background,
                size: ring_inner,
                shape: MarkerShape::Circle,
            },
        });

        render.push(RenderCommand::Points {
            points: vec![screen],
            style: marker_style,
        });

        let x_text = plot.x_axis().format_value(point.x);
        let y_text = plot.y_axis().format_value(point.y);
        let label = format!("{}\nx: {x_text}\ny: {y_text}", series.name());
        let size = measurer.measure_multiline(&label, font_size);
        labels.push(PinLabel {
            screen,
            label,
            size,
        });
    }

    if labels.is_empty() {
        render.push(RenderCommand::ClipEnd);
        return;
    }

    let plot_area = plot_rect.width().max(1.0) * plot_rect.height().max(1.0);
    let total_label_area: f32 = labels.iter().map(|label| label.size.0 * label.size.1).sum();
    let dense =
        labels.len() > MAX_PIN_LABELS || total_label_area > plot_area * MAX_PIN_LABEL_COVERAGE;

    let mut clusters = cluster_pin_labels(&labels, PIN_CLUSTER_RADIUS);
    clusters.sort_by(|a, b| {
        let size_cmp = b.len().cmp(&a.len());
        if size_cmp != std::cmp::Ordering::Equal {
            return size_cmp;
        }
        let min_a = a.iter().copied().min().unwrap_or(0);
        let min_b = b.iter().copied().min().unwrap_or(0);
        min_a.cmp(&min_b)
    });

    let mut placed: Vec<ScreenRect> = Vec::new();
    let mut single_budget = if dense { MAX_PIN_LABELS } else { usize::MAX };
    for cluster in clusters {
        if cluster.len() >= 2 {
            if !dense {
                let mut local_placed = placed.clone();
                let mut placements: Vec<(ScreenPoint, ScreenRect, usize)> = Vec::new();
                let mut success = true;
                for index in &cluster {
                    let entry = &labels[*index];
                    if let Some((origin, rect)) = place_label(
                        entry.screen,
                        entry.size,
                        plot_rect,
                        PIN_LABEL_OFFSET,
                        &local_placed,
                    ) {
                        local_placed.push(rect);
                        placements.push((origin, rect, *index));
                    } else {
                        success = false;
                        break;
                    }
                }

                if success {
                    placed = local_placed;
                    for (origin, rect, index) in placements {
                        let entry = &labels[index];
                        push_label_with_leader(
                            render,
                            rect,
                            origin,
                            entry.screen,
                            &entry.label,
                            font_size,
                            line_height,
                            theme,
                        );
                    }
                    continue;
                }
            }

            let center = cluster_center(&labels, &cluster);
            let label = format!("{} pins", cluster.len());
            let size = measurer.measure_multiline(&label, font_size);
            if let Some((origin, rect)) =
                place_label(center, size, plot_rect, PIN_LABEL_OFFSET, &placed)
            {
                placed.push(rect);
                push_label_with_leader(
                    render,
                    rect,
                    origin,
                    center,
                    &label,
                    font_size,
                    line_height,
                    theme,
                );
            }
            continue;
        }

        if single_budget == 0 {
            continue;
        }
        let index = cluster[0];
        let entry = &labels[index];
        if let Some((origin, rect)) = place_label(
            entry.screen,
            entry.size,
            plot_rect,
            PIN_LABEL_OFFSET,
            &placed,
        ) {
            placed.push(rect);
            push_label_with_leader(
                render,
                rect,
                origin,
                entry.screen,
                &entry.label,
                font_size,
                line_height,
                theme,
            );
            single_budget = single_budget.saturating_sub(1);
        }
    }

    render.push(RenderCommand::ClipEnd);
}

#[allow(clippy::too_many_arguments)]
fn build_axes(
    render: &mut RenderList,
    plot: &Plot,
    x_layout: &AxisLayout,
    y_layout: &AxisLayout,
    plot_rect: ScreenRect,
    transform: &Transform,
    _x_axis_rect: ScreenRect,
    _y_axis_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let mut ticks_major = Vec::new();
    let mut ticks_minor = Vec::new();

    if plot.x_axis().show_border() {
        render.push(RenderCommand::Rect {
            rect: plot_rect,
            style: RectStyle {
                fill: Color::new(0.0, 0.0, 0.0, 0.0),
                stroke: theme.axis,
                stroke_width: 1.0,
            },
        });
    }

    for tick in &x_layout.ticks {
        if let Some(x) = transform
            .data_to_screen(DataPoint::new(tick.value, transform.viewport().y.min))
            .map(|p| p.x)
        {
            let length = if tick.is_major {
                TICK_LENGTH_MAJOR
            } else {
                TICK_LENGTH_MINOR
            };
            let segment = LineSegment::new(
                ScreenPoint::new(x, plot_rect.max.y),
                ScreenPoint::new(x, plot_rect.max.y + length),
            );
            if tick.is_major {
                ticks_major.push(segment);
            } else if plot.x_axis().show_minor_grid() {
                ticks_minor.push(segment);
            }

            if tick.is_major && !tick.label.is_empty() {
                let size = measurer.measure(&tick.label, plot.x_axis().label_size());
                let pos = ScreenPoint::new(
                    x - size.0 * 0.5,
                    plot_rect.max.y + TICK_LENGTH_MAJOR + AXIS_PADDING,
                );
                render.push(RenderCommand::Text {
                    position: pos,
                    text: tick.label.clone(),
                    style: TextStyle {
                        color: theme.axis,
                        size: plot.x_axis().label_size(),
                    },
                });
            }
        }
    }

    for tick in &y_layout.ticks {
        if let Some(y) = transform
            .data_to_screen(DataPoint::new(transform.viewport().x.min, tick.value))
            .map(|p| p.y)
        {
            let length = if tick.is_major {
                TICK_LENGTH_MAJOR
            } else {
                TICK_LENGTH_MINOR
            };
            let segment = LineSegment::new(
                ScreenPoint::new(plot_rect.min.x - length, y),
                ScreenPoint::new(plot_rect.min.x, y),
            );
            if tick.is_major {
                ticks_major.push(segment);
            } else if plot.y_axis().show_minor_grid() {
                ticks_minor.push(segment);
            }

            if tick.is_major && !tick.label.is_empty() {
                let size = measurer.measure(&tick.label, plot.y_axis().label_size());
                let pos = ScreenPoint::new(
                    plot_rect.min.x - TICK_LENGTH_MAJOR - AXIS_PADDING - size.0,
                    y - size.1 * 0.5,
                );
                render.push(RenderCommand::Text {
                    position: pos,
                    text: tick.label.clone(),
                    style: TextStyle {
                        color: theme.axis,
                        size: plot.y_axis().label_size(),
                    },
                });
            }
        }
    }

    if !ticks_minor.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: ticks_minor,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
    if !ticks_major.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: ticks_major,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
}

fn build_axis_titles(
    render: &mut RenderList,
    plot: &Plot,
    plot_rect: ScreenRect,
    x_axis_rect: ScreenRect,
    y_axis_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    if let Some(title) = axis_title_text(plot.x_axis()) {
        let size = measurer.measure(&title, plot.x_axis().label_size());
        let pos = ScreenPoint::new(
            plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
            x_axis_rect.max.y - size.1 - AXIS_PADDING,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: title,
            style: TextStyle {
                color: theme.axis,
                size: plot.x_axis().label_size(),
            },
        });
    }

    if let Some(title) = axis_title_text(plot.y_axis()) {
        let pos = ScreenPoint::new(
            y_axis_rect.min.x + AXIS_PADDING,
            y_axis_rect.min.y + AXIS_PADDING,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: title,
            style: TextStyle {
                color: theme.axis,
                size: plot.y_axis().label_size(),
            },
        });
    }
}

fn build_hover(
    render: &mut RenderList,
    plot: &Plot,
    state: &PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let Some(cursor) = state.hover else { return };
    if cursor.x < plot_rect.min.x
        || cursor.x > plot_rect.max.x
        || cursor.y < plot_rect.min.y
        || cursor.y > plot_rect.max.y
    {
        return;
    }

    if let Some(target) = state.hover_target {
        let Some(series) = plot
            .series()
            .iter()
            .find(|series| series.id() == target.pin.series_id)
        else {
            return;
        };
        let Some(point) = series.data().data().point(target.pin.point_index) else {
            return;
        };
        let screen = target.screen;
        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            return;
        }

        if target.is_pinned {
            let (_, base_size) = marker_style_and_size(series);
            let ring_outer = base_size + PIN_RING_OUTER_PAD;
            let ring_inner = base_size + PIN_RING_INNER_PAD;
            render.push(RenderCommand::Points {
                points: vec![screen],
                style: MarkerStyle {
                    color: PIN_UNPIN_HIGHLIGHT,
                    size: ring_outer,
                    shape: MarkerShape::Circle,
                },
            });
            render.push(RenderCommand::Points {
                points: vec![screen],
                style: MarkerStyle {
                    color: theme.background,
                    size: ring_inner,
                    shape: MarkerShape::Circle,
                },
            });
            return;
        }

        let (marker_style, base_size) = marker_style_and_size(series);
        let ring_outer = base_size + PIN_RING_OUTER_PAD;
        let ring_inner = base_size + PIN_RING_INNER_PAD;
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.axis,
                size: ring_outer,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.background,
                size: ring_inner,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: marker_style,
        });

        let x_text = plot.x_axis().format_value(point.x);
        let y_text = plot.y_axis().format_value(point.y);
        let label = format!("{}\nx: {x_text}\ny: {y_text}", series.name());
        let size = measurer.measure_multiline(&label, 12.0);
        let mut origin = ScreenPoint::new(screen.x + 12.0, screen.y + 12.0);
        if origin.x + size.0 > plot_rect.max.x {
            origin.x = screen.x - size.0 - 12.0;
        }
        if origin.y + size.1 > plot_rect.max.y {
            origin.y = screen.y - size.1 - 12.0;
        }
        origin = clamp_point(origin, plot_rect, size);

        render.push(RenderCommand::Rect {
            rect: ScreenRect::new(
                origin,
                ScreenPoint::new(origin.x + size.0, origin.y + size.1),
            ),
            style: RectStyle {
                fill: theme.pin_bg,
                stroke: theme.pin_border,
                stroke_width: 1.0,
            },
        });

        for (index, line) in label.lines().enumerate() {
            let line_y = origin.y + index as f32 * 14.0 + 2.0;
            render.push(RenderCommand::Text {
                position: ScreenPoint::new(origin.x + 4.0, line_y),
                text: line.to_string(),
                style: TextStyle {
                    color: theme.axis,
                    size: 12.0,
                },
            });
        }
        return;
    }

    let Some(data) = transform.screen_to_data(cursor) else {
        return;
    };
    let x_text = plot.x_axis().format_value(data.x);
    let y_text = plot.y_axis().format_value(data.y);
    let label = format!("x: {x_text}\ny: {y_text}");

    let size = measurer.measure_multiline(&label, 12.0);
    let mut origin = ScreenPoint::new(cursor.x + 12.0, cursor.y + 12.0);
    if origin.x + size.0 > plot_rect.max.x {
        origin.x = cursor.x - size.0 - 12.0;
    }
    if origin.y + size.1 > plot_rect.max.y {
        origin.y = cursor.y - size.1 - 12.0;
    }
    origin = clamp_point(origin, plot_rect, size);

    render.push(RenderCommand::Rect {
        rect: ScreenRect::new(
            origin,
            ScreenPoint::new(origin.x + size.0, origin.y + size.1),
        ),
        style: RectStyle {
            fill: theme.hover_bg,
            stroke: theme.hover_border,
            stroke_width: 1.0,
        },
    });

    for (index, line) in label.lines().enumerate() {
        let line_y = origin.y + index as f32 * 14.0 + 2.0;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(origin.x + 4.0, line_y),
            text: line.to_string(),
            style: TextStyle {
                color: theme.axis,
                size: 12.0,
            },
        });
    }
}

fn build_legend(
    render: &mut RenderList,
    plot: &Plot,
    state: &mut PlotUiState,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let series_list = plot.series();
    if series_list.is_empty() {
        state.legend_layout = None;
        return;
    }

    let font_size = LEGEND_FONT_SIZE;
    let line_height = LEGEND_LINE_HEIGHT;
    let padding = LEGEND_PADDING;
    let text_start_x = padding
        + LEGEND_TOGGLE_DIAMETER
        + LEGEND_TOGGLE_GAP
        + LEGEND_SWATCH_WIDTH
        + LEGEND_SWATCH_GAP;
    let mut max_width: f32 = 0.0;
    for series in series_list {
        let size = measurer.measure(series.name(), font_size);
        max_width = max_width.max(size.0);
    }
    let legend_width = text_start_x + max_width + padding;
    let legend_height = series_list.len() as f32 * line_height + padding * 2.0;

    let mut origin = ScreenPoint::new(
        plot_rect.max.x - legend_width - padding,
        plot_rect.min.y + padding,
    );
    origin = clamp_point(origin, plot_rect, (legend_width, legend_height));
    let legend_rect = ScreenRect::new(
        origin,
        ScreenPoint::new(origin.x + legend_width, origin.y + legend_height),
    );

    render.push(RenderCommand::Rect {
        rect: legend_rect,
        style: RectStyle {
            fill: theme.legend_bg,
            stroke: theme.legend_border,
            stroke_width: 1.0,
        },
    });

    let mut entries = Vec::with_capacity(series_list.len());
    for (idx, series) in series_list.iter().enumerate() {
        let row_y = origin.y + padding + idx as f32 * line_height;
        let row_rect = ScreenRect::new(
            ScreenPoint::new(origin.x, row_y),
            ScreenPoint::new(origin.x + legend_width, row_y + line_height),
        );
        let row_center_y = row_y + line_height * 0.5;
        let toggle_origin = ScreenPoint::new(
            origin.x + padding,
            row_center_y - LEGEND_TOGGLE_DIAMETER * 0.5,
        );
        let toggle_rect = ScreenRect::new(
            toggle_origin,
            ScreenPoint::new(
                toggle_origin.x + LEGEND_TOGGLE_DIAMETER,
                toggle_origin.y + LEGEND_TOGGLE_DIAMETER,
            ),
        );
        entries.push(LegendEntry {
            series_id: series.id(),
            row_rect,
        });

        let visible = series.is_visible();
        let series_color = series_color(series);
        let swatch_color = if visible {
            series_color
        } else {
            with_alpha(series_color, LEGEND_HIDDEN_ALPHA)
        };
        let text_color = if visible {
            theme.axis
        } else {
            with_alpha(theme.axis, LEGEND_TEXT_HIDDEN_ALPHA)
        };
        let ring_color = if visible {
            with_alpha(theme.axis, 0.7)
        } else {
            with_alpha(theme.axis, 0.45)
        };
        let fill_color = if visible {
            series_color
        } else {
            theme.legend_bg
        };
        let toggle_center = ScreenPoint::new(
            toggle_rect.min.x + LEGEND_TOGGLE_DIAMETER * 0.5,
            toggle_rect.min.y + LEGEND_TOGGLE_DIAMETER * 0.5,
        );

        render.push(RenderCommand::Points {
            points: vec![toggle_center],
            style: MarkerStyle {
                color: ring_color,
                size: LEGEND_TOGGLE_DIAMETER,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![toggle_center],
            style: MarkerStyle {
                color: fill_color,
                size: LEGEND_TOGGLE_INNER_DIAMETER,
                shape: MarkerShape::Circle,
            },
        });

        let swatch_start = ScreenPoint::new(toggle_rect.max.x + LEGEND_TOGGLE_GAP, row_center_y);
        let swatch_end = ScreenPoint::new(swatch_start.x + LEGEND_SWATCH_WIDTH, row_center_y);
        render.push(RenderCommand::LineSegments {
            segments: vec![LineSegment::new(swatch_start, swatch_end)],
            style: LineStyle {
                color: swatch_color,
                width: 2.0,
            },
        });
        let text_y = row_y + (line_height - font_size) * 0.5;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(swatch_end.x + LEGEND_SWATCH_GAP, text_y),
            text: series.name().to_string(),
            style: TextStyle {
                color: text_color,
                size: font_size,
            },
        });
    }

    state.legend_layout = Some(LegendLayout {
        rect: legend_rect,
        entries,
    });
}

fn paint_frame(frame: &PlotFrame, window: &mut Window, cx: &mut App) {
    let mut clip_stack: Vec<ContentMask<Pixels>> = Vec::new();
    for command in frame.render.commands() {
        match command {
            RenderCommand::ClipRect(rect) => {
                clip_stack.push(ContentMask {
                    bounds: to_bounds(*rect),
                });
            }
            RenderCommand::ClipEnd => {
                clip_stack.pop();
            }
            RenderCommand::LineSegments { segments, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_lines(window, segments, *style);
                });
            }
            RenderCommand::Points { points, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_points(window, points, *style);
                });
            }
            RenderCommand::Rect { rect, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_rect(window, *rect, *style);
                });
            }
            RenderCommand::Text {
                position,
                text,
                style,
            } => {
                with_clip(window, &clip_stack, |window| {
                    paint_text(window, cx, *position, text, style);
                });
            }
        }
    }
}

fn paint_lines(window: &mut Window, segments: &[LineSegment], style: LineStyle) {
    if segments.is_empty() {
        return;
    }
    let width = style.width.max(0.5);
    let mut builder = PathBuilder::stroke(px(width));
    for segment in segments {
        builder.move_to(point(px(segment.start.x), px(segment.start.y)));
        builder.line_to(point(px(segment.end.x), px(segment.end.y)));
    }
    if let Ok(path) = builder.build() {
        window.paint_path(path, to_rgba(style.color));
    }
}

fn paint_points(window: &mut Window, points: &[ScreenPoint], style: MarkerStyle) {
    if points.is_empty() {
        return;
    }

    let size = style.size.max(2.0);
    match style.shape {
        MarkerShape::Circle => {
            let radius = size * 0.5;
            for pt in points {
                let bounds = Bounds::from_corners(
                    point(px(pt.x - radius), px(pt.y - radius)),
                    point(px(pt.x + radius), px(pt.y + radius)),
                );
                window.paint_quad(quad(
                    bounds,
                    Corners::all(px(radius)),
                    to_rgba(style.color),
                    Edges::all(px(0.0)),
                    to_rgba(style.color),
                    BorderStyle::default(),
                ));
            }
        }
        MarkerShape::Square => {
            let half = size * 0.5;
            for pt in points {
                let bounds = Bounds::from_corners(
                    point(px(pt.x - half), px(pt.y - half)),
                    point(px(pt.x + half), px(pt.y + half)),
                );
                window.paint_quad(quad(
                    bounds,
                    Corners::all(px(0.0)),
                    to_rgba(style.color),
                    Edges::all(px(0.0)),
                    to_rgba(style.color),
                    BorderStyle::default(),
                ));
            }
        }
        MarkerShape::Cross => {
            let half = size * 0.5;
            let mut builder = PathBuilder::stroke(px(1.0));
            for pt in points {
                let h_start = point(px(pt.x - half), px(pt.y));
                let h_end = point(px(pt.x + half), px(pt.y));
                let v_start = point(px(pt.x), px(pt.y - half));
                let v_end = point(px(pt.x), px(pt.y + half));
                builder.move_to(h_start);
                builder.line_to(h_end);
                builder.move_to(v_start);
                builder.line_to(v_end);
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, to_rgba(style.color));
            }
        }
    }
}

fn paint_rect(window: &mut Window, rect: ScreenRect, style: RectStyle) {
    let bounds = to_bounds(rect);
    let quad = quad(
        bounds,
        Corners::all(px(0.0)),
        to_rgba(style.fill),
        Edges::all(px(style.stroke_width)),
        to_rgba(style.stroke),
        BorderStyle::default(),
    );
    window.paint_quad(quad);
}

fn paint_text(
    window: &mut Window,
    cx: &mut App,
    position: ScreenPoint,
    text: &str,
    style: &TextStyle,
) {
    if text.is_empty() {
        return;
    }
    let font_size = px(style.size);
    let run = TextRun {
        len: text.len(),
        font: font(".SystemUIFont"),
        color: to_hsla(style.color),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window
        .text_system()
        .shape_line(text.to_string().into(), font_size, &[run], None);
    let line_height = shaped.ascent + shaped.descent;
    let origin = point(px(position.x), px(position.y));
    let _ = shaped.paint(origin, line_height, window, cx);
}

fn to_rgba(color: Color) -> gpui::Rgba {
    gpui::Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn to_hsla(color: Color) -> gpui::Hsla {
    gpui::Hsla::from(to_rgba(color))
}

fn screen_point(point: Point<Pixels>) -> ScreenPoint {
    ScreenPoint::new(f32::from(point.x), f32::from(point.y))
}

fn to_bounds(rect: ScreenRect) -> Bounds<Pixels> {
    Bounds::from_corners(
        point(px(rect.min.x), px(rect.min.y)),
        point(px(rect.max.x), px(rect.max.y)),
    )
}

fn with_clip(window: &mut Window, stack: &[ContentMask<Pixels>], f: impl FnOnce(&mut Window)) {
    if let Some(mask) = stack.last() {
        window.with_content_mask(Some(mask.clone()), f);
    } else {
        f(window);
    }
}

fn normalized_rect(rect: ScreenRect) -> ScreenRect {
    let min_x = rect.min.x.min(rect.max.x);
    let max_x = rect.min.x.max(rect.max.x);
    let min_y = rect.min.y.min(rect.max.y);
    let max_y = rect.min.y.max(rect.max.y);
    ScreenRect::new(
        ScreenPoint::new(min_x, min_y),
        ScreenPoint::new(max_x, max_y),
    )
}

fn rect_contains(rect: ScreenRect, point: ScreenPoint) -> bool {
    point.x >= rect.min.x && point.x <= rect.max.x && point.y >= rect.min.y && point.y <= rect.max.y
}

fn distance_sq(a: ScreenPoint, b: ScreenPoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        a: (color.a * alpha).clamp(0.0, 1.0),
        ..color
    }
}

fn marker_style_and_size(series: &Series) -> (MarkerStyle, f32) {
    match series.kind() {
        SeriesKind::Line(line) => (
            MarkerStyle {
                color: line.color,
                size: 6.0,
                shape: MarkerShape::Circle,
            },
            6.0,
        ),
        SeriesKind::Scatter(marker) => (
            MarkerStyle {
                color: marker.color,
                size: marker.size.max(6.0),
                shape: marker.shape,
            },
            marker.size.max(6.0),
        ),
    }
}

fn cluster_pin_labels(labels: &[PinLabel], radius: f32) -> Vec<Vec<usize>> {
    let radius_sq = radius * radius;
    let mut visited = vec![false; labels.len()];
    let mut clusters: Vec<Vec<usize>> = Vec::new();

    for start in 0..labels.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut cluster = Vec::new();
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            cluster.push(index);
            for next in 0..labels.len() {
                if visited[next] {
                    continue;
                }
                if distance_sq(labels[index].screen, labels[next].screen) <= radius_sq {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        clusters.push(cluster);
    }

    clusters
}

fn cluster_center(labels: &[PinLabel], cluster: &[usize]) -> ScreenPoint {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for index in cluster {
        let screen = labels[*index].screen;
        sum_x += screen.x;
        sum_y += screen.y;
    }
    let count = cluster.len().max(1) as f32;
    ScreenPoint::new(sum_x / count, sum_y / count)
}

fn pin_label_candidates(screen: ScreenPoint, size: (f32, f32), offset: f32) -> [ScreenPoint; 6] {
    [
        ScreenPoint::new(screen.x + offset, screen.y + offset),
        ScreenPoint::new(screen.x + offset, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 - offset, screen.y + offset),
        ScreenPoint::new(screen.x - size.0 - offset, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 * 0.5, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 * 0.5, screen.y + offset),
    ]
}

fn rect_intersects(a: ScreenRect, b: ScreenRect) -> bool {
    !(a.max.x <= b.min.x || a.min.x >= b.max.x || a.max.y <= b.min.y || a.min.y >= b.max.y)
}

fn rect_intersects_any(rect: ScreenRect, others: &[ScreenRect]) -> bool {
    others.iter().any(|other| rect_intersects(rect, *other))
}

fn place_label(
    screen: ScreenPoint,
    size: (f32, f32),
    plot_rect: ScreenRect,
    offset: f32,
    placed: &[ScreenRect],
) -> Option<(ScreenPoint, ScreenRect)> {
    for origin in pin_label_candidates(screen, size, offset) {
        let origin = clamp_point(origin, plot_rect, size);
        let rect = ScreenRect::new(
            origin,
            ScreenPoint::new(origin.x + size.0, origin.y + size.1),
        );
        if !rect_intersects_any(rect, placed) {
            return Some((origin, rect));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn push_label_with_leader(
    render: &mut RenderList,
    rect: ScreenRect,
    origin: ScreenPoint,
    screen: ScreenPoint,
    label: &str,
    font_size: f32,
    line_height: f32,
    theme: &Theme,
) {
    let anchor = ScreenPoint::new(
        screen.x.clamp(rect.min.x, rect.max.x),
        screen.y.clamp(rect.min.y, rect.max.y),
    );
    render.push(RenderCommand::LineSegments {
        segments: vec![LineSegment::new(screen, anchor)],
        style: LineStyle {
            color: theme.pin_border,
            width: 1.0,
        },
    });
    render.push(RenderCommand::Rect {
        rect,
        style: RectStyle {
            fill: theme.pin_bg,
            stroke: theme.pin_border,
            stroke_width: 1.0,
        },
    });
    for (index, line) in label.lines().enumerate() {
        let line_y = origin.y + index as f32 * line_height + 2.0;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(origin.x + 4.0, line_y),
            text: line.to_string(),
            style: TextStyle {
                color: theme.axis,
                size: font_size,
            },
        });
    }
}

fn hover_target_within_threshold(
    target: &HoverTarget,
    cursor: ScreenPoint,
    config: &PlotViewConfig,
) -> bool {
    let threshold = if target.is_pinned {
        config.unpin_threshold_px
    } else {
        config.pin_threshold_px
    };
    distance_sq(target.screen, cursor) <= threshold * threshold
}

fn update_hover_target(
    plot: &Plot,
    state: &mut PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
    pin_threshold: f32,
    unpin_threshold: f32,
) {
    let Some(cursor) = state.hover else {
        state.hover_target = None;
        return;
    };
    state.hover_target = compute_hover_target(
        plot,
        transform,
        cursor,
        Some(plot_rect),
        pin_threshold,
        unpin_threshold,
    );
}

fn compute_hover_target(
    plot: &Plot,
    transform: &Transform,
    cursor: ScreenPoint,
    plot_rect: Option<ScreenRect>,
    pin_threshold: f32,
    unpin_threshold: f32,
) -> Option<HoverTarget> {
    let plot_rect = plot_rect?;
    if cursor.x < plot_rect.min.x
        || cursor.x > plot_rect.max.x
        || cursor.y < plot_rect.min.y
        || cursor.y > plot_rect.max.y
    {
        return None;
    }

    if let Some(target) = nearest_pinned_within(plot, transform, cursor, plot_rect, unpin_threshold)
    {
        return Some(target);
    }

    find_nearest_unpinned_point(plot, transform, cursor, plot_rect, pin_threshold)
}

fn nearest_pinned_within(
    plot: &Plot,
    transform: &Transform,
    cursor: ScreenPoint,
    plot_rect: ScreenRect,
    threshold: f32,
) -> Option<HoverTarget> {
    let threshold_sq = threshold * threshold;
    let mut best: Option<(crate::interaction::Pin, ScreenPoint, f32)> = None;
    for pin in plot.pins() {
        let Some(screen) = pin_screen_point(plot, *pin, transform) else {
            continue;
        };
        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            continue;
        }
        let dist = distance_sq(screen, cursor);
        if dist > threshold_sq {
            continue;
        }
        if best.is_none_or(|best| dist < best.2) {
            best = Some((*pin, screen, dist));
        }
    }
    best.map(|(pin, screen, _)| HoverTarget {
        pin,
        screen,
        is_pinned: true,
    })
}

fn find_nearest_unpinned_point(
    plot: &Plot,
    transform: &Transform,
    cursor: ScreenPoint,
    plot_rect: ScreenRect,
    threshold: f32,
) -> Option<HoverTarget> {
    let center = transform.screen_to_data(cursor)?;
    let edge = transform.screen_to_data(ScreenPoint::new(cursor.x + threshold, cursor.y))?;
    let dx = (edge.x - center.x).abs();
    let search_range = Range::new(center.x - dx, center.x + dx);
    let threshold_sq = threshold * threshold;
    let pins = plot.pins();
    let mut best: Option<(crate::interaction::Pin, ScreenPoint, f32)> = None;

    for series in plot.series() {
        if !series.is_visible() {
            continue;
        }
        let data = series.data().data();
        let index_range = data.range_by_x(search_range);
        for index in index_range {
            let Some(point) = data.point(index) else {
                continue;
            };
            let pin = crate::interaction::Pin {
                series_id: series.id(),
                point_index: index,
            };
            if pins.contains(&pin) {
                continue;
            }
            let Some(screen) = transform.data_to_screen(point) else {
                continue;
            };
            if screen.x < plot_rect.min.x
                || screen.x > plot_rect.max.x
                || screen.y < plot_rect.min.y
                || screen.y > plot_rect.max.y
            {
                continue;
            }
            let dist = distance_sq(screen, cursor);
            if dist > threshold_sq {
                continue;
            }
            if best.is_none_or(|best| dist < best.2) {
                best = Some((pin, screen, dist));
            }
        }
    }

    best.map(|(pin, screen, _)| HoverTarget {
        pin,
        screen,
        is_pinned: false,
    })
}

fn pin_screen_point(
    plot: &Plot,
    pin: crate::interaction::Pin,
    transform: &Transform,
) -> Option<ScreenPoint> {
    let series = plot
        .series()
        .iter()
        .find(|series| series.id() == pin.series_id)?;
    if !series.is_visible() {
        return None;
    }
    let point = series.data().data().point(pin.point_index)?;
    transform.data_to_screen(point)
}

fn clamp_point(point: ScreenPoint, rect: ScreenRect, size: (f32, f32)) -> ScreenPoint {
    let mut x = point.x;
    let mut y = point.y;
    if x < rect.min.x {
        x = rect.min.x;
    }
    if y < rect.min.y {
        y = rect.min.y;
    }
    if x + size.0 > rect.max.x {
        x = rect.max.x - size.0;
    }
    if y + size.1 > rect.max.y {
        y = rect.max.y - size.1;
    }
    ScreenPoint::new(x, y)
}

fn revert_pin_toggle(plot: &mut Plot, toggle: PinToggle) {
    let pins = plot.pins_mut();
    if toggle.added {
        if let Some(index) = pins.iter().position(|pin| *pin == toggle.pin) {
            pins.swap_remove(index);
        }
    } else if !pins.contains(&toggle.pin) {
        pins.push(toggle.pin);
    }
}

fn axis_title_text(axis: &AxisConfig) -> Option<String> {
    match (axis.title(), axis.units()) {
        (Some(title), Some(units)) => Some(format!("{title} ({units})")),
        (Some(title), None) => Some(title.to_string()),
        (None, Some(units)) => Some(units.to_string()),
        (None, None) => None,
    }
}

fn series_color(series: &Series) -> Color {
    match series.kind() {
        SeriesKind::Line(style) => style.color,
        SeriesKind::Scatter(style) => style.color,
    }
}

fn apply_manual_view(
    plot: &mut Plot,
    state: &mut PlotUiState,
    rect: ScreenRect,
    viewport: Viewport,
) {
    plot.set_manual_view(viewport);
    state.viewport = Some(viewport);
    state.transform = Transform::new(viewport, rect, plot.x_axis().scale(), plot.y_axis().scale());
}

trait ViewportCenter {
    fn center(&self) -> DataPoint;
    fn x_center(&self) -> DataPoint;
    fn y_center(&self) -> DataPoint;
}

impl ViewportCenter for Viewport {
    fn center(&self) -> DataPoint {
        DataPoint::new(
            (self.x.min + self.x.max) * 0.5,
            (self.y.min + self.y.max) * 0.5,
        )
    }

    fn x_center(&self) -> DataPoint {
        DataPoint::new(
            (self.x.min + self.x.max) * 0.5,
            (self.y.min + self.y.max) * 0.5,
        )
    }

    fn y_center(&self) -> DataPoint {
        DataPoint::new(
            (self.x.min + self.x.max) * 0.5,
            (self.y.min + self.y.max) * 0.5,
        )
    }
}

struct GpuiTextMeasurer<'a> {
    window: &'a Window,
}

impl<'a> GpuiTextMeasurer<'a> {
    fn new(window: &'a Window) -> Self {
        Self { window }
    }

    fn measure_multiline(&self, text: &str, size: f32) -> (f32, f32) {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for line in text.lines() {
            let (w, h) = self.measure(line, size);
            width = width.max(w);
            height += h.max(size * 1.2);
        }
        (width + 8.0, height + 8.0)
    }
}

impl TextMeasurer for GpuiTextMeasurer<'_> {
    fn measure(&self, text: &str, size: f32) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, 0.0);
        }
        let run = TextRun {
            len: text.len(),
            font: font(".SystemUIFont"),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped =
            self.window
                .text_system()
                .shape_line(text.to_string().into(), px(size), &[run], None);
        let width = f32::from(shaped.width);
        let height = f32::from(shaped.ascent + shaped.descent);
        (width, height.max(size * 1.2))
    }
}
