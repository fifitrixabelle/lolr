use crate::color::{rgb_to_256, Rgb};
use crate::gradient::{gradient_color, Gradient};
use anstyle_parse::{Params, Parser, Perform};

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

fn format_color(rgb: Rgb, truecolor: bool, invert: bool) -> String {
    if invert {
        if truecolor {
            format!("\x1b[48;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
        } else {
            format!("\x1b[48;5;{}m", rgb_to_256(rgb))
        }
    } else if truecolor {
        format!("\x1b[38;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
    } else {
        format!("\x1b[38;5;{}m", rgb_to_256(rgb))
    }
}

struct Colorize<'a> {
    output: Vec<u8>,
    pending: Vec<u8>,
    offset: f64,
    col: usize,
    opts: &'a RenderOpts,
}

impl Colorize<'_> {
    fn flush_control(&mut self) {
        self.output.append(&mut self.pending);
    }

    fn colorize_char(&mut self, ch: char) {
        let char_len = ch.len_utf8();
        let char_start = self.pending.len() - char_len;
        self.output.extend_from_slice(&self.pending[..char_start]);

        let i = self.offset + self.col as f64 / self.opts.spread;
        let rgb = gradient_color(self.opts.gradient, self.opts.freq, i);
        self.output
            .extend_from_slice(format_color(rgb, self.opts.truecolor, self.opts.invert).as_bytes());
        self.output.extend_from_slice(&self.pending[char_start..]);
        self.output.extend_from_slice(if self.opts.invert {
            b"\x1b[49m"
        } else {
            b"\x1b[39m"
        });
        self.pending.clear();
        self.col += 1;
    }
}

impl Perform for Colorize<'_> {
    fn print(&mut self, ch: char) {
        self.colorize_char(ch);
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
    let mut parser: Parser = Parser::default();
    let mut performer = Colorize {
        output: Vec::with_capacity(line.len()),
        pending: Vec::new(),
        offset,
        col: 0,
        opts,
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
}
