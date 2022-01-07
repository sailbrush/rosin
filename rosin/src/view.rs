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
pub struct View<T: 'static> {
    pub(crate) name: &'static [u8],
    pub(crate) func: ViewCallback<T>,
}

impl<T> Debug for View<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.name)
    }
}

#[doc(hidden)]
impl<T> View<T> {
    pub fn new(name: &'static [u8], func: ViewCallback<T>) -> Self {
        Self { name, func }
    }
}
