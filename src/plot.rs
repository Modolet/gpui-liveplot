//! Plot widget entry points and builders.

use crate::axis::AxisConfig;
use crate::interaction::Pin;
use crate::series::Series;
use crate::style::Theme;
use crate::view::{Range, View, Viewport};

/// Main plot widget container.
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
    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Access the pinned points.
    pub fn pins(&self) -> &[Pin] {
        &self.pins
    }

    /// Access the pinned points mutably.
    pub fn pins_mut(&mut self) -> &mut Vec<Pin> {
        &mut self.pins
    }

    /// Compute bounds across all series.
    pub fn data_bounds(&self) -> Option<Viewport> {
        let mut x_range: Option<Range> = None;
        let mut y_range: Option<Range> = None;
        for series in &self.series {
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
            let data = series.data().data();
            if let Some(point) = data.points().last().copied()
                && max_point.is_none_or(|max| point.x > max.x)
            {
                max_point = Some(point);
                max_series = Some(series);
            }
        }

        let max_series = max_series?;
        let max_point = max_point?;
        let data = max_series.data().data();
        let len = data.len();
        if len == 0 {
            return None;
        }
        let start_index = len.saturating_sub(points);
        let start_point = data.point(start_index)?;
        let x_range = Range::new(start_point.x, max_point.x);

        let y_range = if follow_y {
            let mut y_range: Option<Range> = None;
            for series in &self.series {
                if !series.is_visible() {
                    continue;
                }
                let series_data = series.data().data();
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
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
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
