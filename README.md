# gpui_plot

A high-performance plotting library for GPUI, focused on append-only sensor and
telemetry data. The core crate is backend-agnostic; a GPUI backend is included
for rendering and interaction.

## Highlights

- Append-only data model optimized for streaming workloads.
- Plot-level axes shared by all series (consistent transforms and formatting).
- Viewport-aware decimation and multi-level summaries for stable 60fps.
- Interactive pan, zoom, box zoom, hover readout, and pin annotations.
- Light/dark themes and configurable styling.

## Feature flags

- : Enable time-axis formatting via  and local
  offsets.

## Installation

Add to your :



Enable time axis formatting (optional):



## Quick start (plot only)



## Quick start (GPUI)



## Examples

Each example focuses on a single feature:

- Basic usage: 
- Append-only streaming + FollowLastN: 
- Time axis: 
- Pin annotations: 

Time formatting requires :



## Data model

- Append-only series data for high-throughput streaming.
- Two X modes:
  - Implicit X (index-based):  /  + .
  - Explicit X/Y:  or .
- Explicit X values are expected to be monotonic for fast range queries; the
  library will still render non-monotonic data but may fall back to full scans.

## View modes

- : Automatically fit all visible data (default).
- : View remains fixed; used after user interaction.
- : Follow last N points on X (oscilloscope-style).
- : Follow last N points on X and auto-scale Y.

## Interaction summary (GPUI backend)

- Left drag in plot area: pan.
- Right drag in plot area: box zoom.
- Mouse wheel:
  - Plot area: zoom both axes around cursor.
  - X axis: zoom X only.
  - Y axis: zoom Y only.
- Left click: pin nearest point (toggle).
- Double click in plot area: reset view (AutoAll).

## Performance model

- Viewport-aware decimation (min/max envelope) keeps line rendering near
  .
- Multi-level summaries provide efficient zoomed-out rendering.
- Render caches are keyed by viewport, size, and data generation.

## Theming

Use  or  and customize axis/grid colors as
needed. Themes apply to the whole plot.

## Limitations

- Append-only data model is the primary target; random inserts/deletes are not
  optimized.
- Only linear and time axes are supported in v0.1.

## Development checks

- 
- 

## Repro steps (manual QA)

1. Run  and verify the plot renders.
2. Run  and observe streaming updates.
3. Run  and verify time labels.
4. Run  and click on points to pin/unpin.
