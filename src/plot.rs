//! Plot widget entry points and builders.
//!
//! A [`Plot`] owns axis configuration, view mode, and a set of series. All
//! series in a plot share the same axes and transforms.

use crate::axis::AxisConfig;
use crate::interaction::Pin;
use crate::series::Series;
use crate::style::Theme;
use crate::view::{Range, View, Viewport};

/// Main plot widget container.
///
/// A plot is backend-agnostic and focuses on data, view state, and styling.
/// Render backends (such as the GPUI backend) drive viewport refreshes and
/// interaction state.
#[derive(Debug, Clone)]
pub struct Plot {
    theme: Theme,
    x_axis: AxisConfig,
    y_axis: AxisConfig,
    view: View,
    viewport: Option<Viewport>,
    series: Vec<Series>,
    pins: Vec<Pin>,
}

impl Plot {
    /// Create a plot with default configuration.
    ///
    /// Equivalent to `PlotBuilder::default().build()`.
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
            x_axis: AxisConfig::default(),
            y_axis: AxisConfig::default(),
            view: View::default(),
            viewport: None,
            series: Vec::new(),
            pins: Vec::new(),
        }
    }

    /// Start building a plot with custom configuration.
    pub fn builder() -> PlotBuilder {
        PlotBuilder::default()
    }

    /// Access the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Set the plot theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Access the X axis configuration.
    pub fn x_axis(&self) -> &AxisConfig {
        &self.x_axis
    }

    /// Access the Y axis configuration.
    pub fn y_axis(&self) -> &AxisConfig {
        &self.y_axis
    }

    /// Access the active view mode.
    pub fn view(&self) -> View {
        self.view
    }

    /// Access the current viewport.
    ///
    /// The viewport is computed by [`Plot::refresh_viewport`].
    pub fn viewport(&self) -> Option<Viewport> {
        self.viewport
    }

    /// Access all series.
    pub fn series(&self) -> &[Series] {
        &self.series
    }

    /// Access all series mutably.
    pub fn series_mut(&mut self) -> &mut [Series] {
        &mut self.series
    }

    /// Add a series to the plot.
    ///
    /// The plot stores a shared handle instead of taking unique ownership.
    /// Appends made through other shared handles are visible immediately.
    pub fn add_series(&mut self, series: &Series) {
        self.series.push(series.share());
    }

    /// Access the pinned points.
    pub fn pins(&self) -> &[Pin] {
        &self.pins
    }

    /// Access the pinned points mutably.
    pub fn pins_mut(&mut self) -> &mut Vec<Pin> {
        &mut self.pins
    }

    /// Compute bounds across all visible series.
    pub fn data_bounds(&self) -> Option<Viewport> {
        let mut x_range: Option<Range> = None;
        let mut y_range: Option<Range> = None;
        for series in &self.series {
            if !series.is_visible() {
                continue;
            }
            if let Some(bounds) = series.bounds() {
                x_range = Some(match x_range {
                    None => bounds.x,
                    Some(existing) => Range::union(existing, bounds.x)?,
                });
                y_range = Some(match y_range {
                    None => bounds.y,
                    Some(existing) => Range::union(existing, bounds.y)?,
                });
            }
        }
        match (x_range, y_range) {
            (Some(x), Some(y)) => Some(Viewport::new(x, y)),
            _ => None,
        }
    }

    /// Enter manual view with the given viewport.
    pub fn set_manual_view(&mut self, viewport: Viewport) {
        self.view = View::Manual;
        self.viewport = Some(viewport);
    }

    /// Reset to automatic view.
    pub fn reset_view(&mut self) {
        self.view = View::default();
        self.viewport = None;
    }

    /// Refresh the viewport based on the current view mode and data.
    ///
    /// This updates the cached viewport and applies padding to avoid tight
    /// bounds during auto-fit.
    pub fn refresh_viewport(&mut self, padding_frac: f64, min_padding: f64) -> Option<Viewport> {
        let bounds = self.data_bounds()?;
        match self.view {
            View::AutoAll { auto_x, auto_y } => {
                let mut next = bounds;
                if let Some(current) = self.viewport {
                    if !auto_x {
                        next.x = current.x;
                    }
                    if !auto_y {
                        next.y = current.y;
                    }
                }
                self.viewport = Some(next.padded(padding_frac, min_padding));
            }
            View::Manual => {
                if self.viewport.is_none() {
                    self.viewport = Some(bounds);
                }
            }
            View::FollowLastN { points } => {
                self.viewport = self.follow_last(points, false);
            }
            View::FollowLastNXY { points } => {
                self.viewport = self.follow_last(points, true);
            }
        }
        self.viewport
    }

    fn follow_last(&self, points: usize, follow_y: bool) -> Option<Viewport> {
        let mut max_series: Option<&Series> = None;
        let mut max_point: Option<crate::geom::Point> = None;
        for series in &self.series {
            if !series.is_visible() {
                continue;
            }
            let last_point = series.with_store(|store| store.data().points().last().copied());
            if let Some(point) = last_point
                && max_point.is_none_or(|max| point.x > max.x)
            {
                max_point = Some(point);
                max_series = Some(series);
            }
        }

        let max_series = max_series?;
        let max_point = max_point?;
        let (len, start_point) = max_series.with_store(|store| {
            let data = store.data();
            let len = data.len();
            let start_index = len.saturating_sub(points);
            (len, data.point(start_index))
        });
        if len == 0 {
            return None;
        }
        let start_point = start_point?;
        let x_range = Range::new(start_point.x, max_point.x);

        let y_range = if follow_y {
            let mut y_range: Option<Range> = None;
            for series in &self.series {
                if !series.is_visible() {
                    continue;
                }
                series.with_store(|store| {
                    let series_data = store.data();
                    let index_range = series_data.range_by_x(x_range);
                    for index in index_range {
                        if let Some(point) = series_data.point(index) {
                            y_range = Some(match y_range {
                                None => Range::new(point.y, point.y),
                                Some(mut existing) => {
                                    existing.expand_to_include(point.y);
                                    existing
                                }
                            });
                        }
                    }
                });
            }
            y_range?
        } else if let Some(current) = self.viewport {
            current.y
        } else {
            self.data_bounds()?.y
        };

        Some(Viewport::new(x_range, y_range))
    }
}

impl Default for Plot {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring a plot before construction.
///
/// The builder captures theme, axes, view mode, and any initial series.
#[derive(Debug, Default)]
pub struct PlotBuilder {
    theme: Theme,
    x_axis: AxisConfig,
    y_axis: AxisConfig,
    view: View,
    series: Vec<Series>,
}

impl PlotBuilder {
    /// Set the theme used by the plot.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the X axis configuration.
    pub fn x_axis(mut self, axis: AxisConfig) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis configuration.
    pub fn y_axis(mut self, axis: AxisConfig) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the initial view mode.
    pub fn view(mut self, view: View) -> Self {
        self.view = view;
        self
    }

    /// Add a series to the plot.
    ///
    /// The builder stores a shared handle to the given series.
    pub fn series(mut self, series: &Series) -> Self {
        self.series.push(series.share());
        self
    }

    /// Build the plot.
    pub fn build(self) -> Plot {
        Plot {
            theme: self.theme,
            x_axis: self.x_axis,
            y_axis: self.y_axis,
            view: self.view,
            viewport: None,
            series: self.series,
            pins: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::Series;

    #[test]
    fn add_series_uses_shared_data_stream() {
        let mut source = Series::line("shared");
        let _ = source.extend_y([1.0, 2.0]);

        let mut plot = Plot::new();
        plot.add_series(&source);

        let initial_bounds = plot.data_bounds().expect("plot bounds");
        assert_eq!(initial_bounds.y.min, 1.0);
        assert_eq!(initial_bounds.y.max, 2.0);

        let _ = source.push_y(3.0);
        let next_bounds = plot.data_bounds().expect("plot bounds");
        assert_eq!(next_bounds.y.max, 3.0);
    }
}
