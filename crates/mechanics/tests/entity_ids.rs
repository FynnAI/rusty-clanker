//! Entity identity acceptance tests (M4-B01 Deliverables, `entity::ids`).

use std::collections::HashSet;

use rc_mechanics::entity::{EntityUuid, NetworkEntityIdAllocator};

#[test]
fn entity_uuid_new_random_is_unique_across_many_calls() {
    let uuids: HashSet<u128> = (0..10_000).map(|_| EntityUuid::new_random().0).collect();
    assert_eq!(uuids.len(), 10_000);
}

#[test]
fn network_entity_id_allocator_first_alloc_is_one() {
    let allocator = NetworkEntityIdAllocator::new();
    assert_eq!(allocator.alloc(), 1);
}

#[test]
fn network_entity_id_allocator_is_thread_safe_and_unique_under_contention() {
    use std::sync::Arc;
    use std::thread;

    let allocator = Arc::new(NetworkEntityIdAllocator::new());
    let mut handles = Vec::with_capacity(8);
    for _ in 0..8 {
        let allocator = Arc::clone(&allocator);
        handles.push(thread::spawn(move || {
            (0..1000).map(|_| allocator.alloc()).collect::<Vec<i32>>()
        }));
    }

    let mut all_ids: Vec<i32> = Vec::with_capacity(8000);
    for handle in handles {
        all_ids.extend(handle.join().unwrap());
    }

    assert_eq!(all_ids.len(), 8000);
    let unique: HashSet<i32> = all_ids.into_iter().collect();
    assert_eq!(unique.len(), 8000);
}
