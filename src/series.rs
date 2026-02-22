//! Data series configuration and storage.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::datasource::{AppendError, AppendOnlyData, SeriesStore};
use crate::geom::Point;
use crate::render::{LineStyle, MarkerStyle};
use crate::view::Viewport;

static SERIES_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a series.
///
/// Series IDs are stable within a process and are used to bind pins to
/// specific series and point indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeriesId(u64);

impl SeriesId {
    fn next() -> Self {
        Self(SERIES_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Series rendering kind.
///
/// A series always has exactly one rendering kind.
#[derive(Debug, Clone)]
pub enum SeriesKind {
    /// Line series with styling.
    Line(LineStyle),
    /// Scatter series with styling.
    Scatter(MarkerStyle),
}

/// Plot series with data storage and styling.
///
/// Series own their data and provide append-only methods for streaming
/// workloads. All axes and transforms are handled at the plot level.
///
/// Use [`Series::share`] when multiple plots should observe the same live
/// append-only stream. By contrast, [`Clone`] creates an independent copy.
#[derive(Debug)]
pub struct Series {
    id: SeriesId,
    name: String,
    kind: SeriesKind,
    data: Arc<RwLock<SeriesStore>>,
    visible: bool,
}

impl Series {
    /// Create a line series with indexed data.
    ///
    /// Indexed data uses implicit X values (0, 1, 2, ...).
    pub fn line(name: impl Into<String>) -> Self {
        Self {
            id: SeriesId::next(),
            name: name.into(),
            kind: SeriesKind::Line(LineStyle::default()),
            data: Arc::new(RwLock::new(SeriesStore::indexed())),
            visible: true,
        }
    }

    /// Create a scatter series with indexed data.
    ///
    /// Indexed data uses implicit X values (0, 1, 2, ...).
    pub fn scatter(name: impl Into<String>) -> Self {
        Self {
            id: SeriesId::next(),
            name: name.into(),
            kind: SeriesKind::Scatter(MarkerStyle::default()),
            data: Arc::new(RwLock::new(SeriesStore::indexed())),
            visible: true,
        }
    }

    /// Create a series from existing append-only data.
    pub(crate) fn with_data(
        name: impl Into<String>,
        data: AppendOnlyData,
        kind: SeriesKind,
    ) -> Self {
        Self {
            id: SeriesId::next(),
            name: name.into(),
            kind,
            data: Arc::new(RwLock::new(SeriesStore::with_base_chunk(data, 64))),
            visible: true,
        }
    }

    /// Build a series from an iterator of Y values.
    ///
    /// X values are assigned as implicit indices.
    pub fn from_iter_y<I, T>(name: impl Into<String>, iter: I, kind: SeriesKind) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<f64>,
    {
        let data = AppendOnlyData::from_iter_y(iter);
        Self::with_data(name, data, kind)
    }

    /// Build a series from an iterator of points.
    ///
    /// X values are taken from each [`Point`](crate::geom::Point).
    pub fn from_iter_points<I>(name: impl Into<String>, iter: I, kind: SeriesKind) -> Self
    where
        I: IntoIterator<Item = Point>,
    {
        let data = AppendOnlyData::from_iter_points(iter);
        Self::with_data(name, data, kind)
    }

    /// Build a series by sampling a callback function.
    ///
    /// The callback is sampled uniformly across `x_range`.
    pub fn from_explicit_callback(
        name: impl Into<String>,
        function: impl Fn(f64) -> f64,
        x_range: crate::view::Range,
        points: usize,
        kind: SeriesKind,
    ) -> Self {
        let data = AppendOnlyData::from_explicit_callback(function, x_range, points);
        Self::with_data(name, data, kind)
    }

    /// Access the series identifier.
    pub fn id(&self) -> SeriesId {
        self.id
    }

    /// Access the series name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Access the series kind.
    pub fn kind(&self) -> &SeriesKind {
        &self.kind
    }

    /// Replace the series kind.
    pub fn with_kind(mut self, kind: SeriesKind) -> Self {
        self.kind = kind;
        self
    }

    /// Create another series handle that shares the same append-only data.
    ///
    /// The returned series receives a new [`SeriesId`], so it can coexist with
    /// the source series in the same plot. Data appends through either series
    /// are immediately visible to all shared handles.
    pub fn share(&self) -> Self {
        Self {
            id: SeriesId::next(),
            name: self.name.clone(),
            kind: self.kind.clone(),
            data: Arc::clone(&self.data),
            visible: self.visible,
        }
    }

    /// Access the underlying series store.
    pub(crate) fn with_store<R>(&self, f: impl FnOnce(&SeriesStore) -> R) -> R {
        let data = self.data.read().expect("series data lock");
        f(&data)
    }

    /// Append a Y value to an indexed series.
    pub fn push_y(&mut self, y: f64) -> Result<usize, AppendError> {
        self.with_store_mut(|data| data.push_y(y))
    }

    /// Append multiple Y values to an indexed series.
    ///
    /// Returns the number of appended points.
    pub fn extend_y<I, T>(&mut self, values: I) -> Result<usize, AppendError>
    where
        I: IntoIterator<Item = T>,
        T: Into<f64>,
    {
        self.with_store_mut(|data| data.extend_y(values))
    }

    /// Append a point to an explicit series.
    pub fn push_point(&mut self, point: Point) -> Result<usize, AppendError> {
        self.with_store_mut(|data| data.push_point(point))
    }

    /// Append multiple explicit points to a series.
    ///
    /// Returns the number of appended points when X values stay monotonic.
    /// If any new point has a smaller X than the previous point, all points are
    /// still appended and [`AppendError::NonMonotonicX`] is returned.
    pub fn extend_points<I>(&mut self, points: I) -> Result<usize, AppendError>
    where
        I: IntoIterator<Item = Point>,
    {
        self.with_store_mut(|data| data.extend_points(points))
    }

    /// Access the series bounds.
    pub fn bounds(&self) -> Option<Viewport> {
        self.with_store(SeriesStore::bounds)
    }

    /// Access the series generation.
    ///
    /// This monotonically increasing value is used for render cache invalidation.
    pub fn generation(&self) -> u64 {
        self.with_store(SeriesStore::generation)
    }

    /// Check if the series is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Toggle series visibility.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn with_store_mut<R>(&self, f: impl FnOnce(&mut SeriesStore) -> R) -> R {
        let mut data = self.data.write().expect("series data lock");
        f(&mut data)
    }
}

impl Clone for Series {
    fn clone(&self) -> Self {
        let data = self.data.read().expect("series data lock").clone();
        Self {
            id: self.id,
            name: self.name.clone(),
            kind: self.kind.clone(),
            data: Arc::new(RwLock::new(data)),
            visible: self.visible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_observes_appends_from_source() {
        let mut source = Series::line("shared");
        let mut shared = source.share();

        let _ = source.extend_y([1.0, 2.0, 3.0]);
        assert_eq!(shared.generation(), 3);

        let _ = shared.push_y(4.0);
        assert_eq!(source.generation(), 4);
        assert_eq!(source.bounds(), shared.bounds());
    }

    #[test]
    fn clone_is_independent_copy() {
        let mut source = Series::line("sensor");
        let mut cloned = source.clone();

        let _ = source.push_y(1.0);
        assert_eq!(source.generation(), 1);
        assert_eq!(cloned.generation(), 0);

        let _ = cloned.push_y(2.0);
        assert_eq!(source.generation(), 1);
        assert_eq!(cloned.generation(), 1);
    }
}
