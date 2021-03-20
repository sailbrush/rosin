#![forbid(unsafe_code)]

// TODO - make lenses composable so widgets can be combined

#[macro_export]
macro_rules! new_lens {
    ($input:ident $($path:tt)*) => {
        Lens::new(|a: &$input| { &a$($path)* }, |a: &mut $input| { &mut a$($path)* });
    }
}

pub struct Lens<A, B> {
    get: fn(&A) -> &B,
    get_mut: fn(&mut A) -> &mut B,
}

impl<A, B> Copy for Lens<A, B> {}
impl<A, B> Clone for Lens<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> std::fmt::Debug for Lens<A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lens<{}, {}>", std::any::type_name::<A>(), std::any::type_name::<B>())
    }
}

impl<A, B> Lens<A, B> {
    pub fn new(get: fn(&A) -> &B, get_mut: fn(&mut A) -> &mut B) -> Self {
        Self { get, get_mut }
    }

    pub fn get<'a>(&self, data: &'a A) -> &'a B {
        (self.get)(data)
    }

    pub fn get_mut<'a>(&self, data: &'a mut A) -> &'a mut B {
        (self.get_mut)(data)
    }
}
