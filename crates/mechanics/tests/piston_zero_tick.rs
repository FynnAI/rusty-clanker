//! M3-B05 — the "zero-tick stance" acceptance test (Context §F): MECH-D7/D11's own binding
//! bug-for-bug update-order commitment, applied to a piston reversed twice within one tick.

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::{TRIGGER_CONTRACT, TRIGGER_EXTEND};
use rc_mechanics::redstone::{PistonBehavior, RedstoneSignalSource, SignalSourceRegistry};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEvent, BlockEventQueue, BlockWorldAccess,
    BorderHalo, NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, UpdateContext, stage4,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const PISTON_ID: BlockStateId = BlockStateId(100);
const SOURCE_ID: BlockStateId = BlockStateId(1);
const PISTON_HEAD_EAST: BlockStateId = BlockStateId(900_002);
const AIR: BlockStateId = BlockStateId(0);

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

type SetupResult = (
    Harness,
    Arc<PistonBehavior>,
    Arc<TestSignalSource>,
    Arc<Mutex<Vec<u8>>>,
    BlockPos,
    BlockPos,
);

fn setup() -> SetupResult {
    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let front_pos = facing.apply(piston_pos);

    let source = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        Arc::clone(&source) as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);

    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false);

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
    // Own-state writeback (M3 field-report fix): once a commit writes this regular, facing=East
    // piston's own real id (2258 extended / 2264 retracted, `piston_state_id`'s own doc comment
    // in piston.rs) back into the world, a *later* block-event's own `block_state` (captured
    // live at emit time, in `on_neighbor_changed`) reflects that real id instead of the
    // placeholder-only `PISTON_ID` above -- `run_block_event_subphase`'s own dispatch needs it
    // registered too, or a second commit's own TRIGGER_CONTRACT/TRIGGER_EXTEND event silently
    // falls through to `NoOpBehavior`.
    behaviors.register_range(BlockStateId(2258), BlockStateId(2265), logging_piston);

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), SOURCE_ID);

    (h, piston, source, event_ids, piston_pos, front_pos)
}

#[test]
fn pulse_shorter_than_commit_window_is_absorbed() {
    let (mut h, piston, source, event_ids, piston_pos, front_pos) = setup();

    // Within one tick: the signal reads true, then false, both before the block-event
    // sub-phase ever runs (Context §F -- `NeighborUpdateEngine::drain` can revisit the same
    // position twice within one tick's own settling pass).
    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    assert!(piston.should_be_extended(piston_pos));

    source.set_power(0);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    assert!(!piston.should_be_extended(piston_pos));

    h.run_block_events(0);
    assert_eq!(
        *event_ids.lock().unwrap(),
        vec![TRIGGER_EXTEND, TRIGGER_CONTRACT],
        "both events queued and processed in emission order, same tick"
    );

    // Both events scheduled a commit at trigger_tick == 2 (both emitted at tick 0); the second
    // (retracting) plan overwrote the first (extending) one before either ever committed.
    h.run_scheduled(2);

    assert!(
        !piston.is_extended(piston_pos),
        "the piston never visibly reaches the extended state -- it settles back at its start"
    );
    // Only the surviving (retracting) plan's own commit ever fires -- it re-affirms `front_pos`
    // as AIR (a legitimate, real write/fan-out for the *surviving* commit, matching vanilla's
    // own unconditional post-`setBlock` notify convention even when nothing observably changes,
    // `UpdateContext::set_block`'s own documented behavior). The superseded extension's own
    // distinct content (`piston_head`) is what must never appear -- it never does.
    assert_ne!(
        h.world.get_block(front_pos),
        Some(PISTON_HEAD_EAST),
        "the superseded extension's own piston_head write must never land"
    );
    assert!(!piston.has_pending_move(piston_pos));
}

#[test]
fn two_events_in_different_ticks_do_not_supersede() {
    let (mut h, piston, source, event_ids, piston_pos, front_pos) = setup();

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    h.run_scheduled(2);
    assert!(
        piston.is_extended(piston_pos),
        "the first commit fired in full, independently"
    );
    assert_eq!(h.world.get_block(front_pos), Some(PISTON_HEAD_EAST));

    event_ids.lock().unwrap().clear();

    source.set_power(0);
    {
        let mut ctx = h.ctx_at(5);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(5);
    assert_eq!(*event_ids.lock().unwrap(), vec![TRIGGER_CONTRACT]);
    h.run_scheduled(7);

    assert!(
        !piston.is_extended(piston_pos),
        "the second commit also fired in full, independently"
    );
    assert_eq!(h.world.get_block(front_pos), Some(AIR));
}

/// Own-state writeback (M3 field-report fix): the piston base's own `EXTENDED` bit is
/// expressed in its own stored `BlockStateId`, not only in `PistonBehavior::is_extended`'s
/// internal side-table (blocks.json's own `minecraft:piston` entry, protocol 776:
/// `facing=east,extended=true` = state 2258, `...extended=false` = state 2264, both cited
/// directly off `datagen-output/26.2/generated/reports/blocks.json`, TEST-D56).
#[test]
fn piston_own_state_writeback_reflects_extended() {
    let (mut h, piston, source, _event_ids, piston_pos, _front_pos) = setup();

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    h.run_scheduled(2);
    assert!(piston.is_extended(piston_pos));
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BlockStateId(2258)),
        "piston base's own stored BlockStateId must flip to the real extended=true id"
    );

    source.set_power(0);
    {
        let mut ctx = h.ctx_at(5);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(5);
    h.run_scheduled(7);
    assert!(!piston.is_extended(piston_pos));
    assert_eq!(h.world.get_block(piston_pos), Some(BlockStateId(2264)));
}
