//! Handles for functions that Rosin calls to display the UI.

use std::fmt;

use crate::prelude::*;

/// A macro that takes a function item and makes it useable as either a [`ViewFn`] or a [`WgpuFn`].
///
/// For example:
///
/// ```ignore
/// fn my_view(...) { ... }
///
/// let desc = WindowDesc::new(callback!(my_view));
/// ```
#[macro_export]
macro_rules! callback {
    ($func:path) => {{ (stringify!($func), $func as _) }};
}

/// A handle for a function that will be called to construct a UI tree.
///
/// The [`callback`] macro takes a function item with the signature `fn(&S, &mut Ui<S, WindowHandle>)` and creates a tuple that implements `Into<ViewFn>`.
pub struct ViewFn<S: 'static, H: 'static> {
    pub(crate) symbol: &'static str,
    pub(crate) func: for<'a, 'b> fn(&'a S, &'b mut Ui<S, H>),
}

impl<S, H> Copy for ViewFn<S, H> {}
impl<S, H> Clone for ViewFn<S, H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, H> fmt::Display for ViewFn<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl<S, H> fmt::Debug for ViewFn<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.symbol)
    }
}

impl<S, H> From<(&'static str, for<'a, 'b> fn(&'a S, &'b mut Ui<S, H>))> for ViewFn<S, H> {
    fn from((symbol, func): (&'static str, for<'a, 'b> fn(&'a S, &'b mut Ui<S, H>))) -> Self {
        Self { symbol, func }
    }
}

/// A handle for a function that will be called to run custom WGPU work.
///
/// The [`callback`] macro takes a function item with the signature `fn(&S, &mut WgpuCtx<'_>)` and creates a tuple that implements `Into<WgpuFn>`.
pub struct WgpuFn<S: 'static> {
    pub(crate) symbol: &'static str,
    pub(crate) func: for<'a, 'b, 'c> fn(&'a S, &'b mut WgpuCtx<'c>),
}

impl<S> Copy for WgpuFn<S> {}
impl<S> Clone for WgpuFn<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> fmt::Display for WgpuFn<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl<S> fmt::Debug for WgpuFn<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}()", self.symbol)
    }
}

impl<S> From<(&'static str, for<'a, 'b, 'c> fn(&'a S, &'b mut WgpuCtx<'c>))> for WgpuFn<S> {
    fn from((symbol, func): (&'static str, for<'a, 'b, 'c> fn(&'a S, &'b mut WgpuCtx<'c>))) -> Self {
        Self { symbol, func }
    }
}
