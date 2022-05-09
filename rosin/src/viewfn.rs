#![forbid(unsafe_code)]

use std::{fmt, fmt::Debug};

use crate::prelude::ViewCallback;

/// Create a ViewFn.
#[macro_export]
macro_rules! new_viewfn {
    ($($id:tt)*) => {
        ViewFn::new(stringify!($($id)*).as_bytes(), $($id)*)
    };
}

/// A handle to a function that will be called to construct a view tree. Create a ViewFn with the `new_viewfn!()` macro.
pub struct ViewFn<S: 'static, H: 'static> {
    pub(crate) name: &'static [u8],
    pub(crate) func: ViewCallback<S, H>,
}

impl<S, H> Debug for ViewFn<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.name)
    }
}

#[doc(hidden)]
impl<S, H> ViewFn<S, H> {
    pub fn new(name: &'static [u8], func: ViewCallback<S, H>) -> Self {
        Self { name, func }
    }
}
