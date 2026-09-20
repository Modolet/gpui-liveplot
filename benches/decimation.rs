//! @file decimation.rs
//! @brief Reproducible viewport decimation benchmark using the private production pipeline
//! @author modolet <y@xxyx.io>
//! @date 2026-09-20

#![allow(dead_code)]
// A harness-free bench includes the production modules but does not collect
// their unit tests, leaving those modules' cfg(test) imports unused.
#![cfg_attr(test, allow(unused_imports))]

#[path = "../src/datasource/mod.rs"]
mod datasource;
#[path = "../src/geom.rs"]
mod geom;
#[path = "../src/view.rs"]
mod view;

use std::{hint::black_box, time::Instant};

use datasource::{AppendOnlyData, DecimationScratch, SeriesStore};
use geom::Point;
use view::Range;

fn waveform(t: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    0.12 * (tau * 0.37 * t + 1.4 * (tau * 0.11 * t).sin()).sin()
        + 0.16 * (60.0 * (tau * 1.7 * t).sin()).tanh() * (12.0 * (tau * 0.23 * t).sin()).tanh()
        + (0.04 + 0.18 * (tau * 0.61 * t).sin().powi(2)) * (tau * (15.0 * t + 160.0 * t * t)).sin()
        + 0.12 * (tau * (3300.0 * t - 160.0 * t * t)).sin()
        + 0.35
            * (-((t - 4.7) / 0.18).powi(2)).exp()
            * (tau * 9500.0 * t + 3.0 * (tau * 71.0 * t).sin()).sin()
        + 0.55 * (-((tau * 7.3 * t).sin() / 0.025).powi(2)).exp() * (tau * 1.13 * t).sin()
        + 0.16 * ((300.0 * (t - 5.2)).tanh() - (300.0 * (t - 5.6)).tanh())
        + 1.1 * (-((t - 2.137) / 0.00004).powi(2)).exp()
        - 1.1 * (-((t - 2.13714) / 0.00004).powi(2)).exp()
        - 1.2 * (-((t - 7.319) / 0.00004).powi(2)).exp()
        + 1.2 * (-((t - 7.31914) / 0.00004).powi(2)).exp()
}

fn main() {
    if std::env::args().any(|arg| arg == "--test") {
        return;
    }
    let dump_dir = std::env::var_os("LIVEPLOT_BENCH_DUMP").map(std::path::PathBuf::from);
    for count in [1_000_000, 10_000_000] {
        let rate = count as f64 / 10.0;
        let data = AppendOnlyData::from_iter_points((0..count).map(|i| {
            let t = i as f64 / rate;
            Point::new(t, waveform(t) as f32 as f64)
        }));
        let build = Instant::now();
        let store = SeriesStore::with_base_chunk(data, 64);
        println!(
            "points={count}, summary_build_ms={:.3}",
            build.elapsed().as_secs_f64() * 1000.0
        );
        for (name, range) in [
            ("overview", Range::new(0.0, 10.0)),
            ("zoom_a", Range::new(3.56, 5.37)),
            ("zoom_b", Range::new(3.67, 5.30)),
            ("detail", Range::new(4.699, 4.701)),
        ] {
            let mut scratch = DecimationScratch::new();
            let width = 1500;
            let output = store.decimate(range, width, &mut scratch);
            let len = output.len();
            if let Some(dir) = &dump_dir {
                std::fs::create_dir_all(dir).unwrap();
                use std::io::Write;
                let mut csv = std::io::BufWriter::new(
                    std::fs::File::create(dir.join(format!("{count}_{name}.csv"))).unwrap(),
                );
                for point in output {
                    writeln!(csv, "{},{}", point.x, point.y).unwrap();
                }
            }
            let mut batches = Vec::new();
            for _ in 0..7 {
                let start = Instant::now();
                for step in 0..200 {
                    let shift = step as f64 * range.span() * 0.00001;
                    black_box(store.decimate(
                        Range::new(range.min + shift, range.max + shift),
                        width,
                        &mut scratch,
                    ));
                }
                batches.push(start.elapsed().as_secs_f64() * 1_000_000.0 / 200.0);
            }
            batches.sort_by(f64::total_cmp);
            println!("{name}: median_us={:.3}, output_points={len}", batches[3]);
        }
    }
}
