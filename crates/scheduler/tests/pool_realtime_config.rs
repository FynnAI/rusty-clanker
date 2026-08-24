//! M0-B04 acceptance: proves PERF-D55's `SCHED_RR` opt-in never panics and
//! the pool stays fully functional regardless of whether the underlying
//! `sched_setscheduler` syscall actually succeeds (the expected outcome on
//! an unprivileged CI runner lacking `CAP_SYS_NICE` is `EPERM`, handled as a
//! graceful fallback to `SCHED_OTHER`). Linux-only: compiles to zero test
//! cases on every other target, which is expected, not a failure.
#![cfg(target_os = "linux")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rc_scheduler::pool::{PoolMode, RcWorkerPool, RcWorkerPoolConfig, RealtimeConfig};

#[test]
fn realtime_opt_in_never_panics_and_pool_still_works() {
    let pool = RcWorkerPool::with_config(RcWorkerPoolConfig {
        baseline: 2,
        hard_cap: 2,
        mode: PoolMode::Elastic { auto_sample: false },
        realtime: RealtimeConfig {
            enabled: true,
            priority: 15,
        },
    });

    let counter = Arc::new(AtomicUsize::new(0));
    for _ in 0..50 {
        let counter = Arc::clone(&counter);
        pool.spawn(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
    }
    pool.wait_idle();

    assert_eq!(counter.load(Ordering::SeqCst), 50);
}
