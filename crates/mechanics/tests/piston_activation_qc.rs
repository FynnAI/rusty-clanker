//! M3-B05 — piston activation (quasi-connectivity) acceptance tests (Context §A): the exact
//! set of positions `piston_neighbor_signal` checks, and the "does not re-check until directly
//! notified" staleness property.

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::{TRIGGER_CONTRACT, piston_neighbor_signal};
use rc_mechanics::redstone::{
    PistonBehavior, RedstoneSignalSource, SignalSourceRegistry, TorchAttachment, TorchBehavior,
};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine, RegionOwnership,
    ScheduledTickQueue, UpdateContext,
};

use support::{FakeWorld, TestSignalSource};

const SOURCE_ID: BlockStateId = BlockStateId(1);
const TORCH_ID: BlockStateId = BlockStateId(2);
/// Not registered in any `SignalSourceRegistry` range — resolves to `NoSignalSource`, and (per
/// `rc_physics::tier1_shape_table()`'s own default fallback) a full-cube conductor.
const PLAIN: BlockStateId = BlockStateId(9_999_001);

#[test]
fn piston_extends_from_a_direct_side_signal() {
    let mut world = FakeWorld::new();
    let piston_pos = BlockPos::new(0, 0, 0);
    let source_pos = Direction::North.apply(piston_pos);
    world.set_block(source_pos, SOURCE_ID);

    let source = Arc::new(TestSignalSource::fixed(15));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );

    // North is neither the push direction (East) nor Down -- one of "every one of the piston's
    // own 6 faces except DOWN and except the push direction" (Context §A step 1).
    assert!(piston_neighbor_signal(
        &world,
        &registry,
        piston_pos,
        Direction::East
    ));
}

#[test]
fn piston_extends_from_below() {
    let mut world = FakeWorld::new();
    let piston_pos = BlockPos::new(0, 0, 0);
    let source_pos = Direction::Down.apply(piston_pos);
    world.set_block(source_pos, SOURCE_ID);

    let source = Arc::new(TestSignalSource::fixed(15));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );

    assert!(piston_neighbor_signal(
        &world,
        &registry,
        piston_pos,
        Direction::East
    ));
}

#[test]
fn qc_torch_powers_piston_two_above() {
    let mut world = FakeWorld::new();
    let t = BlockPos::new(0, 0, 0);
    let b = Direction::Up.apply(t);
    let p = Direction::Up.apply(b);
    world.set_block(t, TORCH_ID);
    world.set_block(b, PLAIN);
    world.set_block(p, PLAIN);

    let torch: Arc<dyn RedstoneSignalSource> = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(TORCH_ID, BlockStateId(TORCH_ID.0 + 1), torch);

    // Step 3 (Context §A): quasi-connectivity through the block directly above the piston --
    // one of `b`'s own 4 horizontal faces (here, `b`'s Down face, read from `t`) carries the
    // torch's direct signal, which `b`'s own conductor status raises into `p`'s horizontal
    // check.
    assert!(piston_neighbor_signal(
        &world,
        &registry,
        p,
        Direction::East
    ));
}

#[test]
fn piston_never_reads_its_own_push_direction() {
    let mut world = FakeWorld::new();
    let piston_pos = BlockPos::new(0, 0, 0);
    let source_pos = Direction::East.apply(piston_pos);
    world.set_block(source_pos, SOURCE_ID);

    let source = Arc::new(TestSignalSource::fixed(15));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );

    // The only powered face is East, which is also the push direction -- never read (Context
    // §A step 1's own exclusion).
    assert!(!piston_neighbor_signal(
        &world,
        &registry,
        piston_pos,
        Direction::East
    ));
}

#[test]
fn piston_stays_stale_until_directly_notified() {
    let mut world = FakeWorld::new();
    let piston_pos = BlockPos::new(0, 0, 0);
    // `c` sits on the piston's own North face (non-push, non-down) -- the piston reads it via
    // ordinary conductor propagation (`c` itself, not `c.up()`, mirroring
    // `qc_torch_powers_piston_two_above`'s own shape one hop shorter, Acceptance tests' own
    // framing).
    let c = Direction::North.apply(piston_pos);
    let source_pos = Direction::Down.apply(c);
    world.set_block(piston_pos, BlockStateId(100));
    world.set_block(c, PLAIN);
    world.set_block(source_pos, SOURCE_ID);

    let source = Arc::new(TestSignalSource::fixed(0));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        Arc::clone(&source) as Arc<dyn RedstoneSignalSource>,
    );
    let registry = Arc::new(registry);

    let piston = PistonBehavior::new(Arc::clone(&registry));
    piston.place(piston_pos, Direction::East, false, false);

    assert!(!piston_neighbor_signal(
        &world,
        &registry,
        piston_pos,
        Direction::East
    ));
    assert!(!piston.should_be_extended(piston_pos));

    // The source changes power, but nothing ever calls `on_neighbor_changed` on the piston --
    // simulating "the source changed but nothing propagated a notify to P" (Context §A).
    source.set_power(15);
    assert!(
        piston_neighbor_signal(&world, &registry, piston_pos, Direction::East),
        "a fresh query bypassing the behavior would now see the change"
    );
    assert!(
        !piston.should_be_extended(piston_pos),
        "the piston's own cached flag must stay exactly where it last was"
    );

    // The notify finally arrives.
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let ownership = RegionOwnership::always_local(world.local);
    let mut ctx = UpdateContext {
        world: &mut world,
        engine: &mut engine,
        scheduled: &mut scheduled,
        events: &mut events,
        outbound: &mut outbound,
        changed: &mut changed,
        ownership: &ownership,
        current_tick: 0,
    };
    piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::North);
    assert!(piston.should_be_extended(piston_pos));
}

/// M3 field-report fix (phantom-extend-on-already-extended-placement defect,
/// `docs/findings-for-planning.md`'s own "two of the four originally-failing piston
/// fixtures" entry): both `redstone/piston/piston_retract_pull_sticky_vs_normal` and
/// `redstone/piston/sticky_piston_retractor_door_2x2` place their piston already extended via
/// a raw `blocks:` state id (e.g. `piston[facing=east,extended=true]`, `state_id: 2258`), with
/// the triggering `redstone_block`'s own placement listed *after* it in the same list --
/// `rc_gametest::replay::replay_contraption` settles `spec.blocks` in list order, so the
/// redstone_block's own placement fans a genuine `on_neighbor_changed` to the already-placed
/// piston before the fixture's own scripted action ever runs. `place`'s own former unconditional
/// `should_be_extended: false` seeding made this look like a real `false -> true` transition and
/// queued a spurious `TRIGGER_EXTEND` -- real vanilla never runs an extend animation from a raw
/// `/setblock` of an already-extended id; the state is simply already extended, and a signal
/// that already matches it triggers nothing at all.
#[test]
fn already_extended_placement_with_signal_present_queues_no_extend_event() {
    let mut world = FakeWorld::new();
    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    // The real EXTENDED=true id for a plain piston, facing=east (`piston_state_id`'s own doc
    // comment citation) -- the exact id `piston_retract_pull_sticky_vs_normal.ron`'s own
    // `blocks:` entry declares for its non-sticky piston (`state_id: 2258`).
    world.set_block(piston_pos, BlockStateId(2258));

    let source_pos = Direction::Down.apply(piston_pos);
    world.set_block(source_pos, SOURCE_ID);
    let source = Arc::new(TestSignalSource::fixed(15));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );
    let registry = Arc::new(registry);

    let piston = PistonBehavior::new(Arc::clone(&registry));
    // The fix under test: seeding with `extended: true` (mirroring the placed id's own real
    // `extended` property) must also seed `should_be_extended: true` -- not unconditionally
    // `false`.
    piston.place(piston_pos, facing, false, true);
    assert!(piston.should_be_extended(piston_pos));

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let ownership = RegionOwnership::always_local(world.local);
    let mut ctx = UpdateContext {
        world: &mut world,
        engine: &mut engine,
        scheduled: &mut scheduled,
        events: &mut events,
        outbound: &mut outbound,
        changed: &mut changed,
        ownership: &ownership,
        current_tick: 0,
    };
    // Mirrors the fixture's own setup order: the redstone_block's own placement (already
    // powering the piston's Down face) fans this call to the already-placed, already-extended
    // piston.
    piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);

    assert!(
        piston.should_be_extended(piston_pos),
        "already true, and the freshly-computed value is also true -- no transition"
    );
    assert!(
        events.drain_all().is_empty(),
        "no spurious TRIGGER_EXTEND -- should_be_extended already matched the placed id's own \
         extended property at seed time, so on_neighbor_changed's own `changed` gate never fires"
    );
    assert!(
        !scheduled.is_block_tick_pending(piston_pos),
        "nothing was ever queued to finalize, so no commit is pending either"
    );
}

/// Companion to `already_extended_placement_with_signal_present_queues_no_extend_event`: an
/// already-extended placement whose signal is genuinely absent must still queue a real retract
/// on the first notify -- the fix narrows seeding to match the placed id's own `extended`
/// property, it must never suppress a real `true -> false` transition.
#[test]
fn already_extended_placement_with_signal_absent_queues_retract() {
    let mut world = FakeWorld::new();
    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    world.set_block(piston_pos, BlockStateId(2258)); // extended=true, facing=east

    let source_pos = Direction::Down.apply(piston_pos);
    world.set_block(source_pos, SOURCE_ID);
    // No signal ever reaches the piston -- mirrors the fixtures' own later scripted action
    // (the redstone_block's own removal), collapsed here into "was never powered to begin
    // with" since only the resulting false-QC state matters to `on_neighbor_changed`.
    let source = Arc::new(TestSignalSource::fixed(0));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );
    let registry = Arc::new(registry);

    let piston = PistonBehavior::new(Arc::clone(&registry));
    piston.place(piston_pos, facing, false, true);
    assert!(piston.should_be_extended(piston_pos));

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let ownership = RegionOwnership::always_local(world.local);
    let mut ctx = UpdateContext {
        world: &mut world,
        engine: &mut engine,
        scheduled: &mut scheduled,
        events: &mut events,
        outbound: &mut outbound,
        changed: &mut changed,
        ownership: &ownership,
        current_tick: 0,
    };
    piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);

    assert!(
        !piston.should_be_extended(piston_pos),
        "the signal is genuinely absent -- a real true -> false transition"
    );
    let mut queued = events.drain_all();
    assert_eq!(queued.len(), 1, "exactly one retract event queued");
    let event = queued.remove(0);
    assert_eq!(
        event.event_id, TRIGGER_CONTRACT,
        "non-sticky, nothing to pull -- TRIGGER_CONTRACT, not TRIGGER_DROP"
    );

    // Drive the queued event through `on_block_event` -- `on_neighbor_changed` alone only ever
    // enqueues it (`emit_block_event`'s own contract); the deferred commit is scheduled by
    // `on_block_event`'s own TRIGGER_CONTRACT arm, exactly like any other retract.
    let mut ctx = UpdateContext {
        world: &mut world,
        engine: &mut engine,
        scheduled: &mut scheduled,
        events: &mut events,
        outbound: &mut outbound,
        changed: &mut changed,
        ownership: &ownership,
        current_tick: 0,
    };
    piston.on_block_event(&mut ctx, piston_pos, &event);
    assert!(
        scheduled.is_block_tick_pending(piston_pos),
        "the retract's own commit is scheduled exactly as an ordinary retract would be"
    );
}
