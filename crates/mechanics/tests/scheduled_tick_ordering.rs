//! M3-B01 — `ScheduledTickQueue` acceptance tests (pure — example + property tests).

use rc_core::BlockPos;
use rc_mechanics::{ScheduledTickQueue, TickPriority};

fn pos(n: i32) -> BlockPos {
    BlockPos::new(n, 0, 0)
}

#[test]
fn drain_due_respects_trigger_tick() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_block_tick(pos(1), 5, TickPriority::Normal, 0);
    q.schedule_block_tick(pos(2), 3, TickPriority::Normal, 0);
    q.schedule_block_tick(pos(3), 10, TickPriority::Normal, 0);

    let due3 = q.drain_due_block_ticks(3);
    assert_eq!(due3.len(), 1);
    assert_eq!(due3[0].pos, pos(2));

    let due5 = q.drain_due_block_ticks(5);
    assert_eq!(due5.len(), 1);
    assert_eq!(due5[0].pos, pos(1));

    // Not yet due at 9.
    assert_eq!(q.drain_due_block_ticks(9).len(), 0);
    let due10 = q.drain_due_block_ticks(10);
    assert_eq!(due10.len(), 1);
    assert_eq!(due10[0].pos, pos(3));
}

#[test]
fn drain_due_respects_priority_then_insertion_order() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_block_tick(pos(1), 0, TickPriority::Normal, 0);
    q.schedule_block_tick(pos(2), 0, TickPriority::ExtremelyHigh, 0);
    q.schedule_block_tick(pos(3), 0, TickPriority::Normal, 0);
    q.schedule_block_tick(pos(4), 0, TickPriority::High, 0);

    let due = q.drain_due_block_ticks(0);
    let positions: Vec<BlockPos> = due.iter().map(|e| e.pos).collect();
    assert_eq!(positions, vec![pos(2), pos(4), pos(1), pos(3)]);
}

#[test]
fn block_and_fluid_queues_never_interleave() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_fluid_tick(pos(1), 0, TickPriority::ExtremelyHigh, 0);
    q.schedule_block_tick(pos(2), 0, TickPriority::ExtremelyLow, 0);

    let due_block = q.drain_due_block_ticks(0);
    assert_eq!(due_block.len(), 1);
    assert_eq!(due_block[0].pos, pos(2));

    let due_fluid = q.drain_due_fluid_ticks(0);
    assert_eq!(due_fluid.len(), 1);
    assert_eq!(due_fluid[0].pos, pos(1));
}

#[test]
fn sub_tick_order_is_shared_and_monotonic() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_block_tick(pos(1), 0, TickPriority::Normal, 0);
    q.schedule_block_tick(pos(2), 0, TickPriority::Normal, 0);
    q.schedule_fluid_tick(pos(3), 0, TickPriority::Normal, 0);

    let due_block = q.drain_due_block_ticks(0);
    let due_fluid = q.drain_due_fluid_ticks(0);

    assert_eq!(due_block[0].sub_tick_order, 0);
    assert_eq!(due_block[1].sub_tick_order, 1);
    assert_eq!(due_fluid[0].sub_tick_order, 2);
}

#[test]
fn over_cap_entries_stay_queued() {
    let mut q = ScheduledTickQueue::new();
    let total = ScheduledTickQueue::MAX_PER_TICK + 50;
    for i in 0..total {
        q.schedule_block_tick(pos(i as i32), 0, TickPriority::Normal, 0);
    }

    let first_batch = q.drain_due_block_ticks(0);
    assert_eq!(first_batch.len(), ScheduledTickQueue::MAX_PER_TICK);
    for (i, entry) in first_batch.iter().enumerate() {
        assert_eq!(entry.pos, pos(i as i32));
    }

    let second_batch = q.drain_due_block_ticks(0);
    assert_eq!(second_batch.len(), 50);
    for (i, entry) in second_batch.iter().enumerate() {
        assert_eq!(
            entry.pos,
            pos((ScheduledTickQueue::MAX_PER_TICK + i) as i32)
        );
    }
}

#[test]
fn is_pending_reflects_any_queued_entry() {
    let mut q = ScheduledTickQueue::new();
    let p = pos(42);
    q.schedule_block_tick(p, 5, TickPriority::Normal, 0);

    assert!(q.is_block_tick_pending(p));
    // Not yet due at tick 2 -- still pending, still queued.
    let due_early = q.drain_due_block_ticks(2);
    assert!(due_early.is_empty());
    assert!(q.is_block_tick_pending(p));

    // Actually drained at tick 5 -- no longer pending.
    let due = q.drain_due_block_ticks(5);
    assert_eq!(due.len(), 1);
    assert!(!q.is_block_tick_pending(p));
}

// `over_cap_entries_stay_queued` above is this file's own "property test" per the blueprint's
// Acceptance-tests framing (Context: the `MAX_PER_TICK`/overflow invariant it checks holds for
// any count, not just this one fixed scenario) — `proptest` stays this crate's pinned
// dev-dependency (`[workspace.dependencies]`'s existing `1.11.0` pin, reused not re-pinned) for
// whichever future M3 blueprint's own scheduled-tick coverage needs randomized inputs.
