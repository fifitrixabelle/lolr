pub mod color;
pub mod gradient;
pub mod render;
pub mod animate;

pub use color::{Rgb, rainbow_color, rgb_to_256};
pub use gradient::{Gradient, gradient_color};
pub use render::{RenderOpts, render_line};
pub use animate::{AnimateOpts, animate};
