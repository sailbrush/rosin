mod draw;
mod geometry;
mod layout;
mod parser;

pub mod alloc;
pub mod callbacks;
pub mod key;
pub mod sheet;
pub mod style;
pub mod tree;
pub mod window;

/// Basic set of widgets
pub mod widgets;

/// The public API
pub mod prelude {
    pub use crate::callbacks::{
        AnimCallback, DrawCallback, DrawCtx, EventCallback, EventCtx, On, Phase, ShouldStop, StyleCallback, ViewCallback,
    };
    pub use crate::key::Key;
    pub use crate::sheet::{SheetId, SheetLoader};
    pub use crate::style::Style;
    pub use crate::tree::Node;
    pub use crate::window::RosinWindow;
    pub use crate::{load_sheet, ui};
}
