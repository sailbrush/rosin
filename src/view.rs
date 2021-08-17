#![forbid(unsafe_code)]

use std::{fmt, fmt::Debug};

use crate::libloader::LibLoader;
use crate::prelude::*;

#[macro_export]
macro_rules! new_view {
    ($($id:tt)*) => {
        View::new(stringify!($($id)*).as_bytes(), $($id)*)
    };
}

pub struct View<T: 'static>(&'static [u8], pub ViewCallback<T>);

impl<T> Debug for View<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.0)
    }
}

impl<T> View<T> {
    pub fn new(name: &'static [u8], func: ViewCallback<T>) -> Self {
        View::<T>(name, func)
    }

    // Default behaviour
    #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
    pub(crate) fn get(&self, _: &Option<LibLoader>) -> ViewCallback<T> {
        self.1
    }

    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    pub(crate) fn get(&self, lib: &Option<LibLoader>) -> ViewCallback<T> {
        // Better to panic so it's obvious that hot-reloading failed
        *lib.as_ref()
            .expect("[Rosin] Hot-reload: Not initialized properly")
            .get(self.0)
            .expect("[Rosin] Hot-reload: LibLoading returned an error")
    }
}
