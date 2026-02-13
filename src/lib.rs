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

pub use axis::{AxisConfig, AxisFormatter, AxisScale, TickConfig};
pub use datasource::AppendError;
pub use geom::Point;
pub use interaction::Pin;
pub use plot::{Plot, PlotBuilder};
pub use render::{
    Color, LineStyle, MarkerShape, MarkerStyle,
};
pub use series::{Series, SeriesId, SeriesKind};
pub use style::Theme;
pub use view::{Range, View, Viewport};

pub use gpui_backend::{GpuiPlotView, PlotHandle, PlotViewConfig};
