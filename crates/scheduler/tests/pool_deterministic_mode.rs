//! M0-B04 acceptance: proves `PoolMode::Deterministic` (TEST-D17) never
//! resizes under either heavy synthetic backlog or sustained idleness, and
//! that `RcWorkerPool::new(n)` yields exactly `n` workers for the pool's
//! entire lifetime, independent of `baseline`/`hard_cap`.

use std::sync::{Arc, Condvar, Mutex};

use rc_scheduler::pool::RcWorkerPool;

#[test]
fn deterministic_mode_never_resizes() {
    let pool = RcWorkerPool::new(3);
    assert_eq!(pool.worker_count(), 3);

    let gate: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
    for _ in 0..3 {
        let gate = Arc::clone(&gate);
        pool.spawn(move || {
            let (lock, cvar) = &*gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = cvar.wait(released).unwrap();
            }
        });
    }
    for _ in 0..10_000 {
        pool.spawn(|| {});
    }

    for _ in 0..500 {
        pool.sample_and_maybe_resize();
        assert_eq!(pool.worker_count(), 3);
    }

    let (lock, cvar) = &*gate;
    *lock.lock().unwrap() = true;
    cvar.notify_all();
    pool.wait_idle();

    for _ in 0..500 {
        pool.sample_and_maybe_resize();
        assert_eq!(
            pool.worker_count(),
            3,
            "must never shrink, even though 3 exceeds what a baseline would be in Elastic mode"
        );
    }
}
