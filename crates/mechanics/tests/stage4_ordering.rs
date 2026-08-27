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

// --- Test 5: block_event_emitted_during_subphase_is_deferred_to_next_call ---------------

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

#[test]
fn block_event_emitted_during_subphase_is_deferred_to_next_call() {
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
    assert_eq!(*log.lock().unwrap(), vec![1]);

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
    assert_eq!(*log.lock().unwrap(), vec![1, 2]);
}
