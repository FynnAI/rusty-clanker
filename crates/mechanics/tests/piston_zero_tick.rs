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
/// M3 field-report fix (Task 3): the real `minecraft:piston_head` id for `type=normal,
/// facing=east, short=false` (`piston.rs`'s own `piston_head_id` doc comment has the full
/// arithmetic citation).
const PISTON_HEAD_EAST: BlockStateId = BlockStateId(2275);
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
    piston.place(piston_pos, facing, false, false);

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

/// Section B4 (M3 field-report fix): verified correction of `blueprints/M3/M3-B05-piston.md`'s
/// own former "absorption" interpretation (that blueprint itself flagged this as mechanically-
/// derived and unverified). A second trigger arriving while the first's own `MovingPistonState`
/// is still in flight FORCE-FINALIZES it synchronously, right there in `on_block_event` -- its
/// real content actually lands -- rather than being silently superseded/dropped; the new trigger
/// then starts fresh from that now-settled state. Replaces this file's own former `pulse_
/// shorter_than_commit_window_is_absorbed`, which asserted only the two ticks' own *end* states
/// (both of which happen to come out identical either way for this particular zero-net pulse) --
/// too weak to distinguish "absorbed, never visibly extends" from "force-finalized, visibly
/// extends, then retracts," so it never actually pinned which of the two was real.
///
/// (Update, M3 field-report fix, retract content/base split: `on_block_event`'s retract arm now
/// applies a retraction's own content half immediately too -- `piston.rs`'s own
/// `apply_retract_content` doc comment has the full oracle citation -- so the force-finalized
/// extend's `piston_head` write and the immediately-following retract's own content clear both
/// land, synchronously, within this same tick-0 `on_block_event` call for `TRIGGER_CONTRACT`; by
/// the time `run_block_events(0)` returns, `front_pos` already reads `AIR` again, not
/// `piston_head`. The distinguishing power against "absorbed" survives intact even so: an
/// absorbed extend would leave `front_pos` at its own untouched pre-test `None` and
/// `is_extended` at its original `false`, while a force-finalized-then-retracted one leaves
/// `front_pos` at the real, momentarily-then-overwritten `AIR` and `is_extended` still `true`
/// (only the retract's own base flip stays deferred to tick 2).
#[test]
fn pulse_shorter_than_commit_window_force_finalizes_then_retracts() {
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

    // The force-finalization proof: TRIGGER_CONTRACT's own handling force-finalized the still-
    // in-flight TRIGGER_EXTEND commit *synchronously*, right here in tick 0 -- its real content
    // (piston_head at front_pos) actually landed, briefly, before this same on_block_event call's
    // own retract arm immediately cleared it again (retract content/base split) -- well before
    // either trigger's own COMMIT_DELAY_TICKS-later scheduled tick ever fires. `is_extended`
    // stays `true` here: the force-finalized extend's base flip already landed, and the
    // following retract's own base flip is the one thing still genuinely deferred.
    assert!(
        piston.is_extended(piston_pos),
        "the force-finalized extend's own base flip must actually land, not be silently dropped"
    );
    assert_eq!(
        h.world.get_block(front_pos),
        Some(AIR),
        "the retract's own content half is synchronous too, so it already cleared the \
         force-finalized extend's own piston_head write within this same tick-0 pass"
    );
    // The retract that force-finalization made way for is itself still in flight -- only its own
    // base flip remains, scheduled for tick 2, not yet committed.
    assert!(piston.has_pending_move(piston_pos));

    // Both TRIGGER_EXTEND's and TRIGGER_CONTRACT's own `schedule_block_tick` calls target
    // trigger_tick == 2 (both emitted at tick 0); the first fire commits the still-pending
    // retract's own base flip (`on_scheduled_tick`'s own "already consumed" no-op guard silently
    // absorbs whichever due entry, if any, arrives second for the same position).
    h.run_scheduled(2);

    assert!(
        !piston.is_extended(piston_pos),
        "the retract that followed the force-finalized extend settles its own base flip back to \
         retracted"
    );
    assert_eq!(
        h.world.get_block(front_pos),
        Some(AIR),
        "unchanged -- the retract's content already cleared this position back in tick 0"
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
    // Retract content/base split (M3 field-report fix): the content half clears `front_pos`
    // immediately here, at tick 5's own block-event time -- only the base's own `EXTENDED` flip
    // is genuinely deferred to tick 7 below.
    assert_eq!(
        h.world.get_block(front_pos),
        Some(AIR),
        "retract content settles immediately, at block-event time"
    );
    assert!(
        piston.is_extended(piston_pos),
        "only the base flip is deferred -- still true until the commit tick"
    );
    h.run_scheduled(7);

    assert!(
        !piston.is_extended(piston_pos),
        "the second commit's own base flip also fired in full, independently"
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
