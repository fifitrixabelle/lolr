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
