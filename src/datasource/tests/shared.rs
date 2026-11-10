//! Shared-sample regression checks.
//! Author: modolet <y@xxyx.io>, 2026-11-10.
use super::*;
use std::sync::Arc;

#[test]
fn shared_storage_matches_explicit_bounds_queries_and_decimation() {
    let values: Arc<[f32]> = (0..10003)
        .map(|i| {
            if i % 137 == 0 {
                50.
            } else {
                (i as f32 * 0.17).sin()
            }
        })
        .collect::<Vec<_>>()
        .into();
    let range = 37..9991;
    let rate = 1234.5;
    let shared = AppendOnlyData::shared_samples(values.clone(), range.clone(), rate).unwrap();
    assert_eq!(Arc::strong_count(&values), 2);
    assert!(shared.points.is_empty());
    let explicit = AppendOnlyData::from_iter_points(
        range.map(|i| Point::new(i as f64 / rate, values[i] as f64)),
    );
    assert_eq!(shared.bounds(), explicit.bounds());
    for x in [-10., 0., 37. / rate, 38. / rate, 0.125, 1.01, 3.7, 10.] {
        assert_eq!(shared.nearest_index_by_x(x), explicit.nearest_index_by_x(x));
        for width in [0., 0.001, 0.05, 5., 100.] {
            let viewport = Range::new(x, x + width);
            assert_eq!(shared.range_by_x(viewport), explicit.range_by_x(viewport));
        }
    }
    let shared = SeriesStore::with_base_chunk(shared, 64);
    let explicit = SeriesStore::with_base_chunk(explicit, 64);
    for width in [0, 1, 7, 100, 3000, 20000] {
        for range in [
            Range::new(-1., 10.),
            Range::new(0.2, 0.21),
            Range::new(0.2, 3.3),
            Range::new(7.9, 8.1),
        ] {
            let mut a = DecimationScratch::new();
            let mut b = DecimationScratch::new();
            assert_eq!(
                shared.decimate(range, width, &mut a),
                explicit.decimate(range, width, &mut b)
            );
        }
    }
}

#[test]
fn readonly_samples_reject_mutation_and_validate_inputs() {
    let values: Arc<[f32]> = Arc::from([1., 2., 3.]);
    let mut shared = AppendOnlyData::shared_samples(values.clone(), 1..3, 2.).unwrap();
    assert_eq!(shared.point(0), Some(Point::new(0.5, 2.)));
    assert_eq!(shared.point(2), None);
    assert_eq!(shared.push_y(4.), Err(AppendError::WrongMode));
    assert_eq!(
        shared.push_point(Point::new(2., 4.)),
        Err(AppendError::WrongMode)
    );
    assert_eq!(shared.len(), 2);
    assert!(AppendOnlyData::shared_samples(values.clone(), 0..4, 2.).is_err());
    assert!(
        AppendOnlyData::shared_samples(values.clone(), std::ops::Range { start: 2, end: 1 }, 2.)
            .is_err()
    );
    assert!(AppendOnlyData::shared_samples(values.clone(), 0..3, 0.).is_err());
    assert!(AppendOnlyData::shared_samples(Arc::from([f32::NAN]), 0..1, 1.).is_err());
    let empty = AppendOnlyData::shared_samples(values, 2..2, 2.).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.bounds(), None);
    assert_eq!(empty.nearest_index_by_x(1.), None);
}

#[test]
fn windows_validate_last_sample_without_rejecting_u64_max() {
    let values: Arc<[f32]> = Arc::from([1., 2.]);
    let last = AppendOnlyData::shared_samples_at(values.clone(), 0..1, 1., u64::MAX).unwrap();
    assert_eq!(last.point(0), Some(Point::new(u64::MAX as f64, 1.)));
    assert!(AppendOnlyData::shared_samples_at(values.clone(), 0..2, 1., u64::MAX).is_err());
    assert!(
        AppendOnlyData::shared_samples_at(values.clone(), 0..1, f64::MIN_POSITIVE, u64::MAX)
            .is_err()
    );
    assert!(
        AppendOnlyData::shared_samples_at(values, 1..1, f64::MIN_POSITIVE, u64::MAX)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn window_positions_queries_and_summary_match_explicit_points() {
    let values: std::sync::Arc<[f32]> = (0..513)
        .map(|i| if i == 300 { -1000. } else { i as f32 })
        .collect::<Vec<_>>()
        .into();
    let data = AppendOnlyData::shared_samples_at(values.clone(), 0..513, 2., 1000).unwrap();
    assert_eq!(data.point(0), Some(Point::new(500., 0.)));
    assert_eq!(data.point(512), Some(Point::new(756., 512.)));
    let range = Range::new(600., 700.);
    let explicit = AppendOnlyData::from_iter_points(
        (0..513).map(|i| Point::new((1000 + i) as f64 / 2., values[i] as f64)),
    );
    assert_eq!(data.range_by_x(range), explicit.range_by_x(range));
    assert_eq!(data.bounds(), explicit.bounds());
    let store = SeriesStore::with_base_chunk(data, 64);
    let expected = SeriesStore::with_base_chunk(explicit, 64);
    let mut a = DecimationScratch::default();
    let mut b = DecimationScratch::default();
    assert_eq!(
        store.decimate(range, 2, &mut a),
        expected.decimate(range, 2, &mut b)
    );
    assert!(AppendOnlyData::shared_samples_at(values.clone(), 0..513, 1., u64::MAX).is_err());
    assert!(AppendOnlyData::shared_samples_at(values, 0..513, f64::MIN_POSITIVE, 1000).is_err());
}
