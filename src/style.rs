//! Style and theming configuration.

use crate::render::Color;

/// Visual theme for plot-level elements such as axes, grid, and overlays.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Plot background color.
    pub background: Color,
    /// Axis, tick, and label color.
    pub axis: Color,
    /// Major grid line color.
    pub grid_major: Color,
    /// Minor grid line color.
    pub grid_minor: Color,
    /// Hover tooltip background color.
    pub hover_bg: Color,
    /// Hover tooltip border color.
    pub hover_border: Color,
    /// Pin tooltip background color.
    pub pin_bg: Color,
    /// Pin tooltip border color.
    pub pin_border: Color,
    /// Selection rectangle fill color.
    pub selection_fill: Color,
    /// Selection rectangle border color.
    pub selection_border: Color,
    /// Legend background color.
    pub legend_bg: Color,
    /// Legend border color.
    pub legend_border: Color,
}

impl Theme {
    /// Create the default theme (alias of [`Theme::light`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a light theme palette.
    pub fn light() -> Self {
        Self {
            background: Color::new(1.0, 1.0, 1.0, 1.0),
            axis: Color::new(0.2, 0.2, 0.2, 1.0),
            grid_major: Color::new(0.86, 0.86, 0.86, 1.0),
            grid_minor: Color::new(0.93, 0.93, 0.93, 1.0),
            hover_bg: Color::new(1.0, 1.0, 1.0, 0.9),
            hover_border: Color::new(0.2, 0.2, 0.2, 0.8),
            pin_bg: Color::new(1.0, 1.0, 1.0, 0.92),
            pin_border: Color::new(0.2, 0.2, 0.2, 0.8),
            selection_fill: Color::new(0.1, 0.4, 0.9, 0.15),
            selection_border: Color::new(0.1, 0.4, 0.9, 0.9),
            legend_bg: Color::new(1.0, 1.0, 1.0, 0.85),
            legend_border: Color::new(0.2, 0.2, 0.2, 0.6),
        }
    }

    /// Create a dark theme palette.
    pub fn dark() -> Self {
        Self {
            background: Color::new(0.08, 0.08, 0.09, 1.0),
            axis: Color::new(0.85, 0.85, 0.85, 1.0),
            grid_major: Color::new(0.25, 0.25, 0.28, 1.0),
            grid_minor: Color::new(0.18, 0.18, 0.2, 1.0),
            hover_bg: Color::new(0.12, 0.12, 0.13, 0.92),
            hover_border: Color::new(0.6, 0.6, 0.6, 0.8),
            pin_bg: Color::new(0.12, 0.12, 0.13, 0.92),
            pin_border: Color::new(0.6, 0.6, 0.6, 0.85),
            selection_fill: Color::new(0.2, 0.5, 0.95, 0.18),
            selection_border: Color::new(0.3, 0.6, 1.0, 0.9),
            legend_bg: Color::new(0.12, 0.12, 0.13, 0.9),
            legend_border: Color::new(0.5, 0.5, 0.5, 0.7),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}
