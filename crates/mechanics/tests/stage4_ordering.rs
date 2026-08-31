//! M3-B01 — Stage-4 driver acceptance tests, integration over the ECS-agnostic core (an
//! in-memory `BlockWorldAccess` test double, no `bevy_ecs::World`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::direction::{Direction, NEIGHBOR_CHANGED_ORDER, SHAPE_UPDATE_ORDER};
use rc_mechanics::stage4::{run_block_event_subphase, run_scheduled_phase};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEvent, BlockEventQueue, BlockWorldAccess,
    BorderHalo, NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, TickPriority,
    UpdateContext,
};
use rc_messaging::{Address, RegionId, RegionMessage};

/// Test double `FakeWorld` (in this file only): a `HashMap<BlockPos, BlockStateId>` plus a
/// fixed `ChunkKey -> Address` map and a `local: Address`, implementing `BlockWorldAccess`.
struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    local: Address,
}

impl FakeWorld {
    fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            local: Address::Region(RegionId(0)),
        }
    }
}

impl BlockWorldAccess for FakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        self.local
    }
    fn local_identity(&self) -> Address {
        self.local
    }
}

fn all_six(origin: BlockPos) -> Vec<BlockPos> {
    [
        Direction::West,
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Down,
        Direction::Up,
    ]
    .into_iter()
    .map(|d| d.apply(origin))
    .collect()
}

fn harness() -> (
    NeighborUpdateEngine,
    ScheduledTickQueue,
    BlockEventQueue,
    BorderHalo,
    Vec<(Address, RegionMessage)>,
    RegionOwnership,
) {
    (
        NeighborUpdateEngine::new(),
        ScheduledTickQueue::new(),
        BlockEventQueue::new(),
        BorderHalo::new(),
        Vec::new(),
        RegionOwnership::always_local(Address::Region(RegionId(0))),
    )
}

// --- Test 1: set_block_fans_out_both_signals_locally ------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FanoutKind {
    NeighborChanged,
    ShapeUpdate,
}

struct TriggerBehavior {
    log: Arc<Mutex<Vec<(BlockPos, Direction, FanoutKind)>>>,
    trigger_pos: BlockPos,
    new_state: BlockStateId,
}

impl BlockBehavior for TriggerBehavior {
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        if pos == self.trigger_pos {
            ctx.set_block(pos, self.new_state);
        }
    }
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        self.log
            .lock()
            .unwrap()
            .push((pos, from, FanoutKind::NeighborChanged));
    }
    fn on_shape_update(
        &self,
        _ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        self.log
            .lock()
            .unwrap()
            .push((pos, from, FanoutKind::ShapeUpdate));
        None
    }
}

#[test]
fn set_block_fans_out_both_signals_locally() {
    let origin = BlockPos::new(0, 0, 0);
    let mut world = FakeWorld::new();
    world.set_block(origin, BlockStateId(1));
    for p in all_six(origin) {
        world.set_block(p, BlockStateId(1));
    }

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(TriggerBehavior {
            log: Arc::clone(&log),
            trigger_pos: origin,
            new_state: BlockStateId(2),
        }),
    );

    let (mut engine, mut scheduled, mut events, mut halo, mut outbound, ownership) = harness();
    scheduled.schedule_block_tick(origin, 0, TickPriority::Normal, 0);

    run_scheduled_phase(
        &mut world,
        &[],
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        0,
    );

    let logged = log.lock().unwrap();

    let mut expected: Vec<(BlockPos, Direction, FanoutKind)> = NEIGHBOR_CHANGED_ORDER
        .into_iter()
        .map(|d| (d.apply(origin), d.opposite(), FanoutKind::NeighborChanged))
        .collect();
    expected.extend(
        SHAPE_UPDATE_ORDER
            .into_iter()
            .map(|d| (d.apply(origin), d.opposite(), FanoutKind::ShapeUpdate)),
    );

    assert_eq!(*logged, expected);
    assert_eq!(logged.len(), 12);
}

// --- Test 2: scheduled_phase_settles_neighbor_updates_between_each_due_tick --------------

struct SettleBehavior {
    log: Arc<Mutex<Vec<String>>>,
    trigger_pos: BlockPos,
    new_state: BlockStateId,
}

impl BlockBehavior for SettleBehavior {
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        self.log
            .lock()
            .unwrap()
            .push(format!("scheduled_tick:{pos:?}"));
        if pos == self.trigger_pos {
            ctx.set_block(pos, self.new_state);
        }
    }
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        self.log
            .lock()
            .unwrap()
            .push(format!("neighbor_changed:{pos:?}"));
    }
    fn on_shape_update(
        &self,
        _ctx: &mut UpdateContext,
        pos: BlockPos,
        _from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        self.log
            .lock()
            .unwrap()
            .push(format!("shape_update:{pos:?}"));
        None
    }
}

#[test]
fn scheduled_phase_settles_neighbor_updates_between_each_due_tick() {
    let pos_a = BlockPos::new(0, 0, 0);
    let pos_b = BlockPos::new(1000, 0, 1000);

    let mut world = FakeWorld::new();
    world.set_block(pos_a, BlockStateId(1));
    world.set_block(pos_b, BlockStateId(1));
    for p in all_six(pos_a) {
        world.set_block(p, BlockStateId(1));
    }

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(SettleBehavior {
            log: Arc::clone(&log),
            trigger_pos: pos_a,
            new_state: BlockStateId(2),
        }),
    );

    let (mut engine, mut scheduled, mut events, mut halo, mut outbound, ownership) = harness();
    // pos_a scheduled first, at a strictly higher priority than pos_b -- both due at tick 5.
    scheduled.schedule_block_tick(pos_a, 5, TickPriority::High, 0);
    scheduled.schedule_block_tick(pos_b, 5, TickPriority::Normal, 0);

    run_scheduled_phase(
        &mut world,
        &[],
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        5,
    );

    let logged = log.lock().unwrap();
    assert_eq!(logged.len(), 1 + 12 + 1);
    assert_eq!(logged[0], format!("scheduled_tick:{pos_a:?}"));
    for entry in &logged[1..13] {
        assert!(entry.starts_with("neighbor_changed:") || entry.starts_with("shape_update:"));
        assert!(!entry.contains(&format!("{pos_b:?}")));
    }
    assert_eq!(logged[13], format!("scheduled_tick:{pos_b:?}"));
}

// --- Test 3: block_before_fluid_ordering -------------------------------------------------

struct OrderLogBehavior {
    log: Arc<Mutex<Vec<BlockPos>>>,
}

impl BlockBehavior for OrderLogBehavior {
    fn on_scheduled_tick(&self, _ctx: &mut UpdateContext, pos: BlockPos) {
        self.log.lock().unwrap().push(pos);
    }
}

#[test]
fn block_before_fluid_ordering() {
    let block_pos = BlockPos::new(0, 0, 0);
    let fluid_pos = BlockPos::new(1, 0, 0);
    let mut world = FakeWorld::new();
    world.set_block(block_pos, BlockStateId(1));
    world.set_block(fluid_pos, BlockStateId(1));

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(OrderLogBehavior {
            log: Arc::clone(&log),
        }),
    );

    let (mut engine, mut scheduled, mut events, mut halo, mut outbound, ownership) = harness();
    // A naive combined-priority merge would drain fluid first (ExtremelyHigh < ExtremelyLow).
    scheduled.schedule_fluid_tick(fluid_pos, 0, TickPriority::ExtremelyHigh, 0);
    scheduled.schedule_block_tick(block_pos, 0, TickPriority::ExtremelyLow, 0);

    run_scheduled_phase(
        &mut world,
        &[],
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        0,
    );

    assert_eq!(*log.lock().unwrap(), vec![block_pos, fluid_pos]);
}

// --- Test 4: block_event_subphase_runs_after_scheduled_phase_within_the_same_stage4_pass -

struct EmitThenLogBehavior {
    log: Arc<Mutex<Vec<(BlockPos, u8)>>>,
}

impl BlockBehavior for EmitThenLogBehavior {
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        ctx.emit_block_event(pos, 7, 0, BlockStateId(1));
    }
    fn on_block_event(&self, _ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {
        self.log.lock().unwrap().push((pos, event.event_id));
    }
}

#[test]
fn block_event_subphase_runs_after_scheduled_phase_within_the_same_stage4_pass() {
    let pos = BlockPos::new(0, 0, 0);
    let mut world = FakeWorld::new();
    world.set_block(pos, BlockStateId(1));

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(EmitThenLogBehavior {
            log: Arc::clone(&log),
        }),
    );

    let (mut engine, mut scheduled, mut events, mut halo, mut outbound, ownership) = harness();
    scheduled.schedule_block_tick(pos, 0, TickPriority::Normal, 0);

    run_scheduled_phase(
        &mut world,
        &[],
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        0,
    );
    assert!(
        log.lock().unwrap().is_empty(),
        "block event must not process before the subphase runs"
    );

    run_block_event_subphase(
        &mut world,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        0,
    );

    assert_eq!(*log.lock().unwrap(), vec![(pos, 7)]);
}

// --- Test 5: block_event_emitted_during_subphase_fires_within_the_same_call -------------

struct ReemittingBehavior {
    log: Arc<Mutex<Vec<u8>>>,
}

impl BlockBehavior for ReemittingBehavior {
    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {
        self.log.lock().unwrap().push(event.event_id);
        if event.event_id == 1 {
            ctx.emit_block_event(pos, 2, 0, event.block_state);
        }
    }
}

/// MECH-D9 (corrected): an event `emit_block_event`-ed directly from inside another event's own
/// `on_block_event` handler is picked up by that *same* `run_block_event_subphase` call's own
/// re-entrant drain loop -- same tick, same pass. Replaces this file's own former (identically
/// named up to `_is_deferred_to_next_call`) test, which asserted the opposite: that event 2
/// would sit unprocessed (`log == vec![1]`) until a *second* top-level call. That assertion
/// encoded M3's own since-disproven double-buffered design -- `05-game-mechanics.md`'s MECH-D9
/// row (the reference-audited spec) states vanilla's real `ServerLevel.runBlockEvents()` fires
/// such a same-pass re-queue in the very call that produced it, which this test now asserts
/// directly (see this changeset's own commit body for the full justification).
#[test]
fn block_event_emitted_during_subphase_fires_within_the_same_call() {
    let pos = BlockPos::new(0, 0, 0);
    let mut world = FakeWorld::new();
    world.set_block(pos, BlockStateId(1));

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(ReemittingBehavior {
            log: Arc::clone(&log),
        }),
    );

    let (mut engine, mut scheduled, mut events, _halo, mut outbound, ownership) = harness();
    events.emit(BlockEvent {
        pos,
        event_id: 1,
        event_param: 0,
        block_state: BlockStateId(1),
    });

    run_block_event_subphase(
        &mut world,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        0,
    );
    assert_eq!(
        *log.lock().unwrap(),
        vec![1, 2],
        "event 2, re-emitted from inside event 1's own handler, must fire in this SAME call"
    );
    assert_eq!(events.pending(), 0);
}

// --- Test 6: two_adjacent_positions_cascade_within_the_same_block_event_pass ------------

/// Stands in for one piston's own commit-then-notify-the-neighbor-piston cascade (Context:
/// "one piston's state change causing a neighbor piston's checkIfExtend to queue its own
/// event") without needing `redstone::piston::PistonBehavior`'s own real machinery -- a real
/// two-`PistonBehavior` setup can never actually exercise a same-*block-event-pass* cascade at
/// M3's own scope, because `PistonBehavior::on_block_event` only resolves and *schedules* a
/// commit (`COMMIT_DELAY_TICKS` ticks later); the write that would fan out to a neighbor always
/// happens later, from `on_scheduled_tick`, never synchronously from inside `on_block_event`
/// itself. This minimal equivalent performs that real write (`ctx.set_block`) directly from
/// `on_block_event`, which is exactly what MECH-D9's own re-entrancy guarantee has to hold for
/// regardless of which concrete behavior triggers it.
struct WriteOwnStateOnEventBehavior {
    log: Arc<Mutex<Vec<(&'static str, u8)>>>,
    written_state: BlockStateId,
}

impl BlockBehavior for WriteOwnStateOnEventBehavior {
    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {
        self.log.lock().unwrap().push(("piston1", event.event_id));
        // The real write a piston's own commit performs -- fans out neighbor-changed/
        // shape-update to every side, synchronously, from inside this very on_block_event call
        // (a no-op write still fans out, `UpdateContext::set_block`'s own documented contract).
        ctx.set_block(pos, self.written_state);
    }
}

/// The adjacent "piston2": reacts to the neighbor-changed fan-out `WriteOwnStateOnEventBehavior`
/// above just produced by queuing its own event -- exactly `PistonBehavior::on_neighbor_changed`'s
/// own real reaction (`ctx.emit_block_event`) when its cached activation target flips.
struct EmitOnNeighborChangedBehavior {
    log: Arc<Mutex<Vec<(&'static str, u8)>>>,
    emitted_event_id: u8,
}

impl BlockBehavior for EmitOnNeighborChangedBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        let Some(state) = ctx.get_block(pos) else {
            return;
        };
        ctx.emit_block_event(pos, self.emitted_event_id, 0, state);
    }
    fn on_block_event(&self, _ctx: &mut UpdateContext, _pos: BlockPos, event: &BlockEvent) {
        self.log.lock().unwrap().push(("piston2", event.event_id));
    }
}

#[test]
fn two_adjacent_positions_cascade_within_the_same_block_event_pass() {
    let piston1_pos = BlockPos::new(0, 0, 0);
    let piston2_pos = Direction::East.apply(piston1_pos); // directly adjacent

    let mut world = FakeWorld::new();
    world.set_block(piston1_pos, BlockStateId(1));
    world.set_block(piston2_pos, BlockStateId(50));

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(2),
        Arc::new(WriteOwnStateOnEventBehavior {
            log: Arc::clone(&log),
            written_state: BlockStateId(1),
        }),
    );
    registry.register_range(
        BlockStateId(50),
        BlockStateId(51),
        Arc::new(EmitOnNeighborChangedBehavior {
            log: Arc::clone(&log),
            emitted_event_id: 9,
        }),
    );

    let (mut engine, mut scheduled, mut events, _halo, mut outbound, ownership) = harness();
    events.emit(BlockEvent {
        pos: piston1_pos,
        event_id: 7,
        event_param: 0,
        block_state: BlockStateId(1),
    });

    run_block_event_subphase(
        &mut world,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        0,
    );

    assert_eq!(
        *log.lock().unwrap(),
        vec![("piston1", 7), ("piston2", 9)],
        "piston2's own event, queued as a synchronous side effect of handling piston1's event, \
         must fire within this SAME run_block_event_subphase call -- the same tick's own pass, \
         not a second call"
    );
    assert_eq!(events.pending(), 0);
}
