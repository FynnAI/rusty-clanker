//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only); varies signal strength, not a block's own property state)
//! M3-B04 — the shared power-query substrate + quasi-connectivity acceptance tests
//! (Context §A/§B/§C).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::BlockWorldAccess;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    self, RedstoneSignalSource, SignalSourceRegistry, TorchAttachment, TorchBehavior,
};
use rc_mechanics::{
    BlockEventQueue, NeighborUpdateEngine, PendingUpdate, RegionOwnership, ScheduledTickQueue,
    UpdateContext,
};

use support::{FakeWorld, TestSignalSource};

/// Not registered in any `SignalSourceRegistry` range in these tests, and not one of
/// `tier1_shape_table()`'s own hand-authored ids — resolves to `default_full_cube()` (Context
/// §B: "unregistered ids default to full-cube/conductor").
const PLAIN: BlockStateId = BlockStateId(9_999_001);
/// `tier1_shape_table()`'s own `redstone_wire` id — a real, non-full tier-1 shape (Context §B),
/// reused here purely as "some block whose registered *shape* is non-full," independent of
/// whether a `WireBehavior` is registered for it in a given test's own `SignalSourceRegistry`.
const NON_FULL: BlockStateId = BlockStateId(5171);
const TORCH_ID: BlockStateId = BlockStateId(1);

#[test]
fn plain_block_is_conductor_by_default() {
    let mut world = FakeWorld::new();
    let pos = BlockPos::new(0, 0, 0);
    world.set_block(pos, PLAIN);

    assert!(redstone::is_conductor(&world, pos));
}

#[test]
fn qc_torch_powers_block_two_above() {
    let mut world = FakeWorld::new();
    let t = BlockPos::new(0, 0, 0);
    let b = Direction::Up.apply(t);
    let w = Direction::Up.apply(b);
    world.set_block(t, TORCH_ID);
    world.set_block(b, PLAIN);
    world.set_block(w, PLAIN);

    let torch: Arc<dyn RedstoneSignalSource> = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(TORCH_ID, BlockStateId(TORCH_ID.0 + 1), torch);

    assert_eq!(
        redstone::signal_into(&world, &registry, w, Direction::Down),
        15
    );
    assert_eq!(redstone::direct_signal_to(&world, &registry, b), 15);
}

#[test]
fn qc_does_not_apply_through_a_non_conductor() {
    let mut world = FakeWorld::new();
    let t = BlockPos::new(0, 0, 0);
    let b = Direction::Up.apply(t);
    let w = Direction::Up.apply(b);
    world.set_block(t, TORCH_ID);
    world.set_block(b, NON_FULL);
    world.set_block(w, PLAIN);

    let torch: Arc<dyn RedstoneSignalSource> = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(TORCH_ID, BlockStateId(TORCH_ID.0 + 1), torch);

    assert_eq!(
        redstone::signal_into(&world, &registry, w, Direction::Down),
        0
    );
}

#[test]
fn weak_signal_gated_by_connects_from() {
    let mut world = FakeWorld::new();
    let pos = BlockPos::new(0, 0, 0);
    world.set_block(pos, BlockStateId(1));

    let source = Arc::new(TestSignalSource::fixed(7));
    source.set_connects_from(Direction::West, false);
    let mut registry = SignalSourceRegistry::new();
    registry.register_range(
        BlockStateId(1),
        BlockStateId(2),
        source as Arc<dyn RedstoneSignalSource>,
    );

    // `connects_from` gates *wire's own connectivity computation*, not `emitted_toward` itself
    // (Context §D/§C) — `emitted_toward` still reports the source's full weak output.
    assert_eq!(
        redstone::emitted_toward(&world, &registry, pos, Direction::West),
        7
    );
}

/// M3 field-report fix (Task 1): `notify_neighbor_changed_only`'s own QC relay --
/// `SignalGetter.getSignal`'s conductor rule (research doc §3.1/Notes) means a conductor's own
/// aggregate signal can change purely because one of its six faces changed, so a position
/// reading *through* that conductor from a *different* face needs its own recompute retriggered
/// too, not only the conductor's immediate neighbor in the direction the original change came
/// from. Geometry: `at` -- East --> `conductor` (a plain, unregistered/full-cube block) -- East
/// --> `far` (two hops from `at`, sharing no face with `at` at all). Confirmed against a real
/// oracle diff (`redstone/qc/wire_strong_vs_weak_power_door`'s own `(1,1,1)`, `docs/findings-
/// for-planning.md`).
#[test]
fn notify_relays_through_a_conductor_neighbor_to_its_own_far_side() {
    let mut world = FakeWorld::new();
    let at = BlockPos::new(0, 0, 0);
    let conductor = Direction::East.apply(at);
    let far = Direction::East.apply(conductor);
    world.set_block(at, BlockStateId(1));
    world.set_block(conductor, PLAIN);
    world.set_block(far, BlockStateId(2));

    let local = world.local;
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let ownership = RegionOwnership::always_local(local);
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

    redstone::notify_neighbor_changed_only(&mut ctx, at);

    let mut reached_far = false;
    engine.drain(&mut |_eng, item| {
        if let PendingUpdate::NeighborChanged { pos, from } = item
            && pos == far
        {
            assert_eq!(
                from,
                Direction::West,
                "far must see the relay coming from its own West"
            );
            reached_far = true;
        }
    });
    assert!(
        reached_far,
        "notify_neighbor_changed_only(at) must relay through its conductor neighbor to reach a \
         position on that conductor's own far side, two hops from `at`"
    );
}
