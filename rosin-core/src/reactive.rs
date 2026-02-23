#![allow(clippy::mutable_key_type)]
#![allow(clippy::type_complexity)]
//! Provides values that allow Rosin to track data dependencies automatically.
//! - Use [`Var<T>`] for owned values inside your application state.
//! - Use [`WeakVar<T>`] to give non-owning handles for those values to callbacks.
//!
//! Rosin records which variables are read while building the UI. When a [`Var`] is
//! later written, it can efficiently update only the parts of the UI that depended on it.
//!
//! A [`WeakVar`] becomes invalid once the owning [`Var`] is dropped; most operations then return `None`.
//!
//! Dependency tracking is cheap, so these types can be used extensively.
//!
//! **Deadlock note:** like other synchronization primitives, accessing [`Var`]s across
//! threads in an inconsistent order can cause lock inversion, potentially leading to deadlocks.
//! Always acquire locks in a consistent order to prevent this.
//!
//! ## Example
//! ```ignore
//! struct State {
//!     count: Var<u32>,
//! }
//!
//! fn view(state: &State, ui: &mut Ui<State, WindowHandle>) {
//!     let count = state.count.downgrade();
//!
//!     ui.node().children(move |ui| {
//!         label(ui, id!(), count);
//!
//!         button(ui, id!(), "Count", move |_, _| {
//!             if let Some(mut c) = count.write() {
//!                 *c += 1;
//!             }
//!         });
//!     });
//! }
//! ```
//!
//! ### `serde` feature
//!
//! When the `serde` feature is enabled, [`Var`] and [`WeakVar`] can be serialized and deserialized,
//! but that must be done inside of a `serde_impl::serde_scope` in order to preserve dependencies.
//!
//! A [`WeakVar`] must be serialized or deserialized in the same scope as its associated [`Var`]. Scopes cannot be nested.

use std::{
    any::Any,
    cell::{OnceCell, RefCell},
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::OnceLock,
};

use crate::sync::*;

fn fmt_var_debug<T: fmt::Debug + Send + Sync + 'static>(slot: &Slot, generation: u64, struct_name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut ds = f.debug_struct(struct_name);

    if slot.generation.load(Ordering::Acquire) != generation {
        return ds.field("status", &"dropped").finish_non_exhaustive();
    }

    let Some(guard) = slot.value.try_read() else {
        return ds.field("status", &"locked").finish_non_exhaustive();
    };

    if let Some(any_val) = guard.as_ref() {
        if let Some(value) = any_val.downcast_ref::<T>() {
            ds.field("value", value).finish_non_exhaustive()
        } else {
            ds.field("status", &"type_mismatch")
                .field("expected", &std::any::type_name::<T>())
                .finish_non_exhaustive()
        }
    } else {
        ds.field("status", &"dropped").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VarKey {
    slot: &'static Slot,
    generation: u64,
}

impl Eq for VarKey {}
impl PartialEq for VarKey {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.slot, other.slot) && self.generation == other.generation
    }
}

impl Hash for VarKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.slot, state);
        self.generation.hash(state);
    }
}

/// A reactive value that affects on-screen content.
///
/// Viewports track when a [`Var`] is read, automatically determining which parts of the UI it affects.
/// When the variable is modified, the required updates will be applied to the screen.
///
/// This derefs to [`WeakVar`] a non-owning, Copy handle.
///
/// Anything visible and dynamic should be stored in a [`Var`].
///
/// **Deadlock note:** like other synchronization primitives, accessing [`Var`]s across
/// threads in an inconsistent order can cause lock inversion, potentially leading to deadlocks.
/// Always acquire locks in a consistent order to prevent this.
pub struct Var<T: Send + Sync + 'static>(pub(crate) WeakVar<T>);

impl<T: Send + Sync + 'static> Deref for Var<T> {
    type Target = WeakVar<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Send + Sync + 'static> Drop for Var<T> {
    fn drop(&mut self) {
        let registry = self.0.registry;
        let slot = self.0.slot;
        let prev = slot.generation.fetch_add(1, Ordering::Release);
        registry.write_count.fetch_add(1, Ordering::Release);

        // If a WeakVar has a lock on the data, it will handle the cleanup when it's done.
        slot.attempt_cleanup(registry, prev + 1);
    }
}

impl<T: Send + Sync + 'static> From<T> for Var<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Send + Sync + Default + 'static> Default for Var<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: fmt::Debug + Send + Sync + 'static> fmt::Debug for Var<T> {
    /// Debug formatting is side-effect free, so viewports won't register the value as having been read from.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_var_debug::<T>(self.slot, self.generation, "Var", f)
    }
}

impl<T: Send + Sync + 'static> Var<T> {
    /// Creates a new [`Var`] in the global registry with the specified initial value.
    pub fn new(value: T) -> Self {
        Self::new_in(Registry::global(), value)
    }

    /// Creates a new [`Var`] in the provided registry with the specified initial value.
    pub(crate) fn new_in(registry: &'static Registry, value: T) -> Self {
        let (slot, generation) = registry.alloc_slot();
        *slot.value.write() = Some(Box::new(value));
        slot.version.store(0, Ordering::Release);

        Var(WeakVar {
            registry,
            slot,
            generation,
            ty: PhantomData,
        })
    }

    /// Returns a read guard to the value if the [`Var`] is alive, marking it as read from.
    pub fn read<'a>(&'a self) -> VarReadGuard<'a, T> {
        WeakVar::read(self).unwrap() // Unwrap ok: we have a Var, so we know the value hasn't been dropped.
    }

    /// Returns a write guard to the value if the [`Var`] is alive.
    ///
    /// The guard handles marking the [`Var`] as written to and read from when it is dropped.
    pub fn write<'a>(&'a self) -> VarWriteGuard<'a, T> {
        WeakVar::write(self).unwrap() // Unwrap ok: we have a Var, so we know the value hasn't been dropped.
    }

    /// Returns the current version of the variable.
    pub fn get_version(&self) -> u64 {
        WeakVar::get_version(self).unwrap() // Unwrap ok: we have a Var, so we know the value hasn't been dropped.
    }

    /// Returns a clone of the stored value if it is still alive, marking it as read from.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        WeakVar::get(self).unwrap() // Unwrap ok: we have a Var, so we know the value hasn't been dropped.
    }

    /// Sets the value of the variable, but only bumps the version if the value actually changed.
    pub fn set(&self, new: T)
    where
        T: PartialEq,
    {
        WeakVar::set(self, new).unwrap() // Unwrap ok: we have a Var, so we know the value hasn't been dropped.
    }

    /// Replaces the value in the registry and returns the old value.
    pub fn replace(&self, new: T) -> T {
        WeakVar::replace(self, new).unwrap() // Unwrap ok: we have a Var, so we know the value hasn't been dropped.
    }

    /// Takes the current value, leaving [`Default::default()`] in its place.
    pub fn take(&self) -> T
    where
        T: Default,
    {
        WeakVar::take(self).unwrap() // Unwrap ok: we have a Var, so we know the value hasn't been dropped.
    }

    /// Returns a [`WeakVar`] that references the same value without taking ownership.
    ///
    /// This is useful for storing or passing a handle to the value without keeping it alive.
    /// The returned [`WeakVar`] can be cheaply copied and used to access or update the value
    /// as long as the original [`Var`] is still alive.
    pub fn downgrade(&self) -> WeakVar<T> {
        self.0
    }
}

/// A handle to a [`Var`] that implements Copy.
///
/// Intended to be stored in callbacks.
pub struct WeakVar<T: Send + Sync + 'static> {
    pub(crate) registry: &'static Registry,
    pub(crate) slot: &'static Slot,
    pub(crate) generation: u64,
    pub(crate) ty: PhantomData<T>,
}

// Impl Copy even if `T` doesn't
impl<T: Send + Sync + 'static> Copy for WeakVar<T> {}
impl<T: Send + Sync + 'static> Clone for WeakVar<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Eq for WeakVar<T> {}
impl<T: Send + Sync + 'static> PartialEq for WeakVar<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.slot, other.slot) && self.generation == other.generation
    }
}

impl<T: fmt::Debug + Send + Sync + 'static> fmt::Debug for WeakVar<T> {
    /// Debug formatting is side-effect free, so viewports won't register the value as having been read from.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_var_debug::<T>(self.slot, self.generation, "WeakVar", f)
    }
}

impl<T: Send + Sync + 'static> WeakVar<T> {
    /// Returns an opaque key representing the associated [`Var`].
    pub(crate) fn get_key(&self) -> VarKey {
        VarKey {
            slot: self.slot,
            generation: self.generation,
        }
    }

    /// Checks if the associated Var has been dropped.
    pub fn is_alive(&self) -> bool {
        self.slot.generation.load(Ordering::Acquire) == self.generation
    }

    /// Returns the current version of the variable.
    pub fn get_version(&self) -> Option<u64> {
        if self.is_alive() {
            Some(self.slot.version.load(Ordering::Acquire))
        } else {
            None
        }
    }

    /// Returns a read guard to the value if the [`Var`] is alive, marking it as read from.
    ///
    /// Returns [`None`] if the [`Var`] has been destroyed.
    pub fn read<'a>(&'a self) -> Option<VarReadGuard<'a, T>> {
        let guard = self.slot.value.read();

        if self.slot.generation.load(Ordering::Acquire) != self.generation {
            drop(guard);
            self.slot.attempt_cleanup(self.registry, self.generation + 1);
            return None;
        }

        let guard = RwLockReadGuard::try_map(guard, |opt: &Option<Box<dyn Any + Send + Sync>>| {
            let boxed = opt.as_ref()?;
            (boxed.as_ref() as &dyn Any).downcast_ref::<T>()
        })
        .ok()?;

        Some(VarReadGuard {
            meta: VarGuardMeta {
                registry: self.registry,
                slot: self.slot,
                generation: self.generation,
            },
            guard: Some(guard),
            armed: true,
            _marker: PhantomData,
        })
    }

    /// Returns a write guard to the value if the [`Var`] is alive.
    ///
    /// The guard handles marking the [`Var`] as written to and read from when it is dropped.
    ///
    /// Returns [`None`] if the [`Var`] has been destroyed.
    pub fn write<'a>(&'a self) -> Option<VarWriteGuard<'a, T>> {
        let guard = self.slot.value.write();

        if self.slot.generation.load(Ordering::Acquire) != self.generation {
            drop(guard);
            self.slot.attempt_cleanup(self.registry, self.generation + 1);
            return None;
        }

        let guard = RwLockWriteGuard::try_map(guard, |opt: &mut Option<Box<dyn Any + Send + Sync>>| {
            let boxed = opt.as_mut()?;
            (boxed.as_mut() as &mut dyn Any).downcast_mut::<T>()
        })
        .ok()?;

        Some(VarWriteGuard {
            meta: VarGuardMeta {
                registry: self.registry,
                slot: self.slot,
                generation: self.generation,
            },
            guard: Some(guard),
            changed: true,
            armed: true,
            _marker: PhantomData,
        })
    }

    /// Returns a clone of the stored value if it is still alive, marking it as read from. Returns [`None`] otherwise.
    pub fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.read().map(|guard| (*guard).clone())
    }

    /// Returns the value if the [`Var`] is alive, or `default` if it has been dropped.
    ///
    /// This method evaluates `default` eagerly, even if the value is alive.
    /// Use [`WeakVar::get_or_else`] to evaluate the default lazily.
    pub fn get_or(&self, default: T) -> T
    where
        T: Clone,
    {
        self.get().unwrap_or(default)
    }

    /// Returns a clone of the value if alive, or computes a default from the given closure.
    ///
    /// Unlike [`WeakVar::get_or`], this only evaluates `default` if the [`Var`] is dead.
    pub fn get_or_else(&self, default: impl FnOnce() -> T) -> T
    where
        T: Clone,
    {
        if let Some(val) = self.get() { val } else { default() }
    }

    /// Sets the value of the variable, but only bumps the version if the value actually changed.
    ///
    /// Returns `None` if the [`Var`] has been destroyed, `Some(())` otherwise.
    pub fn set(&self, new: T) -> Option<()>
    where
        T: PartialEq,
    {
        let mut guard = self.write()?;
        if *guard != new {
            *guard = new;
        } else {
            guard.cancel_change();
        }
        Some(())
    }

    /// Replaces the value in the registry and returns the old value.
    pub fn replace(&self, new: T) -> Option<T> {
        let mut guard = self.write()?;
        Some(std::mem::replace(&mut *guard, new))
    }

    /// Takes the current value, leaving [`Default::default()`] in its place.
    pub fn take(&self) -> Option<T>
    where
        T: Default,
    {
        let mut guard = self.write()?;
        Some(std::mem::take(&mut *guard))
    }

    /// Marks a [`Var`] as if it had been read from.
    ///
    /// This is a no-op if the [`Var`] has been destroyed.
    pub fn mark_read(&self) {
        if self.is_alive() {
            let key = self.get_key();
            let version = self.slot.version.load(Ordering::Acquire);
            notify_scopes(key, version);
        }
    }
}

/// Used to track when [`Var`]s have been read from and written to.
///
/// Most apps will not need to use this.
#[derive(Debug, Default, Clone)]
pub struct DependencyMap {
    pub(crate) deps: HashMap<VarKey, u64>,
}

impl DependencyMap {
    pub(crate) fn record(&mut self, key: VarKey, version: u64) {
        self.deps.entry(key).and_modify(|v| *v = std::cmp::max(*v, version)).or_insert(version);
    }

    /// Removes all dependencies from the map.
    pub fn clear(&mut self) {
        self.deps.clear();
    }

    /// Returns the map with all dependencies removed.
    pub fn cleared(mut self) -> Self {
        self.deps.clear();
        self
    }

    /// Returns `true` as soon as it finds any var in the map that's been dropped, or whose current_version != last_seen.
    ///
    /// Does not update the last seen versions.
    pub fn any_changed(&self) -> bool {
        for (key, last_seen) in self.deps.iter() {
            let current = key.slot.version.load(Ordering::Acquire);
            if key.slot.generation.load(Ordering::Acquire) != key.generation {
                return true;
            }
            if current != *last_seen {
                return true;
            }
        }
        false
    }

    /// Returns `true` if any var in the map changed since last time this function was called, or if any var was dropped.
    ///
    /// Updates the last seen versions and removes dropped vars.
    pub fn any_changed_update(&mut self) -> bool {
        let mut changed = false;

        self.deps.retain(|key, last_seen| {
            // If the var was dropped, remove it and report changed.
            if key.slot.generation.load(Ordering::Acquire) != key.generation {
                changed = true;
                return false;
            }

            // Otherwise update the stored value if it changed.
            let current = key.slot.version.load(Ordering::Acquire);
            if current != *last_seen {
                changed = true;
                *last_seen = current;
            }

            true
        });

        changed
    }

    /// Marks all dependencies in the map as read from.
    pub fn mark_read(&self) {
        for key in self.deps.keys() {
            // Only mark if still alive
            if key.slot.generation.load(Ordering::Acquire) == key.generation {
                let version = key.slot.version.load(Ordering::Acquire);
                notify_scopes(*key, version);
            }
        }
    }

    /// Any [`Var`] that is read from during the provided closure will be added to the dependency map.
    /// Nested scopes are supported; any reads will be logged in all scopes.
    ///
    /// NOTE: Dependencies are only recorded when a read or write guard drops,
    /// so it's important that guards don't escape the scope.
    pub fn read_scope(self, func: impl FnMut()) -> Self {
        scopes_with(|stack| stack.borrow_mut().push(self));

        // If the provided closure panics, clean up so the stack is never in an inconsistent state.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(func));

        let deps = scopes_with(|stack| stack.borrow_mut().pop().unwrap()); // Unwrap ok: we pushed an element on to the stack, so we know it will be there to pop.

        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }

        deps
    }
}

/// Identity metadata for a Var, stored in guards.
///
/// We intentionally don't store `WeakVar<T>` in mapped guards.
/// After mapping, `T` would no longer represent the actual stored type inside the slot.
#[derive(Clone, Copy)]
struct VarGuardMeta {
    registry: &'static Registry,
    slot: &'static Slot,
    generation: u64,
}

impl VarGuardMeta {
    #[inline]
    fn key(&self) -> VarKey {
        VarKey {
            slot: self.slot,
            generation: self.generation,
        }
    }
}

/// A read lock guard returned by [`Var::read`] or [`WeakVar::read`].
///
/// While the guard is alive, it holds a shared lock on the underlying value.
/// When the guard is dropped, the variable is registered as having been read
/// so viewports can track dependencies.
///
/// Mapped guards produced by [`VarReadGuard::map`] and friends preserve the same
/// behavior: dependency tracking happens when the final mapped guard is dropped.
pub struct VarReadGuard<'a, T: ?Sized + Send + Sync + 'static> {
    meta: VarGuardMeta,
    guard: Option<MappedRwLockReadGuard<'a, T>>,
    armed: bool,
    _marker: PhantomData<*const ()>, // !Send + !Sync
}

impl<'a, T: ?Sized + Send + Sync + 'static> Deref for VarReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap() // Unwrap ok: The inner guard is only `None` after being consumed by `Drop`.
    }
}

impl<'a, T: ?Sized + Send + Sync + 'static> Drop for VarReadGuard<'a, T> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        let version = self.meta.slot.version.load(Ordering::Acquire);
        drop(self.guard.take());
        notify_scopes(self.meta.key(), version);
        if self.meta.slot.generation.load(Ordering::Acquire) != self.meta.generation {
            self.meta.slot.attempt_cleanup(self.meta.registry, self.meta.generation + 1);
        }
    }
}

impl<'a, T: ?Sized + Send + Sync + 'static> VarReadGuard<'a, T> {
    /// Maps a read guard to a subfield, like `RwLockReadGuard::map`.
    ///
    /// This is an associated function that needs to be used as `VarReadGuard::map(...)`.
    pub fn map<U: ?Sized + Send + Sync + 'static>(mut this: Self, f: impl FnOnce(&T) -> &U) -> VarReadGuard<'a, U> {
        let meta = this.meta;
        let guard = this.guard.take().expect("mapped guard is missing");

        // If `f` panics, `this` will be dropped with `armed = true`, notifying once.
        let guard = MappedRwLockReadGuard::map(guard, f);

        // Disarm so we don't double-notify on Drop.
        this.armed = false;

        VarReadGuard {
            meta,
            guard: Some(guard),
            armed: true,
            _marker: PhantomData,
        }
    }

    /// Fallible map, like `RwLockReadGuard::try_map`.
    ///
    /// This is an associated function that needs to be used as `VarReadGuard::try_map(...)`.
    pub fn try_map<U: ?Sized + Send + Sync + 'static>(mut this: Self, f: impl FnOnce(&T) -> Option<&U>) -> Result<VarReadGuard<'a, U>, VarReadGuard<'a, T>> {
        let meta = this.meta;
        let guard = this.guard.take().expect("mapped guard is missing");

        match MappedRwLockReadGuard::try_map(guard, f) {
            Ok(mapped) => {
                // Disarm so we don't double-notify on Drop.
                this.armed = false;

                Ok(VarReadGuard {
                    meta,
                    guard: Some(mapped),
                    armed: true,
                    _marker: PhantomData,
                })
            }
            Err(original) => {
                // Put the original guard back and return it.
                this.guard = Some(original);
                Err(this)
            }
        }
    }

    /// Fallible map, like `RwLockReadGuard::try_map`, but returns the error produced by the mapping function.
    ///
    /// This is an associated function that needs to be used as `VarReadGuard::try_map_or_err(...)`.
    pub fn try_map_or_err<U: ?Sized + Send + Sync + 'static, E>(
        mut this: Self,
        f: impl FnOnce(&T) -> Result<&U, E>,
    ) -> Result<VarReadGuard<'a, U>, (VarReadGuard<'a, T>, E)> {
        let meta = this.meta;
        let guard = this.guard.take().expect("mapped guard is missing");

        let mut err: Option<E> = None;

        let mapped = MappedRwLockReadGuard::try_map(guard, |t| match f(t) {
            Ok(r) => Some(r),
            Err(e) => {
                err = Some(e);
                None
            }
        });

        match mapped {
            Ok(mapped) => {
                // Disarm so we don't double-notify on Drop.
                this.armed = false;

                Ok(VarReadGuard {
                    meta,
                    guard: Some(mapped),
                    armed: true,
                    _marker: PhantomData,
                })
            }
            Err(original) => {
                // Put the original guard back and return it along with the error.
                this.guard = Some(original);
                let e = err.expect("try_map_or_err failed without producing an error");
                Err((this, e))
            }
        }
    }
}

/// A write lock guard returned by [`Var::write`] or [`WeakVar::write`].
///
/// While the guard is alive, it holds an exclusive lock on the underlying value.
/// When the guard is dropped, the variable is registered as having been read
/// and, by default, written, which bumps the variable's version and triggers
/// dependent UI updates.
///
/// If no observable change occurred, call [`VarWriteGuard::cancel_change`]
/// before dropping the guard to prevent the version bump.
///
/// Mapped guards produced by [`VarWriteGuard::map`] and friends preserve the same
/// behavior: the write is committed (unless canceled) when the final mapped guard
/// is dropped.
pub struct VarWriteGuard<'a, T: ?Sized + Send + Sync + 'static> {
    meta: VarGuardMeta,
    guard: Option<MappedRwLockWriteGuard<'a, T>>,
    changed: bool,
    armed: bool,
    _marker: PhantomData<*const ()>, // !Send + !Sync
}

impl<'a, T: ?Sized + Send + Sync + 'static> Deref for VarWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap() // Unwrap ok: The inner guard is only `None` after being consumed by `Drop`.
    }
}

impl<'a, T: ?Sized + Send + Sync + 'static> DerefMut for VarWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap() // Unwrap ok: The inner guard is only `None` after being consumed by `Drop`.
    }
}

impl<'a, T: ?Sized + Send + Sync + 'static> Drop for VarWriteGuard<'a, T> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        let version = if self.changed {
            self.meta.registry.write_count.fetch_add(1, Ordering::Release);
            self.meta.slot.version.fetch_add(1, Ordering::Release) + 1
        } else {
            self.meta.slot.version.load(Ordering::Acquire)
        };

        drop(self.guard.take());
        notify_scopes(self.meta.key(), version);
        if self.meta.slot.generation.load(Ordering::Acquire) != self.meta.generation {
            self.meta.slot.attempt_cleanup(self.meta.registry, self.meta.generation + 1);
        }
    }
}

impl<'a, T: ?Sized + Send + Sync + 'static> VarWriteGuard<'a, T> {
    /// Prevents the dependency tracking system from registering a change.
    ///
    /// When this is called, the [`Var`]'s version number will not be incremented upon `Drop`.
    pub fn cancel_change(&mut self) {
        self.changed = false;
    }

    /// Maps a write guard to a subfield, like `RwLockWriteGuard::map`.
    ///
    /// This is an associated function that needs to be used as `VarWriteGuard::map(...)`.
    pub fn map<U: ?Sized + Send + Sync + 'static>(mut this: Self, f: impl FnOnce(&mut T) -> &mut U) -> VarWriteGuard<'a, U> {
        let meta = this.meta;
        let changed = this.changed;
        let guard = this.guard.take().expect("mapped guard is missing");

        // If `f` panics, `this` will be dropped with `armed = true`, notifying once.
        let guard = MappedRwLockWriteGuard::map(guard, f);

        // Disarm so we don't double-notify on Drop.
        this.armed = false;

        VarWriteGuard {
            meta,
            guard: Some(guard),
            changed,
            armed: true,
            _marker: PhantomData,
        }
    }

    /// Fallible map, like `RwLockWriteGuard::try_map`.
    ///
    /// This is an associated function that needs to be used as `VarWriteGuard::try_map(...)`.
    pub fn try_map<U: ?Sized + Send + Sync + 'static>(
        mut this: Self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Result<VarWriteGuard<'a, U>, VarWriteGuard<'a, T>> {
        let meta = this.meta;
        let changed = this.changed;
        let guard = this.guard.take().expect("mapped guard is missing");

        match MappedRwLockWriteGuard::try_map(guard, f) {
            Ok(mapped) => {
                // Disarm so we don't double-notify on Drop.
                this.armed = false;

                Ok(VarWriteGuard {
                    meta,
                    guard: Some(mapped),
                    changed,
                    armed: true,
                    _marker: PhantomData,
                })
            }
            Err(original) => {
                // Put the original guard back and return it.
                this.guard = Some(original);
                Err(this)
            }
        }
    }

    /// Fallible map, like `RwLockWriteGuard::try_map`, but returns the error produced by the mapping function.
    ///
    /// This is an associated function that needs to be used as `VarWriteGuard::try_map_or_err(...)`.
    pub fn try_map_or_err<U: ?Sized + Send + Sync + 'static, E>(
        mut this: Self,
        f: impl FnOnce(&mut T) -> Result<&mut U, E>,
    ) -> Result<VarWriteGuard<'a, U>, (VarWriteGuard<'a, T>, E)> {
        let meta = this.meta;
        let changed = this.changed;
        let guard = this.guard.take().expect("mapped guard is missing");

        let mut err: Option<E> = None;

        let mapped = MappedRwLockWriteGuard::try_map(guard, |t| match f(t) {
            Ok(r) => Some(r),
            Err(e) => {
                err = Some(e);
                None
            }
        });

        match mapped {
            Ok(mapped) => {
                // Disarm so we don't double-notify on Drop.
                this.armed = false;

                Ok(VarWriteGuard {
                    meta,
                    guard: Some(mapped),
                    changed,
                    armed: true,
                    _marker: PhantomData,
                })
            }
            Err(original) => {
                // Put the original guard back and return it along with the error.
                this.guard = Some(original);
                let e = err.expect("try_map_or_err failed without producing an error");
                Err((this, e))
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Slot {
    pub(crate) generation: AtomicU64,
    pub(crate) version: AtomicU64,
    pub(crate) value: RwLock<Option<Box<dyn Any + Send + Sync>>>,
}

impl Slot {
    fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            version: AtomicU64::new(0),
            value: RwLock::new(None),
        }
    }

    fn attempt_cleanup(&'static self, registry: &'static Registry, expected_gen: u64) {
        // We only recycle the slot if we are the thread that transitions the value from Some -> None.
        if let Some(mut guard) = self.value.try_write()
            && self.generation.load(Ordering::Acquire) == expected_gen
            && guard.is_some()
        {
            let value = guard.take();
            drop(guard);

            struct RecycleGuard {
                registry: &'static Registry,
                slot: &'static Slot,
            }

            impl Drop for RecycleGuard {
                fn drop(&mut self) {
                    self.registry.recycle_slot(self.slot);
                }
            }

            let _recycle_guard = RecycleGuard { registry, slot: self };
            drop(value);
        }
    }
}

#[cfg(not(loom))]
std::thread_local! {
    /// A stack of all current read_scopes used by viewports for dependency tracking.
    static READ_SCOPES: OnceCell<Rc<RefCell<Vec<DependencyMap>>>> = const { OnceCell::new() };
}

// Loom doesn't support const in thread_local macro
#[cfg(loom)]
loom::thread_local! {
    static READ_SCOPES: OnceCell<Rc<RefCell<Vec<DependencyMap>>>> = OnceCell::new();
}

/// Notify read scopes of a dependency.
fn notify_scopes(key: VarKey, version: u64) {
    scopes_with(|stack| {
        for vars in stack.borrow_mut().iter_mut() {
            vars.record(key, version);
        }
    });
}

/// Access the thread-local stack of read scopes.
fn scopes_with<F, R>(f: F) -> R
where
    F: FnOnce(&Rc<RefCell<Vec<DependencyMap>>>) -> R,
{
    READ_SCOPES.with(|once| f(once.get_or_init(|| Rc::new(RefCell::new(Vec::new())))))
}

/// Returns a clone of the current thread-local `READ_SCOPES` Rc, initializing it if needed.
/// This is used internally by the hot-reload feature.
#[doc(hidden)]
pub fn read_scopes_rc() -> Rc<RefCell<Vec<DependencyMap>>> {
    scopes_with(|stack| stack.clone())
}

/// Tries to initialize the thread-local `READ_SCOPES`, returning `true` if it succeeded.
/// This is used internally by the hot-reload feature.
#[doc(hidden)]
pub fn try_init_read_scopes(rc: Rc<RefCell<Vec<DependencyMap>>>) -> bool {
    READ_SCOPES.with(|cell| cell.set(rc).is_ok())
}

/// The container for all reactive variables. Most code won't need to interact with this directly.
#[doc(hidden)]
pub struct Registry {
    free_slots: Mutex<Vec<&'static Slot>>,
    write_count: AtomicU64,
}

static GLOBAL_REGISTRY: OnceLock<&'static Registry> = OnceLock::new();

impl Default for Registry {
    fn default() -> Self {
        Self {
            free_slots: Mutex::new(Vec::new()),
            write_count: AtomicU64::new(0),
        }
    }
}

impl Registry {
    /// Gets a reference to the global [`Registry`] instance, initializing it if needed.
    pub fn global() -> &'static Self {
        GLOBAL_REGISTRY.get_or_init(|| Box::leak(Box::new(Self::default())))
    }

    /// Initialize the global [`Registry`] instance, if it isn't already.
    pub fn set_global(&'static self) -> bool {
        GLOBAL_REGISTRY.set(self).is_ok()
    }

    /// Returns the total number of committed writes for all variables in the registry.
    pub fn write_count(&self) -> u64 {
        self.write_count.load(Ordering::Acquire)
    }

    /// Returns an empty [`Slot`] and generation counter.
    fn alloc_slot(&self) -> (&'static Slot, u64) {
        let mut free = self.free_slots.lock();
        if let Some(slot) = free.pop() {
            let next_gen = slot.generation.fetch_add(1, Ordering::Release) + 1;
            (slot, next_gen)
        } else {
            let slot = Box::leak(Box::new(Slot::new()));
            (slot, 0)
        }
    }

    /// Queues up a [`Slot`] to be re-used later.
    fn recycle_slot(&self, slot: &'static Slot) {
        let mut free = self.free_slots.lock();
        free.push(slot);
    }
}

#[cfg(feature = "serde")]
pub mod serde_impl {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};
    use std::cell::RefCell;

    type SlotId = u64;

    struct DeEntry {
        slot: &'static Slot,
        pending_generation: u64,
        initialized: bool,
    }

    struct SerdeContext {
        ser_map: HashMap<VarKey, SlotId>,
        de_map: HashMap<SlotId, DeEntry>,
        next_id: SlotId,
    }

    impl SerdeContext {
        fn new() -> Self {
            Self {
                ser_map: HashMap::new(),
                de_map: HashMap::new(),
                next_id: 1, // 0 reserved for dead vars
            }
        }

        fn cleanup_uninitialized(self) {
            let registry = Registry::global();

            for entry in self.de_map.into_values() {
                if entry.initialized {
                    continue;
                }

                entry.slot.generation.store(entry.pending_generation + 1, Ordering::Release);
                *entry.slot.value.write() = None;
                entry.slot.version.store(0, Ordering::Release);
                registry.recycle_slot(entry.slot);
            }
        }
    }

    thread_local! {
        static CONTEXT: RefCell<Option<SerdeContext>> = const { RefCell::new(None) };
    }

    // TODO - this can be made more testable by accepting a registry and storing it in the SerdeContext
    pub fn serde_scope<R>(f: impl FnOnce() -> R) -> R {
        CONTEXT.with(|ctx| {
            let mut borrow = ctx.borrow_mut();
            if borrow.is_some() {
                panic!("nested serde_scope is not supported");
            }
            *borrow = Some(SerdeContext::new());
        });

        struct ScopeGuard;
        impl Drop for ScopeGuard {
            fn drop(&mut self) {
                CONTEXT.with(|ctx| {
                    let mut borrow = ctx.borrow_mut();
                    if let Some(ctx) = borrow.take() {
                        ctx.cleanup_uninitialized();
                    }
                });
            }
        }

        let _guard = ScopeGuard;
        f()
    }

    fn resolve_id<S: Serializer, T>(v: &WeakVar<T>) -> Result<SlotId, S::Error>
    where
        T: Send + Sync + 'static,
    {
        if !v.is_alive() {
            return Ok(0);
        }

        CONTEXT.with(|cell| {
            let mut borrow = cell.borrow_mut();
            let ctx = borrow.as_mut().ok_or_else(|| ser::Error::custom("Serialize called outside of a scope"))?;

            let key = v.get_key();
            Ok(*ctx.ser_map.entry(key).or_insert_with(|| {
                let id = ctx.next_id;
                ctx.next_id += 1;
                id
            }))
        })
    }

    fn resolve_handle<'de, D: Deserializer<'de>, T>(id: SlotId) -> Result<WeakVar<T>, D::Error>
    where
        T: Send + Sync + 'static,
    {
        let registry = Registry::global();

        if id == 0 {
            static DEAD_SLOT: OnceLock<Slot> = OnceLock::new();
            let slot = DEAD_SLOT.get_or_init(|| Slot {
                generation: AtomicU64::new(1),
                version: AtomicU64::new(0),
                value: RwLock::new(None),
            });

            return Ok(WeakVar {
                registry,
                slot,
                generation: 0,
                ty: PhantomData,
            });
        }

        let (slot, pending_generation) = CONTEXT.with(|cell| {
            let mut borrow = cell.borrow_mut();
            let ctx = borrow.as_mut().ok_or_else(|| de::Error::custom("Deserialize called outside of a scope"))?;

            if let Some(entry) = ctx.de_map.get(&id) {
                Ok((entry.slot, entry.pending_generation))
            } else {
                let (slot, generation) = registry.alloc_slot();
                let pending_generation = generation + 1;

                ctx.de_map.insert(
                    id,
                    DeEntry {
                        slot,
                        pending_generation,
                        initialized: false,
                    },
                );

                Ok((slot, pending_generation))
            }
        })?;

        Ok(WeakVar {
            registry,
            slot,
            generation: pending_generation,
            ty: PhantomData,
        })
    }

    impl<T> Serialize for Var<T>
    where
        T: Serialize + Send + Sync + 'static,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if let Some(guard) = self.0.read() {
                let id = resolve_id::<S, T>(&self.0)?;
                (id, Some(&*guard)).serialize(serializer)
            } else {
                (0u64, Option::<T>::None).serialize(serializer)
            }
        }
    }

    impl<'de, T> Deserialize<'de> for Var<T>
    where
        T: Deserialize<'de> + Send + Sync + 'static,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let (id, maybe_value) = <(SlotId, Option<T>)>::deserialize(deserializer)?;

            let Some(val) = maybe_value else {
                let handle = resolve_handle::<D, T>(0)?;
                return Ok(Var(handle));
            };

            let handle = resolve_handle::<D, T>(id)?;
            *handle.slot.value.write() = Some(Box::new(val));
            handle.slot.version.store(0, Ordering::Release);
            handle.slot.generation.store(handle.generation, Ordering::Release);

            CONTEXT.with(|cell| {
                let mut borrow = cell.borrow_mut();
                let ctx = borrow.as_mut().expect("Deserialize called outside of a scope");
                if let Some(entry) = ctx.de_map.get_mut(&id) {
                    entry.initialized = true;
                }
            });

            Ok(Var(handle))
        }
    }

    impl<T> Serialize for WeakVar<T>
    where
        T: Send + Sync + 'static,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            resolve_id::<S, T>(self)?.serialize(serializer)
        }
    }

    impl<'de, T> Deserialize<'de> for WeakVar<T>
    where
        T: Send + Sync + 'static,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let id = SlotId::deserialize(deserializer)?;
            resolve_handle::<D, T>(id)
        }
    }
}
