use std::cmp::Ordering;

use gpui::{Bounds, Pixels, Window};

use crate::axis::{AxisConfig, AxisLayout, TextMeasurer};
use crate::geom::{Point as DataPoint, ScreenPoint, ScreenRect};
use crate::plot::Plot;
use crate::render::{
    Color, LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCacheKey,
    RenderCommand, RenderList, TextStyle, build_line_segments, build_scatter_points,
};
use crate::series::{Series, SeriesKind};
use crate::style::Theme;
use crate::transform::Transform;
use crate::view::{Range, Viewport};

use super::config::PlotViewConfig;
use super::constants::*;
use super::geometry::{
    clamp_point, distance_sq, normalized_rect, rect_intersects, rect_intersects_any,
};
use super::hover::update_hover_target;
use super::state::{LegendEntry, LegendLayout, PlotUiState};
use super::text::GpuiTextMeasurer;

#[derive(Debug, Clone)]
pub(crate) struct PlotFrame {
    pub(crate) render: RenderList,
}

pub(crate) fn build_frame(
    plot: &mut Plot,
    state: &mut PlotUiState,
    config: &PlotViewConfig,
    bounds: Bounds<Pixels>,
    window: &Window,
) -> PlotFrame {
    let mut render = RenderList::new();

    let full_width = f32::from(bounds.size.width);
    let full_height = f32::from(bounds.size.height);
    if full_width <= 1.0 || full_height <= 1.0 {
        return PlotFrame { render };
    }

    let viewport = plot
        .refresh_viewport(config.padding_frac, config.min_padding)
        .unwrap_or_else(|| Viewport::new(Range::new(0.0, 1.0), Range::new(0.0, 1.0)));

    state.viewport = Some(viewport);

    let measurer = GpuiTextMeasurer::new(window);

    let mut plot_width = full_width;
    let mut plot_height = full_height;

    let x_layout = state
        .x_layout
        .update(plot.x_axis(), viewport.x, plot_width as u32, &measurer)
        .clone();
    let y_layout = state
        .y_layout
        .update(plot.y_axis(), viewport.y, plot_height as u32, &measurer)
        .clone();

    let x_title = axis_title_text(plot.x_axis());
    let x_title_size = x_title
        .as_ref()
        .map(|title| measurer.measure(title, plot.x_axis().label_size()))
        .unwrap_or((0.0, 0.0));

    let x_axis_height =
        x_layout.max_label_size.1 + TICK_LENGTH_MAJOR + AXIS_PADDING * 2.0 + x_title_size.1;
    let y_axis_width = y_layout.max_label_size.0 + TICK_LENGTH_MAJOR + AXIS_PADDING * 2.0;

    let x_axis_height = x_axis_height.clamp(0.0, full_height - 1.0);
    let y_axis_width = y_axis_width.clamp(0.0, full_width - 1.0);

    plot_width = (full_width - y_axis_width).max(1.0);
    plot_height = (full_height - x_axis_height).max(1.0);

    let x_layout = state
        .x_layout
        .update(plot.x_axis(), viewport.x, plot_width as u32, &measurer)
        .clone();
    let y_layout = state
        .y_layout
        .update(plot.y_axis(), viewport.y, plot_height as u32, &measurer)
        .clone();

    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let full_max_x = origin_x + full_width;
    let full_max_y = origin_y + full_height;

    let plot_rect = ScreenRect::new(
        ScreenPoint::new(origin_x + y_axis_width, origin_y),
        ScreenPoint::new(full_max_x, full_max_y - x_axis_height),
    );
    let x_axis_rect = ScreenRect::new(
        ScreenPoint::new(plot_rect.min.x, plot_rect.max.y),
        ScreenPoint::new(plot_rect.max.x, full_max_y),
    );
    let y_axis_rect = ScreenRect::new(
        ScreenPoint::new(origin_x, plot_rect.min.y),
        ScreenPoint::new(plot_rect.min.x, plot_rect.max.y),
    );

    state.regions = crate::interaction::PlotRegions {
        plot: plot_rect,
        x_axis: x_axis_rect,
        y_axis: y_axis_rect,
    };
    state.plot_rect = Some(plot_rect);

    let transform = Transform::new(viewport, plot_rect);
    state.transform = transform.clone();

    if let Some(transform) = transform {
        build_grid(
            &mut render,
            plot,
            &x_layout,
            &y_layout,
            &transform,
            plot_rect,
        );
        build_series(&mut render, plot, state, &transform, plot_rect);
        build_selection(&mut render, plot, state);
        update_hover_target(
            plot,
            state,
            &transform,
            plot_rect,
            config.pin_threshold_px,
            config.unpin_threshold_px,
        );
        build_pins(&mut render, plot, &transform, plot_rect, &measurer);
        build_axes(
            &mut render,
            plot,
            &x_layout,
            &y_layout,
            plot_rect,
            &transform,
            x_axis_rect,
            y_axis_rect,
            &measurer,
        );
        if config.show_hover {
            build_hover(&mut render, plot, state, &transform, plot_rect, &measurer);
        }
        if config.show_legend {
            build_legend(&mut render, plot, state, plot_rect, &measurer);
        } else {
            state.legend_layout = None;
        }
        build_axis_titles(
            &mut render,
            plot,
            plot_rect,
            x_axis_rect,
            y_axis_rect,
            &measurer,
        );
    } else {
        state.legend_layout = None;
        let message = "Invalid axis range";
        let size = measurer.measure(message, 14.0);
        let pos = ScreenPoint::new(
            plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
            plot_rect.min.y + (plot_rect.height() - size.1) * 0.5,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: message.to_string(),
            style: TextStyle {
                color: plot.theme().axis,
                size: 14.0,
            },
        });
    }

    PlotFrame { render }
}

fn build_grid(
    render: &mut RenderList,
    plot: &Plot,
    x_layout: &AxisLayout,
    y_layout: &AxisLayout,
    transform: &Transform,
    plot_rect: ScreenRect,
) {
    let theme = plot.theme();
    let mut major = Vec::new();
    let mut minor = Vec::new();

    if plot.x_axis().show_grid() {
        for tick in &x_layout.ticks {
            let x = transform
                .data_to_screen(DataPoint::new(tick.value, transform.viewport().y.min))
                .map(|p| p.x);
            let Some(x) = x else { continue };
            let segment = LineSegment::new(
                ScreenPoint::new(x, plot_rect.min.y),
                ScreenPoint::new(x, plot_rect.max.y),
            );
            if tick.is_major {
                major.push(segment);
            } else if plot.x_axis().show_minor_grid() {
                minor.push(segment);
            }
        }
    }

    if plot.y_axis().show_grid() {
        for tick in &y_layout.ticks {
            let y = transform
                .data_to_screen(DataPoint::new(transform.viewport().x.min, tick.value))
                .map(|p| p.y);
            let Some(y) = y else { continue };
            let segment = LineSegment::new(
                ScreenPoint::new(plot_rect.min.x, y),
                ScreenPoint::new(plot_rect.max.x, y),
            );
            if tick.is_major {
                major.push(segment);
            } else if plot.y_axis().show_minor_grid() {
                minor.push(segment);
            }
        }
    }

    render.push(RenderCommand::ClipRect(plot_rect));
    if !minor.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: minor,
            style: LineStyle {
                color: theme.grid_minor,
                width: 1.0,
            },
        });
    }
    if !major.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: major,
            style: LineStyle {
                color: theme.grid_major,
                width: 1.0,
            },
        });
    }

    if plot.x_axis().show_zero_line() {
        if transform.viewport().y.min <= 0.0 && transform.viewport().y.max >= 0.0 {
            if let Some(y) = transform
                .data_to_screen(DataPoint::new(transform.viewport().x.min, 0.0))
                .map(|p| p.y)
            {
                render.push(RenderCommand::LineSegments {
                    segments: vec![LineSegment::new(
                        ScreenPoint::new(plot_rect.min.x, y),
                        ScreenPoint::new(plot_rect.max.x, y),
                    )],
                    style: LineStyle {
                        color: theme.axis,
                        width: 1.0,
                    },
                });
            }
        }
    }

    if plot.y_axis().show_zero_line() {
        if transform.viewport().x.min <= 0.0 && transform.viewport().x.max >= 0.0 {
            if let Some(x) = transform
                .data_to_screen(DataPoint::new(0.0, transform.viewport().y.min))
                .map(|p| p.x)
            {
                render.push(RenderCommand::LineSegments {
                    segments: vec![LineSegment::new(
                        ScreenPoint::new(x, plot_rect.min.y),
                        ScreenPoint::new(x, plot_rect.max.y),
                    )],
                    style: LineStyle {
                        color: theme.axis,
                        width: 1.0,
                    },
                });
            }
        }
    }

    render.push(RenderCommand::ClipEnd);
}

fn build_series(
    render: &mut RenderList,
    plot: &Plot,
    state: &mut PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
) {
    let plot_width = plot_rect.width().max(1.0) as usize;
    let size = (
        plot_rect.width().round() as u32,
        plot_rect.height().round() as u32,
    );

    render.push(RenderCommand::ClipRect(plot_rect));

    for series in plot.series() {
        if !series.is_visible() {
            continue;
        }
        let cache = state.series_cache.entry(series.id()).or_default();
        let key = RenderCacheKey {
            viewport: transform.viewport(),
            size,
            generation: series.generation(),
        };
        if cache.key.as_ref() != Some(&key) {
            let decimated = series.data().decimate(
                transform.viewport().x,
                plot_width,
                &mut state.decimation_scratch,
            );
            cache.points.clear();
            cache.points.extend_from_slice(decimated);
            cache.key = Some(key.clone());
        }

        match series.kind() {
            SeriesKind::Line(style) => {
                let mut segments = Vec::new();
                build_line_segments(&cache.points, transform, plot_rect, &mut segments);
                if !segments.is_empty() {
                    render.push(RenderCommand::LineSegments {
                        segments,
                        style: *style,
                    });
                }
            }
            SeriesKind::Scatter(style) => {
                let mut points = Vec::new();
                build_scatter_points(&cache.points, transform, plot_rect, &mut points);
                if !points.is_empty() {
                    render.push(RenderCommand::Points {
                        points,
                        style: *style,
                    });
                }
            }
        }
    }

    render.push(RenderCommand::ClipEnd);
}

fn build_selection(render: &mut RenderList, plot: &Plot, state: &PlotUiState) {
    if let Some(rect) = state.selection_rect {
        let rect = normalized_rect(rect);
        render.push(RenderCommand::Rect {
            rect,
            style: RectStyle {
                fill: plot.theme().selection_fill,
                stroke: plot.theme().selection_border,
                stroke_width: 1.0,
            },
        });
    }
}

fn build_pins(
    render: &mut RenderList,
    plot: &Plot,
    transform: &Transform,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    if plot.pins().is_empty() {
        return;
    }

    let theme = plot.theme();
    let font_size = 12.0;
    let line_height = 14.0;
    let mut labels: Vec<PinLabel> = Vec::new();
    render.push(RenderCommand::ClipRect(plot_rect));

    for pin in plot.pins() {
        let Some(series) = plot
            .series()
            .iter()
            .find(|series| series.id() == pin.series_id)
        else {
            continue;
        };
        if !series.is_visible() {
            continue;
        }
        let Some(point) = series.data().data().point(pin.point_index) else {
            continue;
        };
        let Some(screen) = transform.data_to_screen(point) else {
            continue;
        };

        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            continue;
        }

        let (marker_style, base_size) = marker_style_and_size(series);

        let ring_outer = base_size + PIN_RING_OUTER_PAD;
        let ring_inner = base_size + PIN_RING_INNER_PAD;
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.axis,
                size: ring_outer,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.background,
                size: ring_inner,
                shape: MarkerShape::Circle,
            },
        });

        render.push(RenderCommand::Points {
            points: vec![screen],
            style: marker_style,
        });

        let x_text = plot.x_axis().format_value(point.x);
        let y_text = plot.y_axis().format_value(point.y);
        let label = format!("{}\nx: {x_text}\ny: {y_text}", series.name());
        let size = measurer.measure_multiline(&label, font_size);
        labels.push(PinLabel {
            screen,
            label,
            size,
        });
    }

    if labels.is_empty() {
        render.push(RenderCommand::ClipEnd);
        return;
    }

    let plot_area = plot_rect.width().max(1.0) * plot_rect.height().max(1.0);
    let total_label_area: f32 = labels.iter().map(|label| label.size.0 * label.size.1).sum();
    let dense =
        labels.len() > MAX_PIN_LABELS || total_label_area > plot_area * MAX_PIN_LABEL_COVERAGE;

    let mut clusters = cluster_pin_labels(&labels, PIN_CLUSTER_RADIUS);
    clusters.sort_by(|a, b| {
        let size_cmp = b.len().cmp(&a.len());
        if size_cmp != Ordering::Equal {
            return size_cmp;
        }
        let min_a = a.iter().copied().min().unwrap_or(0);
        let min_b = b.iter().copied().min().unwrap_or(0);
        min_a.cmp(&min_b)
    });

    let mut placed: Vec<ScreenRect> = Vec::new();
    let mut single_budget = if dense { MAX_PIN_LABELS } else { usize::MAX };
    for cluster in clusters {
        if cluster.len() >= 2 {
            if !dense {
                let mut local_placed = placed.clone();
                let mut placements: Vec<(ScreenPoint, ScreenRect, usize)> = Vec::new();
                let mut success = true;
                for index in &cluster {
                    let entry = &labels[*index];
                    if let Some((origin, rect)) = place_label(
                        entry.screen,
                        entry.size,
                        plot_rect,
                        PIN_LABEL_OFFSET,
                        &local_placed,
                    ) {
                        local_placed.push(rect);
                        placements.push((origin, rect, *index));
                    } else {
                        success = false;
                        break;
                    }
                }

                if success {
                    placed = local_placed;
                    for (origin, rect, index) in placements {
                        let entry = &labels[index];
                        push_label_with_leader(
                            render,
                            rect,
                            origin,
                            entry.screen,
                            &entry.label,
                            font_size,
                            line_height,
                            theme,
                        );
                    }
                    continue;
                }
            }

            let center = cluster_center(&labels, &cluster);
            let label = format!("{} pins", cluster.len());
            let size = measurer.measure_multiline(&label, font_size);
            if let Some((origin, rect)) =
                place_label(center, size, plot_rect, PIN_LABEL_OFFSET, &placed)
            {
                placed.push(rect);
                push_label_with_leader(
                    render,
                    rect,
                    origin,
                    center,
                    &label,
                    font_size,
                    line_height,
                    theme,
                );
            }
            continue;
        }

        if single_budget == 0 {
            continue;
        }
        let index = cluster[0];
        let entry = &labels[index];
        if let Some((origin, rect)) = place_label(
            entry.screen,
            entry.size,
            plot_rect,
            PIN_LABEL_OFFSET,
            &placed,
        ) {
            placed.push(rect);
            push_label_with_leader(
                render,
                rect,
                origin,
                entry.screen,
                &entry.label,
                font_size,
                line_height,
                theme,
            );
            single_budget = single_budget.saturating_sub(1);
        }
    }

    render.push(RenderCommand::ClipEnd);
}

#[allow(clippy::too_many_arguments)]
fn build_axes(
    render: &mut RenderList,
    plot: &Plot,
    x_layout: &AxisLayout,
    y_layout: &AxisLayout,
    plot_rect: ScreenRect,
    transform: &Transform,
    x_axis_rect: ScreenRect,
    y_axis_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let mut ticks_major = Vec::new();
    let mut ticks_minor = Vec::new();
    let label_gap = 2.0_f32;
    let mut last_x_label_right = f32::NEG_INFINITY;
    let mut last_y_label_top = f32::INFINITY;
    let x_title_rect = axis_title_text(plot.x_axis()).map(|title| {
        let size = measurer.measure(&title, plot.x_axis().label_size());
        let pos = clamp_label_position(
            ScreenPoint::new(
                plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
                x_axis_rect.max.y - size.1 - AXIS_PADDING,
            ),
            size,
            x_axis_rect,
        );
        ScreenRect::new(pos, ScreenPoint::new(pos.x + size.0, pos.y + size.1))
    });
    let y_title_rect = axis_title_text(plot.y_axis()).map(|title| {
        let size = measurer.measure(&title, plot.y_axis().label_size());
        let pos = clamp_label_position(
            ScreenPoint::new(
                y_axis_rect.min.x + AXIS_PADDING,
                y_axis_rect.min.y + AXIS_PADDING,
            ),
            size,
            y_axis_rect,
        );
        ScreenRect::new(pos, ScreenPoint::new(pos.x + size.0, pos.y + size.1))
    });

    if plot.x_axis().show_border() {
        render.push(RenderCommand::Rect {
            rect: plot_rect,
            style: RectStyle {
                fill: Color::new(0.0, 0.0, 0.0, 0.0),
                stroke: theme.axis,
                stroke_width: 1.0,
            },
        });
    }

    for tick in &x_layout.ticks {
        if let Some(x) = transform
            .data_to_screen(DataPoint::new(tick.value, transform.viewport().y.min))
            .map(|p| p.x)
        {
            let length = if tick.is_major {
                TICK_LENGTH_MAJOR
            } else {
                TICK_LENGTH_MINOR
            };
            let segment = LineSegment::new(
                ScreenPoint::new(x, plot_rect.max.y),
                ScreenPoint::new(x, plot_rect.max.y + length),
            );
            if tick.is_major {
                ticks_major.push(segment);
            } else if plot.x_axis().show_minor_grid() {
                ticks_minor.push(segment);
            }

            if tick.is_major && !tick.label.is_empty() {
                let size = measurer.measure(&tick.label, plot.x_axis().label_size());
                let pos = clamp_label_position(
                    ScreenPoint::new(
                        x - size.0 * 0.5,
                        plot_rect.max.y + TICK_LENGTH_MAJOR + AXIS_PADDING,
                    ),
                    size,
                    x_axis_rect,
                );
                let label_left = pos.x;
                let label_right = pos.x + size.0;
                let label_rect =
                    ScreenRect::new(pos, ScreenPoint::new(label_right, pos.y + size.1));
                let overlaps_title = x_title_rect
                    .map(|rect| rect_intersects(label_rect, rect))
                    .unwrap_or(false);
                if !overlaps_title && label_left >= last_x_label_right + label_gap {
                    render.push(RenderCommand::Text {
                        position: pos,
                        text: tick.label.clone(),
                        style: TextStyle {
                            color: theme.axis,
                            size: plot.x_axis().label_size(),
                        },
                    });
                    last_x_label_right = label_right;
                }
            }
        }
    }

    for tick in &y_layout.ticks {
        if let Some(y) = transform
            .data_to_screen(DataPoint::new(transform.viewport().x.min, tick.value))
            .map(|p| p.y)
        {
            let length = if tick.is_major {
                TICK_LENGTH_MAJOR
            } else {
                TICK_LENGTH_MINOR
            };
            let segment = LineSegment::new(
                ScreenPoint::new(plot_rect.min.x - length, y),
                ScreenPoint::new(plot_rect.min.x, y),
            );
            if tick.is_major {
                ticks_major.push(segment);
            } else if plot.y_axis().show_minor_grid() {
                ticks_minor.push(segment);
            }

            if tick.is_major && !tick.label.is_empty() {
                let size = measurer.measure(&tick.label, plot.y_axis().label_size());
                let pos = clamp_label_position(
                    ScreenPoint::new(
                        plot_rect.min.x - TICK_LENGTH_MAJOR - AXIS_PADDING - size.0,
                        y - size.1 * 0.5,
                    ),
                    size,
                    y_axis_rect,
                );
                let label_top = pos.y;
                let label_bottom = pos.y + size.1;
                let label_rect =
                    ScreenRect::new(pos, ScreenPoint::new(pos.x + size.0, label_bottom));
                let overlaps_title = y_title_rect
                    .map(|rect| rect_intersects(label_rect, rect))
                    .unwrap_or(false);
                if !overlaps_title && label_bottom <= last_y_label_top - label_gap {
                    render.push(RenderCommand::Text {
                        position: pos,
                        text: tick.label.clone(),
                        style: TextStyle {
                            color: theme.axis,
                            size: plot.y_axis().label_size(),
                        },
                    });
                    last_y_label_top = label_top;
                }
            }
        }
    }

    if !ticks_minor.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: ticks_minor,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
    if !ticks_major.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: ticks_major,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
}

fn build_axis_titles(
    render: &mut RenderList,
    plot: &Plot,
    plot_rect: ScreenRect,
    x_axis_rect: ScreenRect,
    y_axis_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    if let Some(title) = axis_title_text(plot.x_axis()) {
        let size = measurer.measure(&title, plot.x_axis().label_size());
        let pos = clamp_label_position(
            ScreenPoint::new(
                plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
                x_axis_rect.max.y - size.1 - AXIS_PADDING,
            ),
            size,
            x_axis_rect,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: title,
            style: TextStyle {
                color: theme.axis,
                size: plot.x_axis().label_size(),
            },
        });
    }

    if let Some(title) = axis_title_text(plot.y_axis()) {
        let pos = clamp_label_position(
            ScreenPoint::new(
                y_axis_rect.min.x + AXIS_PADDING,
                y_axis_rect.min.y + AXIS_PADDING,
            ),
            measurer.measure(&title, plot.y_axis().label_size()),
            y_axis_rect,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: title,
            style: TextStyle {
                color: theme.axis,
                size: plot.y_axis().label_size(),
            },
        });
    }
}

fn clamp_label_position(pos: ScreenPoint, size: (f32, f32), rect: ScreenRect) -> ScreenPoint {
    let max_x = (rect.max.x - size.0).max(rect.min.x);
    let max_y = (rect.max.y - size.1).max(rect.min.y);
    ScreenPoint::new(
        pos.x.clamp(rect.min.x, max_x),
        pos.y.clamp(rect.min.y, max_y),
    )
}

fn build_hover(
    render: &mut RenderList,
    plot: &Plot,
    state: &PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let Some(cursor) = state.hover else { return };
    if cursor.x < plot_rect.min.x
        || cursor.x > plot_rect.max.x
        || cursor.y < plot_rect.min.y
        || cursor.y > plot_rect.max.y
    {
        return;
    }

    if let Some(target) = state.hover_target {
        let Some(series) = plot
            .series()
            .iter()
            .find(|series| series.id() == target.pin.series_id)
        else {
            return;
        };
        let Some(point) = series.data().data().point(target.pin.point_index) else {
            return;
        };
        let screen = target.screen;
        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            return;
        }

        if target.is_pinned {
            let (_, base_size) = marker_style_and_size(series);
            let ring_outer = base_size + PIN_RING_OUTER_PAD;
            let ring_inner = base_size + PIN_RING_INNER_PAD;
            render.push(RenderCommand::Points {
                points: vec![screen],
                style: MarkerStyle {
                    color: PIN_UNPIN_HIGHLIGHT,
                    size: ring_outer,
                    shape: MarkerShape::Circle,
                },
            });
            render.push(RenderCommand::Points {
                points: vec![screen],
                style: MarkerStyle {
                    color: theme.background,
                    size: ring_inner,
                    shape: MarkerShape::Circle,
                },
            });
            return;
        }

        let (marker_style, base_size) = marker_style_and_size(series);
        let ring_outer = base_size + PIN_RING_OUTER_PAD;
        let ring_inner = base_size + PIN_RING_INNER_PAD;
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.axis,
                size: ring_outer,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.background,
                size: ring_inner,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: marker_style,
        });

        let x_text = plot.x_axis().format_value(point.x);
        let y_text = plot.y_axis().format_value(point.y);
        let label = format!("{}\nx: {x_text}\ny: {y_text}", series.name());
        let size = measurer.measure_multiline(&label, 12.0);
        let mut origin = ScreenPoint::new(screen.x + 12.0, screen.y + 12.0);
        if origin.x + size.0 > plot_rect.max.x {
            origin.x = screen.x - size.0 - 12.0;
        }
        if origin.y + size.1 > plot_rect.max.y {
            origin.y = screen.y - size.1 - 12.0;
        }
        origin = clamp_point(origin, plot_rect, size);

        render.push(RenderCommand::Rect {
            rect: ScreenRect::new(
                origin,
                ScreenPoint::new(origin.x + size.0, origin.y + size.1),
            ),
            style: RectStyle {
                fill: theme.pin_bg,
                stroke: theme.pin_border,
                stroke_width: 1.0,
            },
        });

        for (index, line) in label.lines().enumerate() {
            let line_y = origin.y + index as f32 * 14.0 + 2.0;
            render.push(RenderCommand::Text {
                position: ScreenPoint::new(origin.x + 4.0, line_y),
                text: line.to_string(),
                style: TextStyle {
                    color: theme.axis,
                    size: 12.0,
                },
            });
        }
        return;
    }

    let Some(data) = transform.screen_to_data(cursor) else {
        return;
    };
    let x_text = plot.x_axis().format_value(data.x);
    let y_text = plot.y_axis().format_value(data.y);
    let label = format!("x: {x_text}\ny: {y_text}");

    let size = measurer.measure_multiline(&label, 12.0);
    let mut origin = ScreenPoint::new(cursor.x + 12.0, cursor.y + 12.0);
    if origin.x + size.0 > plot_rect.max.x {
        origin.x = cursor.x - size.0 - 12.0;
    }
    if origin.y + size.1 > plot_rect.max.y {
        origin.y = cursor.y - size.1 - 12.0;
    }
    origin = clamp_point(origin, plot_rect, size);

    render.push(RenderCommand::Rect {
        rect: ScreenRect::new(
            origin,
            ScreenPoint::new(origin.x + size.0, origin.y + size.1),
        ),
        style: RectStyle {
            fill: theme.hover_bg,
            stroke: theme.hover_border,
            stroke_width: 1.0,
        },
    });

    for (index, line) in label.lines().enumerate() {
        let line_y = origin.y + index as f32 * 14.0 + 2.0;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(origin.x + 4.0, line_y),
            text: line.to_string(),
            style: TextStyle {
                color: theme.axis,
                size: 12.0,
            },
        });
    }
}

fn build_legend(
    render: &mut RenderList,
    plot: &Plot,
    state: &mut PlotUiState,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let series_list = plot.series();
    if series_list.is_empty() {
        state.legend_layout = None;
        return;
    }

    let font_size = LEGEND_FONT_SIZE;
    let line_height = LEGEND_LINE_HEIGHT;
    let padding = LEGEND_PADDING;
    let text_start_x = padding
        + LEGEND_TOGGLE_DIAMETER
        + LEGEND_TOGGLE_GAP
        + LEGEND_SWATCH_WIDTH
        + LEGEND_SWATCH_GAP;
    let mut max_width: f32 = 0.0;
    for series in series_list {
        let size = measurer.measure(series.name(), font_size);
        max_width = max_width.max(size.0);
    }
    let legend_width = text_start_x + max_width + padding;
    let legend_height = series_list.len() as f32 * line_height + padding * 2.0;

    let mut origin = ScreenPoint::new(
        plot_rect.max.x - legend_width - padding,
        plot_rect.min.y + padding,
    );
    origin = clamp_point(origin, plot_rect, (legend_width, legend_height));
    let legend_rect = ScreenRect::new(
        origin,
        ScreenPoint::new(origin.x + legend_width, origin.y + legend_height),
    );

    render.push(RenderCommand::Rect {
        rect: legend_rect,
        style: RectStyle {
            fill: theme.legend_bg,
            stroke: theme.legend_border,
            stroke_width: 1.0,
        },
    });

    let mut entries = Vec::with_capacity(series_list.len());
    for (idx, series) in series_list.iter().enumerate() {
        let row_y = origin.y + padding + idx as f32 * line_height;
        let row_rect = ScreenRect::new(
            ScreenPoint::new(origin.x, row_y),
            ScreenPoint::new(origin.x + legend_width, row_y + line_height),
        );
        let row_center_y = row_y + line_height * 0.5;
        let toggle_origin = ScreenPoint::new(
            origin.x + padding,
            row_center_y - LEGEND_TOGGLE_DIAMETER * 0.5,
        );
        let toggle_rect = ScreenRect::new(
            toggle_origin,
            ScreenPoint::new(
                toggle_origin.x + LEGEND_TOGGLE_DIAMETER,
                toggle_origin.y + LEGEND_TOGGLE_DIAMETER,
            ),
        );
        entries.push(LegendEntry {
            series_id: series.id(),
            row_rect,
        });

        let visible = series.is_visible();
        let series_color = series_color(series);
        let swatch_color = if visible {
            series_color
        } else {
            with_alpha(series_color, LEGEND_HIDDEN_ALPHA)
        };
        let text_color = if visible {
            theme.axis
        } else {
            with_alpha(theme.axis, LEGEND_TEXT_HIDDEN_ALPHA)
        };
        let ring_color = if visible {
            with_alpha(theme.axis, 0.7)
        } else {
            with_alpha(theme.axis, 0.45)
        };
        let fill_color = if visible {
            series_color
        } else {
            theme.legend_bg
        };
        let toggle_center = ScreenPoint::new(
            toggle_rect.min.x + LEGEND_TOGGLE_DIAMETER * 0.5,
            toggle_rect.min.y + LEGEND_TOGGLE_DIAMETER * 0.5,
        );

        render.push(RenderCommand::Points {
            points: vec![toggle_center],
            style: MarkerStyle {
                color: ring_color,
                size: LEGEND_TOGGLE_DIAMETER,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![toggle_center],
            style: MarkerStyle {
                color: fill_color,
                size: LEGEND_TOGGLE_INNER_DIAMETER,
                shape: MarkerShape::Circle,
            },
        });

        let swatch_start = ScreenPoint::new(toggle_rect.max.x + LEGEND_TOGGLE_GAP, row_center_y);
        let swatch_end = ScreenPoint::new(swatch_start.x + LEGEND_SWATCH_WIDTH, row_center_y);
        render.push(RenderCommand::LineSegments {
            segments: vec![LineSegment::new(swatch_start, swatch_end)],
            style: LineStyle {
                color: swatch_color,
                width: 2.0,
            },
        });
        let text_y = row_y + (line_height - font_size) * 0.5;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(swatch_end.x + LEGEND_SWATCH_GAP, text_y),
            text: series.name().to_string(),
            style: TextStyle {
                color: text_color,
                size: font_size,
            },
        });
    }

    state.legend_layout = Some(LegendLayout {
        rect: legend_rect,
        entries,
    });
}

#[derive(Debug, Clone)]
struct PinLabel {
    screen: ScreenPoint,
    label: String,
    size: (f32, f32),
}

fn marker_style_and_size(series: &Series) -> (MarkerStyle, f32) {
    match series.kind() {
        SeriesKind::Line(line) => (
            MarkerStyle {
                color: line.color,
                size: 6.0,
                shape: MarkerShape::Circle,
            },
            6.0,
        ),
        SeriesKind::Scatter(marker) => (
            MarkerStyle {
                color: marker.color,
                size: marker.size.max(6.0),
                shape: marker.shape,
            },
            marker.size.max(6.0),
        ),
    }
}

fn cluster_pin_labels(labels: &[PinLabel], radius: f32) -> Vec<Vec<usize>> {
    let radius_sq = radius * radius;
    let mut visited = vec![false; labels.len()];
    let mut clusters: Vec<Vec<usize>> = Vec::new();

    for start in 0..labels.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut cluster = Vec::new();
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            cluster.push(index);
            for next in 0..labels.len() {
                if visited[next] {
                    continue;
                }
                if distance_sq(labels[index].screen, labels[next].screen) <= radius_sq {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        clusters.push(cluster);
    }

    clusters
}

fn cluster_center(labels: &[PinLabel], cluster: &[usize]) -> ScreenPoint {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for index in cluster {
        let screen = labels[*index].screen;
        sum_x += screen.x;
        sum_y += screen.y;
    }
    let count = cluster.len().max(1) as f32;
    ScreenPoint::new(sum_x / count, sum_y / count)
}

fn pin_label_candidates(screen: ScreenPoint, size: (f32, f32), offset: f32) -> [ScreenPoint; 6] {
    [
        ScreenPoint::new(screen.x + offset, screen.y + offset),
        ScreenPoint::new(screen.x + offset, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 - offset, screen.y + offset),
        ScreenPoint::new(screen.x - size.0 - offset, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 * 0.5, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 * 0.5, screen.y + offset),
    ]
}

fn place_label(
    screen: ScreenPoint,
    size: (f32, f32),
    plot_rect: ScreenRect,
    offset: f32,
    placed: &[ScreenRect],
) -> Option<(ScreenPoint, ScreenRect)> {
    for origin in pin_label_candidates(screen, size, offset) {
        let origin = clamp_point(origin, plot_rect, size);
        let rect = ScreenRect::new(
            origin,
            ScreenPoint::new(origin.x + size.0, origin.y + size.1),
        );
        if !rect_intersects_any(rect, placed) {
            return Some((origin, rect));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn push_label_with_leader(
    render: &mut RenderList,
    rect: ScreenRect,
    origin: ScreenPoint,
    screen: ScreenPoint,
    label: &str,
    font_size: f32,
    line_height: f32,
    theme: &Theme,
) {
    let anchor = ScreenPoint::new(
        screen.x.clamp(rect.min.x, rect.max.x),
        screen.y.clamp(rect.min.y, rect.max.y),
    );
    render.push(RenderCommand::LineSegments {
        segments: vec![LineSegment::new(screen, anchor)],
        style: LineStyle {
            color: theme.pin_border,
            width: 1.0,
        },
    });
    render.push(RenderCommand::Rect {
        rect,
        style: RectStyle {
            fill: theme.pin_bg,
            stroke: theme.pin_border,
            stroke_width: 1.0,
        },
    });
    for (index, line) in label.lines().enumerate() {
        let line_y = origin.y + index as f32 * line_height + 2.0;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(origin.x + 4.0, line_y),
            text: line.to_string(),
            style: TextStyle {
                color: theme.axis,
                size: font_size,
            },
        });
    }
}

fn axis_title_text(axis: &AxisConfig) -> Option<String> {
    match (axis.title(), axis.units()) {
        (Some(title), Some(units)) => Some(format!("{title} ({units})")),
        (Some(title), None) => Some(title.to_string()),
        (None, Some(units)) => Some(units.to_string()),
        (None, None) => None,
    }
}

fn series_color(series: &Series) -> Color {
    match series.kind() {
        SeriesKind::Line(style) => style.color,
        SeriesKind::Scatter(style) => style.color,
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        a: (color.a * alpha).clamp(0.0, 1.0),
        ..color
    }
}
