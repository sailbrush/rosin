use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use bumpalo::{collections::Vec as BumpVec, Bump};

thread_local!(static ALLOC: RefCell<Option<Rc<Alloc>>> = RefCell::new(Some(Rc::new(Alloc::default()))));

#[derive(Clone)]
pub(crate) struct Scope<T> {
    value: T,
    _token: Rc<()>,
}

impl<T> Scope<T> {
    pub fn borrow(&self) -> &T {
        &self.value
    }

    pub fn borrow_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

#[derive(Default, Debug)]
pub struct Alloc {
    bump: RefCell<Bump>,
    enabled: Cell<bool>,
    counter: Cell<usize>,
    _token: Rc<()>,
}

impl Alloc {
    #[no_mangle]
    pub(crate) fn set_thread_local_alloc(alloc: Option<Rc<Alloc>>) {
        ALLOC.with(|a| a.replace(alloc));
    }

    pub(crate) fn get_thread_local_alloc() -> Option<Rc<Alloc>> {
        ALLOC.with(|a| a.borrow().clone())
    }

    pub(crate) fn reset_counter(&self) {
        self.counter.set(0);
    }

    pub(crate) fn increment_counter(&self) {
        self.counter.set(self.counter.get() + 1);
    }

    pub(crate) fn get_counter(&self) -> usize {
        self.counter.get()
    }

    pub(crate) fn alloc<T>(&self, val: T) -> &'static mut T {
        assert!(self.enabled.get(), "[Rosin] Allocator used outside of a scope");
        let ptr: *mut T = self.bump.borrow().alloc(val);
        unsafe { &mut *ptr }
    }

    pub(crate) fn vec<T>(&self) -> BumpVec<'static, T> {
        assert!(self.enabled.get(), "[Rosin] Allocator used outside of a scope");
        let bump = self.bump.borrow();
        let vec: BumpVec<T> = BumpVec::new_in(&bump);
        unsafe { std::mem::transmute(vec) }
    }

    pub(crate) fn vec_capacity<T>(&self, size: usize) -> BumpVec<'static, T> {
        assert!(self.enabled.get(), "[Rosin] Allocator used outside of a scope");
        let bump = self.bump.borrow();
        let vec: BumpVec<T> = BumpVec::with_capacity_in(size, &bump);
        unsafe { std::mem::transmute(vec) }
    }

    pub(crate) fn reset(&self) -> Result<(), ()> {
        if Rc::strong_count(&self._token) == 1 {
            self.bump.borrow_mut().reset();
            Ok(())
        } else {
            Err(())
        }
    }

    // SAFETY: Ensure that all allocations made within a scope are
    //         exclusively owned by T to prevent dangling pointers
    pub(crate) unsafe fn scope<T>(&self, func: impl FnOnce() -> T) -> Scope<T> {
        self.enabled.set(true);
        let scope = Scope {
            _token: self._token.clone(),
            value: func(),
        };
        self.enabled.set(false);
        scope
    }
}
