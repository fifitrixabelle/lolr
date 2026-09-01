use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn rainbow_color(freq: f64, i: f64) -> Rgb {
    let r = (freq * i + 0.0).sin() * 127.0 + 128.0;
    let g = (freq * i + 2.0 * PI / 3.0).sin() * 127.0 + 128.0;
    let b = (freq * i + 4.0 * PI / 3.0).sin() * 127.0 + 128.0;
    Rgb {
        r: r as u8,
        g: g as u8,
        b: b as u8,
    }
}

pub fn rgb_to_256(color: Rgb) -> u8 {
    let r = color.r as u16;
    let g = color.g as u16;
    let b = color.b as u16;

    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return (((r - 8) as f64 / 247.0 * 24.0) as u8) + 232;
    }

    let r_idx = (r as f64 / 255.0 * 5.0).round() as u8;
    let g_idx = (g as f64 / 255.0 * 5.0).round() as u8;
    let b_idx = (b as f64 / 255.0 * 5.0).round() as u8;

    16 + 36 * r_idx + 6 * g_idx + b_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rainbow_at_zero_offset() {
        let color = rainbow_color(0.1, 0.0);
        assert_eq!(color.r, 128);
        assert_eq!(color.g, 237);
        assert_eq!(color.b, 18);
    }

    #[test]
    fn rainbow_values_in_valid_range() {
        for i in 0..100 {
            let color = rainbow_color(0.1, i as f64);
            assert!(color.r <= 255);
            assert!(color.g <= 255);
            assert!(color.b <= 255);
        }
    }

    #[test]
    fn rgb_to_256_pure_red() {
        let color = Rgb { r: 255, g: 0, b: 0 };
        let code = rgb_to_256(color);
        assert_eq!(code, 196);
    }

    #[test]
    fn rgb_to_256_pure_white() {
        let color = Rgb { r: 255, g: 255, b: 255 };
        let code = rgb_to_256(color);
        assert_eq!(code, 231);
    }

    #[test]
    fn rgb_to_256_grayscale() {
        let color = Rgb { r: 128, g: 128, b: 128 };
        let code = rgb_to_256(color);
        assert!(code >= 232 && code <= 255);
    }
}
