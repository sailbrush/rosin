//! Rosin is a GUI library.
//!
//! ### Quick Links
//! - The [`css`] module documentation lists which CSS features are supported.
//! - The [`layout`] module documentation explains how to use the layout system.
//! - The [`localization`] module documentation explains how to use the localization features.
//! - The [`reactive`] module documentation explains how to use reactive variables.
//! - The [`prelude`] module exports all of the relevant types for building an application with Rosin.
//! - The [`widgets`] module contains reusable widgets.
//!
//! ### The Basics
//! At a high level, starting an application simply requires constructing an [`AppLauncher`](app::AppLauncher)
//! and calling [`run`](app::AppLauncher::run). In order to construct an [`AppLauncher`](app::AppLauncher),
//! you provide a [`WindowDesc`](desc::WindowDesc) that describes the properties of the first window,
//! as well as optional configuration parameters.
//!
//! In Rosin, a UI is described declaratively as a pure function of state. So, to create a [`WindowDesc`](desc::WindowDesc)
//! you provide a function that will be called to construct the UI tree for that window using a simple [`builder-style API`](tree::Ui).
//!
//! The [`Var`](reactive::Var) type is used to track changes to application state, automatically updating the screen when needed.
//! Anything visible and dynamic should be stored in a [`Var`](reactive::Var).
//!
//! In order to provide a stable identity for nodes between rebuilds, the [`id`] macro can be used to generate a unique ID based on the call location.
//!
//! The [`EventCtx`](rosin_core::events::EventCtx) struct is the primary interface for reacting to user input.
//!
//! The [`WindowHandle`](crate::handle::WindowHandle) struct is the primary interface for interacting with the platform.
//!
//! **Example:**
//! ```ignore
//! use rosin::{prelude::*, widgets::*};
//!
//! // All application state is stored in a single type
//! struct State {
//!     style: Stylesheet,
//!     count: Var<i32>,
//! }
//!
//! impl Default for State {
//!     fn default() -> Self {
//!         Self {
//!             style: stylesheet!("examples/styles/counter.css"),
//!             count: Var::new(0),
//!         }
//!     }
//! }
//!
//! fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
//!     // create the root node
//!     ui.node()
//!         // assign a stylesheet
//!         .style_sheet(&state.style)
//!         // add a CSS class
//!         .classes("root")
//!         // add children
//!         .children(|ui| {
//!             // build a label widget with an additional CSS class
//!             label(ui, id!(), *state.count).classes("number");
//!             // build a button widget that increases the count when activated
//!             button(ui, id!(), "Count", |s, _| {
//!                 *s.count.write() += 1;
//!             });
//!         });
//! }
//!
//! fn main() {
//!     // init the logger so errors can be displayed
//!     env_logger::init();
//!
//!     // describe a simple window
//!     let window = WindowDesc::new(callback!(main_view))
//!         .title("Counter Example")
//!         .size(400, 300)
//!         .min_size(250, 150);
//!
//!     // launch the app with an empty translation map
//!     AppLauncher::new(window)
//!         .run(State::default(), TranslationMap::default())
//!         .expect("Failed to launch");
//! }
//! ```

#[doc(inline)]
pub use rosin_core::{css, data, events, layout, localization, nodeid, pointer, reactive, text, tree};

#[cfg(feature = "icu")]
#[doc(inline)]
pub use rosin_core::time;

pub use rosin_core::accesskit;
pub use rosin_core::keyboard_types;
pub use rosin_core::kurbo;
pub use rosin_core::log;
pub use rosin_core::parking_lot;
pub use rosin_core::parley;
pub use rosin_core::peniko;
pub use rosin_core::unic_langid;
pub use rosin_core::vello;
pub use rosin_core::wgpu;

pub mod app;
pub mod callbacks;
pub mod desc;
pub mod dialog;
pub mod gpu;
pub mod handle;
pub mod ime;
pub mod menu;
pub mod widgets;

#[cfg(all(feature = "hot-reload", debug_assertions))]
pub mod typehash;

#[cfg(target_os = "macos")]
#[doc(hidden)]
pub mod mac;

#[cfg(target_os = "macos")]
use crate::mac as platform;

#[cfg(target_os = "windows")]
#[doc(hidden)]
pub mod win;

#[cfg(target_os = "windows")]
use crate::win as platform;

#[cfg(target_os = "linux")]
#[doc(hidden)]
pub mod linux;

#[cfg(target_os = "linux")]
use crate::linux as platform;

#[doc(inline)]
pub use rosin_core::{id, stylesheet, ui_format};

/// The public API
pub mod prelude {
    pub use crate::callback;
    pub use crate::unic_langid::{langid, langids};
    pub use rosin_core::{id, stylesheet, ui_format};

    #[doc(inline)]
    pub use crate::app::*;

    #[doc(inline)]
    pub use crate::callbacks::*;

    #[doc(inline)]
    pub use crate::desc::*;

    #[doc(inline)]
    pub use crate::dialog::*;

    #[doc(inline)]
    pub use crate::gpu::*;

    #[doc(inline)]
    pub use crate::handle::*;

    #[doc(inline)]
    pub use crate::ime::*;

    #[doc(inline)]
    pub use crate::menu::*;

    pub use rosin_core::css::*;
    pub use rosin_core::data::*;
    pub use rosin_core::localization::*;
    pub use rosin_core::nodeid::*;
    pub use rosin_core::pointer::*;
    pub use rosin_core::reactive::*;
    pub use rosin_core::text::*;
    pub use rosin_core::tree::*;

    // Everything except DispatchInfo
    pub use rosin_core::events::{AccessibilityCtx, CanvasCtx, CommandId, EventCtx, EventInfo, FileDialogResponse, MeasureCtx, On, PerfInfo};

    pub use crate::keyboard_types::*;
    pub use crate::parley::editing::Cursor;

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub use rosin_derive::*;

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub use crate::typehash::TypeHash;

    #[cfg(feature = "icu")]
    pub use crate::time::{Date, OffsetDateTime, Time};
}
