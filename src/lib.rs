#[cfg(test)]
mod tests;

mod app;
mod geometry;
mod layout;
mod libloader;
mod minmax;
mod parser;
mod render;
mod style;
mod tree;
mod view;
mod window;

pub mod widgets;
pub mod prelude {
    pub use crate::app::{App, On, Stage, StopTask};
    pub use crate::style::{Style, Stylesheet};
    pub use crate::tree::{Alloc, Content, NodeID, TreeNode, UI};
    pub use crate::view::View;
    pub use crate::window::WindowDesc;
    pub use crate::{style_new, ui, view_new};
}
