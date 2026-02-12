use gpui::{
    App, BorderStyle, Bounds, ContentMask, Corners, Edges, PathBuilder, Pixels, TextRun, Window,
    font, point, px, quad,
};

use crate::geom::{ScreenPoint, ScreenRect};
use crate::render::{
    Color, LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCommand, TextStyle,
};

use super::frame::PlotFrame;

pub(crate) fn paint_frame(frame: &PlotFrame, window: &mut Window, cx: &mut App) {
    let mut clip_stack: Vec<ContentMask<Pixels>> = Vec::new();
    for command in frame.render.commands() {
        match command {
            RenderCommand::ClipRect(rect) => {
                clip_stack.push(ContentMask {
                    bounds: to_bounds(*rect),
                });
            }
            RenderCommand::ClipEnd => {
                clip_stack.pop();
            }
            RenderCommand::LineSegments { segments, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_lines(window, segments, *style);
                });
            }
            RenderCommand::Points { points, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_points(window, points, *style);
                });
            }
            RenderCommand::Rect { rect, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_rect(window, *rect, *style);
                });
            }
            RenderCommand::Text {
                position,
                text,
                style,
            } => {
                with_clip(window, &clip_stack, |window| {
                    paint_text(window, cx, *position, text, style);
                });
            }
        }
    }
}

fn paint_lines(window: &mut Window, segments: &[LineSegment], style: LineStyle) {
    if segments.is_empty() {
        return;
    }
    let width = style.width.max(0.5);
    let mut builder = PathBuilder::stroke(px(width));
    for segment in segments {
        builder.move_to(point(px(segment.start.x), px(segment.start.y)));
        builder.line_to(point(px(segment.end.x), px(segment.end.y)));
    }
    if let Ok(path) = builder.build() {
        window.paint_path(path, to_rgba(style.color));
    }
}

fn paint_points(window: &mut Window, points: &[ScreenPoint], style: MarkerStyle) {
    if points.is_empty() {
        return;
    }

    let size = style.size.max(2.0);
    match style.shape {
        MarkerShape::Circle => {
            let radius = size * 0.5;
            for pt in points {
                let bounds = Bounds::from_corners(
                    point(px(pt.x - radius), px(pt.y - radius)),
                    point(px(pt.x + radius), px(pt.y + radius)),
                );
                window.paint_quad(quad(
                    bounds,
                    Corners::all(px(radius)),
                    to_rgba(style.color),
                    Edges::all(px(0.0)),
                    to_rgba(style.color),
                    BorderStyle::default(),
                ));
            }
        }
        MarkerShape::Square => {
            let half = size * 0.5;
            for pt in points {
                let bounds = Bounds::from_corners(
                    point(px(pt.x - half), px(pt.y - half)),
                    point(px(pt.x + half), px(pt.y + half)),
                );
                window.paint_quad(quad(
                    bounds,
                    Corners::all(px(0.0)),
                    to_rgba(style.color),
                    Edges::all(px(0.0)),
                    to_rgba(style.color),
                    BorderStyle::default(),
                ));
            }
        }
        MarkerShape::Cross => {
            let half = size * 0.5;
            let mut builder = PathBuilder::stroke(px(1.0));
            for pt in points {
                let h_start = point(px(pt.x - half), px(pt.y));
                let h_end = point(px(pt.x + half), px(pt.y));
                let v_start = point(px(pt.x), px(pt.y - half));
                let v_end = point(px(pt.x), px(pt.y + half));
                builder.move_to(h_start);
                builder.line_to(h_end);
                builder.move_to(v_start);
                builder.line_to(v_end);
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, to_rgba(style.color));
            }
        }
    }
}

fn paint_rect(window: &mut Window, rect: ScreenRect, style: RectStyle) {
    let bounds = to_bounds(rect);
    let quad = quad(
        bounds,
        Corners::all(px(0.0)),
        to_rgba(style.fill),
        Edges::all(px(style.stroke_width)),
        to_rgba(style.stroke),
        BorderStyle::default(),
    );
    window.paint_quad(quad);
}

fn paint_text(
    window: &mut Window,
    cx: &mut App,
    position: ScreenPoint,
    text: &str,
    style: &TextStyle,
) {
    if text.is_empty() {
        return;
    }
    let font_size = px(style.size);
    let run = TextRun {
        len: text.len(),
        font: font(".SystemUIFont"),
        color: to_hsla(style.color),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window
        .text_system()
        .shape_line(text.to_string().into(), font_size, &[run], None);
    let line_height = shaped.ascent + shaped.descent;
    let origin = point(px(position.x), px(position.y));
    let _ = shaped.paint(origin, line_height, window, cx);
}

fn to_rgba(color: Color) -> gpui::Rgba {
    gpui::Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

pub(crate) fn to_hsla(color: Color) -> gpui::Hsla {
    gpui::Hsla::from(to_rgba(color))
}

fn to_bounds(rect: ScreenRect) -> Bounds<Pixels> {
    Bounds::from_corners(
        point(px(rect.min.x), px(rect.min.y)),
        point(px(rect.max.x), px(rect.max.y)),
    )
}

fn with_clip(window: &mut Window, stack: &[ContentMask<Pixels>], f: impl FnOnce(&mut Window)) {
    if let Some(mask) = stack.last() {
        window.with_content_mask(Some(mask.clone()), f);
    } else {
        f(window);
    }
}
