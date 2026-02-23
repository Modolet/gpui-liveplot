//! Axis configuration, scaling, and formatting.
//!
//! Axes are configured at the plot level and shared across all series. This module provides:
//! - scale types (linear),
//! - formatting and tick generation,
//! - layout metadata used by render backends.

use std::sync::Arc;

use crate::view::Range;

/// Formatter for axis tick labels.
///
/// Use [`AxisFormatter::Custom`] to provide a locale-aware or domain-specific
/// formatting function.
#[derive(Clone, Default)]
pub enum AxisFormatter {
    /// Default numeric formatter.
    #[default]
    Default,
    /// Custom formatter callback.
    ///
    /// The function must be thread-safe because plots can be rendered from
    /// multiple contexts.
    Custom(Arc<dyn Fn(f64) -> String + Send + Sync>),
}

impl AxisFormatter {
    /// Format a value for display.
    pub fn format(&self, value: f64) -> String {
        match self {
            Self::Default => format!("{value:.6}"),
            Self::Custom(formatter) => formatter(value),
        }
    }
}

impl std::fmt::Debug for AxisFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "AxisFormatter::Default"),
            Self::Custom(_) => write!(f, "AxisFormatter::Custom(..)"),
        }
    }
}

/// Axis configuration shared across all series in a plot.
///
/// The axis configuration is owned by [`Plot`](crate::plot::Plot) and affects
/// all series within the plot. Each series contributes data only; axes control
/// scaling, ticks, formatting, and grid/border appearance.
#[derive(Debug, Clone)]
pub struct AxisConfig {
    title: Option<String>,
    units: Option<String>,
    formatter: AxisFormatter,
    tick_config: TickConfig,
    show_grid: bool,
    show_minor_grid: bool,
    show_zero_line: bool,
    show_border: bool,
    label_size: f32,
}

impl AxisConfig {
    /// Create a new axis configuration.
    ///
    /// Use [`AxisConfig::builder`] for a fluent configuration style.
    pub fn new() -> Self {
        Self {
            title: None,
            units: None,
            formatter: AxisFormatter::default(),
            tick_config: TickConfig::default(),
            show_grid: true,
            show_minor_grid: false,
            show_zero_line: false,
            show_border: true,
            label_size: 12.0,
        }
    }

    /// Start building an axis configuration.
    pub fn builder() -> AxisConfigBuilder {
        AxisConfigBuilder { axis: Self::new() }
    }

    /// Access the axis title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Access the axis units.
    pub fn units(&self) -> Option<&str> {
        self.units.as_deref()
    }

    /// Access the formatter.
    pub fn formatter(&self) -> &AxisFormatter {
        &self.formatter
    }

    /// Format a value for display using the configured formatter.
    pub fn format_value(&self, value: f64) -> String {
        self.formatter.format(value)
    }

    /// Access the tick configuration.
    pub fn tick_config(&self) -> TickConfig {
        self.tick_config
    }

    /// Check if major grid lines are enabled.
    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    /// Check if minor grid lines are enabled.
    pub fn show_minor_grid(&self) -> bool {
        self.show_minor_grid
    }

    /// Check if the zero line is enabled.
    pub fn show_zero_line(&self) -> bool {
        self.show_zero_line
    }

    /// Check if the axis border is enabled.
    pub fn show_border(&self) -> bool {
        self.show_border
    }

    /// Access the tick label font size.
    pub fn label_size(&self) -> f32 {
        self.label_size
    }
}

/// Builder for [`AxisConfig`].
#[derive(Debug, Clone)]
pub struct AxisConfigBuilder {
    axis: AxisConfig,
}

impl AxisConfigBuilder {
    /// Set the axis title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.axis.title = Some(title.into());
        self
    }

    /// Set the axis units.
    pub fn units(mut self, units: impl Into<String>) -> Self {
        self.axis.units = Some(units.into());
        self
    }

    /// Set the axis formatter.
    ///
    /// Custom formatters override the default numeric formatting.
    pub fn formatter(mut self, formatter: AxisFormatter) -> Self {
        self.axis.formatter = formatter;
        self
    }

    /// Set the tick configuration.
    ///
    /// The `pixel_spacing` hint determines how dense major ticks are.
    pub fn tick_config(mut self, config: TickConfig) -> Self {
        self.axis.tick_config = config;
        self
    }

    /// Enable or disable major grid lines.
    pub fn grid(mut self, enabled: bool) -> Self {
        self.axis.show_grid = enabled;
        self
    }

    /// Enable or disable minor grid lines.
    pub fn minor_grid(mut self, enabled: bool) -> Self {
        self.axis.show_minor_grid = enabled;
        self
    }

    /// Enable or disable the zero line.
    pub fn zero_line(mut self, enabled: bool) -> Self {
        self.axis.show_zero_line = enabled;
        self
    }

    /// Enable or disable the axis border.
    pub fn border(mut self, enabled: bool) -> Self {
        self.axis.show_border = enabled;
        self
    }

    /// Set the tick label font size.
    pub fn label_size(mut self, size: f32) -> Self {
        self.axis.label_size = size;
        self
    }

    /// Build the axis configuration.
    pub fn build(self) -> AxisConfig {
        self.axis
    }
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Tick generation configuration.
///
/// The tick generator uses `pixel_spacing` as a target distance between
/// major ticks and inserts `minor_count` minor ticks in between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickConfig {
    /// Target pixel spacing between major ticks.
    pub pixel_spacing: f32,
    /// Number of minor ticks between major ticks.
    pub minor_count: usize,
}

impl Default for TickConfig {
    fn default() -> Self {
        Self {
            pixel_spacing: 80.0,
            minor_count: 4,
        }
    }
}

/// Axis tick metadata.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tick {
    /// Tick value in data space.
    pub(crate) value: f64,
    /// Tick label.
    pub(crate) label: String,
    /// Whether the tick is a major tick.
    pub(crate) is_major: bool,
}

/// Layout information for axis labels and ticks.
#[derive(Debug, Clone)]
pub(crate) struct AxisLayout {
    /// Ticks to render.
    pub(crate) ticks: Vec<Tick>,
    /// Maximum tick label size (width, height).
    pub(crate) max_label_size: (f32, f32),
}

impl Default for AxisLayout {
    fn default() -> Self {
        Self {
            ticks: Vec::new(),
            max_label_size: (0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AxisLayoutKey {
    range: Range,
    pixels: u32,
    tick_config: TickConfig,
}

/// Cached layout for axis ticks and labels.
#[derive(Debug, Default, Clone)]
pub(crate) struct AxisLayoutCache {
    key: Option<AxisLayoutKey>,
    layout: AxisLayout,
}

impl AxisLayoutCache {
    /// Update the cache if inputs have changed.
    pub(crate) fn update(
        &mut self,
        axis: &AxisConfig,
        range: Range,
        pixels: u32,
        measurer: &impl TextMeasurer,
    ) -> &AxisLayout {
        let key = AxisLayoutKey {
            range,
            pixels,
            tick_config: axis.tick_config(),
        };
        if self.key.as_ref() == Some(&key) {
            return &self.layout;
        }

        let ticks = generate_ticks(axis, range, pixels as f32);
        let mut max_size = (0.0_f32, 0.0_f32);
        for tick in &ticks {
            if tick.label.is_empty() {
                continue;
            }
            let (w, h) = measurer.measure(&tick.label, axis.label_size());
            max_size.0 = max_size.0.max(w);
            max_size.1 = max_size.1.max(h);
        }

        self.layout = AxisLayout {
            ticks,
            max_label_size: max_size,
        };
        self.key = Some(key);
        &self.layout
    }
}

/// Text measurement interface for layout.
pub(crate) trait TextMeasurer {
    /// Measure a text label at the given size.
    fn measure(&self, text: &str, size: f32) -> (f32, f32);
}

/// Generate axis ticks for a range and pixel length.
fn generate_ticks(axis: &AxisConfig, range: Range, pixel_length: f32) -> Vec<Tick> {
    if !range.is_valid() || pixel_length <= 0.0 {
        return Vec::new();
    }
    generate_linear_ticks(axis, range, pixel_length)
}

fn generate_linear_ticks(axis: &AxisConfig, range: Range, pixel_length: f32) -> Vec<Tick> {
    let target = (pixel_length / axis.tick_config().pixel_spacing).max(2.0);
    let raw_step = range.span() / target as f64;
    let step = nice_step(raw_step);
    if !step.is_finite() || step <= 0.0 {
        return Vec::new();
    }

    let minor_count = axis.tick_config().minor_count;
    let minor_step = step / (minor_count as f64 + 1.0);

    let mut ticks = Vec::new();
    let mut value = (range.min / step).floor() * step;
    if value == -0.0 {
        value = 0.0;
    }
    let max_value = range.max + step * 0.5;

    while value <= max_value {
        if value >= range.min - step * 0.5 {
            ticks.push(Tick {
                value,
                label: axis.format_value(value),
                is_major: true,
            });
        }
        for i in 1..=minor_count {
            let minor = value + minor_step * i as f64;
            if minor >= range.min && minor <= range.max {
                ticks.push(Tick {
                    value: minor,
                    label: String::new(),
                    is_major: false,
                });
            }
        }
        value += step;
    }

    ticks
}

fn nice_step(step: f64) -> f64 {
    if step <= 0.0 {
        return 0.0;
    }
    let exp = step.log10().floor();
    let base = 10_f64.powf(exp);
    let fraction = step / base;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_ticks_generate_major() {
        let axis = AxisConfig::new();
        let ticks = generate_ticks(&axis, Range::new(0.0, 10.0), 400.0);
        assert!(ticks.iter().any(|tick| tick.is_major));
    }
}
