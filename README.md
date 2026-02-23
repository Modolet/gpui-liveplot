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

- None at the moment.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
gpui_plot = "0.1"
```

## Quick start (plot only)

```rust
use gpui_plot::{LineStyle, Plot, Series, SeriesKind, Theme};

let mut plot = Plot::builder().theme(Theme::dark()).build();
let series = Series::from_iter_y(
    "sensor",
    (0..1000).map(|i| (i as f64 * 0.01).sin()),
    SeriesKind::Line(LineStyle::default()),
);
plot.add_series(&series);
plot.refresh_viewport(0.05, 1e-6);
```

## Quick start (GPUI)

```rust
use gpui::{AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_plot::{AxisConfig, GpuiPlotView, Plot, Series, SeriesKind, Theme};

Application::new().run(|cx| {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(720.0), px(480.0)),
            cx,
        ))),
        ..Default::default()
    };

    cx.open_window(options, |_window, cx| {
        let series = Series::from_iter_y(
            "signal",
            (0..400).map(|i| (i as f64 * 0.03).sin()),
            SeriesKind::Line(Default::default()),
        );

        let mut plot = Plot::builder()
            .theme(Theme::dark())
            .x_axis(AxisConfig::builder().title("Sample").build())
            .y_axis(AxisConfig::builder().title("Amplitude").build())
            .build();
        plot.add_series(&series);

        let view = GpuiPlotView::new(plot);
        cx.new(|_| view)
    })
    .unwrap();
});
```

## Examples

Each example focuses on a single feature:

- Basic usage: `cargo run --example basic`
- Append-only streaming + FollowLastN: `cargo run --example append_only`
- Pin annotations: `cargo run --example pins`
- Shared series across multiple plots: `cargo run --example shared_series`
- Linked plots (P0: linked X/reset): `cargo run --example linked_p0`
- Linked plots (P1: linked cursor/brush): `cargo run --example linked_p1`

## Data model

- Append-only series data for high-throughput streaming.
- Two X modes:
  - Implicit X (index-based): `Series::line` / `Series::scatter` + `push_y`.
  - Explicit X/Y: `Series::from_iter_points` or `push_point`.
- `Plot::add_series` always stores a shared-series handle.
- Explicit X values are expected to be monotonic for fast range queries; the
  library will still render non-monotonic data but may fall back to full scans.

## View modes

- `View::AutoAll`: Automatically fit all visible data (default).
- `View::Manual`: View remains fixed; used after user interaction.
- `View::FollowLastN`: Follow last N points on X (oscilloscope-style).
- `View::FollowLastNXY`: Follow last N points on X and auto-scale Y.

## Interaction summary (GPUI backend)

- Left drag in plot area: pan.
- Right drag in plot area: box zoom.
- Mouse wheel:
  - Plot area: zoom both axes around cursor.
  - X axis: zoom X only.
  - Y axis: zoom Y only.
- Left click: pin nearest point (toggle).
- Double click in plot area: reset view (AutoAll).

## Multi-plot linking (GPUI backend)

- Use `PlotLinkGroup` to attach multiple `GpuiPlotView` instances.
- Configure per-view behavior with `PlotLinkOptions`:
  - `link_x` / `link_y` for viewport sync
  - `link_cursor` for crosshair X sync
  - `link_brush` for brush range sync
  - `link_reset` for synchronized reset

## Performance model

- Viewport-aware decimation (min/max envelope) keeps line rendering near
  `O(plot_width)`.
- Multi-level summaries provide efficient zoomed-out rendering.
- Render caches are keyed by viewport, size, and data generation.

## Theming

Use `Theme::dark()` or `Theme::light()` and customize axis/grid colors as
needed. Themes apply to the whole plot.

## Limitations

- Append-only data model is the primary target; random inserts/deletes are not
  optimized.
- Only linear axes are supported in v0.1.

## Development checks

- `cargo check`
- `cargo clippy --all-targets`

## Repro steps (manual QA)

1. Run `cargo run --example basic` and verify the plot renders.
2. Run `cargo run --example append_only` and observe streaming updates.
3. Run `cargo run --example pins` and click on points to pin/unpin.
