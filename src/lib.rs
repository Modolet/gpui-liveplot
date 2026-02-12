//! gpui_plot is a high-performance plotting library built for GPUI.
//! The crate targets append-only sensor data with stable 60fps interaction.

#![forbid(unsafe_code)]

pub mod axis;
pub mod datasource;
pub mod geom;
pub mod plot;
pub mod render;
pub mod style;
pub mod transform;
pub mod view;

pub use axis::{AxisConfig, AxisFormatter, AxisScale};
pub use datasource::{AppendError, AppendOnlyData, DecimationScratch, SeriesStore, XMode};
pub use geom::{Point, ScreenPoint, ScreenRect};
pub use plot::{Plot, PlotBuilder};
pub use render::{
    Color, LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCacheKey,
    RenderCommand, RenderList, TextStyle, build_line_segments, build_scatter_points,
};
pub use style::Theme;
pub use transform::Transform;
pub use view::{Range, View, Viewport};
