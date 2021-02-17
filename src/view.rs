#![forbid(unsafe_code)]

use crate::libloader::LibLoader;
use crate::tree::{Alloc, UI};

use std::fmt;

#[macro_export]
macro_rules! view_new {
    ($id:ident) => {
        View::new(stringify!($id).as_bytes(), $id)
    };
}

pub struct View<T>(&'static [u8], pub for<'a> fn(&'a Alloc, &T) -> UI<'a, T>);

impl<T> fmt::Debug for View<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.0)
    }
}

impl<T> View<T> {
    pub fn new(name: &'static [u8], func: for<'a> fn(&'a Alloc, &T) -> UI<'a, T>) -> Self {
        View::<T>(name, func)
    }

    // Default behaviour
    #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
    pub(crate) fn get(&self, _: &Option<LibLoader>) -> for<'a> fn(&'a Alloc, &T) -> UI<'a, T> {
        self.1
    }

    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    pub(crate) fn get(&self, lib: &Option<LibLoader>) -> for<'a> fn(&'a Alloc, &T) -> UI<'a, T> {
        // Better to panic so it's obvious that hot-reloading failed
        *lib.as_ref()
            .expect("[Rosin] Hot-reload: Not initialized properly")
            .get(self.0)
            .expect("[Rosin] Hot-reload: LibLoading returned an error")
    }
}
