#![forbid(unsafe_code)]

use std::{any, fmt};

#[macro_export]
macro_rules! lens {
    ($obj_type:ident -> $($path:tt)*) => {
        Lens::new(|a: &$obj_type| { &a.$($path)* }, |a: &mut $obj_type| { &mut a.$($path)* });
    }
}

// ---------- Trait ----------
pub trait Lensable {
    type In;
    type Out;

    fn get<'a>(&self, obj: &'a <Self as Lensable>::In) -> &'a <Self as Lensable>::Out;
    fn get_mut<'a>(&self, obj: &'a mut <Self as Lensable>::In) -> &'a mut <Self as Lensable>::Out;
}

// ---------- Lens ----------
pub struct Lens<A, B> {
    get: fn(&A) -> &B,
    get_mut: fn(&mut A) -> &mut B,
}

impl<A, B> Lens<A, B> {
    pub fn new(get: fn(&A) -> &B, get_mut: fn(&mut A) -> &mut B) -> Self {
        Self { get, get_mut }
    }
}

impl<A, B> Copy for Lens<A, B> {}
impl<A, B> Clone for Lens<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> fmt::Debug for Lens<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lens {{ {} -> {} }}", any::type_name::<A>(), any::type_name::<B>())
    }
}

impl<A, B> fmt::Display for Lens<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", any::type_name::<A>())
    }
}

impl<A, B> Lensable for Lens<A, B> {
    type In = A;
    type Out = B;

    fn get<'a>(&self, obj: &'a A) -> &'a B {
        (self.get)(obj)
    }

    fn get_mut<'a>(&self, obj: &'a mut A) -> &'a mut B {
        (self.get_mut)(obj)
    }
}

// ---------- CompoundLens ----------
pub struct CompoundLens<X, Y> {
    pub lhs: X,
    pub rhs: Y,
}

impl<X, Y> CompoundLens<X, Y> {
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

impl<'a, X, Y> fmt::Debug for CompoundLens<X, Y>
where
    X: Lensable + fmt::Display,
    Y: Lensable + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompoundLens {{ {} -> {} -> {} }}",
            self.lhs,
            self.rhs,
            any::type_name::<Y::Out>()
        )
    }
}

impl<X, Y> fmt::Display for CompoundLens<X, Y>
where
    X: Lensable,
    Y: Lensable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", any::type_name::<X::In>(), any::type_name::<X::Out>())
    }
}

impl<A, B, C, X, Y> Lensable for CompoundLens<X, Y>
where
    X: Lensable<In = A, Out = B>,
    Y: Lensable<In = B, Out = C>,
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
}

impl<A, B> From<Lens<A, B>> for CompoundLens<Lens<A, B>, Lens<B, B>> {
    fn from(lens: Lens<A, B>) -> CompoundLens<Lens<A, B>, Lens<B, B>> {
        CompoundLens {
            lhs: lens,
            rhs: Lens::new(|obj: &B| obj, |obj: &mut B| obj),
        }
    }
}
