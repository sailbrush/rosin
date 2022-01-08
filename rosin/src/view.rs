#![forbid(unsafe_code)]

use std::{fmt, fmt::Debug};

use crate::prelude::ViewCallback;

/// Create a View.
#[macro_export]
macro_rules! new_view {
    ($($id:tt)*) => {
        View::new(stringify!($($id)*).as_bytes(), $($id)*)
    };
}

/// A handle to a function that will be called to construct a view tree. Create a View with the `new_view!()` macro.
pub struct View<S: 'static> {
    pub(crate) name: &'static [u8],
    pub(crate) func: ViewCallback<S>,
}

impl<S> Debug for View<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.name)
    }
}

#[doc(hidden)]
impl<S> View<S> {
    pub fn new(name: &'static [u8], func: ViewCallback<S>) -> Self {
        Self { name, func }
    }
}
