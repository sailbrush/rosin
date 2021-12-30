#![forbid(unsafe_code)]

use std::{
    any, fmt,
    fmt::{Debug, Display},
    marker::PhantomData,
};

/// Create a new Lens.
#[macro_export]
macro_rules! lens {
    ($obj_type:ty => $($path:tt)*) => {
        SingleLens::new(|obj: &$obj_type| { &obj.$($path)* }, |obj: &mut $obj_type| { &mut obj.$($path)* })
    };
    ($first_lens:expr, $obj_type:ty => $($path:tt)*) => {
        CompoundLens::new($first_lens, lens!($obj_type => $($path)*))
    };
}

// ---------- Trait ----------
/// A datatype that returns a reference to an internal component of another type. Intended to be used by widgets.
pub trait Lens<A, B>: Debug {
    fn get_ref<'a>(&self, obj: &'a A) -> &'a B;
    fn get_mut<'a>(&self, obj: &'a mut A) -> &'a mut B;
}

// ---------- SingleLens ----------
#[doc(hidden)]
pub struct SingleLens<A, B> {
    get_ref: fn(&A) -> &B,
    get_mut: fn(&mut A) -> &mut B,
}

impl<A, B> SingleLens<A, B> {
    pub fn new(get_ref: fn(&A) -> &B, get_mut: fn(&mut A) -> &mut B) -> Self {
        Self { get_ref, get_mut }
    }
}

impl<A, B> Copy for SingleLens<A, B> {}
impl<A, B> Clone for SingleLens<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> Debug for SingleLens<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lens({} => {})", any::type_name::<A>(), any::type_name::<B>())
    }
}

impl<A, B> Display for SingleLens<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", any::type_name::<A>())
    }
}

impl<A, B> Lens<A, B> for SingleLens<A, B> {
    fn get_ref<'a>(&self, obj: &'a A) -> &'a B {
        (self.get_ref)(obj)
    }

    fn get_mut<'a>(&self, obj: &'a mut A) -> &'a mut B {
        (self.get_mut)(obj)
    }
}

// ---------- CompoundLens ----------
#[doc(hidden)]
pub struct CompoundLens<A, B, C, X, Y> {
    pub lhs: X,
    pub rhs: Y,
    _a: PhantomData<*const A>,
    _b: PhantomData<*const B>,
    _c: PhantomData<*const C>,
}

impl<A, B, C, X, Y> CompoundLens<A, B, C, X, Y>
where
    X: Lens<A, B>,
    Y: Lens<B, C>,
{
    pub fn new(lhs: X, rhs: Y) -> Self {
        CompoundLens {
            lhs,
            rhs,
            _a: PhantomData,
            _b: PhantomData,
            _c: PhantomData,
        }
    }
}

impl<A, B, C, X: Copy, Y: Copy> Copy for CompoundLens<A, B, C, X, Y> {}
impl<A, B, C, X: Copy, Y: Copy> Clone for CompoundLens<A, B, C, X, Y> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B, C, X, Y> Debug for CompoundLens<A, B, C, X, Y>
where
    X: Lens<A, B> + Display,
    Y: Lens<B, C> + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lens({} => {} => {})", self.lhs, self.rhs, any::type_name::<C>())
    }
}

impl<A, B, C, X, Y> Display for CompoundLens<A, B, C, X, Y>
where
    X: Lens<A, B>,
    Y: Lens<B, C>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} => {}", any::type_name::<A>(), any::type_name::<B>())
    }
}

impl<A, B, C, X, Y> Lens<A, C> for CompoundLens<A, B, C, X, Y>
where
    X: Lens<A, B> + Debug + Display,
    Y: Lens<B, C> + Debug + Display,
    B: 'static,
{
    fn get_ref<'a>(&self, obj: &'a A) -> &'a C {
        self.rhs.get_ref(self.lhs.get_ref(obj))
    }

    fn get_mut<'a>(&self, obj: &'a mut A) -> &'a mut C {
        self.rhs.get_mut(self.lhs.get_mut(obj))
    }
}
