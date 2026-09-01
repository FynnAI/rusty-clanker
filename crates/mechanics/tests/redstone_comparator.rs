//! M3-B04 — comparator acceptance tests (Context §G).

mod support;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    ComparatorBehavior, ComparatorMode, ContainerSignalSource, RedstoneSignalSource,
    SignalSourceRegistry, TorchAttachment, TorchBehavior, WireBehavior,
};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine, PendingUpdate,
    RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const FRONT_ID: BlockStateId = BlockStateId(1);
const SIDE_ID: BlockStateId = BlockStateId(2);
const TORCH_ID: BlockStateId = BlockStateId(3);
/// A real `minecraft:redstone_wire` id (`WIRE_BASE`, `wire.rs`'s own doc comment) rather than an
/// arbitrary small placeholder -- `rc_physics::tier1_shape_table()` maps every id outside its own
/// hand-authored entries to `default_full_cube()` (`shapes.rs`'s own `lookup` doc comment), so a
/// placeholder id here would wrongly read as a *conductor*, letting `signal::emitted_toward`'s
/// own conductor-relay branch (`direct_signal_to`) accidentally reproduce the same numeric result
/// the fix's real `raw_wire_power` bypass produces -- silently defeating this test's own
/// pre-fix/post-fix discrimination. The real wire range genuinely maps to a non-conductor thin
/// shape (`shapes.rs`'s own `4011u32..=5306` entry), so only the real fix's bypass can surface a
/// nonzero reading here.
const WIRE_ID: BlockStateId = BlockStateId(4011);

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    outbound: Vec<(Address, RegionMessage)>,
    changed: Vec<(BlockPos, BlockStateId)>,
    ownership: RegionOwnership,
}

impl Harness {
    fn new() -> Self {
        let world = FakeWorld::new();
        let local = world.local;
        Self {
            world,
            engine: NeighborUpdateEngine::new(),
            scheduled: ScheduledTickQueue::new(),
            events: BlockEventQueue::new(),
            outbound: Vec::new(),
            changed: Vec::new(),
            ownership: RegionOwnership::always_local(local),
        }
    }

    fn ctx_at(&mut self, current_tick: u64) -> UpdateContext<'_> {
        UpdateContext {
            world: &mut self.world,
            engine: &mut self.engine,
            scheduled: &mut self.scheduled,
            events: &mut self.events,
            outbound: &mut self.outbound,
            changed: &mut self.changed,
            ownership: &self.ownership,
            current_tick,
        }
    }
}

struct FakeContainerSignalSource(Mutex<HashMap<BlockPos, u8>>);

impl ContainerSignalSource for FakeContainerSignalSource {
    fn container_signal(&self, pos: BlockPos) -> Option<u8> {
        self.0.lock().unwrap().get(&pos).copied()
    }
}

#[test]
fn comparator_calculate_output_signal_table() {
    use ComparatorMode::{Compare, Subtract};
    assert_eq!(
        ComparatorBehavior::calculate_output_signal(10, 4, Subtract),
        6
    );
    assert_eq!(
        ComparatorBehavior::calculate_output_signal(10, 10, Subtract),
        0
    );
    assert_eq!(
        ComparatorBehavior::calculate_output_signal(4, 10, Subtract),
        0
    );
    assert_eq!(
        ComparatorBehavior::calculate_output_signal(10, 4, Compare),
        10
    );
    assert_eq!(
        ComparatorBehavior::calculate_output_signal(10, 10, Compare),
        10
    );
    assert_eq!(
        ComparatorBehavior::calculate_output_signal(0, 0, Subtract),
        0
    );
    assert_eq!(
        ComparatorBehavior::calculate_output_signal(0, 5, Compare),
        0
    );
}

#[test]
fn comparator_should_turn_on_table() {
    use ComparatorMode::{Compare, Subtract};
    assert!(ComparatorBehavior::should_turn_on(10, 4, Compare));
    assert!(ComparatorBehavior::should_turn_on(10, 10, Compare));
    assert!(!ComparatorBehavior::should_turn_on(10, 10, Subtract));
    assert!(!ComparatorBehavior::should_turn_on(4, 10, Compare));
    assert!(!ComparatorBehavior::should_turn_on(4, 10, Subtract));
    // M3 field-report (hopper_clock_basic root cause): a zero input never turns the
    // comparator on, even on the Compare-mode 0 == 0 tie -- the same input == 0 guard
    // `calculate_output_signal` already carries. Oracle evidence: hopper_clock_basic's
    // comparator reads an empty container (input 0, side 0) across its drained windows
    // and stays unpowered at every one of those ticks in the captured trace.
    assert!(!ComparatorBehavior::should_turn_on(0, 0, Compare));
    assert!(!ComparatorBehavior::should_turn_on(0, 0, Subtract));
}

/// Defect-1 regression, direct formula-level check (Context/ASSET-D18(f) research verdict):
/// `FACING` points toward the comparator's own INPUT side; output flows out the *opposite*
/// side, matching repeater's own symmetric behavior (`weak_signal_toward(pos, towards) =
/// stored_output(pos)` iff `towards == facing(pos).opposite()`, never `towards == facing(pos)`).
/// Before this fix, `weak_signal_toward` used `towards == facing(pos)` -- the assertions below
/// would find `West` answering `0` and `East` (wrongly) answering the stored output.
#[test]
fn comparator_output_flows_out_the_side_opposite_facing() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Compare);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        front_source as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(front, FRONT_ID);

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(comparator.output(pos), 10);

    assert_eq!(
        comparator.weak_signal_toward(&h.world, pos, Direction::West),
        10,
        "output must flow out facing.opposite() (West)"
    );
    assert_eq!(
        comparator.weak_signal_toward(&h.world, pos, Direction::East),
        0,
        "the input side (facing, East) must never re-see the comparator's own output"
    );
    assert_eq!(
        comparator.direct_signal_toward(&h.world, pos, Direction::West),
        10,
        "direct_signal_toward delegates to weak_signal_toward and must agree"
    );
}

/// M3 field-report fix (Task 1): unlike `RepeaterBehavior` (axis-restricted to `facing`/
/// `facing.opposite()` only), a comparator visually connects a wire touching *any* of its four
/// horizontal faces, including its perpendicular sides -- vanilla's own `RedStoneWireBlock.
/// shouldConnectTo` special-cases only `Blocks.REPEATER` to the axis-restricted check; every
/// other `isSignalSource()` block (comparator included) falls through to the direction-agnostic
/// "any signal source connects from any direction" branch. Confirmed against a real oracle diff
/// (`redstone/comparator/comparator_compare_vs_subtract`'s own `(-1,1,0)`, a wire on the
/// comparator's side face, `docs/findings-for-planning.md`).
#[test]
fn comparator_connects_from_any_side_not_only_front_back() {
    let pos = BlockPos::new(0, 0, 0);
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::North, ComparatorMode::Compare);

    for side in [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ] {
        assert!(
            comparator.connects_from(&FakeWorld::new(), pos, side),
            "a comparator must visually connect a wire from every horizontal side, not only \
             its own front/back axis (side {side:?})"
        );
    }
}

#[test]
fn comparator_reads_container_directly_in_front() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);

    let mut map = HashMap::new();
    map.insert(front, 8u8); // 1 slot, 32/64 -> floor(0.5*14)+1 = 8 (Context §G worked example).
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(map)));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Compare);
    let comparator = Arc::new(comparator);

    // A plain signal at `front` too, with a *different* value -- proves the container reading
    // replaces it entirely rather than being maxed with it (Context §G).
    let plain = Arc::new(TestSignalSource::fixed(3));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        plain as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(front, FRONT_ID);

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(comparator.output(pos), 8);
}

#[test]
fn comparator_falls_back_to_plain_signal_when_no_container() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new()))); // always None
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Compare);
    let comparator = Arc::new(comparator);

    let plain = Arc::new(TestSignalSource::fixed(3));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        plain as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(front, FRONT_ID);

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(comparator.output(pos), 3);
}

#[test]
fn comparator_subtract_mode_analog_only_change_still_notifies() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let side_pos = Direction::North.apply(pos);

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Subtract);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    let side_source = Arc::new(TestSignalSource::fixed(2));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        front_source as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        SIDE_ID,
        BlockStateId(SIDE_ID.0 + 1),
        Arc::clone(&side_source) as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(front, FRONT_ID);
    h.world.set_block(side_pos, SIDE_ID);

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(comparator.output(pos), 8); // 10 - 2
    h.engine.drain(&mut |_eng, _item| {}); // discard the first (expected) notify

    side_source.set_power(5); // should_turn_on(10,5,Subtract) is still true -- only the analog
    // output value (10-5=5) changes, not the boolean.
    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(comparator.output(pos), 5);
    let mut notified = false;
    h.engine.drain(&mut |_eng, item| {
        if let PendingUpdate::NeighborChanged { .. } = item {
            notified = true;
        }
    });
    assert!(
        notified,
        "an analog-only output change in Subtract mode must still notify"
    );
}

#[test]
fn comparator_compare_mode_always_notifies() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Compare);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        front_source as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(front, FRONT_ID);

    for _ in 0..2 {
        {
            let mut ctx = h.ctx_at(0);
            comparator.on_scheduled_tick(&mut ctx, pos);
        }
        let mut notified = false;
        h.engine.drain(&mut |_eng, item| {
            if let PendingUpdate::NeighborChanged { .. } = item {
                notified = true;
            }
        });
        assert!(
            notified,
            "Compare mode must notify even with an unchanged input/side pair"
        );
    }
}

/// Own-state writeback (M3 field-report fix): the comparator's own `POWERED` bit is expressed
/// in its own stored `BlockStateId`, not only in `ComparatorBehavior::powered`'s internal
/// side-table (blocks.json's own `minecraft:comparator` entry, protocol 776: `facing=east,
/// mode=subtract,powered=false` = state 11278, `...powered=true` = state 11277, both cited
/// directly off `datagen-output/26.2/generated/reports/blocks.json`, TEST-D56). `Subtract` mode
/// (rather than `Compare`) deliberately avoids `should_turn_on`'s own documented tie rule
/// (`input == side && mode == Compare` turns on even at `input=side=0` -- Subtract mode's own
/// tie never turns on), so `powered` stays cleanly tied to whether `input` is actually nonzero.
/// The analog `output` value has no `BlockStateId` representation at all (out of scope,
/// block-entity storage) -- only `POWERED` is ever encoded.
#[test]
fn comparator_own_state_writeback_reflects_powered() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Subtract);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        Arc::clone(&front_source) as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(pos, BlockStateId(11278)); // east, subtract, powered=false
    h.world.set_block(front, FRONT_ID);

    // input=0 -> stays off.
    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(!comparator.powered(pos));
    assert_eq!(h.world.get_block(pos), Some(BlockStateId(11278)));

    front_source.set_power(10);
    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(comparator.powered(pos));
    assert_eq!(
        h.world.get_block(pos),
        Some(BlockStateId(11277)),
        "comparator's own stored BlockStateId must flip to the real powered=true id"
    );

    front_source.set_power(0);
    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(!comparator.powered(pos));
    assert_eq!(h.world.get_block(pos), Some(BlockStateId(11278)));
}

#[test]
fn comparator_checktick_compares_stored_output_not_just_powered() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let side_pos = Direction::North.apply(pos);

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Subtract);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    let side_source = Arc::new(TestSignalSource::fixed(2));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        front_source as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        SIDE_ID,
        BlockStateId(SIDE_ID.0 + 1),
        Arc::clone(&side_source) as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    // A real floor -- `on_neighbor_changed`'s own relocated support check (Section A) would
    // otherwise destroy this comparator the moment it fires below.
    h.world
        .set_block(Direction::Down.apply(pos), BlockStateId(9_999_500));
    h.world.set_block(front, FRONT_ID);
    h.world.set_block(side_pos, SIDE_ID);

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(comparator.powered(pos)); // should_turn_on(10, 2, Subtract) == true
    assert_eq!(comparator.output(pos), 8);

    // `should_turn_on(10, 5, Subtract)` is still true (10 > 5) -- `powered` would stay the
    // same -- but the analog output would change (10-5=5 != 8).
    side_source.set_power(5);
    {
        let mut ctx = h.ctx_at(0);
        comparator.on_neighbor_changed(&mut ctx, pos, Direction::North);
    }
    assert!(h.scheduled.is_block_tick_pending(pos));
}

/// M3 field-report fix (Task 2): `on_placed` — placement-state seeding is now re-entrant,
/// mirroring `RepeaterBehavior::on_placed`'s identical fix. A comparator re-placed at an
/// already-registered position must have its own `facing`/`mode` reseeded straight off the
/// freshly-written raw id. blocks.json (protocol 776): `facing=west,mode=subtract,
/// powered=false` = state 11274 (cited directly off
/// `datagen-output/26.2/generated/reports/blocks.json`, TEST-D56).
#[test]
fn comparator_on_placed_reseeds_facing_and_mode_from_the_raw_id() {
    let pos = BlockPos::new(0, 0, 0);
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::North, ComparatorMode::Compare);
    let comparator = Arc::new(comparator);
    comparator.bind_registry(Arc::new(SignalSourceRegistry::new()));

    let mut h = Harness::new();
    h.world.set_block(pos, BlockStateId(11274)); // west, subtract, powered=false

    let mut ctx = h.ctx_at(0);
    comparator.on_placed(&mut ctx, pos);

    assert_eq!(comparator.facing(pos), Direction::West);
    assert_eq!(comparator.mode(pos), ComparatorMode::Subtract);
}

/// M3 field-report fix (regression correction, Section A): `on_placed` no longer self-destructs
/// an unsupported diode -- vanilla never self-validates a command-placed block; a `/setblock`'d
/// comparator with no floor support survives untouched until some neighbor changes (confirmed
/// against a real oracle diff: `redstone/comparator/comparator_2tick_fixed_delay`'s own isolated
/// floor-less comparator stays alive in the oracle trace with no floor ever placed anywhere in
/// that fixture, while the old `on_placed` check destroyed it at tick 0,
/// `docs/findings-for-planning.md`).
#[test]
fn comparator_on_placed_survives_with_no_floor_support() {
    let pos = BlockPos::new(0, 0, 0);
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.bind_registry(Arc::new(SignalSourceRegistry::new()));

    let mut h = Harness::new();
    // No floor placed at all -- `Direction::Down.apply(pos)` stays entirely unset.
    h.world.set_block(pos, BlockStateId(11264)); // north, compare, powered=false

    let mut ctx = h.ctx_at(0);
    comparator.on_placed(&mut ctx, pos);

    assert_eq!(
        h.world.get_block(pos),
        Some(BlockStateId(11264)),
        "on_placed must never self-validate support -- the comparator survives untouched"
    );
}

/// The relocated check (Section A): `on_neighbor_changed` now re-validates support on *every*
/// trigger, direction-agnostically -- support always looks straight down, regardless of which of
/// the six neighbors changed. Triggered here from `East` (a horizontal neighbor, never `Down`)
/// specifically to pin the direction-agnostic claim -- mirrors `RepeaterBehavior`'s own identical
/// regression test.
#[test]
fn comparator_self_destructs_on_neighbor_changed_with_no_floor_support() {
    let pos = BlockPos::new(0, 0, 0);
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::North, ComparatorMode::Compare);
    comparator.bind_registry(Arc::new(SignalSourceRegistry::new()));

    let mut h = Harness::new();
    h.world.set_block(pos, BlockStateId(11264)); // north, compare, powered=false
    // No floor placed at all.

    let mut ctx = h.ctx_at(0);
    comparator.on_neighbor_changed(&mut ctx, pos, Direction::East);

    assert_eq!(
        h.world.get_block(pos),
        Some(BlockStateId(0)),
        "an unsupported comparator must self-destruct on any neighbor-changed trigger, not just \
         a Down-direction one"
    );
}

/// The mirror case: a real conductor floor is present -- `on_neighbor_changed` must not destroy
/// the comparator, continuing straight into its ordinary output/tick-scheduling logic instead.
#[test]
fn comparator_survives_neighbor_changed_with_a_real_floor() {
    let pos = BlockPos::new(0, 0, 0);
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::North, ComparatorMode::Compare);
    comparator.bind_registry(Arc::new(SignalSourceRegistry::new()));

    let mut h = Harness::new();
    h.world
        .set_block(Direction::Down.apply(pos), BlockStateId(9_999_500));
    h.world.set_block(pos, BlockStateId(11264)); // north, compare, powered=false

    let mut ctx = h.ctx_at(0);
    comparator.on_neighbor_changed(&mut ctx, pos, Direction::East);

    assert_eq!(h.world.get_block(pos), Some(BlockStateId(11264)));
}

/// The mirror case: a real conductor floor is present -- the comparator survives and seeds
/// normally, exactly as every other `on_placed` test in this file already establishes.
#[test]
fn comparator_survives_placement_with_a_real_floor() {
    let pos = BlockPos::new(0, 0, 0);
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.bind_registry(Arc::new(SignalSourceRegistry::new()));

    let mut h = Harness::new();
    h.world
        .set_block(Direction::Down.apply(pos), BlockStateId(9_999_500));
    h.world.set_block(pos, BlockStateId(11264)); // north, compare, powered=false

    let mut ctx = h.ctx_at(0);
    comparator.on_placed(&mut ctx, pos);

    assert_eq!(h.world.get_block(pos), Some(BlockStateId(11264)));
    assert_eq!(comparator.facing(pos), Direction::North);
    assert_eq!(comparator.mode(pos), ComparatorMode::Compare);
}

/// `on_placed` is a full replace-on-replace, not a partial update — a comparator re-placed
/// while previously `powered`/holding a nonzero `output` resets both to their fresh-placement
/// default, mirroring `RepeaterBehavior::on_placed_resets_powered_to_the_fresh_placement_
/// default`'s identical case.
#[test]
fn comparator_on_placed_resets_powered_and_output_to_the_fresh_placement_default() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::North.apply(pos);
    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::North, ComparatorMode::Compare);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        front_source as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    // A real floor -- `on_neighbor_changed`'s own relocated support check (Section A) would
    // otherwise destroy this comparator the moment it fires below.
    h.world
        .set_block(Direction::Down.apply(pos), BlockStateId(9_999_500));
    h.world.set_block(pos, BlockStateId(11268)); // north, compare, powered=false
    h.world.set_block(front, FRONT_ID);

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_neighbor_changed(&mut ctx, pos, Direction::North);
    }
    {
        let mut ctx = h.ctx_at(2);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(comparator.powered(pos));
    assert_eq!(comparator.output(pos), 10);

    // Re-placed (a real `/setblock`-shaped write, `powered=false` in the fresh raw id too).
    h.world.set_block(pos, BlockStateId(11268));
    let mut ctx = h.ctx_at(4);
    comparator.on_placed(&mut ctx, pos);

    assert!(!comparator.powered(pos));
    assert_eq!(comparator.output(pos), 0);
}

/// M3 field-report fix (Rule 1, `redstone/clock/comparator_clock_container_fill`): a
/// comparator's own side reading (`getControlInputSignal(pos, direction, onlyDiodes = false)`)
/// takes a non-diode neighbor's own DIRECT signal, never its weak one. Before this fix,
/// `side_input_signal` routed through `signal::signal_into` -- the general quasi-connectivity
/// *weak*-signal primitive -- so a lit floor torch standing beside a comparator (queried from a
/// horizontal direction) wrongly contributed its unconditional weak `15`. `TorchBehavior::
/// direct_signal_toward` is nonzero only straight `Up` from a floor torch (its own doc comment),
/// so a horizontal side query must now read `0`: subtract-mode `10 - 0 = 10`, not the pre-fix
/// `10 - 15` clamped to `0`.
#[test]
fn comparator_side_input_ignores_a_lit_floor_torchs_weak_signal() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let side_pos = Direction::North.apply(pos); // perpendicular to facing = East

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Subtract);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    // Lit by default (`TorchBehavior::lit`'s own doc comment: "`true` if never observed") --
    // never bound to a registry of its own, since neither `weak_signal_toward` nor `direct_
    // signal_toward` reads it (only `has_neighbor_signal`, unused by this test, does).
    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Floor));

    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        front_source as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        TORCH_ID,
        BlockStateId(TORCH_ID.0 + 1),
        torch as Arc<dyn RedstoneSignalSource>,
    );
    comparator.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(front, FRONT_ID);
    h.world.set_block(side_pos, TORCH_ID);

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(
        comparator.output(pos),
        10,
        "side must read 0 from the horizontally-adjacent lit torch (10 - 0), not its weak 15"
    );
}

/// M3 field-report fix (Rule 1): the shared `only_diodes = false` branch's `raw_wire_power`
/// bypass (Context §F/§C) surfaces a side-adjacent wire's real stored power directly, exactly as
/// `repeater_input_reads_wire_power_directly`'s own front-input case already established --
/// never gated by that wire's own `connections` (which stays the unset, all-`false` default
/// here, since `on_shape_update` never runs): `weak_signal_toward` alone would read `0` from
/// every direction, so a passing result here proves the bypass, not a coincidence of connectivity.
#[test]
fn comparator_side_input_reads_wire_raw_power_bypassing_connections() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let side_pos = Direction::North.apply(pos); // perpendicular to facing = East
    let wire_source_pos = Direction::North.apply(side_pos);

    let containers = Arc::new(FakeContainerSignalSource(Mutex::new(HashMap::new())));
    let comparator = ComparatorBehavior::new(containers);
    comparator.place(pos, Direction::East, ComparatorMode::Subtract);
    let comparator = Arc::new(comparator);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    let wire_source = Arc::new(TestSignalSource::fixed(9));
    let wire = Arc::new(WireBehavior::new());

    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        FRONT_ID,
        BlockStateId(FRONT_ID.0 + 1),
        front_source as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        SIDE_ID,
        BlockStateId(SIDE_ID.0 + 1),
        wire_source as Arc<dyn RedstoneSignalSource>,
    );
    // The full real `minecraft:redstone_wire` range, not just `WIRE_ID` itself -- `wire.on_
    // neighbor_changed`'s own writeback below re-encodes `side_pos`'s stored id to reflect its
    // freshly-computed power (`WireBehavior::new_power_state_id`), landing on a *different* real
    // wire id than `WIRE_ID`; a narrower registered range would then fail to resolve back to
    // `wire` on the read below, silently falling through to `NoSignalSource`.
    signals.register_range(
        WIRE_ID,
        BlockStateId(5307), // `wire.rs::WIRE_MAX` (5306) + 1
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);
    wire.bind_registry(Arc::clone(&signals));
    comparator.bind_registry(Arc::clone(&signals));

    let mut h = Harness::new();
    h.world.set_block(front, FRONT_ID);
    h.world.set_block(side_pos, WIRE_ID);
    h.world.set_block(wire_source_pos, SIDE_ID);

    {
        let mut ctx = h.ctx_at(0);
        wire.on_neighbor_changed(&mut ctx, side_pos, Direction::North);
    }
    assert_eq!(wire.power(side_pos), 9);
    assert_eq!(
        wire.weak_signal_toward(&h.world, side_pos, Direction::South),
        0,
        "connections were never computed (on_shape_update never ran) -- plain weak output must \
         stay gated shut, so a nonzero comparator reading below can only come from the raw bypass"
    );

    {
        let mut ctx = h.ctx_at(0);
        comparator.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(
        comparator.output(pos),
        1,
        "10 - 9 -- side must read the wire's raw stored power (9), not its gated weak output (0)"
    );
}
