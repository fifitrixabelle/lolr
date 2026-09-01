# lolr Design Spec

A Rust CLI and library that replicates Ruby lolcat, including the `-a/--animate` feature missing from other Rust ports.

## Goals

- Faithful port of Ruby lolcat behavior
- Animation support (`-a`) via cursor save/restore
- Gradient presets beyond classic rainbow
- Cross-platform via crossterm
- CLI-first, library as secondary API

## Architecture

```
lolr/
├── src/
│   ├── main.rs          # CLI entry, arg parsing (clap)
│   ├── lib.rs           # Public API for library use
│   ├── color.rs         # Rainbow math, gradient generation
│   ├── gradient.rs      # Preset gradients (rainbow, fire, ocean, etc.)
│   ├── render.rs        # Apply colors to text, handle ANSI pass-through
│   └── animate.rs       # Animation loop, cursor control via crossterm
├── Cargo.toml
└── README.md
```

### Dependencies

- `clap` (derive) — argument parsing
- `crossterm` — terminal control, colors, cursor positioning
- `rand` — seed randomization

### Data Flow

```
stdin/file → render (colorize each char) → stdout
                ↑
         gradient + offset
                ↑
         animate loop (if -a)
```

## Color Algorithm

### Rainbow (faithful to Ruby)

```rust
fn rainbow_color(freq: f64, i: f64) -> (u8, u8, u8) {
    let r = (freq * i + 0.0).sin() * 127.0 + 128.0;
    let g = (freq * i + 2.0 * PI / 3.0).sin() * 127.0 + 128.0;
    let b = (freq * i + 4.0 * PI / 3.0).sin() * 127.0 + 128.0;
    (r as u8, g as u8, b as u8)
}
```

### Gradient Presets

```rust
enum Gradient {
    Rainbow,           // classic sine wave
    Fire,              // red → orange → yellow
    Ocean,             // blue → cyan → white
    Pastel,            // lower saturation rainbow
    Neon,              // high saturation vibrant
    Custom(Vec<RGB>),  // user-defined stops
}
```

Presets use linear interpolation between color stops. Rainbow uses sine math.

### 256-Color Fallback

Convert RGB to nearest xterm-256 color when truecolor unavailable. Detect via `COLORTERM=truecolor` env var.

## Animation

### Mechanism

```rust
fn animate(text: &str, opts: &AnimateOpts) {
    execute!(stdout(), SavePosition)?;
    
    for frame in 0..opts.duration {
        execute!(stdout(), RestorePosition)?;
        let offset = opts.seed + (frame as f64 * opts.spread);
        render_colored(text, offset, &opts.gradient);
        thread::sleep(Duration::from_secs_f64(1.0 / opts.speed));
    }
}
```

### Defaults (matching Ruby)

- duration: 12 frames
- speed: 20 fps (50ms per frame)
- spread: 3.0

### Edge Cases

- Non-TTY: skip animation, colorize once
- Ctrl+C: crossterm raw mode cleanup via Drop guard
- Large input: buffer in memory (animation needs full text for redraw)

## CLI Interface

```
lolr [OPTIONS] [FILE]...

Arguments:
  [FILE]...  Files to read (default: stdin)

Options:
  -p, --spread <N>      Rainbow spread [default: 3.0]
  -F, --freq <N>        Rainbow frequency [default: 0.1]
  -S, --seed <N>        Color seed, 0 = random [default: 0]
  -a, --animate         Enable animation
  -d, --duration <N>    Animation frames [default: 12]
  -s, --speed <N>       Animation FPS [default: 20]
  -i, --invert          Swap foreground/background
  -t, --truecolor       Force 24-bit color
  -f, --force           Force color on non-TTY
  -g, --gradient <NAME> Gradient preset [default: rainbow]
                        Options: rainbow, fire, ocean, pastel, neon
  -h, --help            Print help
  -V, --version         Print version
```

## Library API

```rust
use lolr::{Gradient, rainbow_string, animate};

// One-shot colorization
let colored = rainbow_string("Hello", Gradient::Rainbow, opts);

// Animation
animate(&text, AnimateOpts::default());
```

## Testing Strategy

### Unit Tests

- `color.rs` — verify sine math produces expected RGB at known offsets
- `gradient.rs` — interpolation between color stops
- `render.rs` — ANSI escape preservation, UTF-8 handling

### Integration Tests

- CLI arg parsing via `assert_cmd`
- Output contains expected ANSI codes
- `--force` works on non-TTY

### Manual Verification

- Visual check in terminal
- Compare against Ruby lolcat output

Animation is visual/timing-based — test the render function it calls, not the animation loop itself.
