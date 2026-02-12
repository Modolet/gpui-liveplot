use std::time::{Duration, Instant};

use gpui_plot::{DecimationScratch, Range, SeriesStore};

fn main() {
    let mut series = SeriesStore::indexed();
    let mut scratch = DecimationScratch::new();

    let duration_secs: f64 = std::env::var("DURATION_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5.0);

    let target_rate = 30_000.0;
    let start = Instant::now();
    let mut next_report = Instant::now();
    let mut count = 0_usize;

    while start.elapsed().as_secs_f64() < duration_secs {
        let elapsed = start.elapsed().as_secs_f64();
        let target = (elapsed * target_rate) as usize;
        while count < target {
            let x = count as f64;
            let y = (x * 0.01).sin();
            let _ = series.push_y(y);
            count += 1;
        }

        if next_report.elapsed() >= Duration::from_secs(1) {
            let bounds = series
                .bounds()
                .map(|bounds| bounds.x)
                .unwrap_or(Range::new(0.0, 1.0));
            let decimated = series.decimate(bounds, 800, &mut scratch);
            println!(
                "points: {:>8} | decimated: {:>6}",
                series.data().len(),
                decimated.len()
            );
            next_report = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(2));
    }

    println!(
        "realtime append complete: {} points in {:.2}s",
        series.data().len(),
        duration_secs
    );
}
