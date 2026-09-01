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
}
