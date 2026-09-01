use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::{RestorePosition, SavePosition},
    execute,
};

use crate::gradient::Gradient;
use crate::render::{RenderOpts, render_line};

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
            duration: 12,
            speed: 20.0,
            truecolor: true,
            invert: false,
        }
    }
}

pub fn animate(text: &str, opts: &AnimateOpts) -> io::Result<()> {
    let mut stdout = io::stdout();
    let frame_duration = Duration::from_secs_f64(1.0 / opts.speed);

    let render_opts = RenderOpts {
        gradient: opts.gradient,
        spread: opts.spread,
        freq: opts.freq,
        truecolor: opts.truecolor,
        invert: opts.invert,
    };

    execute!(stdout, SavePosition)?;

    for frame in 0..opts.duration {
        execute!(stdout, RestorePosition)?;

        let offset = opts.seed + frame as f64 * opts.spread;

        for line in text.lines() {
            let colored = render_line(line, offset, &render_opts);
            writeln!(stdout, "{}", colored)?;
        }

        stdout.flush()?;
        thread::sleep(frame_duration);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animate_opts_has_sane_defaults() {
        let opts = AnimateOpts::default();
        assert_eq!(opts.duration, 12);
        assert_eq!(opts.speed, 20.0);
        assert_eq!(opts.spread, 3.0);
    }
}
