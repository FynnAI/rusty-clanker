//! M3-B03 acceptance test: the dig-packet state machine (`DestroyState`/`begin_destroy`/
//! `stop_destroy`/`abort_destroy`/`tick_destroy_state`) -- pure, no sockets. See
//! `blueprints/M3/M3-B03-breaking-placing.md`, Acceptance tests,
//! "`crates/server/tests/mining_destroy_state_machine.rs`".

use rc_chunk_storage::{BlockStateId, RegistryId};
use rc_core::BlockPos;
use rusty_clanker_server::play::{
    DestroyOutcome, DestroySpeed, DestroyState, StopOutcome, TickOutcome, abort_destroy,
    begin_destroy, stop_destroy, tick_destroy_state,
};

const STONE_RAW: u32 = 1;
const AIR_RAW: u32 = 0;

fn air() -> BlockStateId {
    BlockStateId::from_raw(AIR_RAW)
}

fn stone() -> BlockStateId {
    BlockStateId::from_raw(STONE_RAW)
}

#[test]
fn start_destroy_enters_tracking_for_a_multi_tick_block() {
    let mut state = DestroyState::default();
    let pos = BlockPos::new(5, -60, 5);
    let speed = DestroySpeed::PerTick(1.0 / 23.0);

    let outcome = begin_destroy(&mut state, pos, false, speed, 100);

    assert_eq!(outcome, DestroyOutcome::Tracking);
    assert!(state.is_destroying);
    assert_eq!(state.destroy_pos, pos);
    assert_eq!(state.destroy_progress_start, 100);
}

#[test]
fn start_destroy_finalizes_immediately_for_instant_blocks() {
    let mut state = DestroyState::default();
    let pos = BlockPos::new(5, -60, 5);

    let outcome = begin_destroy(&mut state, pos, false, DestroySpeed::Instant, 100);

    assert_eq!(outcome, DestroyOutcome::FinalizeNow);
    assert!(!state.is_destroying);
}

#[test]
fn start_destroy_always_finalizes_in_creative_regardless_of_speed() {
    let mut state = DestroyState::default();
    let pos = BlockPos::new(5, -60, 5);
    let speed = DestroySpeed::PerTick(1.0 / 150.0);

    let outcome = begin_destroy(&mut state, pos, true, speed, 100);

    assert_eq!(outcome, DestroyOutcome::FinalizeNow);
}

fn tracking_state_from_test_1() -> (DestroyState, BlockPos, DestroySpeed) {
    let mut state = DestroyState::default();
    let pos = BlockPos::new(5, -60, 5);
    let speed = DestroySpeed::PerTick(1.0 / 23.0);
    let outcome = begin_destroy(&mut state, pos, false, speed, 100);
    assert_eq!(outcome, DestroyOutcome::Tracking);
    (state, pos, speed)
}

#[test]
fn stop_before_threshold_queues_delayed_destroy() {
    let (mut state, pos, speed) = tracking_state_from_test_1();

    // elapsed = current_tick - start + 1 = 105 - 100 + 1 = 6; progress = 6/23 ~= 0.261 < 0.7.
    let outcome = stop_destroy(&mut state, pos, speed, 105);

    assert_eq!(outcome, StopOutcome::DelayedQueued);
    assert!(!state.is_destroying);
    assert!(state.has_delayed_destroy);
    // The *original* start tick, not 105 -- a delayed destroy does not restart the clock.
    assert_eq!(state.delayed_tick_start, 100);
}

#[test]
fn stop_at_or_above_threshold_finalizes_immediately() {
    let (mut state, pos, speed) = tracking_state_from_test_1();

    // elapsed = 116 - 100 + 1 = 17; progress = 17/23 ~= 0.739 >= 0.7.
    let outcome = stop_destroy(&mut state, pos, speed, 116);

    assert_eq!(outcome, StopOutcome::FinalizeNow);
}

#[test]
fn abort_clears_active_but_not_delayed() {
    let mut state = DestroyState {
        is_destroying: true,
        destroy_pos: BlockPos::new(1, -60, 1),
        destroy_progress_start: 10,
        has_delayed_destroy: true,
        delayed_destroy_pos: BlockPos::new(9, -60, 9),
        delayed_tick_start: 3,
        last_sent_stage: 2,
    };

    abort_destroy(&mut state);

    assert!(!state.is_destroying);
    assert!(state.has_delayed_destroy);
    assert_eq!(state.delayed_destroy_pos, BlockPos::new(9, -60, 9));
    assert_eq!(state.delayed_tick_start, 3);
}

#[test]
fn tick_reports_rising_stage_and_detects_cancellation() {
    let (mut state, _pos, speed) = tracking_state_from_test_1();

    // elapsed = 105 - 100 + 1 = 6; 6/23 ~= 0.2609; *10 ~= 2.609; floor = 2.
    let outcome = tick_destroy_state(&mut state, speed, 105, stone(), air(), air());
    assert_eq!(outcome, TickOutcome::ActiveProgress(2));
    assert!(state.is_destroying);

    // The block already turned to air -- cancelled, no further progress reported.
    let outcome = tick_destroy_state(&mut state, speed, 106, air(), air(), air());
    assert_eq!(outcome, TickOutcome::CancelledBlockChanged);
    assert!(!state.is_destroying);
}

#[test]
fn delayed_destroy_finalizes_once_progress_reaches_one_via_tick() {
    let (mut state, pos, speed) = tracking_state_from_test_1();
    let outcome = stop_destroy(&mut state, pos, speed, 105);
    assert_eq!(outcome, StopOutcome::DelayedQueued);
    assert_eq!(state.delayed_tick_start, 100);

    // elapsed = 122 - 100 + 1 = 23; 23 * (1/23) == 1.0 exactly.
    let outcome = tick_destroy_state(&mut state, speed, 122, air(), stone(), air());

    assert_eq!(outcome, TickOutcome::FinalizeDelayedNow);
    assert!(!state.has_delayed_destroy);
}
