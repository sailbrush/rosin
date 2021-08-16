#![forbid(unsafe_code)]

use std::{
    any, fmt,
    fmt::{Debug, Display},
};

#[macro_export]
macro_rules! lens {
    ($obj_type:ty => $($path:tt)*) => {
        SingleLens::new(|state: &$obj_type| { &state.$($path)* }, |state: &mut $obj_type| { &mut state.$($path)* })
    };
    ($first_lens:expr, $obj_type:ty => $($path:tt)*) => {
        CompoundLens::new($first_lens, SingleLens::new(|state: &$obj_type| { &state.$($path)* }, |state: &mut $obj_type| { &mut state.$($path)* }))
    };
}

// ---------- Trait ----------
pub trait Lens: Debug {
    type In;
    type Out;

    fn get<'a>(&self, obj: &'a Self::In) -> &'a Self::Out;
    fn get_mut<'a>(&self, obj: &'a mut Self::In) -> &'a mut Self::Out;
    fn leak(self) -> &'static dyn Lens<In = Self::In, Out = Self::Out>;
}

// ---------- SingleLens ----------
pub struct SingleLens<A, B> {
    get: fn(&A) -> &B,
    get_mut: fn(&mut A) -> &mut B,
}

impl<A, B> SingleLens<A, B> {
    pub fn new(get: fn(&A) -> &B, get_mut: fn(&mut A) -> &mut B) -> Self {
        Self { get, get_mut }
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
        write!(f, "Lens({} -> {})", any::type_name::<A>(), any::type_name::<B>())
    }
}

impl<A, B> Display for SingleLens<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", any::type_name::<A>())
    }
}

impl<A, B> Lens for SingleLens<A, B> {
    type In = A;
    type Out = B;

    fn get<'a>(&self, obj: &'a A) -> &'a B {
        (self.get)(obj)
    }

    fn get_mut<'a>(&self, obj: &'a mut A) -> &'a mut B {
        (self.get_mut)(obj)
    }

    fn leak(self) -> &'static dyn Lens<In = A, Out = B> {
        Box::leak(Box::new(self))
    }
}

// ---------- CompoundLens ----------
pub struct CompoundLens<X, Y> {
    pub lhs: X,
    pub rhs: Y,
}

impl<X, Y> CompoundLens<X, Y>
where
    X: Lens,
    Y: Lens,
{
    pub fn new(lhs: X, rhs: Y) -> Self {
        CompoundLens { lhs, rhs }
    }
}

impl<X: Copy, Y: Copy> Copy for CompoundLens<X, Y> {}
impl<X: Copy, Y: Copy> Clone for CompoundLens<X, Y> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, X, Y> Debug for CompoundLens<X, Y>
where
    X: Lens + Display,
    Y: Lens + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompoundLens({} -> {} -> {})", self.lhs, self.rhs, any::type_name::<Y::Out>())
    }
}

impl<X, Y> Display for CompoundLens<X, Y>
where
    X: Lens,
    Y: Lens,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", any::type_name::<X::In>(), any::type_name::<X::Out>())
    }
}

impl<A, B, C, X, Y> Lens for CompoundLens<X, Y>
where
    X: 'static + Lens<In = A, Out = B> + Debug + Display,
    Y: 'static + Lens<In = B, Out = C> + Debug + Display,
    B: 'static,
{
    type In = X::In;
    type Out = Y::Out;

    fn get<'a>(&self, obj: &'a Self::In) -> &'a Self::Out {
        self.rhs.get(self.lhs.get(obj))
    }

    fn get_mut<'a>(&self, obj: &'a mut Self::In) -> &'a mut Self::Out {
        self.rhs.get_mut(self.lhs.get_mut(obj))
    }

    fn leak(self) -> &'static dyn Lens<In = A, Out = C> {
        Box::leak(Box::new(self))
    }
}
