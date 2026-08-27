//! M3-B05 — hand-derived tick tables for canonical contraptions (Acceptance tests' own
//! framing): simple extension, two independently-QC-activated pistons, sticky retraction with
//! and without a block to pull, and Context §G's two re-validation cases.

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::{TRIGGER_CONTRACT, TRIGGER_DROP};
use rc_mechanics::redstone::{PistonBehavior, RedstoneSignalSource, SignalSourceRegistry};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEvent, BlockEventQueue, BlockWorldAccess,
    BorderHalo, NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, UpdateContext, stage4,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FanoutKind {
    NeighborChanged,
    ShapeUpdate,
}

/// Records every `on_neighbor_changed`/`on_shape_update` call it receives (Acceptance tests'
/// own "instrumented neighbor" framing, mirroring `behavior_registry.rs`'s established
/// `LoggingBehavior` pattern).
#[derive(Default)]
struct LoggingBehavior {
    log: Arc<Mutex<Vec<(BlockPos, Direction, FanoutKind)>>>,
}

impl BlockBehavior for LoggingBehavior {
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

/// Wraps a `PistonBehavior`, logging every `on_block_event`'s own `event_id` before delegating
/// -- this file's own instrumentation for distinguishing `TRIGGER_CONTRACT`/`TRIGGER_DROP`
/// (Context §E), since `BlockEventQueue` exposes no other way to observe a queued event's
/// content before `run_block_event_subphase` consumes it.
struct EventLoggingWrapper {
    inner: Arc<PistonBehavior>,
    event_ids: Arc<Mutex<Vec<u8>>>,
}

impl BlockBehavior for EventLoggingWrapper {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        self.inner.on_neighbor_changed(ctx, pos, from);
    }
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        self.inner.on_shape_update(ctx, pos, from, neighbor_state)
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        self.inner.on_scheduled_tick(ctx, pos);
    }
    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {
        self.event_ids.lock().unwrap().push(event.event_id);
        self.inner.on_block_event(ctx, pos, event);
    }
}

const PISTON_HEAD_EAST: BlockStateId = BlockStateId(900_002);

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    halo: BorderHalo,
    outbound: Vec<(Address, RegionMessage)>,
    ownership: RegionOwnership,
    behaviors: BlockBehaviorRegistry,
}

impl Harness {
    fn new(behaviors: BlockBehaviorRegistry) -> Self {
        let world = FakeWorld::new();
        let local = world.local;
        Self {
            world,
            engine: NeighborUpdateEngine::new(),
            scheduled: ScheduledTickQueue::new(),
            events: BlockEventQueue::new(),
            halo: BorderHalo::new(),
            outbound: Vec::new(),
            ownership: RegionOwnership::always_local(local),
            behaviors,
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

    fn run_block_events(&mut self, current_tick: u64) {
        stage4::run_block_event_subphase(
            &mut self.world,
            &self.ownership,
            &mut self.engine,
            &mut self.scheduled,
            &mut self.events,
            &self.behaviors,
            &mut self.outbound,
            current_tick,
        );
    }

    fn run_scheduled(&mut self, current_tick: u64) {
        stage4::run_scheduled_phase(
            &mut self.world,
            &[],
            &mut self.halo,
            &self.ownership,
            &mut self.engine,
            &mut self.scheduled,
            &mut self.events,
            &self.behaviors,
            &mut self.outbound,
            current_tick,
        );
    }
}

fn make_signal(
    power: u8,
) -> (
    Arc<TestSignalSource>,
    Arc<SignalSourceRegistry>,
    BlockStateId,
) {
    const SOURCE_ID: BlockStateId = BlockStateId(1);
    let source = Arc::new(TestSignalSource::fixed(power));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        Arc::clone(&source) as Arc<dyn RedstoneSignalSource>,
    );
    (source, Arc::new(signals), SOURCE_ID)
}

#[test]
fn simple_extension() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const WATCH_ID: BlockStateId = BlockStateId(200);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let front_pos = facing.apply(piston_pos);
    let watch_pos = facing.apply(front_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false);

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        WATCH_ID,
        BlockStateId(WATCH_ID.0 + 1),
        Arc::new(LoggingBehavior {
            log: Arc::clone(&log),
        }) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);
    h.world.set_block(watch_pos, WATCH_ID);

    // Tick 0: the source flips 0 -> 15, touching the piston's Down face.
    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    assert!(piston.should_be_extended(piston_pos));
    h.run_block_events(0);
    assert!(h.scheduled.is_block_tick_pending(piston_pos));
    assert!(!piston.is_extended(piston_pos));
    assert!(log.lock().unwrap().is_empty());

    // Tick 1: nothing due yet.
    h.run_scheduled(1);
    assert!(!piston.is_extended(piston_pos));
    assert!(log.lock().unwrap().is_empty());

    // Tick 2: the commit fires.
    h.run_scheduled(2);
    assert!(piston.is_extended(piston_pos));
    assert_eq!(h.world.get_block(front_pos), Some(PISTON_HEAD_EAST));

    let logged = log.lock().unwrap();
    assert_eq!(
        logged.len(),
        2,
        "exactly one on_neighbor_changed/on_shape_update pair at the instrumented neighbor"
    );
    assert!(logged.contains(&(watch_pos, Direction::West, FanoutKind::NeighborChanged)));
    assert!(logged.contains(&(watch_pos, Direction::West, FanoutKind::ShapeUpdate)));
}

#[test]
fn qc_double_piston() {
    const PISTON1_ID: BlockStateId = BlockStateId(100);
    const PISTON2_ID: BlockStateId = BlockStateId(101);

    let source_pos = BlockPos::new(0, 0, 0);
    let p1 = Direction::West.apply(source_pos);
    let p2 = Direction::North.apply(source_pos);

    let (_source, signals, source_id) = make_signal(15);

    let piston1 = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston1.place(p1, Direction::Up, false);
    let piston2 = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston2.place(p2, Direction::Up, false);

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON1_ID,
        BlockStateId(PISTON1_ID.0 + 1),
        Arc::clone(&piston1) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        PISTON2_ID,
        BlockStateId(PISTON2_ID.0 + 1),
        Arc::clone(&piston2) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(source_pos, source_id);
    h.world.set_block(p1, PISTON1_ID);
    h.world.set_block(p2, PISTON2_ID);

    {
        let mut ctx = h.ctx_at(0);
        piston1.on_neighbor_changed(&mut ctx, p1, Direction::East);
        piston2.on_neighbor_changed(&mut ctx, p2, Direction::South);
    }
    assert!(piston1.should_be_extended(p1));
    assert!(piston2.should_be_extended(p2));
    h.run_block_events(0);

    assert!(h.scheduled.is_block_tick_pending(p1));
    assert!(h.scheduled.is_block_tick_pending(p2));
    assert!(!piston1.is_extended(p1));
    assert!(!piston2.is_extended(p2));

    h.run_scheduled(2);
    assert!(
        piston1.is_extended(p1),
        "piston1 must have committed independently at tick 2"
    );
    assert!(
        piston2.is_extended(p2),
        "piston2 must have committed independently at tick 2"
    );
}

/// Extends `piston` fully (tick 0 trigger, tick 2 commit) via the same mechanism as
/// `simple_extension` -- shared setup for the sticky-retraction cases below, which all start
/// from an already-extended piston.
fn extend_fully(
    h: &mut Harness,
    piston: &Arc<PistonBehavior>,
    piston_pos: BlockPos,
    source: &TestSignalSource,
) {
    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    h.run_scheduled(2);
    assert!(piston.is_extended(piston_pos));
}

#[test]
fn piston_door_element() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const STONE: BlockStateId = BlockStateId(1000);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let head_pos = facing.apply(piston_pos);
    let door_pos = facing.apply(head_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, true);

    let event_ids = Arc::new(Mutex::new(Vec::new()));
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::new(EventLoggingWrapper {
            inner: Arc::clone(&piston),
            event_ids: Arc::clone(&event_ids),
        }) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    extend_fully(&mut h, &piston, piston_pos, &source);
    assert_eq!(h.world.get_block(head_pos), Some(PISTON_HEAD_EAST));

    // The door: a single stone block directly in front of the now-settled head.
    h.world.set_block(door_pos, STONE);
    event_ids.lock().unwrap().clear();

    // Tick 10 (arbitrary, well past the commit tick above): the source drops back to 0.
    source.set_power(0);
    {
        let mut ctx = h.ctx_at(10);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    assert!(!piston.should_be_extended(piston_pos));
    h.run_block_events(10);
    assert_eq!(*event_ids.lock().unwrap(), vec![TRIGGER_CONTRACT]);
    assert!(h.scheduled.is_block_tick_pending(piston_pos));

    h.run_scheduled(11);
    assert_eq!(
        h.world.get_block(door_pos),
        Some(STONE),
        "unchanged until commit"
    );

    h.run_scheduled(12);
    assert_eq!(h.world.get_block(door_pos), Some(BlockStateId(0)));
    assert_eq!(h.world.get_block(head_pos), Some(STONE));
    assert!(!piston.is_extended(piston_pos));
}

#[test]
fn sticky_retract_with_nothing_to_pull_fires_drop() {
    const PISTON_ID: BlockStateId = BlockStateId(100);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let head_pos = facing.apply(piston_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, true);

    let event_ids = Arc::new(Mutex::new(Vec::new()));
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::new(EventLoggingWrapper {
            inner: Arc::clone(&piston),
            event_ids: Arc::clone(&event_ids),
        }) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    extend_fully(&mut h, &piston, piston_pos, &source);
    // No door block set at all -- nothing to pull.
    event_ids.lock().unwrap().clear();

    source.set_power(0);
    {
        let mut ctx = h.ctx_at(10);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(10);
    assert_eq!(*event_ids.lock().unwrap(), vec![TRIGGER_DROP]);

    h.run_scheduled(12);
    assert!(!piston.is_extended(piston_pos));
    assert_eq!(h.world.get_block(head_pos), Some(BlockStateId(0)));
}

#[test]
fn commit_reads_live_state_and_skips_a_changed_position() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const STONE: BlockStateId = BlockStateId(1000);
    const INJECTED: BlockStateId = BlockStateId(2000);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let pushed_pos = facing.apply(piston_pos);
    let far_pos = facing.apply(pushed_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false);

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);
    h.world.set_block(pushed_pos, STONE);

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    assert!(h.scheduled.is_block_tick_pending(piston_pos));

    // Between trigger and commit: something else changes the to-be-pushed position's own state.
    h.world.set_block(pushed_pos, INJECTED);

    h.run_scheduled(2);
    assert!(
        piston.is_extended(piston_pos),
        "the base's own EXTENDED flip still commits normally"
    );
    assert_eq!(
        h.world.get_block(pushed_pos),
        Some(INJECTED),
        "the changed position's own write is skipped -- it retains the test-injected value"
    );
    assert_eq!(
        h.world.get_block(far_pos),
        None,
        "nothing was shifted forward from a distrusted source"
    );
}

#[test]
fn breaking_the_base_mid_flight_aborts_the_whole_commit() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const WATCH_ID: BlockStateId = BlockStateId(200);
    const BROKEN_ID: BlockStateId = BlockStateId(9_999_777);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false);

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        WATCH_ID,
        BlockStateId(WATCH_ID.0 + 1),
        Arc::new(LoggingBehavior {
            log: Arc::clone(&log),
        }) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);
    h.world
        .set_block(Direction::West.apply(piston_pos), WATCH_ID);

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    assert!(h.scheduled.is_block_tick_pending(piston_pos));

    // Simulate the base having been broken mid-flight (M3-B03's own mining path, not itself
    // invoked here -- only its net effect on world state, Acceptance tests' own framing).
    h.world.set_block(piston_pos, BROKEN_ID);

    // The stage4 dispatcher would itself skip a broken position (it resolves the *current*
    // block state to find a behavior, and `BROKEN_ID` is unregistered) -- this test calls
    // `on_scheduled_tick` directly, bypassing that dispatch, to exercise `PistonBehavior`'s own
    // re-validation independently of it.
    {
        let mut ctx = h.ctx_at(2);
        piston.on_scheduled_tick(&mut ctx, piston_pos);
    }

    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BROKEN_ID),
        "nothing written, not even the base"
    );
    assert_eq!(
        h.world.get_block(facing.apply(piston_pos)),
        None,
        "no piston_head ever appears"
    );
    assert!(!piston.has_pending_move(piston_pos));
    assert!(
        log.lock().unwrap().is_empty(),
        "zero fan-out for the abandoned move"
    );
}
