# gpui_plot

基于 GPUI 的高性能图表库（当前版本提供核心数据/坐标/抽稀/交互逻辑与示例，渲染后端可接入 GPUI）。

## 特性

- append-only 数据模型，适配传感器采样场景
- 视口级抽稀（min/max envelope）+ 多级摘要
- 线性/对数/时间轴（时间轴需启用 `time` feature）
- 交互逻辑：平移、缩放、框选、Pin 命中逻辑
- 渲染命令抽象与裁剪算法

## 快速开始

```rust
use gpui_plot::{LineStyle, Plot, Series, SeriesKind};

let mut plot = Plot::new();
let series = Series::from_iter_y(
    "sensor",
    (0..1000).map(|i| (i as f64 * 0.01).sin()),
    SeriesKind::Line(LineStyle::default()),
);
plot.add_series(series);
plot.refresh_viewport(0.05, 1e-6);
```

## Examples

- 基础示例：
  - `cargo run --example basic`
- 高频追加（默认 5s，可设置环境变量 `DURATION_SECS`）：
  - `DURATION_SECS=120 cargo run --example realtime_append`
- 多 series 与对数轴：
  - `cargo run --example multi_series`
- 时间轴（需 feature）：
  - `cargo run --example time_axis --features time`
- GPUI 交互演示（需 feature）：
  - `cargo run --example gpui_demo --features gpui`

## 开发自检

- `cargo check`
- `cargo clippy --all-targets`

## 验收操作（可复现步骤）

1. 运行 `cargo run --example realtime_append` 并观察每秒输出的 decimated 点数保持在 O(width) 规模。
2. 运行 `cargo run --example multi_series`，确认多条 series 同时参与抽稀与视口计算。
3. 启用 `time` feature 运行 `cargo run --example time_axis --features time`。
