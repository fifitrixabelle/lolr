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
    // Match the Paint gem used by Ruby lolcat: use the grayscale ramp when
    // all three components fall in the same 42.5-wide band, otherwise use
    // the xterm 6x6x6 color cube.
    let components = [color.r as u16, color.g as u16, color.b as u16];
    let [r_band, g_band, b_band] = components.map(|component| component * 2 / 85);
    if r_band == g_band && g_band == b_band {
        // The sum is integral and 33 is odd, so adding 16 rounds to nearest.
        return 232 + ((components.iter().sum::<u16>() + 16) / 33) as u8;
    }

    let [r, g, b] = components.map(|component| (6 * component / 256) as u8);

    16 + 36 * r + 6 * g + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_conversion_matches_float_reference_at_every_channel_value() {
        fn reference(color: Rgb) -> u8 {
            let components = [color.r as f64, color.g as f64, color.b as f64];
            let mut separator = 42.5;
            loop {
                if components.iter().any(|component| *component < separator) {
                    if components.iter().all(|component| *component < separator) {
                        return 232 + (components.iter().sum::<f64>() / 33.0).round() as u8;
                    }
                    break;
                }
                separator += 42.5;
            }
            let [r, g, b] = components.map(|component| (6.0 * component / 256.0) as u8);
            16 + 36 * r + 6 * g + b
        }

        // Straddle every grayscale band and cube boundary, including 255's band.
        let boundaries = [
            0, 42, 43, 84, 85, 86, 127, 128, 169, 170, 171, 212, 213, 214, 254, 255,
        ];
        for value in 0..=255 {
            for a in boundaries {
                for b in boundaries {
                    for [r, g, b] in [[value, a, b], [a, value, b], [a, b, value]] {
                        let rgb = Rgb { r, g, b };
                        assert_eq!(rgb_to_256(rgb), reference(rgb), "{rgb:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn rainbow_at_zero_offset() {
        let color = rainbow_color(0.1, 0.0);
        assert_eq!(color.r, 128);
        assert_eq!(color.g, 237);
        assert_eq!(color.b, 18);
    }

    #[test]
    fn rainbow_values_in_valid_range() {
        // Test that rainbow_color produces valid RGB values
        // (This test primarily ensures the function completes without panic)
        for i in 0..100 {
            let _color = rainbow_color(0.1, i as f64);
            // u8 values are always in 0..=255, so just verify it computes
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
        let color = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let code = rgb_to_256(color);
        assert_eq!(code, 255);
    }

    #[test]
    fn rgb_to_256_grayscale() {
        let color = Rgb {
            r: 128,
            g: 128,
            b: 128,
        };
        let code = rgb_to_256(color);
        assert_eq!(code, 244);
    }

    #[test]
    fn rgb_to_256_matches_paint_color_cube() {
        let color = Rgb {
            r: 153,
            g: 223,
            b: 76,
        };
        assert_eq!(rgb_to_256(color), 155);
    }
}
