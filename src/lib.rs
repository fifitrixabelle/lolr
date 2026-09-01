pub mod animate;
pub mod color;
pub mod gradient;
pub mod render;

pub use animate::{animate, AnimateOpts};
pub use color::{rainbow_color, rgb_to_256, Rgb};
pub use gradient::{gradient_color, Gradient};
pub use render::{render_line, RenderOpts};
