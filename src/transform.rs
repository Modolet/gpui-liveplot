//! Coordinate transforms between data and screen space.
use crate::geom::{Point, ScreenPoint, ScreenRect};
use crate::view::{Range, Viewport};

const MIN_SPAN: f64 = 1e-12;

/// Transform from data coordinates into screen coordinates.
#[derive(Debug, Clone)]
pub(crate) struct Transform {
    viewport: Viewport,
    screen: ScreenRect,
    x_axis: Range,
    y_axis: Range,
}

impl Transform {
    /// Create a transform for the given viewport and screen rectangle.
    pub(crate) fn new(viewport: Viewport, screen: ScreenRect) -> Option<Self> {
        if !screen.is_valid() {
            return None;
        }
        let x_axis = map_range(viewport.x.with_min_span(MIN_SPAN))?;
        let y_axis = map_range(viewport.y.with_min_span(MIN_SPAN))?;
        Some(Self {
            viewport,
            screen,
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
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let x_norm = (point.x - self.x_axis.min) / self.x_axis.span();
        let y_norm = (point.y - self.y_axis.min) / self.y_axis.span();
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
        Some(Point::new(x_axis, y_axis))
    }
}

fn map_range(range: Range) -> Option<Range> {
    if !range.is_finite() {
        return None;
    }
    Some(range)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_roundtrip() {
        let viewport = Viewport::new(Range::new(0.0, 10.0), Range::new(0.0, 10.0));
        let screen = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(100.0, 100.0));
        let transform = Transform::new(viewport, screen).expect("valid transform");
        let point = Point::new(5.0, 7.5);
        let screen_point = transform.data_to_screen(point).unwrap();
        let roundtrip = transform.screen_to_data(screen_point).unwrap();
        assert!((roundtrip.x - point.x).abs() < 1e-9);
        assert!((roundtrip.y - point.y).abs() < 1e-9);
    }
}
