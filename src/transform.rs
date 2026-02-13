//! Coordinate transforms between data and screen space.

use crate::axis::AxisScale;
use crate::geom::{Point, ScreenPoint, ScreenRect};
use crate::view::{Range, Viewport};

const MIN_SPAN: f64 = 1e-12;

/// Transform from data coordinates into screen coordinates.
#[derive(Debug, Clone)]
pub(crate) struct Transform {
    viewport: Viewport,
    screen: ScreenRect,
    x_scale: AxisScale,
    y_scale: AxisScale,
    x_axis: Range,
    y_axis: Range,
}

impl Transform {
    /// Create a transform for the given viewport and screen rectangle.
    pub(crate) fn new(
        viewport: Viewport,
        screen: ScreenRect,
        x_scale: AxisScale,
        y_scale: AxisScale,
    ) -> Option<Self> {
        if !screen.is_valid() {
            return None;
        }
        let x_axis = map_range(viewport.x.with_min_span(MIN_SPAN), x_scale)?;
        let y_axis = map_range(viewport.y.with_min_span(MIN_SPAN), y_scale)?;
        Some(Self {
            viewport,
            screen,
            x_scale,
            y_scale,
            x_axis,
            y_axis,
        })
    }

    /// Access the viewport.
    pub(crate) fn viewport(&self) -> Viewport {
        self.viewport
    }

    /// Access the screen rectangle.
    pub(crate) fn screen(&self) -> ScreenRect {
        self.screen
    }

    /// Map a data point into screen space.
    pub(crate) fn data_to_screen(&self, point: Point) -> Option<ScreenPoint> {
        let x = self.x_scale.map_value(point.x)?;
        let y = self.y_scale.map_value(point.y)?;
        let x_norm = (x - self.x_axis.min) / self.x_axis.span();
        let y_norm = (y - self.y_axis.min) / self.y_axis.span();
        let sx = self.screen.min.x as f64 + x_norm * self.screen.width() as f64;
        let sy = self.screen.max.y as f64 - y_norm * self.screen.height() as f64;
        Some(ScreenPoint::new(sx as f32, sy as f32))
    }

    /// Map a screen point into data space.
    pub(crate) fn screen_to_data(&self, point: ScreenPoint) -> Option<Point> {
        let x_norm = (point.x as f64 - self.screen.min.x as f64) / self.screen.width() as f64;
        let y_norm = (self.screen.max.y as f64 - point.y as f64) / self.screen.height() as f64;
        let x_axis = self.x_axis.min + x_norm * self.x_axis.span();
        let y_axis = self.y_axis.min + y_norm * self.y_axis.span();
        let x = self.x_scale.invert_value(x_axis)?;
        let y = self.y_scale.invert_value(y_axis)?;
        Some(Point::new(x, y))
    }
}

fn map_range(range: Range, scale: AxisScale) -> Option<Range> {
    let min = scale.map_value(range.min)?;
    let max = scale.map_value(range.max)?;
    Some(Range::new(min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axis::AxisScale;

    #[test]
    fn linear_roundtrip() {
        let viewport = Viewport::new(Range::new(0.0, 10.0), Range::new(0.0, 10.0));
        let screen = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(100.0, 100.0));
        let transform = Transform::new(viewport, screen, AxisScale::Linear, AxisScale::Linear)
            .expect("valid transform");
        let point = Point::new(5.0, 7.5);
        let screen_point = transform.data_to_screen(point).unwrap();
        let roundtrip = transform.screen_to_data(screen_point).unwrap();
        assert!((roundtrip.x - point.x).abs() < 1e-9);
        assert!((roundtrip.y - point.y).abs() < 1e-9);
    }

    #[test]
    fn log_rejects_non_positive_range() {
        let viewport = Viewport::new(Range::new(-1.0, 10.0), Range::new(1.0, 10.0));
        let screen = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(100.0, 100.0));
        let transform = Transform::new(viewport, screen, AxisScale::Log10, AxisScale::Linear);
        assert!(transform.is_none());
    }
}
