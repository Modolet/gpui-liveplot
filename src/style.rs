//! Style and theming configuration.
//!
//! Themes describe plot-level colors (background, grid, axes, overlays).

use gpui_kit::Rgba;

/// Visual theme for plot-level elements such as axes, grid, and overlays.
///
/// Themes are applied at the plot level and affect all series and overlays.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Plot background color.
    pub background: Rgba,
    /// Axis, tick, and label color.
    pub axis: Rgba,
    /// Major grid line color.
    pub grid_major: Rgba,
    /// Minor grid line color.
    pub grid_minor: Rgba,
    /// Hover tooltip background color.
    pub hover_bg: Rgba,
    /// Hover tooltip border color.
    pub hover_border: Rgba,
    /// Pin tooltip background color.
    pub pin_bg: Rgba,
    /// Pin tooltip border color.
    pub pin_border: Rgba,
    /// Selection rectangle fill color.
    pub selection_fill: Rgba,
    /// Selection rectangle border color.
    pub selection_border: Rgba,
    /// Legend background color.
    pub legend_bg: Rgba,
    /// Legend border color.
    pub legend_border: Rgba,
}

impl Theme {
    /// Create the default theme (alias of [`Theme::dark`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a light theme palette.
    pub fn light() -> Self {
        Self {
            background: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            axis: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            },
            grid_major: Rgba {
                r: 0.86,
                g: 0.86,
                b: 0.86,
                a: 1.0,
            },
            grid_minor: Rgba {
                r: 0.93,
                g: 0.93,
                b: 0.93,
                a: 1.0,
            },
            hover_bg: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.9,
            },
            hover_border: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 0.8,
            },
            pin_bg: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.92,
            },
            pin_border: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 0.8,
            },
            selection_fill: Rgba {
                r: 0.1,
                g: 0.4,
                b: 0.9,
                a: 0.15,
            },
            selection_border: Rgba {
                r: 0.1,
                g: 0.4,
                b: 0.9,
                a: 0.9,
            },
            legend_bg: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.85,
            },
            legend_border: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 0.6,
            },
        }
    }

    /// Create a dark theme palette.
    pub fn dark() -> Self {
        Self {
            background: Rgba {
                r: 0.08,
                g: 0.08,
                b: 0.09,
                a: 1.0,
            },
            axis: Rgba {
                r: 0.85,
                g: 0.85,
                b: 0.85,
                a: 1.0,
            },
            grid_major: Rgba {
                r: 0.25,
                g: 0.25,
                b: 0.28,
                a: 1.0,
            },
            grid_minor: Rgba {
                r: 0.18,
                g: 0.18,
                b: 0.2,
                a: 1.0,
            },
            hover_bg: Rgba {
                r: 0.12,
                g: 0.12,
                b: 0.13,
                a: 0.92,
            },
            hover_border: Rgba {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                a: 0.8,
            },
            pin_bg: Rgba {
                r: 0.12,
                g: 0.12,
                b: 0.13,
                a: 0.92,
            },
            pin_border: Rgba {
                r: 0.6,
                g: 0.6,
                b: 0.6,
                a: 0.85,
            },
            selection_fill: Rgba {
                r: 0.2,
                g: 0.5,
                b: 0.95,
                a: 0.18,
            },
            selection_border: Rgba {
                r: 0.3,
                g: 0.6,
                b: 1.0,
                a: 0.9,
            },
            legend_bg: Rgba {
                r: 0.12,
                g: 0.12,
                b: 0.13,
                a: 0.9,
            },
            legend_border: Rgba {
                r: 0.5,
                g: 0.5,
                b: 0.5,
                a: 0.7,
            },
        }
    }

    /// Build a plot theme from the current `gpui-component` theme.
    ///
    /// This is only available when the `gpui_component_theme` feature is enabled.
    #[cfg(feature = "gpui_component_theme")]
    pub fn from_gpui_component_theme(theme: &gpui_kit::component::Theme) -> Self {
        Self {
            background: theme.background.into(),
            axis: theme.foreground.into(),
            grid_major: theme.border.opacity(0.9).into(),
            grid_minor: theme.border.opacity(0.45).into(),
            hover_bg: theme.popover.opacity(0.92).into(),
            hover_border: theme.border.opacity(0.9).into(),
            pin_bg: theme.popover.opacity(0.92).into(),
            pin_border: theme.border.opacity(0.95).into(),
            selection_fill: theme.selection.opacity(0.35).into(),
            selection_border: theme.selection.opacity(0.9).into(),
            legend_bg: theme.popover.opacity(0.9).into(),
            legend_border: theme.border.opacity(0.8).into(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
