# gpui_plot

基于 GPUI 的高性能图表库（当前版本提供核心数据/坐标/抽稀/交互逻辑与示例，渲染后端可接入 GPUI）。

## 特性

- append-only 数据模型，适配传感器采样场景
- 视口级抽稀（min/max envelope）+ 多级摘要
- 线性/时间轴（时间轴需启用 `time` feature）
- 交互逻辑：平移、缩放、框选、Pin 命中逻辑
- GPUI 渲染与交互后端

## 快速开始

```rust
use gpui_plot::{LineStyle, Plot, Series, SeriesKind, Theme};

let mut plot = Plot::builder().theme(Theme::dark()).build();
let series = Series::from_iter_y(
    "sensor",
    (0..1000).map(|i| (i as f64 * 0.01).sin()),
    SeriesKind::Line(LineStyle::default()),
);
plot.add_series(series);
plot.refresh_viewport(0.05, 1e-6);
```

## Examples

- GUI 展示（包含实时追加、多序列、函数采样、时间轴与交互）：
  - `cargo run --example showcase`
  - 如需时间轴格式化：`cargo run --example showcase --features time`

## 开发自检

- `cargo check`
- `cargo clippy --all-targets`

## 验收操作（可复现步骤）

1. 运行 `cargo run --example showcase`，确认各 Tab 正常渲染与交互。
2. 切换 Live Stream，观察实时追加与 Pin/缩放/框选。
3. 启用 `time` feature 运行 `cargo run --example showcase --features time`，确认时间轴格式化。
