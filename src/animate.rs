use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crossterm::cursor::{Hide, RestorePosition, SavePosition, Show};
use crossterm::style::ResetColor;
use crossterm::terminal::{Clear, ClearType};
use crossterm::QueueableCommand;

use crate::gradient::Gradient;
use crate::render::{filter_replayed_animation_controls, render_line, RenderOpts};

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
    animate_lines_to(&mut stdout, [(line, false)], opts, thread::sleep, || false)
}

fn animate_lines_to<'a, W, I, S, C>(
    stdout: &mut W,
    lines: I,
    opts: &AnimateOpts,
    mut sleep: S,
    cancelled: C,
) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = (&'a str, bool)>,
    S: FnMut(Duration),
    C: Fn() -> bool,
{
    let frame_duration = Duration::from_secs_f64(1.0 / opts.speed);

    let render_opts = RenderOpts {
        gradient: opts.gradient,
        spread: opts.spread,
        freq: opts.freq,
        truecolor: opts.truecolor,
        invert: opts.invert,
    };

    stdout.queue(Hide)?;
    let animation_result = (|| {
        let mut line_offset = opts.seed;
        for (line, had_newline) in lines {
            line_offset += 1.0;
            if !line.is_empty() {
                stdout.queue(SavePosition)?;
                let replay_line = filter_replayed_animation_controls(line);
                for frame in 1..=opts.duration {
                    if cancelled() {
                        return Ok(());
                    }
                    stdout
                        .queue(RestorePosition)?
                        .queue(Clear(ClearType::UntilNewLine))?;
                    let offset = line_offset + frame as f64 * opts.spread;
                    let frame_line = if frame == 1 { line } else { &replay_line };
                    write!(stdout, "{}", render_line(frame_line, offset, &render_opts))?;
                    stdout.flush()?;
                    sleep(frame_duration);
                }
            }
            if had_newline {
                writeln!(stdout)?;
            }
        }
        Ok(())
    })();

    // Always attempt to restore terminal state on ordinary I/O errors.
    let cleanup_result = stdout
        .queue(ResetColor)
        .and_then(|stdout| stdout.queue(Show))
        .and_then(Write::flush);
    animation_result.and(cleanup_result)
}

pub fn animate(text: &str, opts: &AnimateOpts) -> io::Result<()> {
    animate_until(text, opts, || false)
}

/// Animate until all frames have rendered or `cancelled` becomes true.
/// Terminal colors and cursor visibility are restored in either case.
pub fn animate_until<C>(text: &str, opts: &AnimateOpts, cancelled: C) -> io::Result<()>
where
    C: Fn() -> bool,
{
    let mut lines = Vec::new();
    let mut remaining = text;
    while let Some(newline) = remaining.find('\n') {
        lines.push((&remaining[..newline], true));
        remaining = &remaining[newline + 1..];
    }
    if !remaining.is_empty() {
        lines.push((remaining, false));
    }
    animate_lines_to(&mut io::stdout(), lines, opts, thread::sleep, cancelled)
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

    #[test]
    fn animation_uses_ruby_phase_and_preserves_final_newline() {
        let opts = AnimateOpts {
            duration: 1,
            speed: 20.0,
            truecolor: true,
            seed: 10.0,
            ..AnimateOpts::default()
        };
        let mut output = Vec::new();
        animate_lines_to(&mut output, [("x", true)], &opts, |_| {}, || false).unwrap();
        let output = String::from_utf8(output).unwrap();
        let expected = render_line(
            "x",
            10.0 + 1.0 + opts.spread,
            &RenderOpts {
                gradient: opts.gradient,
                spread: opts.spread,
                freq: opts.freq,
                truecolor: opts.truecolor,
                invert: opts.invert,
            },
        );
        assert!(output.contains(&expected));
        assert!(output.contains(&format!("{expected}\n")));
        assert!(output.ends_with("\x1b[0m\x1b[?25h"));
    }

    #[test]
    fn cancelled_animation_restores_the_cursor() {
        let mut output = Vec::new();
        animate_lines_to(
            &mut output,
            [("x", false)],
            &AnimateOpts::default(),
            |_| {},
            || true,
        )
        .unwrap();
        assert!(output.ends_with(b"\x1b[0m\x1b[?25h"));
    }
}
