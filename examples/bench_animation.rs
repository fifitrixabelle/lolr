//! Includes line preparation, but excludes animation sleeps and terminal I/O.
use lolr::{render_line_styled_into, PreparedLine, RenderOpts, RenderStyle, Renderer};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let mixed = "A colorful line with ANSI \x1b[1mstyle\x1b[0m, tabs\tand Unicode: café 界 🌈";
    let long = mixed.repeat(16);
    for (name, sample, iterations) in [
        (
            "ascii",
            "A colorful line with ordinary ASCII text and a few more words.",
            10_000,
        ),
        ("mixed", mixed, 10_000),
        ("long-mixed", long.as_str(), 1_000),
    ] {
        for frames in [1, 2, 6, 12] {
            for truecolor in [false, true] {
                let opts = RenderOpts {
                    truecolor,
                    ..RenderOpts::default()
                };
                let style = RenderStyle {
                    compact: !truecolor,
                    ..RenderStyle::default()
                };
                for mode in ["output", "scratch", "prepared"] {
                    let mut output = Vec::new();
                    let mut renderer = Renderer::default();
                    let start = Instant::now();
                    for line in 0..iterations {
                        let prepared =
                            (mode == "prepared").then(|| PreparedLine::new(black_box(sample)));
                        for frame in 1..=frames {
                            let offset = line as f64 + frame as f64 * opts.spread;
                            if let Some(prepared) = &prepared {
                                prepared.render_into(
                                    offset,
                                    black_box(&opts),
                                    black_box(&style),
                                    &mut output,
                                );
                            } else if mode == "scratch" {
                                renderer.render_into(
                                    black_box(sample),
                                    offset,
                                    black_box(&opts),
                                    black_box(&style),
                                    &mut output,
                                );
                            } else {
                                render_line_styled_into(
                                    black_box(sample),
                                    offset,
                                    black_box(&opts),
                                    black_box(&style),
                                    &mut output,
                                );
                            }
                            black_box(&output);
                        }
                    }
                    let elapsed = start.elapsed();
                    let mib = (sample.len() * iterations * frames) as f64 / 1_048_576.0;
                    println!("{name} frames={frames} truecolor={truecolor} mode={mode}: {elapsed:.2?}, {:.1} MiB/s input", mib / elapsed.as_secs_f64());
                }
            }
        }
    }
}
