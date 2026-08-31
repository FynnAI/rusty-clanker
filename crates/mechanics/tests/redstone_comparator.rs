//! M3-B04 — comparator acceptance tests (Context §G).

mod support;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    ComparatorBehavior, ComparatorMode, ContainerSignalSource, RedstoneSignalSource,
    SignalSourceRegistry,
};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine, PendingUpdate,
    RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const FRONT_ID: BlockStateId = BlockStateId(1);
const SIDE_ID: BlockStateId = BlockStateId(2);

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    outbound: Vec<(Address, RegionMessage)>,
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
