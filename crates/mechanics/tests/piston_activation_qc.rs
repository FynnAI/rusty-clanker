//! M3-B05 — piston activation (quasi-connectivity) acceptance tests (Context §A): the exact
//! set of positions `piston_neighbor_signal` checks, and the "does not re-check until directly
//! notified" staleness property.

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::piston_neighbor_signal;
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
    piston.place(piston_pos, Direction::East, false);

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
    let ownership = RegionOwnership::always_local(world.local);
    let mut ctx = UpdateContext {
        world: &mut world,
        engine: &mut engine,
        scheduled: &mut scheduled,
        events: &mut events,
        outbound: &mut outbound,
        ownership: &ownership,
        current_tick: 0,
    };
    piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::North);
    assert!(piston.should_be_extended(piston_pos));
}
