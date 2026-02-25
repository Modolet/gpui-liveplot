//! GPUI integration for gpui_liveplot.
//!
//! This module provides a GPUI view that renders a [`Plot`](crate::plot::Plot)
//! and handles interactive behaviors such as pan, zoom, box zoom, hover
//! readouts, and pin annotations.

#![allow(clippy::collapsible_if)]

mod config;
mod constants;
mod frame;
mod geometry;
mod hover;
mod link;
mod paint;
mod state;
mod text;
mod view;

pub use config::PlotViewConfig;
pub use link::{LinkMemberId, PlotLinkGroup, PlotLinkOptions};
pub use view::{GpuiPlotView, PlotHandle};
