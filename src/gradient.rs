use crate::color::{rainbow_color, Rgb};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Gradient {
    #[default]
    Rainbow,
    Fire,
    Ocean,
    Pastel,
    Neon,
    Sunset,
    Forest,
    Synthwave,
    Viridis,
    Aura,
}

impl Gradient {
    pub const ALL: [Gradient; 10] = [
        Gradient::Rainbow,
        Gradient::Fire,
        Gradient::Ocean,
        Gradient::Pastel,
        Gradient::Neon,
        Gradient::Sunset,
        Gradient::Forest,
        Gradient::Synthwave,
        Gradient::Viridis,
        Gradient::Aura,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Gradient::Rainbow => "rainbow",
            Gradient::Fire => "fire",
            Gradient::Ocean => "ocean",
            Gradient::Pastel => "pastel",
            Gradient::Neon => "neon",
            Gradient::Sunset => "sunset",
            Gradient::Forest => "forest",
            Gradient::Synthwave => "synthwave",
            Gradient::Viridis => "viridis",
            Gradient::Aura => "aura",
        }
    }

    pub fn from_name(name: &str) -> Option<Gradient> {
        match name.to_lowercase().as_str() {
            "rainbow" => Some(Gradient::Rainbow),
            "fire" => Some(Gradient::Fire),
            "ocean" => Some(Gradient::Ocean),
            "pastel" => Some(Gradient::Pastel),
            "neon" => Some(Gradient::Neon),
            "sunset" => Some(Gradient::Sunset),
            "forest" => Some(Gradient::Forest),
            "synthwave" => Some(Gradient::Synthwave),
            "viridis" => Some(Gradient::Viridis),
            "aura" => Some(Gradient::Aura),
            _ => None,
        }
    }
}

impl FromStr for Gradient {
    type Err = ParseGradientError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::from_name(name).ok_or_else(|| ParseGradientError(name.to_owned()))
    }
}

impl fmt::Display for Gradient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseGradientError(String);

impl fmt::Display for ParseGradientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown gradient {:?}; expected one of: ", self.0)?;
        for (index, gradient) in Gradient::ALL.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            gradient.fmt(f)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseGradientError {}

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
    colors.last().map(|(_, c)| *c).unwrap_or(Rgb {
        r: 255,
        g: 255,
        b: 255,
    })
}

pub fn gradient_color(gradient: Gradient, freq: f64, i: f64) -> Rgb {
    match gradient {
        Gradient::Rainbow => rainbow_color(freq, i),
        Gradient::Fire => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (0.0, Rgb { r: 128, g: 0, b: 0 }),
                (0.3, Rgb { r: 255, g: 0, b: 0 }),
                (
                    0.6,
                    Rgb {
                        r: 255,
                        g: 165,
                        b: 0,
                    },
                ),
                (
                    1.0,
                    Rgb {
                        r: 255,
                        g: 255,
                        b: 0,
                    },
                ),
            ];
            lerp_color(&stops, t)
        }
        Gradient::Ocean => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (0.0, Rgb { r: 0, g: 0, b: 128 }),
                (
                    0.4,
                    Rgb {
                        r: 0,
                        g: 128,
                        b: 255,
                    },
                ),
                (
                    0.7,
                    Rgb {
                        r: 0,
                        g: 255,
                        b: 255,
                    },
                ),
                (
                    1.0,
                    Rgb {
                        r: 255,
                        g: 255,
                        b: 255,
                    },
                ),
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
                r: base.r.saturating_add(30),
                g: base.g.saturating_add(30),
                b: base.b.saturating_add(30),
            }
        }
        Gradient::Sunset => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (
                    0.0,
                    Rgb {
                        r: 45,
                        g: 27,
                        b: 105,
                    },
                ),
                (
                    0.25,
                    Rgb {
                        r: 128,
                        g: 0,
                        b: 128,
                    },
                ),
                (
                    0.5,
                    Rgb {
                        r: 255,
                        g: 64,
                        b: 129,
                    },
                ),
                (
                    0.75,
                    Rgb {
                        r: 255,
                        g: 140,
                        b: 66,
                    },
                ),
                (
                    1.0,
                    Rgb {
                        r: 255,
                        g: 214,
                        b: 102,
                    },
                ),
            ];
            lerp_color(&stops, t)
        }
        Gradient::Forest => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (
                    0.0,
                    Rgb {
                        r: 10,
                        g: 54,
                        b: 34,
                    },
                ),
                (
                    0.33,
                    Rgb {
                        r: 20,
                        g: 120,
                        b: 70,
                    },
                ),
                (
                    0.66,
                    Rgb {
                        r: 77,
                        g: 184,
                        b: 72,
                    },
                ),
                (
                    1.0,
                    Rgb {
                        r: 218,
                        g: 236,
                        b: 126,
                    },
                ),
            ];
            lerp_color(&stops, t)
        }
        Gradient::Synthwave => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (
                    0.0,
                    Rgb {
                        r: 94,
                        g: 23,
                        b: 235,
                    },
                ),
                (
                    0.35,
                    Rgb {
                        r: 207,
                        g: 37,
                        b: 247,
                    },
                ),
                (
                    0.65,
                    Rgb {
                        r: 255,
                        g: 41,
                        b: 117,
                    },
                ),
                (
                    1.0,
                    Rgb {
                        r: 0,
                        g: 229,
                        b: 255,
                    },
                ),
            ];
            lerp_color(&stops, t)
        }
        Gradient::Viridis => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (0.0, Rgb { r: 68, g: 1, b: 84 }),
                (
                    0.25,
                    Rgb {
                        r: 59,
                        g: 82,
                        b: 139,
                    },
                ),
                (
                    0.5,
                    Rgb {
                        r: 33,
                        g: 145,
                        b: 140,
                    },
                ),
                (
                    0.75,
                    Rgb {
                        r: 94,
                        g: 201,
                        b: 98,
                    },
                ),
                (
                    1.0,
                    Rgb {
                        r: 253,
                        g: 231,
                        b: 37,
                    },
                ),
            ];
            lerp_color(&stops, t)
        }
        Gradient::Aura => {
            let t = ((freq * i).sin() + 1.0) / 2.0;
            let stops = [
                (
                    0.0,
                    Rgb {
                        r: 162,
                        g: 119,
                        b: 255,
                    },
                ),
                (
                    0.5,
                    Rgb {
                        r: 97,
                        g: 255,
                        b: 202,
                    },
                ),
                (
                    1.0,
                    Rgb {
                        r: 162,
                        g: 119,
                        b: 255,
                    },
                ),
            ];
            lerp_color(&stops, t)
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
    fn aura_gradient_stays_in_the_cool_palette() {
        for step in 0..=1_000 {
            let color = gradient_color(Gradient::Aura, 0.01, step as f64);
            assert!(color.b >= color.r, "unexpected warm Aura color: {color:?}");
        }

        assert_eq!(
            gradient_color(Gradient::Aura, 0.1, 0.0),
            Rgb {
                r: 97,
                g: 255,
                b: 202,
            }
        );
    }

    #[test]
    fn from_name_parses_correctly() {
        assert_eq!(Gradient::from_name("rainbow"), Some(Gradient::Rainbow));
        assert_eq!(Gradient::from_name("FIRE"), Some(Gradient::Fire));
        assert_eq!(Gradient::from_name("invalid"), None);
    }

    #[test]
    fn all_gradient_names_are_parseable() {
        for gradient in Gradient::ALL {
            assert_eq!(Gradient::from_name(gradient.as_str()), Some(gradient));
        }
    }
}
