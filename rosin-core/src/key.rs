#![forbid(unsafe_code)]

use std::{
    num::NonZeroU32,
    sync::atomic::{AtomicU32, Ordering},
};

/// A unique identifier for a node.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Key(NonZeroU32);

impl Key {
    pub fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        Self(NonZeroU32::new(COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap())
    }
}

impl Default for Key {
    fn default() -> Self {
        Self::new()
    }
}
