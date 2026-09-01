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

/// M3 field-report fix (Task 3): the real `minecraft:piston_head` ids for facing=East,
/// `short=false` (`piston.rs`'s own `piston_head_id` doc comment has the full arithmetic
/// citation) -- `type` distinguishes a plain piston's own head from a sticky one, so the two
/// need separate constants now that both are real, distinct ids (unlike the former single
/// placeholder both shared).
const PISTON_HEAD_EAST: BlockStateId = BlockStateId(2275); // type=normal
const STICKY_PISTON_HEAD_EAST: BlockStateId = BlockStateId(2276); // type=sticky

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
    // Own-state writeback (M3 field-report fix, Task 3): the base's own `EXTENDED` flip is now
    // written *immediately*, at block-event time (`write_base_extended`'s own doc comment) --
    // `piston_pos`'s own stored id moves to the real facing=East range (2257..=2268) well before
    // the later scheduled commit's own dispatch (`run_scheduled(2)` below) needs to resolve it
    // back to this same `piston` instance, mirroring `piston_door_element`'s own established
    // "own-state writeback moves the id outside the placeholder-only range" precedent.
    behaviors.register_range(
        BlockStateId(2257),
        BlockStateId(2269),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
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
    // M3 field-report fix (Task 3): the base's own `EXTENDED` id is already written at this
    // point (real vanilla timing, `write_base_extended`'s own doc comment) -- only the
    // structural move (piston_head/pushed content, `PistonBehavior::is_extended`'s own
    // internal-flag semantics, tracked separately from the world's stored id) still awaits the
    // 2-tick commit below. This immediate write's own fan-out reaches only `piston_pos`'s own 6
    // direct neighbors (none of which is `watch_pos`, two cells out) -- `log` stays empty here,
    // unchanged from before this fix.
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BlockStateId(2258)), // extended=true, facing=east
        "the base's own EXTENDED id flips immediately, at block-event time"
    );
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

/// M3 field-report fix (Task 4): `commit_extend`'s own push-chain loop used to read each
/// `to_push[i]`'s own content live, *after* an earlier iteration had already overwritten that
/// same position (index 0 writes `piston_head` to `to_push[0]`'s own position; index 1 then read
/// `to_push[0]` again for *its own* content, seeing the just-written `piston_head` instead of the
/// real block that used to be there) -- every pushed block beyond the first became a duplicate of
/// the position ahead of it instead of shifting forward one cell. Pushes two real, distinct
/// blocks (not two of the same id, so a duplication bug is directly observable) and asserts each
/// lands at its own correctly shifted-forward position, not overwritten with its neighbor's
/// content. Confirmed against a real oracle diff
/// (`redstone/pulse/zero_tick_pulse_dropper_piston`'s own pushed `redstone_block`, `docs/
/// findings-for-planning.md`).
#[test]
fn multi_block_push_shifts_each_block_forward_not_duplicated() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const STONE: BlockStateId = BlockStateId(1000);
    const DIRT: BlockStateId = BlockStateId(1001);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let near_pos = facing.apply(piston_pos); // STONE, directly in front -- becomes piston_head.
    let far_pos = facing.apply(near_pos); // DIRT, two cells out -- shifts to three cells out.
    let far_shifted_pos = facing.apply(far_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false);

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        BlockStateId(2257),
        BlockStateId(2269),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);
    h.world.set_block(near_pos, STONE);
    h.world.set_block(far_pos, DIRT);

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    h.run_scheduled(2);

    assert_eq!(
        h.world.get_block(near_pos),
        Some(PISTON_HEAD_EAST),
        "the piston's own head lands where STONE used to be"
    );
    assert_eq!(
        h.world.get_block(far_pos),
        Some(STONE),
        "STONE shifts forward one cell into DIRT's own old position -- not overwritten with \
         DIRT's own content, and not left as piston_head"
    );
    assert_eq!(
        h.world.get_block(far_shifted_pos),
        Some(DIRT),
        "DIRT shifts forward one cell past its own old position, carrying its own real content"
    );
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
    // M3 field-report fix (Task 3): `piston2` is deliberately `sticky = true` here (unlike
    // `piston1`), even though this test cares only about QC-activation independence, not
    // stickiness -- own-state writeback (`write_base_extended`'s own doc comment) now writes
    // each piston's own *real* facing=Up id immediately, and `piston`/`sticky_piston` share the
    // identical `facing=Up` id (2261 vs 2239) only when they differ in this one property, which
    // is exactly what keeps `piston1`'s and `piston2`'s own registered dispatch ranges below
    // disjoint (two genuinely separate `PistonBehavior` instances, standing in for two
    // independent in-region positions, cannot otherwise both claim the identical real id).
    let piston2 = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston2.place(p2, Direction::Up, true);

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
    behaviors.register_range(
        BlockStateId(2257),
        BlockStateId(2269),
        Arc::clone(&piston1) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        BlockStateId(2235),
        BlockStateId(2247),
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
    let logging_piston = Arc::new(EventLoggingWrapper {
        inner: Arc::clone(&piston),
        event_ids: Arc::clone(&event_ids),
    }) as Arc<dyn BlockBehavior>;
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&logging_piston),
    );
    // Own-state writeback (M3 field-report fix): once the commit below writes the sticky
    // piston's own real facing=East id (2236 extended / 2242 retracted, `piston_state_id`'s own
    // doc comment) back into the world, `piston_pos`'s own stored id moves outside the
    // placeholder-only `[PISTON_ID, PISTON_ID+1)` range above -- the retract commit's own
    // scheduled-tick dispatch (`run_scheduled`, which resolves the *live* world id, unlike the
    // block-event dispatch above which resolves the id captured at emit time) needs this too.
    behaviors.register_range(BlockStateId(2236), BlockStateId(2243), logging_piston);

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    extend_fully(&mut h, &piston, piston_pos, &source);
    assert_eq!(h.world.get_block(head_pos), Some(STICKY_PISTON_HEAD_EAST));

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

    // Retract content/base split (M3 field-report fix, verified against a now-deterministic
    // real-oracle capture -- `piston.rs`'s own `apply_retract_content` doc comment has the full
    // per-case citation): a sticky pull settles less immediately than a bare retract. Only the
    // pulled block's own *source* position (the door) clears right here, synchronously with
    // tick 10's own triggering block event; the old head itself is left untouched -- still
    // showing its own pre-retract content (the settled sticky piston_head) -- and only receives
    // the pulled block's own real content at the deferred commit, alongside the base's own
    // `EXTENDED` flip.
    assert_eq!(
        h.world.get_block(door_pos),
        Some(BlockStateId(0)),
        "the pulled block's own old position clears immediately, at block-event time"
    );
    assert_eq!(
        h.world.get_block(head_pos),
        Some(STICKY_PISTON_HEAD_EAST),
        "the old head is left untouched at trigger time -- still its own pre-retract content"
    );
    assert!(
        piston.is_extended(piston_pos),
        "only the base's own EXTENDED flip is deferred -- the internal flag stays true until \
         the commit tick"
    );

    h.run_scheduled(11);
    assert!(
        piston.is_extended(piston_pos),
        "still unchanged -- the deferred commit is not due until tick 12"
    );
    assert_eq!(
        h.world.get_block(head_pos),
        Some(STICKY_PISTON_HEAD_EAST),
        "still unchanged -- the deferred commit is not due until tick 12"
    );

    h.run_scheduled(12);
    assert_eq!(
        h.world.get_block(door_pos),
        Some(BlockStateId(0)),
        "unchanged -- the source already cleared back in tick 10"
    );
    assert_eq!(
        h.world.get_block(head_pos),
        Some(STONE),
        "the pulled block's own real content finally settles at the deferred commit"
    );
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
    let logging_piston = Arc::new(EventLoggingWrapper {
        inner: Arc::clone(&piston),
        event_ids: Arc::clone(&event_ids),
    }) as Arc<dyn BlockBehavior>;
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&logging_piston),
    );
    // Own-state writeback (M3 field-report fix) -- `piston_door_element`'s own identical note.
    behaviors.register_range(BlockStateId(2236), BlockStateId(2243), logging_piston);

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

    // Retract content/base split (M3 field-report fix): the content half clears the old head
    // immediately here too, even with nothing to pull -- `TRIGGER_DROP`'s own "arm retracts
    // without pulling" case is no exception. Only the base's own EXTENDED flip is deferred.
    assert_eq!(
        h.world.get_block(head_pos),
        Some(BlockStateId(0)),
        "content clears immediately, at block-event time, even with nothing to pull"
    );
    assert!(
        piston.is_extended(piston_pos),
        "only the base flip is deferred"
    );

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
    // Own-state writeback (M3 field-report fix, Task 3) -- `simple_extension`'s own identical
    // note: the base's own real id is written immediately, well before `h.run_scheduled(2)`
    // below needs to resolve it back to this same `piston` instance.
    behaviors.register_range(
        BlockStateId(2257),
        BlockStateId(2269),
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

    // M3 field-report fix (Task 3): the base's own `EXTENDED` id already flipped immediately,
    // at block-event time above (`write_base_extended`'s own doc comment) -- its own fan-out
    // already reached `WATCH_ID` (a *direct* West neighbor of `piston_pos` here, unlike `simple_
    // extension`'s two-cells-out `watch_pos`), before this test's own "external break"
    // simulation below ever runs.
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BlockStateId(2258)), // extended=true, facing=east
    );
    let logged_before_break = log.lock().unwrap().len();
    assert_eq!(
        logged_before_break, 2,
        "the immediate base flip already fanned out one on_neighbor_changed/on_shape_update pair"
    );

    // Simulate the base having been broken mid-flight (M3-B03's own mining path, not itself
    // invoked here -- only its net effect on world state, Acceptance tests' own framing) --
    // *after* the immediate base flip above, exactly the ordering Context §G case 1 exists to
    // detect (something else changed the base between the block event resolving and the
    // structural commit that was scheduled to follow it).
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
        "nothing further written, not even a re-affirmed base -- Context §G case 1's whole-abort"
    );
    assert_eq!(
        h.world.get_block(facing.apply(piston_pos)),
        None,
        "no piston_head ever appears"
    );
    assert!(!piston.has_pending_move(piston_pos));
    assert_eq!(
        log.lock().unwrap().len(),
        logged_before_break,
        "the abandoned *structural* move produces zero further fan-out beyond the immediate \
         base flip's own already-logged pair"
    );
}
