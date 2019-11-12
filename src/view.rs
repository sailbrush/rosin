#[cfg(debug_assertions)]
use std::marker::PhantomData;

use libloading::Library;

use crate::dom::*;

#[cfg(target_os = "windows")]
pub const DYLIB_EXT: &str = "dll";

#[cfg(target_os = "macos")]
pub const DYLIB_EXT: &str = "dylib";

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub const DYLIB_EXT: &str = "so";

#[cfg(not(debug_assertions))]
pub struct View<T>(pub fn(&T) -> Dom<T>);

#[cfg(debug_assertions)]
pub struct View<T>(&'static [u8], PhantomData<T>);

impl<T> View<T> {
    #[cfg(not(debug_assertions))]
    pub fn new(func: fn(&T) -> Dom<T>) -> Self {
        View::<T>(func)
    }

    #[cfg(debug_assertions)]
    pub fn new(name: &'static [u8]) -> Self {
        View::<T>(name, PhantomData)
    }

    #[cfg(not(debug_assertions))]
    pub fn get(&self, _: &Option<Library>) -> (fn(&T) -> Dom<T>) {
        self.0
    }

    #[cfg(debug_assertions)]
    pub fn get(&self, lib: &Option<Library>) -> (fn(&T) -> Dom<T>) {
        unsafe { *lib.as_ref().unwrap().get(self.0).unwrap() }
    }
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! view_new {
    ($id:ident) => {
        View::new($id)
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! view_new {
    ($id:ident) => {
        View::new(stringify!($id).as_bytes())
    };
}
