//! Reusable widgets.
//!
//! This is currently the least mature module. Expect significant changes and improvements.
//!
//! If what you need isn't here, the [Widget Garden](https://github.com/sailbrush/rosin-widget-garden) may have what you're looking for.
//!
//! ## Overview
//! Rosin provides relatively low level primitives for describing a UI precisely, but it's usually preferable to work at a higher level of abstraction.
//! Widgets are reusable components that handle the low level details for you.
//!
//! Building widgets does not require implementing any traits. Widgets can take whatever shape they need to.
//!
//! The included widgets can be split into three categories:
//! 1) Function widgets: stateless functions that add nodes to the UI tree.
//!     - [`button`](button::button), [`checkbox`](checkbox::checkbox), [`dragvalue`](dragvalue::dragvalue), [`label`](label::label)
//! 2) Parameterized widgets: stateless widgets whose behavior can be customized.
//!     - [`ProgressBarParams`], [`SliderParams`]
//!     - `*Params` can be created in the view callback, and don't need to be stored in the app's state.
//! 3) Stateful widgets: widgets that maintain internal state between frames.
//!     - [`DropDown`], [`PerfDisplay`], [`ScrollArea`], [`TextBox`]
//!     - These widgets need to be stored in the app's state.
//!
//! By convention, Parameterized and Stateful widgets have a `view()`
//! method that takes [`&mut Ui`](crate::tree::Ui) and adds their nodes to the tree.
//!
//! ### Themes
//!
//! The [`dark_theme`] function returns the parsed stylesheet located at `rosin/src/widgets/styles/dark_theme.css`
//! and should be assigned to the root node of the tree, along with a CSS class of `root`.
//!
//! More themes are planned once the set of included widgets has matured.

#![forbid(unsafe_code)]

use std::sync::OnceLock;

use crate::prelude::*;

pub fn dark_theme() -> &'static Stylesheet {
    static DARK_THEME: OnceLock<Stylesheet> = OnceLock::new();
    DARK_THEME.get_or_init(|| stylesheet!("src/widgets/styles/dark_theme.css"))
}

pub(crate) fn widget_styles() -> &'static Stylesheet {
    static WIDGET_STYLES: OnceLock<Stylesheet> = OnceLock::new();
    WIDGET_STYLES.get_or_init(|| stylesheet!("src/widgets/styles/widgets.css"))
}

mod button;
mod checkbox;
mod dragvalue;
mod dropdown;
mod label;
mod perfdisplay;
mod progressbar;
mod scrollarea;
mod slider;
mod textbox;

pub use button::*;
pub use checkbox::*;
pub use dragvalue::*;
pub use dropdown::*;
pub use label::*;
pub use perfdisplay::*;
pub use progressbar::*;
pub use scrollarea::*;
pub use slider::*;
pub use textbox::*;
