//! @file store.rs
//! @brief Downsampling correctness regressions
//! @author modolet <y@xxyx.io>
//! @date 2026-09-20

use super::*;

#[test]
fn visible_extrema_and_unpaired_tail_survive_summary_decimation() {
    let mut points = (0..200)
        .map(|index| Point::new(index as f64, (index as f64).sin()))
        .collect::<Vec<_>>();
    points[0].y = 1000.;
    points[199].y = -1000.;
    points[31].y = 100.;
    points[150].y = -100.;
    let store = SeriesStore::with_base_chunk(AppendOnlyData::from_iter_points(points.clone()), 64);
    let mut scratch = DecimationScratch::new();
    for width in [1, 2, 3] {
        let output = store.decimate(Range::new(10., 190.), width, &mut scratch);
        assert!(output.contains(&points[31]));
        assert!(output.contains(&points[150]));
        assert!(output.iter().all(|point| (10. ..=190.).contains(&point.x)));
        assert!(output.windows(2).all(|pair| pair[0].x <= pair[1].x));
    }
    let output = store.decimate(Range::new(0., 198.), 1, &mut scratch);
    assert!(
        output.contains(&points[150]),
        "complete but unpaired lower-level tail must remain visible"
    );
}

#[test]
fn zoom_across_summary_threshold_keeps_at_least_one_bucket_per_pixel() {
    let store = SeriesStore::with_base_chunk(
        AppendOnlyData::from_iter_y((0..70_000).map(|index| (index as f64).sin())),
        64,
    );
    let mut scratch = DecimationScratch::new();
    let width = 512;
    for count in [32_768, 32_769, 33_000, 65_536, 65_537] {
        let output = store.decimate(Range::new(0., (count - 1) as f64), width, &mut scratch);
        assert!(
            output.len() >= width * 2,
            "density dropped at {count} samples"
        );
        assert!(
            output.len() <= width * 4 + 4,
            "render work must stay bounded"
        );
    }
}

#[test]
fn appended_partial_and_complete_buckets_keep_their_extrema() {
    for base in [1, 4, 64] {
        let mut store = SeriesStore::with_base_chunk(AppendOnlyData::indexed(), base);
        let mut scratch = DecimationScratch::new();
        for end in [6, 8, 14, 129, 193, 257] {
            let start = store.data().len();
            store
                .extend_y((start..end).map(|index| {
                    if index % 2 == 0 {
                        index as f64
                    } else {
                        -(index as f64)
                    }
                }))
                .unwrap();
            let output = store.decimate(Range::new(0., (end - 1) as f64), 1, &mut scratch);
            assert!(output.contains(&store.data().points()[end - 1]));
            assert!(output.contains(&store.data().points()[end - 2]));
        }
    }
}

#[test]
fn extend_y_updates_generation_for_each_new_point() {
    let mut store = SeriesStore::indexed();
    let added = store.extend_y([1.0, 2.0, 3.0]).unwrap();
    assert_eq!(added, 3);
    assert_eq!(store.generation(), 3);
}

#[test]
fn extend_points_non_monotonic_still_updates_generation() {
    let mut store = SeriesStore::with_base_chunk(AppendOnlyData::explicit(), 4);
    let result = store.extend_points([
        Point::new(1.0, 1.0),
        Point::new(2.0, 2.0),
        Point::new(1.5, 3.0),
    ]);
    assert_eq!(result, Err(AppendError::NonMonotonicX));
    assert_eq!(store.data().len(), 3);
    assert_eq!(store.generation(), 3);
}
