//! test-matrix: boundaries=yes orientations=waived(the tick queue is keyed by position and priority alone — it has no facing, and no direction ever reaches it) self=waived(no player/actor entity in this suite's own domain model) composition=waived(multi-component chains are the mechanic suites' own job — redstone_repeater.rs for the diode-level rule, and the redstone/clock/repeater_loop_clock_delay1_pulse1 corpus fixture end to end) nondefault-state=waived(this suite exercises the tick queue, which stores positions, trigger ticks and priorities — never a block state id)
//! M3 field-report wave 3 (PLAN-D10, finding 3) — `ScheduledTickQueue`'s two distinct
//! vanilla-derived guards, pinned separately because conflating them is exactly the bug this
//! wave root-caused.
//!
//! Vanilla keeps two structures, and a diode or redstone torch consults only the second one:
//!
//! * a per-position dedup set of *queued* ticks, held by a chunk's own tick container —
//!   scheduling a second tick for a position that already has one queued is dropped, and a
//!   position leaves the set when its tick is collected for a game tick. This backs
//!   `hasScheduledTick` / our `is_block_tick_pending`.
//! * the level-wide *run set* for the current game tick — filled when that tick's due entries
//!   are collected, drained entry by entry as the run loop takes each one off to run it, and
//!   emptied once the loop ends. This backs `willTickThisTick` / our
//!   `will_block_tick_this_tick`, and it is the guard `DiodeBlock.checkTickOnNeighbor`,
//!   `ComparatorBlock.checkTickOnNeighbor` and `RedstoneTorchBlock.neighborChanged` use.
//!
//! The two disagree in exactly one window — a position whose tick has been collected for this
//! game tick but has not run yet — and a two-repeater loop clock lands in that window every
//! period, which is how the divergence was found.

use rc_core::BlockPos;
use rc_mechanics::{ScheduledTickQueue, TickPriority};

fn pos(n: i32) -> BlockPos {
    BlockPos::new(n, 0, 0)
}

/// A queued-but-not-yet-collected tick is *pending*, and is explicitly **not** "will tick this
/// tick" — the distinction the three diode/torch call sites depend on.
#[test]
fn a_tick_queued_for_a_later_game_tick_is_pending_but_not_in_the_run_set() {
    let mut q = ScheduledTickQueue::new();
    let p = pos(1);
    q.schedule_block_tick(p, 4, TickPriority::High, 0);

    assert!(q.is_block_tick_pending(p));
    assert!(!q.will_block_tick_this_tick(p));

    // A drain that collects nothing must not invent a run set either.
    assert!(q.drain_due_block_ticks(2).is_empty());
    assert!(q.is_block_tick_pending(p));
    assert!(!q.will_block_tick_this_tick(p));
}

/// Collecting a game tick's due entries moves each of them out of the queued-tick dedup set and
/// into the run set — vanilla's collect step, which polls the container (dropping the position
/// from its dedup set) and appends the entry to the run queue.
#[test]
fn collecting_a_batch_moves_positions_from_pending_into_the_run_set() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_block_tick(pos(1), 0, TickPriority::VeryHigh, 0);
    q.schedule_block_tick(pos(2), 0, TickPriority::High, 0);

    let due = q.drain_due_block_ticks(0);
    assert_eq!(due.len(), 2);

    for p in [pos(1), pos(2)] {
        assert!(!q.is_block_tick_pending(p));
        assert!(q.will_block_tick_this_tick(p));
    }
}

/// This is the exact window the loop-clock divergence lived in: while the first collected entry
/// runs, every entry still waiting in the same batch must answer `true`, so a neighbour change
/// reaching one of them is refused — and the entry that is running must already answer `false`,
/// so a diode's own turn-on branch can re-arm itself.
#[test]
fn running_one_entry_clears_only_that_position_from_the_run_set() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_block_tick(pos(1), 0, TickPriority::VeryHigh, 0);
    q.schedule_block_tick(pos(2), 0, TickPriority::High, 0);
    let due = q.drain_due_block_ticks(0);
    assert_eq!(due[0].pos, pos(1), "VeryHigh runs before High");

    q.run_block_tick(pos(1));
    assert!(
        !q.will_block_tick_this_tick(pos(1)),
        "the entry currently being run is already out of the run set"
    );
    assert!(
        q.will_block_tick_this_tick(pos(2)),
        "an entry collected for this same game tick that has not run yet is still guarded"
    );

    q.run_block_tick(pos(2));
    assert!(!q.will_block_tick_this_tick(pos(2)));
}

/// Vanilla empties the run set unconditionally once the collected ticks have run, so nothing
/// later in the same game tick — the fluid ticks, the block-event sub-phase, a player action —
/// and nothing in any later game tick can still see it.
#[test]
fn ending_the_batch_empties_the_run_set() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_block_tick(pos(1), 0, TickPriority::Normal, 0);
    q.schedule_block_tick(pos(2), 0, TickPriority::Normal, 0);
    assert_eq!(q.drain_due_block_ticks(0).len(), 2);

    q.end_block_tick_batch();
    assert!(!q.will_block_tick_this_tick(pos(1)));
    assert!(!q.will_block_tick_this_tick(pos(2)));
}

/// The queue itself carries vanilla's per-position dedup, so a caller that has already passed
/// the run-set guard still cannot queue a second tick for a position that has one queued. The
/// first entry's trigger tick, priority and sub-tick order all survive untouched — the later
/// schedule is dropped, never merged and never allowed to re-prioritise.
#[test]
fn a_second_schedule_for_an_already_queued_position_is_dropped() {
    let mut q = ScheduledTickQueue::new();
    let p = pos(1);
    q.schedule_block_tick(p, 4, TickPriority::High, 0);
    q.schedule_block_tick(p, 1, TickPriority::ExtremelyHigh, 0);

    assert_eq!(q.block_len(), 1);
    let due = q.drain_due_block_ticks(4);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger_tick, 4);
    assert_eq!(due[0].priority, TickPriority::High);
    assert_eq!(due[0].sub_tick_order, 0);
}

/// Vanilla builds the scheduled tick — drawing the level's next sub-tick number as it does — at
/// the call site, and only then hands it to the container that may drop it. So a dropped
/// schedule still burns a sub-tick number, which shifts the intra-tick run order of every
/// equal-priority tick queued afterwards. Pinned because it is invisible until two ticks tie on
/// priority.
#[test]
fn a_dropped_schedule_still_consumes_a_sub_tick_order() {
    let mut q = ScheduledTickQueue::new();
    q.schedule_block_tick(pos(1), 0, TickPriority::Normal, 0);
    q.schedule_block_tick(pos(1), 0, TickPriority::Normal, 0); // dropped, but numbered
    q.schedule_block_tick(pos(2), 0, TickPriority::Normal, 0);

    let due = q.drain_due_block_ticks(0);
    assert_eq!(due.len(), 2);
    assert_eq!(due[0].sub_tick_order, 0);
    assert_eq!(
        due[1].sub_tick_order, 2,
        "sub-tick order 1 belongs to the dropped schedule and is never reused"
    );
}

/// The dedup covers queued ticks only. Once a position's tick has been collected it is free to
/// take a fresh one — which is exactly how a diode's tick body re-arms itself on the same game
/// tick it runs (`DiodeBlock.tick`'s turn-on branch, when the input has already gone away).
#[test]
fn a_collected_position_can_be_scheduled_again_within_the_same_game_tick() {
    let mut q = ScheduledTickQueue::new();
    let p = pos(1);
    q.schedule_block_tick(p, 0, TickPriority::High, 0);
    assert_eq!(q.drain_due_block_ticks(0).len(), 1);
    q.run_block_tick(p);

    q.schedule_block_tick(p, 2, TickPriority::VeryHigh, 0);
    assert!(q.is_block_tick_pending(p));
    let due = q.drain_due_block_ticks(2);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger_tick, 2);
    assert_eq!(due[0].priority, TickPriority::VeryHigh);
}

/// Entries the `MAX_PER_TICK` cap leaves behind were never collected, so they stay queued *and*
/// stay in the dedup set — the two must not drift apart, or a capped-out position would silently
/// accept a duplicate tick.
#[test]
fn positions_left_behind_by_the_per_tick_cap_stay_pending() {
    let mut q = ScheduledTickQueue::new();
    let total = ScheduledTickQueue::MAX_PER_TICK + 2;
    for i in 0..total {
        q.schedule_block_tick(pos(i as i32), 0, TickPriority::Normal, 0);
    }

    let first = q.drain_due_block_ticks(0);
    assert_eq!(first.len(), ScheduledTickQueue::MAX_PER_TICK);

    let left_behind = pos((ScheduledTickQueue::MAX_PER_TICK + 1) as i32);
    assert!(q.is_block_tick_pending(left_behind));
    assert!(!q.will_block_tick_this_tick(left_behind));
    q.schedule_block_tick(left_behind, 0, TickPriority::ExtremelyHigh, 0);
    assert_eq!(
        q.block_len(),
        2,
        "the duplicate was dropped, as when uncapped"
    );
}

/// TEST-D55 (a): both world-height boundaries. Positions at Y = -64 and Y = 319 are ordinary
/// keys to this queue, and both guards must track them exactly as they track any other — a
/// redstone contraption sitting on bedrock or under the build ceiling gets the same rule.
#[test]
fn run_set_tracks_positions_at_both_world_height_boundary_cases() {
    let mut q = ScheduledTickQueue::new();
    let floor = BlockPos::new(0, -64, 0);
    let ceiling = BlockPos::new(0, 319, 0);

    q.schedule_block_tick(floor, 0, TickPriority::VeryHigh, 0);
    q.schedule_block_tick(ceiling, 0, TickPriority::High, 0);
    q.schedule_block_tick(floor, 0, TickPriority::ExtremelyHigh, 0); // dropped by the dedup

    let due = q.drain_due_block_ticks(0);
    assert_eq!(due.len(), 2);
    assert_eq!(due[0].pos, floor);
    assert_eq!(due[1].pos, ceiling);

    assert!(q.will_block_tick_this_tick(floor));
    assert!(q.will_block_tick_this_tick(ceiling));
    q.run_block_tick(floor);
    assert!(!q.will_block_tick_this_tick(floor));
    assert!(q.will_block_tick_this_tick(ceiling));

    q.end_block_tick_batch();
    assert!(!q.will_block_tick_this_tick(ceiling));
}

/// The fluid queue's own guards are untouched by any of the above: a block tick never enters the
/// fluid run set, and vice versa.
#[test]
fn the_block_run_set_and_the_fluid_batch_stay_independent() {
    let mut q = ScheduledTickQueue::new();
    let p = pos(1);
    q.schedule_block_tick(p, 0, TickPriority::Normal, 0);
    q.schedule_fluid_tick(p, 0, TickPriority::Normal, 0);

    assert_eq!(q.drain_due_block_ticks(0).len(), 1);
    assert!(q.will_block_tick_this_tick(p));
    assert!(!q.is_fluid_tick_in_current_batch(p));

    assert_eq!(q.drain_due_fluid_ticks(0).len(), 1);
    assert!(q.is_fluid_tick_in_current_batch(p));
    assert!(
        q.will_block_tick_this_tick(p),
        "draining the fluid queue never touches the block run set"
    );
}
