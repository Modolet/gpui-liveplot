use std::collections::HashMap;
use std::time::Instant;

use gpui::MouseButton;

use crate::axis::AxisLayoutCache;
use crate::datasource::DecimationScratch;
use crate::geom::{ScreenPoint, ScreenRect};
use crate::interaction::{HitRegion, Pin, PlotRegions};
use crate::render::RenderCacheKey;
use crate::series::SeriesId;
use crate::transform::Transform;
use crate::view::Viewport;

use super::geometry::rect_contains;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DragMode {
    Pan,
    ZoomRect,
    ZoomX,
    ZoomY,
}

#[derive(Debug, Clone)]
pub(crate) struct DragState {
    pub(crate) mode: DragMode,
    pub(crate) start: ScreenPoint,
    pub(crate) last: ScreenPoint,
    pub(crate) active: bool,
}

impl DragState {
    pub(crate) fn new(mode: DragMode, start: ScreenPoint, active: bool) -> Self {
        Self {
            mode,
            start,
            last: start,
            active,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClickState {
    pub(crate) region: HitRegion,
    pub(crate) button: MouseButton,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PinToggle {
    pub(crate) pin: Pin,
    pub(crate) added: bool,
    pub(crate) at: Instant,
    pub(crate) screen_pos: ScreenPoint,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HoverTarget {
    pub(crate) pin: Pin,
    pub(crate) screen: ScreenPoint,
    pub(crate) is_pinned: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SeriesCache {
    pub(crate) key: Option<RenderCacheKey>,
    pub(crate) points: Vec<crate::geom::Point>,
}

#[derive(Debug, Clone)]
pub(crate) struct LegendEntry {
    pub(crate) series_id: SeriesId,
    pub(crate) row_rect: ScreenRect,
}

#[derive(Debug, Clone)]
pub(crate) struct LegendLayout {
    pub(crate) rect: ScreenRect,
    pub(crate) entries: Vec<LegendEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlotUiState {
    pub(crate) x_layout: AxisLayoutCache,
    pub(crate) y_layout: AxisLayoutCache,
    pub(crate) regions: PlotRegions,
    pub(crate) plot_rect: Option<ScreenRect>,
    pub(crate) transform: Option<Transform>,
    pub(crate) viewport: Option<Viewport>,
    pub(crate) drag: Option<DragState>,
    pub(crate) pending_click: Option<ClickState>,
    pub(crate) last_pin_toggle: Option<PinToggle>,
    pub(crate) hover_target: Option<HoverTarget>,
    pub(crate) selection_rect: Option<ScreenRect>,
    pub(crate) hover: Option<ScreenPoint>,
    pub(crate) last_cursor: Option<ScreenPoint>,
    pub(crate) decimation_scratch: DecimationScratch,
    pub(crate) series_cache: HashMap<SeriesId, SeriesCache>,
    pub(crate) legend_layout: Option<LegendLayout>,
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
    pub(crate) fn clear_interaction(&mut self) {
        self.drag = None;
        self.pending_click = None;
        self.selection_rect = None;
    }

    pub(crate) fn legend_hit(&self, point: ScreenPoint) -> Option<SeriesId> {
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
