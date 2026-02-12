//! Plot widget entry points and builders.

use crate::style::Theme;

/// Main plot widget container.
#[derive(Debug, Clone)]
pub struct Plot {
    theme: Theme,
}

impl Plot {
    /// Create a plot with default configuration.
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
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
}

impl PlotBuilder {
    /// Set the theme used by the plot.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Build the plot.
    pub fn build(self) -> Plot {
        Plot { theme: self.theme }
    }
}
