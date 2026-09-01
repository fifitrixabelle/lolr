use crate::color::{rgb_to_256, Rgb};
use crate::gradient::{gradient_color, Gradient};

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
    let code = if truecolor {
        format!("38;2;{};{};{}", rgb.r, rgb.g, rgb.b)
    } else {
        format!("38;5;{}", rgb_to_256(rgb))
    };

    if invert {
        let bg_code = if truecolor {
            format!("48;2;{};{};{}", rgb.r, rgb.g, rgb.b)
        } else {
            format!("48;5;{}", rgb_to_256(rgb))
        };
        let fg_black = if truecolor { "38;2;0;0;0" } else { "38;5;0" };
        format!("\x1b[{};{}m", bg_code, fg_black)
    } else {
        format!("\x1b[{}m", code)
    }
}

pub fn render_line(line: &str, offset: f64, opts: &RenderOpts) -> String {
    let mut result = String::new();
    let mut col = 0;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\n' || ch == '\r' {
            result.push(ch);
            continue;
        }

        // Pass through ANSI escape sequences without colorizing
        if ch == '\x1b' {
            result.push(ch);
            // Continue reading until we hit the sequence terminator
            while let Some(&next_ch) = chars.peek() {
                result.push(next_ch);
                chars.next();
                // Common ANSI terminators
                if next_ch.is_ascii_alphabetic() || next_ch == 'm' {
                    break;
                }
            }
            continue;
        }

        let i = offset + col as f64 / opts.spread;
        let rgb = gradient_color(opts.gradient, opts.freq, i);
        let color_code = format_color(rgb, opts.truecolor, opts.invert);

        result.push_str(&color_code);
        result.push(ch);
        col += 1;
    }

    result.push_str("\x1b[0m");
    result
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
        assert!(result.contains("38;2;0;0;0"));
    }
}
