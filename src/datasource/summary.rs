//! Multi-level summaries and decimation helpers.

use crate::geom::Point;
use crate::view::Range;

/// Min/max envelope for a bucket of points.
///
/// This preserves extrema within a bucket and supports ordered emission to
/// maintain a non-self-intersecting polyline.
#[derive(Debug, Clone, Copy)]
pub struct MinMax {
    /// Minimum Y point within the bucket.
    pub min: Point,
    /// Maximum Y point within the bucket.
    pub max: Point,
    /// Covered X range for the bucket.
    pub x_range: Range,
}

impl MinMax {
    fn from_point(point: Point) -> Self {
        Self {
            min: point,
            max: point,
            x_range: Range::new(point.x, point.x),
        }
    }

    fn push(&mut self, point: Point) {
        if point.y < self.min.y {
            self.min = point;
        }
        if point.y > self.max.y {
            self.max = point;
        }
        self.x_range.max = point.x;
    }

    fn from_partial(partial: &PartialBucket) -> Self {
        Self {
            min: partial.min,
            max: partial.max,
            x_range: Range::new(partial.first_x, partial.last_x),
        }
    }

    fn merge(a: Self, b: Self) -> Self {
        let min = if a.min.y < b.min.y {
            a.min
        } else if b.min.y < a.min.y {
            b.min
        } else if a.min.x <= b.min.x {
            a.min
        } else {
            b.min
        };
        let max = if a.max.y > b.max.y {
            a.max
        } else if b.max.y > a.max.y {
            b.max
        } else if a.max.x <= b.max.x {
            a.max
        } else {
            b.max
        };
        Self {
            min,
            max,
            x_range: Range::new(
                a.x_range.min.min(b.x_range.min),
                a.x_range.max.max(b.x_range.max),
            ),
        }
    }

    /// Push min/max points in X order into the output.
    ///
    /// This maintains monotonic X order when emitting a min/max envelope.
    pub fn push_ordered(&self, out: &mut Vec<Point>) {
        if self.min == self.max {
            out.push(self.min);
            return;
        }
        if self.min.x <= self.max.x {
            out.push(self.min);
            out.push(self.max);
        } else {
            out.push(self.max);
            out.push(self.min);
        }
    }
}

#[derive(Debug, Clone)]
struct PartialBucket {
    count: usize,
    min: Point,
    max: Point,
    first_x: f64,
    last_x: f64,
}

impl PartialBucket {
    fn new(point: Point) -> Self {
        Self {
            count: 1,
            min: point,
            max: point,
            first_x: point.x,
            last_x: point.x,
        }
    }

    fn push(&mut self, point: Point) {
        self.count += 1;
        self.last_x = point.x;
        if point.y < self.min.y {
            self.min = point;
        }
        if point.y > self.max.y {
            self.max = point;
        }
    }
}

/// Summary bucket level.
///
/// Each level aggregates min/max buckets at a specific chunk size.
#[derive(Debug, Clone)]
pub struct SummaryLevel {
    chunk_size: usize,
    buckets: Vec<MinMax>,
}

impl SummaryLevel {
    fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            buckets: Vec::new(),
        }
    }

    pub(crate) fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub(crate) fn buckets(&self) -> &[MinMax] {
        &self.buckets
    }
}

/// Multi-level min/max summaries for append-only data.
///
/// Higher levels provide lower-resolution summaries that are cheaper to scan
/// when zoomed out.
#[derive(Debug, Clone)]
pub struct SummaryLevels {
    base_chunk: usize,
    levels: Vec<SummaryLevel>,
    partial: Option<PartialBucket>,
}

impl SummaryLevels {
    /// Create summaries with the given base chunk size.
    ///
    /// A smaller chunk size yields more accurate summaries but more work.
    pub fn new(base_chunk: usize) -> Self {
        let base_chunk = base_chunk.max(1);
        Self {
            base_chunk,
            levels: Vec::new(),
            partial: None,
        }
    }

    /// Base chunk size for the first level.
    pub fn base_chunk(&self) -> usize {
        self.base_chunk
    }

    /// Push a new point into the summaries.
    ///
    /// Appends are incremental and do not rebuild existing summaries.
    pub fn push(&mut self, point: Point) {
        match self.partial.as_mut() {
            None => {
                self.partial = Some(PartialBucket::new(point));
            }
            Some(partial) => {
                partial.push(point);
            }
        }
        if let Some(partial) = &self.partial
            && partial.count >= self.base_chunk
        {
            let bucket = MinMax::from_partial(partial);
            self.partial = None;
            self.push_bucket(0, bucket);
        }
    }

    /// Never choose a bucket coarser than the requested screen resolution.
    pub(crate) fn choose_level(&self, target_chunk: usize) -> Option<&SummaryLevel> {
        self.levels
            .iter()
            .rev()
            .find(|level| level.chunk_size <= target_chunk)
    }

    /// Query exact extrema without scanning complete summarized chunks.
    /// Only the two base-chunk boundaries are read from raw data. Aligned
    /// interior chunks are covered by the largest available summary nodes,
    /// including complete lower-level buckets in an unpaired streaming tail.
    pub(crate) fn extrema(
        &self,
        points: &[Point],
        range: std::ops::Range<usize>,
    ) -> Option<MinMax> {
        if range.is_empty() {
            return None;
        }
        let mut result = MinMax::from_point(points[range.start]);
        let mut index = range.start;
        while index < range.end {
            let remaining = range.end - index;
            if index.is_multiple_of(self.base_chunk)
                && remaining >= self.base_chunk
                && !self.levels.is_empty()
            {
                let aligned = (index / self.base_chunk).trailing_zeros() as usize;
                let fitting = (remaining / self.base_chunk).ilog2() as usize;
                let level = &self.levels[aligned.min(fitting).min(self.levels.len() - 1)];
                result = MinMax::merge(result, level.buckets[index / level.chunk_size]);
                index += level.chunk_size;
            } else {
                result.push(points[index]);
                index += 1;
            }
        }
        Some(result)
    }

    fn push_bucket(&mut self, level_index: usize, bucket: MinMax) {
        if self.levels.len() <= level_index {
            let chunk_size = self.base_chunk.saturating_mul(1 << level_index);
            self.levels.push(SummaryLevel::new(chunk_size));
        }
        let level = &mut self.levels[level_index];
        level.buckets.push(bucket);
        let len = level.buckets.len();
        if len.is_multiple_of(2) {
            let merged = MinMax::merge(level.buckets[len - 2], level.buckets[len - 1]);
            self.push_bucket(level_index + 1, merged);
        }
    }
}

/// Scratch buffers for decimation.
#[derive(Debug, Default, Clone)]
pub(crate) struct DecimationScratch {
    buckets: Vec<Bucket>,
    points: Vec<Point>,
}

impl DecimationScratch {
    /// Create an empty scratch buffer.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Clear the output points.
    pub(crate) fn clear(&mut self) {
        self.points.clear();
    }

    /// Access the output points.
    pub(crate) fn output(&self) -> &[Point] {
        &self.points
    }

    /// Access the output points mutably.
    pub(crate) fn output_mut(&mut self) -> &mut Vec<Point> {
        &mut self.points
    }
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    has_data: bool,
    min: Point,
    max: Point,
}

impl Default for Bucket {
    fn default() -> Self {
        Self {
            has_data: false,
            min: Point::new(0.0, 0.0),
            max: Point::new(0.0, 0.0),
        }
    }
}

impl Bucket {
    fn reset(&mut self) {
        self.has_data = false;
    }

    fn push(&mut self, point: Point) {
        if !self.has_data {
            self.has_data = true;
            self.min = point;
            self.max = point;
            return;
        }
        if point.y < self.min.y {
            self.min = point;
        }
        if point.y > self.max.y {
            self.max = point;
        }
    }

    fn push_ordered(&self, out: &mut Vec<Point>) {
        if !self.has_data {
            return;
        }
        if self.min == self.max {
            out.push(self.min);
            return;
        }
        if self.min.x <= self.max.x {
            out.push(self.min);
            out.push(self.max);
        } else {
            out.push(self.max);
            out.push(self.min);
        }
    }
}

/// Decimate points into a min/max envelope with approximately one bucket per pixel.
///
/// The output preserves extrema and is suitable for rendering dense lines at
/// interactive frame rates.
pub fn decimate_minmax<'a>(
    points: &[Point],
    x_range: Range,
    pixel_width: usize,
    scratch: &'a mut DecimationScratch,
) -> &'a [Point] {
    scratch.points.clear();
    if points.is_empty() || pixel_width == 0 {
        return scratch.output();
    }
    let span = x_range.span();
    if span <= 0.0 {
        scratch.points.extend_from_slice(points);
        return scratch.output();
    }

    if scratch.buckets.len() < pixel_width {
        scratch.buckets.resize(pixel_width, Bucket::default());
    }
    for bucket in scratch.buckets.iter_mut().take(pixel_width) {
        bucket.reset();
    }

    let width = pixel_width as f64;
    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() {
            continue;
        }
        let t = (point.x - x_range.min) / span;
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let mut index = (t * width) as usize;
        if index >= pixel_width {
            index = pixel_width - 1;
        }
        scratch.buckets[index].push(*point);
    }

    for bucket in scratch.buckets.iter().take(pixel_width) {
        bucket.push_ordered(&mut scratch.points);
    }

    scratch.output()
}

#[cfg(test)]
#[path = "tests/summary.rs"]
mod tests;
