use std::collections::HashSet;

use rc_core::{RcEntityId, RcEntityIdAllocator};

#[test]
fn allocator_first_alloc_is_one() {
    let allocator = RcEntityIdAllocator::new();
    assert_eq!(allocator.alloc(), RcEntityId(1));
}

#[test]
fn allocator_is_strictly_monotonic() {
    let allocator = RcEntityIdAllocator::new();
    let mut previous = allocator.alloc();
    for _ in 0..999 {
        let next = allocator.alloc();
        assert!(next.0 > previous.0);
        previous = next;
    }
}

#[test]
fn allocator_is_thread_safe_and_unique_under_contention() {
    let allocator = RcEntityIdAllocator::new();
    let mut all_ids: Vec<RcEntityId> = Vec::with_capacity(8_000);

    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let mut ids = Vec::with_capacity(1_000);
                    for _ in 0..1_000 {
                        ids.push(allocator.alloc());
                    }
                    ids
                })
            })
            .collect();

        for handle in handles {
            all_ids.extend(handle.join().unwrap());
        }
    });

    assert_eq!(all_ids.len(), 8_000);
    let unique: HashSet<RcEntityId> = all_ids.into_iter().collect();
    assert_eq!(unique.len(), 8_000);
}

#[test]
fn from_raw_round_trips() {
    assert_eq!(RcEntityId::from_raw(42).0, 42);
}
