use std::{cell::RefCell, fmt::Debug};

use bumpalo::{collections::Vec as BumpVec, Bump};

#[derive(Debug, Default)]
pub(crate) struct Alloc(RefCell<Bump>);

impl Alloc {
    pub fn alloc<T>(&self, val: T) -> &'static mut T {
        let ptr: *mut T = self.0.borrow().alloc(val);
        unsafe { &mut *ptr }
    }

    pub fn vec<T>(&self) -> BumpVec<'static, T> {
        let bump = self.0.borrow();
        let vec: BumpVec<T> = BumpVec::new_in(&bump);
        unsafe { std::mem::transmute(vec) }
    }

    pub fn vec_capacity<T>(&self, size: usize) -> BumpVec<'static, T> {
        let bump = self.0.borrow();
        let vec: BumpVec<T> = BumpVec::with_capacity_in(size, &bump);
        unsafe { std::mem::transmute(vec) }
    }

    /// SAFETY: All references to allocations must be out of scope before it is safe to call `reset()`
    pub unsafe fn reset(&self) {
        self.0.borrow_mut().reset();
    }
}
