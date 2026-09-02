//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(single canonical value/facing asserted, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain, see piston_tick_tables.rs) nondefault-state=yes
//! M3 field-report test-authoring: `PistonBehavior::on_placed`
//! (`crates/mechanics/src/redstone/piston.rs`) — closes `docs/findings-for-planning.md`'s own "a
//! piston placed by an actual connected player is never wired into `PistonBehavior`'s own
//! internal per-position state at all" finding. Exercises `on_placed` directly, mirroring
//! `redstone_repeater.rs`'s/`redstone_comparator.rs`'s own established `on_placed`-focused
//! test-file convention — independent of the full real-client path
//! `crates/server/tests/play_redstone_field_report.rs`'s own new tests cover end-to-end.
//!
//! Unlike `piston_tick_tables.rs`'s own arbitrary placeholder dispatch ids (that file drives
//! `PistonBehavior` methods directly, never through a decode step), `on_placed`'s own
//! decode-from-raw-id logic requires the WORLD's own stored id to already be a real reachable
//! `piston`/`sticky_piston` state — every constant below is hand-derived from `piston.rs`'s own
//! `piston_state_id`/`piston_head_id`/`PISTON_BASE`/`STICKY_PISTON_BASE` doc comments and
//! cross-checked against `piston_tick_tables.rs`'s own identical `PISTON_HEAD_EAST`/`STICKY_
//! PISTON_HEAD_EAST` constants.

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{PistonBehavior, RedstoneSignalSource, SignalSourceRegistry};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo,
    NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, UpdateContext, stage4,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

// `minecraft:piston`, facing=east (`piston_state_id(sticky=false, extended, East)`,
// `PISTON_BASE == 2257`, `piston_facing_index(East) == 1`).
const PISTON_EAST_RETRACTED: BlockStateId = BlockStateId(2264); // extended=false
const PISTON_EAST_EXTENDED: BlockStateId = BlockStateId(2258); // extended=true
// `minecraft:piston_head`, type=normal, facing=east, short=false (`piston_head_id(East, false)`)
// -- identical literal to `piston_tick_tables.rs`'s own `PISTON_HEAD_EAST`.
const PISTON_HEAD_EAST: BlockStateId = BlockStateId(2275);

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    halo: BorderHalo,
    outbound: Vec<(Address, RegionMessage)>,
    changed: Vec<(BlockPos, BlockStateId)>,
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
            changed: Vec::new(),
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
            changed: &mut self.changed,
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
            &mut self.changed,
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
            &mut self.changed,
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

/// Registers `piston` at both real `piston`/`sticky_piston` ranges directly (no placeholder
/// dispatch id needed — `on_placed`'s own decode step always reads the world's real stored id).
fn piston_registry(piston: &Arc<PistonBehavior>) -> BlockBehaviorRegistry {
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        BlockStateId(2257),
        BlockStateId(2269),
        Arc::clone(piston) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        BlockStateId(2235),
        BlockStateId(2247),
        Arc::clone(piston) as Arc<dyn BlockBehavior>,
    );
    behaviors
}

#[test]
fn on_placed_seeds_a_fresh_piston_with_no_signal_and_fires_nothing() {
    let piston_pos = BlockPos::new(0, 0, 0);

    let (_source, signals, _source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    let behaviors = piston_registry(&piston);
    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_EAST_RETRACTED);

    {
        let mut ctx = h.ctx_at(0);
        piston.on_placed(&mut ctx, piston_pos);
    }

    assert_eq!(piston.facing(piston_pos), Direction::East);
    assert!(!piston.is_sticky(piston_pos));
    assert!(!piston.is_extended(piston_pos));
    assert!(
        !piston.should_be_extended(piston_pos),
        "no signal at placement -- checkIfExtend must find nothing to do"
    );
    h.run_block_events(0);
    assert!(
        !h.scheduled.is_block_tick_pending(piston_pos),
        "no signal at placement -- nothing queued"
    );
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(PISTON_EAST_RETRACTED),
        "no spurious write"
    );
}

/// The mechanics-level half of the M3 field-report task: `on_placed` seeding + immediate-extend
/// when placed next to an existing signal — vanilla's own `PistonBaseBlock.setPlacedBy` ->
/// `checkIfExtend`, exercised directly against `PistonBehavior`.
#[test]
fn on_placed_extends_immediately_when_placed_beside_an_already_active_signal_nondefault_case() {
    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let head_pos = facing.apply(piston_pos);

    let (_source, signals, source_id) = make_signal(15);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    let behaviors = piston_registry(&piston);
    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_EAST_RETRACTED);
    // The signal source sits at the piston's own DOWN neighbor -- always checked
    // unconditionally by `piston_neighbor_signal`, regardless of push direction.
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    {
        let mut ctx = h.ctx_at(0);
        piston.on_placed(&mut ctx, piston_pos);
    }

    assert!(
        piston.should_be_extended(piston_pos),
        "on_placed's own immediate checkIfExtend must see the already-active neighbor signal"
    );
    h.run_block_events(0);
    assert!(
        h.scheduled.is_block_tick_pending(piston_pos),
        "a real extend must have been queued, mirroring vanilla's own setPlacedBy -> \
         checkIfExtend"
    );
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(PISTON_EAST_EXTENDED),
        "the base's own EXTENDED id flips immediately, at block-event time -- same timing as \
         every other extend trigger"
    );

    h.run_scheduled(2);
    assert!(piston.is_extended(piston_pos));
    assert_eq!(h.world.get_block(head_pos), Some(PISTON_HEAD_EAST));
}

/// Mirrors `crates/testing/gametest/src/replay.rs`'s own `tier1_registry` pre-scan ->
/// `place_and_settle` ordering exactly: `place` seeds this position FIRST (as the pre-scan does,
/// strictly before `on_placed` is ever called for it), with the SAME properties the raw id below
/// decodes to — `on_placed`'s own idempotency gate must recognize this as a pure re-affirmation
/// and skip the immediate `checkIfExtend` entirely, even though an active signal sits right there
/// (the exact "already-extended fixture, triggering signal placed later in the same batch"
/// corpus shape this gate exists to keep byte-identical -- `parity-check redstone` is the
/// arbiter).
#[test]
fn on_placed_reseeding_an_already_matching_position_is_idempotent_and_fires_nothing() {
    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;

    let (_source, signals, source_id) = make_signal(15);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    let behaviors = piston_registry(&piston);
    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_EAST_RETRACTED);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    // The pre-scan's own equivalent: seed BEFORE `on_placed` ever runs, with the identical
    // decoded properties (facing=east, sticky=false, extended=false) the raw id above encodes.
    piston.place(piston_pos, facing, false, false);

    {
        let mut ctx = h.ctx_at(0);
        piston.on_placed(&mut ctx, piston_pos);
    }

    assert!(
        !piston.should_be_extended(piston_pos),
        "a pure re-seed of already-matching state must never run the immediate check -- the \
         active signal beside it must not be newly discovered here"
    );
    h.run_block_events(0);
    assert!(
        !h.scheduled.is_block_tick_pending(piston_pos),
        "no event may be queued by a re-seed of already-known, unchanged placement state"
    );
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(PISTON_EAST_RETRACTED),
        "no spurious write either"
    );
}

/// A genuinely different placement at an already-tracked position (e.g. broken and replaced with
/// a new facing) must still run the immediate check -- the idempotency gate above compares
/// actual decoded VALUES, not merely "was this position ever seen before."
#[test]
fn on_placed_still_runs_the_immediate_check_when_re_placed_with_different_properties() {
    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let head_pos = facing.apply(piston_pos);

    let (_source, signals, source_id) = make_signal(15);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    let behaviors = piston_registry(&piston);
    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_EAST_RETRACTED);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    // A stale entry from some earlier, unrelated placement at this same position -- a different
    // facing/sticky than the raw id in the world now encodes.
    piston.place(piston_pos, Direction::North, true, false);

    {
        let mut ctx = h.ctx_at(0);
        piston.on_placed(&mut ctx, piston_pos);
    }

    assert_eq!(
        piston.facing(piston_pos),
        facing,
        "reseeded to the NEW placement's own facing"
    );
    assert!(
        !piston.is_sticky(piston_pos),
        "reseeded to the NEW placement's own sticky flag too"
    );
    assert!(
        piston.should_be_extended(piston_pos),
        "a genuinely different placement must still run the immediate checkIfExtend"
    );
    h.run_block_events(0);
    h.run_scheduled(2);
    assert!(piston.is_extended(piston_pos));
    assert_eq!(h.world.get_block(head_pos), Some(PISTON_HEAD_EAST));
}
