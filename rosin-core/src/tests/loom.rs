use crate::reactive::{DependencyMap, Registry, Var};
use crate::sync::*;

const PREEMPTION_BOUND: Option<usize> = None;

struct CountDrop(Arc<AtomicUsize>);

impl Drop for CountDrop {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn write_count_polling() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let var = Var::new_in(registry, 0u32);
        let weak = var.downgrade();
        let deps = DependencyMap::default().read_scope(|| {
            assert_eq!(var.get(), 0);
        });
        let mut last_write_count = registry.write_count();
        let mut update_requested = false;

        let writer = loom::thread::spawn(move || {
            *weak.write().unwrap() = 1;
        });

        let mut poll = || {
            let current_write_count = registry.write_count();
            if current_write_count != last_write_count {
                last_write_count = current_write_count;
                update_requested |= deps.any_changed();
            }
        };

        poll();
        writer.join().unwrap();
        poll();

        assert!(deps.any_changed());
        assert!(update_requested);
    });
}

#[test]
fn no_deadlock_cross_var_writes() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let a = Var::new_in(registry_ref, 0u32);
        let b = Var::new_in(registry_ref, 0u32);
        let a_ref = a.downgrade();
        let b_ref = b.downgrade();

        let t1 = loom::thread::spawn(move || {
            if let Some(mut guard) = a_ref.write() {
                *guard += 1;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = b_ref.write() {
                *guard += 1;
            }
        });

        let t2 = loom::thread::spawn(move || {
            if let Some(mut guard) = b_ref.write() {
                *guard += 1;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = a_ref.write() {
                *guard += 1;
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(a.get(), 2);
        assert_eq!(b.get(), 2);
    });
}

#[test]
fn read_scope_tracks_reads_under_race() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let a = Var::new_in(registry_ref, 0u32);
        let b = Var::new_in(registry_ref, 10u32);
        let a_ref = a.downgrade();
        let b_ref = b.downgrade();

        let aw = a.downgrade();
        let writer = loom::thread::spawn(move || {
            if let Some(mut guard) = aw.write() {
                *guard = 1;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = aw.write() {
                *guard = 2;
            }
        });

        let reader = loom::thread::spawn(move || {
            DependencyMap::default().read_scope(|| {
                let a_now = a_ref.get().unwrap();
                if a_now == 0 {
                    let _ = b_ref.get();
                }
            })
        });

        writer.join().unwrap();
        let deps = reader.join().unwrap();

        let a_key = a.downgrade().get_key();
        let b_key = b.downgrade().get_key();

        assert!(deps.deps.get(&a_key).is_some());
        let _ = deps.deps.get(&b_key);

        let count = deps.deps.len();
        assert!(count == 1 || count == 2, "unexpected deps size: {count}");
    });
}

#[test]
fn versions_change_detection_monotonic() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var = Var::new_in(registry_ref, 0u32);
        let var_ref = var.downgrade();

        let mut deps = DependencyMap::default().read_scope(|| {
            let _ = var_ref.read();
        });

        let t1 = loom::thread::spawn(move || {
            if let Some(mut guard) = var_ref.write() {
                *guard = 1;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = var_ref.write() {
                *guard = 2;
            }
        });

        let t2 = loom::thread::spawn(move || {
            if let Some(mut guard) = var_ref.write() {
                *guard += 1;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = var_ref.write() {
                *guard += 1;
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert!(deps.any_changed());

        // Collect a fresh snapshot.
        deps = deps.cleared().read_scope(|| {
            let _ = var_ref.read();
        });
        assert!(!deps.any_changed());

        if let Some(mut guard) = var.downgrade().write() {
            *guard = 999;
        }
        assert!(deps.any_changed());
    });
}

#[test]
fn get_version_does_not_return_reused_slot_version() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));
        let var = Var::new_in(registry, 0u32);
        let weak = var.downgrade();

        let reader = loom::thread::spawn(move || weak.get_version());

        drop(var);
        let replacement = Var::new_in(registry, 10u32);
        assert!(std::ptr::eq(weak.slot, replacement.downgrade().slot));
        replacement.set(11);

        let observed = reader.join().unwrap();
        assert!(matches!(observed, None | Some(0)));
    });
}

#[test]
fn drop_makes_weakvar_dead_and_accessors_return_none() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var = Var::new_in(registry_ref, 123u32);
        let var_ref = var.downgrade();

        let t_drop = loom::thread::spawn(move || {
            loom::thread::yield_now();
            drop(var);
        });

        let t_read = loom::thread::spawn(move || {
            let _ = var_ref.get();
            loom::thread::yield_now();
        });

        t_drop.join().unwrap();
        t_read.join().unwrap();

        assert!(!var_ref.is_alive());
        assert!(var_ref.get().is_none());
        assert!(var_ref.read().is_none());
    });
}

#[test]
fn drop_while_read_guard_held_no_panic_and_cleans_up() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var = Var::new_in(registry_ref, 7u32);
        let weak = var.downgrade();

        let stage = Arc::new(AtomicUsize::new(0));

        let stage_a = stage.clone();
        let weak_a = weak;
        let t_hold = loom::thread::spawn(move || {
            let guard = weak_a.read().unwrap();
            stage_a.store(1, Ordering::SeqCst);

            for _ in 0..5 {
                loom::thread::yield_now();
                let _ = *guard;
            }

            drop(guard);
            stage_a.store(2, Ordering::SeqCst);
        });

        let stage_b = stage.clone();
        let t_drop = loom::thread::spawn(move || {
            while stage_b.load(Ordering::SeqCst) != 1 {
                loom::thread::yield_now();
            }
            drop(var);
            while stage_b.load(Ordering::SeqCst) != 2 {
                loom::thread::yield_now();
            }
        });

        t_hold.join().unwrap();
        t_drop.join().unwrap();

        assert!(!weak.is_alive());
        assert!(weak.get().is_none());
        assert!(weak.read().is_none());
    });
}

#[test]
fn drop_while_write_guard_held_no_panic() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var = Var::new_in(registry_ref, 0u32);
        let weak = var.downgrade();

        let stage = Arc::new(AtomicUsize::new(0));

        let stage_a = stage.clone();
        let weak_a = weak;
        let t_hold = loom::thread::spawn(move || {
            let mut guard = weak_a.write().unwrap();
            *guard += 1;
            stage_a.store(1, Ordering::SeqCst);

            for _ in 0..5 {
                loom::thread::yield_now();
                *guard += 1;
            }

            drop(guard);
            stage_a.store(2, Ordering::SeqCst);
        });

        let stage_b = stage.clone();
        let t_drop = loom::thread::spawn(move || {
            while stage_b.load(Ordering::SeqCst) != 1 {
                loom::thread::yield_now();
            }
            drop(var);
            while stage_b.load(Ordering::SeqCst) != 2 {
                loom::thread::yield_now();
            }
        });

        t_hold.join().unwrap();
        t_drop.join().unwrap();

        assert!(!weak.is_alive());
        assert!(weak.get().is_none());
        assert!(weak.read().is_none());
    });
}

#[test]
fn stale_weakvar_read_does_not_prevent_reused_slot_cleanup() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let original = Var::new_in(registry, 0u32);
        let stale = original.downgrade();
        drop(original);

        let drops = Arc::new(AtomicUsize::new(0));
        let replacement = Var::new_in(registry, CountDrop(drops.clone()));
        assert!(std::ptr::eq(stale.slot, replacement.slot));

        let reader = loom::thread::spawn(move || {
            assert!(stale.read().is_none());
        });

        let owner = loom::thread::spawn(move || drop(replacement));

        reader.join().unwrap();
        owner.join().unwrap();

        assert_eq!(drops.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn read_started_before_reuse_does_not_leak() {
    let mut model = loom::model::Builder::new();
    model.preemption_bound = Some(3);
    model.check(|| {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let original = Var::new_in(registry, 0u32);
        let weak = original.downgrade();

        let drops = Arc::new(AtomicUsize::new(0));

        let reader = loom::thread::spawn(move || {
            drop(weak.read());
        });

        let counter = drops.clone();
        let owner = loom::thread::spawn(move || {
            drop(original);
            let replacement = Var::new_in(registry, CountDrop(counter));
            drop(replacement);
        });

        reader.join().unwrap();
        owner.join().unwrap();

        assert_eq!(drops.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn racing_read_guard_drop_cleans_up_value() {
    let mut model = loom::model::Builder::new();
    model.preemption_bound = Some(3);
    model.check(|| {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let drops = Arc::new(AtomicUsize::new(0));
        let original = Var::new_in(registry, CountDrop(drops.clone()));
        let weak = original.downgrade();

        let reader = loom::thread::spawn(move || {
            drop(weak.read());
        });

        let owner = loom::thread::spawn(move || {
            drop(original);
        });

        reader.join().unwrap();
        owner.join().unwrap();

        assert_eq!(drops.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn racing_write_guard_drop_cleans_up_value() {
    let mut model = loom::model::Builder::new();
    model.preemption_bound = Some(3);
    model.check(|| {
        let registry: &'static Registry = Box::leak(Box::new(Registry::default()));

        let drops = Arc::new(AtomicUsize::new(0));
        let original = Var::new_in(registry, CountDrop(drops.clone()));
        let weak = original.downgrade();

        let reader = loom::thread::spawn(move || {
            drop(weak.write());
        });

        let owner = loom::thread::spawn(move || {
            drop(original);
        });

        reader.join().unwrap();
        owner.join().unwrap();

        assert_eq!(drops.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn slot_reuse_does_not_confuse_dependency_keys() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var1 = Var::new_in(registry_ref, 1u32);
        let w1 = var1.downgrade();
        let key1 = w1.get_key();

        let deps1 = DependencyMap::default().read_scope(|| {
            let _ = w1.read();
        });

        assert!(deps1.deps.get(&key1).is_some());

        drop(var1);

        assert!(deps1.any_changed());

        let var2 = Var::new_in(registry_ref, 2u32);
        let w2 = var2.downgrade();

        if let Some(mut g) = w2.write() {
            *g = 3;
        }

        assert_ne!(key1, w2.get_key());
        assert!(deps1.any_changed());
    });
}

#[test]
fn nested_scopes_record_into_both_maps() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let a = Var::new_in(registry_ref, 1u32);
        let b = Var::new_in(registry_ref, 2u32);

        let a_ref = a.downgrade();
        let b_ref = b.downgrade();

        let a_key = a_ref.get_key();
        let b_key = b_ref.get_key();

        let reader = loom::thread::spawn(move || {
            let mut inner_out: Option<DependencyMap> = None;

            let deps_outer = DependencyMap::default().read_scope(|| {
                let _ = a_ref.read();
                loom::thread::yield_now();

                let deps_inner = DependencyMap::default().read_scope(|| {
                    let _ = b_ref.read();
                    loom::thread::yield_now();
                });

                inner_out = Some(deps_inner);

                loom::thread::yield_now();
                let _ = a_ref.read();
            });

            let deps_inner = inner_out.expect("inner scope did not run");
            (deps_outer, deps_inner)
        });

        let writer = loom::thread::spawn(move || {
            if let Some(mut guard) = a_ref.write() {
                *guard += 1;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = b_ref.write() {
                *guard += 1;
            }
        });

        let (deps_outer, deps_inner) = reader.join().unwrap();
        writer.join().unwrap();

        assert!(deps_outer.deps.get(&a_key).is_some());
        assert!(deps_outer.deps.get(&b_key).is_some());
        assert!(deps_inner.deps.get(&b_key).is_some());
        assert!(deps_inner.deps.get(&a_key).is_none());
    });
}

#[test]
fn read_scope_panics_do_not_corrupt_stack() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let a = Var::new_in(registry_ref, 1u32);
        let b = Var::new_in(registry_ref, 2u32);

        let a_ref = a.downgrade();
        let b_ref = b.downgrade();

        let a_key = a_ref.get_key();
        let b_key = b_ref.get_key();

        let stage = Arc::new(AtomicUsize::new(0));

        let stage_p = stage.clone();
        let panics = loom::thread::spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = DependencyMap::default().read_scope(|| {
                    let _ = a_ref.read();
                    stage_p.store(1, Ordering::SeqCst);
                    loom::thread::yield_now();
                    panic!("boom");
                });
            }));
            stage_p.store(2, Ordering::SeqCst);
        });

        let stage_w = stage.clone();
        let worker = loom::thread::spawn(move || {
            while stage_w.load(Ordering::SeqCst) == 0 {
                loom::thread::yield_now();
            }

            for _ in 0..3 {
                if let Some(mut g) = b_ref.write() {
                    *g += 1;
                }
                loom::thread::yield_now();
            }

            while stage_w.load(Ordering::SeqCst) != 2 {
                loom::thread::yield_now();
            }
        });

        panics.join().unwrap();
        worker.join().unwrap();

        let deps = DependencyMap::default().read_scope(|| {
            let _ = b_ref.read();
        });

        assert!(deps.deps.get(&b_key).is_some());
        assert!(deps.deps.get(&a_key).is_none());
    });
}

#[test]
fn reader_sees_coherent_state_monotonic_values() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var = Var::new_in(registry_ref, 0u32);
        let var_ref = var.downgrade();

        let writer = loom::thread::spawn(move || {
            if let Some(mut guard) = var_ref.write() {
                *guard = 1;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = var_ref.write() {
                *guard = 2;
            }
            loom::thread::yield_now();
            if let Some(mut guard) = var_ref.write() {
                *guard = 3;
            }
        });

        let reader = loom::thread::spawn(move || {
            let mut last = 0u32;
            for _ in 0..5 {
                if let Some(now) = var_ref.get() {
                    assert!(now >= last, "observed decreasing value: {now} < {last}");
                    assert!(now <= 3, "observed value beyond writer's max");
                    last = now;
                }
                loom::thread::yield_now();
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    });
}

#[test]
fn read_scope_does_not_mask_racing_writes() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var = Var::new_in(registry_ref, 0u32);
        let var_ref = var.downgrade();

        let stage = Arc::new(AtomicUsize::new(0));
        let observed = Arc::new(AtomicUsize::new(usize::MAX));

        // Reader: enters a scope, reads the var, records what it saw, then signals writer.
        let reader_stage = stage.clone();
        let reader_observed = observed.clone();
        let reader_ref = var_ref;
        let reader = loom::thread::spawn(move || {
            let deps = DependencyMap::default().read_scope(|| {
                if let Some(guard) = reader_ref.read() {
                    reader_observed.store(*guard as usize, Ordering::SeqCst);
                    reader_stage.store(1, Ordering::SeqCst);
                }
            });

            // Wait for writer to complete before returning deps.
            while reader_stage.load(Ordering::SeqCst) != 2 {
                loom::thread::yield_now();
            }

            deps
        });

        // Writer: wait until reader has performed the read inside the scope, then write.
        let writer_stage = stage.clone();
        let writer_ref = var_ref;
        let writer = loom::thread::spawn(move || {
            while writer_stage.load(Ordering::SeqCst) != 1 {
                loom::thread::yield_now();
            }

            if let Some(mut guard) = writer_ref.write() {
                *guard = 1u32;
            }

            writer_stage.store(2, Ordering::SeqCst);
        });

        let deps = reader.join().unwrap();
        writer.join().unwrap();

        // Reader must have seen the old value.
        assert_eq!(observed.load(Ordering::SeqCst) as u32, 0);

        // The dependency snapshot must reflect the read that actually happened, not the later write.
        let key = var.downgrade().get_key();
        let recorded = deps.deps.get(&key).copied().expect("dependency missing");
        assert_eq!(recorded, 0, "read_scope recorded post-write version");

        // And change detection must still notice the write.
        assert!(deps.any_changed(), "change detection masked racing write");
    });
}

#[test]
fn no_deadlock_two_writers_same_var() {
    let mut model = loom::model::Builder::default();
    model.preemption_bound = PREEMPTION_BOUND;
    model.check(|| {
        let registry = Box::new(Registry::default());
        let registry_ref: &'static Registry = unsafe { &*(registry.as_ref() as *const Registry) };

        let var = Var::new_in(registry_ref, 0u32);
        let var_ref = var.downgrade();

        let t1 = loom::thread::spawn(move || {
            if let Some(mut guard) = var_ref.write() {
                *guard += 1;
            }
        });

        let t2 = loom::thread::spawn(move || {
            if let Some(mut guard) = var_ref.write() {
                *guard += 2;
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(var.get(), 3);
    });
}
