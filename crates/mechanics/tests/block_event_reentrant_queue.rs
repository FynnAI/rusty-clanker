//! M3-B01 — MECH-D9's re-entrant, single-buffered `BlockEventQueue` acceptance tests, at the
//! raw-queue level only. `stage4_ordering.rs`'s own `block_event_*`/`two_adjacent_positions_*`
//! tests cover the full same-tick cascade through `run_block_event_subphase`/
//! `BlockBehavior::on_block_event`, which is where MECH-D9's guarantee actually matters to a
//! real contraption; this file only pins the queue primitive those tests build on.
//!
//! This file previously (as `block_event_double_buffer.rs`) pinned M3's double-buffered
//! queue-then-flush-once-per-tick design, which `05-game-mechanics.md`'s MECH-D9 row now
//! documents as a disproven, closed parity deviation (PLAN-D9) — see this changeset's own
//! commit body for the reference-audit justification.

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
fn emitted_before_any_pop_is_returned_in_fifo_order() {
    let mut q = BlockEventQueue::new();
    q.emit(event(1));
    q.emit(event(2));

    assert_eq!(q.pop_next(), Some(event(1)));
    assert_eq!(q.pop_next(), Some(event(2)));
    assert_eq!(q.pop_next(), None);
}

/// MECH-D9's own re-entrancy guarantee, at the raw queue level: an event `emit`ted while
/// another one is "in flight" (already popped by a driver loop, not yet fully handled) lands
/// in the *same* live queue that loop keeps draining — it comes back on the very next
/// `pop_next` call, never held back to a separate buffer for a later call to drain.
///
/// Corrects this file's own former (`block_event_double_buffer.rs`) `emitted_during_
/// processing_is_deferred_one_tick` test, which asserted the opposite — that a mid-processing
/// `emit` would *not* reappear until a second top-level call — because that was M3's own
/// double-buffered design's real behavior at the time. `05-game-mechanics.md`'s MECH-D9 row now
/// states, and this project's reference audit confirms, that vanilla drains one single live
/// queue in a re-entrant `while`-loop with no such deferral; that old test therefore encoded a
/// disproven blueprint claim rather than a real requirement, and is replaced by this one.
#[test]
fn emitted_while_another_is_in_flight_is_returned_by_the_very_next_pop() {
    let mut q = BlockEventQueue::new();
    q.emit(event(1)); // event A
    assert_eq!(q.pop_next(), Some(event(1)));

    // Handling event A (test code standing in for a real `on_block_event` — `stage4_ordering.
    // rs`'s own tests exercise the real handler path) emits event B as a synchronous side
    // effect, before the driver loop's own next `pop_next` call.
    q.emit(event(2)); // event B

    assert_eq!(
        q.pop_next(),
        Some(event(2)),
        "B must come back on the very next pop -- the same pass, not a separate later call"
    );
    assert_eq!(q.pop_next(), None);
}

#[test]
fn pop_next_with_nothing_queued_returns_none() {
    let mut q = BlockEventQueue::new();
    assert_eq!(q.pop_next(), None);
}

#[test]
fn drain_all_takes_everything_queued_right_now_in_fifo_order() {
    let mut q = BlockEventQueue::new();
    q.emit(event(1));
    q.emit(event(2));

    assert_eq!(q.drain_all(), vec![event(1), event(2)]);
    assert_eq!(q.pending(), 0);
}

/// M3 field-report fix (Section B3): `emit` called while `begin_scheduled_phase_dispatch` is in
/// effect (`run_scheduled_phase`'s own dispatch span -- exactly where a piston's real
/// finalization lives) lands in a held-back buffer, invisible to `pop_next` not just until the
/// *next* `begin_pass`, but until the one *after* that (`BlockEventQueue`'s own doc comment has
/// the full two-generation rationale: "next begin_pass" alone would still fire the very same
/// tick, since `system_scheduled_phase` and `system_block_event_subphase` run back to back). The
/// primitive-level half of the "piston finalization cascade waits for the next tick" fix
/// (`docs/findings-for-planning.md`'s own entry; `stage4_ordering.rs`'s own `block_event_from_
/// scheduled_phase_waits_for_the_next_block_event_pass` exercises the real cross-phase scenario
/// this primitive serves).
#[test]
fn emit_during_scheduled_phase_dispatch_is_held_back_for_a_full_extra_begin_pass_cycle() {
    let mut q = BlockEventQueue::new();

    q.begin_scheduled_phase_dispatch();
    q.emit(event(1));
    q.end_scheduled_phase_dispatch();
    assert_eq!(
        q.pop_next(),
        None,
        "an event emitted during scheduled-phase dispatch must not be visible to pop_next yet"
    );
    assert_eq!(
        q.pending(),
        1,
        "it must still count as pending, just deferred"
    );

    // The very next begin_pass (standing in for THIS SAME tick's own run_block_event_subphase
    // call) must not surface it yet.
    q.begin_pass();
    assert_eq!(
        q.pop_next(),
        None,
        "the immediately-next pass must not see an event deferred before it started"
    );

    // The pass after that (standing in for the NEXT tick's own call) does.
    q.begin_pass();
    assert_eq!(
        q.pop_next(),
        Some(event(1)),
        "the second begin_pass must fold the deferred event into the live queue"
    );
    assert_eq!(q.pop_next(), None);
}

/// Outside `begin_scheduled_phase_dispatch`'s own span, a bare `emit`+`pop_next` call behaves
/// exactly as before this fix -- no `begin_pass` involvement needed at all. Every other test in
/// this file already relies on this implicitly; this one pins it directly (`BlockEventQueue`'s
/// own doc comment: `emit` lands straight in the live queue "by default").
#[test]
fn a_fresh_queue_lets_a_bare_emit_be_immediately_visible() {
    let mut q = BlockEventQueue::new();
    q.emit(event(1));
    assert_eq!(q.pop_next(), Some(event(1)));
}

/// `emit` called re-entrantly while `run_block_event_subphase`'s own drain loop is running (i.e.
/// after a `begin_pass` call, never inside `begin_scheduled_phase_dispatch`'s own span) still
/// lands in the live queue, same-pass, exactly as MECH-D9 requires.
#[test]
fn emit_during_a_block_event_pass_still_lands_in_the_live_queue() {
    let mut q = BlockEventQueue::new();
    q.begin_pass();
    q.emit(event(1));
    assert_eq!(q.pop_next(), Some(event(1)));
}
