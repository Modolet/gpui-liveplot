use std::sync::{Arc, RwLock};

use crate::view::{Range, Viewport};

const LINK_EPSILON: f64 = 1e-9;

/// Member identifier inside a plot link group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkMemberId(u64);

/// Link behavior switches for multi-plot synchronization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotLinkOptions {
    /// Synchronize X-axis range updates.
    pub link_x: bool,
    /// Synchronize Y-axis range updates.
    pub link_y: bool,
    /// Synchronize cursor X position (crosshair).
    pub link_cursor: bool,
    /// Synchronize brush X range selections.
    pub link_brush: bool,
    /// Synchronize reset-view actions (double click reset).
    pub link_reset: bool,
}

impl Default for PlotLinkOptions {
    fn default() -> Self {
        Self {
            link_x: true,
            link_y: false,
            link_cursor: false,
            link_brush: false,
            link_reset: true,
        }
    }
}

/// Shared link group used to synchronize multiple `GpuiPlotView` instances.
#[derive(Debug, Clone, Default)]
pub struct PlotLinkGroup {
    inner: Arc<RwLock<LinkGroupState>>,
}

impl PlotLinkGroup {
    /// Create an empty link group.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register_member(&self) -> LinkMemberId {
        let mut state = self.inner.write().expect("link group lock");
        state.next_member_id = state.next_member_id.wrapping_add(1);
        LinkMemberId(state.next_member_id)
    }

    pub(crate) fn publish_manual_view(
        &self,
        source: LinkMemberId,
        viewport: Viewport,
        sync_x: bool,
        sync_y: bool,
    ) {
        if !sync_x && !sync_y {
            return;
        }
        let mut state = self.inner.write().expect("link group lock");
        if let Some(current) = state.view_update
            && let ViewSyncKind::Manual {
                viewport: current_viewport,
                sync_x: current_sync_x,
                sync_y: current_sync_y,
            } = current.kind
            && current.source == source
            && current_sync_x == sync_x
            && current_sync_y == sync_y
            && viewport_approx_eq(current_viewport, viewport)
        {
            return;
        }
        let seq = state.next_seq();
        state.view_update = Some(ViewLinkUpdate {
            seq,
            source,
            kind: ViewSyncKind::Manual {
                viewport,
                sync_x,
                sync_y,
            },
        });
    }

    pub(crate) fn publish_reset(&self, source: LinkMemberId) {
        let mut state = self.inner.write().expect("link group lock");
        if let Some(current) = state.view_update
            && matches!(current.kind, ViewSyncKind::Reset)
            && current.source == source
        {
            return;
        }
        let seq = state.next_seq();
        state.view_update = Some(ViewLinkUpdate {
            seq,
            source,
            kind: ViewSyncKind::Reset,
        });
    }

    pub(crate) fn publish_cursor_x(&self, source: LinkMemberId, x: Option<f64>) {
        let mut state = self.inner.write().expect("link group lock");
        if let Some(current) = state.cursor_update
            && current.source == source
            && option_f64_approx_eq(current.x, x)
        {
            return;
        }
        let seq = state.next_seq();
        state.cursor_update = Some(CursorLinkUpdate { seq, source, x });
    }

    pub(crate) fn publish_brush_x(&self, source: LinkMemberId, x_range: Option<Range>) {
        let mut state = self.inner.write().expect("link group lock");
        if let Some(current) = state.brush_update
            && current.source == source
            && option_range_approx_eq(current.x_range, x_range)
        {
            return;
        }
        let seq = state.next_seq();
        state.brush_update = Some(BrushLinkUpdate {
            seq,
            source,
            x_range,
        });
    }

    pub(crate) fn latest_view_update(&self) -> Option<ViewLinkUpdate> {
        self.inner.read().expect("link group lock").view_update
    }

    pub(crate) fn latest_cursor_update(&self) -> Option<CursorLinkUpdate> {
        self.inner.read().expect("link group lock").cursor_update
    }

    pub(crate) fn latest_brush_update(&self) -> Option<BrushLinkUpdate> {
        self.inner.read().expect("link group lock").brush_update
    }
}

#[derive(Debug, Default)]
struct LinkGroupState {
    next_member_id: u64,
    next_seq: u64,
    view_update: Option<ViewLinkUpdate>,
    cursor_update: Option<CursorLinkUpdate>,
    brush_update: Option<BrushLinkUpdate>,
}

impl LinkGroupState {
    fn next_seq(&mut self) -> u64 {
        self.next_seq = self.next_seq.wrapping_add(1);
        self.next_seq
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LinkBinding {
    pub(crate) group: PlotLinkGroup,
    pub(crate) member_id: LinkMemberId,
    pub(crate) options: PlotLinkOptions,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewLinkUpdate {
    pub(crate) seq: u64,
    pub(crate) source: LinkMemberId,
    pub(crate) kind: ViewSyncKind,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ViewSyncKind {
    Manual {
        viewport: Viewport,
        sync_x: bool,
        sync_y: bool,
    },
    Reset,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CursorLinkUpdate {
    pub(crate) seq: u64,
    pub(crate) source: LinkMemberId,
    pub(crate) x: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BrushLinkUpdate {
    pub(crate) seq: u64,
    pub(crate) source: LinkMemberId,
    pub(crate) x_range: Option<Range>,
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= LINK_EPSILON
}

fn option_f64_approx_eq(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => approx_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn range_approx_eq(a: Range, b: Range) -> bool {
    approx_eq(a.min, b.min) && approx_eq(a.max, b.max)
}

fn option_range_approx_eq(a: Option<Range>, b: Option<Range>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => range_approx_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn viewport_approx_eq(a: Viewport, b: Viewport) -> bool {
    range_approx_eq(a.x, b.x) && range_approx_eq(a.y, b.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_view_publish_deduplicates_same_payload() {
        let group = PlotLinkGroup::new();
        let member = group.register_member();
        let viewport = Viewport::new(Range::new(0.0, 10.0), Range::new(-1.0, 1.0));

        group.publish_manual_view(member, viewport, true, false);
        let first = group.latest_view_update().expect("view update");
        group.publish_manual_view(member, viewport, true, false);
        let second = group.latest_view_update().expect("view update");

        assert_eq!(first.seq, second.seq);
    }

    #[test]
    fn reset_publish_replaces_previous_view_event() {
        let group = PlotLinkGroup::new();
        let member = group.register_member();
        let viewport = Viewport::new(Range::new(0.0, 5.0), Range::new(0.0, 1.0));
        group.publish_manual_view(member, viewport, true, false);
        let first = group.latest_view_update().expect("view update").seq;

        group.publish_reset(member);
        let update = group.latest_view_update().expect("view update");
        assert!(update.seq > first);
        assert!(matches!(update.kind, ViewSyncKind::Reset));
    }
}
