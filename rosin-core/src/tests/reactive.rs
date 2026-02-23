use std::{
    panic::{self, AssertUnwindSafe},
    thread,
};

use crate::{
    prelude::*,
    reactive::{Registry, VarKey},
};

fn get_slot_addr<T: Send + Sync + 'static>(var_ref: WeakVar<T>) -> usize {
    var_ref.slot as *const _ as usize
}

mod formatting {
    use super::*;

    #[test]
    fn format_var() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let output = format!("{:?}", var);
        assert!(output.contains("Var"));
        assert!(output.contains("value"));
        assert!(output.contains("10"));
    }

    #[test]
    fn format_ref() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();
        let output = format!("{:?}", var_ref);
        assert!(output.contains("WeakVar"));
        assert!(output.contains("value"));
        assert!(output.contains("10"));
    }

    #[test]
    fn format_locked() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();
        let _guard = var.write();

        let output = format!("{:?}", var_ref);
        assert!(output.contains("status"));
        assert!(output.contains("locked"));
    }

    #[test]
    fn format_dropped() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();
        drop(var);

        let output = format!("{:?}", var_ref);
        assert!(output.contains("status"));
        assert!(output.contains("dropped"));
    }

    #[test]
    fn format_type_mismatch() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, String::from("test"));
        let var_ref = var.downgrade();

        let bad_handle: WeakVar<i32> = WeakVar {
            registry: var_ref.registry,
            slot: var_ref.slot,
            generation: var_ref.generation,
            ty: std::marker::PhantomData,
        };

        let output = format!("{:?}", bad_handle);
        assert!(output.contains("status"));
        assert!(output.contains("type_mismatch"));
        assert!(output.contains("expected"));
        assert!(output.contains("i32"));
    }

    #[test]
    fn format_is_untracked() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 123);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            let _ = format!("{:?}", var);
            let _ = format!("{:?}", var_ref);
        });

        assert!(deps.deps.is_empty());
    }

    #[test]
    fn format_empty_slot() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10i32);
        let var_ref = var.downgrade();
        drop(var);

        let forged: WeakVar<i32> = WeakVar {
            registry: var_ref.registry,
            slot: var_ref.slot,
            generation: var_ref.generation + 1,
            ty: std::marker::PhantomData,
        };

        let output = format!("{:?}", forged);
        assert!(output.contains("status"));
        assert!(output.contains("dropped"));
    }
}

mod lifecycle {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn from_implementation() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let variable = Var::new_in(registry, 100);
        assert_eq!(variable.get(), 100);
    }

    #[test]
    fn default_implementation() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let variable: Var<i32> = Var::new_in(registry, i32::default());
        assert_eq!(variable.get(), 0);

        let str_var: Var<String> = Var::new_in(registry, String::default());
        assert_eq!(str_var.get(), "");
    }

    #[test]
    fn create_and_access() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 42);
        assert_eq!(var.get(), 42);
    }

    #[test]
    fn downgrade_lifespan() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 42);
        let var_ref = var.downgrade();
        assert!(var_ref.is_alive());

        drop(var);
        assert!(!var_ref.is_alive());
    }

    #[test]
    fn ref_equality() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var_a = Var::new_in(registry, 10);
        let var_b = Var::new_in(registry, 10);

        let var_ref_a = var_a.downgrade();
        let var_ref_b = var_b.downgrade();

        assert_eq!(var_ref_a, var_ref_a.clone());
        assert_ne!(var_ref_a, var_ref_b);
    }

    #[test]
    fn equality_strictness() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var_a = Var::new_in(registry, 10);
        let var_b = Var::new_in(registry, 10);
        assert_ne!(var_a.downgrade(), var_b.downgrade());
    }

    #[test]
    fn partial_eq_strictness_combinations() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var_a = Var::new_in(registry, 10);
        let var_ref_a = var_a.downgrade();

        let var_b = Var::new_in(registry, 99);
        let var_ref_b = var_b.downgrade();

        let forged_wrong_gen: WeakVar<i32> = WeakVar {
            registry: var_ref_a.registry,
            slot: var_ref_a.slot,
            generation: var_ref_a.generation + 1,
            ty: std::marker::PhantomData,
        };
        assert_ne!(var_ref_a, forged_wrong_gen);

        let forged_wrong_slot: WeakVar<i32> = WeakVar {
            registry: var_ref_a.registry,
            slot: var_ref_b.slot,
            generation: var_ref_a.generation,
            ty: std::marker::PhantomData,
        };
        assert_ne!(var_ref_a, forged_wrong_slot);
    }

    #[test]
    fn hash_implementation_discrimination() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var_a = Var::new_in(registry, 1);
        let var_b = Var::new_in(registry, 2);

        let mut hash_a = DefaultHasher::new();
        var_a.downgrade().get_key().hash(&mut hash_a);

        let mut hash_b = DefaultHasher::new();
        var_b.downgrade().get_key().hash(&mut hash_b);

        assert_ne!(hash_a.finish(), hash_b.finish());
    }

    #[test]
    fn version_tracking() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 7i32);
        let var_ref = var.downgrade();

        let ver_initial = var_ref.get_version();
        assert_eq!(ver_initial, Some(0));

        var_ref.set(8).unwrap();
        let ver_updated = var_ref.get_version();
        assert_eq!(ver_updated, Some(1));

        drop(var);
        assert_eq!(var_ref.get_version(), None);
    }

    #[test]
    fn var_get_version_owned() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 123i32);

        assert_eq!(var.get_version(), 0);

        var.set(124);
        assert_eq!(var.get_version(), 1);
    }

    #[test]
    fn var_replace_owned() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 1i32);

        let old = var.replace(999);
        assert_eq!(old, 1);
        assert_eq!(var.get(), 999);
        assert_eq!(var.get_version(), 1);
    }

    #[test]
    fn var_take_owned() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 7i32);

        let taken = var.take();
        assert_eq!(taken, 7);
        assert_eq!(var.get(), 0);
        assert_eq!(var.get_version(), 1);
    }
}

mod accessors {
    use super::*;

    #[test]
    fn get_or_variants() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, "hello");
        let var_ref = var.downgrade();

        assert_eq!(var_ref.get(), Some("hello"));
        assert_eq!(var_ref.get_or("default"), "hello");

        let mut callback_invoked = false;
        assert_eq!(
            var_ref.get_or_else(|| {
                callback_invoked = true;
                "default"
            }),
            "hello"
        );
        assert!(!callback_invoked);

        drop(var);

        assert_eq!(var_ref.get(), None);
        assert_eq!(var_ref.get_or("default"), "default");
        assert_eq!(
            var_ref.get_or_else(|| {
                callback_invoked = true;
                "default"
            }),
            "default"
        );
        assert!(callback_invoked);
    }

    #[test]
    fn set_value() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        assert!(var_ref.set(20).is_some());
        assert_eq!(var_ref.get(), Some(20));

        drop(var);
        assert!(var_ref.set(30).is_none());
    }

    #[test]
    fn replace_value() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let old = var_ref.replace(20);
        assert_eq!(old, Some(10));
        assert_eq!(var_ref.get(), Some(20));

        drop(var);
        assert_eq!(var_ref.replace(30), None);
    }

    #[test]
    fn take_value() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let val = var_ref.take();
        assert_eq!(val, Some(10));
        assert_eq!(var_ref.get(), Some(0));

        drop(var);
        assert_eq!(var_ref.take(), None);
    }

    #[test]
    fn panic_safety() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();
        drop(var);

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            var_ref.get_or_else(|| panic!("user panic"));
        }));

        assert!(result.is_err());

        let var_b = Var::new_in(registry, 20);
        assert_eq!(var_b.get(), 20);
    }

    #[test]
    fn access_dead_var() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 5);
        let var_ref = var.downgrade();
        drop(var);

        assert!(var_ref.read().is_none());
        assert!(var_ref.write().is_none());
    }
}

mod changes {
    use super::*;

    #[test]
    fn empty_change_detection() {
        let deps = DependencyMap::default();
        assert!(!deps.any_changed());
    }

    #[test]
    fn detect_change() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        assert!(!deps.any_changed());

        var_ref.set(20);
        assert!(deps.any_changed());
    }

    #[test]
    fn redundant_set_ignored() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        let ver_before = var_ref.get_version().unwrap();
        var_ref.set(10);
        let ver_after = var_ref.get_version().unwrap();

        assert_eq!(ver_before, ver_after);
        assert!(!deps.any_changed());

        var_ref.set(11);
        assert!(deps.any_changed());
    }

    #[test]
    fn write_guard_cancel() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        let ver_before = var_ref.get_version().unwrap();

        if let Some(mut guard) = var_ref.write() {
            *guard = 15;
            guard.cancel_change();
        }

        let ver_after = var_ref.get_version().unwrap();
        assert_eq!(ver_before, ver_after);
        assert_eq!(var_ref.get(), Some(15));
        assert!(!deps.any_changed());
    }

    #[test]
    fn write_guard_bumps_version() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let ver_initial = var_ref.get_version().unwrap();
        {
            let mut guard = var_ref.write().unwrap();
            *guard = 999;
        }
        let ver_final = var_ref.get_version().unwrap();

        assert_eq!(ver_initial + 1, ver_final);
        assert_eq!(var_ref.get(), Some(999));
    }

    #[test]
    fn track_read() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            let _ = var_ref.read();
        });

        assert!(deps.deps.contains_key(&var_ref.get_key()));
    }

    #[test]
    fn track_get() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            let _ = var_ref.get();
        });

        assert!(deps.deps.contains_key(&var_ref.get_key()));
    }

    #[test]
    fn track_set() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            let _ = var_ref.set(20);
        });

        assert!(deps.deps.contains_key(&var_ref.get_key()));
    }

    #[test]
    fn manual_mark_read() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        assert!(deps.deps.contains_key(&var_ref.get_key()));
    }

    #[test]
    fn dead_mark_read_noop() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();
        drop(var);

        let deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        assert!(deps.deps.is_empty());
    }

    #[test]
    fn nested_scopes() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let outer_key = var.downgrade().get_key();

        let inner_result: std::cell::RefCell<Option<DependencyMap>> = std::cell::RefCell::new(None);

        let outer = DependencyMap::default().read_scope(|| {
            let inner = DependencyMap::default().read_scope(|| {
                var.downgrade().mark_read();
            });
            *inner_result.borrow_mut() = Some(inner);
        });

        let inner = inner_result.borrow_mut().take().expect("inner map missing");

        assert!(inner.deps.contains_key(&outer_key));
        assert!(outer.deps.contains_key(&outer_key));
    }

    #[test]
    fn scope_isolation() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var_a = Var::new_in(registry, 1);
        let var_b = Var::new_in(registry, 2);

        let key_a = var_a.downgrade().get_key();
        let key_b = var_b.downgrade().get_key();

        let deps_a = DependencyMap::default().read_scope(|| {
            var_a.downgrade().mark_read();
        });

        let deps_b = DependencyMap::default().read_scope(|| {
            var_b.downgrade().mark_read();
        });

        assert!(deps_a.deps.contains_key(&key_a));
        assert!(!deps_a.deps.contains_key(&key_b));

        assert!(!deps_b.deps.contains_key(&key_a));
        assert!(deps_b.deps.contains_key(&key_b));
    }

    #[test]
    fn scope_unwind_on_panic() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _deps = DependencyMap::default().read_scope(|| {
                var.downgrade().mark_read();
                panic!("intentional panic inside read_scope");
            });
        }));

        assert!(result.is_err());

        let deps_b = DependencyMap::default().read_scope(|| {});
        assert!(deps_b.deps.is_empty());
    }

    #[test]
    fn thread_scope_isolation() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 123);
        let var_ref = var.downgrade();

        let deps_main = DependencyMap::default();
        let deps_thread = DependencyMap::default();

        let deps_main = deps_main.read_scope(|| {
            var_ref.mark_read();
        });
        assert!(deps_main.deps.contains_key(&var_ref.get_key()));

        let thread_handle = thread::spawn(move || {
            deps_thread.read_scope(|| {
                var_ref.mark_read();
            })
        });
        let deps_thread = thread_handle.join().unwrap();

        assert!(deps_thread.deps.contains_key(&var_ref.get_key()));

        assert!(deps_main.deps.keys().copied().collect::<Vec<VarKey>>() == vec![var_ref.get_key()]);
        assert!(deps_thread.deps.keys().copied().collect::<Vec<VarKey>>() == vec![var_ref.get_key()]);
    }

    #[test]
    fn multi_version_scope_update() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
            var_ref.set(20);
            var_ref.mark_read();
        });

        let recorded_version = deps.deps.get(&var_ref.get_key()).unwrap();
        assert_eq!(*recorded_version, 1);
    }

    #[test]
    fn any_changed_detects_dropped_var() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 123i32);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        drop(var);

        assert!(deps.any_changed());
    }
}

mod internals {
    use super::*;

    #[test]
    fn create_default() {
        let var: Var<u32> = Var::default();
        assert_eq!(var.get(), 0);
    }

    #[test]
    fn create_from() {
        let var = Var::from(10);
        assert_eq!(var.get(), 10);
    }

    #[test]
    fn stale_access_cleanup() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 100);
        let var_ref = var.downgrade();

        drop(var);

        assert!(var_ref.read().is_none());
        assert!(var_ref.write().is_none());
        assert_eq!(var_ref.get(), None);
    }

    #[test]
    fn generation_uniqueness() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let mut keys = Vec::new();
        for i in 0..64 {
            let var = Var::new_in(registry, i);
            keys.push(var.downgrade().get_key());
        }

        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert!(keys[i] != keys[j]);
            }
        }
    }

    #[test]
    fn drop_during_read() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 42i32);
        let var_ref = var.downgrade();

        let guard = var_ref.read().expect("should read while alive");
        assert_eq!(*guard, 42);

        drop(var);

        assert!(!var_ref.is_alive());

        drop(guard);

        assert!(var_ref.read().is_none());
        assert!(var_ref.write().is_none());
        assert_eq!(var_ref.get(), None);
    }

    #[test]
    fn drop_during_write() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 1i32);
        let var_ref = var.downgrade();

        let mut guard = var_ref.write().expect("write while alive");
        *guard = 2;

        drop(var);
        assert!(!var_ref.is_alive());

        drop(guard);

        assert!(var_ref.read().is_none());
        assert!(var_ref.write().is_none());
        assert_eq!(var_ref.get(), None);
    }

    #[test]
    fn verify_cleanup_on_read_guard_drop() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let var = Var::new_in(registry, 100);
        let var_ref_a = var.downgrade();
        let addr_a = get_slot_addr(var_ref_a);

        let guard = var_ref_a.read().unwrap();

        drop(var);

        drop(guard);

        let var_b = Var::new_in(registry, 200);
        let var_ref_b = var_b.downgrade();
        let addr_b = get_slot_addr(var_ref_b);

        assert_eq!(addr_a, addr_b);
    }

    #[test]
    fn verify_cleanup_on_write_guard_drop() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let var = Var::new_in(registry, 100);
        let var_ref_a = var.downgrade();
        let addr_a = get_slot_addr(var_ref_a);

        let guard = var_ref_a.write().unwrap();

        drop(var);

        drop(guard);

        let var_b = Var::new_in(registry, 200);
        let var_ref_b = var_b.downgrade();
        let addr_b = get_slot_addr(var_ref_b);

        assert_eq!(addr_a, addr_b);
    }

    #[test]
    fn write_guard_records_current_version_in_read_scope() {
        use crate::prelude::Var;
        use crate::reactive::Registry;

        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 0i32);
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            let mut guard = var_ref.write().unwrap();
            *guard += 1;
        });

        let key = var_ref.get_key();
        let recorded = *deps.deps.get(&key).expect("write should have been tracked");
        let actual = var_ref.get_version().unwrap();

        assert_eq!(recorded, actual);
    }

    #[test]
    fn stale_read_after_drop_recycles_slot() {
        use crate::prelude::Var;
        use std::sync::mpsc;

        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 123i32);
        let var_ref = var.downgrade();

        let slot = var_ref.slot;

        let block_writer = slot.value.write();

        let (tx_ready, rx_ready) = mpsc::channel::<()>();
        let (tx_done, rx_done) = mpsc::channel::<bool>();

        std::thread::spawn(move || {
            tx_ready.send(()).unwrap();
            let got = var_ref.read();
            tx_done.send(got.is_none()).unwrap();
        });

        rx_ready.recv().unwrap();

        drop(var);
        drop(block_writer);
        assert!(rx_done.recv().unwrap());

        let var_b = Var::new_in(registry, 999i32);
        let var_ref_b = var_b.downgrade();

        assert!(std::ptr::eq(slot, var_ref_b.slot));
    }

    #[test]
    fn cleanup_race_handoff_to_blocked_writer() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let var = Var::new_in(registry, 100);
        let var_ref = var.downgrade();
        let slot_addr = get_slot_addr(var_ref);

        let (tx_locked, rx_locked) = std::sync::mpsc::channel();
        let (tx_drop_lock, rx_drop_lock) = std::sync::mpsc::channel();
        let (tx_writer_done, rx_writer_done) = std::sync::mpsc::channel();

        std::thread::spawn({
            move || {
                let _guard = var_ref.read().unwrap();
                tx_locked.send(()).unwrap();
                rx_drop_lock.recv().unwrap();
            }
        });

        rx_locked.recv().unwrap();

        std::thread::spawn({
            move || {
                let result = var_ref.write();
                assert!(result.is_none());
                tx_writer_done.send(()).unwrap();
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(50));

        drop(var);

        tx_drop_lock.send(()).unwrap();
        rx_writer_done.recv().unwrap();

        let new_var = Var::new_in(registry, 200);
        let new_addr = get_slot_addr(new_var.downgrade());

        assert_eq!(slot_addr, new_addr);
    }

    #[test]
    fn impossible_state_safety_guards() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10);
        let var_ref = var.downgrade();

        {
            let mut guard = var.0.slot.value.write();
            *guard = None;
        }

        assert!(var_ref.read().is_none());
        assert!(var_ref.write().is_none());
    }

    #[test]
    fn registry_write_count_reports_actual_changes() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        assert_eq!(registry.write_count(), 0);

        let var = Var::new_in(registry, 0i32);
        assert_eq!(registry.write_count(), 0);

        // A real change must increment write_count.
        var.set(1);
        assert_eq!(registry.write_count(), 1);

        // Dropping a Var also increments write_count (lifecycle write).
        drop(var);
        assert_eq!(registry.write_count(), 2);
    }
}

mod concurrency {
    use super::*;

    #[test]
    fn concurrent_increments() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 0i32);
        let var_ref = var.downgrade();
        let n_threads = 10;
        let n_inc = 1_000;

        thread::scope(|scope| {
            for _ in 0..n_threads {
                scope.spawn(move || {
                    for _ in 0..n_inc {
                        if let Some(mut guard) = var_ref.write() {
                            *guard += 1;
                        }
                    }
                });
            }
        });

        assert_eq!(var_ref.get(), Some(n_threads * n_inc));
    }

    #[test]
    fn concurrent_scope_isolation() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 123i32);
        let var_ref = var.downgrade();

        let deps_main = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        let deps_other = thread::spawn(move || {
            DependencyMap::default().read_scope(|| {
                var_ref.mark_read();
            })
        })
        .join()
        .unwrap();

        assert!(deps_main.deps.contains_key(&var_ref.get_key()));
        assert!(deps_other.deps.contains_key(&var_ref.get_key()));
        assert_eq!(deps_main.deps.len(), 1);
        assert_eq!(deps_other.deps.len(), 1);
    }
}

mod dependency_map {
    use super::*;

    #[test]
    fn dependency_map_default_and_clear() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 1i32);
        let var_ref = var.downgrade();
        let key = var_ref.get_key();

        let mut deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        assert!(deps.deps.contains_key(&key));

        deps.clear();
        assert!(deps.deps.is_empty());
        assert!(!deps.any_changed());
    }

    #[test]
    fn any_changed_update_updates_versions_and_removes_dropped_vars() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 10i32);
        let var_ref = var.downgrade();
        let key = var_ref.get_key();

        let mut deps = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });

        assert!(deps.deps.contains_key(&key));
        assert_eq!(*deps.deps.get(&key).unwrap(), 0);
        assert!(!deps.any_changed());

        var_ref.set(11);
        assert!(deps.any_changed_update());
        assert_eq!(*deps.deps.get(&key).unwrap(), 1);

        assert!(!deps.any_changed());
        assert!(!deps.any_changed_update());

        drop(var);
        assert!(deps.any_changed_update());
        assert!(deps.deps.is_empty());
    }

    #[test]
    fn dependency_map_mark_read_propagates_to_active_scope() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 42i32);
        let var_ref = var.downgrade();
        let key = var_ref.get_key();

        let cached = DependencyMap::default().read_scope(|| {
            var_ref.mark_read();
        });
        assert!(cached.deps.contains_key(&key));

        let forwarded = DependencyMap::default().read_scope(|| {
            cached.mark_read();
        });

        assert!(forwarded.deps.contains_key(&key));
    }

    #[test]
    fn cleared_preserves_hashmap_capacity() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 1i32);
        let key = var.downgrade().get_key();

        let mut deps = DependencyMap::default();
        // Force allocation in the internal HashMap.
        deps.deps.insert(key, 0);

        let cap_before = deps.deps.capacity();
        assert!(cap_before > 0);

        let deps = deps.cleared();

        assert!(deps.deps.is_empty());
        let cap_after = deps.deps.capacity();

        // HashMap::clear keeps capacity, Default::default() resets to 0.
        assert!(cap_after >= cap_before, "capacity shrank: {cap_before} -> {cap_after}");
        assert!(cap_after > 0, "capacity unexpectedly reset to 0");
    }
}

mod guard_mapping {
    use super::*;
    use crate::reactive::{VarReadGuard, VarWriteGuard};

    #[test]
    fn read_guard_map_tracks_and_projects_subfield() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, (123i32, 456i32));
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            let guard = var_ref.read().unwrap();
            let mapped = VarReadGuard::map(guard, |t| &t.0);
            assert_eq!(*mapped, 123);
        });

        assert!(deps.deps.contains_key(&var_ref.get_key()));
    }

    #[test]
    fn read_guard_try_map_success_and_failure_paths() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, (1i32, 2i32));
        let var_ref = var.downgrade();

        let deps_ok = DependencyMap::default().read_scope(|| {
            let guard = var_ref.read().unwrap();
            let mapped = match VarReadGuard::try_map(guard, |t| Some(&t.1)) {
                Ok(m) => m,
                Err(_) => panic!("try_map unexpectedly failed"),
            };
            assert_eq!(*mapped, 2);
        });
        assert!(deps_ok.deps.contains_key(&var_ref.get_key()));

        let deps_err = DependencyMap::default().read_scope(|| {
            let guard = var_ref.read().unwrap();
            let res: Result<VarReadGuard<'_, i32>, VarReadGuard<'_, (i32, i32)>> = VarReadGuard::try_map(guard, |_t| None);
            assert!(res.is_err());
            let original = res.err().unwrap();
            assert_eq!(original.0, 1);
            assert_eq!(original.1, 2);
        });
        assert!(deps_err.deps.contains_key(&var_ref.get_key()));
    }

    #[test]
    fn read_guard_try_map_or_err_success_and_error_paths() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, (10i32, 20i32));
        let var_ref = var.downgrade();

        let deps_ok = DependencyMap::default().read_scope(|| {
            let guard = var_ref.read().unwrap();
            let mapped = match VarReadGuard::try_map_or_err::<i32, &'static str>(guard, |t| Ok(&t.0)) {
                Ok(m) => m,
                Err((_original, _e)) => panic!("try_map_or_err unexpectedly failed"),
            };
            assert_eq!(*mapped, 10);
        });
        assert!(deps_ok.deps.contains_key(&var_ref.get_key()));

        let deps_err = DependencyMap::default().read_scope(|| {
            let guard = var_ref.read().unwrap();
            let res = VarReadGuard::try_map_or_err::<i32, &'static str>(guard, |_t| Err("nope"));
            assert!(res.is_err());

            let (original, err) = res.err().unwrap();
            assert_eq!(err, "nope");
            assert_eq!(original.0, 10);
            assert_eq!(original.1, 20);
        });
        assert!(deps_err.deps.contains_key(&var_ref.get_key()));
    }

    #[test]
    fn write_guard_map_tracks_and_projects_subfield() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, (1i32, 2i32));
        let var_ref = var.downgrade();

        let deps = DependencyMap::default().read_scope(|| {
            let guard = var_ref.write().unwrap();
            let mut mapped = VarWriteGuard::map(guard, |t| &mut t.1);
            *mapped += 10;
        });

        assert!(deps.deps.contains_key(&var_ref.get_key()));
        assert_eq!(var_ref.get(), Some((1, 12)));
        assert_eq!(var_ref.get_version(), Some(1));
    }

    #[test]
    fn write_guard_try_map_success_and_failure_paths() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, (5i32, 6i32));
        let var_ref = var.downgrade();

        {
            let guard = var_ref.write().unwrap();
            let mut mapped = match VarWriteGuard::try_map(guard, |t| Some(&mut t.0)) {
                Ok(m) => m,
                Err(_) => panic!("try_map unexpectedly failed"),
            };
            *mapped *= 2;
        }
        assert_eq!(var_ref.get(), Some((10, 6)));
        assert_eq!(var_ref.get_version(), Some(1));

        {
            let guard = var_ref.write().unwrap();
            let res: Result<VarWriteGuard<'_, i32>, VarWriteGuard<'_, (i32, i32)>> = VarWriteGuard::try_map(guard, |_t| None);
            assert!(res.is_err());

            let mut original = res.err().unwrap();
            original.1 += 100;
        }
        assert_eq!(var_ref.get(), Some((10, 106)));
        assert_eq!(var_ref.get_version(), Some(2));
    }

    #[test]
    fn write_guard_try_map_or_err_success_and_error_paths() {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, (7i32, 8i32));
        let var_ref = var.downgrade();

        {
            let guard = var_ref.write().unwrap();
            let mut mapped = match VarWriteGuard::try_map_or_err::<i32, &'static str>(guard, |t| Ok(&mut t.1)) {
                Ok(m) => m,
                Err((_original, _e)) => panic!("try_map_or_err unexpectedly failed"),
            };
            *mapped += 1;
        }
        assert_eq!(var_ref.get(), Some((7, 9)));
        assert_eq!(var_ref.get_version(), Some(1));

        {
            let guard = var_ref.write().unwrap();
            let res = VarWriteGuard::try_map_or_err::<i32, &'static str>(guard, |_t| Err("bad"));
            assert!(res.is_err());

            let (mut original, err) = res.err().unwrap();
            assert_eq!(err, "bad");
            original.0 += 10;
        }
        assert_eq!(var_ref.get(), Some((17, 9)));
        assert_eq!(var_ref.get_version(), Some(2));
    }
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;
    use crate::reactive::serde_impl::serde_scope;
    use serde::de::{self, Deserializer};
    use serde::{Deserialize, Serialize};

    #[test]
    #[should_panic]
    fn nested_scopes() {
        serde_scope(|| {
            serde_scope(|| {});
        });
    }

    #[test]
    fn basic_roundtrip() {
        serde_scope(|| {
            let var = Var::new(String::from("payload"));
            let serialized = serde_json::to_string(&var).unwrap();
            let deserialized: Var<String> = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized.get().as_str(), "payload");
        });
    }

    #[test]
    fn var_serialize_fails_without_scope() {
        let var = Var::new(10);
        let result = serde_json::to_string(&var);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("outside of a scope"));
    }

    #[test]
    fn var_deserialize_fails_without_scope() {
        let json = "[1, 10]";
        let result = serde_json::from_str::<Var<i32>>(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("outside of a scope"));
    }

    #[test]
    fn weakvar_serialize_fails_without_scope() {
        let var = Var::new(10);
        let var_ref = var.downgrade();

        let result = serde_json::to_string(&var_ref);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("outside of a scope"));
    }

    #[test]
    fn weakvar_deserialize_fails_on_invalid_id_type() {
        serde_scope(|| {
            let json = "\"not-a-u64\"";
            let result = serde_json::from_str::<WeakVar<i32>>(json);

            assert!(result.is_err());
            assert!(result.unwrap_err().is_data());
        });
    }

    #[test]
    fn weakvar_deserialize_without_var() {
        #[derive(Serialize, Deserialize)]
        struct Graph {
            var_ref: WeakVar<i32>,
        }

        let var = Var::new(789);
        let graph = Graph { var_ref: var.downgrade() };

        serde_scope(|| {
            let serialized = serde_json::to_string(&graph).unwrap();
            let deserialized: Graph = serde_json::from_str(&serialized).unwrap();

            assert!(!deserialized.var_ref.is_alive());
        });
    }

    #[test]
    fn weakvar_deserialize_fails_without_scope() {
        let json = "1";
        let result = serde_json::from_str::<WeakVar<i32>>(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("outside of a scope"));
    }

    #[test]
    fn serialization_tracks_dependency() {
        let var = Var::new(10);

        let deps = DependencyMap::default().read_scope(|| {
            serde_scope(|| {
                let _ = serde_json::to_string(&var);
            });
        });

        assert!(deps.deps.contains_key(&var.downgrade().get_key()));
    }

    #[test]
    fn graph_integrity_var_and_ref() {
        #[derive(Serialize, Deserialize)]
        struct Graph {
            var: Var<i32>,
            var_ref: WeakVar<i32>,
        }

        serde_scope(|| {
            let owner = Var::new(123);
            let handle = owner.downgrade();

            let original = Graph { var: owner, var_ref: handle };
            let serialized = serde_json::to_string(&original).unwrap();
            let deserialized: Graph = serde_json::from_str(&serialized).unwrap();

            assert_eq!(deserialized.var.get(), 123);
            assert_eq!(deserialized.var_ref.get(), Some(123));

            deserialized.var.downgrade().set(456);
            assert_eq!(deserialized.var_ref.get(), Some(456));

            assert_eq!(deserialized.var.downgrade(), deserialized.var_ref);
        });
    }

    #[test]
    fn shared_references() {
        #[derive(Serialize, Deserialize)]
        struct SharedHandles {
            owner: Var<i32>,
            ref1: WeakVar<i32>,
            ref2: WeakVar<i32>,
        }

        serde_scope(|| {
            let owner = Var::new(99);
            let ref1 = owner.downgrade();
            let ref2 = owner.downgrade();

            let original = SharedHandles { owner, ref1, ref2 };
            let serialized = serde_json::to_string(&original).unwrap();
            let deserialized: SharedHandles = serde_json::from_str(&serialized).unwrap();

            assert_eq!(deserialized.ref1, deserialized.ref2);
            assert_eq!(deserialized.ref1.get(), Some(99));

            deserialized.ref1.set(456);
            assert_eq!(deserialized.ref2.get(), Some(456));
        });
    }

    #[test]
    fn dead_references_serialize_as_dead() {
        #[derive(Serialize, Deserialize)]
        struct Container {
            var_ref: WeakVar<i32>,
        }

        serde_scope(|| {
            let var = Var::new(10);
            let var_ref = var.downgrade();
            drop(var);

            let input = Container { var_ref };
            let serialized = serde_json::to_string(&input).unwrap();
            let deserialized: Container = serde_json::from_str(&serialized).unwrap();

            assert!(!deserialized.var_ref.is_alive());
            assert_eq!(deserialized.var_ref.get(), None);
        });
    }

    #[test]
    fn deserialize_dead_var() {
        serde_scope(|| {
            let json = "[0, null]";
            let var: Var<i32> = serde_json::from_str(json).unwrap();
            assert!(!var.downgrade().is_alive());
            assert_eq!(var.downgrade().get(), None);
        });
    }

    #[test]
    fn deserialization_error_propagation() {
        #[derive(Debug)]
        struct FailDeserializer;

        impl<'de> Deserialize<'de> for FailDeserializer {
            fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Err(de::Error::custom("intentional failure"))
            }
        }

        serde_scope(|| {
            let json = "[1, 0]";
            let result = serde_json::from_str::<Var<FailDeserializer>>(json);

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("intentional failure"));
        });
    }

    #[test]
    fn unavailable_error() {
        serde_scope(|| {
            let var = Var::new(42);
            var.0.slot.generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let serialized = serde_json::to_string(&var).unwrap();
            assert_eq!(serialized, "[0,null]");
        });
    }

    #[test]
    fn scope_cleans_up_context_on_exit() {
        serde_scope(|| {
            let _ = Var::new(1);
        });

        let var = Var::new(10);
        let result = serde_json::to_string(&var);

        assert!(result.is_err(), "Serialization succeeded outside scope! The SerdeContext leaked.");
    }

    #[test]
    fn distinct_vars_have_unique_identities() {
        #[derive(Serialize, Deserialize)]
        struct TwoVars {
            a: Var<i32>,
            b: Var<i32>,
        }

        serde_scope(|| {
            let a = Var::new(10);
            let b = Var::new(20);

            let json = serde_json::to_string(&TwoVars { a, b }).unwrap();
            let result: TwoVars = serde_json::from_str(&json).unwrap();

            assert_eq!(result.a.get(), 10, "Var 'a' was overwritten by 'b' (ID collision)!");
            assert_eq!(result.b.get(), 20, "Var 'b' deserialized as dead/empty!");

            result.a.downgrade().set(99);
            assert_eq!(result.b.get(), 20);
        });
    }

    #[test]
    fn cleanup_uninitialized_recycles_and_prevents_aba_resurrection() {
        // Deserialize a WeakVar with a non-zero id, but never deserialize its owning Var.
        // This creates a dep_map entry that must be cleaned up by cleanup_uninitialized.
        let weak: WeakVar<i32> = serde_scope(|| serde_json::from_str("1").unwrap());

        // After scope exit, it must definitely be dead.
        assert!(!weak.is_alive());
        assert_eq!(weak.get(), None);

        // Capture the slot address so we can verify it's recycled.
        let addr = get_slot_addr(weak);

        // Now allocate Vars until we see that same slot address re-used.
        // This must happen if cleanup_uninitialized correctly recycles the slot.
        let mut found_reuse = false;

        for i in 0..4096 {
            let v = Var::new(i);
            let v_addr = get_slot_addr(v.downgrade());

            if v_addr == addr {
                found_reuse = true;

                // Even after the slot gets re-used, the old handle must stay dead.
                // If cleanup_uninitialized used a bad generation update, this can resurrect.
                assert!(!weak.is_alive());
                assert_eq!(weak.get(), None);

                drop(v);
                break;
            }

            drop(v);
        }

        assert!(found_reuse, "Expected the uninitialized slot to be recycled and re-used");
    }
}
