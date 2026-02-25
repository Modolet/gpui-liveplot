# gpui-liveplot

`gpui-liveplot` is a high-performance plotting library for GPUI, designed for
append-only telemetry and sensor streams.

It focuses on GPUI-native layout, rendering, and interaction for real-time
charts.

## Features

- Append-only data model optimized for streaming workloads.
- Shared plot-level axes across all series.
- Viewport-aware decimation with multi-level summaries for stable interaction at scale.
- Interactive pan, zoom, box-zoom, hover readout, and point pinning.
- Linked multi-plot interactions (`x/y` view sync, cursor sync, brush sync, reset sync).
- Configurable styles and dark/light themes.

## Installation

Add this crate to your project:

```toml
[dependencies]
gpui-liveplot = "0.1"
```

In Rust code, import it as `gpui_liveplot`:

```rust
use gpui_liveplot::{Plot, Series, SeriesKind};
```

## Quick Start

```rust
use gpui::{AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_liveplot::{AxisConfig, GpuiPlotView, Plot, Series, SeriesKind, Theme};

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

        cx.new(|_| GpuiPlotView::new(plot))
    })
    .unwrap();
});
```

## Streaming Data

`Series` is append-only. You can keep a shared handle and push new points over time.

- Implicit X mode: `Series::line` / `Series::scatter` + `push_y` / `extend_y`
- Explicit X/Y mode: `Series::from_iter_points` + `push_point` / `extend_points`

`Plot::add_series` stores a shared series handle, so appends from other handles
become visible immediately.

## View Modes

- `View::AutoAll` (default)
- `View::Manual`
- `View::FollowLastN`
- `View::FollowLastNXY`

## Interaction (GPUI Backend)

- Left drag in plot area: pan
- Right drag in plot area: box zoom
- Mouse wheel in plot area: zoom both axes around cursor
- Mouse wheel on axis area: zoom single axis
- Left click: toggle nearest-point pin
- Double click in plot area: reset view

## Multi-Plot Linking

Use `PlotLinkGroup` and `PlotLinkOptions` to link multiple `GpuiPlotView` instances.

See `examples/advanced.rs` for a complete linked-streaming demo.

## Examples

- Basic usage: `cargo run --example basic`
- Streaming + linked plots: `cargo run --example advanced`

## Performance Notes

- Line rendering is kept close to `O(plot_width)` through decimation.
- Multi-level summaries speed up zoomed-out views.
- Render caching is keyed by viewport, size, and data generation.

## Limitations

- Append-only workflows are the primary optimization target.
- Only linear axes are currently supported.

## Development

```bash
RUSTC_WRAPPER= cargo check
RUSTC_WRAPPER= cargo clippy --all-targets
cargo test
```

## License

MIT. See [LICENSE](LICENSE).
