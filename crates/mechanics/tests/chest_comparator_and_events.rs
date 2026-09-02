//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain, see hopper_transfer_order.rs) nondefault-state=yes
//! M3-B06 — chest comparator-signal and open-count/block-event acceptance tests (Acceptance
//! tests' own `chest_comparator_and_events.rs` section, the task's own required acceptance
//! category).

use rc_chunk_storage::{BlockStateId, ItemStackRecord};
use rc_core::BlockPos;
use rc_mechanics::block_entity::chest::{CHEST_OPEN_EVENT_ID, ChestBlockEntity};
use rc_mechanics::block_event::BlockEventQueue;
use rc_mechanics::container::DefaultMaxStackSize;

fn full_stack(id: &str) -> Option<ItemStackRecord> {
    Some(ItemStackRecord {
        id: id.to_string(),
        count: 64,
        components: None,
    })
}

#[test]
fn empty_chest_signal_is_zero() {
    let chest = ChestBlockEntity::empty();
    assert_eq!(chest.comparator_signal(&DefaultMaxStackSize), 0);
}

#[test]
fn single_full_stack_in_one_slot_signal_matches_formula() {
    let mut chest = ChestBlockEntity::empty();
    chest.slots[0] = full_stack("minecraft:item");
    // average = (64/64) / 27 = 1/27; floor((1/27) * 14) + 1 = floor(0.5185...) + 1 = 0 + 1 = 1
    assert_eq!(chest.comparator_signal(&DefaultMaxStackSize), 1);
}

#[test]
fn completely_full_chest_signal_is_fifteen_nondefault_case() {
    let mut chest = ChestBlockEntity::empty();
    for slot in chest.slots.iter_mut() {
        *slot = full_stack("minecraft:item");
    }
    assert_eq!(chest.comparator_signal(&DefaultMaxStackSize), 15);
}

#[test]
fn open_count_transition_zero_to_one_emits_block_event() {
    let mut chest = ChestBlockEntity::empty();
    let mut queue = BlockEventQueue::new();
    let pos = BlockPos::new(0, 0, 0);
    let state = BlockStateId(10);

    let new_count = chest.add_viewer(pos, state, &mut queue);
    assert_eq!(new_count, 1);

    let batch = queue.drain_all();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].event_id, CHEST_OPEN_EVENT_ID);
    assert_eq!(batch[0].event_param, 1);
    assert_eq!(batch[0].pos, pos);
    assert_eq!(batch[0].block_state, state);
}

#[test]
fn open_count_further_increments_do_not_re_emit() {
    let mut chest = ChestBlockEntity::empty();
    let mut queue = BlockEventQueue::new();
    let pos = BlockPos::new(0, 0, 0);
    let state = BlockStateId(10);

    chest.add_viewer(pos, state, &mut queue);
    queue.drain_all(); // drain the first (0 -> 1) event

    let new_count = chest.add_viewer(pos, state, &mut queue);
    assert_eq!(new_count, 2);

    let batch = queue.drain_all();
    assert!(
        batch.is_empty(),
        "only the 0<->nonzero transition should emit an event"
    );
}

#[test]
fn open_count_transition_one_to_zero_emits_block_event() {
    let mut chest = ChestBlockEntity::empty();
    chest.open_count = 1;
    let mut queue = BlockEventQueue::new();
    let pos = BlockPos::new(0, 0, 0);
    let state = BlockStateId(10);

    let new_count = chest.remove_viewer(pos, state, &mut queue);
    assert_eq!(new_count, 0);

    let batch = queue.drain_all();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].event_id, CHEST_OPEN_EVENT_ID);
    assert_eq!(batch[0].event_param, 0);
}

#[test]
fn remove_viewer_never_underflows_below_zero() {
    let mut chest = ChestBlockEntity::empty();
    let mut queue = BlockEventQueue::new();
    let pos = BlockPos::new(0, 0, 0);
    let state = BlockStateId(10);

    let new_count = chest.remove_viewer(pos, state, &mut queue);
    assert_eq!(new_count, 0);

    let batch = queue.drain_all();
    assert!(batch.is_empty(), "no real 1 -> 0 transition occurred");
}
