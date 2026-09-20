# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-09-20

### Fixed

- Reduce hairline gaps in dense waveforms during zoom by choosing a summary level no coarser than the requested resolution.
- Preserve visible extrema at partially clipped summary buckets and in streaming tails whose lower-level buckets have not yet merged into higher levels.
- Draw line segments with direct stroke quads and bounded bevel joins, preserving short segments and sharp reversals without adding artificial miter spikes or bridging disconnected segments.
- Avoid the generic stroke tessellator's 16-bit vertex-index limit for large line paths.

### Performance

- Query only visible summary buckets instead of scanning the entire selected level.
- Keep decimation output bounded by approximately four points per plot pixel; fully zoomed-in views still use the original samples.
- Add a reproducible million-/ten-million-point zoom benchmark and regression coverage for extrema, level transitions, streaming tails, and stroke geometry.

## [0.2.3] - 2026-03-06

### Fixed

- Use clipped visible tick-label bounds for edge collision checks so partially visible axis labels do not hide as many neighboring labels.

## [0.2.2] - 2026-03-03

### Fixed

- Keep axis tick labels anchored to tick centers so out-of-range labels are clipped instead of being squeezed onto axis edges.

## [0.2.1] - 2026-03-03

### Fixed

- Clip X/Y axis tick rendering to their own axis regions to prevent Y-axis tick marks from leaking outside the chart area.

## [0.2.0] - 2026-03-02

### Added

- Optional feature `gpui_component_theme` to automatically consume `gpui-component` global theme when available.

### Changed

- Rename GPUI backend view type from `GpuiPlotView` to `PlotView`.
- Replace custom `Color` type with `gpui::Rgba` in public styling/rendering APIs.

### Fixed

- Clear hover tooltip state when cursor leaves plot interaction region.

## [0.1.1] - 2026-02-28

### Fixed

- Clear drag interaction when mouse button state no longer matches the active drag mode.
- Clear drag interaction on mouse-up events that occur outside the plot hitbox.

## [0.1.0] - 2026-02-25

### Added

- Initial public release of `gpui-liveplot`.
- Backend-agnostic plot core for append-only telemetry and sensor streams.
- GPUI backend with interactive pan, zoom, box zoom, hover readout, and pinning.
- Plot-level shared axes and multiple view modes (`AutoAll`, `Manual`, `FollowLastN`, `FollowLastNXY`).
- Viewport-aware decimation, summary layers, and render caching for large datasets.
- Linked multi-plot synchronization via `PlotLinkGroup` and `PlotLinkOptions`.
- Runnable examples: `basic` and `advanced`.
