use std::{cell::RefCell, fmt::Debug};

use bumpalo::{collections::Vec as BumpVec, Bump};

#[derive(Debug, Default)]
pub(crate) struct Alloc(RefCell<(bool, Bump)>);

impl Alloc {
    pub fn alloc<T>(&self, val: T) -> &'static mut T {
        let cell = self.0.borrow();
        if !cell.0 {
            panic!("[Rosin] Bump allocator used while disabled")
        };
        let ptr: *mut T = cell.1.alloc(val);
        unsafe { &mut *ptr }
    }

    pub fn vec<T>(&self) -> BumpVec<'static, T> {
        let cell = self.0.borrow();
        if !cell.0 {
            panic!("[Rosin] Bump allocator used while disabled")
        };
        let vec: BumpVec<T> = BumpVec::new_in(&cell.1);
        unsafe { std::mem::transmute(vec) }
    }

    pub fn vec_capacity<T>(&self, size: usize) -> BumpVec<'static, T> {
        let cell = self.0.borrow();
        if !cell.0 {
            panic!("[Rosin] Bump allocator used while disabled")
        };
        let vec: BumpVec<T> = BumpVec::with_capacity_in(size, &cell.1);
        unsafe { std::mem::transmute(vec) }
    }

    pub fn enable(&self) {
        self.0.borrow_mut().0 = true;
    }

    pub fn disable(&self) {
        self.0.borrow_mut().0 = false;
    }

    // SAFETY: All references to allocations must be dropped before calling `reset()`
    pub unsafe fn reset(&self) {
        self.0.borrow_mut().1.reset();
    }
}
