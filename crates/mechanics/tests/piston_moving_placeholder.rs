//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(single canonical value/facing asserted, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=yes nondefault-state=yes
//! M3 field-report test-authoring (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84):
//! vanilla does not wait two ticks to change the world when an accepted block event is executed
//! (`PistonBaseBlock.triggerEvent` -> `moveBlocks`) — every pushed block's destination cell and
//! the head cell (extend), or the base cell and a sticky pull's destination cell (retract),
//! become `minecraft:moving_piston[facing,type]` immediately, synchronously with the triggering
//! block event, and every reactive cascade that placeholder appearance triggers (e.g. a wire
//! resting on a pushed block losing its own support) settles immediately too — real, permanent,
//! observable effects. The placeholder's own block state, at its own position, is a different
//! matter: a real oracle capture (`xtask parity-check redstone`, every multi-tick piston fixture)
//! settled empirically that it is NEVER independently visible there at all, not even for one
//! tick — a client's (and this engine's own directly-queried `BlockWorldAccess`) last-known value
//! for that position stays exactly what it held before the push/pull, all the way through the
//! whole 2-tick window, then jumps straight to the real final content once the deferred commit
//! (`commit_extend`/`commit_retract`, unchanged by this changeset) lands. `PistonBehavior::
//! on_after_drain` (`crates/mechanics/src/behavior.rs`'s own doc comment has the "why this hook
//! exists" citation) is what restores each placeholder's own pre-write content immediately after
//! every reactive cascade it triggered has already settled, all still within the very same
//! synchronous block-event dispatch. `MovingPistonBlock.getShape` is unconditionally `Shapes.
//! empty()` (MECH-D84) — a moving_piston cell provides no support on any face and is never a
//! redstone conductor, verified directly against the decompiled reference (`net.minecraft.world.
//! level.block.piston.MovingPistonBlock`/`PistonBaseBlock`/`PistonStructureResolver`,
//! ASSET-D18(f)).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::{PushClass, classify, resolve_extend, resolve_retract};
use rc_mechanics::redstone::{
    PistonBehavior, RedstoneSignalSource, SignalSourceRegistry, WireBehavior,
};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo,
    LightDirtyQueue, NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, UpdateContext,
    stage4,
};
use rc_messaging::{Address, RegionMessage};
use rc_physics::{Face, SupportKind, tier1_shape_table};
use rc_registries::block_state_properties::{range_of, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state;

use support::{FakeWorld, TestSignalSource};

/// The exact same name-based lookup `piston.rs`'s own private `moving_piston_id` uses
/// (`state_id(block_id::MOVING_PISTON, &[("facing", ..), ("type", ..)])`) — restated here since
/// that helper is crate-private; kept honest by construction (both derive from the identical
/// generated registry call, never a hand-copied literal).
fn moving_piston_id(facing: Direction, sticky: bool) -> BlockStateId {
    let facing_str = match facing {
        Direction::North => "north",
        Direction::East => "east",
        Direction::South => "south",
        Direction::West => "west",
        Direction::Up => "up",
        Direction::Down => "down",
    };
    BlockStateId(
        state_id(
            block_id::MOVING_PISTON,
            &[
                ("facing", facing_str),
                ("type", if sticky { "sticky" } else { "normal" }),
            ],
        )
        .expect("every (facing, sticky) pair is a legal moving_piston state")
        .0,
    )
}

const PISTON_HEAD_EAST: BlockStateId = BlockStateId(2275); // type=normal, facing=east, short=false
const STICKY_PISTON_HEAD_EAST: BlockStateId = BlockStateId(2276); // type=sticky, facing=east, short=false
const AIR: BlockStateId = BlockStateId(0);

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    halo: BorderHalo,
    outbound: Vec<(Address, RegionMessage)>,
    changed: Vec<(BlockPos, BlockStateId)>,
    light_dirty: LightDirtyQueue,
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
            light_dirty: LightDirtyQueue::new(),
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
            light_dirty: &mut self.light_dirty,
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
            &mut self.light_dirty,
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
            &mut self.light_dirty,
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

/// The real, generated `[2257, 2269)` `minecraft:piston` (facing=east range spans the whole
/// block's 12 states) — matches `piston_tick_tables.rs`'s own identical registration. Also
/// registers the full real `moving_piston` range at the same `piston` instance, mirroring
/// `register_piston`'s own production registration (`piston.rs`'s own `PistonStateIds` doc
/// comment has the full "why" citation) — a retract's own deferred commit fires at the piston's
/// own base position, which now holds the `moving_piston` placeholder for the whole 2-tick
/// window, and `dispatch_scheduled_tick` resolves the behavior to call from that position's own
/// LIVE block state.
fn register_piston_range(behaviors: &mut BlockBehaviorRegistry, piston: &Arc<PistonBehavior>) {
    behaviors.register_range(
        BlockStateId(2257),
        BlockStateId(2269),
        Arc::clone(piston) as Arc<dyn BlockBehavior>,
    );
    let moving_piston_range = range_of(block_id::MOVING_PISTON);
    behaviors.register_range(
        BlockStateId(moving_piston_range.first.0),
        BlockStateId(moving_piston_range.last.0 + 1),
        Arc::clone(piston) as Arc<dyn BlockBehavior>,
    );
}

#[test]
fn extend_leaves_destinations_at_their_own_true_content_after_the_accept_tick_settles() {
    const PISTON_ID: BlockStateId = BlockStateId(100);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let near_pos = facing.apply(piston_pos); // STONE, directly in front.
    let far_pos = facing.apply(near_pos); // empty -- the head's own landing cell.

    const STONE: BlockStateId = BlockStateId(1000);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false, false);

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    register_piston_range(&mut behaviors, &piston);

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);
    h.world.set_block(near_pos, STONE);

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);

    // Real-oracle-verified (`xtask parity-check redstone`): the `moving_piston` placeholder
    // `write_extend_placeholders` writes at accept time is never independently visible at
    // either destination at all, not even for the remainder of this same tick --
    // `PistonBehavior::on_after_drain` restores each one to its own true pre-write content
    // (`near_pos`'s own STONE, `far_pos`'s own untouched air) immediately after every reactive
    // cascade the placeholder's own appearance triggered has already settled, all within this
    // same `run_block_events(0)` call. `wire_on_the_pushed_block_pops_at_the_accept_tick` below
    // is this same mechanism's own positive proof: a wire directly above `near_pos` DOES lose
    // support and pop, immediately, permanently -- the placeholder's own real, transient
    // existence is proven by that side effect, never by directly observing `near_pos` itself.
    assert_eq!(
        h.world.get_block(near_pos),
        Some(STONE),
        "the pushed STONE's own old position already reads back its own true content once the \
         accept tick's own dispatch has settled"
    );
    assert_eq!(
        h.world.get_block(far_pos),
        Some(AIR),
        "the head's own landing cell is restored to its own true (untouched, i.e. air) content \
         too"
    );
    assert!(
        piston.has_pending_move(piston_pos),
        "the commit is scheduled, not yet fired"
    );

    h.run_scheduled(2);

    // Two ticks later: the real, final settled content lands -- exactly what `commit_extend`
    // already wrote before this changeset.
    assert_eq!(
        h.world.get_block(near_pos),
        Some(PISTON_HEAD_EAST),
        "the piston's own head settles where STONE used to be"
    );
    assert_eq!(
        h.world.get_block(far_pos),
        Some(STONE),
        "STONE shifts forward one cell, carrying its own real content"
    );
}

#[test]
fn bare_retract_leaves_the_base_at_its_own_true_content_after_the_accept_tick_settles() {
    const PISTON_ID: BlockStateId = BlockStateId(100);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let front_pos = facing.apply(piston_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false, false);

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    register_piston_range(&mut behaviors, &piston);

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    // Extend fully first.
    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    h.run_scheduled(2);
    assert!(piston.is_extended(piston_pos));
    assert_eq!(h.world.get_block(front_pos), Some(PISTON_HEAD_EAST));

    // Retract.
    source.set_power(0);
    {
        let mut ctx = h.ctx_at(10);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(10);

    // Vanilla's own `triggerEvent` (b0 == TRIGGER_CONTRACT/TRIGGER_DROP) writes the
    // `moving_piston` placeholder directly AT THE BASE CELL, immediately, synchronously with
    // this same block event -- but (real-oracle-verified, `xtask parity-check redstone`) it is
    // never independently visible there at all: `PistonBehavior::on_after_drain` restores the
    // base's own true pre-retract content (still the real, currently-extended id) immediately
    // after every reactive cascade the placeholder's own appearance triggered has already
    // settled, all within this same `run_block_events(10)` call. The head cell's own content,
    // unaffected by this changeset, still clears to air immediately (and PERMANENTLY -- that
    // write is never queued for a revert) for a bare retraction (`apply_retract_content`'s own
    // doc comment).
    // source: blocks.json
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BlockStateId(2258)), // extended=true, facing=east
        "the base cell already reads back its own true, still-extended content once the accept \
         tick's own dispatch has settled"
    );
    assert_eq!(
        h.world.get_block(front_pos),
        Some(AIR),
        "a bare retraction's head content still clears to air immediately, unchanged"
    );
    assert!(
        piston.is_extended(piston_pos),
        "the internal flag stays true until the deferred commit actually fires"
    );

    h.run_scheduled(12);
    assert!(!piston.is_extended(piston_pos));
    // source: blocks.json
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BlockStateId(2264)), // extended=false, facing=east
        "the real retracted base id settles at the deferred commit"
    );
}

#[test]
fn sticky_pull_leaves_the_old_head_at_its_own_true_content_after_the_accept_tick_settles() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const STONE: BlockStateId = BlockStateId(1000);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let head_pos = facing.apply(piston_pos);
    let candidate_pos = facing.apply(head_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, true, false);

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        BlockStateId(2236),
        BlockStateId(2243),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    let moving_piston_range = range_of(block_id::MOVING_PISTON);
    behaviors.register_range(
        BlockStateId(moving_piston_range.first.0),
        BlockStateId(moving_piston_range.last.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);
    h.run_scheduled(2);
    assert_eq!(h.world.get_block(head_pos), Some(STICKY_PISTON_HEAD_EAST));

    // The door: a Stone directly in front of the settled sticky head.
    h.world.set_block(candidate_pos, STONE);

    source.set_power(0);
    {
        let mut ctx = h.ctx_at(10);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(10);

    // Base cell: reads back its own true (still sticky-extended) content once the accept tick's
    // own dispatch has settled -- the `moving_piston` placeholder it held transiently, mid-
    // dispatch, is never independently visible (real-oracle-verified, same mechanism as the
    // bare-retract case above).
    // source: blocks.json
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BlockStateId(2236)), // sticky_piston, extended=true, facing=east
        "the sticky base cell already reads back its own true content"
    );
    // The pulled block's own SOURCE clears to air immediately AND PERMANENTLY (unchanged --
    // that write is never queued for a revert).
    assert_eq!(
        h.world.get_block(candidate_pos),
        Some(AIR),
        "the pulled block's own source clears to air immediately, unchanged"
    );
    // The old head -- the pulled block's own DESTINATION -- also reads back its own true
    // pre-retract content (the settled sticky piston_head): the `moving_piston` placeholder it
    // held transiently, carrying the pulled block's own content for the deferred commit, is
    // never independently visible either.
    assert_eq!(
        h.world.get_block(head_pos),
        Some(STICKY_PISTON_HEAD_EAST),
        "the old head already reads back its own true, still-settled piston_head content"
    );

    h.run_scheduled(12);
    assert_eq!(
        h.world.get_block(head_pos),
        Some(STONE),
        "the pulled Stone's own real content lands at the deferred commit"
    );
    // source: blocks.json
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(BlockStateId(2242)), // sticky_piston, extended=false, facing=east
        "the sticky base's own real retracted id lands at the deferred commit too"
    );
}

/// The withheld corpus draft's own scenario (`docs/findings-for-planning.md`'s own "moving_piston
/// placeholder: now a measured parity divergence" entry; promoted as
/// `redstone/piston/piston_push_pops_wire_on_moved_block`): a wire resting on top of a pushed
/// block must pop the SAME tick the extend accepts, not two ticks later when the structural
/// commit used to be the only write this engine ever made there. This is the ONE direct,
/// positive proof (across this whole file) that the `moving_piston` placeholder genuinely,
/// transiently existed at all: `pushed_pos` itself already reads back its own true content
/// (STONE) by the time `run_block_events` returns (real-oracle-verified — every other test in
/// this file asserts the identical "no direct trace" outcome at the placeholder's own position),
/// yet the wire's own real, PERMANENT support-loss additionally proves the placeholder was
/// real and briefly, actually present in `BlockWorldAccess` while the shape-update cascade ran.
#[test]
fn wire_on_the_pushed_block_pops_at_the_accept_tick_nondefault_case() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const STONE: BlockStateId = BlockStateId(1000);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let pushed_pos = facing.apply(piston_pos);
    let wire_pos = Direction::Up.apply(pushed_pos);

    let (source, signals, source_id) = make_signal(0);
    let piston = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston.place(piston_pos, facing, false, false);

    let wire = Arc::new(WireBehavior::new());
    wire.bind_registry(Arc::clone(&signals));

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_ID,
        BlockStateId(PISTON_ID.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    register_piston_range(&mut behaviors, &piston);
    let wire_range = range_of(block_id::REDSTONE_WIRE);
    behaviors.register_range(
        BlockStateId(wire_range.first.0),
        BlockStateId(wire_range.last.0 + 1),
        Arc::clone(&wire) as Arc<dyn BlockBehavior>,
    );

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_pos, PISTON_ID);
    h.world
        .set_block(Direction::Down.apply(piston_pos), source_id);
    h.world.set_block(pushed_pos, STONE);
    h.world
        .set_block(wire_pos, BlockStateId(default_state::REDSTONE_WIRE.0));

    source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);

    assert_eq!(
        h.world.get_block(pushed_pos),
        Some(STONE),
        "the pushed block's own old cell already reads back its own true content once the \
         accept tick's own dispatch has settled -- the placeholder it held transiently is never \
         independently visible"
    );
    assert_eq!(
        h.world.get_block(wire_pos),
        Some(AIR),
        "the wire above it must already have popped this same tick, permanently -- MECH-D84's \
         own empty moving_piston shape provided no support at all while the placeholder was \
         briefly, actually present there during the shape-update cascade"
    );
}

#[test]
fn moving_piston_cell_is_neither_a_conductor_nor_a_support_surface() {
    let facing = Direction::East;
    for sticky in [false, true] {
        let id = moving_piston_id(facing, sticky).0;
        let table = tier1_shape_table();
        assert!(
            table.lookup(id).shape.is_empty(),
            "moving_piston (sticky={sticky}) must have an empty shape -- \
             MovingPistonBlock.getShape is unconditionally Shapes.empty()"
        );
        assert!(
            !table.is_face_sturdy(id, Face::Up, SupportKind::Full),
            "an empty shape can never be Full-sturdy on any face -- a moving_piston cell \
             provides no support"
        );
        // `is_conductor` (`signal.rs`) resolves conductor-ness from this exact same shape
        // table -- a non-full-cube shape (empty, in particular) is never a conductor.
        let boxes = table.lookup(id).shape.boxes().to_vec();
        let is_full_cube = boxes.len() == 1
            && boxes[0].min == rc_physics::Vec3::new(0.0, 0.0, 0.0)
            && boxes[0].max == rc_physics::Vec3::new(1.0, 1.0, 1.0);
        assert!(
            !is_full_cube,
            "an empty shape is never the full-cube conductor shape"
        );
    }
}

#[test]
fn a_second_piston_cannot_push_a_moving_piston_cell_composition_case() {
    let mut world = FakeWorld::new();
    let ownership = RegionOwnership::always_local(Address::Region(rc_messaging::RegionId(0)));

    let piston2_pos = BlockPos::new(0, 0, 0);
    let target = Direction::East.apply(piston2_pos);
    world.set_block(target, moving_piston_id(Direction::North, false));

    assert_eq!(
        classify(&world, target, true),
        PushClass::Immovable,
        "classify must treat a moving_piston cell exactly like a real extended piston base or \
         piston_head -- PistonBaseBlock.isPushable's own real reference has no exception that \
         would let a second piston push through an in-flight one"
    );

    let result = resolve_extend(&world, &ownership, piston2_pos, Direction::East);
    assert!(
        result.is_err(),
        "a piston facing straight into a moving_piston cell must refuse to extend at all"
    );

    // The retract-pull side of the same rule: a moving_piston cell is never a valid sticky-pull
    // candidate either.
    let mut world2 = FakeWorld::new();
    let piston3_pos = BlockPos::new(0, 0, 0);
    let old_head = Direction::East.apply(piston3_pos);
    let candidate = Direction::East.apply(old_head);
    world2.set_block(old_head, PISTON_HEAD_EAST);
    world2.set_block(candidate, moving_piston_id(Direction::North, false));
    let pull = resolve_retract(&world2, &ownership, piston3_pos, Direction::East, true);
    assert!(
        pull.pulled.is_none(),
        "a moving_piston cell must never be pulled either"
    );
}
