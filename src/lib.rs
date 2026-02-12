//! gpui_plot is a high-performance plotting library built for GPUI.
//! The crate targets append-only sensor data with stable 60fps interaction.

#![forbid(unsafe_code)]

pub mod plot;
pub mod style;

pub use plot::{Plot, PlotBuilder};
pub use style::Theme;
