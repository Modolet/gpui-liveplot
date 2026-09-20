//! Series storage combining raw data and summaries.

use crate::datasource::summary::{DecimationScratch, SummaryLevels, decimate_minmax};
use crate::datasource::{AppendError, AppendOnlyData, XMode};
use crate::geom::Point;
use crate::view::Range;

const DEFAULT_BASE_CHUNK: usize = 64;

/// Append-only series storage with summaries and generation tracking.
#[derive(Debug, Clone)]
pub(crate) struct SeriesStore {
    data: AppendOnlyData,
    summary: SummaryLevels,
    generation: u64,
}

impl SeriesStore {
    /// Create an indexed series store with default summary settings.
    pub fn indexed() -> Self {
        Self::with_base_chunk(AppendOnlyData::indexed(), DEFAULT_BASE_CHUNK)
    }

    /// Create a store from existing data and base chunk size.
    pub fn with_base_chunk(data: AppendOnlyData, base_chunk: usize) -> Self {
        let mut summary = SummaryLevels::new(base_chunk);
        for point in data.points() {
            summary.push(*point);
        }
        Self {
            data,
            summary,
            generation: 0,
        }
    }

    /// Append a Y value for indexed data.
    pub fn push_y(&mut self, y: f64) -> Result<usize, AppendError> {
        let result = self.data.push_y(y);
        if let Ok(index) = result
            && let Some(point) = self.data.point(index)
        {
            self.summary.push(point);
            self.generation = self.generation.wrapping_add(1);
        }
        result
    }

    /// Append multiple Y values for indexed data.
    pub fn extend_y<I, T>(&mut self, values: I) -> Result<usize, AppendError>
    where
        I: IntoIterator<Item = T>,
        T: Into<f64>,
    {
        let start_len = self.data.len();
        let result = self.data.extend_y(values);
        if result.is_ok() {
            self.update_summary_from(start_len);
        }
        result
    }

    /// Append an explicit point.
    pub fn push_point(&mut self, point: Point) -> Result<usize, AppendError> {
        let index = self.data.len();
        self.extend_points([point]).map(|_| index)
    }

    /// Append multiple explicit points.
    pub fn extend_points<I>(&mut self, points: I) -> Result<usize, AppendError>
    where
        I: IntoIterator<Item = Point>,
    {
        let start_len = self.data.len();
        let result = self.data.extend_points(points);
        if matches!(result, Ok(_) | Err(AppendError::NonMonotonicX)) {
            self.update_summary_from(start_len);
        }
        result
    }

    /// Access the underlying data.
    pub fn data(&self) -> &AppendOnlyData {
        &self.data
    }

    /// Access the series bounds.
    pub fn bounds(&self) -> Option<crate::view::Viewport> {
        self.data.bounds()
    }

    /// Access the data generation (increments on append).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Decimate data for rendering within an X range and pixel width.
    pub fn decimate<'a>(
        &self,
        x_range: Range,
        pixel_width: usize,
        scratch: &'a mut DecimationScratch,
    ) -> &'a [Point] {
        scratch.clear();
        if pixel_width == 0 || self.data.is_empty() {
            return scratch.output();
        }
        let index_range = self.data.range_by_x(x_range);
        let points = &self.data.points()[index_range.clone()];
        if points.is_empty() {
            return scratch.output();
        }
        if points.len() <= pixel_width.saturating_mul(2) {
            scratch.output_mut().extend_from_slice(points);
            return scratch.output();
        }
        if self.data.x_mode() == XMode::Explicit && !self.data.is_monotonic() {
            return decimate_minmax(points, x_range, pixel_width, scratch);
        }

        let target_bucket = (points.len() as f64 / pixel_width as f64).ceil() as usize;
        if target_bucket < self.summary.base_chunk() {
            return decimate_minmax(points, x_range, pixel_width, scratch);
        }
        if let Some(level) = self.summary.choose_level(target_bucket) {
            // Visit only complete visible buckets. Boundary and streaming-tail
            // ranges use finer summaries, so out-of-view peaks cannot replace
            // visible extrema and unpaired lower-level buckets are never lost.
            let chunk = level.chunk_size();
            let first = index_range.start.div_ceil(chunk);
            let last = index_range.end / chunk;
            let prefix_end = (first * chunk).min(index_range.end);
            if let Some(extrema) = self
                .summary
                .extrema(self.data.points(), index_range.start..prefix_end)
            {
                extrema.push_ordered(scratch.output_mut());
            }
            if first < last {
                for bucket in &level.buckets()[first..last] {
                    bucket.push_ordered(scratch.output_mut());
                }
            }
            let suffix_start = (last * chunk).max(prefix_end);
            if let Some(extrema) = self
                .summary
                .extrema(self.data.points(), suffix_start..index_range.end)
            {
                extrema.push_ordered(scratch.output_mut());
            }
            return scratch.output();
        }

        decimate_minmax(points, x_range, pixel_width, scratch)
    }

    fn update_summary_from(&mut self, start_len: usize) {
        let new_len = self.data.len();
        if new_len <= start_len {
            return;
        }
        for point in &self.data.points()[start_len..new_len] {
            self.summary.push(*point);
        }
        self.generation = self
            .generation
            .wrapping_add((new_len.saturating_sub(start_len)) as u64);
    }
}

#[cfg(test)]
#[path = "tests/store.rs"]
mod tests;
