use crate::geom::{ScreenPoint, ScreenRect};

pub(crate) fn normalized_rect(rect: ScreenRect) -> ScreenRect {
    let min_x = rect.min.x.min(rect.max.x);
    let max_x = rect.min.x.max(rect.max.x);
    let min_y = rect.min.y.min(rect.max.y);
    let max_y = rect.min.y.max(rect.max.y);
    ScreenRect::new(
        ScreenPoint::new(min_x, min_y),
        ScreenPoint::new(max_x, max_y),
    )
}

pub(crate) fn rect_contains(rect: ScreenRect, point: ScreenPoint) -> bool {
    point.x >= rect.min.x && point.x <= rect.max.x && point.y >= rect.min.y && point.y <= rect.max.y
}

pub(crate) fn distance_sq(a: ScreenPoint, b: ScreenPoint) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

pub(crate) fn clamp_point(point: ScreenPoint, rect: ScreenRect, size: (f32, f32)) -> ScreenPoint {
    let mut x = point.x;
    let mut y = point.y;
    if x < rect.min.x {
        x = rect.min.x;
    }
    if y < rect.min.y {
        y = rect.min.y;
    }
    if x + size.0 > rect.max.x {
        x = rect.max.x - size.0;
    }
    if y + size.1 > rect.max.y {
        y = rect.max.y - size.1;
    }
    ScreenPoint::new(x, y)
}

pub(crate) fn rect_intersects(a: ScreenRect, b: ScreenRect) -> bool {
    !(a.max.x <= b.min.x || a.min.x >= b.max.x || a.max.y <= b.min.y || a.min.y >= b.max.y)
}

pub(crate) fn rect_intersects_any(rect: ScreenRect, others: &[ScreenRect]) -> bool {
    others.iter().any(|other| rect_intersects(rect, *other))
}
