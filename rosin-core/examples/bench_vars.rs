use std::hint::black_box;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use rosin_core::prelude::*;

fn run_for(label: &str, dur: Duration, mut f: impl FnMut()) {
    // Warmup
    let warm = Duration::from_millis(250);
    let warm_end = Instant::now() + warm;
    while Instant::now() < warm_end {
        f();
    }

    // Timed run
    let start = Instant::now();
    let end = start + dur;

    let mut iters: u64 = 0;
    while Instant::now() < end {
        f();
        iters += 1;
    }

    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let ops_per_sec = (iters as f64) / secs;
    let ns_per_op = (elapsed.as_nanos() as f64) / (iters as f64);

    println!("{label:<40}  {:>12.3} Mops/s   {:>10.2} ns/op   (iters ={:>10})", ops_per_sec / 1e6, ns_per_op, iters);
}

/// Spawns threads that continuously call `w.read()` until `stop` is set.
/// Uses a barrier so the caller can synchronize start.
fn spawn_read_hammer(w: WeakVar<u64>, readers: usize, stop: Arc<AtomicBool>, start_barrier: Arc<Barrier>) -> Vec<thread::JoinHandle<()>> {
    (0..readers)
        .map(|_| {
            let stop = stop.clone();
            let start_barrier = start_barrier.clone();
            thread::spawn(move || {
                // Wait for all threads to be ready.
                start_barrier.wait();

                while !stop.load(Ordering::Relaxed) {
                    if let Some(g) = w.read() {
                        black_box(*g);
                    }
                    // slight backoff to reduce pure-core saturation effects
                    std::hint::spin_loop();
                }
            })
        })
        .collect()
}

fn run_contention_suite(label_prefix: &str, w: WeakVar<u64>, dur: Duration, readers: usize) {
    let stop = Arc::new(AtomicBool::new(false));
    let start_barrier = Arc::new(Barrier::new(readers + 1));

    let handles = spawn_read_hammer(w, readers, stop.clone(), start_barrier.clone());

    // Release the hammers at the same time as the timed benchmark starts
    start_barrier.wait();

    // Let them get hot briefly to measure steady-state contention
    thread::sleep(Duration::from_millis(50));

    // Read under contention (main thread read while readers read)
    run_for(&format!("{label_prefix}read() under {readers} reader threads"), dur, || {
        let g = w.read().unwrap();
        black_box(*g);
    });

    // Write under contention (main thread write while readers read)
    run_for(&format!("{label_prefix}write(changed) under {readers} reader threads"), dur, || {
        let mut g = w.write().unwrap();
        *g = g.wrapping_add(1);
    });

    // Stop threads and join.
    stop.store(true, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }
}

fn main() {
    let dur = Duration::from_secs(2);

    let v = Var::new(123u64);
    let w: WeakVar<u64> = v.downgrade();

    println!("Running ops/sec microbench (run with --release)\n");

    run_for("WeakVar::is_alive", dur, || {
        black_box(w.is_alive());
    });

    run_for("WeakVar::get_version", dur, || {
        black_box(w.get_version());
    });

    run_for("WeakVar::read (acquire+drop)", dur, || {
        let g = w.read().unwrap();
        black_box(*g);
    });

    run_for("WeakVar::write (changed)", dur, || {
        let mut g = w.write().unwrap();
        *g = g.wrapping_add(1);
    });

    run_for("WeakVar::write (cancel_change)", dur, || {
        let mut g = w.write().unwrap();
        g.cancel_change();
        black_box(&*g);
    });

    run_for("WeakVar::get (clone u64)", dur, || {
        black_box(w.get().unwrap());
    });

    let mut t = 0u64;
    run_for("WeakVar::set (changed)", dur, || {
        t = t.wrapping_add(1);
        w.set(black_box(t)).unwrap();
    });

    let cur = w.get().unwrap();
    run_for("WeakVar::set (unchanged)", dur, || {
        w.set(cur).unwrap();
    });

    run_for("WeakVar::replace", dur, || {
        black_box(w.replace(black_box(777u64)).unwrap());
    });

    run_for("WeakVar::take", dur, || {
        w.set(123).unwrap();
        black_box(w.take().unwrap());
    });

    run_for("DependencyMap::read_scope (2 reads)", dur, || {
        let deps = DependencyMap::default().read_scope(|| {
            let g1 = w.read().unwrap();
            black_box(*g1);
            let g2 = w.read().unwrap();
            black_box(*g2);
        });
        black_box(deps);
    });

    println!("\n--- High Contention (background threads hammer read()) ---\n");
    for &readers in &[1usize, 2, 4] {
        run_contention_suite("", w, dur, readers);
        println!();
    }
}
