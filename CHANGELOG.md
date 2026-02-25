# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-25

### Added

- Initial public release of `gpui-liveplot`.
- Backend-agnostic plot core for append-only telemetry and sensor streams.
- GPUI backend with interactive pan, zoom, box zoom, hover readout, and pinning.
- Plot-level shared axes and multiple view modes (`AutoAll`, `Manual`, `FollowLastN`, `FollowLastNXY`).
- Viewport-aware decimation, summary layers, and render caching for large datasets.
- Linked multi-plot synchronization via `PlotLinkGroup` and `PlotLinkOptions`.
- Runnable examples: `basic` and `advanced`.
