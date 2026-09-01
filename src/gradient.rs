use crate::color::{Rgb, rainbow_color};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gradient {
    #[default]
    Rainbow,
    Fire,
    Ocean,
    Pastel,
    Neon,
}

impl Gradient {
    pub fn from_name(name: &str) -> Option<Gradient> {
        match name.to_lowercase().as_str() {
            "rainbow" => Some(Gradient::Rainbow),
            "fire" => Some(Gradient::Fire),
            "ocean" => Some(Gradient::Ocean),
            "pastel" => Some(Gradient::Pastel),
            "neon" => Some(Gradient::Neon),
            _ => None,
        }
    }
}

fn lerp_color(colors: &[(f64, Rgb)], t: f64) -> Rgb {
    let t = t.rem_euclid(1.0);
    for window in colors.windows(2) {
        let (t0, c0) = window[0];
        let (t1, c1) = window[1];
        if t >= t0 && t < t1 {
            let local_t = (t - t0) / (t1 - t0);
            return Rgb {
                r: (c0.r as f64 + (c1.r as f64 - c0.r as f64) * local_t) as u8,
                g: (c0.g as f64 + (c1.g as f64 - c0.g as f64) * local_t) as u8,
                b: (c0.b as f64 + (c1.b as f64 - c0.b as f64) * local_t) as u8,
            };
        }
    }
    colors.last().map(|(_, c)| *c).unwrap_or(Rgb { r: 255, g: 255, b: 255 })
}

pub fn gradient_color(gradient: Gradient, freq: f64, i: f64) -> Rgb {
    match gradient {
        Gradient::Rainbow => rainbow_color(freq, i),
        Gradient::Fire => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (0.0, Rgb { r: 128, g: 0, b: 0 }),
                (0.3, Rgb { r: 255, g: 0, b: 0 }),
                (0.6, Rgb { r: 255, g: 165, b: 0 }),
                (1.0, Rgb { r: 255, g: 255, b: 0 }),
            ];
            lerp_color(&stops, t)
        }
        Gradient::Ocean => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (0.0, Rgb { r: 0, g: 0, b: 128 }),
                (0.4, Rgb { r: 0, g: 128, b: 255 }),
                (0.7, Rgb { r: 0, g: 255, b: 255 }),
                (1.0, Rgb { r: 255, g: 255, b: 255 }),
            ];
            lerp_color(&stops, t)
        }
        Gradient::Pastel => {
            let base = rainbow_color(freq, i);
            Rgb {
                r: ((base.r as u16 + 255) / 2) as u8,
                g: ((base.g as u16 + 255) / 2) as u8,
                b: ((base.b as u16 + 255) / 2) as u8,
            }
        }
        Gradient::Neon => {
            let base = rainbow_color(freq * 1.5, i);
            Rgb {
                r: base.r.saturating_add(30).min(255),
                g: base.g.saturating_add(30).min(255),
                b: base.b.saturating_add(30).min(255),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rainbow_gradient_matches_rainbow_color() {
        let gradient_rgb = gradient_color(Gradient::Rainbow, 0.1, 5.0);
        let direct_rgb = rainbow_color(0.1, 5.0);
        assert_eq!(gradient_rgb, direct_rgb);
    }

    #[test]
    fn fire_gradient_is_warm() {
        let color = gradient_color(Gradient::Fire, 0.1, 0.0);
        assert!(color.r >= color.b);
    }

    #[test]
    fn ocean_gradient_is_cool() {
        let color = gradient_color(Gradient::Ocean, 0.1, 0.0);
        assert!(color.b >= color.r);
    }

    #[test]
    fn from_name_parses_correctly() {
        assert_eq!(Gradient::from_name("rainbow"), Some(Gradient::Rainbow));
        assert_eq!(Gradient::from_name("FIRE"), Some(Gradient::Fire));
        assert_eq!(Gradient::from_name("invalid"), None);
    }
}
