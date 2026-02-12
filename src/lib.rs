//! gpui_plot is a high-performance plotting library built for GPUI.
//! The crate targets append-only sensor data with stable 60fps interaction.

#![forbid(unsafe_code)]

pub mod axis;
pub mod datasource;
pub mod geom;
pub mod interaction;
pub mod plot;
pub mod render;
pub mod series;
pub mod style;
pub mod transform;
pub mod view;

pub mod gpui_backend;

pub use axis::{
    ApproxTextMeasurer, AxisConfig, AxisFormatter, AxisLayout, AxisLayoutCache, AxisScale,
    TextMeasurer, Tick, TickConfig, generate_ticks,
};
pub use datasource::{AppendError, AppendOnlyData, DecimationScratch, SeriesStore, XMode};
pub use geom::{Point, ScreenPoint, ScreenRect};
pub use interaction::{
    HitRegion, Pin, PinHit, PlotRegions, find_nearest_point, pan_viewport, toggle_pin,
    zoom_factor_from_drag, zoom_to_rect, zoom_viewport,
};
pub use plot::{Plot, PlotBuilder};
pub use render::{
    Color, LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCacheKey,
    RenderCommand, RenderList, TextStyle, build_line_segments, build_scatter_points,
};
pub use series::{Series, SeriesId, SeriesKind};
pub use style::Theme;
pub use transform::Transform;
pub use view::{Range, View, Viewport};

pub use gpui_backend::{GpuiPlotView, PlotHandle, PlotViewConfig};
