//! M0-B04 acceptance: proves `run_batch`'s blocking, scoped batch-dispatch
//! semantics — every task runs exactly once, non-`'static` borrowed task
//! closures are supported and their writes are observed by the caller after
//! `run_batch` returns, and exactly one panic is propagated after every
//! other task has finished.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rc_scheduler::pool::RcWorkerPool;

#[test]
fn run_batch_runs_every_task_exactly_once() {
    let pool = RcWorkerPool::new(4);
    let counter = Arc::new(AtomicUsize::new(0));

    let tasks: Vec<Box<dyn FnOnce() + Send>> = (0..1_000)
        .map(|_| {
            let counter = Arc::clone(&counter);
            let task: Box<dyn FnOnce() + Send> = Box::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
            task
        })
        .collect();

    pool.run_batch(tasks);

    assert_eq!(counter.load(Ordering::SeqCst), 1_000);
}

#[test]
fn run_batch_supports_borrowed_non_static_writes() {
    let pool = RcWorkerPool::new(4);
    let mut values = vec![0i32; 100];

    let tasks: Vec<Box<dyn FnOnce() + Send + '_>> = values
        .iter_mut()
        .enumerate()
        .map(|(i, slot)| {
            let task: Box<dyn FnOnce() + Send + '_> = Box::new(move || {
                *slot = i as i32;
            });
            task
        })
        .collect();

    pool.run_batch(tasks);

    assert_eq!(values, (0..100).collect::<Vec<i32>>());
}

#[test]
fn run_batch_propagates_exactly_one_panic_after_all_tasks_finish() {
    let pool = RcWorkerPool::new(4);
    let completed = Arc::new(AtomicUsize::new(0));
    const PANIC_MESSAGE: &str = "known panic payload";

    let tasks: Vec<Box<dyn FnOnce() + Send>> = (0..10)
        .map(|i| {
            let completed = Arc::clone(&completed);
            let task: Box<dyn FnOnce() + Send> = if i == 5 {
                Box::new(|| std::panic::panic_any(PANIC_MESSAGE))
            } else {
                Box::new(move || {
                    completed.fetch_add(1, Ordering::SeqCst);
                })
            };
            task
        })
        .collect();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pool.run_batch(tasks);
    }));

    assert!(result.is_err());
    assert_eq!(completed.load(Ordering::SeqCst), 9);
    let payload = result.unwrap_err();
    let message: &&str = payload
        .downcast_ref::<&str>()
        .expect("panic payload should downcast to the known &'static str message");
    assert_eq!(*message, PANIC_MESSAGE);
}

#[test]
fn run_batch_empty_is_a_no_op() {
    let pool = RcWorkerPool::new(4);
    pool.run_batch(Vec::new());
}
