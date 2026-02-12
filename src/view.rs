//! View models and data ranges.

/// Numeric range with inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
}

impl Range {
    /// Create a new range, swapping bounds if needed.
    pub fn new(mut min: f64, mut max: f64) -> Self {
        if min > max {
            std::mem::swap(&mut min, &mut max);
        }
        Self { min, max }
    }

    /// Span of the range.
    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    /// Check whether both bounds are finite.
    pub fn is_finite(&self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }

    /// Check whether the range has positive span and finite bounds.
    pub fn is_valid(&self) -> bool {
        self.is_finite() && self.span() > 0.0
    }

    /// Expand the range to include a value.
    pub fn expand_to_include(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    /// Union two ranges if both are finite.
    pub fn union(a: Self, b: Self) -> Option<Self> {
        if !a.is_finite() || !b.is_finite() {
            return None;
        }
        Some(Self {
            min: a.min.min(b.min),
            max: a.max.max(b.max),
        })
    }

    /// Clamp a value into the range.
    pub fn clamp(&self, value: f64) -> f64 {
        value.max(self.min).min(self.max)
    }

    /// Add padding around the range.
    pub fn padded(&self, frac: f64, min_padding: f64) -> Self {
        let span = self.span().abs();
        let padding = (span * frac).max(min_padding);
        Self {
            min: self.min - padding,
            max: self.max + padding,
        }
    }

    /// Ensure the range has at least the given span.
    pub fn with_min_span(&self, min_span: f64) -> Self {
        let span = self.span();
        if span >= min_span {
            return *self;
        }
        let center = (self.min + self.max) * 0.5;
        let half = min_span * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }
}

/// The active view mode for a plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Automatically show the full data range (default).
    AutoAll {
        /// Allow automatic X range expansion.
        auto_x: bool,
        /// Allow automatic Y range expansion.
        auto_y: bool,
    },
    /// Manual view that does not auto-update.
    Manual,
    /// Follow the last N points on X.
    FollowLastN {
        /// Number of points to keep in view.
        points: usize,
    },
    /// Follow the last N points on X and auto-scale Y.
    FollowLastNXY {
        /// Number of points to keep in view.
        points: usize,
    },
}

impl Default for View {
    fn default() -> Self {
        Self::AutoAll {
            auto_x: true,
            auto_y: true,
        }
    }
}

/// Visible data ranges on both axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// X axis range.
    pub x: Range,
    /// Y axis range.
    pub y: Range,
}

impl Viewport {
    /// Create a viewport from X and Y ranges.
    pub fn new(x: Range, y: Range) -> Self {
        Self { x, y }
    }

    /// Check whether both axes are valid.
    pub fn is_valid(&self) -> bool {
        self.x.is_valid() && self.y.is_valid()
    }

    /// Apply padding to both axes.
    pub fn padded(&self, frac: f64, min_padding: f64) -> Self {
        Self {
            x: self.x.padded(frac, min_padding),
            y: self.y.padded(frac, min_padding),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_with_min_span_expands() {
        let range = Range::new(2.0, 2.0);
        let expanded = range.with_min_span(1.0);
        assert!(expanded.span() >= 1.0);
        assert!((expanded.min + expanded.max) * 0.5 - 2.0 < 1e-9);
    }
}
