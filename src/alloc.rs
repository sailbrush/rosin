use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use bumpalo::{collections::Vec as BumpVec, Bump};

#[derive(Clone)]
pub(crate) struct Scope<T> {
    value: Arc<RwLock<T>>,
    count: Arc<()>,
}

impl<T> Scope<T> {
    pub fn read(&self) -> LockResult<RwLockReadGuard<T>> {
        self.value.read()
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<T>> {
        self.value.write()
    }
}

// Allows client code to use a custom allocator without passing an ugly handle around
#[derive(Debug, Default)]
pub(crate) struct Alloc {
    bump: RefCell<Bump>,
    enabled: Cell<bool>,
    count: Arc<()>,
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
        if Arc::strong_count(&self.count) == 1 {
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
            value: Arc::new(RwLock::new(func())),
            count: self.count.clone(),
        };
        self.enabled.replace(false);
        scope
    }
}
