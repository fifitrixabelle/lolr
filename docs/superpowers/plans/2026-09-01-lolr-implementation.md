# lolr Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI and library that replicates Ruby lolcat with animation support.

**Architecture:** Core modules (color → gradient → render → animate) build on each other. CLI wraps the library. Each module has clear boundaries: color handles RGB math, gradient handles presets and interpolation, render applies colors to text, animate handles the animation loop.

**Tech Stack:** Rust, clap (derive), crossterm, rand

**Spec:** `docs/superpowers/specs/2026-09-01-lolr-design.md`

## Global Constraints

- Rust edition 2021
- Dependencies: clap 4.x (derive feature), crossterm 0.27+, rand 0.8+
- Match Ruby lolcat defaults: spread=3.0, freq=0.1, duration=12, speed=20fps
- Support 256-color fallback when COLORTERM≠truecolor
- Cross-platform via crossterm (no platform-specific code)

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`

**Interfaces:**
- Consumes: nothing (first task)
- Produces: compilable empty project with dependencies declared

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "lolr"
version = "0.1.0"
edition = "2021"
description = "Rainbow colorizer for text with animation support"
license = "MIT"

[dependencies]
clap = { version = "4", features = ["derive"] }
crossterm = "0.27"
rand = "0.8"

[lib]
name = "lolr"
path = "src/lib.rs"

[[bin]]
name = "lolr"
path = "src/main.rs"
```

- [ ] **Step 2: Create empty lib.rs**

```rust
pub fn placeholder() {}
```

- [ ] **Step 3: Create empty main.rs**

```rust
fn main() {
    println!("lolr");
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: scaffold lolr project with dependencies"
```

---

### Task 2: Color Module - Rainbow Math

**Files:**
- Create: `src/color.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }`, `pub fn rainbow_color(freq: f64, i: f64) -> Rgb`

- [ ] **Step 1: Write failing test for rainbow_color**

Create `src/color.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn rainbow_color(freq: f64, i: f64) -> Rgb {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rainbow_at_zero_offset() {
        let color = rainbow_color(0.1, 0.0);
        assert_eq!(color.r, 128);
        assert_eq!(color.g, 238);
        assert_eq!(color.b, 17);
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
```

- [ ] **Step 2: Export color module from lib.rs**

Replace `src/lib.rs`:

```rust
pub mod color;

pub use color::{Rgb, rainbow_color};
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test color`
Expected: FAIL with "not yet implemented"

- [ ] **Step 4: Implement rainbow_color**

Replace the `todo!()` in `src/color.rs`:

```rust
use std::f64::consts::PI;

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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test color`
Expected: PASS (2 tests)

- [ ] **Step 6: Commit**

```bash
git add src/color.rs src/lib.rs
git commit -m "feat: add rainbow color math"
```

---

### Task 3: Color Module - 256-Color Fallback

**Files:**
- Modify: `src/color.rs`

**Interfaces:**
- Consumes: `Rgb`
- Produces: `pub fn rgb_to_256(color: Rgb) -> u8`

- [ ] **Step 1: Write failing test for rgb_to_256**

Add to `src/color.rs` tests module:

```rust
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
```

- [ ] **Step 2: Add function stub**

Add to `src/color.rs` after `rainbow_color`:

```rust
pub fn rgb_to_256(color: Rgb) -> u8 {
    todo!()
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test rgb_to_256`
Expected: FAIL with "not yet implemented"

- [ ] **Step 4: Implement rgb_to_256**

Replace the function:

```rust
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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test rgb_to_256`
Expected: PASS (3 tests)

- [ ] **Step 6: Export from lib.rs**

Update `src/lib.rs`:

```rust
pub mod color;

pub use color::{Rgb, rainbow_color, rgb_to_256};
```

- [ ] **Step 7: Commit**

```bash
git add src/color.rs src/lib.rs
git commit -m "feat: add 256-color fallback conversion"
```

---

### Task 4: Gradient Module - Preset Gradients

**Files:**
- Create: `src/gradient.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `Rgb`
- Produces: `pub enum Gradient { Rainbow, Fire, Ocean, Pastel, Neon }`, `pub fn gradient_color(gradient: Gradient, freq: f64, i: f64) -> Rgb`

- [ ] **Step 1: Create gradient module with enum and test**

Create `src/gradient.rs`:

```rust
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

pub fn gradient_color(gradient: Gradient, freq: f64, i: f64) -> Rgb {
    todo!()
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
```

- [ ] **Step 2: Export gradient module from lib.rs**

Update `src/lib.rs`:

```rust
pub mod color;
pub mod gradient;

pub use color::{Rgb, rainbow_color, rgb_to_256};
pub use gradient::{Gradient, gradient_color};
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test gradient`
Expected: FAIL with "not yet implemented"

- [ ] **Step 4: Implement gradient_color**

Replace the `todo!()` in `src/gradient.rs`:

```rust
use std::f64::consts::PI;

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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test gradient`
Expected: PASS (4 tests)

- [ ] **Step 6: Commit**

```bash
git add src/gradient.rs src/lib.rs
git commit -m "feat: add gradient presets with interpolation"
```

---

### Task 5: Render Module - Text Colorization

**Files:**
- Create: `src/render.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `Rgb`, `Gradient`, `gradient_color`, `rgb_to_256`
- Produces: `pub struct RenderOpts`, `pub fn render_line(line: &str, offset: f64, opts: &RenderOpts) -> String`

- [ ] **Step 1: Create render module with struct and test**

Create `src/render.rs`:

```rust
use crate::color::{Rgb, rgb_to_256};
use crate::gradient::{Gradient, gradient_color};

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

pub fn render_line(line: &str, offset: f64, opts: &RenderOpts) -> String {
    todo!()
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
        let opts = RenderOpts { truecolor: true, ..Default::default() };
        let result = render_line("A", 0.0, &opts);
        assert!(result.contains("\x1b[38;2;"));
    }

    #[test]
    fn render_256_uses_256_code() {
        let opts = RenderOpts { truecolor: false, ..Default::default() };
        let result = render_line("A", 0.0, &opts);
        assert!(result.contains("\x1b[38;5;"));
    }

    #[test]
    fn render_preserves_newlines() {
        let opts = RenderOpts::default();
        let result = render_line("a\nb", 0.0, &opts);
        assert!(result.contains("\n"));
    }
}
```

- [ ] **Step 2: Export render module from lib.rs**

Update `src/lib.rs`:

```rust
pub mod color;
pub mod gradient;
pub mod render;

pub use color::{Rgb, rainbow_color, rgb_to_256};
pub use gradient::{Gradient, gradient_color};
pub use render::{RenderOpts, render_line};
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test render`
Expected: FAIL with "not yet implemented"

- [ ] **Step 4: Implement render_line**

Replace the `todo!()` in `src/render.rs`:

```rust
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
        format!("\x1b[{};{}m", bg_code, "38;5;0")
    } else {
        format!("\x1b[{}m", code)
    }
}

pub fn render_line(line: &str, offset: f64, opts: &RenderOpts) -> String {
    let mut result = String::new();
    let mut col = 0;

    for ch in line.chars() {
        if ch == '\n' || ch == '\r' {
            result.push(ch);
            continue;
        }

        if ch == '\x1b' {
            result.push(ch);
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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test render`
Expected: PASS (4 tests)

- [ ] **Step 6: Commit**

```bash
git add src/render.rs src/lib.rs
git commit -m "feat: add text colorization with ANSI output"
```

---

### Task 6: Animate Module - Animation Loop

**Files:**
- Create: `src/animate.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `RenderOpts`, `render_line`
- Produces: `pub struct AnimateOpts`, `pub fn animate(text: &str, opts: &AnimateOpts) -> std::io::Result<()>`

- [ ] **Step 1: Create animate module with struct**

Create `src/animate.rs`:

```rust
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor::{RestorePosition, SavePosition},
    execute,
    terminal::is_raw_mode_enabled,
};

use crate::gradient::Gradient;
use crate::render::{RenderOpts, render_line};

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
            duration: 12,
            speed: 20.0,
            truecolor: true,
            invert: false,
        }
    }
}

pub fn animate(text: &str, opts: &AnimateOpts) -> io::Result<()> {
    let mut stdout = io::stdout();
    let frame_duration = Duration::from_secs_f64(1.0 / opts.speed);

    let render_opts = RenderOpts {
        gradient: opts.gradient,
        spread: opts.spread,
        freq: opts.freq,
        truecolor: opts.truecolor,
        invert: opts.invert,
    };

    execute!(stdout, SavePosition)?;

    for frame in 0..opts.duration {
        execute!(stdout, RestorePosition)?;

        let offset = opts.seed + frame as f64 * opts.spread;

        for line in text.lines() {
            let colored = render_line(line, offset, &render_opts);
            writeln!(stdout, "{}", colored)?;
        }

        stdout.flush()?;
        thread::sleep(frame_duration);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animate_opts_has_sane_defaults() {
        let opts = AnimateOpts::default();
        assert_eq!(opts.duration, 12);
        assert_eq!(opts.speed, 20.0);
        assert_eq!(opts.spread, 3.0);
    }
}
```

- [ ] **Step 2: Export animate module from lib.rs**

Update `src/lib.rs`:

```rust
pub mod color;
pub mod gradient;
pub mod render;
pub mod animate;

pub use color::{Rgb, rainbow_color, rgb_to_256};
pub use gradient::{Gradient, gradient_color};
pub use render::{RenderOpts, render_line};
pub use animate::{AnimateOpts, animate};
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test animate`
Expected: PASS (1 test)

- [ ] **Step 4: Commit**

```bash
git add src/animate.rs src/lib.rs
git commit -m "feat: add animation loop with cursor control"
```

---

### Task 7: CLI - Argument Parsing

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `Gradient`, `RenderOpts`, `AnimateOpts`, `render_line`, `animate`
- Produces: Working CLI binary

- [ ] **Step 1: Replace main.rs with CLI structure**

Replace `src/main.rs`:

```rust
use std::fs;
use std::io::{self, BufRead, IsTerminal, Read};

use clap::Parser;
use rand::Rng;

use lolr::{Gradient, RenderOpts, AnimateOpts, render_line, animate};

#[derive(Parser, Debug)]
#[command(name = "lolr")]
#[command(about = "Rainbow colorizer for text")]
#[command(version)]
struct Args {
    /// Files to read (default: stdin)
    #[arg()]
    files: Vec<String>,

    /// Rainbow spread
    #[arg(short = 'p', long, default_value = "3.0")]
    spread: f64,

    /// Rainbow frequency
    #[arg(short = 'F', long, default_value = "0.1")]
    freq: f64,

    /// Color seed (0 = random)
    #[arg(short = 'S', long, default_value = "0")]
    seed: u64,

    /// Enable animation
    #[arg(short, long)]
    animate: bool,

    /// Animation frames
    #[arg(short, long, default_value = "12")]
    duration: u32,

    /// Animation FPS
    #[arg(short, long, default_value = "20")]
    speed: f64,

    /// Swap foreground/background
    #[arg(short, long)]
    invert: bool,

    /// Force 24-bit color
    #[arg(short, long)]
    truecolor: bool,

    /// Force color on non-TTY
    #[arg(short, long)]
    force: bool,

    /// Gradient preset
    #[arg(short, long, default_value = "rainbow")]
    gradient: String,
}

fn detect_truecolor() -> bool {
    std::env::var("COLORTERM")
        .map(|v| v == "truecolor" || v == "24bit")
        .unwrap_or(false)
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let gradient = Gradient::from_name(&args.gradient)
        .unwrap_or(Gradient::Rainbow);

    let truecolor = args.truecolor || detect_truecolor();

    let seed = if args.seed == 0 {
        rand::thread_rng().gen_range(0.0..256.0)
    } else {
        args.seed as f64
    };

    let stdout = io::stdout();
    let is_tty = stdout.is_terminal();

    if !is_tty && !args.force {
        let text = read_input(&args.files)?;
        print!("{}", text);
        return Ok(());
    }

    let text = read_input(&args.files)?;

    if args.animate && is_tty {
        let opts = AnimateOpts {
            gradient,
            spread: args.spread,
            freq: args.freq,
            seed,
            duration: args.duration,
            speed: args.speed,
            truecolor,
            invert: args.invert,
        };
        animate(&text, &opts)?;
    } else {
        let opts = RenderOpts {
            gradient,
            spread: args.spread,
            freq: args.freq,
            truecolor,
            invert: args.invert,
        };

        let mut offset = seed;
        for line in text.lines() {
            let colored = render_line(line, offset, &opts);
            println!("{}", colored);
            offset += 1.0;
        }
    }

    Ok(())
}

fn read_input(files: &[String]) -> io::Result<String> {
    if files.is_empty() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        let mut combined = String::new();
        for path in files {
            let content = fs::read_to_string(path)?;
            combined.push_str(&content);
        }
        Ok(combined)
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles without errors

- [ ] **Step 3: Manual test - help output**

Run: `cargo run -- --help`
Expected: Shows help with all documented options

- [ ] **Step 4: Manual test - basic colorization**

Run: `echo "Hello, rainbow!" | cargo run`
Expected: Colorized output with ANSI codes

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add CLI with full argument parsing"
```

---

### Task 8: Integration Tests

**Files:**
- Create: `tests/cli.rs`

**Interfaces:**
- Consumes: compiled binary
- Produces: integration test suite

- [ ] **Step 1: Add assert_cmd dev dependency**

Update `Cargo.toml`, add under `[dependencies]`:

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 2: Create integration test file**

Create `tests/cli.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn lolr() -> Command {
    Command::cargo_bin("lolr").unwrap()
}

#[test]
fn help_works() {
    lolr()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Rainbow colorizer"));
}

#[test]
fn version_works() {
    lolr()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("lolr"));
}

#[test]
fn stdin_produces_ansi() {
    lolr()
        .arg("--force")
        .write_stdin("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));
}

#[test]
fn gradient_option_works() {
    lolr()
        .args(["--force", "--gradient", "fire"])
        .write_stdin("test")
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));
}

#[test]
fn spread_option_works() {
    lolr()
        .args(["--force", "--spread", "5.0"])
        .write_stdin("test")
        .assert()
        .success();
}

#[test]
fn freq_option_works() {
    lolr()
        .args(["--force", "--freq", "0.2"])
        .write_stdin("test")
        .assert()
        .success();
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test --test cli`
Expected: PASS (6 tests)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml tests/
git commit -m "test: add CLI integration tests"
```

---

### Task 9: Documentation and Polish

**Files:**
- Create: `README.md`
- Modify: `Cargo.toml` (metadata)

**Interfaces:**
- Consumes: nothing
- Produces: user-facing documentation

- [ ] **Step 1: Create README.md**

Create `README.md`:

```markdown
# lolr

A Rust CLI that colorizes text with rainbow gradients, including animation support.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Colorize stdin
echo "Hello, world!" | lolr

# Colorize files
lolr file.txt

# With animation
fortune | lolr -a

# Different gradient
cat README.md | lolr --gradient fire
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --spread` | Rainbow spread | 3.0 |
| `-F, --freq` | Rainbow frequency | 0.1 |
| `-S, --seed` | Color seed (0=random) | 0 |
| `-a, --animate` | Enable animation | off |
| `-d, --duration` | Animation frames | 12 |
| `-s, --speed` | Animation FPS | 20 |
| `-g, --gradient` | Gradient preset | rainbow |
| `-i, --invert` | Swap fg/bg | off |
| `-t, --truecolor` | Force 24-bit | auto |
| `-f, --force` | Force on non-TTY | off |

## Gradients

- `rainbow` - Classic sine-wave rainbow
- `fire` - Red → orange → yellow
- `ocean` - Blue → cyan → white
- `pastel` - Soft rainbow
- `neon` - Vibrant high-saturation

## Library Usage

```rust
use lolr::{Gradient, RenderOpts, render_line};

let opts = RenderOpts::default();
let colored = render_line("Hello", 0.0, &opts);
println!("{}", colored);
```

## License

MIT
```

- [ ] **Step 2: Update Cargo.toml metadata**

Update the `[package]` section in `Cargo.toml`:

```toml
[package]
name = "lolr"
version = "0.1.0"
edition = "2021"
description = "Rainbow colorizer for text with animation support"
license = "MIT"
repository = "https://github.com/fifitrixabelle/lolr"
keywords = ["cli", "terminal", "rainbow", "lolcat", "ansi"]
categories = ["command-line-utilities"]
```

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add README.md Cargo.toml
git commit -m "docs: add README and package metadata"
```

---

## Self-Review Checklist (completed)

1. **Spec coverage:** All features from spec covered:
   - Rainbow math ✓ (Task 2)
   - 256-color fallback ✓ (Task 3)
   - Gradient presets ✓ (Task 4)
   - Text rendering ✓ (Task 5)
   - Animation ✓ (Task 6)
   - CLI with all options ✓ (Task 7)
   - Tests ✓ (Tasks 2-8)

2. **Placeholder scan:** No TBD/TODO/placeholders found

3. **Type consistency:** `Rgb`, `Gradient`, `RenderOpts`, `AnimateOpts` used consistently across all tasks
