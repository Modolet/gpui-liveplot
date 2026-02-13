//! Rendering primitives and clipping helpers.
//!
//! These types are backend-agnostic and are used by render backends (such as the
//! GPUI backend) to describe how plots should be drawn.

use crate::geom::{Point, ScreenPoint, ScreenRect};
use crate::transform::Transform;
use crate::view::Viewport;

/// RGBA color in linear space.
///
/// All components are expected to be in the 0.0..=1.0 range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
    /// Alpha channel.
    pub a: f32,
}

impl Color {
    /// Create a new color.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque black.
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// Opaque white.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
}

/// Line stroke styling.
///
/// The width is expressed in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineStyle {
    /// Stroke color.
    pub color: Color,
    /// Stroke width in pixels.
    pub width: f32,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            width: 1.0,
        }
    }
}

/// Marker shape for scatter plots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerShape {
    /// Circle marker.
    Circle,
    /// Square marker.
    Square,
    /// Cross marker.
    Cross,
}

/// Marker styling for scatter plots.
///
/// Marker sizes are expressed in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkerStyle {
    /// Marker color.
    pub color: Color,
    /// Marker size in pixels.
    pub size: f32,
    /// Marker shape.
    pub shape: MarkerShape,
}

impl Default for MarkerStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            size: 4.0,
            shape: MarkerShape::Circle,
        }
    }
}

/// Rectangle styling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RectStyle {
    /// Fill color.
    pub fill: Color,
    /// Stroke color.
    pub stroke: Color,
    /// Stroke width.
    pub stroke_width: f32,
}

impl Default for RectStyle {
    fn default() -> Self {
        Self {
            fill: Color::new(0.0, 0.0, 0.0, 0.0),
            stroke: Color::BLACK,
            stroke_width: 1.0,
        }
    }
}

/// Text styling.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextStyle {
    /// Text color.
    pub color: Color,
    /// Font size in pixels.
    pub size: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            size: 12.0,
        }
    }
}

/// A line segment in screen space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineSegment {
    /// Segment start.
    pub start: ScreenPoint,
    /// Segment end.
    pub end: ScreenPoint,
}

impl LineSegment {
    /// Create a new line segment.
    pub(crate) fn new(start: ScreenPoint, end: ScreenPoint) -> Self {
        Self { start, end }
    }
}

/// Render command list.
#[derive(Debug, Clone)]
pub(crate) enum RenderCommand {
    /// Start clipping to a rectangle.
    ClipRect(ScreenRect),
    /// End clipping.
    ClipEnd,
    /// Draw line segments.
    LineSegments {
        /// Segments to draw.
        segments: Vec<LineSegment>,
        /// Styling for the segments.
        style: LineStyle,
    },
    /// Draw scatter points.
    Points {
        /// Points to draw.
        points: Vec<ScreenPoint>,
        /// Marker styling.
        style: MarkerStyle,
    },
    /// Draw a rectangle.
    Rect {
        /// Rectangle bounds.
        rect: ScreenRect,
        /// Rectangle styling.
        style: RectStyle,
    },
    /// Draw text.
    Text {
        /// Text position.
        position: ScreenPoint,
        /// Text content.
        text: String,
        /// Text styling.
        style: TextStyle,
    },
}

/// Aggregated render commands.
#[derive(Debug, Default, Clone)]
pub(crate) struct RenderList {
    commands: Vec<RenderCommand>,
}

impl RenderList {
    /// Create an empty render list.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Push a render command.
    pub(crate) fn push(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    /// Access all render commands.
    pub(crate) fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }
}

/// Cache key for rendered series data.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RenderCacheKey {
    /// Viewport used for decimation.
    pub viewport: Viewport,
    /// Plot size in pixels.
    pub size: (u32, u32),
    /// Data generation for cache invalidation.
    pub generation: u64,
}

/// Build clipped line segments from data points.
pub(crate) fn build_line_segments(
    points: &[Point],
    transform: &Transform,
    clip: ScreenRect,
    out: &mut Vec<LineSegment>,
) {
    out.clear();
    if points.len() < 2 {
        return;
    }
    for window in points.windows(2) {
        let Some(start) = transform.data_to_screen(window[0]) else {
            continue;
        };
        let Some(end) = transform.data_to_screen(window[1]) else {
            continue;
        };
        if let Some((clipped_start, clipped_end)) = clip_segment(start, end, clip) {
            out.push(LineSegment::new(clipped_start, clipped_end));
        }
    }
}

/// Build clipped scatter points from data points.
pub(crate) fn build_scatter_points(
    points: &[Point],
    transform: &Transform,
    clip: ScreenRect,
    out: &mut Vec<ScreenPoint>,
) {
    out.clear();
    for point in points {
        let Some(screen) = transform.data_to_screen(*point) else {
            continue;
        };
        if screen.x >= clip.min.x
            && screen.x <= clip.max.x
            && screen.y >= clip.min.y
            && screen.y <= clip.max.y
        {
            out.push(screen);
        }
    }
}

fn clip_segment(
    mut start: ScreenPoint,
    mut end: ScreenPoint,
    rect: ScreenRect,
) -> Option<(ScreenPoint, ScreenPoint)> {
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const TOP: u8 = 4;
    const BOTTOM: u8 = 8;

    let mut out_start = region_code(start, rect, LEFT, RIGHT, TOP, BOTTOM);
    let mut out_end = region_code(end, rect, LEFT, RIGHT, TOP, BOTTOM);

    loop {
        if (out_start | out_end) == 0 {
            return Some((start, end));
        }
        if (out_start & out_end) != 0 {
            return None;
        }

        let out_code = if out_start != 0 { out_start } else { out_end };
        let (mut x, mut y) = (0.0_f32, 0.0_f32);

        if (out_code & TOP) != 0 {
            x = start.x + (end.x - start.x) * (rect.min.y - start.y) / (end.y - start.y);
            y = rect.min.y;
        } else if (out_code & BOTTOM) != 0 {
            x = start.x + (end.x - start.x) * (rect.max.y - start.y) / (end.y - start.y);
            y = rect.max.y;
        } else if (out_code & RIGHT) != 0 {
            y = start.y + (end.y - start.y) * (rect.max.x - start.x) / (end.x - start.x);
            x = rect.max.x;
        } else if (out_code & LEFT) != 0 {
            y = start.y + (end.y - start.y) * (rect.min.x - start.x) / (end.x - start.x);
            x = rect.min.x;
        }

        let new_point = ScreenPoint::new(x, y);
        if out_code == out_start {
            start = new_point;
            out_start = region_code(start, rect, LEFT, RIGHT, TOP, BOTTOM);
        } else {
            end = new_point;
            out_end = region_code(end, rect, LEFT, RIGHT, TOP, BOTTOM);
        }
    }
}

fn region_code(
    point: ScreenPoint,
    rect: ScreenRect,
    left: u8,
    right: u8,
    top: u8,
    bottom: u8,
) -> u8 {
    let mut code = 0;
    if point.x < rect.min.x {
        code |= left;
    } else if point.x > rect.max.x {
        code |= right;
    }
    if point.y < rect.min.y {
        code |= top;
    } else if point.y > rect.max.y {
        code |= bottom;
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;
    use crate::view::Range;
    use crate::view::Viewport;

    #[test]
    fn clip_segment_inside() {
        let rect = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(10.0, 10.0));
        let start = ScreenPoint::new(2.0, 2.0);
        let end = ScreenPoint::new(8.0, 8.0);
        let clipped = clip_segment(start, end, rect).expect("segment should clip");
        assert_eq!(clipped.0, start);
        assert_eq!(clipped.1, end);
    }

    #[test]
    fn build_segments_with_transform() {
        let viewport = Viewport::new(Range::new(0.0, 1.0), Range::new(0.0, 1.0));
        let rect = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(10.0, 10.0));
        let transform = Transform::new(viewport, rect).expect("valid transform");
        let points = [Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        let mut out = Vec::new();
        build_line_segments(&points, &transform, rect, &mut out);
        assert_eq!(out.len(), 1);
    }
}
