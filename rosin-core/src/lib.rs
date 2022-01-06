#[macro_use]
extern crate lazy_static;

mod geometry;
mod layout;
mod parser;

pub mod alloc;
pub mod callbacks;
pub mod grc;
pub mod key;
pub mod render;
pub mod sheet;
pub mod style;
pub mod tree;
pub mod window;

/// Basic set of widgets
pub mod widgets;

/// The public API
pub mod prelude {
    pub use crate::alloc::Alloc;
    pub use crate::callbacks::{AnimCallback, DrawCallback, EventCallback, EventCtx, On, Phase, ShouldStop, StyleCallback, ViewCallback};
    pub use crate::grc::{Grc, Weak};
    pub use crate::key::Key;
    pub use crate::render::DrawCtx;
    pub use crate::sheet::{SheetId, SheetLoader};
    pub use crate::style::Style;
    pub use crate::tree::Node;
    pub use crate::window::RosinWindow;
    pub use crate::{load_sheet, ui};
}
