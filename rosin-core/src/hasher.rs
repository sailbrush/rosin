use std::hash::{BuildHasher, Hasher};

#[derive(Clone, Default)]
pub(crate) struct IdentityBuildHasher;

pub(crate) struct IdentityHasher(u64);

impl Default for IdentityHasher {
    #[inline]
    fn default() -> Self {
        Self(0)
    }
}

impl Hasher for IdentityHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.0 = i as u64;
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.0 = i as u64;
    }

    // Fallback to FNV-1a 64-bit. This should never be called.
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        debug_assert!(false);

        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET_BASIS;
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        self.0 = hash;
    }
}

impl BuildHasher for IdentityBuildHasher {
    type Hasher = IdentityHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        IdentityHasher::default()
    }
}
