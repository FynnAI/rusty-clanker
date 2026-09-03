//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(single canonical value/facing asserted, not a four-way sweep — every case fixes facing=down) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single hopper instance in this file, no ≥3-component chain) nondefault-state=yes
//! M3.5-B06 — `HopperBehavior::on_neighbor_changed`'s own re-evaluation of the raw block-state
//! `ENABLED` bit (Context §3.2, TEST-D57 CONFIRMED against the pinned oracle jar's own
//! `HopperBlock.checkPoweredState`): `ENABLED = !hasNeighborSignal(pos)`, re-checked on every
//! neighbor change, not only at placement (`crates/server/src/play/mining.rs`'s
//! `apply_placement_with_redstone` already covers the placement half).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::block_entity::hopper::{HopperBehavior, HopperStateIds, register_hopper};
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::SignalSourceRegistry;
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, LightDirtyQueue,
    NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, UpdateContext,
};

use support::{FakeWorld, TestSignalSource};

// source: blueprint M3.5-B06 §3.2 ("HOPPER.0 ... = 11313 = enabled=true, facing=down")
const HOPPER_BASE: u32 = 11313;
// A small, arbitrary placeholder id for the neighbor's own signal-source registration —
// mirrors `redstone_update_order_quirks.rs`'s own established convention (any id outside the
// hopper's own range works; nothing here dispatches through `BlockBehaviorRegistry` at all).
const SOURCE_ID: BlockStateId = BlockStateId(5);

/// Builds a fresh `UpdateContext` bundle over `world`/`signals`-independent scratch state —
/// mirrors `redstone_update_order_quirks.rs`'s own identical construction shape. Returns the
/// owned pieces so the caller can call `on_neighbor_changed` more than once against the same
/// `world`/`changed` collector.
struct Scratch {
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    outbound: Vec<(rc_messaging::Address, rc_messaging::RegionMessage)>,
    changed: Vec<(BlockPos, BlockStateId)>,
    light_dirty: LightDirtyQueue,
    ownership: RegionOwnership,
}

impl Scratch {
    fn new(local: rc_messaging::Address) -> Self {
        Self {
            engine: NeighborUpdateEngine::new(),
            scheduled: ScheduledTickQueue::new(),
            events: BlockEventQueue::new(),
            outbound: Vec::new(),
            changed: Vec::new(),
            light_dirty: LightDirtyQueue::new(),
            ownership: RegionOwnership::always_local(local),
        }
    }

    fn ctx<'a>(&'a mut self, world: &'a mut dyn BlockWorldAccess) -> UpdateContext<'a> {
        UpdateContext {
            world,
            engine: &mut self.engine,
            scheduled: &mut self.scheduled,
            events: &mut self.events,
            outbound: &mut self.outbound,
            changed: &mut self.changed,
            ownership: &self.ownership,
            current_tick: 0,
            light_dirty: &mut self.light_dirty,
        }
    }
}

#[test]
fn on_neighbor_changed_disables_when_a_neighbor_starts_emitting_signal_nondefault_case() {
    let pos = BlockPos::new(0, -60, 0);
    let neighbor = Direction::North.apply(pos);

    let mut world = FakeWorld::new();
    world.set_block(pos, BlockStateId(HOPPER_BASE));
    world.set_block(neighbor, SOURCE_ID);

    let source = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        Arc::clone(&source) as Arc<dyn rc_mechanics::redstone::RedstoneSignalSource>,
    );

    let behavior = HopperBehavior::new(Arc::new(signals));
    let mut scratch = Scratch::new(world.local);

    {
        let mut ctx = scratch.ctx(&mut world);
        behavior.on_neighbor_changed(&mut ctx, pos, Direction::North);
    }
    assert!(
        scratch.changed.is_empty(),
        "no neighbor signal yet -> no write: {:?}",
        scratch.changed
    );

    source.set_power(15);
    {
        let mut ctx = scratch.ctx(&mut world);
        behavior.on_neighbor_changed(&mut ctx, pos, Direction::North);
    }
    assert_eq!(
        scratch.changed,
        vec![(pos, BlockStateId(HOPPER_BASE + 5))],
        "a neighbor now emitting signal -> ENABLED flips false (same facing, +5 offset)"
    );
}

#[test]
fn on_neighbor_changed_re_enables_when_the_neighbor_signal_drops_to_zero() {
    let pos = BlockPos::new(0, -60, 0);
    let neighbor = Direction::North.apply(pos);

    let mut world = FakeWorld::new();
    world.set_block(pos, BlockStateId(HOPPER_BASE + 5));
    world.set_block(neighbor, SOURCE_ID);

    let source = Arc::new(TestSignalSource::fixed(15));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        Arc::clone(&source) as Arc<dyn rc_mechanics::redstone::RedstoneSignalSource>,
    );

    let behavior = HopperBehavior::new(Arc::new(signals));
    let mut scratch = Scratch::new(world.local);

    source.set_power(0);
    let mut ctx = scratch.ctx(&mut world);
    behavior.on_neighbor_changed(&mut ctx, pos, Direction::North);

    assert_eq!(
        scratch.changed,
        vec![(pos, BlockStateId(HOPPER_BASE))],
        "the neighbor's own signal dropped to zero -> ENABLED flips back true"
    );
}

#[test]
fn on_neighbor_changed_is_a_no_op_when_enabled_state_already_matches() {
    let pos = BlockPos::new(0, -60, 0);
    let neighbor = Direction::North.apply(pos);

    // enabled id + zero signal -> already matches, no write.
    let mut world = FakeWorld::new();
    world.set_block(pos, BlockStateId(HOPPER_BASE));
    world.set_block(neighbor, SOURCE_ID);

    let source = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        Arc::clone(&source) as Arc<dyn rc_mechanics::redstone::RedstoneSignalSource>,
    );

    let behavior = HopperBehavior::new(Arc::new(signals));
    let mut scratch = Scratch::new(world.local);
    let mut ctx = scratch.ctx(&mut world);
    behavior.on_neighbor_changed(&mut ctx, pos, Direction::North);

    assert!(
        scratch.changed.is_empty(),
        "enabled id + zero signal already matches -> no write: {:?}",
        scratch.changed
    );

    // disabled id + nonzero signal -> already matches, no write.
    let mut world2 = FakeWorld::new();
    world2.set_block(pos, BlockStateId(HOPPER_BASE + 5));
    world2.set_block(neighbor, SOURCE_ID);
    source.set_power(15);

    let mut scratch2 = Scratch::new(world2.local);
    let mut ctx2 = scratch2.ctx(&mut world2);
    behavior.on_neighbor_changed(&mut ctx2, pos, Direction::North);

    assert!(
        scratch2.changed.is_empty(),
        "disabled id + nonzero signal already matches -> no write: {:?}",
        scratch2.changed
    );
}

#[test]
fn register_hopper_registers_into_behaviors_only_never_into_signals() {
    let mut behaviors = BlockBehaviorRegistry::new();
    let signals = Arc::new(SignalSourceRegistry::new());
    let ids = HopperStateIds {
        hopper: (BlockStateId(HOPPER_BASE), BlockStateId(HOPPER_BASE + 10)),
    };

    let _hopper = register_hopper(&mut behaviors, Arc::clone(&signals), &ids);

    assert!(
        !signals
            .resolve(BlockStateId(HOPPER_BASE))
            .is_signal_source(),
        "register_hopper must never register into SignalSourceRegistry — a hopper emits no \
         redstone signal of its own"
    );
}
