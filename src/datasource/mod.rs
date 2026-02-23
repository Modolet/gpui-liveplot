//! Data sources and append-only storage.
//!
//! The data layer is optimized for append-only workloads and fast range
//! queries. It underpins streaming plots and decimation logic.

mod store;
mod summary;

pub(crate) use store::SeriesStore;
pub(crate) use summary::DecimationScratch;

use crate::geom::Point;
use crate::view::{Range, Viewport};

/// Mode of the X axis data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XMode {
    /// X values are implicit indices.
    Index,
    /// X values are explicitly provided.
    Explicit,
}

/// Errors that can occur when appending data.
///
/// These errors indicate misuse of an append-only series (for example, mixing
/// implicit and explicit X modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendError {
    /// Attempted to append with an incompatible X mode.
    WrongMode,
    /// Explicit X values are not monotonic.
    ///
    /// Non-monotonic X values disable fast range slicing.
    NonMonotonicX,
}

/// Append-only data storage with incremental bounds tracking.
#[derive(Debug, Clone)]
pub(crate) struct AppendOnlyData {
    points: Vec<Point>,
    x_mode: XMode,
    monotonic: bool,
    bounds: Option<Viewport>,
}

impl AppendOnlyData {
    /// Create an empty data set with implicit X indices.
    pub fn indexed() -> Self {
        Self {
            points: Vec::new(),
            x_mode: XMode::Index,
            monotonic: true,
            bounds: None,
        }
    }

    /// Create an empty data set with explicit X values.
    pub fn explicit() -> Self {
        Self {
            points: Vec::new(),
            x_mode: XMode::Explicit,
            monotonic: true,
            bounds: None,
        }
    }

    /// Build an indexed data set from an iterator of Y values.
    pub fn from_iter_y<I, T>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<f64>,
    {
        let mut data = Self::indexed();
        let _ = data.extend_y(iter);
        data
    }

    /// Build an explicit data set from an iterator of points.
    pub fn from_iter_points<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Point>,
    {
        let mut data = Self::explicit();
        let _ = data.extend_points(iter);
        data
    }

    /// Build an explicit data set by sampling a callback function.
    pub fn from_explicit_callback(
        function: impl Fn(f64) -> f64,
        x_range: Range,
        points: usize,
    ) -> Self {
        let mut data = Self::explicit();
        if points == 0 {
            return data;
        }
        let span = x_range.span();
        let step = if points > 1 {
            span / (points - 1) as f64
        } else {
            0.0
        };
        for i in 0..points {
            let x = x_range.min + step * i as f64;
            let y = function(x);
            let _ = data.push_point(Point::new(x, y));
        }
        data
    }

    /// Append a Y value for indexed data.
    pub fn push_y(&mut self, y: f64) -> Result<usize, AppendError> {
        let index = self.points.len();
        self.extend_y([y]).map(|_| index)
    }

    /// Append multiple Y values for indexed data.
    pub fn extend_y<I, T>(&mut self, values: I) -> Result<usize, AppendError>
    where
        I: IntoIterator<Item = T>,
        T: Into<f64>,
    {
        if self.x_mode != XMode::Index {
            return Err(AppendError::WrongMode);
        }

        let values = values.into_iter();
        let (reserve, _) = values.size_hint();
        self.points.reserve(reserve);

        let start_len = self.points.len();
        for value in values {
            let index = self.points.len();
            let point = Point::new(index as f64, value.into());
            self.points.push(point);
            self.update_bounds(point);
        }
        Ok(self.points.len() - start_len)
    }

    /// Append a point with explicit X value.
    pub fn push_point(&mut self, point: Point) -> Result<usize, AppendError> {
        let index = self.points.len();
        self.extend_points([point]).map(|_| index)
    }

    /// Append multiple points with explicit X values.
    pub fn extend_points<I>(&mut self, points: I) -> Result<usize, AppendError>
    where
        I: IntoIterator<Item = Point>,
    {
        if self.x_mode != XMode::Explicit {
            return Err(AppendError::WrongMode);
        }

        let points = points.into_iter();
        let (reserve, _) = points.size_hint();
        self.points.reserve(reserve);

        let start_len = self.points.len();
        let mut last_x = self.points.last().map(|point| point.x);
        let mut non_monotonic = false;
        for point in points {
            if let Some(last_x) = last_x
                && point.x < last_x
            {
                self.monotonic = false;
                non_monotonic = true;
            }
            self.points.push(point);
            self.update_bounds(point);
            last_x = Some(point.x);
        }

        if non_monotonic {
            Err(AppendError::NonMonotonicX)
        } else {
            Ok(self.points.len() - start_len)
        }
    }

    /// Access all points as a slice.
    pub fn points(&self) -> &[Point] {
        &self.points
    }

    /// Access a single point by index.
    pub fn point(&self, index: usize) -> Option<Point> {
        self.points.get(index).copied()
    }

    /// Number of points stored.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if there are no points.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Get the bounds for all points.
    pub fn bounds(&self) -> Option<Viewport> {
        self.bounds
    }

    /// Access the X mode.
    pub fn x_mode(&self) -> XMode {
        self.x_mode
    }

    /// Check whether explicit X values are monotonic.
    pub fn is_monotonic(&self) -> bool {
        self.monotonic
    }

    /// Find the index range that intersects the X range.
    pub fn range_by_x(&self, range: Range) -> std::ops::Range<usize> {
        if self.points.is_empty() {
            return 0..0;
        }
        match self.x_mode {
            XMode::Index => index_range(range, self.points.len()),
            XMode::Explicit => {
                if !self.monotonic {
                    return 0..self.points.len();
                }
                let start = lower_bound(&self.points, range.min);
                let end = upper_bound(&self.points, range.max);
                start..end
            }
        }
    }

    /// Find the index of the point with nearest X value.
    pub fn nearest_index_by_x(&self, x: f64) -> Option<usize> {
        if self.points.is_empty() || !x.is_finite() {
            return None;
        }

        match self.x_mode {
            XMode::Index => {
                let max_index = self.points.len().saturating_sub(1) as f64;
                let clamped = x.round().clamp(0.0, max_index);
                Some(clamped as usize)
            }
            XMode::Explicit => {
                if !self.monotonic {
                    return self.nearest_index_linear(x);
                }
                let lower = lower_bound(&self.points, x);
                if lower == 0 {
                    return Some(0);
                }
                if lower >= self.points.len() {
                    return Some(self.points.len() - 1);
                }
                let left = lower - 1;
                let right = lower;
                let left_dist = (self.points[left].x - x).abs();
                let right_dist = (self.points[right].x - x).abs();
                if left_dist <= right_dist {
                    Some(left)
                } else {
                    Some(right)
                }
            }
        }
    }

    fn update_bounds(&mut self, point: Point) {
        match self.bounds {
            None => {
                self.bounds = Some(Viewport::new(
                    Range::new(point.x, point.x),
                    Range::new(point.y, point.y),
                ));
            }
            Some(mut bounds) => {
                bounds.x.expand_to_include(point.x);
                bounds.y.expand_to_include(point.y);
                self.bounds = Some(bounds);
            }
        }
    }

    fn nearest_index_linear(&self, x: f64) -> Option<usize> {
        let mut best_index = None;
        let mut best_distance = f64::INFINITY;
        for (index, point) in self.points.iter().enumerate() {
            let distance = (point.x - x).abs();
            if distance < best_distance {
                best_distance = distance;
                best_index = Some(index);
            }
        }
        best_index
    }
}

fn index_range(range: Range, len: usize) -> std::ops::Range<usize> {
    if len == 0 {
        return 0..0;
    }
    let min = range.min.ceil() as isize;
    let max = range.max.floor() as isize;
    if max < 0 || min > max {
        return 0..0;
    }
    let start = min.max(0) as usize;
    let end = (max as usize).saturating_add(1).min(len);
    start.min(end)..end
}

fn lower_bound(points: &[Point], target: f64) -> usize {
    let mut left = 0;
    let mut right = points.len();
    while left < right {
        let mid = (left + right) / 2;
        if points[mid].x < target {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    left
}

fn upper_bound(points: &[Point], target: f64) -> usize {
    let mut left = 0;
    let mut right = points.len();
    while left < right {
        let mid = (left + right) / 2;
        if points[mid].x <= target {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_range_matches_indices() {
        let data = AppendOnlyData::from_iter_y([1.0, 2.0, 3.0, 4.0]);
        let range = data.range_by_x(Range::new(1.0, 2.1));
        let slice = &data.points()[range];
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].x, 1.0);
        assert_eq!(slice[1].x, 2.0);
    }

    #[test]
    fn indexed_range_respects_fractional_bounds() {
        let data = AppendOnlyData::from_iter_y([1.0, 2.0, 3.0, 4.0, 5.0]);
        let range = data.range_by_x(Range::new(1.2, 3.8));
        let slice = &data.points()[range];
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].x, 2.0);
        assert_eq!(slice[1].x, 3.0);
    }

    #[test]
    fn explicit_range_uses_binary_search() {
        let points = [
            Point::new(0.0, 1.0),
            Point::new(1.0, 2.0),
            Point::new(2.0, 3.0),
            Point::new(3.0, 4.0),
        ];
        let data = AppendOnlyData::from_iter_points(points);
        let range = data.range_by_x(Range::new(0.5, 2.5));
        let slice = &data.points()[range];
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].x, 1.0);
        assert_eq!(slice[1].x, 2.0);
    }

    #[test]
    fn non_monotonic_explicit_marks_flag() {
        let mut data = AppendOnlyData::explicit();
        let _ = data.push_point(Point::new(1.0, 1.0));
        let result = data.push_point(Point::new(0.5, 2.0));
        assert_eq!(result, Err(AppendError::NonMonotonicX));
        assert!(!data.is_monotonic());
    }

    #[test]
    fn extend_y_appends_multiple_values() {
        let mut data = AppendOnlyData::indexed();
        let added = data.extend_y([1.0, 2.0, 3.0]).unwrap();
        assert_eq!(added, 3);
        assert_eq!(data.point(0), Some(Point::new(0.0, 1.0)));
        assert_eq!(data.point(2), Some(Point::new(2.0, 3.0)));
    }

    #[test]
    fn extend_points_non_monotonic_still_appends_batch() {
        let mut data = AppendOnlyData::explicit();
        let _ = data.extend_points([Point::new(1.0, 1.0), Point::new(2.0, 2.0)]);
        let result = data.extend_points([Point::new(1.5, 3.0), Point::new(4.0, 4.0)]);

        assert_eq!(result, Err(AppendError::NonMonotonicX));
        assert_eq!(data.len(), 4);
        assert_eq!(data.point(2), Some(Point::new(1.5, 3.0)));
        assert_eq!(data.point(3), Some(Point::new(4.0, 4.0)));
        assert!(!data.is_monotonic());
    }

    #[test]
    fn extend_points_wrong_mode_does_not_append() {
        let mut data = AppendOnlyData::indexed();
        let result = data.extend_points([Point::new(0.0, 1.0)]);
        assert_eq!(result, Err(AppendError::WrongMode));
        assert!(data.is_empty());
    }

    #[test]
    fn nearest_index_for_indexed_data_rounds() {
        let data = AppendOnlyData::from_iter_y([0.0, 1.0, 2.0, 3.0]);
        assert_eq!(data.nearest_index_by_x(2.4), Some(2));
        assert_eq!(data.nearest_index_by_x(2.6), Some(3));
        assert_eq!(data.nearest_index_by_x(-2.0), Some(0));
        assert_eq!(data.nearest_index_by_x(99.0), Some(3));
    }

    #[test]
    fn nearest_index_for_monotonic_explicit_data_uses_binary_search() {
        let data = AppendOnlyData::from_iter_points([
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(3.0, 3.0),
            Point::new(10.0, 4.0),
        ]);
        assert_eq!(data.nearest_index_by_x(2.2), Some(2));
        assert_eq!(data.nearest_index_by_x(8.0), Some(3));
        assert_eq!(data.nearest_index_by_x(-5.0), Some(0));
    }

    #[test]
    fn nearest_index_for_non_monotonic_explicit_data_falls_back_to_linear_scan() {
        let mut data = AppendOnlyData::explicit();
        let _ = data.extend_points([
            Point::new(0.0, 0.0),
            Point::new(5.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(10.0, 3.0),
        ]);
        assert_eq!(data.nearest_index_by_x(2.1), Some(2));
    }
}
