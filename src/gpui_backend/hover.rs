use crate::geom::{ScreenPoint, ScreenRect};
use crate::plot::Plot;
use crate::transform::Transform;
use crate::view::Range;

use super::config::PlotViewConfig;
use super::geometry::distance_sq;
use super::state::{HoverTarget, PlotUiState};

pub(crate) fn hover_target_within_threshold(
    target: &HoverTarget,
    cursor: ScreenPoint,
    config: &PlotViewConfig,
) -> bool {
    let threshold = if target.is_pinned {
        config.unpin_threshold_px
    } else {
        config.pin_threshold_px
    };
    distance_sq(target.screen, cursor) <= threshold * threshold
}

pub(crate) fn update_hover_target(
    plot: &Plot,
    state: &mut PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
    pin_threshold: f32,
    unpin_threshold: f32,
) {
    let Some(cursor) = state.hover else {
        state.hover_target = None;
        return;
    };
    state.hover_target = compute_hover_target(
        plot,
        transform,
        cursor,
        Some(plot_rect),
        pin_threshold,
        unpin_threshold,
    );
}

pub(crate) fn compute_hover_target(
    plot: &Plot,
    transform: &Transform,
    cursor: ScreenPoint,
    plot_rect: Option<ScreenRect>,
    pin_threshold: f32,
    unpin_threshold: f32,
) -> Option<HoverTarget> {
    let plot_rect = plot_rect?;
    if cursor.x < plot_rect.min.x
        || cursor.x > plot_rect.max.x
        || cursor.y < plot_rect.min.y
        || cursor.y > plot_rect.max.y
    {
        return None;
    }

    if let Some(target) = nearest_pinned_within(plot, transform, cursor, plot_rect, unpin_threshold)
    {
        return Some(target);
    }

    find_nearest_unpinned_point(plot, transform, cursor, plot_rect, pin_threshold)
}

fn nearest_pinned_within(
    plot: &Plot,
    transform: &Transform,
    cursor: ScreenPoint,
    plot_rect: ScreenRect,
    threshold: f32,
) -> Option<HoverTarget> {
    let threshold_sq = threshold * threshold;
    let mut best: Option<(crate::interaction::Pin, ScreenPoint, f32)> = None;
    for pin in plot.pins() {
        let Some(screen) = pin_screen_point(plot, *pin, transform) else {
            continue;
        };
        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            continue;
        }
        let dist = distance_sq(screen, cursor);
        if dist > threshold_sq {
            continue;
        }
        if best.is_none_or(|best| dist < best.2) {
            best = Some((*pin, screen, dist));
        }
    }
    best.map(|(pin, screen, _)| HoverTarget {
        pin,
        screen,
        is_pinned: true,
    })
}

fn find_nearest_unpinned_point(
    plot: &Plot,
    transform: &Transform,
    cursor: ScreenPoint,
    plot_rect: ScreenRect,
    threshold: f32,
) -> Option<HoverTarget> {
    let center = transform.screen_to_data(cursor)?;
    let edge = transform.screen_to_data(ScreenPoint::new(cursor.x + threshold, cursor.y))?;
    let dx = (edge.x - center.x).abs();
    let search_range = Range::new(center.x - dx, center.x + dx);
    let threshold_sq = threshold * threshold;
    let pins = plot.pins();
    let mut best: Option<(crate::interaction::Pin, ScreenPoint, f32)> = None;

    for series in plot.series() {
        if !series.is_visible() {
            continue;
        }
        series.with_store(|store| {
            let data = store.data();
            let index_range = data.range_by_x(search_range);
            for index in index_range {
                let Some(point) = data.point(index) else {
                    continue;
                };
                let Some(point_index) = store.pin_index(index) else {
                    continue;
                };
                let pin = crate::interaction::Pin {
                    series_id: series.id(),
                    point_index,
                };
                if pins.contains(&pin) {
                    continue;
                }
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
                let dist = distance_sq(screen, cursor);
                if dist > threshold_sq {
                    continue;
                }
                if best.is_none_or(|best| dist < best.2) {
                    best = Some((pin, screen, dist));
                }
            }
        });
    }

    best.map(|(pin, screen, _)| HoverTarget {
        pin,
        screen,
        is_pinned: false,
    })
}

fn pin_screen_point(
    plot: &Plot,
    pin: crate::interaction::Pin,
    transform: &Transform,
) -> Option<ScreenPoint> {
    let series = plot
        .series()
        .iter()
        .find(|series| series.id() == pin.series_id)?;
    if !series.is_visible() {
        return None;
    }
    let point = series.with_store(|store| store.pinned_point(pin.point_index))?;
    transform.data_to_screen(point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        geom::Point,
        series::{Series, SeriesKind},
        view::Viewport,
    };

    #[test]
    fn hover_and_pin_keep_absolute_positions_when_window_slides() {
        let mut series = Series::from_sample_window(
            "window",
            vec![10., 20., 30.].into(),
            100,
            1.,
            SeriesKind::Line(Default::default()),
        )
        .unwrap();
        let mut plot = Plot::new();
        plot.add_series(&series);
        let rect = ScreenRect::new(ScreenPoint::new(0., 0.), ScreenPoint::new(300., 300.));
        let transform = Transform::new(
            Viewport::new(Range::new(100., 103.), Range::new(0., 40.)),
            rect,
        )
        .unwrap();
        let cursor = transform.data_to_screen(Point::new(101., 20.)).unwrap();
        let target = compute_hover_target(&plot, &transform, cursor, Some(rect), 5., 10.).unwrap();
        assert_eq!(target.pin.point_index, 101);
        assert!(!target.is_pinned);
        plot.pins_mut().push(target.pin);
        series
            .update_sample_window(vec![20., 30., 40.].into(), 101, 1.)
            .unwrap();
        let retained =
            compute_hover_target(&plot, &transform, cursor, Some(rect), 5., 10.).unwrap();
        assert_eq!(retained.pin, target.pin);
        assert!(retained.is_pinned);
        assert_eq!(retained.screen, cursor);
        assert_eq!(
            pin_screen_point(&plot, target.pin, &transform),
            Some(cursor)
        );
        series
            .update_sample_window(vec![30., 40.].into(), 102, 1.)
            .unwrap();
        assert!(pin_screen_point(&plot, target.pin, &transform).is_none());
        assert!(compute_hover_target(&plot, &transform, cursor, Some(rect), 5., 10.).is_none());
    }
}
