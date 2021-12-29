use std::{marker::PhantomData, ops::Deref, ptr::NonNull, sync::Mutex};

lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
}

#[derive(Debug)]
struct Count {
    pub generation: usize,
    pub strong_count: usize,
}

#[derive(Debug)]
struct Registry {
    pub alive: Vec<Count>,
    pub dead: Vec<usize>,
}

// TODO: Benchmark an alternate implementation based on Tinyset, or a Hashset with FxHashMap
//       Will probably need to store strong_count inside the allocation
//       I don't think it will need a generation counter since each item in the set will be a sequential integer
//       Also, put this behind an unstable feature flag and use the CoerceUnsized trait to keep the user from needing to box items
impl Registry {
    pub const fn new() -> Self {
        Self {
            alive: Vec::new(),
            dead: Vec::new(),
        }
    }

    pub(crate) fn register(&mut self) -> (usize, usize) {
        if let Some(index) = self.dead.pop() {
            self.alive[index].strong_count = 1;
            (index, self.alive[index].generation)
        } else {
            let generation = 0;
            let index = self.alive.len();
            self.alive.push(Count {
                generation,
                strong_count: 1,
            });
            (index, generation)
        }
    }
}

#[derive(Debug)]
pub struct Strong<T: ?Sized> {
    index: usize,
    generation: usize,
    data: NonNull<T>,
    phantom: PhantomData<T>,
}

// Let Box do the work of coercing unsized types until CoerceUnsized stabilizes
// https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html
impl<T: ?Sized> Strong<T> {
    pub fn new(init: Box<T>) -> Self {
        let (index, generation) = REGISTRY.lock().unwrap().register();
        Self {
            index,
            generation,
            data: NonNull::new(Box::into_raw(init)).unwrap(),
            phantom: PhantomData,
        }
    }

    pub fn downgrade(this: &Self) -> Weak<T> {
        Weak {
            index: this.index,
            generation: this.generation,
            data: this.data,
        }
    }
}

impl<T: ?Sized> Clone for Strong<T> {
    fn clone(&self) -> Self {
        let mut reg = REGISTRY.lock().unwrap();
        reg.alive[self.index].strong_count += 1;

        Self {
            index: self.index,
            generation: self.generation,
            data: self.data,
            phantom: self.phantom,
        }
    }
}

impl<T: ?Sized> Drop for Strong<T> {
    fn drop(&mut self) {
        let mut reg = REGISTRY.lock().unwrap();
        reg.alive[self.index].strong_count -= 1;
        if reg.alive[self.index].strong_count == 0 {
            reg.alive[self.index].generation += 1;
            reg.dead.push(self.index);
            unsafe {
                Box::from_raw(self.data.as_ptr());
            }
        }
    }
}

impl<T: ?Sized> Deref for Strong<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.data.as_ref() }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Weak<T: ?Sized> {
    index: usize,
    generation: usize,
    data: NonNull<T>,
}

impl<T: ?Sized> Weak<T> {
    pub fn upgrade(&self) -> Option<Strong<T>> {
        let mut reg = REGISTRY.lock().unwrap();
        if reg.alive[self.index].generation == self.generation {
            reg.alive[self.index].strong_count += 1;
            Some(Strong {
                index: self.index,
                generation: self.generation,
                data: self.data,
                phantom: PhantomData,
            })
        } else {
            None
        }
    }
}
