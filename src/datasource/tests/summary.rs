//! @file summary.rs
//! @brief Downsampling correctness regressions
//! @author modolet <y@xxyx.io>
//! @date 2026-09-20

use super::*;

#[test]
fn range_extrema_match_raw_points_across_alignment_and_streaming_tails() {
    let points = (0..257)
        .map(|index| Point::new(index as f64, ((index * 97) % 53) as f64 - 26.))
        .collect::<Vec<_>>();
    for base in [1, 4, 64] {
        let mut summary = SummaryLevels::new(base);
        for &point in &points {
            summary.push(point);
        }
        for start in 0..points.len() {
            for end in start + 1..=points.len() {
                let actual = summary.extrema(&points, start..end).unwrap();
                let mut expected = MinMax::from_point(points[start]);
                for &point in &points[start..end] {
                    expected.push(point);
                }
                assert_eq!(
                    actual.min, expected.min,
                    "base={base}, range={start}..{end}"
                );
                assert_eq!(
                    actual.max, expected.max,
                    "base={base}, range={start}..{end}"
                );
                assert_eq!(actual.x_range, expected.x_range);
            }
        }
        assert!(summary.extrema(&points, 17..17).is_none());
    }
}

#[test]
fn selected_summary_is_never_coarser_than_requested_resolution() {
    let mut summary = SummaryLevels::new(64);
    for index in 0..1024 {
        summary.push(Point::new(index as f64, 0.));
    }
    assert!(summary.choose_level(63).is_none());
    for target in 64..1024 {
        let level = summary.choose_level(target).unwrap();
        assert!(level.chunk_size <= target);
        assert!(level.chunk_size * 2 > target);
    }
}

#[test]
fn decimate_preserves_extremes() {
    let points = [
        Point::new(0.0, 1.0),
        Point::new(1.0, 5.0),
        Point::new(2.0, 0.5),
        Point::new(3.0, 3.0),
    ];
    let mut scratch = DecimationScratch::new();
    let out = decimate_minmax(&points, Range::new(0.0, 3.0), 1, &mut scratch);
    assert_eq!(out.len(), 2);
    let ys = [out[0].y, out[1].y];
    assert!(ys.contains(&0.5));
    assert!(ys.contains(&5.0));
}

#[test]
fn summary_levels_grow() {
    let mut summary = SummaryLevels::new(2);
    summary.push(Point::new(0.0, 1.0));
    summary.push(Point::new(1.0, 2.0));
    summary.push(Point::new(2.0, 3.0));
    summary.push(Point::new(3.0, 4.0));
    assert!(!summary.levels.is_empty());
    let level = &summary.levels[0];
    assert_eq!(level.chunk_size, 2);
    assert_eq!(level.buckets.len(), 2);
}
