#[cfg(test)]
mod tests;

mod alloc;
mod app;
mod geometry;
mod key;
mod layout;
mod lenses;
mod libloader;
mod parser;
mod render;
mod style;
mod tree;
mod view;
mod window;

pub mod widgets;
pub mod prelude {
    pub use crate::app::{AnimCallback, App, AppLauncher, EventCallback, On, Stage, StopTask, StyleCallback, TaskCallback, ViewCallback};
    pub use crate::key::Key;
    pub use crate::lenses::{CompoundLens, Lens, SingleLens};
    pub use crate::style::{Style, Stylesheet};
    pub use crate::tree::{Content, Node};
    pub use crate::view::View;
    pub use crate::window::WindowDesc;
    pub use crate::{lens, new_key, new_style, new_view, ui};
}
