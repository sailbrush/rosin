use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    sync::Arc,
};

use bumpalo::{collections::Vec as BumpVec, Bump};

#[cfg(all(debug_assertions, feature = "hot-reload"))]
pub(crate) type ScopeToken = Arc<()>;

#[derive(Clone)]
pub(crate) struct Scope<T> {
    token: Arc<()>,
    value: T,
}

impl<T> Scope<T> {
    pub fn borrow(&self) -> &T {
        &self.value
    }

    pub fn borrow_mut(&mut self) -> &mut T {
        &mut self.value
    }

    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    pub fn bind(token: Arc<()>, value: T) -> Scope<T> {
        Self { token, value }
    }
}

// Allows client code to use a custom allocator without passing an ugly handle around
#[derive(Debug, Default)]
pub(crate) struct Alloc {
    bump: RefCell<Bump>,
    enabled: Cell<bool>,
    token: Arc<()>,
}

impl Alloc {
    pub fn alloc<T>(&self, val: T) -> &'static mut T {
        assert!(self.enabled.get(), "[Rosin] Thread local allocator used outside of a scope");
        let ptr: *mut T = self.bump.borrow().alloc(val);
        unsafe { &mut *ptr }
    }

    pub fn vec<T>(&self) -> BumpVec<'static, T> {
        assert!(self.enabled.get(), "[Rosin] Thread local allocator used outside of a scope");
        let bump = self.bump.borrow();
        let vec: BumpVec<T> = BumpVec::new_in(&bump);
        unsafe { std::mem::transmute(vec) }
    }

    pub fn vec_capacity<T>(&self, size: usize) -> BumpVec<'static, T> {
        assert!(self.enabled.get(), "[Rosin] Thread local allocator used outside of a scope");
        let bump = self.bump.borrow();
        let vec: BumpVec<T> = BumpVec::with_capacity_in(size, &bump);
        unsafe { std::mem::transmute(vec) }
    }

    pub fn reset(&self) -> Result<(), ()> {
        if Arc::strong_count(&self.token) == 1 {
            self.bump.borrow_mut().reset();
            Ok(())
        } else {
            Err(())
        }
    }

    // SAFETY: Ensure that all allocations made within a scope are
    //         exclusively owned by T to prevent dangling pointers
    pub unsafe fn scope<T>(&self, func: impl FnOnce() -> T) -> Scope<T> {
        self.enabled.replace(true);
        let scope = Scope {
            token: self.token.clone(),
            value: func(),
        };
        self.enabled.replace(false);
        scope
    }
}

// An alternate API suitable for FFI
#[cfg(all(debug_assertions, feature = "hot-reload"))]
impl Alloc {
    pub unsafe fn begin(&self) {
        self.enabled.replace(true);
    }

    pub unsafe fn end(&self) {
        self.enabled.replace(false);
    }

    pub unsafe fn new_scope_token(&self) -> ScopeToken {
        self.token.clone()
    }
}
