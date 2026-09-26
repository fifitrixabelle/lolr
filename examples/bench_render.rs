//! Reproducible renderer baseline: cargo run --release --example bench_render

use std::hint::black_box;
use std::time::Instant;

use lolr::{render_line_styled, Gradient, RenderOpts, RenderStyle};

fn main() {
    const ITERATIONS: usize = 50_000;
    const MIXED: &str =
        "A colorful line with ANSI \x1b[1mstyle\x1b[0m, tabs\tand Unicode: café 界 🌈\n";
    const ASCII: &str = "A colorful line with ordinary ASCII text and a few more words.\n";

    for (name, sample) in [("ascii", ASCII), ("mixed", MIXED)] {
        for compact in [false, true] {
            for gradient in [Gradient::Rainbow, Gradient::Viridis] {
                let opts = RenderOpts {
                    gradient,
                    ..RenderOpts::default()
                };
                let style = RenderStyle {
                    compact,
                    ..RenderStyle::default()
                };
                let start = Instant::now();
                let mut output_bytes = 0;
                for line in 0..ITERATIONS {
                    output_bytes += black_box(render_line_styled(
                        black_box(sample),
                        line as f64,
                        black_box(&opts),
                        black_box(&style),
                    ))
                    .len();
                }
                let elapsed = start.elapsed();
                let input_mib = (sample.len() * ITERATIONS) as f64 / 1_048_576.0;
                println!(
                "{name} {gradient} compact={compact}: {elapsed:.2?}, {:.1} MiB/s input, {:.1} MiB output",
                input_mib / elapsed.as_secs_f64(),
                output_bytes as f64 / 1_048_576.0
            );
            }
        }
    }
}
