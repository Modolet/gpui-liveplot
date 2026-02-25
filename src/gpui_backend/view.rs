use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollWheelEvent,
    Window, canvas, div, px,
};

use crate::geom::{Point as DataPoint, ScreenPoint, ScreenRect};
use crate::interaction::{
    HitRegion, pan_viewport, toggle_pin, zoom_factor_from_drag, zoom_to_rect, zoom_viewport,
};
use crate::plot::Plot;
use crate::transform::Transform;
use crate::view::{Range, Viewport};

use super::config::PlotViewConfig;
use super::constants::DOUBLE_CLICK_PIN_GRACE_MS;
use super::frame::build_frame;
use super::geometry::{distance_sq, normalized_rect};
use super::hover::{compute_hover_target, hover_target_within_threshold};
use super::link::{LinkBinding, PlotLinkGroup, PlotLinkOptions, ViewSyncKind};
use super::paint::{paint_frame, to_hsla};
use super::state::{ClickState, DragMode, DragState, PinToggle, PlotUiState};

/// A GPUI view that renders a [`Plot`] with interactive controls.
///
/// This view handles pan/zoom/box-zoom, hover readouts, and pin interactions
/// while delegating data management to the underlying [`Plot`].
#[derive(Clone)]
pub struct GpuiPlotView {
    plot: Arc<RwLock<Plot>>,
    state: Arc<RwLock<PlotUiState>>,
    config: PlotViewConfig,
    link: Option<LinkBinding>,
}

impl GpuiPlotView {
    /// Create a new GPUI plot view for the given plot.
    ///
    /// Uses the default [`PlotViewConfig`].
    pub fn new(plot: Plot) -> Self {
        Self {
            plot: Arc::new(RwLock::new(plot)),
            state: Arc::new(RwLock::new(PlotUiState::default())),
            config: PlotViewConfig::default(),
            link: None,
        }
    }

    /// Create a new GPUI plot view with a custom configuration.
    pub fn with_config(plot: Plot, config: PlotViewConfig) -> Self {
        Self {
            plot: Arc::new(RwLock::new(plot)),
            state: Arc::new(RwLock::new(PlotUiState::default())),
            config,
            link: None,
        }
    }

    /// Attach this view to a multi-plot link group.
    ///
    /// Link groups synchronize viewport/cursor/brush state between views.
    pub fn with_link_group(mut self, group: PlotLinkGroup, options: PlotLinkOptions) -> Self {
        self.link = Some(LinkBinding {
            member_id: group.register_member(),
            group,
            options,
        });
        self
    }

    /// Get a handle for mutating the underlying plot.
    ///
    /// This is useful for streaming updates from async tasks.
    pub fn plot_handle(&self) -> PlotHandle {
        PlotHandle {
            plot: Arc::clone(&self.plot),
        }
    }

    fn publish_manual_view_link(&self, viewport: Viewport) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        link.group.publish_manual_view(
            link.member_id,
            viewport,
            link.options.link_x,
            link.options.link_y,
        );
    }

    fn publish_reset_link(&self) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        if link.options.link_reset {
            link.group.publish_reset(link.member_id);
        }
    }

    fn publish_cursor_link(&self, x: Option<f64>) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        if link.options.link_cursor {
            link.group.publish_cursor_x(link.member_id, x);
        }
    }

    fn publish_brush_link(&self, x_range: Option<Range>) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        if link.options.link_brush {
            link.group.publish_brush_x(link.member_id, x_range);
        }
    }

    fn apply_manual_view_with_link(
        &self,
        plot: &mut Plot,
        state: &mut PlotUiState,
        rect: ScreenRect,
        viewport: Viewport,
    ) {
        apply_manual_view(plot, state, rect, viewport);
        state.linked_brush_x = None;
        self.publish_manual_view_link(viewport);
        self.publish_brush_link(None);
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
                state.linked_brush_x = None;
                self.publish_reset_link();
                self.publish_brush_link(None);
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
        let linked_cursor_x = state.hover.and_then(|_| {
            state
                .transform
                .as_ref()
                .and_then(|transform| transform.screen_to_data(pos))
                .map(|point| point.x)
        });
        self.publish_cursor_link(linked_cursor_x);

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
                                self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
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
                            self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
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
                            self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
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
                                self.apply_manual_view_with_link(
                                    &mut plot,
                                    &mut state,
                                    transform.screen(),
                                    next,
                                );
                                self.publish_brush_link(Some(next.x));
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
        self.publish_cursor_link(None);
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
                        self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
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
        let link = self.link.clone();
        let theme = plot.read().expect("plot lock").theme().clone();

        div()
            .size_full()
            .bg(to_hsla(theme.background))
            .child(
                canvas(
                    move |bounds, window, _| {
                        let mut plot = plot.write().expect("plot lock");
                        let mut state = state.write().expect("plot state lock");
                        if let Some(link) = &link {
                            apply_link_updates(link, &mut plot, &mut state);
                        }
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
///
/// The handle clones cheaply and can be moved into async tasks.
#[derive(Clone)]
pub struct PlotHandle {
    plot: Arc<RwLock<Plot>>,
}

impl PlotHandle {
    /// Read the plot state.
    ///
    /// The plot is locked for the duration of the callback.
    pub fn read<R>(&self, f: impl FnOnce(&Plot) -> R) -> R {
        let plot = self.plot.read().expect("plot lock");
        f(&plot)
    }

    /// Mutate the plot state.
    ///
    /// The plot is locked for the duration of the callback.
    pub fn write<R>(&self, f: impl FnOnce(&mut Plot) -> R) -> R {
        let mut plot = self.plot.write().expect("plot lock");
        f(&mut plot)
    }
}

fn apply_link_updates(link: &LinkBinding, plot: &mut Plot, state: &mut PlotUiState) {
    if let Some(update) = link.group.latest_view_update()
        && update.seq > state.link_view_seq
    {
        state.link_view_seq = update.seq;
        if update.source != link.member_id {
            match update.kind {
                ViewSyncKind::Reset => {
                    if link.options.link_reset {
                        plot.reset_view();
                        state.viewport = None;
                        state.transform = None;
                        state.linked_brush_x = None;
                    }
                }
                ViewSyncKind::Manual {
                    viewport,
                    sync_x,
                    sync_y,
                } => {
                    let mut next = plot
                        .viewport()
                        .or_else(|| plot.data_bounds())
                        .unwrap_or(viewport);
                    let mut changed = false;
                    if sync_x && link.options.link_x {
                        next.x = viewport.x;
                        changed = true;
                    }
                    if sync_y && link.options.link_y {
                        next.y = viewport.y;
                        changed = true;
                    }
                    if changed {
                        plot.set_manual_view(next);
                        state.viewport = Some(next);
                        if let Some(rect) = state.plot_rect {
                            state.transform = Transform::new(next, rect);
                        }
                    }
                }
            }
        }
    }

    if let Some(update) = link.group.latest_cursor_update()
        && update.seq > state.link_cursor_seq
    {
        state.link_cursor_seq = update.seq;
        if update.source != link.member_id && link.options.link_cursor {
            state.linked_cursor_x = update.x;
        }
    }

    if let Some(update) = link.group.latest_brush_update()
        && update.seq > state.link_brush_seq
    {
        state.link_brush_seq = update.seq;
        if update.source != link.member_id && link.options.link_brush {
            state.linked_brush_x = update.x_range;
            if let Some(x_range) = update.x_range {
                let y_range = plot
                    .viewport()
                    .or_else(|| plot.data_bounds())
                    .map(|viewport| viewport.y)
                    .unwrap_or_else(|| Range::new(0.0, 1.0));
                let next = Viewport::new(x_range, y_range);
                plot.set_manual_view(next);
                state.viewport = Some(next);
                if let Some(rect) = state.plot_rect {
                    state.transform = Transform::new(next, rect);
                }
            }
        }
    }
}

fn screen_point(point: Point<Pixels>) -> ScreenPoint {
    ScreenPoint::new(f32::from(point.x), f32::from(point.y))
}

fn apply_manual_view(
    plot: &mut Plot,
    state: &mut PlotUiState,
    rect: ScreenRect,
    viewport: Viewport,
) {
    plot.set_manual_view(viewport);
    state.viewport = Some(viewport);
    state.transform = Transform::new(viewport, rect);
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
