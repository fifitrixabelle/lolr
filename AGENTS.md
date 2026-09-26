# Working in lolr

lolr is a small Rust CLI and library for coloring terminal text. Keep it fast,
predictable, and fun. Read `README.md` before changing user-facing behavior.

## Map

- `src/main.rs`: CLI, input routing, TTY policy, config precedence.
- `src/config.rs`: config paths, atomic creation, validation.
- `src/color.rs` and `src/gradient.rs`: color math and built-in palettes.
- `src/render.rs`: ANSI-aware text rendering; preserve input control sequences.
- `src/animate.rs`: streaming animation, cursor cleanup, redraw safety.
- `src/lib.rs`: public API; `tests/cli.rs`: end-to-end behavior.

## Invariants

- Non-TTY output is byte-for-byte passthrough unless `--force` is set. Animation
  runs only on a TTY. Preserve final newlines and existing ANSI sequences.
- Keep normal rendering and animation streaming. Avoid whole-input buffering and
  per-character allocations on hot paths.
- Restore terminal color and cursor visibility on cancellation and I/O errors.
  Lines that cannot be redrawn safely should still render once.
- CLI options override config. Keep `--no-*` overrides working. Help, version,
  gradient listing, preview, and config-path queries must not create a config
  file. Honor `NO_COLOR` unless `--force` or configured `force = true` overrides it.
- Preserve ASCII/Ruby color output by default. Named palettes, contrast,
  continuous file coloring, reverse animation, and compact output are opt-in.
- Preserve documented defaults and Ruby lolcat compatibility unless a change is
  intentional, tested, and documented. Consider both CLI and library callers.

## Workflow

1. Make the smallest coherent change. Add tests for behavior and edge cases;
   keep `README.md` and `--help` aligned with user-facing options.
2. Run `just check` (format, Clippy, tests, release build). For renderer speed
   changes, compare `cargo run --release --example bench_render` before and
   after on the same machine and toolchain.
3. Check `git diff --check` and review the full diff. Preserve unrelated work in
   the working tree.

`just release-check` also verifies packaging. `just release VERSION` commits,
tags, pushes, and publishes via GitHub; run it only for an explicit release task.
