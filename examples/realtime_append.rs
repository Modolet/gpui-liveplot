use std::time::{Duration, Instant};

use gpui_plot::{Plot, Series};

fn main() {
    let mut plot = Plot::new();
    plot.add_series(Series::line("sensor"));

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
            let y = (count as f64 * 0.01).sin();
            if let Some(series) = plot.series_mut().first_mut() {
                let _ = series.push_y(y);
            }
            count += 1;
        }

        if next_report.elapsed() >= Duration::from_secs(1) {
            let _ = plot.refresh_viewport(0.05, 1e-6);
            println!("points: {:>8}", count);
            next_report = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(2));
    }

    println!(
        "realtime append complete: {} points in {:.2}s",
        count,
        duration_secs
    );
}
