use std::{
    cell::{Cell, RefCell},
    fmt,
    fmt::Debug,
    sync::Arc,
};

use bumpalo::{collections::Vec as BumpVec, Bump};

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
}

// Allows client code to use a custom allocator without passing an ugly handle around
#[derive(Default)]
pub(crate) struct Alloc {
    bump: RefCell<Bump>,
    enabled: Cell<bool>,
    token: RefCell<Arc<()>>,
}

impl Debug for Alloc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Alloc {{ enabled: {}, token: {} }}",
            self.enabled.get(),
            Arc::strong_count(&self.token.borrow())
        )
    }
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
        if Arc::strong_count(&self.token.borrow()) == 1 {
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
            token: self.token.borrow().clone(),
            value: func(),
        };
        self.enabled.replace(false);
        scope
    }
}

// An alternate API for FFI
#[cfg(all(debug_assertions, feature = "hot-reload"))]
impl Alloc {
    pub unsafe fn get_token(&self) -> Arc<()> {
        self.token.borrow().clone()
    }

    pub unsafe fn entangle(&self, token: Arc<()>) {
        if self.enabled.get() || Arc::strong_count(&self.token.borrow()) != 1 {
            panic!("[Rosin] Can't entangle an active allocator");
        }

        let ptr = Arc::into_raw(token);
        Arc::decrement_strong_count(ptr);
        self.token.replace(Arc::from_raw(ptr));
    }

    pub unsafe fn begin(&self) {
        self.enabled.replace(true);
    }

    pub unsafe fn end(&self) {
        self.enabled.replace(false);
    }
}
