//! Types for UI parameters and text.

use std::{
    fmt::{self, Write},
    num::{NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128},
    ops::Deref,
    sync::Arc,
};

use parking_lot::MappedRwLockReadGuard;
use smallstr::SmallString;

use crate::{prelude::*, reactive::VarReadGuard};

#[doc(hidden)]
pub type StackString = SmallString<[u8; 64]>;

/// Constructs a [`UIString`] with a custom format string.
///
/// Example:
/// ```rust,ignore
/// let value: WeakVar<f32> = ...;
/// let example: UIString = ui_format!(value, "{:.2}");
/// ```
#[macro_export]
macro_rules! ui_format {
    ($val:expr, $fmt:literal) => {{
        let var = ($val).clone();
        UIString::__deferred_stack(move || {
            use std::fmt::Write;
            use $crate::data::StackString;

            let v = var.read()?;
            let mut buf = StackString::new();
            let _ = write!(&mut buf, $fmt, &*v);
            Some(buf)
        })
    }};
}

/// A wrapper type that unifies the different storage requirements of a resolved UIString.
///
/// It implements `Deref<Target = str>`, so it can be used just like a `&str`.
pub enum UIStringRef<'a> {
    /// A static string literal.
    Borrowed(&'a str),
    /// A lock guard holding a reference to a resolved localized string.
    Guard(MappedRwLockReadGuard<'a, str>),
    /// A lock guard holding a reference to a VarString.
    VarString(VarReadGuard<'a, String>),
    /// An owned string on the stack created by formatting.
    OwnedStackString(StackString),
    /// An owned string created by user-provided closures.
    OwnedString(String),
}

impl<'a> Deref for UIStringRef<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            UIStringRef::Borrowed(s) => s,
            UIStringRef::Guard(g) => g,
            UIStringRef::VarString(g) => g.as_str(),
            UIStringRef::OwnedStackString(s) => s.as_str(),
            UIStringRef::OwnedString(s) => s.as_str(),
        }
    }
}

impl<'a> From<UIStringRef<'a>> for Box<str> {
    fn from(value: UIStringRef<'a>) -> Self {
        match value {
            UIStringRef::OwnedString(s) => s.into_boxed_str(),
            UIStringRef::Borrowed(s) => s.to_owned().into_boxed_str(),
            UIStringRef::Guard(g) => g.to_owned().into_boxed_str(),
            UIStringRef::VarString(g) => g.to_owned().into_boxed_str(),
            UIStringRef::OwnedStackString(s) => s.as_str().to_owned().into_boxed_str(),
        }
    }
}

/// A concrete type that represents a string that can be displayed on screen.
///
/// Most widgets will accept an `impl Into<UIString>` parameter to receive the text that they should display.
///
/// [`Into<UIString>`] has been implemented for:
///
/// - `&'static str`, [`String`], [`LocalizedString`]
/// - [`WeakVar<T>`] where `T` is:
///
/// `&'static str`, [`String`], [`f32`], [`f64`], [`bool`], [`char`], [`usize`], [`isize`],
/// [`u8`], [`u16`], [`u32`], [`u64`], [`u128`],
/// [`i8`], [`i16`], [`i32`], [`i64`], [`i128`],
/// [`NonZeroU8`], [`NonZeroU16`], [`NonZeroU32`], [`NonZeroU64`],
/// [`NonZeroU128`], [`NonZeroI8`], [`NonZeroI16`], [`NonZeroI32`],
/// [`NonZeroI64`], or [`NonZeroI128`].
///
/// Every type except `&'static str` and [`String`] will automatically update the screen when changed.
///
/// The [`ui_format`] macro can be used to create a [`UIString`] from a value using a custom format string.
#[derive(Clone, Debug)]
pub struct UIString(UIStringInner);

#[derive(Clone)]
enum UIStringInner {
    Static(&'static str),
    Owned(String),
    VarStaticStr(WeakVar<&'static str>),
    VarString(WeakVar<String>),
    Localized(LocalizedString),
    DeferredStack(Arc<dyn Fn() -> Option<StackString> + Send + Sync + 'static>),
    DeferredHeap(Arc<dyn for<'a> Fn(&'a TranslationMap) -> String + Send + Sync + 'static>),
}

impl fmt::Debug for UIStringInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UIStringInner::Owned(s) => f.debug_tuple("Owned").field(s).finish(),
            UIStringInner::Localized(_) => f.debug_tuple("Localized").field(&"<localized>").finish(),
            UIStringInner::Static(s) => f.debug_tuple("Static").field(s).finish(),
            UIStringInner::VarString(_) => f.debug_tuple("VarString").field(&"<var>").finish(),
            UIStringInner::VarStaticStr(_) => f.debug_tuple("VarStaticStr").field(&"<var>").finish(),
            UIStringInner::DeferredStack(_) => f.debug_tuple("DeferredStack").field(&"<deferred>").finish(),
            UIStringInner::DeferredHeap(_) => f.debug_tuple("DeferredHeap").field(&"<deferred>").finish(),
        }
    }
}

impl UIString {
    /// This is used internally to keep [`UIString::resolve`] heap-free for simple string formatting.
    #[doc(hidden)]
    pub fn __deferred_stack<F>(f: F) -> Self
    where
        F: Fn() -> Option<SmallString<[u8; 64]>> + Send + Sync + 'static,
    {
        UIString(UIStringInner::DeferredStack(Arc::new(f)))
    }

    /// Creates a `UIString` that is computed later during [`UIString::resolve`].
    pub fn deferred<F>(f: F) -> Self
    where
        F: for<'a> Fn(&'a TranslationMap) -> String + Send + Sync + 'static,
    {
        UIString(UIStringInner::DeferredHeap(Arc::new(f)))
    }

    /// Resolves the string to a type that derefs to `&str`.
    ///
    /// This keeps the underlying lock active for the lifetime of the returned `UIStringRef`.
    pub fn resolve<'a>(&'a self, translation_map: &'a TranslationMap) -> Option<UIStringRef<'a>> {
        match &self.0 {
            UIStringInner::Owned(s) => Some(UIStringRef::Borrowed(s.as_str())),
            UIStringInner::Localized(localized_string) => Some(UIStringRef::Guard(localized_string.resolve(translation_map))),
            UIStringInner::Static(string) => Some(UIStringRef::Borrowed(string)),
            UIStringInner::VarString(var) => {
                let guard = var.read()?;
                Some(UIStringRef::VarString(guard))
            }
            UIStringInner::VarStaticStr(var) => {
                let guard = var.read()?;
                // Reading registers dependencies; value is 'static so it can be borrowed directly.
                Some(UIStringRef::Borrowed(*guard))
            }
            UIStringInner::DeferredHeap(f) => Some(UIStringRef::OwnedString((f)(translation_map))),
            UIStringInner::DeferredStack(f) => Some(UIStringRef::OwnedStackString((f)()?)),
        }
    }
}

impl From<&'static str> for UIString {
    fn from(value: &'static str) -> Self {
        UIString(UIStringInner::Static(value))
    }
}

impl From<String> for UIString {
    fn from(value: String) -> Self {
        UIString(UIStringInner::Owned(value))
    }
}

impl From<WeakVar<&'static str>> for UIString {
    fn from(value: WeakVar<&'static str>) -> Self {
        UIString(UIStringInner::VarStaticStr(value))
    }
}

impl From<WeakVar<String>> for UIString {
    fn from(value: WeakVar<String>) -> Self {
        UIString(UIStringInner::VarString(value))
    }
}

impl From<LocalizedString> for UIString {
    fn from(value: LocalizedString) -> Self {
        UIString(UIStringInner::Localized(value))
    }
}

/// Sealed trait used to avoid overlapping `From<WeakVar<T>>` impls with the `String` and `&'static str` fast paths.
mod sealed {
    pub trait Sealed {}
}

/// Types that can be converted from `WeakVar<T>` into a formatted `UIString`
///
/// This exists to avoid overlap with `WeakVar<String>` and `WeakVar<&'static str>` conversions.
#[doc(hidden)]
pub trait UIVarDisplay: sealed::Sealed + fmt::Display + Send + Sync + 'static {}
impl<T> UIVarDisplay for T where T: sealed::Sealed + fmt::Display + Send + Sync + 'static {}

macro_rules! impl_ui_var_display {
    ($($ty:ty),* $(,)?) => {
        $(
            impl sealed::Sealed for $ty {}
        )*
    };
}

#[rustfmt::skip]
impl_ui_var_display!(
    f32, f64,
    bool, char,
    usize, isize,
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
);

/// Any `WeakVar<T>` where `T` is in `UIVarDisplay` becomes a fast deferred formatter.
///
/// This does one heap alloc when constructing the UIString, and no heap alloc on resolve in
/// the common case because `SmallString` stays inline for short strings.
impl<T> From<WeakVar<T>> for UIString
where
    T: UIVarDisplay,
{
    fn from(var: WeakVar<T>) -> Self {
        UIString::__deferred_stack(move || {
            let v = var.read()?;
            let mut buf = SmallString::<[u8; 64]>::new();
            let _ = write!(&mut buf, "{}", &*v);
            Some(buf)
        })
    }
}

/// A type that allows widgets to accept either dynamic or static parameters.
///
/// For example:
///
/// ```ignore
/// fn example(param: impl Into<UIParam<bool>>) { ... }
///
/// // `example` can be called with a constant
/// example(true);
///
/// // ... or with a reactive variable.
/// let my_var = Var::new(true);
/// example(*my_var);
/// ```
#[derive(Copy, Clone)]
pub enum UIParam<T: Send + Sync + 'static> {
    Static(T),
    Dynamic(WeakVar<T>),
}

impl<T: Send + Sync + 'static> From<T> for UIParam<T> {
    fn from(value: T) -> Self {
        UIParam::Static(value)
    }
}

impl<T: Send + Sync + 'static> From<WeakVar<T>> for UIParam<T> {
    fn from(var: WeakVar<T>) -> Self {
        UIParam::Dynamic(var)
    }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> UIParam<T> {
    pub fn with_mut<R>(&mut self, func: impl FnOnce(&mut T) -> R) -> Option<R> {
        match self {
            UIParam::Static(value) => Some(func(value)),
            UIParam::Dynamic(var) => {
                let mut guard = var.write()?;
                Some(func(&mut guard))
            }
        }
    }

    pub fn get(&self) -> Option<T> {
        match self {
            UIParam::Static(value) => Some(value.clone()),
            UIParam::Dynamic(var) => var.get(),
        }
    }

    pub fn get_or(&self, default: T) -> T {
        match self {
            UIParam::Static(value) => value.clone(),
            UIParam::Dynamic(var) => var.get().unwrap_or(default),
        }
    }
}
