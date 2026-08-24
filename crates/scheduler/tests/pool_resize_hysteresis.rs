//! M0-B04 acceptance: proves ARCH-D19's exact grow/shrink hysteresis
//! thresholds (3rd-not-2nd consecutive over-threshold sample to grow,
//! 100th-not-99th consecutive idle sample to shrink) and the `hard_cap`/
//! `baseline` bounds, driven deterministically via `auto_sample: false` +
//! explicit `sample_and_maybe_resize()` calls (no reliance on wall-clock
//! timing).

use std::sync::{Arc, Condvar, Mutex};

use rc_scheduler::pool::{PoolMode, RcWorkerPool, RcWorkerPoolConfig, RealtimeConfig};

/// Occupies every currently-live worker with a job parked on a shared gate,
/// so backlog accumulates across samples instead of draining between them.
/// Returns the gate; call `release` on it once the test no longer needs the
/// workers blocked, then `pool.wait_idle()` to let the parked jobs (and
/// whatever backlog they were blocking) drain and join implicitly.
fn block_workers(pool: &RcWorkerPool, n: usize) -> Arc<(Mutex<bool>, Condvar)> {
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    for _ in 0..n {
        let gate = Arc::clone(&gate);
        pool.spawn(move || {
            let (lock, cvar) = &*gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = cvar.wait(released).unwrap();
            }
        });
    }
    gate
}

fn release(gate: &Arc<(Mutex<bool>, Condvar)>) {
    let (lock, cvar) = &**gate;
    *lock.lock().unwrap() = true;
    cvar.notify_all();
}

fn push_noop_jobs(pool: &RcWorkerPool, n: usize) {
    for _ in 0..n {
        pool.spawn(|| {});
    }
}

#[test]
fn grow_fires_on_third_not_second_consecutive_sample() {
    let pool = RcWorkerPool::with_config(RcWorkerPoolConfig {
        baseline: 2,
        hard_cap: 6,
        mode: PoolMode::Elastic { auto_sample: false },
        realtime: RealtimeConfig::default(),
    });
    assert_eq!(pool.worker_count(), 2);

    let gate = block_workers(&pool, 2);
    push_noop_jobs(&pool, 20);

    pool.sample_and_maybe_resize();
    assert_eq!(pool.worker_count(), 2, "1st over-threshold sample");

    pool.sample_and_maybe_resize();
    assert_eq!(pool.worker_count(), 2, "2nd over-threshold sample");

    pool.sample_and_maybe_resize();
    assert_eq!(
        pool.worker_count(),
        3,
        "3rd consecutive over-threshold sample must fire the grow"
    );

    release(&gate);
    pool.wait_idle();
}

#[test]
fn grow_never_exceeds_hard_cap() {
    let pool = RcWorkerPool::with_config(RcWorkerPoolConfig {
        baseline: 2,
        hard_cap: 3,
        mode: PoolMode::Elastic { auto_sample: false },
        realtime: RealtimeConfig::default(),
    });

    let gate = block_workers(&pool, 2);
    push_noop_jobs(&pool, 100);

    for i in 0..20 {
        pool.sample_and_maybe_resize();
        if i >= 10 {
            assert_eq!(
                pool.worker_count(),
                3,
                "must stay pinned at hard_cap, never exceed it (sample {i})"
            );
        }
    }

    release(&gate);
    pool.wait_idle();
}

#[test]
fn shrink_fires_on_100th_not_99th_consecutive_idle_sample() {
    let pool = RcWorkerPool::with_config(RcWorkerPoolConfig {
        baseline: 1,
        hard_cap: 4,
        mode: PoolMode::Elastic { auto_sample: false },
        realtime: RealtimeConfig::default(),
    });
    assert_eq!(pool.worker_count(), 1);

    // Grow the pool to size 2 (reusing the grow test's own pattern), then
    // drain the setup backlog before measuring shrink hysteresis.
    let gate = block_workers(&pool, 1);
    push_noop_jobs(&pool, 20);
    pool.sample_and_maybe_resize();
    pool.sample_and_maybe_resize();
    pool.sample_and_maybe_resize();
    assert_eq!(pool.worker_count(), 2);
    release(&gate);
    pool.wait_idle();

    // This call's outcome is not asserted: it only absorbs the nonzero
    // steal count the setup backlog's real draining produced, resetting
    // each worker's idle streak to a clean 0 baseline.
    pool.sample_and_maybe_resize();

    for i in 0..99 {
        pool.sample_and_maybe_resize();
        assert_eq!(
            pool.worker_count(),
            2,
            "must not shrink before the 100th consecutive idle sample (sample {i})"
        );
    }

    pool.sample_and_maybe_resize();
    assert_eq!(
        pool.worker_count(),
        1,
        "100th consecutive idle sample must fire the shrink"
    );
}

#[test]
fn shrink_never_goes_below_baseline() {
    let pool = RcWorkerPool::with_config(RcWorkerPoolConfig {
        baseline: 2,
        hard_cap: 4,
        mode: PoolMode::Elastic { auto_sample: false },
        realtime: RealtimeConfig::default(),
    });
    assert_eq!(pool.worker_count(), 2);

    for i in 0..200 {
        pool.sample_and_maybe_resize();
        assert_eq!(
            pool.worker_count(),
            2,
            "must never shrink below baseline (sample {i})"
        );
    }
}
