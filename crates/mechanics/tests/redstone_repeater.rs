//! M3-B04 — repeater acceptance tests (Context §F).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    RedstoneSignalSource, RepeaterBehavior, SignalSourceRegistry, WireBehavior, emitted_toward,
};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo,
    NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, TickPriority, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const REPEATER_ID: BlockStateId = BlockStateId(1);
const INPUT_ID: BlockStateId = BlockStateId(2);
const SIDE_ID: BlockStateId = BlockStateId(3);
const WIRE_ID: BlockStateId = BlockStateId(4);
const SOURCE_ID: BlockStateId = BlockStateId(5);
const BEHIND_ID: BlockStateId = BlockStateId(6);
/// `crates/physics/src/shapes.rs`'s own real "repeater" row (`low_slab`, non-full) --
/// `repeater_chain_relays_signal_end_to_end` uses this real id (rather than one of the
/// arbitrary small ids above) specifically so `signal::is_conductor` correctly reports `false`
/// for every chain position. An arbitrary unregistered id defaults to `default_full_cube` (a
/// conductor), which would let quasi-connectivity (`direct_signal_to`) leak the adjacent
/// source's own power straight through a repeater regardless of whether that repeater's own
/// `weak_signal_toward` is even correct -- silently defeating that regression's whole purpose.
const CHAIN_REPEATER_ID: BlockStateId = BlockStateId(7037);
/// Own-state writeback (M3 field-report fix) exclusive upper bound for this test's own
/// `BlockBehaviorRegistry`/`SignalSourceRegistry` dispatch range: every chain repeater here is
/// placed `facing=East, delay=1` (Deliverables below), so once `RepeaterBehavior::
/// write_state_id` starts writing this component's real own `BlockStateId` back into the world,
/// each position's stored id moves to blocks.json's real `facing=east,delay=1` block
/// (`7046..=7049`, covering every `locked`x`powered` combination) -- outside the single-id
/// `[CHAIN_REPEATER_ID, CHAIN_REPEATER_ID + 1)` range this test used before that write existed.
/// `7050` covers both the placement id (`CHAIN_REPEATER_ID` = 7037, kept unchanged so `is_
/// conductor` still resolves correctly at tick 0, before the first writeback) and every one of
/// those real reachable post-writeback ids, mirroring `registration.rs`'s own established "one
/// wide range per component, regardless of which specific state a position is in" convention.
const CHAIN_REPEATER_RANGE_END: BlockStateId = BlockStateId(7050);

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

fn setup_repeater(
    pos: BlockPos,
    facing: Direction,
    delay_setting: u8,
    extra: Vec<(BlockStateId, BlockStateId, Arc<dyn RedstoneSignalSource>)>,
) -> Arc<RepeaterBehavior> {
    let mut repeater = RepeaterBehavior::new();
    repeater.place(pos, facing, delay_setting);
    let repeater = Arc::new(repeater);
    let mut signals = SignalSourceRegistry::new();
    for (start, end, source) in extra {
        signals.register_range(start, end, source);
    }
    repeater.bind_registry(Arc::new(signals));
    repeater
}

#[test]
fn repeater_delay_matrix() {
    let pos = BlockPos::new(0, 0, 0);
    let mut repeater = RepeaterBehavior::new();
    for (delay_setting, expected) in [(1u8, 2u64), (2, 4), (3, 6), (4, 8)] {
        repeater.place(pos, Direction::East, delay_setting);
        assert_eq!(repeater.get_delay(pos), expected);
    }
}

#[test]
fn repeater_turns_on_and_off_at_its_own_delay() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let input = Arc::new(TestSignalSource::fixed(0));
    let repeater = setup_repeater(
        pos,
        Direction::East,
        2,
        vec![(
            INPUT_ID,
            BlockStateId(INPUT_ID.0 + 1),
            Arc::clone(&input) as Arc<dyn RedstoneSignalSource>,
        )],
    );
    let mut h = Harness::new();
    h.world.set_block(pos, REPEATER_ID);
    h.world.set_block(front, INPUT_ID);

    input.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    assert!(h.scheduled.is_block_tick_pending(pos));
    let due = h.scheduled.drain_due_block_ticks(4);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger_tick, 4);
    assert_eq!(due[0].priority, TickPriority::High);

    {
        let mut ctx = h.ctx_at(4);
        repeater.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(repeater.powered(pos));

    input.set_power(0);
    {
        let mut ctx = h.ctx_at(4);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    let due2 = h.scheduled.drain_due_block_ticks(8);
    assert_eq!(due2.len(), 1);
    assert_eq!(due2[0].trigger_tick, 8);
    assert_eq!(due2[0].priority, TickPriority::VeryHigh);

    {
        let mut ctx = h.ctx_at(8);
        repeater.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(!repeater.powered(pos));
}

#[test]
fn repeater_catches_a_short_pulse() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let input = Arc::new(TestSignalSource::fixed(0));
    let repeater = setup_repeater(
        pos,
        Direction::East,
        2,
        vec![(
            INPUT_ID,
            BlockStateId(INPUT_ID.0 + 1),
            Arc::clone(&input) as Arc<dyn RedstoneSignalSource>,
        )],
    );
    let mut h = Harness::new();
    h.world.set_block(front, INPUT_ID);

    // Rising edge at tick 0 -> schedule turn-on at tick 4 (get_delay(2) = 4), High.
    input.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    assert!(h.scheduled.is_block_tick_pending(pos));

    // Input drops back to 0 well before tick 4 -- `powered` is still false and `should` is now
    // false too, so this second call finds no mismatch and does not reschedule; the original
    // turn-on entry at trigger_tick=4 remains the only pending one.
    input.set_power(0);
    {
        let mut ctx = h.ctx_at(1);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    assert_eq!(h.scheduled.block_len(), 1);

    // At tick 4, the live input has already returned to 0.
    let due = h.scheduled.drain_due_block_ticks(4);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger_tick, 4);
    {
        let mut ctx = h.ctx_at(4);
        repeater.on_scheduled_tick(&mut ctx, pos);
    }

    // The repeater still turns on (honoring the already-scheduled tick), then immediately
    // notices the input is already gone and self-schedules a matching turn-off tick at its own
    // fixed delay width, reproducing the short pulse at that fixed width (Context §F).
    assert!(repeater.powered(pos));
    let due2 = h.scheduled.drain_due_block_ticks(8);
    assert_eq!(due2.len(), 1);
    assert_eq!(due2[0].trigger_tick, 8);
    assert_eq!(due2[0].priority, TickPriority::VeryHigh);

    {
        let mut ctx = h.ctx_at(8);
        repeater.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(!repeater.powered(pos));
}

/// Defect-1 regression, direct formula-level check (Context/ASSET-D18(f) research verdict):
/// `FACING` points toward the repeater's own INPUT side; output flows out the *opposite* side
/// ("a repeater fires away from you" -- placement sets `FACING = playerLookDirection.opposite()`).
/// `weak_signal_toward`/`direct_signal_toward` must therefore answer nonzero only for
/// `towards == facing(pos).opposite()`, never `towards == facing(pos)`. Before this fix,
/// `weak_signal_toward` used `towards == facing(pos)` -- the assertions below would find `West`
/// answering `0` and `East` (wrongly) answering `15`.
#[test]
fn repeater_output_flows_out_the_side_opposite_facing() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let input = Arc::new(TestSignalSource::fixed(15));
    let repeater = setup_repeater(
        pos,
        Direction::East,
        1,
        vec![(
            INPUT_ID,
            BlockStateId(INPUT_ID.0 + 1),
            Arc::clone(&input) as Arc<dyn RedstoneSignalSource>,
        )],
    );
    let mut h = Harness::new();
    h.world.set_block(pos, REPEATER_ID);
    h.world.set_block(front, INPUT_ID);

    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    {
        let mut ctx = h.ctx_at(2);
        repeater.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(repeater.powered(pos));

    assert_eq!(
        repeater.weak_signal_toward(&h.world, pos, Direction::West),
        15,
        "output must flow out facing.opposite() (West) once powered"
    );
    assert_eq!(
        repeater.weak_signal_toward(&h.world, pos, Direction::East),
        0,
        "the input side (facing, East) must never re-see the repeater's own output"
    );
    assert_eq!(
        repeater.direct_signal_toward(&h.world, pos, Direction::West),
        15,
        "direct_signal_toward delegates to weak_signal_toward and must agree"
    );
}

/// Own-state writeback (M3 field-report fix): the repeater's own `POWERED` bit is expressed in
/// its own stored `BlockStateId`, not only in `RepeaterBehavior::powered`'s internal side-table
/// (blocks.json's own `minecraft:repeater` entry, protocol 776: `facing=east,delay=1,
/// locked=false,powered=false` = state 7049, `...powered=true` = state 7048, both cited
/// directly off `datagen-output/26.2/generated/reports/blocks.json`, TEST-D56).
#[test]
fn repeater_own_state_writeback_reflects_powered() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);
    let input = Arc::new(TestSignalSource::fixed(0));
    let repeater = setup_repeater(
        pos,
        Direction::East,
        1,
        vec![(
            INPUT_ID,
            BlockStateId(INPUT_ID.0 + 1),
            Arc::clone(&input) as Arc<dyn RedstoneSignalSource>,
        )],
    );
    let mut h = Harness::new();
    h.world.set_block(pos, BlockStateId(7049)); // east, delay=1, locked=false, powered=false
    h.world.set_block(front, INPUT_ID);

    input.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    {
        let mut ctx = h.ctx_at(2);
        repeater.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(repeater.powered(pos));
    assert_eq!(
        h.world.get_block(pos),
        Some(BlockStateId(7048)),
        "repeater's own stored BlockStateId must flip to the real powered=true id"
    );

    input.set_power(0);
    {
        let mut ctx = h.ctx_at(2);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    {
        let mut ctx = h.ctx_at(4);
        repeater.on_scheduled_tick(&mut ctx, pos);
    }
    assert!(!repeater.powered(pos));
    assert_eq!(h.world.get_block(pos), Some(BlockStateId(7049)));
}

/// Own-state writeback (M3 field-report fix), `LOCKED`: written immediately from
/// `on_neighbor_changed`, never deferred to a scheduled tick (`RepeaterBlock::neighborChanged`'s
/// own additional writeback beyond `DiodeBlock`'s base, Context). blocks.json:
/// `facing=north,delay=1,locked=false,powered=false` = state 7037, `...locked=true,
/// powered=false` = state 7035 (both cited directly off
/// `datagen-output/26.2/generated/reports/blocks.json`, TEST-D56).
#[test]
fn repeater_own_state_writeback_reflects_locked_immediately() {
    let pos = BlockPos::new(0, 0, 0);
    let side_pos = Direction::West.apply(pos); // perpendicular to facing = North

    let mut repeater = RepeaterBehavior::new();
    repeater.place(pos, Direction::North, 1);
    let repeater = Arc::new(repeater);

    let side = Arc::new(TestSignalSource::with_diode_flag(1));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SIDE_ID,
        BlockStateId(SIDE_ID.0 + 1),
        side as Arc<dyn RedstoneSignalSource>,
    );
    repeater.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    h.world.set_block(pos, BlockStateId(7037)); // north, delay=1, locked=false, powered=false
    h.world.set_block(side_pos, SIDE_ID);

    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::West);
    }

    assert!(repeater.is_locked(&h.world, pos));
    assert_eq!(
        h.world.get_block(pos),
        Some(BlockStateId(7035)),
        "LOCKED must be written immediately, without waiting for a scheduled tick"
    );
}

#[test]
fn repeater_lock_is_boolean_not_magnitude() {
    let pos = BlockPos::new(0, 0, 0);
    let side_pos = Direction::North.apply(pos); // perpendicular to `facing = East`

    let mut repeater = RepeaterBehavior::new();
    repeater.place(pos, Direction::East, 1);
    let repeater = Arc::new(repeater);

    let side = Arc::new(TestSignalSource::with_diode_flag(1));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SIDE_ID,
        BlockStateId(SIDE_ID.0 + 1),
        side as Arc<dyn RedstoneSignalSource>,
    );
    repeater.bind_registry(Arc::new(signals));

    let mut world = FakeWorld::new();
    world.set_block(side_pos, SIDE_ID);

    assert!(repeater.is_locked(&world, pos));
}

#[test]
fn repeater_side_wire_does_not_lock() {
    let pos = BlockPos::new(0, 0, 0);
    let side_pos = Direction::North.apply(pos);

    let mut repeater = RepeaterBehavior::new();
    repeater.place(pos, Direction::East, 1);
    let repeater = Arc::new(repeater);

    let wire = Arc::new(WireBehavior::new());
    let source = Arc::new(TestSignalSource::fixed(15));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        WIRE_ID,
        BlockStateId(WIRE_ID.0 + 1),
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);
    wire.bind_registry(Arc::clone(&signals));
    repeater.bind_registry(Arc::clone(&signals));

    let mut h = Harness::new();
    h.world.set_block(side_pos, WIRE_ID);
    h.world
        .set_block(Direction::North.apply(side_pos), SOURCE_ID);
    {
        let mut ctx = h.ctx_at(0);
        wire.on_neighbor_changed(&mut ctx, side_pos, Direction::North);
    }
    assert_eq!(wire.power(side_pos), 15); // nonzero, yet must not lock (not a diode).

    assert!(!repeater.is_locked(&h.world, pos));
}

fn setup_chain(
    behind_facing: Direction,
) -> (
    Arc<RepeaterBehavior>,
    Arc<RepeaterBehavior>,
    Arc<TestSignalSource>,
    Harness,
) {
    let pos = BlockPos::new(0, 0, 0);
    let behind = Direction::West.apply(pos); // facing.opposite().apply(pos), facing = East

    let mut repeater = RepeaterBehavior::new();
    repeater.place(pos, Direction::East, 1);
    let mut behind_repeater = RepeaterBehavior::new();
    behind_repeater.place(behind, behind_facing, 1);
    let repeater = Arc::new(repeater);
    let behind_repeater = Arc::new(behind_repeater);

    let input = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        REPEATER_ID,
        BlockStateId(REPEATER_ID.0 + 1),
        Arc::clone(&repeater) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        BEHIND_ID,
        BlockStateId(BEHIND_ID.0 + 1),
        Arc::clone(&behind_repeater) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        INPUT_ID,
        BlockStateId(INPUT_ID.0 + 1),
        Arc::clone(&input) as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);
    repeater.bind_registry(Arc::clone(&signals));
    behind_repeater.bind_registry(Arc::clone(&signals));

    let mut h = Harness::new();
    h.world.set_block(pos, REPEATER_ID);
    h.world.set_block(behind, BEHIND_ID);
    h.world.set_block(Direction::East.apply(pos), INPUT_ID);

    (repeater, behind_repeater, input, h)
}

/// Unaffected by the defect-1 output-direction fix: `North` is neither `facing` (`East`) nor
/// `facing.opposite()` (`West`), so both the pre-fix `behind_facing != facing` and the
/// corrected `behind_facing != facing.opposite()` agree this perpendicular case is prioritized.
#[test]
fn repeater_should_prioritize_perpendicular_chain() {
    let (repeater, _behind, input, mut h) = setup_chain(Direction::North);

    input.set_power(15);
    let pos = BlockPos::new(0, 0, 0);
    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    let due = h.scheduled.drain_due_block_ticks(2);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].priority, TickPriority::ExtremelyHigh);
}

/// Corrected from the original (inverted-convention) `repeater_straight_through_chain_is_not_
/// prioritized`: `behind`'s own `FACING` matches `pos`'s facing (`East`) -- the ordinary
/// same-direction daisy chain, where `behind`'s own input reads straight from `pos`'s output.
/// Per the ASSET-D18(f) research verdict (Context, defect-1 fix), `should_prioritize(pos) =
/// behind_facing != facing.opposite()`; `East != West` is `true`, so this case IS prioritized
/// (`ExtremelyHigh`) -- not the non-prioritized case the original, bug-matching test asserted.
/// The genuine non-prioritized case is the head-to-head one, tested below.
#[test]
fn repeater_same_facing_chain_is_prioritized() {
    let (repeater, _behind, input, mut h) = setup_chain(Direction::East);

    input.set_power(15);
    let pos = BlockPos::new(0, 0, 0);
    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    let due = h.scheduled.drain_due_block_ticks(2);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].priority, TickPriority::ExtremelyHigh);
}

/// The genuine non-prioritized case (Context, defect-1 fix), new coverage: `behind`'s own
/// `FACING` is `facing.opposite()` (`West`) -- `behind`'s own output flows straight back into
/// `pos`, the two diodes facing directly at each other head-to-head, rather than `behind`
/// feeding forward away from `pos`. `should_prioritize(pos) = West != West = false`.
#[test]
fn repeater_head_to_head_chain_is_not_prioritized() {
    let (repeater, _behind, input, mut h) = setup_chain(Direction::West);

    input.set_power(15);
    let pos = BlockPos::new(0, 0, 0);
    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    let due = h.scheduled.drain_due_block_ticks(2);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].priority, TickPriority::High);
}

/// Defect-1 regression, genuine multi-hop propagation (Context/task): a straight 3-repeater
/// daisy chain, all facing East, fed by a fixed source touching only the FIRST repeater's own
/// input face. Drives real scheduled-tick delays through the actual per-tick driver
/// (`stage4::run_scheduled_phase`) rather than hand-poking `powered` -- the only way to
/// genuinely exercise `weak_signal_toward`'s real query path end to end, the way `on_neighbor_
/// changed`/`on_scheduled_tick` actually consume it in production.
///
/// MUST FAIL against the pre-fix `weak_signal_toward` (`towards == facing(pos)`): R1 turns on
/// at tick 2 exactly as before (unaffected -- that half of the state machine is untouched by
/// the defect), but its `weak_signal_toward(r1_pos, West)` then wrongly answers `0` (`West ==
/// East` is false) instead of `15`, so R2 never sees a nonzero input and the chain never
/// reaches R2, let alone R3.
#[test]
fn repeater_chain_relays_signal_end_to_end() {
    let r1_pos = BlockPos::new(0, 0, 0);
    let r2_pos = Direction::West.apply(r1_pos);
    let r3_pos = Direction::West.apply(r2_pos);
    let source_pos = Direction::East.apply(r1_pos);

    let mut repeater = RepeaterBehavior::new();
    repeater.place(r1_pos, Direction::East, 1);
    repeater.place(r2_pos, Direction::East, 1);
    repeater.place(r3_pos, Direction::East, 1);
    let repeater = Arc::new(repeater);

    let source = Arc::new(TestSignalSource::fixed(15));

    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        CHAIN_REPEATER_ID,
        CHAIN_REPEATER_RANGE_END,
        Arc::clone(&repeater) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        INPUT_ID,
        BlockStateId(INPUT_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);
    repeater.bind_registry(Arc::clone(&signals));

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        CHAIN_REPEATER_ID,
        CHAIN_REPEATER_RANGE_END,
        Arc::clone(&repeater) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new();
    h.world.set_block(r1_pos, CHAIN_REPEATER_ID);
    h.world.set_block(r2_pos, CHAIN_REPEATER_ID);
    h.world.set_block(r3_pos, CHAIN_REPEATER_ID);
    h.world.set_block(source_pos, INPUT_ID);

    // The initial trigger: the source becoming live notifies R1, exactly as `on_neighbor_
    // changed` would fire from a real placement/power-on event -- every other hop propagates
    // through the real per-tick driver below, never a second hand call like this one.
    {
        let mut ctx = h.ctx_at(0);
        repeater.on_neighbor_changed(&mut ctx, r1_pos, Direction::East);
    }
    assert!(h.scheduled.is_block_tick_pending(r1_pos));

    let mut halo = BorderHalo::new();
    for tick in 0..=8u64 {
        rc_mechanics::stage4::run_scheduled_phase(
            &mut h.world,
            &[],
            &mut halo,
            &h.ownership,
            &mut h.engine,
            &mut h.scheduled,
            &mut h.events,
            &behaviors,
            &mut h.outbound,
            tick,
        );
    }

    assert!(repeater.powered(r1_pos), "R1 never turned on");
    assert!(
        repeater.powered(r2_pos),
        "signal did not reach R2 -- repeater output-direction defect"
    );
    assert!(
        repeater.powered(r3_pos),
        "signal did not reach the far end of the chain (R3) -- repeater output-direction defect"
    );
    assert_eq!(
        emitted_toward(&h.world, &signals, r3_pos, Direction::West),
        15,
        "R3's own output face reads 0 -- weak_signal_toward still points the wrong way"
    );
}

#[test]
fn repeater_input_reads_wire_power_directly() {
    let pos = BlockPos::new(0, 0, 0);
    let front = Direction::East.apply(pos);

    let wire = Arc::new(WireBehavior::new());
    let source = Arc::new(TestSignalSource::fixed(7));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        WIRE_ID,
        BlockStateId(WIRE_ID.0 + 1),
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);
    wire.bind_registry(Arc::clone(&signals));

    let mut h = Harness::new();
    h.world.set_block(front, WIRE_ID);
    h.world.set_block(Direction::East.apply(front), SOURCE_ID);

    {
        let mut ctx = h.ctx_at(0);
        wire.on_neighbor_changed(&mut ctx, front, Direction::East);
    }
    assert_eq!(wire.power(front), 7);

    // Wire's own plain (weak) output is gated by connectivity, never computed here (`on_shape_
    // update` never runs) -- only the `raw_wire_power` special-case bypass (Context §F/§C)
    // surfaces its real stored power to a diode's own input reading.
    let input = rc_mechanics::redstone::signal::base_diode_input_signal(
        &h.world,
        &signals,
        pos,
        Direction::East,
    );
    assert_eq!(input, 7);
}
