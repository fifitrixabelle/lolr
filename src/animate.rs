use std::io::{self, BufRead, Cursor, Write};
use std::thread;
use std::time::{Duration, Instant};

use anstyle_parse::{Params, Parser, Perform};
use clap::ValueEnum;
use crossterm::cursor::{Hide, RestorePosition, SavePosition, Show};
use crossterm::style::ResetColor;
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::QueueableCommand;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

use crate::gradient::Gradient;
use crate::render::{
    filter_replayed_animation_controls, render_line_styled, RenderOpts, RenderStyle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AnimationDirection {
    Forward,
    Reverse,
}

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
    animate(line, opts)
}

struct LineWidth {
    columns: usize,
    unsafe_control: bool,
    printable: String,
}

impl LineWidth {
    fn flush_printable(&mut self) {
        self.columns = self
            .columns
            .saturating_add(UnicodeWidthStr::width(self.printable.as_str()));
        self.printable.clear();
    }
}

impl Perform for LineWidth {
    fn print(&mut self, ch: char) {
        self.printable.push(ch);
    }

    fn execute(&mut self, byte: u8) {
        self.flush_printable();
        if byte == b'\t' {
            self.columns = self.columns.saturating_add(8);
        } else {
            self.unsafe_control = true;
        }
    }

    fn csi_dispatch(&mut self, _: &Params, _: &[u8], _: bool, action: u8) {
        self.flush_printable();
        if action != b'm' {
            self.unsafe_control = true;
        }
    }

    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {
        self.flush_printable();
        self.unsafe_control = true;
    }

    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {
        self.flush_printable();
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: u8) {
        self.flush_printable();
        self.unsafe_control = true;
    }
}

fn can_redraw(line: &str, terminal_columns: Option<usize>) -> bool {
    let Some(terminal_columns) = terminal_columns else {
        return false;
    };
    let mut parser: Parser = Parser::default();
    let mut width = LineWidth {
        columns: 0,
        unsafe_control: false,
        printable: String::new(),
    };
    for &byte in line.as_bytes() {
        parser.advance(&mut width, byte);
    }
    width.flush_printable();
    !width.unsafe_control && width.columns < terminal_columns
}

struct AnimationRun<'a> {
    opts: &'a AnimateOpts,
    style: &'a RenderStyle,
    direction: AnimationDirection,
}

fn animate_reader_to<W, R, S, C, Z>(
    stdout: &mut W,
    input: &mut R,
    run: AnimationRun<'_>,
    line_offset: &mut f64,
    mut sleep: S,
    cancelled: C,
    mut columns: Z,
) -> io::Result<()>
where
    W: Write,
    R: BufRead + ?Sized,
    S: FnMut(Duration),
    C: Fn() -> bool,
    Z: FnMut() -> Option<usize>,
{
    let opts = run.opts;
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
        let mut input_line = String::new();
        loop {
            input_line.clear();
            if input.read_line(&mut input_line)? == 0 {
                break;
            }
            let had_newline = input_line.ends_with('\n');
            let line = input_line.strip_suffix('\n').unwrap_or(&input_line);
            *line_offset += 1.0;
            if !line.is_empty() {
                if can_redraw(line, columns()) {
                    stdout.queue(SavePosition)?;
                    let replay_line = filter_replayed_animation_controls(line);
                    let started = Instant::now();
                    for frame in 1..=opts.duration {
                        if cancelled() {
                            return Ok(());
                        }
                        if frame > 1 {
                            let elapsed = frame_duration.mul_f64((frame - 1) as f64);
                            if let Some(deadline) = started.checked_add(elapsed) {
                                sleep(deadline.saturating_duration_since(Instant::now()));
                            }
                            if !can_redraw(line, columns()) {
                                break;
                            }
                        }
                        stdout
                            .queue(RestorePosition)?
                            .queue(Clear(ClearType::UntilNewLine))?;
                        let phase = frame as f64 * opts.spread;
                        let offset = match run.direction {
                            AnimationDirection::Forward => *line_offset + phase,
                            AnimationDirection::Reverse => *line_offset - phase,
                        };
                        let frame_line = if frame == 1 { line } else { &replay_line };
                        write!(
                            stdout,
                            "{}",
                            render_line_styled(frame_line, offset, &render_opts, run.style)
                        )?;
                        stdout.flush()?;
                    }
                } else {
                    write!(
                        stdout,
                        "{}",
                        render_line_styled(line, *line_offset, &render_opts, run.style)
                    )?;
                }
            }
            if had_newline {
                writeln!(stdout)?;
            }
            stdout.flush()?;
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
    let mut input = Cursor::new(text.as_bytes());
    animate_reader_until(&mut input, opts, cancelled)
}

/// Animate input as lines arrive, without buffering the complete stream.
pub fn animate_reader_until<R, C>(input: &mut R, opts: &AnimateOpts, cancelled: C) -> io::Result<()>
where
    R: BufRead + ?Sized,
    C: Fn() -> bool,
{
    let mut line_offset = opts.seed;
    animate_reader_styled_until(
        input,
        opts,
        &RenderStyle::default(),
        AnimationDirection::Forward,
        &mut line_offset,
        cancelled,
    )
}

/// Animate streamed input with a palette, contrast setting, and persistent line offset.
pub fn animate_reader_styled_until<R, C>(
    input: &mut R,
    opts: &AnimateOpts,
    style: &RenderStyle,
    direction: AnimationDirection,
    line_offset: &mut f64,
    cancelled: C,
) -> io::Result<()>
where
    R: BufRead + ?Sized,
    C: Fn() -> bool,
{
    animate_reader_to(
        &mut io::stdout(),
        input,
        AnimationRun {
            opts,
            style,
            direction,
        },
        line_offset,
        thread::sleep,
        cancelled,
        || terminal::size().ok().map(|(columns, _)| columns as usize),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_line;

    fn animate_default<W: Write, R: BufRead, S: FnMut(Duration), C: Fn() -> bool>(
        output: &mut W,
        input: &mut R,
        opts: &AnimateOpts,
        sleep: S,
        cancelled: C,
        columns: Option<usize>,
    ) -> io::Result<()> {
        let mut offset = opts.seed;
        animate_reader_to(
            output,
            input,
            AnimationRun {
                opts,
                style: &RenderStyle::default(),
                direction: AnimationDirection::Forward,
            },
            &mut offset,
            sleep,
            cancelled,
            || columns,
        )
    }

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
        animate_default(
            &mut output,
            &mut Cursor::new("x\n"),
            &opts,
            |_| {},
            || false,
            Some(80),
        )
        .unwrap();
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
        animate_default(
            &mut output,
            &mut Cursor::new("x"),
            &AnimateOpts::default(),
            |_| {},
            || true,
            Some(80),
        )
        .unwrap();
        assert!(output.ends_with(b"\x1b[0m\x1b[?25h"));
    }

    #[test]
    fn wide_and_wrapped_lines_render_once() {
        let opts = AnimateOpts {
            duration: 3,
            ..AnimateOpts::default()
        };
        let mut output = Vec::new();
        animate_default(
            &mut output,
            &mut Cursor::new("界界\nabcde\n"),
            &opts,
            |_| {},
            || false,
            Some(4),
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(!output.contains("\x1b[s"));
        assert_eq!(output.matches('界').count(), 2);
        assert_eq!(output.matches('a').count(), 1);
    }

    #[test]
    fn redraw_width_treats_joined_emoji_as_one_glyph() {
        assert!(can_redraw("👩‍💻", Some(3)));
        assert!(!can_redraw("👩‍💻", Some(2)));
    }

    #[test]
    fn reads_next_line_after_first_line_has_been_written() {
        struct StreamingInput {
            next: usize,
            output: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
        }
        impl io::Read for StreamingInput {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if self.next == 1 && !self.output.borrow().contains(&b'a') {
                    return Err(io::Error::other(
                        "first line was not rendered before next read",
                    ));
                }
                let lines = [b"a\n".as_slice(), b"b\n".as_slice()];
                if self.next == lines.len() {
                    return Ok(0);
                }
                let bytes = lines[self.next];
                buf[..bytes.len()].copy_from_slice(bytes);
                self.next += 1;
                Ok(bytes.len())
            }
        }
        struct SharedOutput(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
        impl Write for SharedOutput {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.borrow_mut().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let output = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut input = io::BufReader::with_capacity(
            2,
            StreamingInput {
                next: 0,
                output: output.clone(),
            },
        );
        animate_default(
            &mut SharedOutput(output),
            &mut input,
            &AnimateOpts {
                duration: 1,
                ..AnimateOpts::default()
            },
            |_| {},
            || false,
            Some(80),
        )
        .unwrap();
    }

    #[test]
    fn reverse_animation_uses_negative_phase_and_keeps_offset() {
        let opts = AnimateOpts {
            seed: 10.0,
            duration: 1,
            ..AnimateOpts::default()
        };
        let mut output = Vec::new();
        let mut offset = opts.seed;
        animate_reader_to(
            &mut output,
            &mut Cursor::new("x\n"),
            AnimationRun {
                opts: &opts,
                style: &RenderStyle::default(),
                direction: AnimationDirection::Reverse,
            },
            &mut offset,
            |_| {},
            || false,
            || Some(80),
        )
        .unwrap();
        assert_eq!(offset, 11.0);
        let expected = render_line(
            "x",
            11.0 - opts.spread,
            &RenderOpts {
                gradient: opts.gradient,
                spread: opts.spread,
                freq: opts.freq,
                truecolor: opts.truecolor,
                invert: opts.invert,
            },
        );
        assert!(String::from_utf8(output).unwrap().contains(&expected));
    }

    #[test]
    fn resize_stops_replaying_a_line_that_now_wraps() {
        let opts = AnimateOpts {
            duration: 3,
            ..AnimateOpts::default()
        };
        let mut output = Vec::new();
        let mut offset = 0.0;
        let checks = std::cell::Cell::new(0);
        animate_reader_to(
            &mut output,
            &mut Cursor::new("hello\n"),
            AnimationRun {
                opts: &opts,
                style: &RenderStyle::default(),
                direction: AnimationDirection::Forward,
            },
            &mut offset,
            |_| {},
            || false,
            || {
                let checked = checks.get();
                checks.set(checked + 1);
                Some(if checked == 0 { 80 } else { 4 })
            },
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches("\x1b8").count(), 1);
        assert!(output.ends_with("\x1b[0m\x1b[?25h"));
    }
}
