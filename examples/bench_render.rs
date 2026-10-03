//! Reproducible renderer baseline: cargo run --release --example bench_render

use std::hint::black_box;
use std::time::Instant;

use lolr::{
    render_line_styled, render_line_styled_into, Background, Gradient, RenderOpts, RenderStyle,
    Renderer,
};

fn main() {
    const ITERATIONS: usize = 50_000;
    const MIXED: &str =
        "A colorful line with ANSI \x1b[1mstyle\x1b[0m, tabs\tand Unicode: café 界 🌈\n";
    const ASCII: &str = "A colorful line with ordinary ASCII text and a few more words.\n";

    let long_mixed = MIXED.repeat(16);
    for (name, sample, iterations) in [
        ("ascii", ASCII, ITERATIONS),
        ("mixed", MIXED, ITERATIONS),
        ("long-mixed", long_mixed.as_str(), 5_000),
    ] {
        for truecolor in [false, true] {
            for compact in [false, true] {
                for gradient in [Gradient::Rainbow, Gradient::Viridis] {
                    let opts = RenderOpts {
                        gradient,
                        truecolor,
                        ..RenderOpts::default()
                    };
                    let style = RenderStyle {
                        compact,
                        ..RenderStyle::default()
                    };
                    for reuse in ["fresh", "output", "scratch"] {
                        let mut output = Vec::new();
                        let mut renderer = Renderer::default();
                        let start = Instant::now();
                        let mut output_bytes = 0;
                        for line in 0..iterations {
                            output_bytes += if reuse == "scratch" {
                                renderer.render_into(
                                    black_box(sample),
                                    line as f64,
                                    black_box(&opts),
                                    black_box(&style),
                                    &mut output,
                                );
                                black_box(&output).len()
                            } else if reuse == "output" {
                                render_line_styled_into(
                                    black_box(sample),
                                    line as f64,
                                    black_box(&opts),
                                    black_box(&style),
                                    &mut output,
                                );
                                black_box(&output).len()
                            } else {
                                black_box(render_line_styled(
                                    black_box(sample),
                                    line as f64,
                                    black_box(&opts),
                                    black_box(&style),
                                ))
                                .len()
                            };
                        }
                        let elapsed = start.elapsed();
                        let input_mib = (sample.len() * iterations) as f64 / 1_048_576.0;
                        println!(
                            "{name} {gradient} truecolor={truecolor} compact={compact} reuse={reuse}: \
                             {elapsed:.2?}, {:.1} MiB/s input, {:.1} MiB output",
                            input_mib / elapsed.as_secs_f64(),
                            output_bytes as f64 / 1_048_576.0
                        );
                    }
                }
            }
        }
    }
    for background in [Background::Dark, Background::Light] {
        let opts = RenderOpts::default();
        let style = RenderStyle {
            background: Some(background),
            ..RenderStyle::default()
        };
        let start = Instant::now();
        for line in 0..ITERATIONS {
            black_box(render_line_styled(
                black_box(ASCII),
                line as f64,
                black_box(&opts),
                black_box(&style),
            ));
        }
        let elapsed = start.elapsed();
        println!(
            "ascii rainbow background={background:?}: {elapsed:.2?}, {:.1} MiB/s input",
            (ASCII.len() * ITERATIONS) as f64 / 1_048_576.0 / elapsed.as_secs_f64()
        );
    }
}
