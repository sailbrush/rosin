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

// TODO: Benchmark an alternate implementation based on Tinyset, a Hashset with FxHashMap, or a BTreeSet
//       Also lazy_static vs thread_local in high and low contention situations
//       Will probably need to store strong_count inside the allocation
//       I don't think it will need a generation counter since each item in the set will be a sequential integer
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
pub struct Grc<T> {
    index: usize,
    generation: usize,
    data: NonNull<T>,
    phantom: PhantomData<T>,
}

// Waiting until CoerceUnsized stabilizes
// https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html
impl<T> Grc<T> {
    pub fn new(init: T) -> Self {
        let (index, generation) = REGISTRY.lock().unwrap().register();

        Self {
            index,
            generation,
            data: NonNull::new(Box::into_raw(Box::new(init))).unwrap(),
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

impl<T> Clone for Grc<T> {
    fn clone(&self) -> Self {
        let mut reg = REGISTRY.lock().unwrap();
        reg.alive[self.index].strong_count += 1;
        drop(reg);

        Self {
            index: self.index,
            generation: self.generation,
            data: self.data,
            phantom: self.phantom,
        }
    }
}

impl<T> Drop for Grc<T> {
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

impl<T> Deref for Grc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.data.as_ref() }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Weak<T> {
    index: usize,
    generation: usize,
    data: NonNull<T>,
}

impl<T> Weak<T> {
    pub fn upgrade(&self) -> Option<Grc<T>> {
        let mut reg = REGISTRY.lock().unwrap();
        if reg.alive[self.index].generation == self.generation {
            reg.alive[self.index].strong_count += 1;
            Some(Grc {
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
