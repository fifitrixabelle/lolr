use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::gradient::Gradient;
use crate::render::{render_line, RenderOpts};

#[derive(Debug, Clone)]
pub struct AnimateOpts {
    pub gradient: Gradient,
    pub spread: f64,
    pub freq: f64,
    pub seed: f64,
    pub duration: u32,
    pub speed: f64,
    pub truecolor: bool,
    pub invert: bool,
}

impl Default for AnimateOpts {
    fn default() -> Self {
        Self {
            gradient: Gradient::Rainbow,
            spread: 3.0,
            freq: 0.1,
            seed: 0.0,
            duration: 6,
            speed: 40.0,
            truecolor: true,
            invert: false,
        }
    }
}

pub fn animate_line(line: &str, opts: &AnimateOpts) -> io::Result<()> {
    let mut stdout = io::stdout();
    let safe_speed = opts.speed.max(0.1);
    let frame_duration = Duration::from_secs_f64(1.0 / safe_speed);

    let render_opts = RenderOpts {
        gradient: opts.gradient,
        spread: opts.spread,
        freq: opts.freq,
        truecolor: opts.truecolor,
        invert: opts.invert,
    };

    // Hide cursor, save position (DECSC)
    write!(stdout, "\x1b[?25l\x1b7")?;

    for frame in 0..opts.duration {
        // Restore cursor (DECRC), clear to end of line
        write!(stdout, "\x1b8\x1b[K")?;

        let offset = opts.seed + frame as f64 * opts.spread;
        let colored = render_line(line, offset, &render_opts);
        write!(stdout, "{}", colored)?;
        stdout.flush()?;

        thread::sleep(frame_duration);
    }

    // Show cursor, newline
    writeln!(stdout, "\x1b[?25h")?;
    Ok(())
}

pub fn animate(text: &str, opts: &AnimateOpts) -> io::Result<()> {
    let mut offset = opts.seed;
    for line in text.lines() {
        let line_opts = AnimateOpts {
            seed: offset,
            ..opts.clone()
        };
        animate_line(line, &line_opts)?;
        offset += 1.0;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animate_opts_has_sane_defaults() {
        let opts = AnimateOpts::default();
        assert_eq!(opts.duration, 6);
        assert_eq!(opts.speed, 40.0);
        assert_eq!(opts.spread, 3.0);
    }
}
