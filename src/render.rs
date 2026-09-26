use crate::color::{rgb_to_256, Rgb};
use crate::gradient::{gradient_color, Gradient, Palette};
use anstyle_parse::{Params, Parser, Perform};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::io::Write;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub struct RenderOpts {
    pub gradient: Gradient,
    pub spread: f64,
    pub freq: f64,
    pub truecolor: bool,
    pub invert: bool,
}

impl Default for RenderOpts {
    fn default() -> Self {
        Self {
            gradient: Gradient::Rainbow,
            spread: 3.0,
            freq: 0.1,
            truecolor: true,
            invert: false,
        }
    }
}

/// Assumed terminal background for optional contrast adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Background {
    Dark,
    Light,
}

#[derive(Debug, Clone, Default)]
pub struct RenderStyle {
    pub palette: Option<Palette>,
    pub background: Option<Background>,
    pub compact: bool,
}

fn luminance(rgb: Rgb) -> f64 {
    let linear = |channel: u8| {
        let value = channel as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b)
}

fn ensure_contrast(rgb: Rgb, background: Background) -> Rgb {
    let target = match background {
        Background::Dark => 0.175,
        Background::Light => 1.05 / 4.5 - 0.05,
    };
    let current = luminance(rgb);
    if matches!(background, Background::Dark) && current >= target
        || matches!(background, Background::Light) && current <= target
    {
        return rgb;
    }

    let end = match background {
        Background::Dark => 255.0,
        Background::Light => 0.0,
    };
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..12 {
        let middle = (low + high) / 2.0;
        let blend = |channel: u8| (channel as f64 + (end - channel as f64) * middle).round() as u8;
        let candidate = Rgb {
            r: blend(rgb.r),
            g: blend(rgb.g),
            b: blend(rgb.b),
        };
        let enough = match background {
            Background::Dark => luminance(candidate) >= target,
            Background::Light => luminance(candidate) <= target,
        };
        if enough {
            high = middle;
        } else {
            low = middle;
        }
    }
    let blend = |channel: u8| (channel as f64 + (end - channel as f64) * high).round() as u8;
    Rgb {
        r: blend(rgb.r),
        g: blend(rgb.g),
        b: blend(rgb.b),
    }
}

fn write_color(output: &mut Vec<u8>, rgb: Rgb, truecolor: bool, invert: bool) {
    if invert {
        if truecolor {
            write!(output, "\x1b[48;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
        } else {
            write!(output, "\x1b[48;5;{}m", rgb_to_256(rgb))
        }
    } else if truecolor {
        write!(output, "\x1b[38;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
    } else {
        write!(output, "\x1b[38;5;{}m", rgb_to_256(rgb))
    }
    .expect("writing to a Vec must succeed");
}

struct Colorize<'a> {
    output: Vec<u8>,
    pending: Vec<u8>,
    text_run: Vec<u8>,
    offset: f64,
    col: usize,
    opts: &'a RenderOpts,
    style: &'a RenderStyle,
    last_color: Option<OutputColor>,
    fast_ascii: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputColor {
    Truecolor(Rgb),
    Ansi256(u8),
}

impl Colorize<'_> {
    fn flush_control(&mut self) {
        self.flush_text();
        self.reset_compact();
        self.output.append(&mut self.pending);
    }

    fn reset_compact(&mut self) {
        if self.last_color.take().is_some() {
            self.output.extend_from_slice(if self.opts.invert {
                b"\x1b[49m"
            } else {
                b"\x1b[39m"
            });
        }
    }

    fn flush_text(&mut self) {
        if self.text_run.is_empty() {
            return;
        }
        let mut run = std::mem::take(&mut self.text_run);
        if run.is_ascii() {
            for &byte in &run {
                self.emit_unit(&[byte], 1);
            }
        } else {
            let text = std::str::from_utf8(&run).expect("parser printable text is UTF-8");
            for grapheme in UnicodeSegmentation::graphemes(text, true) {
                self.emit_unit(grapheme.as_bytes(), UnicodeWidthStr::width(grapheme));
            }
        }
        run.clear();
        self.text_run = run;
    }

    fn emit_unit(&mut self, bytes: &[u8], width: usize) {
        let i = self.offset + self.col as f64 / self.opts.spread;
        let rgb = self.style.palette.as_ref().map_or_else(
            || gradient_color(self.opts.gradient, self.opts.freq, i),
            |palette| palette.color(self.opts.freq, i),
        );
        let rgb = self
            .style
            .background
            .map_or(rgb, |background| ensure_contrast(rgb, background));
        if !self.style.compact {
            write_color(&mut self.output, rgb, self.opts.truecolor, self.opts.invert);
            self.output.extend_from_slice(bytes);
            self.output.extend_from_slice(if self.opts.invert {
                b"\x1b[49m"
            } else {
                b"\x1b[39m"
            });
            self.col += width;
            return;
        }
        let key = if self.opts.truecolor {
            OutputColor::Truecolor(rgb)
        } else {
            OutputColor::Ansi256(rgb_to_256(rgb))
        };
        if self.last_color != Some(key) {
            write_color(&mut self.output, rgb, self.opts.truecolor, self.opts.invert);
        }
        self.output.extend_from_slice(bytes);
        self.last_color = Some(key);
        self.col += width;
    }
}

impl Perform for Colorize<'_> {
    fn print(&mut self, ch: char) {
        let char_start = self.pending.len() - ch.len_utf8();
        if self.fast_ascii {
            if char_start > 0 {
                self.reset_compact();
                self.output.extend_from_slice(&self.pending[..char_start]);
            }
            let byte = self.pending[char_start];
            self.emit_unit(&[byte], 1);
            self.pending.clear();
            return;
        }
        if char_start > 0 {
            self.flush_text();
            self.reset_compact();
            self.output.extend_from_slice(&self.pending[..char_start]);
        }
        self.text_run.extend_from_slice(&self.pending[char_start..]);
        self.pending.clear();
    }

    fn execute(&mut self, _byte: u8) {
        self.flush_control();
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: u8) {
        self.flush_control();
    }

    fn put(&mut self, _byte: u8) {
        self.flush_control();
    }

    fn unhook(&mut self) {
        self.flush_control();
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        self.flush_control();
    }

    fn csi_dispatch(
        &mut self,
        _params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: u8,
    ) {
        self.flush_control();
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        self.flush_control();
    }
}

struct AnimationFilter {
    output: Vec<u8>,
    pending: Vec<u8>,
}

impl AnimationFilter {
    fn flush(&mut self) {
        self.output.append(&mut self.pending);
    }
}

impl Perform for AnimationFilter {
    fn print(&mut self, _ch: char) {
        self.flush();
    }

    fn execute(&mut self, _byte: u8) {
        self.flush();
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: u8) {
        self.flush();
    }

    fn put(&mut self, _byte: u8) {
        self.flush();
    }

    fn unhook(&mut self) {
        self.flush();
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        self.flush();
    }

    fn csi_dispatch(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, action: u8) {
        if matches!(action, b'@' | b'J' | b'K' | b'P' | b'X') {
            self.pending.clear();
        } else {
            self.flush();
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        self.flush();
    }
}

/// Remove terminal-editing sequences which must not be replayed on every
/// animation frame. The first frame still receives the original input.
pub(crate) fn filter_replayed_animation_controls(input: &str) -> String {
    let mut parser: Parser = Parser::default();
    let mut filter = AnimationFilter {
        output: Vec::with_capacity(input.len()),
        pending: Vec::new(),
    };
    for &byte in input.as_bytes() {
        filter.pending.push(byte);
        parser.advance(&mut filter, byte);
    }
    filter.flush();
    String::from_utf8(filter.output).expect("filtering valid UTF-8 must produce valid UTF-8")
}

pub fn render_line(line: &str, offset: f64, opts: &RenderOpts) -> String {
    render_line_styled(line, offset, opts, &RenderStyle::default())
}

pub fn render_line_styled(
    line: &str,
    offset: f64,
    opts: &RenderOpts,
    style: &RenderStyle,
) -> String {
    let mut parser: Parser = Parser::default();
    let mut performer = Colorize {
        output: Vec::with_capacity(line.len()),
        pending: Vec::new(),
        text_run: Vec::new(),
        offset,
        col: 0,
        opts,
        style,
        last_color: None,
        fast_ascii: line.is_ascii(),
    };

    for &byte in line.as_bytes() {
        // Ruby lolcat expands every tab to eight spaces before parsing ANSI.
        let bytes: &[u8] = if byte == b'\t' { b"        " } else { &[byte] };
        for &expanded in bytes {
            performer.pending.push(expanded);
            parser.advance(&mut performer, expanded);
        }
    }
    performer.flush_control();

    String::from_utf8(performer.output).expect("rendering valid UTF-8 must produce valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_ansi_codes() {
        let opts = RenderOpts::default();
        let result = render_line("Hi", 0.0, &opts);
        assert!(result.contains("\x1b["));
        assert!(result.contains("H"));
        assert!(result.contains("i"));
    }

    #[test]
    fn render_truecolor_uses_rgb() {
        let opts = RenderOpts {
            truecolor: true,
            ..Default::default()
        };
        let result = render_line("A", 0.0, &opts);
        assert!(result.contains("\x1b[38;2;"));
    }

    #[test]
    fn render_256_uses_256_code() {
        let opts = RenderOpts {
            truecolor: false,
            ..Default::default()
        };
        let result = render_line("A", 0.0, &opts);
        assert!(result.contains("\x1b[38;5;"));
    }

    #[test]
    fn render_preserves_newlines() {
        let opts = RenderOpts::default();
        let result = render_line("a\nb", 0.0, &opts);
        assert!(result.contains("\n"));
    }

    #[test]
    fn render_invert_truecolor_uses_rgb_black() {
        let opts = RenderOpts {
            truecolor: true,
            invert: true,
            ..Default::default()
        };
        let result = render_line("A", 0.0, &opts);
        assert!(result.contains("48;2;"));
        assert!(!result.contains("38;2;0;0;0"));
        assert!(result.ends_with("\x1b[49m"));
    }

    #[test]
    fn render_expands_tabs_to_eight_colored_spaces() {
        let result = render_line("\t", 0.0, &RenderOpts::default());
        assert_eq!(result.matches("\x1b[38;2;").count(), 8);
        assert!(!result.contains('\t'));
    }

    #[test]
    fn render_preserves_osc_sequences_as_one_token() {
        let input = "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\";
        let result = render_line(input, 0.0, &RenderOpts::default());
        assert!(result.starts_with("\x1b]8;;https://example.com\x1b\\"));
        assert!(result.ends_with("\x1b]8;;\x1b\\"));
        assert_eq!(result.matches("\x1b[38;2;").count(), 4);
    }

    #[test]
    fn render_preserves_line_endings_without_adding_one() {
        let opts = RenderOpts::default();
        assert!(!render_line("a", 0.0, &opts).ends_with('\n'));
        assert!(render_line("a\r\n", 0.0, &opts).ends_with("\r\n"));
    }

    #[test]
    fn animation_filter_only_removes_terminal_editing_controls() {
        let input = "\x1b[31mred\x1b[K!\x1b[0m";
        assert_eq!(
            filter_replayed_animation_controls(input),
            "\x1b[31mred!\x1b[0m"
        );
    }

    #[test]
    fn color_position_uses_grapheme_cell_width() {
        let opts = RenderOpts {
            spread: 1.0,
            ..RenderOpts::default()
        };
        for (input, expected_offset) in [("A界B", 3.0), ("e\u{301}B", 1.0), ("👩‍💻B", 2.0)]
        {
            let result = render_line(input, 0.0, &opts);
            let expected_b = render_line("B", expected_offset, &opts);
            assert!(result.ends_with(&expected_b), "{input:?}");
        }
        let combined = render_line("e\u{301}", 0.0, &opts);
        assert_eq!(combined.matches("\x1b[38;2;").count(), 1);
        assert!(combined.contains("e\u{301}"));
    }

    #[test]
    fn compact_mode_reuses_color_and_resets_before_input_control() {
        let opts = RenderOpts {
            freq: 0.0,
            ..RenderOpts::default()
        };
        let style = RenderStyle {
            compact: true,
            ..RenderStyle::default()
        };
        let plain = render_line_styled("abc", 0.0, &opts, &style);
        assert_eq!(plain.matches("\x1b[38;2;").count(), 1);
        assert!(plain.contains("mabc\x1b[39m"));
        let styled = render_line_styled("a\x1b[1mb", 0.0, &opts, &style);
        assert!(styled.contains("a\x1b[39m\x1b[1m\x1b[38;2;"));
        assert!(styled.ends_with("b\x1b[39m"));
    }

    #[test]
    fn explicit_background_reaches_text_contrast_target() {
        let black = Rgb { r: 0, g: 0, b: 0 };
        let white = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let on_dark = ensure_contrast(black, Background::Dark);
        let on_light = ensure_contrast(white, Background::Light);
        assert!((luminance(on_dark) + 0.05) / 0.05 >= 4.5);
        assert!(1.05 / (luminance(on_light) + 0.05) >= 4.5);
        assert_eq!(ensure_contrast(white, Background::Dark), white);
        assert_eq!(ensure_contrast(black, Background::Light), black);
    }
}
