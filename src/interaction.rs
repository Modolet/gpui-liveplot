//! Interaction helpers for panning, zooming, and pin selection.

use crate::geom::{Point, ScreenPoint, ScreenRect};
use crate::series::{Series, SeriesId};
use crate::transform::Transform;
use crate::view::{Range, Viewport};

/// Interaction hit regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitRegion {
    /// Plot data area.
    Plot,
    /// X axis area.
    XAxis,
    /// Y axis area.
    YAxis,
    /// Outside of the plot.
    Outside,
}

/// Screen regions for hit testing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotRegions {
    /// Plot data area.
    pub plot: ScreenRect,
    /// X axis area.
    pub x_axis: ScreenRect,
    /// Y axis area.
    pub y_axis: ScreenRect,
}

impl PlotRegions {
    /// Determine which region contains the point.
    pub fn hit_test(&self, point: ScreenPoint) -> HitRegion {
        if contains(self.plot, point) {
            HitRegion::Plot
        } else if contains(self.x_axis, point) {
            HitRegion::XAxis
        } else if contains(self.y_axis, point) {
            HitRegion::YAxis
        } else {
            HitRegion::Outside
        }
    }
}

/// Pin binding to a stable point identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    /// Series identifier.
    pub series_id: SeriesId,
    /// Point index within the series.
    pub point_index: usize,
}

/// Pin hit information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PinHit {
    /// Pin identity.
    pub pin: Pin,
    /// Distance squared in screen pixels.
    pub distance_sq: f32,
}

/// Toggle a pin in the list. Returns true if added, false if removed.
pub fn toggle_pin(pins: &mut Vec<Pin>, pin: Pin) -> bool {
    if let Some(index) = pins.iter().position(|existing| *existing == pin) {
        pins.swap_remove(index);
        false
    } else {
        pins.push(pin);
        true
    }
}

/// Pan a viewport by a pixel delta.
pub fn pan_viewport(
    viewport: Viewport,
    delta_pixels: ScreenPoint,
    transform: &Transform,
) -> Option<Viewport> {
    let origin = transform.screen_to_data(ScreenPoint::new(0.0, 0.0))?;
    let shifted = transform.screen_to_data(ScreenPoint::new(delta_pixels.x, delta_pixels.y))?;
    let dx = shifted.x - origin.x;
    let dy = shifted.y - origin.y;
    Some(Viewport::new(
        Range::new(viewport.x.min - dx, viewport.x.max - dx),
        Range::new(viewport.y.min - dy, viewport.y.max - dy),
    ))
}

/// Zoom a viewport around a center point.
pub fn zoom_viewport(viewport: Viewport, center: Point, factor_x: f64, factor_y: f64) -> Viewport {
    let x_min = center.x + (viewport.x.min - center.x) * factor_x;
    let x_max = center.x + (viewport.x.max - center.x) * factor_x;
    let y_min = center.y + (viewport.y.min - center.y) * factor_y;
    let y_max = center.y + (viewport.y.max - center.y) * factor_y;
    Viewport::new(Range::new(x_min, x_max), Range::new(y_min, y_max))
}

/// Convert a zoom rectangle into a new viewport.
pub fn zoom_to_rect(
    viewport: Viewport,
    rect: ScreenRect,
    transform: &Transform,
) -> Option<Viewport> {
    if rect.width().abs() < 2.0 || rect.height().abs() < 2.0 {
        return Some(viewport);
    }
    let data_min = transform.screen_to_data(rect.min)?;
    let data_max = transform.screen_to_data(rect.max)?;
    Some(Viewport::new(
        Range::new(data_min.x, data_max.x),
        Range::new(data_min.y, data_max.y),
    ))
}

/// Compute a zoom factor from a drag delta and axis length.
pub fn zoom_factor_from_drag(delta_pixels: f32, axis_pixels: f32) -> f64 {
    if axis_pixels <= 0.0 {
        return 1.0;
    }
    let normalized = delta_pixels as f64 / axis_pixels as f64;
    (1.0 - normalized).clamp(0.1, 10.0)
}

/// Find the nearest point to the cursor within the threshold.
pub fn find_nearest_point(
    series: &[Series],
    transform: &Transform,
    cursor: ScreenPoint,
    threshold: f32,
) -> Option<PinHit> {
    let center = transform.screen_to_data(cursor)?;
    let edge = transform.screen_to_data(ScreenPoint::new(cursor.x + threshold, cursor.y))?;
    let dx = (edge.x - center.x).abs();
    let search_range = Range::new(center.x - dx, center.x + dx);

    let mut best: Option<PinHit> = None;
    let threshold_sq = threshold * threshold;

    for series in series {
        if !series.is_visible() {
            continue;
        }
        let data = series.data().data();
        let index_range = data.range_by_x(search_range);
        for index in index_range {
            let Some(point) = data.point(index) else {
                continue;
            };
            let Some(screen) = transform.data_to_screen(point) else {
                continue;
            };
            let dx = screen.x - cursor.x;
            let dy = screen.y - cursor.y;
            let distance_sq = dx * dx + dy * dy;
            if distance_sq > threshold_sq {
                continue;
            }
            let hit = PinHit {
                pin: Pin {
                    series_id: series.id(),
                    point_index: index,
                },
                distance_sq,
            };
            if best.is_none_or(|best| hit.distance_sq < best.distance_sq) {
                best = Some(hit);
            }
        }
    }

    best
}

fn contains(rect: ScreenRect, point: ScreenPoint) -> bool {
    point.x >= rect.min.x && point.x <= rect.max.x && point.y >= rect.min.y && point.y <= rect.max.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axis::AxisScale;
    use crate::geom::Point;
    use crate::series::SeriesKind;
    use crate::transform::Transform;

    #[test]
    fn hit_test_regions() {
        let regions = PlotRegions {
            plot: ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(10.0, 10.0)),
            x_axis: ScreenRect::new(ScreenPoint::new(0.0, 10.0), ScreenPoint::new(10.0, 12.0)),
            y_axis: ScreenRect::new(ScreenPoint::new(-2.0, 0.0), ScreenPoint::new(0.0, 10.0)),
        };
        assert_eq!(
            regions.hit_test(ScreenPoint::new(5.0, 5.0)),
            HitRegion::Plot
        );
        assert_eq!(
            regions.hit_test(ScreenPoint::new(5.0, 11.0)),
            HitRegion::XAxis
        );
        assert_eq!(
            regions.hit_test(ScreenPoint::new(-1.0, 5.0)),
            HitRegion::YAxis
        );
    }

    #[test]
    fn nearest_point_detects_hit() {
        let series = Series::from_iter_points(
            "test",
            [Point::new(0.0, 0.0), Point::new(1.0, 1.0)],
            SeriesKind::Line(crate::render::LineStyle::default()),
        );
        let viewport = Viewport::new(Range::new(0.0, 1.0), Range::new(0.0, 1.0));
        let rect = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(100.0, 100.0));
        let transform =
            Transform::new(viewport, rect, AxisScale::Linear, AxisScale::Linear).unwrap();
        let cursor = ScreenPoint::new(0.0, 100.0);
        let hit = find_nearest_point(&[series], &transform, cursor, 5.0);
        assert!(hit.is_some());
    }
}
