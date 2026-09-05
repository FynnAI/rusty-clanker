//! M3 field-report test-authoring (MECH-D83, wave 3 Stream B, task B1): the per-tick
//! confirmed-block-event outbox (`BlockEventQueue::confirm`/`drain_confirmed`,
//! `UpdateContext::confirm_block_event`) and `run_block_event_subphase`'s own staleness gate
//! (vanilla's `doBlockEvent`: an event whose recorded block has since become something else
//! is dropped silently, never dispatched at all). Exercises the ECS-agnostic
//! `stage4::run_block_event_subphase` core directly, via `support::FakeWorld` -- mirrors
//! `crates/mechanics/tests/stage4_ordering.rs`'s own established "drive the pure function
//! directly, no bevy `World` needed" shape. No case-matrix header: `block_event_outbox` does
//! not match any of `xtask::case_matrix::MECHANIC_TEST_PREFIXES` (mirrors the pre-existing
//! `block_event_reentrant_queue.rs`'s own identical no-header precedent).

mod support;

use std::sync::Mutex;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::stage4::run_block_event_subphase;
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEvent, BlockEventQueue, BlockWorldAccess,
    LightDirtyQueue, NeighborUpdateEngine, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_registries::generated_v776::block_states::default_state::{PISTON, STONE};

use support::FakeWorld;

/// Confirms every `event_id == 0` it is dispatched for (`ctx.confirm_block_event`), ignores
/// every other id -- and records every call it actually receives (dispatched or not), so a
/// stale event's own "never dispatched at all" claim is directly observable.
struct StubBehavior {
    dispatched: Mutex<Vec<BlockEvent>>,
}

impl StubBehavior {
    fn new() -> Self {
        Self {
            dispatched: Mutex::new(Vec::new()),
        }
    }
}

impl BlockBehavior for StubBehavior {
    fn on_block_event(&self, ctx: &mut UpdateContext, _pos: BlockPos, event: &BlockEvent) {
        self.dispatched.lock().unwrap().push(*event);
        if event.event_id == 0 {
            ctx.confirm_block_event(event);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_one_pass(
    world: &mut FakeWorld,
    ownership: &RegionOwnership,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
) {
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let mut light_dirty = LightDirtyQueue::new();
    run_block_event_subphase(
        world,
        ownership,
        engine,
        scheduled,
        events,
        behaviors,
        &mut outbound,
        &mut changed,
        &mut light_dirty,
        current_tick,
    );
}

#[test]
fn confirms_the_first_event_and_ignores_the_second_with_the_right_tuple() {
    let mut world = FakeWorld::new();
    let pos = BlockPos::new(0, 0, 0);
    let state = BlockStateId(PISTON.0);
    world.set_block(pos, state);

    let ownership = RegionOwnership::always_local(Address::Region(rc_messaging::RegionId(0)));
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut behaviors = BlockBehaviorRegistry::new();
    let stub = std::sync::Arc::new(StubBehavior::new());
    behaviors.register_one(state, stub.clone());

    let first = BlockEvent {
        pos,
        event_id: 0,
        event_param: 42,
        block_state: state,
    };
    let second = BlockEvent {
        pos,
        event_id: 1,
        event_param: 7,
        block_state: state,
    };
    events.emit(first);
    events.emit(second);

    run_one_pass(
        &mut world,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &behaviors,
        0,
    );

    assert_eq!(
        stub.dispatched.lock().unwrap().as_slice(),
        &[first, second],
        "both events must reach on_block_event -- the second is only skipped for \
         confirmation, not for dispatch"
    );

    let confirmed = events.drain_confirmed();
    assert_eq!(
        confirmed,
        vec![first],
        "the outbox must hold exactly the confirmed event, with its own full tuple"
    );
}

#[test]
fn a_stale_event_whose_block_changed_is_never_dispatched() {
    let mut world = FakeWorld::new();
    let pos = BlockPos::new(1, 0, 0);
    let recorded_state = BlockStateId(PISTON.0);
    // The live block at `pos` is now stone, not piston -- a different block *type* than the
    // event's own recorded `block_state` (superseded, e.g. broken and replaced before this
    // pass ever reached it).
    world.set_block(pos, BlockStateId(STONE.0));

    let ownership = RegionOwnership::always_local(Address::Region(rc_messaging::RegionId(0)));
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut behaviors = BlockBehaviorRegistry::new();
    let stub = std::sync::Arc::new(StubBehavior::new());
    behaviors.register_one(recorded_state, stub.clone());
    // Stone must resolve to a real registered behavior too, so a (wrongly) successful
    // dispatch would still be observable via this same stub.
    behaviors.register_one(BlockStateId(STONE.0), stub.clone());

    events.emit(BlockEvent {
        pos,
        event_id: 0,
        event_param: 0,
        block_state: recorded_state,
    });

    run_one_pass(
        &mut world,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &behaviors,
        0,
    );

    assert!(
        stub.dispatched.lock().unwrap().is_empty(),
        "a stale event (recorded block no longer matches the live one) must never reach \
         on_block_event at all"
    );
    assert!(
        events.drain_confirmed().is_empty(),
        "a never-dispatched event can never have been confirmed either"
    );
}

#[test]
fn the_outbox_is_empty_after_the_next_drain() {
    let mut world = FakeWorld::new();
    let pos = BlockPos::new(2, 0, 0);
    let state = BlockStateId(PISTON.0);
    world.set_block(pos, state);

    let ownership = RegionOwnership::always_local(Address::Region(rc_messaging::RegionId(0)));
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut behaviors = BlockBehaviorRegistry::new();
    let stub = std::sync::Arc::new(StubBehavior::new());
    behaviors.register_one(state, stub);

    events.emit(BlockEvent {
        pos,
        event_id: 0,
        event_param: 0,
        block_state: state,
    });

    run_one_pass(
        &mut world,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &behaviors,
        0,
    );

    assert_eq!(events.drain_confirmed().len(), 1);
    assert!(
        events.drain_confirmed().is_empty(),
        "draining twice in a row must return nothing the second time"
    );
}
