//! gpui_liveplot is a high-performance plotting library built for GPUI.
//!
//! # Overview
//! - Designed for append-only, high-throughput telemetry and sensor streams.
//! - Plot-level axes with shared transforms across all series.
//! - Viewport-aware decimation keeps rendering near `O(width)` for smooth interaction.
//! - Interactive pan, zoom, box zoom, hover readout, and pin annotations via GPUI.
//!
//! # Feature flags
//! - `gpui_component_theme`: when enabled, [`gpui_backend::PlotView`] will use the
//!   current `gpui-component` global theme if it exists.
//!
//! # Quick start
//! ```rust
//! use gpui_liveplot::{LineStyle, Plot, Series, SeriesKind, Theme};
//!
//! let mut plot = Plot::builder().theme(Theme::dark()).build();
//! let series = Series::from_iter_y(
//!     "sensor",
//!     (0..1000).map(|i| (i as f64 * 0.01).sin()),
//!     SeriesKind::Line(LineStyle::default()),
//! );
//! plot.add_series(&series);
//! plot.refresh_viewport(0.05, 1e-6);
//! ```
//!
//! # GPUI integration
//! Use [`gpui_backend::PlotView`] to render and interact with a plot inside a GPUI
//! window. See the `examples/` directory for complete runnable examples.

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

pub use gpui_kit::{Hsla, Rgba};

pub use axis::{AxisConfig, AxisConfigBuilder, AxisFormatter, TickConfig};
pub use datasource::AppendError;
pub use geom::Point;
pub use interaction::Pin;
pub use plot::{Plot, PlotBuilder};
pub use render::{LineStyle, MarkerShape, MarkerStyle};
pub use series::{Series, SeriesId, SeriesKind};
pub use style::Theme;
pub use view::{Range, View, Viewport};

pub use gpui_backend::{
    LinkMemberId, PlotHandle, PlotLinkGroup, PlotLinkOptions, PlotView, PlotViewConfig,
};
