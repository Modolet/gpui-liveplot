//! Axis configuration, scaling, and formatting.

use std::sync::Arc;

use crate::view::Range;

/// Axis scale type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisScale {
    /// Linear scaling.
    Linear,
    /// Base-10 logarithmic scaling.
    Log10,
    /// Time axis (mapped as linear values internally).
    Time,
}

impl AxisScale {
    /// Map a value into axis space.
    pub fn map_value(self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        match self {
            Self::Linear | Self::Time => Some(value),
            Self::Log10 => {
                if value <= 0.0 {
                    None
                } else {
                    Some(value.log10())
                }
            }
        }
    }

    /// Invert a value from axis space back into data space.
    pub fn invert_value(self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        match self {
            Self::Linear | Self::Time => Some(value),
            Self::Log10 => Some(10_f64.powf(value)),
        }
    }

    /// Check whether a data range is valid for this scale.
    pub fn is_range_valid(self, range: Range) -> bool {
        if !range.is_finite() {
            return false;
        }
        match self {
            Self::Linear | Self::Time => true,
            Self::Log10 => range.min > 0.0 && range.max > 0.0,
        }
    }
}

/// Formatter for axis tick labels.
#[derive(Clone, Default)]
pub enum AxisFormatter {
    /// Default numeric formatter.
    #[default]
    Default,
    /// Custom formatter callback.
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
#[derive(Debug, Clone)]
pub struct AxisConfig {
    scale: AxisScale,
    title: Option<String>,
    units: Option<String>,
    formatter: AxisFormatter,
}

impl AxisConfig {
    /// Create a new axis configuration.
    pub fn new(scale: AxisScale) -> Self {
        Self {
            scale,
            title: None,
            units: None,
            formatter: AxisFormatter::default(),
        }
    }

    /// Create a linear axis configuration.
    pub fn linear() -> Self {
        Self::new(AxisScale::Linear)
    }

    /// Create a log10 axis configuration.
    pub fn log10() -> Self {
        Self::new(AxisScale::Log10)
    }

    /// Create a time axis configuration.
    pub fn time() -> Self {
        Self::new(AxisScale::Time)
    }

    /// Access the axis scale.
    pub fn scale(&self) -> AxisScale {
        self.scale
    }

    /// Set the axis scale.
    pub fn with_scale(mut self, scale: AxisScale) -> Self {
        self.scale = scale;
        self
    }

    /// Set the axis title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the axis units.
    pub fn with_units(mut self, units: impl Into<String>) -> Self {
        self.units = Some(units.into());
        self
    }

    /// Set the axis formatter.
    pub fn with_formatter(mut self, formatter: AxisFormatter) -> Self {
        self.formatter = formatter;
        self
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
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self::linear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_scale_rejects_non_positive() {
        let scale = AxisScale::Log10;
        assert!(scale.map_value(0.0).is_none());
        assert!(scale.map_value(-1.0).is_none());
        assert!(scale.map_value(1.0).is_some());
    }

    #[test]
    fn log_scale_roundtrip() {
        let scale = AxisScale::Log10;
        let value = 1000.0;
        let mapped = scale.map_value(value).unwrap();
        let roundtrip = scale.invert_value(mapped).unwrap();
        assert!((roundtrip - value).abs() < 1e-9);
    }
}
