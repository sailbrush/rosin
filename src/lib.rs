#[cfg(test)]
mod tests;

mod app;
mod geometry;
mod key;
mod layout;
mod lens;
mod libloader;
mod parser;
mod render;
mod style;
mod tree;
mod view;
mod window;

pub mod widgets;
pub mod prelude {
    pub use crate::app::{AnimCallback, App, EventCallback, On, Stage, StopTask, StyleCallback, TaskCallback, ViewCallback};
    pub use crate::key::Key;
    pub use crate::lens::Lens;
    pub use crate::style::{Style, Stylesheet};
    pub use crate::tree::{Alloc, Content, Node};
    pub use crate::view::View;
    pub use crate::window::WindowDesc;
    pub use crate::{new_key, new_lens, new_stylesheet, new_view, ui};
}
