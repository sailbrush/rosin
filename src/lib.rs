#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod tests;

#[cfg(all(debug_assertions, feature = "hot-reload"))]
mod hot_reload;
#[cfg(all(debug_assertions, feature = "hot-reload"))]
mod libloader;

#[cfg(not(all(debug_assertions, feature = "hot-reload")))]
mod libloader {
    pub(crate) struct LibLoader {}
}

mod alloc;
mod app;
mod grc;
mod geometry;
mod key;
mod layout;
mod lenses;
mod parser;
mod render;
mod style;
mod tree;
mod view;
mod window;

/// Basic set of widgets
pub mod widgets;

/// The public API
pub mod prelude {
    pub use crate::app::{
        AnimCallback, App, AppLauncher, DrawCallback, EventCallback, EventCtx, On, Phase, StopTask, StyleCallback, TaskCallback,
        ViewCallback,
    };
    pub use crate::grc::{Grc, Weak};
    pub use crate::key::Key;
    pub use crate::lenses::{CompoundLens, Lens, SingleLens};
    pub use crate::render::DrawCtx;
    pub use crate::style::{SheetId, Style, Stylesheet};
    pub use crate::tree::Node;
    pub use crate::view::View;
    pub use crate::window::WindowDesc;
    pub use crate::{lens, new_style, new_view, ui};

    pub use femtovg::{Color, Paint, Path};
}
