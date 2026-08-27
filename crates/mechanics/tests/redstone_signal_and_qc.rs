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
