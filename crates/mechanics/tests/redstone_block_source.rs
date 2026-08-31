//! M3 field-report fix — `minecraft:redstone_block` as an always-on `RedstoneSignalSource`
//! (Task 1): `RedstoneBlockSource`'s own constant-15/all-directions/all-faces output, and
//! `register_redstone_block`'s own registration wiring through the shared `signal.rs`
//! primitives (`emitted_toward`/`signal_into`), mirroring `redstone_signal_and_qc.rs`'s own
//! established acceptance-test shape for the four M3-B04 tier-1 components.

mod support;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::BlockWorldAccess;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    self, RedstoneBlockSource, RedstoneSignalSource, SignalSourceRegistry, register_redstone_block,
};

use support::FakeWorld;

/// blocks.json's own `minecraft:redstone_block` entry (protocol 776) — its one, only, always
/// `default: true` state id.
const REDSTONE_BLOCK_ID: BlockStateId = BlockStateId(11311);
/// An ordinary, unregistered full-cube block — `PLAIN` in `redstone_signal_and_qc.rs`'s own
/// naming convention, reused here for a neighbor position with no signal source of its own.
const PLAIN: BlockStateId = BlockStateId(9_999_002);

#[test]
fn redstone_block_source_emits_constant_fifteen_on_every_face() {
    let world = FakeWorld::new();
    let pos = BlockPos::new(0, 0, 0);
    let source = RedstoneBlockSource;

    for towards in [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
        Direction::Up,
        Direction::Down,
    ] {
        assert_eq!(source.weak_signal_toward(&world, pos, towards), 15);
        assert_eq!(source.direct_signal_toward(&world, pos, towards), 15);
    }
    assert!(source.is_signal_source());
    assert!(!source.is_diode());
    assert_eq!(source.diode_facing(pos), None);
}

#[test]
fn register_redstone_block_wires_it_into_signal_into_from_every_side() {
    let mut world = FakeWorld::new();
    let block_pos = BlockPos::new(0, 0, 0);
    world.set_block(block_pos, REDSTONE_BLOCK_ID);

    let mut signals = SignalSourceRegistry::new();
    register_redstone_block(&mut signals);

    for dir in [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
        Direction::Up,
        Direction::Down,
    ] {
        let neighbor = dir.apply(block_pos);
        world.set_block(neighbor, PLAIN);
        assert_eq!(
            redstone::signal_into(&world, &signals, neighbor, dir.opposite()),
            15,
            "neighbor at {dir:?} must read full power from an adjacent redstone_block"
        );
    }
}

#[test]
fn redstone_block_id_is_itself_a_conductor() {
    // `redstone_block`'s own real state id has no explicit `rc_physics::tier1_shape_table()`
    // entry, so it falls through to that table's own default-full-cube fallback (Context §B) —
    // it is itself a conductor, letting quasi-connectivity carry a *neighbor's* signal one hop
    // further through it exactly as through any other plain solid block.
    let mut world = FakeWorld::new();
    let pos = BlockPos::new(0, 0, 0);
    world.set_block(pos, REDSTONE_BLOCK_ID);
    assert!(redstone::is_conductor(&world, pos));
}
