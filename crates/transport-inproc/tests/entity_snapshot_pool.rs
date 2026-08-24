use rc_core::{ChunkKey, DimensionId, RcEntityId};
use rc_messaging::EntitySnapshot;
use rc_transport_inproc::EntitySnapshotPool;

#[test]
fn acquire_on_empty_pool_allocates_fresh() {
    let pool = EntitySnapshotPool::new(4);
    assert_eq!(pool.free_count(), 0);

    let v = EntitySnapshot {
        entity_id: RcEntityId::from_raw(1),
        source_chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        component_data: vec![9],
    };
    let slot = pool.acquire(v.clone());
    assert_eq!(*slot, v);
}

#[test]
fn release_then_acquire_reuses_the_same_allocation() {
    let pool = EntitySnapshotPool::new(4);

    let v_a = EntitySnapshot {
        entity_id: RcEntityId::from_raw(1),
        source_chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        component_data: vec![1],
    };
    let slot_a = pool.acquire(v_a.clone());
    let addr_a = std::ptr::addr_of!(*slot_a) as usize;
    pool.release(slot_a);
    assert_eq!(pool.free_count(), 1);

    let v_b = EntitySnapshot {
        entity_id: RcEntityId::from_raw(2),
        source_chunk: ChunkKey::new(DimensionId::THE_NETHER, 1, 1),
        component_data: vec![2, 3],
    };
    let slot_b = pool.acquire(v_b.clone());
    assert_eq!(std::ptr::addr_of!(*slot_b) as usize, addr_a);
    assert_eq!(*slot_b, v_b);
    assert_eq!(pool.free_count(), 0);
}

#[test]
fn release_beyond_capacity_drops_the_extra_slot() {
    let pool = EntitySnapshotPool::new(1);

    let v1 = EntitySnapshot {
        entity_id: RcEntityId::from_raw(1),
        source_chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        component_data: vec![1],
    };
    let v2 = EntitySnapshot {
        entity_id: RcEntityId::from_raw(2),
        source_chunk: ChunkKey::new(DimensionId::OVERWORLD, 1, 1),
        component_data: vec![2],
    };
    let box1 = pool.acquire(v1);
    let box2 = pool.acquire(v2);

    pool.release(box1);
    assert_eq!(pool.free_count(), 1);

    pool.release(box2);
    assert_eq!(pool.free_count(), 1);
}

#[test]
fn acquire_and_release_are_thread_safe_under_contention() {
    let pool = EntitySnapshotPool::new(16);

    std::thread::scope(|scope| {
        for thread_idx in 0..8u64 {
            let pool_ref = &pool;
            scope.spawn(move || {
                for i in 0..200u64 {
                    let v = EntitySnapshot {
                        entity_id: RcEntityId::from_raw(thread_idx * 1000 + i),
                        source_chunk: ChunkKey::new(
                            DimensionId::OVERWORLD,
                            thread_idx as i32,
                            i as i32,
                        ),
                        component_data: Vec::new(),
                    };
                    let slot = pool_ref.acquire(v);
                    pool_ref.release(slot);
                }
            });
        }
    });

    assert!(pool.free_count() <= 16);
}
