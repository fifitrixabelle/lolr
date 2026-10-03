use crate::color::{rgb_to_256, Rgb};
use crate::gradient::{gradient_color, Gradient, Palette};
use anstyle_parse::{Params, Parser, Perform};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
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

fn linear_channel(channel: u8) -> f64 {
    let value = channel as f64 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(rgb: Rgb) -> f64 {
    // RGB channels have only 256 possible values. Keep the original calculation
    // and precision, but perform its exponentiation just once per channel value.
    static LINEAR_CHANNELS: OnceLock<[f64; 256]> = OnceLock::new();
    let channels = LINEAR_CHANNELS.get_or_init(|| std::array::from_fn(|i| linear_channel(i as u8)));
    0.2126 * channels[rgb.r as usize]
        + 0.7152 * channels[rgb.g as usize]
        + 0.0722 * channels[rgb.b as usize]
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

fn write_decimal(output: &mut Vec<u8>, value: u8) {
    let mut digits = [b'0'; 3];
    digits[2] += value % 10;
    let start = if value >= 100 {
        digits[0] += value / 100;
        digits[1] += value / 10 % 10;
        0
    } else if value >= 10 {
        digits[1] += value / 10;
        1
    } else {
        2
    };
    output.extend_from_slice(&digits[start..]);
}

fn write_256_color(output: &mut Vec<u8>, index: u8, invert: bool) {
    output.extend_from_slice(if invert { b"\x1b[48;5;" } else { b"\x1b[38;5;" });
    write_decimal(output, index);
    output.push(b'm');
}

fn write_color(output: &mut Vec<u8>, rgb: Rgb, truecolor: bool, invert: bool) {
    if truecolor {
        output.extend_from_slice(if invert { b"\x1b[48;2;" } else { b"\x1b[38;2;" });
        write_decimal(output, rgb.r);
        output.push(b';');
        write_decimal(output, rgb.g);
        output.push(b';');
        write_decimal(output, rgb.b);
        output.push(b'm');
    } else {
        write_256_color(output, rgb_to_256(rgb), invert);
    }
}

struct Colorize<'a> {
    output: &'a mut Vec<u8>,
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

impl<'a> Colorize<'a> {
    fn new(
        line: &str,
        offset: f64,
        opts: &'a RenderOpts,
        style: &'a RenderStyle,
        output: &'a mut Vec<u8>,
    ) -> Self {
        output.clear();
        output.reserve(line.len());
        Self {
            output,
            pending: Vec::new(),
            text_run: Vec::new(),
            offset,
            col: 0,
            opts,
            style,
            last_color: None,
            fast_ascii: line.is_ascii(),
        }
    }

    fn render_parsed(&mut self, line: &str) {
        let mut parser: Parser = Parser::default();
        for &byte in line.as_bytes() {
            // Ruby lolcat expands every tab to eight spaces before parsing ANSI.
            let bytes: &[u8] = if byte == b'\t' { b"        " } else { &[byte] };
            for &expanded in bytes {
                self.pending.push(expanded);
                parser.advance(self, expanded);
            }
        }
        self.flush_control();
    }

    fn render_plain_ascii(&mut self, line: &str) {
        for &byte in line.as_bytes() {
            match byte {
                b'\t' => {
                    for _ in 0..8 {
                        self.emit_unit(b" ", 1);
                    }
                }
                b'\r' | b'\n' => {
                    self.reset_compact();
                    self.output.push(byte);
                }
                _ => self.emit_unit(&[byte], 1),
            }
        }
        self.reset_compact();
    }

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
            emit_graphemes(text, GraphemeSink::Colorize(self));
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
            write_color(self.output, rgb, self.opts.truecolor, self.opts.invert);
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
            match key {
                OutputColor::Truecolor(rgb) => {
                    write_color(self.output, rgb, true, self.opts.invert)
                }
                OutputColor::Ansi256(index) => {
                    write_256_color(self.output, index, self.opts.invert)
                }
            }
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
    let mut output = Vec::new();
    render_line_styled_into(line, offset, opts, style, &mut output);
    String::from_utf8(output).expect("rendering valid UTF-8 must produce valid UTF-8")
}

/// Render into a reusable UTF-8 byte buffer, replacing its previous contents.
/// Retains the buffer's capacity across lines and animation frames.
pub fn render_line_styled_into(
    line: &str,
    offset: f64,
    opts: &RenderOpts,
    style: &RenderStyle,
    output: &mut Vec<u8>,
) {
    let mut performer = Colorize::new(line, offset, opts, style, output);
    if is_plain_ascii(line) {
        performer.render_plain_ascii(line);
    } else {
        performer.render_parsed(line);
    }
}

/// Reuses ANSI and Unicode scratch buffers when rendering successive lines.
/// Each call starts with fresh terminal parsing and color state.
#[derive(Debug, Default)]
pub struct Renderer {
    pending: Vec<u8>,
    text_run: Vec<u8>,
}

impl Renderer {
    /// Replace `output` with colored UTF-8 bytes, retaining buffer capacities.
    pub fn render_into(
        &mut self,
        line: &str,
        offset: f64,
        opts: &RenderOpts,
        style: &RenderStyle,
        output: &mut Vec<u8>,
    ) {
        let mut performer = Colorize::new(line, offset, opts, style, output);
        if is_plain_ascii(line) {
            performer.render_plain_ascii(line);
            return;
        }
        performer.pending = std::mem::take(&mut self.pending);
        performer.text_run = std::mem::take(&mut self.text_run);
        performer.render_parsed(line);
        self.pending = performer.pending;
        self.text_run = performer.text_run;
    }
}

fn is_plain_ascii(line: &str) -> bool {
    line.bytes()
        .all(|byte| matches!(byte, b' '..=b'~' | b'\t' | b'\r' | b'\n'))
}

enum GraphemeSink<'sink, 'render, 'line> {
    Colorize(&'sink mut Colorize<'render>),
    Prepare(&'sink mut PreparedLine<'line>),
}

// A single grapheme loop keeps segmentation and width calculation optimized
// together, rather than adding iterator calls to each rendered grapheme.
fn emit_graphemes(text: &str, mut sink: GraphemeSink<'_, '_, '_>) {
    for grapheme in UnicodeSegmentation::graphemes(text, true) {
        let bytes = grapheme.as_bytes();
        let width = UnicodeWidthStr::width(grapheme);
        match &mut sink {
            GraphemeSink::Colorize(colorize) => colorize.emit_unit(bytes, width),
            GraphemeSink::Prepare(prepared) => prepared.record(bytes, Some(width)),
        }
    }
}

// Keep preparation separate from the concrete streaming hot path. The parity
// tests below cover control boundaries, grapheme widths and rendering options.
struct PrepareParser<'a, 'line> {
    prepared: &'a mut PreparedLine<'line>,
    pending: Vec<u8>,
    text_run: Vec<u8>,
    fast_ascii: bool,
}

impl PrepareParser<'_, '_> {
    fn render_parsed(&mut self, line: &str) {
        let mut parser: Parser = Parser::default();
        for &byte in line.as_bytes() {
            // Ruby lolcat expands every tab to eight spaces before parsing ANSI.
            let bytes: &[u8] = if byte == b'\t' { b"        " } else { &[byte] };
            for &expanded in bytes {
                self.pending.push(expanded);
                parser.advance(self, expanded);
            }
        }
        self.flush_control();
    }

    fn flush_control(&mut self) {
        self.flush_text();
        self.reset_compact();
        self.prepared.record(&self.pending, None);
        self.pending.clear();
    }

    fn reset_compact(&mut self) {
        self.prepared.record(&[], None);
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
            emit_graphemes(text, GraphemeSink::Prepare(self.prepared));
        }
        run.clear();
        self.text_run = run;
    }

    fn emit_unit(&mut self, bytes: &[u8], width: usize) {
        self.prepared.record(bytes, Some(width));
    }
}

impl Perform for PrepareParser<'_, '_> {
    fn print(&mut self, ch: char) {
        let char_start = self.pending.len() - ch.len_utf8();
        if self.fast_ascii {
            if char_start > 0 {
                self.reset_compact();
                self.prepared.record(&self.pending[..char_start], None);
            }
            let byte = self.pending[char_start];
            self.emit_unit(&[byte], 1);
            self.pending.clear();
            return;
        }
        if char_start > 0 {
            self.flush_text();
            self.reset_compact();
            self.prepared.record(&self.pending[..char_start], None);
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

/// Text prepared once for repeated rendering at different color offsets.
/// Preserves ANSI controls and grapheme widths; it buffers only this line.
#[derive(Debug)]
pub struct PreparedLine<'a> {
    plain_ascii: Option<&'a str>,
    bytes: Vec<u8>,
    tokens: Vec<PreparedToken>,
}

#[derive(Debug)]
struct PreparedToken {
    end: usize,
    width: Option<usize>,
}

impl<'a> PreparedLine<'a> {
    fn record(&mut self, bytes: &[u8], width: Option<usize>) {
        self.bytes.extend_from_slice(bytes);
        self.tokens.push(PreparedToken {
            end: self.bytes.len(),
            width,
        });
    }

    pub fn new(line: &'a str) -> Self {
        let mut prepared = Self {
            plain_ascii: None,
            bytes: Vec::new(),
            tokens: Vec::new(),
        };
        if is_plain_ascii(line) {
            prepared.plain_ascii = Some(line);
        } else {
            let mut parser = PrepareParser {
                prepared: &mut prepared,
                pending: Vec::new(),
                text_run: Vec::new(),
                fast_ascii: line.is_ascii(),
            };
            parser.render_parsed(line);
        }
        prepared
    }

    /// Replace `output` with this line rendered at the requested color offset.
    pub fn render_into(
        &self,
        offset: f64,
        opts: &RenderOpts,
        style: &RenderStyle,
        output: &mut Vec<u8>,
    ) {
        let line = self.plain_ascii.unwrap_or("");
        let mut colorize = Colorize::new(line, offset, opts, style, output);
        if let Some(line) = self.plain_ascii {
            colorize.render_plain_ascii(line);
        } else {
            let mut start = 0;
            for token in &self.tokens {
                let bytes = &self.bytes[start..token.end];
                if let Some(width) = token.width {
                    colorize.emit_unit(bytes, width);
                } else {
                    colorize.reset_compact();
                    colorize.output.extend_from_slice(bytes);
                }
                start = token.end;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_luminance(rgb: Rgb) -> f64 {
        0.2126 * linear_channel(rgb.r)
            + 0.7152 * linear_channel(rgb.g)
            + 0.0722 * linear_channel(rgb.b)
    }

    fn reference_contrast(rgb: Rgb, background: Background) -> Rgb {
        let target = match background {
            Background::Dark => 0.175,
            Background::Light => 1.05 / 4.5 - 0.05,
        };
        let current = reference_luminance(rgb);
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
            let blend =
                |channel: u8| (channel as f64 + (end - channel as f64) * middle).round() as u8;
            let candidate = Rgb {
                r: blend(rgb.r),
                g: blend(rgb.g),
                b: blend(rgb.b),
            };
            let enough = match background {
                Background::Dark => reference_luminance(candidate) >= target,
                Background::Light => reference_luminance(candidate) <= target,
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

    #[test]
    fn lookup_contrast_matches_original_math() {
        for channel in 0..=255u8 {
            for rgb in [
                Rgb {
                    r: channel,
                    g: 0,
                    b: 0,
                },
                Rgb {
                    r: 0,
                    g: channel,
                    b: 0,
                },
                Rgb {
                    r: 0,
                    g: 0,
                    b: channel,
                },
            ] {
                assert_eq!(luminance(rgb).to_bits(), reference_luminance(rgb).to_bits());
            }
        }
        // Include grayscale and combinations spanning the full RGB cube.
        for r in 0..=255u8 {
            for g in (0..=255u8).step_by(17) {
                for b in (0..=255u8).step_by(17) {
                    let rgb = Rgb { r, g, b };
                    assert_eq!(luminance(rgb).to_bits(), reference_luminance(rgb).to_bits());
                    for background in [Background::Dark, Background::Light] {
                        assert_eq!(
                            ensure_contrast(rgb, background),
                            reference_contrast(rgb, background)
                        );
                    }
                }
            }
            let gray = Rgb { r, g: r, b: r };
            for background in [Background::Dark, Background::Light] {
                assert_eq!(
                    ensure_contrast(gray, background),
                    reference_contrast(gray, background)
                );
            }
        }
    }

    #[test]
    fn ascii_fast_path_matches_parser_output() {
        let printable: String = (b' '..=b'~').map(char::from).collect();
        let all_ascii: String = (0..=127u8).map(char::from).collect();
        let palette = Palette::new(vec![
            Rgb { r: 0, g: 9, b: 99 },
            Rgb {
                r: 100,
                g: 255,
                b: 42,
            },
        ])
        .unwrap();
        let mut reused = Vec::new();
        let mut renderer = Renderer::default();
        for gradient in Gradient::ALL {
            for truecolor in [false, true] {
                for invert in [false, true] {
                    for compact in [false, true] {
                        for background in [None, Some(Background::Dark), Some(Background::Light)] {
                            for palette in [None, Some(palette.clone())] {
                                let opts = RenderOpts {
                                    gradient,
                                    truecolor,
                                    invert,
                                    ..RenderOpts::default()
                                };
                                let style = RenderStyle {
                                    compact,
                                    background,
                                    palette,
                                };
                                for input in [
                                    "",
                                    printable.as_str(),
                                    all_ascii.as_str(),
                                    "\tA\t\r\nB\n\t",
                                    "\r\n",
                                    "abc",
                                    "a\x1b[1mb\x1b[0m",
                                    "a\x1b[",
                                    "café 界 👩‍💻 e\u{301}\n",
                                ] {
                                    let mut output = Vec::new();
                                    let mut reference =
                                        Colorize::new(input, 42.5, &opts, &style, &mut output);
                                    reference.render_parsed(input);
                                    render_line_styled_into(
                                        input,
                                        42.5,
                                        &opts,
                                        &style,
                                        &mut reused,
                                    );
                                    assert_eq!(reused, reference.output.as_slice());
                                    renderer.render_into(input, 42.5, &opts, &style, &mut reused);
                                    assert_eq!(reused, reference.output.as_slice());
                                    PreparedLine::new(input).render_into(
                                        42.5,
                                        &opts,
                                        &style,
                                        &mut reused,
                                    );
                                    assert_eq!(reused, reference.output.as_slice());
                                    assert_eq!(
                                        render_line_styled(input, 42.5, &opts, &style).as_bytes(),
                                        reference.output.as_slice(),
                                        "{input:?}, {opts:?}, {style:?}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn reusable_output_retains_storage_and_clears_previous_contents() {
        let opts = RenderOpts::default();
        let style = RenderStyle::default();
        let mut output = Vec::with_capacity(4096);
        let capacity = output.capacity();
        let storage = output.as_ptr();
        for line in [
            "longer ASCII line",
            "café 界 👩‍💻",
            "a\x1b[1mb\x1b[0m",
            "\t",
            "",
        ] {
            render_line_styled_into(line, 0.0, &opts, &style, &mut output);
            assert_eq!(output.capacity(), capacity);
            assert_eq!(output.as_ptr(), storage);
        }
        assert!(output.is_empty());
    }

    #[test]
    fn scratch_buffers_are_reused_without_carrying_escape_state_between_lines() {
        let opts = RenderOpts::default();
        let style = RenderStyle {
            compact: true,
            ..RenderStyle::default()
        };
        let mut renderer = Renderer::default();
        let mut output = Vec::new();
        let input = "\x1b]0;a long terminal title\x07café 界 👩‍💻 e\u{301}";
        renderer.render_into(input, 0.0, &opts, &style, &mut output);
        let pending = renderer.pending.as_ptr();
        let text = renderer.text_run.as_ptr();
        for line in [input, "unfinished\x1b]", "next line", "", "e\u{301}"] {
            renderer.render_into(line, 7.0, &opts, &style, &mut output);
            assert_eq!(
                output,
                render_line_styled(line, 7.0, &opts, &style).as_bytes()
            );
            assert_eq!(renderer.pending.as_ptr(), pending);
            assert_eq!(renderer.text_run.as_ptr(), text);
        }
    }

    #[test]
    fn prepared_line_recomputes_colors_at_each_offset() {
        let opts = RenderOpts::default();
        let style = RenderStyle {
            compact: true,
            ..RenderStyle::default()
        };
        let input = "\x1b[1me\u{301} 界 👩‍💻\x1b[0m\t!\n";
        let prepared = PreparedLine::new(input);
        let mut output = Vec::new();
        for offset in [0.0, 42.5, -7.0, 1e6] {
            prepared.render_into(offset, &opts, &style, &mut output);
            assert_eq!(
                output,
                render_line_styled(input, offset, &opts, &style).as_bytes()
            );
        }
    }

    #[test]
    fn color_encoding_matches_formatted_ansi_for_every_channel_value() {
        let mut output = Vec::new();
        for value in 0..=u8::MAX {
            // Exercise every value in each channel, including decimal boundaries.
            let rgb = Rgb {
                r: value,
                g: value.wrapping_add(85),
                b: value.wrapping_add(170),
            };
            for truecolor in [false, true] {
                for invert in [false, true] {
                    output.clear();
                    write_color(&mut output, rgb, truecolor, invert);
                    let target = if invert { 48 } else { 38 };
                    let expected = if truecolor {
                        format!("\x1b[{target};2;{};{};{}m", rgb.r, rgb.g, rgb.b)
                    } else {
                        format!("\x1b[{target};5;{}m", rgb_to_256(rgb))
                    };
                    assert_eq!(output, expected.as_bytes());
                }
            }
        }
    }

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
