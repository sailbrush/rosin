#![forbid(unsafe_code)]

/// Create a unique Key. Optionally accepts a discriminant to tell otherwise identical call-sites apart. (Such as in a loop)
#[macro_export]
macro_rules! new_key {
    () => {{
        let loc = std::panic::Location::caller();
        Key::default()
            .hash_djb2(file!().as_bytes())
            .hash_djb2(&(line!()).to_ne_bytes())
            .hash_djb2(&(column!()).to_ne_bytes())
            .hash_djb2(loc.file().as_bytes())
            .hash_djb2(&loc.line().to_ne_bytes())
            .hash_djb2(&loc.column().to_ne_bytes())
    }};
    ($discriminant:expr) => {
        // Not included in a hash round because the hash
        // may be expensive and should happen at compile time
        new_key!().add($discriminant as u64)
    };
}

/// A unique identifier for a node.
/// Create a Key with the `new_key!()` macro.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Key(u64);

#[doc(hidden)]
impl Default for Key {
    fn default() -> Self {
        Self(5381)
    }
}

impl Key {
    // TODO - use a better hash function once track_caller is const, 64 bits should be enough
    // No collisions between u32's under 8,192 so this should be fine for hashing line numbers
    #[doc(hidden)]
    pub const fn hash_djb2(mut self, input: &[u8]) -> Self {
        let mut i = 0;

        while i < input.len() {
            self.0 = (self.0 << 5).wrapping_add(self.0) ^ input[i] as u64;
            i += 1;
        }

        self
    }

    #[doc(hidden)]
    pub const fn add(mut self, val: u64) -> Self {
        self.0 = self.0.wrapping_add(val);
        self
    }
}
