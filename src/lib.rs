//! # lolr
//!
//! Rainbow colorizer for text with animation support.
//!
//! ## Quick Start
//!
//! ```rust
//! use lolr::{render_line, Gradient, RenderOpts};
//!
//! let opts = RenderOpts {
//!     gradient: Gradient::Rainbow,
//!     spread: 3.0,
//!     freq: 0.1,
//!     truecolor: true,
//!     invert: false,
//! };
//!
//! let colored = render_line("Hello, world!", 0.0, &opts);
//! assert!(colored.contains("\x1b[38;2;"));
//! ```
//!
//! ## Streaming with Reusable Buffers
//!
//! ```rust
//! use lolr::{Renderer, RenderOpts, RenderStyle};
//!
//! let opts = RenderOpts::default();
//! let style = RenderStyle::default();
//! let mut renderer = Renderer::default();
//! let mut output = Vec::new();
//! for (offset, line) in ["Hello", "world!"].iter().enumerate() {
//!     renderer.render_into(line, offset as f64, &opts, &style, &mut output);
//!     assert!(!output.is_empty());
//! }
//! ```
//!
//! ## Prepared Animation Lines
//!
//! ```rust
//! use lolr::{PreparedLine, RenderOpts, RenderStyle};
//!
//! let line = PreparedLine::new("Hello 界 👩‍💻");
//! let mut output = Vec::new();
//! for frame in 0..6 {
//!     line.render_into(frame as f64, &RenderOpts::default(), &RenderStyle::default(), &mut output);
//!     assert!(!output.is_empty());
//! }
//! ```

pub mod animate;
pub mod color;
pub mod gradient;
pub mod render;

pub use animate::{
    animate, animate_line, animate_reader_styled_until, animate_reader_until, animate_until,
    AnimateOpts, AnimationDirection,
};
pub use color::{rainbow_color, rgb_to_256, Rgb};
pub use gradient::{gradient_color, Gradient, Palette, ParseGradientError};
pub use render::{
    render_line, render_line_styled, render_line_styled_into, Background, PreparedLine, RenderOpts,
    RenderStyle, Renderer,
};
