//! @file paint.rs
//! @brief Stroke continuity, bounded joins and rendering performance regressions
//! @author modolet <y@xxyx.io>
//! @date 2026-09-20

use super::*;

fn covers(path: &gpui_kit::Path<Pixels>, x: f32, y: f32) -> bool {
    path.vertices.as_chunks::<3>().0.iter().any(|triangle| {
        let side = |a: &gpui_kit::PathVertex<Pixels>, b: &gpui_kit::PathVertex<Pixels>| {
            let ax = f32::from(a.xy_position.x);
            let ay = f32::from(a.xy_position.y);
            let bx = f32::from(b.xy_position.x);
            let by = f32::from(b.xy_position.y);
            (bx - ax) * (y - ay) - (by - ay) * (x - ax)
        };
        let a = side(&triangle[0], &triangle[1]);
        let b = side(&triangle[1], &triangle[2]);
        let c = side(&triangle[2], &triangle[0]);
        (a >= 0. && b >= 0. && c >= 0.) || (a <= 0. && b <= 0. && c <= 0.)
    })
}

#[test]
fn continuous_corner_covers_join_without_bridging_disconnected_segments() {
    let segments = [
        LineSegment::new(ScreenPoint::new(0., 0.), ScreenPoint::new(10., 0.)),
        LineSegment::new(ScreenPoint::new(10., 0.), ScreenPoint::new(10., 10.)),
        LineSegment::new(ScreenPoint::new(20., 10.), ScreenPoint::new(30., 10.)),
    ];
    let path = build_line_path(&segments, 2.);
    assert!(
        covers(&path, 10.25, -0.25),
        "outer corner must not have a crack"
    );
    assert!(
        !covers(&path, 15., 10.),
        "separate subpaths must stay separate"
    );
}

#[test]
fn sharp_peak_does_not_grow_a_miter_spike() {
    let segments = [
        LineSegment::new(ScreenPoint::new(0., 100.), ScreenPoint::new(1., 0.)),
        LineSegment::new(ScreenPoint::new(1., 0.), ScreenPoint::new(2., 100.)),
    ];
    let path = build_line_path(&segments, 2.);
    assert!(path.vertices.iter().all(|vertex| {
        let y = f32::from(vertex.xy_position.y);
        (-1.01..=101.01).contains(&y)
    }));
}

#[test]
fn dense_reversals_and_subpixel_segments_keep_their_coverage() {
    let points = (0..200)
        .map(|index| ScreenPoint::new(index as f32 * 0.03, if index % 2 == 0 { 0. } else { 100. }))
        .collect::<Vec<_>>();
    let segments = points
        .windows(2)
        .map(|pair| LineSegment::new(pair[0], pair[1]))
        .collect::<Vec<_>>();
    let path = build_line_path(&segments, 1.8);
    for segment in &segments {
        for fraction in [0.01, 0.25, 0.5, 0.75, 0.99] {
            assert!(covers(
                &path,
                segment.start.x + (segment.end.x - segment.start.x) * fraction,
                segment.start.y + (segment.end.y - segment.start.y) * fraction
            ));
        }
    }
    let short = [LineSegment::new(
        ScreenPoint::new(1., 1.),
        ScreenPoint::new(1.001, 1.001),
    )];
    assert!(covers(&build_line_path(&short, 1.8), 1.0005, 1.0005));
}

#[test]
fn large_paths_do_not_hit_a_sixteen_bit_tessellation_index_limit() {
    let segments = (0..20_000)
        .map(|index| {
            LineSegment::new(
                ScreenPoint::new(index as f32, 0.),
                ScreenPoint::new(index as f32 + 1., 1.),
            )
        })
        .collect::<Vec<_>>();
    let path = build_line_path(&segments, 1.8);
    assert_eq!(path.vertices.len(), segments.len() * 6);
}

#[test]
#[ignore = "manual baseline/candidate tessellation benchmark; see LIVEPLOT_RENDER_BENCH"]
fn waveform_tessellation_benchmark() {
    use std::{hint::black_box, io::Write, time::Instant};
    let Some(root) = std::env::var_os("LIVEPLOT_RENDER_BENCH") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    for (name, min, max) in [
        ("overview", 0., 10.),
        ("zoom_a", 3.56, 5.37),
        ("zoom_b", 3.67, 5.30),
    ] {
        for (variant, connected) in [("baseline", false), ("candidate", true)] {
            let raw = std::fs::read_to_string(root.join(format!("{variant}/1000000_{name}.csv")))
                .unwrap();
            let points = raw
                .lines()
                .map(|line| {
                    let (x, y) = line.split_once(',').unwrap();
                    ScreenPoint::new(
                        ((x.parse::<f64>().unwrap() - min) / (max - min) * 1500.) as f32,
                        ((1.3 - y.parse::<f64>().unwrap()) / 2.9 * 600.) as f32,
                    )
                })
                .collect::<Vec<_>>();
            let segments = points
                .windows(2)
                .map(|pair| LineSegment::new(pair[0], pair[1]))
                .collect::<Vec<_>>();
            let build = || {
                if connected {
                    build_line_path(&segments, 1.8)
                } else {
                    let mut builder = PathBuilder::stroke(px(1.8));
                    for segment in &segments {
                        builder.move_to(point(px(segment.start.x), px(segment.start.y)));
                        builder.line_to(point(px(segment.end.x), px(segment.end.y)));
                    }
                    builder.build().unwrap()
                }
            };
            let path = build();
            let mut times = Vec::new();
            for _ in 0..7 {
                let start = Instant::now();
                for _ in 0..50 {
                    black_box(build());
                }
                times.push(start.elapsed().as_secs_f64() * 1_000_000. / 50.);
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{variant}/{name}: tessellation_us={:.3}, triangles={}",
                times[3],
                path.vertices.len() / 3
            );
            let mut svg = std::io::BufWriter::new(
                std::fs::File::create(root.join(format!("{variant}_{name}.svg"))).unwrap(),
            );
            writeln!(svg, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1500\" height=\"600\"><rect width=\"1500\" height=\"600\" fill=\"#e4dbb0\"/><path fill=\"#cd5052\" d=\"").unwrap();
            for triangle in path.vertices.as_chunks::<3>().0 {
                let mut triangle = [&triangle[0], &triangle[1], &triangle[2]];
                let a = triangle[0].xy_position;
                let b = triangle[1].xy_position;
                let c = triangle[2].xy_position;
                // SVG nonzero filling must not cancel overlapping triangles
                // whose winding differs; the GPU draws them independently.
                let area = f32::from(b.x - a.x) * f32::from(c.y - a.y)
                    - f32::from(b.y - a.y) * f32::from(c.x - a.x);
                if area < 0. {
                    triangle.swap(1, 2);
                }
                for (index, vertex) in triangle.iter().enumerate() {
                    write!(
                        svg,
                        "{}{} {}",
                        if index == 0 { "M" } else { "L" },
                        f32::from(vertex.xy_position.x),
                        f32::from(vertex.xy_position.y)
                    )
                    .unwrap();
                }
                write!(svg, "Z").unwrap();
            }
            writeln!(svg, "\"/></svg>").unwrap();
        }
    }
}
