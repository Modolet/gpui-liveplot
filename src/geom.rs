//! Geometric primitives used by the plotting pipeline.
//!
//! Public types in this module represent data-space coordinates. Screen-space
//! types are internal to render backends.

/// A point in data space.
///
/// Use this when providing explicit X/Y values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X value in data coordinates.
    pub x: f64,
    /// Y value in data coordinates.
    pub y: f64,
}

impl Point {
    /// Create a new data point.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A point in screen space (pixel coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScreenPoint {
    /// X value in screen pixels.
    pub(crate) x: f32,
    /// Y value in screen pixels.
    pub(crate) y: f32,
}

impl ScreenPoint {
    /// Create a new screen point.
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A rectangle in screen space (pixel coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScreenRect {
    /// Top-left corner.
    pub(crate) min: ScreenPoint,
    /// Bottom-right corner.
    pub(crate) max: ScreenPoint,
}

impl ScreenRect {
    /// Create a new screen rectangle from corners.
    pub(crate) fn new(min: ScreenPoint, max: ScreenPoint) -> Self {
        Self { min, max }
    }

    /// Rectangle width in pixels.
    pub(crate) fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Rectangle height in pixels.
    pub(crate) fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Check whether the rectangle has positive area.
    pub(crate) fn is_valid(&self) -> bool {
        self.width() > 0.0 && self.height() > 0.0
    }
}
