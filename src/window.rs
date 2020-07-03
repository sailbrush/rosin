use std::sync::atomic::{AtomicU32, Ordering};

use druid_shell::kurbo::Size;
use druid_shell::WindowBuilder;

use crate::view::View;

static WINDOW_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowID(u32);

impl WindowID {
    pub fn new() -> Self {
        let id = WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
}

pub struct WindowDesc<T> {
    pub(crate) builder: WindowBuilder,
    pub(crate) view: View<T>,
}

impl<T> WindowDesc<T> {
    pub fn new(view: View<T>) -> Self {
        Self {
            builder: WindowBuilder::new(),
            view,
        }
    }

    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.builder.set_title(title.into());
        self
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.builder.set_size(Size::new(width, height));
        self
    }
}
