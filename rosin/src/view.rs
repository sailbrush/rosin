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
pub struct View<S: 'static, H: 'static> {
    pub(crate) name: &'static [u8],
    pub(crate) func: ViewCallback<S, H>,
}

impl<S, H> Debug for View<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.name)
    }
}

#[doc(hidden)]
impl<S, H> View<S, H> {
    pub fn new(name: &'static [u8], func: ViewCallback<S, H>) -> Self {
        Self { name, func }
    }
}
