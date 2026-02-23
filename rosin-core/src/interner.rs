//! String interner used by the CSS system.
//!
//! This is public because the `hot-reload` feature needs to be able to share an interner with a loaded dynamic library.

use std::fmt::{self, Display};
use std::sync::OnceLock;
use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

// This is stored in an Arc so that it can be shared across shared library boundaries.
static STRING_INTERNER: OnceLock<Arc<RwLock<StringInterner>>> = OnceLock::new();

/// An identifier for an interned string.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrId(pub(crate) u32);

impl Display for StrId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = StringInterner::global().read().get(*self).ok_or(fmt::Error)?;
        f.write_str(string.as_ref())
    }
}

/// Interns strings into small integer ids
#[derive(Debug, Default)]
pub struct StringInterner {
    map: HashMap<Arc<str>, StrId>,
    rev: Vec<Arc<str>>,
}

impl StringInterner {
    /// Creates a new string interner.
    pub fn new() -> Self {
        Self {
            map: HashMap::with_capacity(128),
            rev: Vec::with_capacity(128),
        }
    }

    /// Returns the global string interner, initializing it if needed.
    pub fn global() -> &'static Arc<RwLock<StringInterner>> {
        STRING_INTERNER.get_or_init(|| Arc::new(RwLock::new(StringInterner::new())))
    }

    /// Initialize the global string interner, if it isn't already.
    pub fn set_global(interner: Arc<RwLock<StringInterner>>) -> bool {
        STRING_INTERNER.set(interner).is_ok()
    }

    /// Returns an existing id or assigns a new one.
    pub fn intern(&mut self, name: &str) -> StrId {
        if let Some(&id) = self.map.get(name) {
            return id;
        }

        let id = StrId(self.rev.len() as u32);

        let arc: Arc<str> = Arc::from(name);
        self.rev.push(Arc::clone(&arc));
        self.map.insert(arc, id);

        id
    }

    /// Get the interned string for an id, if it exists.
    pub fn get(&self, id: StrId) -> Option<Arc<str>> {
        self.rev.get(id.0 as usize).cloned()
    }
}
