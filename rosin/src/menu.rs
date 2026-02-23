//! Platform agnostic native menu descriptions.

use std::sync::Arc;

use crate::prelude::*;

/// A platform-agnostic definition of a keyboard shortcut.
#[cfg_attr(feature = "hot-reload", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotKey {
    pub key: Key,
    pub mods: Modifiers,
}

impl HotKey {
    pub fn new(mods: Modifiers, key: Key) -> Self {
        Self { key, mods }
    }

    /// Creates a shortcut using the platform's primary modifier key.
    pub fn primary(key: Key) -> Self {
        #[cfg(target_os = "macos")]
        let mods = Modifiers::META;
        #[cfg(not(target_os = "macos"))]
        let mods = Modifiers::CONTROL;

        Self::new(mods, key)
    }

    /// Primary + Shift + Key
    pub fn primary_shift(key: Key) -> Self {
        #[cfg(target_os = "macos")]
        let mods = Modifiers::META | Modifiers::SHIFT;
        #[cfg(not(target_os = "macos"))]
        let mods = Modifiers::CONTROL | Modifiers::SHIFT;

        Self::new(mods, key)
    }
}

/// Standard system actions that often have specific platform behavior or icons.
#[cfg_attr(all(feature = "hot-reload", debug_assertions), derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardAction {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

/// An entry in a menu.
#[cfg_attr(all(feature = "hot-reload", debug_assertions), derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum MenuItem {
    /// A standard clickable item.
    Action {
        title: LocalizedString,
        /// Application menu command events are sent to the root node.
        command: CommandId,
        shortcut: Option<HotKey>,
        enabled: bool,
        selected: bool,
    },
    /// A submenu.
    Submenu { title: LocalizedString, menu: MenuDesc, enabled: bool },
    /// A standard system item.
    Standard(StandardAction),
    /// A visual separator line.
    Separator,
}

/// A description of a native menu.
#[cfg_attr(all(feature = "hot-reload", debug_assertions), derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct MenuDesc {
    pub items: Arc<Vec<MenuItem>>,
}

impl MenuDesc {
    pub fn new() -> Self {
        Self { items: Arc::new(Vec::new()) }
    }

    /// Convenience if you already have a Vec.
    pub fn from_items(items: Vec<MenuItem>) -> Self {
        Self { items: Arc::new(items) }
    }

    /// Cheap shared append: clones the Vec only if this MenuDesc is shared.
    pub fn add_item(mut self, item: MenuItem) -> Self {
        Arc::make_mut(&mut self.items).push(item);
        self
    }

    /// Cheap shared append: clones the Vec only if this MenuDesc is shared.
    pub fn add_separator(mut self) -> Self {
        Arc::make_mut(&mut self.items).push(MenuItem::Separator);
        self
    }
}
