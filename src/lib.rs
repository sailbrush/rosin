#[cfg(test)]
mod tests;

mod app;
mod layout;
mod libloader;
mod parser;
mod render;
mod style;
mod tree;
mod view;
mod window;

pub mod widgets;
pub mod prelude {
    pub use crate::app::{App, AppLauncher, On, Redraw, StopTask};
    pub use crate::style::{Style, Stylesheet};
    pub use crate::tree::{Content, NodeID, TreeNode, UI};
    pub use crate::view::View;
    pub use crate::window::WindowDesc;
    pub use crate::{style_new, ui, view_new};

    pub use bumpalo::{collections::Vec as BumpVec, Bump};
}
