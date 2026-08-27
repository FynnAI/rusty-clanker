//! M3-B01 — MECH-D9's double-buffered `BlockEventQueue` acceptance tests.

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::{BlockEvent, BlockEventQueue};

fn event(id: u8) -> BlockEvent {
    BlockEvent {
        pos: BlockPos::new(id as i32, 0, 0),
        event_id: id,
        event_param: 0,
        block_state: BlockStateId(1),
    }
}

#[test]
fn emitted_before_first_subphase_call_is_processed_immediately() {
    let mut q = BlockEventQueue::new();
    q.emit(event(1));
    q.emit(event(2));

    let batch = q.begin_subphase();
    assert_eq!(batch, vec![event(1), event(2)]);
}

#[test]
fn emitted_during_processing_is_deferred_one_tick() {
    let mut q = BlockEventQueue::new();
    q.emit(event(1)); // event A
    let batch_a = q.begin_subphase();
    assert_eq!(batch_a, vec![event(1)]);

    // Processing batch_a's event(s) (test code, not the queue itself) emits a second event.
    q.emit(event(2)); // event B

    let batch_b = q.begin_subphase();
    assert_eq!(batch_b, vec![event(2)]);
}

#[test]
fn subphase_call_with_nothing_pending_returns_empty() {
    let mut q = BlockEventQueue::new();
    assert_eq!(q.begin_subphase(), Vec::<BlockEvent>::new());
}
