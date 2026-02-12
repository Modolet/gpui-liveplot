//! Plot widget entry points and builders.

use crate::axis::AxisConfig;
use crate::style::Theme;
use crate::view::View;

/// Main plot widget container.
#[derive(Debug, Clone)]
pub struct Plot {
    theme: Theme,
    x_axis: AxisConfig,
    y_axis: AxisConfig,
    view: View,
}

impl Plot {
    /// Create a plot with default configuration.
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
            x_axis: AxisConfig::default(),
            y_axis: AxisConfig::default(),
            view: View::default(),
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

    /// Build the plot.
    pub fn build(self) -> Plot {
        Plot {
            theme: self.theme,
            x_axis: self.x_axis,
            y_axis: self.y_axis,
            view: self.view,
        }
    }
}
