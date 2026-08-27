//! M3-B04 — repeater acceptance tests (Context §F).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    RedstoneSignalSource, RepeaterBehavior, SignalSourceRegistry, WireBehavior,
};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine, RegionOwnership,
    ScheduledTickQueue, TickPriority, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const REPEATER_ID: BlockStateId = BlockStateId(1);
const INPUT_ID: BlockStateId = BlockStateId(2);
const SIDE_ID: BlockStateId = BlockStateId(3);
const WIRE_ID: BlockStateId = BlockStateId(4);
const SOURCE_ID: BlockStateId = BlockStateId(5);
const BEHIND_ID: BlockStateId = BlockStateId(6);

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

#[test]
fn repeater_straight_through_chain_is_not_prioritized() {
    let (repeater, _behind, input, mut h) = setup_chain(Direction::East);

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
