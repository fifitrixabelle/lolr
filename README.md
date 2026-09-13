# lolr

[![Crates.io](https://img.shields.io/crates/v/lolr.svg)](https://crates.io/crates/lolr)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust implementation of [lolcat](https://github.com/busyloop/lolcat) with full `-a` animation support. Other Rust ports lack animation; lolr brings it back, plus adds multiple gradient presets.

![nomcat](assets/nom.jpg)

*Cat image from the original lolcat by busyloop.*

## Installation

```bash
cargo install lolr
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

# Show available gradients
lolr --list-gradients

# Multiple files
lolr file1.txt file2.txt

# Custom animation speed
echo "Animated text" | lolr -a --speed 30 --duration 20
```

## Options

| Flag | Long | Description | Default |
|------|------|-------------|---------|
| | `--config PATH` | Use a specific config file | XDG path |
| | `--no-config` | Ignore config and skip creation | off |
| | `--print-config-path` | Print the resolved config path | off |
| `-p` | `--spread` | Rainbow spread | 3.0 |
| `-F` | `--freq` | Rainbow frequency | 0.1 |
| `-S` | `--seed` | Color seed (0=random) | 0 |
| `-a` | `--animate` | Enable animation | off |
| | `--no-animate` | Override configured animation | off |
| `-d` | `--duration` | Animation frames | 6 |
| `-s` | `--speed` | Animation FPS | 40 |
| `-g` | `--gradient` | Gradient preset | rainbow |
| | `--list-gradients` | List gradient presets and exit | |
| `-i` | `--invert` | Swap foreground/background | off |
| | `--no-invert` | Override configured inversion | off |
| `-t` | `--truecolor` | Force 24-bit color | auto |
| | `--no-truecolor` | Disable forced and detected 24-bit color | off |
| `-f` | `--force` | Force color on non-TTY | off |
| | `--no-force` | Override configured forced color | off |

## Gradients

- `rainbow` - Classic sine-wave rainbow
- `fire` - Red → orange → yellow
- `ocean` - Blue → cyan → white
- `pastel` - Soft pastel rainbow
- `neon` - Vibrant high-saturation colors
- `sunset` - Purple → magenta → orange → gold
- `forest` - Deep green → emerald → lime
- `synthwave` - Violet → hot pink → electric blue
- `viridis` - Perceptually balanced purple → teal → yellow
- `aura` - Accent colors from the [Aura Theme](https://github.com/daltonmenezes/aura-theme)

## Configuration

lolr creates a configuration file atomically on first normal run. It resolves
the path in this order:

1. `--config PATH`
2. `$LOLR_CONFIG`
3. `$XDG_CONFIG_HOME/lolr/config.toml`
4. `~/.config/lolr/config.toml`

Run `lolr --print-config-path` to see the resolved path. `--help`, `--version`,
`--list-gradients`, and `--print-config-path` do not create the file. Use
`--no-config` to bypass loading and creation, including to recover from a broken
config.

The generated file contains every configurable default:

```toml
spread = 3.0
freq = 0.1
seed = 0
animate = false
duration = 6
speed = 40.0
invert = false
truecolor = false
force = false
gradient = "rainbow"
```

Command-line options take precedence over config values. The `--no-animate`,
`--no-invert`, `--no-truecolor`, and `--no-force` flags explicitly override
configured `true` values. Missing config keys retain their built-in defaults.
Unknown keys, invalid types, invalid ranges, and config files larger than 1 MiB
are rejected with the path and cause in the error. Existing files are never
rewritten by lolr.

lolr intentionally defaults to a shorter 40 FPS animation instead of lolcat's
12 frames at 20 FPS. Use `--duration 12 --speed 20` for the classic timing.

## Library Usage

```rust
use lolr::{Gradient, RenderOpts, render_line};

let opts = RenderOpts {
    gradient: Gradient::Rainbow,
    spread: 3.0,
    freq: 0.1,
    truecolor: true,
    invert: false,
};

let colored = render_line("Hello, world!", 0.0, &opts);
println!("{}", colored);
```

### Animation

```rust
use lolr::{Gradient, AnimateOpts, animate};

let opts = AnimateOpts {
    gradient: Gradient::Fire,
    spread: 3.0,
    freq: 0.1,
    seed: 42.0,
    duration: 12,
    speed: 20.0,
    truecolor: true,
    invert: false,
};

animate("Animated text", &opts)?;
```

## License

MIT
