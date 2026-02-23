#[allow(unused)]
#[cfg(not(loom))]
mod internal {
    pub use std::sync::Arc;
    pub use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    pub use std::thread::{self, ThreadId};

    pub use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
}

#[allow(unused)]
#[cfg(loom)]
mod internal {
    use std::marker::PhantomData;
    use std::ops::{Deref, DerefMut};

    pub use loom::sync::Arc;
    pub use loom::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    pub use loom::thread::{self, ThreadId};

    pub struct Mutex<T>(loom::sync::Mutex<T>);
    pub struct MutexGuard<'a, T>(loom::sync::MutexGuard<'a, T>);

    impl<T> Mutex<T> {
        pub fn new(val: T) -> Self {
            Self(loom::sync::Mutex::new(val))
        }

        pub fn lock(&self) -> MutexGuard<'_, T> {
            MutexGuard(self.0.lock().unwrap()) // Unwrap ok: this is just for testing so the api matches parking_lot's
        }

        pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
            self.0.try_lock().ok().map(MutexGuard)
        }
    }

    impl<'a, T> Deref for MutexGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a, T> DerefMut for MutexGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    #[derive(Debug)]
    pub struct RwLock<T>(loom::sync::RwLock<T>);

    pub struct RwLockReadGuard<'a, T>(loom::sync::RwLockReadGuard<'a, T>);
    pub struct RwLockWriteGuard<'a, T>(loom::sync::RwLockWriteGuard<'a, T>);

    trait HeldLock<'a> {}

    impl<'a, T> HeldLock<'a> for RwLockReadGuard<'a, T> {}
    impl<'a, T> HeldLock<'a> for RwLockWriteGuard<'a, T> {}

    pub struct MappedRwLockReadGuard<'a, T: ?Sized> {
        _guard: Box<dyn HeldLock<'a> + 'a>,
        data: *const T,
        marker: PhantomData<&'a T>,
    }

    impl<'a, T: ?Sized> MappedRwLockReadGuard<'a, T> {
        pub fn map<U: ?Sized, F>(self, f: F) -> MappedRwLockReadGuard<'a, U>
        where
            F: FnOnce(&T) -> &U,
        {
            let data_ref: &T = unsafe { &*self.data }; // SAFETY: This is only used in test code.
            let new_ref: &U = f(data_ref);
            let new_ptr = new_ref as *const U;

            MappedRwLockReadGuard {
                _guard: self._guard,
                data: new_ptr,
                marker: PhantomData,
            }
        }

        pub fn try_map<U: ?Sized, F>(self, f: F) -> Result<MappedRwLockReadGuard<'a, U>, Self>
        where
            F: FnOnce(&T) -> Option<&U>,
        {
            let data_ref: &T = unsafe { &*self.data }; // SAFETY: This is only used in test code.

            if let Some(new_ref) = f(data_ref) {
                let new_ptr = new_ref as *const U;
                Ok(MappedRwLockReadGuard {
                    _guard: self._guard,
                    data: new_ptr,
                    marker: PhantomData,
                })
            } else {
                Err(self)
            }
        }
    }

    pub struct MappedRwLockWriteGuard<'a, T: ?Sized> {
        _guard: Box<dyn HeldLock<'a> + 'a>,
        data: *mut T,
        marker: PhantomData<&'a mut T>,
    }

    impl<'a, T: ?Sized> MappedRwLockWriteGuard<'a, T> {
        pub fn map<U: ?Sized, F>(mut self, f: F) -> MappedRwLockWriteGuard<'a, U>
        where
            F: FnOnce(&mut T) -> &mut U,
        {
            let data_ref: &mut T = unsafe { &mut *self.data }; // SAFETY: This is only used in test code.
            let new_ref: &mut U = f(data_ref);
            let new_ptr = new_ref as *mut U;

            MappedRwLockWriteGuard {
                _guard: self._guard,
                data: new_ptr,
                marker: PhantomData,
            }
        }

        pub fn try_map<U: ?Sized, F>(mut self, f: F) -> Result<MappedRwLockWriteGuard<'a, U>, Self>
        where
            F: FnOnce(&mut T) -> Option<&mut U>,
        {
            let data_ref: &mut T = unsafe { &mut *self.data }; // SAFETY: This is only used in test code.

            if let Some(new_ref) = f(data_ref) {
                let new_ptr = new_ref as *mut U;
                Ok(MappedRwLockWriteGuard {
                    _guard: self._guard,
                    data: new_ptr,
                    marker: PhantomData,
                })
            } else {
                Err(self)
            }
        }
    }

    unsafe impl<'a, T: ?Sized + Sync> Sync for MappedRwLockReadGuard<'a, T> {} // SAFETY: this is just for testing so the api matches parking_lot's
    unsafe impl<'a, T: ?Sized + Send + Sync> Send for MappedRwLockReadGuard<'a, T> {} // SAFETY: this is just for testing so the api matches parking_lot's

    unsafe impl<'a, T: ?Sized + Sync> Sync for MappedRwLockWriteGuard<'a, T> {} // SAFETY: this is just for testing so the api matches parking_lot's
    unsafe impl<'a, T: ?Sized + Send> Send for MappedRwLockWriteGuard<'a, T> {} // SAFETY: this is just for testing so the api matches parking_lot's

    impl<T> RwLock<T> {
        pub fn new(val: T) -> Self {
            Self(loom::sync::RwLock::new(val))
        }

        pub fn read(&self) -> RwLockReadGuard<'_, T> {
            RwLockReadGuard(self.0.read().unwrap()) // Unwrap ok: this is just for testing so the api matches parking_lot's
        }

        pub fn write(&self) -> RwLockWriteGuard<'_, T> {
            RwLockWriteGuard(self.0.write().unwrap()) // Unwrap ok: this is just for testing so the api matches parking_lot's
        }

        pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
            self.0.try_read().ok().map(RwLockReadGuard)
        }

        pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
            self.0.try_write().ok().map(RwLockWriteGuard)
        }
    }

    impl<'a, T> Deref for RwLockReadGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a, T> RwLockReadGuard<'a, T> {
        pub fn try_map<U: ?Sized, F>(self, f: F) -> Result<MappedRwLockReadGuard<'a, U>, Self>
        where
            F: FnOnce(&T) -> Option<&U>,
        {
            let data_ptr = &*self as *const T;
            let data_ref = unsafe { &*data_ptr }; // SAFETY: This is only used in test code.

            if let Some(new_ref) = f(data_ref) {
                let new_ptr = new_ref as *const U;
                Ok(MappedRwLockReadGuard {
                    _guard: Box::new(self), // Coerces to Box<dyn HeldLock<'a> + 'a>
                    data: new_ptr,
                    marker: PhantomData,
                })
            } else {
                Err(self)
            }
        }
    }

    impl<'a, T: ?Sized> Deref for MappedRwLockReadGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            unsafe { &*self.data } // SAFETY: This is only used in test code.
        }
    }

    impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<'a, T> RwLockWriteGuard<'a, T> {
        pub fn try_map<U: ?Sized, F>(mut self, f: F) -> Result<MappedRwLockWriteGuard<'a, U>, Self>
        where
            F: FnOnce(&mut T) -> Option<&mut U>,
        {
            let data_ptr = &mut *self as *mut T;
            let data_ref = unsafe { &mut *data_ptr }; // SAFETY: This is only used in test code.

            if let Some(new_ref) = f(data_ref) {
                let new_ptr = new_ref as *mut U;
                Ok(MappedRwLockWriteGuard {
                    _guard: Box::new(self),
                    data: new_ptr,
                    marker: PhantomData,
                })
            } else {
                Err(self)
            }
        }
    }

    impl<'a, T: ?Sized> Deref for MappedRwLockWriteGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            unsafe { &*self.data } // SAFETY: This is only used in test code.
        }
    }

    impl<'a, T: ?Sized> DerefMut for MappedRwLockWriteGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { &mut *self.data } // SAFETY: This is only used in test code.
        }
    }
}

pub use self::internal::*;
