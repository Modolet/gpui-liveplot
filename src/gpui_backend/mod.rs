//! GPUI integration for gpui_plot.
//!
//! This module provides a GPUI view that renders a [`Plot`] and handles
//! mouse interactions (pan, zoom, box zoom, and pinning).

#![allow(clippy::collapsible_if)]

mod config;
mod constants;
mod frame;
mod geometry;
mod hover;
mod paint;
mod state;
mod text;
mod view;

pub use config::PlotViewConfig;
pub use view::{GpuiPlotView, PlotHandle};
