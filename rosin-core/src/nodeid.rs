//! A stable and unique node identifier.

use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

/// A unique identifier for a node.
///
/// In view callbacks, create NodeIds with the [`crate::id`] macro.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct NodeId(pub(crate) NonZeroU64);

impl From<NodeId> for u64 {
    fn from(id: NodeId) -> u64 {
        id.0.get()
    }
}

impl From<NodeId> for accesskit::NodeId {
    fn from(value: NodeId) -> Self {
        Self(value.0.get())
    }
}

/// The preferred method for creating a [`NodeId`].
///
/// Generates an id based on the call location, so repeated calls to the same view function will create nodes with stable ids.
/// You can optionally pass a comma separated list of anything that implements [`Into<u64>`] to make it unique,
/// such as the id of a parent node, or the position in a list, which is useful when writing reusable widgets.
#[macro_export]
macro_rules! id {
    () => {
        const { NodeId::__internal_new(file!(), line!(), column!()) }
    };
    ($($x:expr),+ $(,)?) => {{
        let mut id = const { NodeId::__internal_new(file!(), line!(), column!()) };
        $(
            id = id.__internal_mix(($x).into());
        )+
        id
    }};
}

impl Default for NodeId {
    fn default() -> Self {
        Self::next()
    }
}

impl NodeId {
    pub(crate) const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    pub(crate) const FNV_PRIME: u64 = 0x100000001b3;

    #[doc(hidden)]
    pub const fn __internal_new(file: &'static str, line: u32, column: u32) -> Self {
        const fn fnv1a_64(seed: Option<u64>, bytes: &[u8]) -> u64 {
            let mut hash = if let Some(seed) = seed { seed } else { NodeId::FNV_OFFSET_BASIS };

            let mut i = 0;
            while i < bytes.len() {
                hash ^= bytes[i] as u64;
                hash = hash.wrapping_mul(NodeId::FNV_PRIME);
                i += 1;
            }

            hash
        }

        let h1 = fnv1a_64(None, file.as_bytes());
        let h2 = fnv1a_64(Some(h1), &line.to_le_bytes());
        let h3 = fnv1a_64(Some(h2), &column.to_le_bytes());

        let non_zero = match NonZeroU64::new(h3) {
            Some(val) => val,
            None => NonZeroU64::MAX,
        };

        NodeId(non_zero)
    }

    #[doc(hidden)]
    pub const fn __internal_mix(self, rhs: u64) -> Self {
        let mixed = (self.0.get() ^ rhs).wrapping_mul(NodeId::FNV_PRIME);
        let non_zero = match NonZeroU64::new(mixed) {
            Some(val) => val,
            None => NonZeroU64::MAX,
        };
        NodeId(non_zero)
    }

    /// Create a new [`NodeId`]. For times when the `id!()` macro isn't appropriate, such as when creating ids in a loop.
    ///
    /// Not suitable for use in a view callback since it returns a unique id every time.
    pub fn next() -> Self {
        // Relaxed is sufficient since we only care about uniqueness.
        Self(NonZeroU64::new(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed)).unwrap_or(NonZeroU64::MAX))
    }

    /// Resume [`NodeId`] generation from a specific value.
    /// This is useful in the exceptionally rare cases when resuming id creation
    /// from a particular value with the [`NodeId::next()`] function is required.
    ///
    /// **Panics**: if `value` is zero, as [`NodeId`] values must be non-zero.
    pub fn set_counter(value: u64) {
        assert!(value != 0, "NodeId counter cannot start at zero");
        NEXT_NODE_ID.store(value, Ordering::SeqCst);
    }

    /// Returns the current value of the [`NodeId`] generator.
    /// Useful for persisting the id counter to resume later with [`NodeId::set_counter()`].
    pub fn get_counter() -> u64 {
        NEXT_NODE_ID.load(Ordering::SeqCst)
    }
}
