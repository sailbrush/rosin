#![cfg_attr(not(loom), deny(unsafe_code))]

#[cfg(test)]
#[allow(unsafe_code)]
mod tests;

mod draw;
mod hasher;
mod sync;
mod util;

#[doc(hidden)]
pub mod interner;

pub mod css;
pub mod data;
pub mod events;
pub mod layout;
pub mod localization;
pub mod nodeid;
pub mod pointer;
pub mod reactive;
pub mod text;
pub mod tree;
pub mod viewport;

pub use accesskit;
pub use keyboard_types;
pub use kurbo;
pub use log;
pub use parking_lot;
pub use parley;
pub use unic_langid;
pub use vello;
pub use vello::peniko;
pub use vello::wgpu;

#[cfg(feature = "icu")]
pub use time;

/// The public API
pub mod prelude {
    pub use crate::{id, stylesheet, ui_format};

    #[doc(inline)]
    pub use crate::css::*;

    #[doc(inline)]
    pub use crate::data::*;

    #[doc(inline)]
    pub use crate::events::*;

    #[doc(inline)]
    pub use crate::localization::*;

    #[doc(inline)]
    pub use crate::nodeid::*;

    #[doc(inline)]
    pub use crate::pointer::*;

    #[doc(inline)]
    pub use crate::reactive::{DependencyMap, Var, WeakVar};

    #[doc(inline)]
    pub use crate::text::*;

    #[doc(inline)]
    pub use crate::tree::*;

    #[doc(inline)]
    pub use crate::viewport::*;

    pub use unic_langid::{langid, langids};

    pub use keyboard_types::{Key, KeyState, NamedKey};

    #[cfg(feature = "icu")]
    pub use time::{Date, OffsetDateTime, Time};
}
