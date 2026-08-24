//! M0-B04 acceptance: proves `RcWorkerPool`'s work-stealing dispatch loses
//! and duplicates zero jobs across a large batch under a fixed worker count.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rc_scheduler::pool::RcWorkerPool;

#[test]
fn steal_correctness_under_load_no_loss_no_duplication() {
    let pool = RcWorkerPool::new(4);

    const N: usize = 50_000;
    let seen: Arc<Vec<AtomicBool>> = Arc::new((0..N).map(|_| AtomicBool::new(false)).collect());
    let duplicates: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let completed: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

    for i in 0..N {
        let seen = Arc::clone(&seen);
        let duplicates = Arc::clone(&duplicates);
        let completed = Arc::clone(&completed);
        pool.spawn(move || {
            if seen[i].swap(true, Ordering::SeqCst) {
                duplicates.lock().unwrap().push(i);
            }
            completed.fetch_add(1, Ordering::SeqCst);
        });
    }

    pool.wait_idle();

    assert!(
        duplicates.lock().unwrap().is_empty(),
        "no job should execute twice"
    );
    assert!(
        seen.iter().all(|b| b.load(Ordering::SeqCst)),
        "every index should execute at least once"
    );
    assert_eq!(
        completed.load(Ordering::SeqCst),
        N,
        "nothing should be silently lost"
    );
}
