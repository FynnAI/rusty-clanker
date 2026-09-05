//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(single canonical value/facing asserted, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=yes nondefault-state=yes
//! M3 field-report test-authoring (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84): a
//! second wave, correcting this suite's own FIRST draft (the three `..._leaves_..._at_its_own_
//! true_content_after_the_accept_tick_settles` tests below, before this changeset, asserted a
//! server-side REVERT: every placeholder cell reading back its own pre-write content once the
//! accept tick's own dispatch settled). That revert was justified by a CLIENT-side oracle
//! capture — a client never receives the placeholder at all (see below) — but a client-side
//! capture cannot distinguish "the server never wrote it" from "the server wrote it and kept
//! it, just never told me", and the two are not the same thing. Verified directly against the
//! decompiled reference (`net.minecraft.world.level.block.piston.PistonBaseBlock.
//! triggerEvent`/`moveBlocks`, `MovingPistonBlock`, `net.minecraft.world.level.block.Block`'s own
//! `UPDATE_*` flag constants, `Blocks.MOVING_PISTON`'s own registration properties): vanilla's
//! own server-side `BlockState` genuinely, permanently holds `minecraft:moving_piston[facing,
//! type]` at every destination cell (extend) or the base cell and a sticky pull's own
//! destination cell (retract), for the WHOLE `COMMIT_DELAY_TICKS` window between accept and
//! commit — every one of vanilla's own placeholder writes uses a flag lacking `UPDATE_CLIENTS`
//! (flag 276 for the base cell; flag 324 for every `moveBlocks` push-loop/arm destination), so
//! the placeholder is absent from the WIRE, never from vanilla's own stored world. A vacated
//! source (a sticky pull's own source cell always; a bare retraction's old head, which has
//! nothing to pull into it) becomes real `air`, also immediately, via a flag that DOES carry
//! `UPDATE_CLIENTS` (82 or 3) — genuinely, permanently, and client-visibly. `crates/mechanics/
//! src/redstone/piston.rs`'s own top-of-file doc comment has the complete fix writeup (the
//! withdrawn `on_after_drain`/`pending_reverts` revert mechanism, the corrected fan-out mapping,
//! and the Context §G snapshot-timing fix this correction also required); `crates/server/src/
//! play/world.rs`'s own moving_piston-ranged broadcast filter is the ONLY place this crate ever
//! hides the placeholder from a client, never a server-side revert.
//!
//! `MovingPistonBlock.getShape` is unconditionally `Shapes.empty()` (MECH-D84) — a moving_piston
//! cell provides no support on any face and is never a redstone conductor, independently
//! corroborated by `Blocks.MOVING_PISTON`'s own registration properties (`.strength(-1.0F)`,
//! `.pushReaction(PushReaction.BLOCK)`, `.isRedstoneConductor(Blocks::never)`) — verified
//! directly against the decompiled reference (ASSET-D18(f)).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::{
    ExtendAbort, PushClass, classify, resolve_extend, resolve_retract,
};
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
/// that helper is crate-private (kept honest by construction: both derive from the identical
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
fn extend_leaves_destinations_holding_the_real_moving_piston_placeholder_for_the_whole_window() {
    const PISTON_ID: BlockStateId = BlockStateId(100);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let near_pos = facing.apply(piston_pos); // STONE, directly in front -- becomes the head cell.
    let far_pos = facing.apply(near_pos); // empty -- the chain's own genuine head_pos.

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

    let placeholder = moving_piston_id(facing, false);

    // Real-oracle-verified (`piston.rs`'s own top-of-file doc comment): the `moving_piston`
    // placeholder `write_extend_placeholders` writes at accept time is kept, PERMANENTLY, for
    // the whole window — no revert. `near_pos` (this chain's own sole `to_push` entry, doubling
    // as `armPos`) and `far_pos` (the genuine head_pos beyond the chain) both hold it.
    assert_eq!(
        h.world.get_block(near_pos),
        Some(placeholder),
        "near_pos genuinely, permanently holds the moving_piston placeholder once the accept \
         tick's own dispatch has settled"
    );
    assert_eq!(
        h.world.get_block(far_pos),
        Some(placeholder),
        "far_pos (the chain's own genuine head_pos) also genuinely, permanently holds the same \
         placeholder"
    );
    assert!(
        piston.has_pending_move(piston_pos),
        "the commit is scheduled, not yet fired"
    );

    // One tick later, still well before the commit: unchanged.
    h.run_scheduled(1);
    assert_eq!(h.world.get_block(near_pos), Some(placeholder));
    assert_eq!(h.world.get_block(far_pos), Some(placeholder));

    h.run_scheduled(2);

    // Two ticks later: the real, final settled content lands -- exactly what `commit_extend`
    // already wrote before this changeset, unaffected by this fix.
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
fn bare_retract_leaves_the_base_holding_the_real_moving_piston_placeholder_for_the_whole_window() {
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

    let placeholder = moving_piston_id(facing, false);

    // Vanilla's own `triggerEvent` (b0 == TRIGGER_CONTRACT/TRIGGER_DROP) writes the
    // `moving_piston` placeholder directly AT THE BASE CELL, immediately, synchronously with
    // this same block event -- and this server keeps it, PERMANENTLY, for the whole window.
    // The head cell's own content, unaffected by this changeset, still clears to air
    // immediately (and PERMANENTLY) for a bare retraction (`apply_retract_content`'s own doc
    // comment).
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(placeholder),
        "the base cell genuinely, permanently holds the moving_piston placeholder once the \
         accept tick's own dispatch has settled"
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

    // One tick later, still well before the commit: unchanged.
    h.run_scheduled(11);
    assert_eq!(h.world.get_block(piston_pos), Some(placeholder));
    assert_eq!(h.world.get_block(front_pos), Some(AIR));

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
fn sticky_pull_leaves_the_old_head_holding_the_real_moving_piston_placeholder_for_the_whole_window()
{
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

    let placeholder = moving_piston_id(facing, true);

    // Base cell: genuinely, permanently holds the placeholder -- the pre-retract sticky
    // `EXTENDED=true` id is gone from this cell for the whole window (real vanilla: the base
    // cell becomes `moving_piston` immediately, kept until the deferred commit).
    assert_eq!(
        h.world.get_block(piston_pos),
        Some(placeholder),
        "the sticky base cell genuinely, permanently holds the moving_piston placeholder"
    );
    // The pulled block's own SOURCE clears to air immediately AND PERMANENTLY (unchanged).
    assert_eq!(
        h.world.get_block(candidate_pos),
        Some(AIR),
        "the pulled block's own source clears to air immediately, unchanged"
    );
    // The old head -- the pulled block's own DESTINATION -- ALSO genuinely, permanently holds
    // the SAME placeholder (not its own pre-retract settled `piston_head` content, and not the
    // pulled STONE either, until the deferred commit).
    assert_eq!(
        h.world.get_block(head_pos),
        Some(placeholder),
        "the old head genuinely, permanently holds the moving_piston placeholder for the whole \
         window"
    );

    // One tick later, still well before the commit: unchanged.
    h.run_scheduled(11);
    assert_eq!(h.world.get_block(piston_pos), Some(placeholder));
    assert_eq!(h.world.get_block(head_pos), Some(placeholder));

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

/// The withheld corpus draft's own scenario (promoted as `redstone/piston/piston_push_pops_
/// wire_on_moved_block`): a wire resting on top of a pushed block must pop the SAME tick the
/// extend accepts, not two ticks later when the structural commit used to be the only write
/// this engine ever made there. `pushed_pos` itself genuinely, permanently holds the
/// `moving_piston` placeholder once the accept tick's own dispatch settles (M3 field-report
/// test-authoring, server-side persistence fix — this test's former "reads back its own true
/// content" expectation, from before this changeset's correction, is now WRONG: that was the
/// withdrawn revert draft's own behavior); the wire's own real, PERMANENT support-loss is this
/// same mechanism's own positive proof that the placeholder genuinely, actually sits in
/// `BlockWorldAccess` while the shape-update cascade runs.
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
        Some(moving_piston_id(facing, false)),
        "the pushed block's own old cell genuinely, permanently holds the moving_piston \
         placeholder once the accept tick's own dispatch has settled"
    );
    assert_eq!(
        h.world.get_block(wire_pos),
        Some(AIR),
        "the wire above it must already have popped this same tick, permanently -- MECH-D84's \
         own empty moving_piston shape provided no support at all while the placeholder was \
         actually present there during the shape-update cascade"
    );
}

/// MECH-D83 (M3 field-report test-authoring, PLAN-D10): a pushed redstone-signal source stops
/// powering its own old neighbor the SAME tick the extend accepts (the source's own old cell
/// already holds the non-conductor, non-signal `moving_piston` placeholder by then, permanently,
/// via `write_extend_placeholders`'s own full fan-out reaching that neighbor's `on_neighbor_
/// changed`), and only starts powering its own NEW neighbor once the deferred commit actually
/// lands, two ticks later (`commit_extend`'s own unchanged, full fan-out) — the exact server-
/// side timing `docs/findings-for-planning.md`'s own withdrawn-revert entry names as the
/// dominant real-world consequence of reverting the placeholder away (a pushed redstone block
/// kept powering its own old neighbors for two extra ticks). Both wire positions share ONE
/// `WireBehavior` instance, registered normally (`WireBehavior`'s own per-position `power`/
/// connections tables are keyed by `BlockPos` internally, exactly like `PistonBehavior`'s own
/// `state`/`moving` maps — one instance already covers every wire position in a region), so
/// every recompute below is driven by REAL `run_block_events`/`run_scheduled` dispatch, never a
/// manual stand-in call. Stickiness is irrelevant to this specific fan-out timing (retract is a
/// completely separate code path); this test uses a plain piston for minimalism —
/// `redstone/piston/piston_pushes_redstone_block_wire_timing`'s own corpus fixture (a real,
/// connected-client capture) is this same property's end-to-end, sticky-piston proof.
#[test]
fn pushed_redstone_block_stops_powering_old_neighbor_and_powers_new_neighbor_only_at_commit() {
    const PISTON_ID: BlockStateId = BlockStateId(100);
    const QC_SOURCE_ID: BlockStateId = BlockStateId(1);
    const REDSTONE_BLOCK_ID: BlockStateId = BlockStateId(2);

    let piston_pos = BlockPos::new(0, 0, 0);
    let facing = Direction::East;
    let block_pos = facing.apply(piston_pos); // the pushed "redstone block" stand-in.
    let head_dest_pos = facing.apply(block_pos); // where it settles, two ticks later.
    let old_neighbor_wire_pos = Direction::South.apply(block_pos);
    let new_neighbor_wire_pos = Direction::South.apply(head_dest_pos);

    // One registry, two independent fixed-power sources: `qc_source` drives the piston's own
    // activation (mutable, starts at 0); `redstone_block_source` stands in for the pushed block
    // itself (this suite's own established `TestSignalSource` convention — a real
    // `redstone_block` behaves identically: unconditional power 15 toward every side, never
    // toggled by anything this test does).
    let qc_source = Arc::new(TestSignalSource::fixed(0));
    let redstone_block_source = Arc::new(TestSignalSource::fixed(15));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        QC_SOURCE_ID,
        BlockStateId(QC_SOURCE_ID.0 + 1),
        Arc::clone(&qc_source) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        REDSTONE_BLOCK_ID,
        BlockStateId(REDSTONE_BLOCK_ID.0 + 1),
        Arc::clone(&redstone_block_source) as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);

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
        .set_block(Direction::Down.apply(piston_pos), QC_SOURCE_ID);
    h.world.set_block(block_pos, REDSTONE_BLOCK_ID);
    h.world.set_block(
        old_neighbor_wire_pos,
        BlockStateId(default_state::REDSTONE_WIRE.0),
    );
    h.world.set_block(
        new_neighbor_wire_pos,
        BlockStateId(default_state::REDSTONE_WIRE.0),
    );

    // Establish each wire's own initial power (this suite's own established pattern,
    // `crates/mechanics/tests/redstone_wire.rs`'s own identical convention — a freshly-placed
    // wire computes nothing until its own first `on_neighbor_changed`).
    {
        let mut ctx = h.ctx_at(0);
        wire.on_neighbor_changed(&mut ctx, old_neighbor_wire_pos, Direction::North);
        wire.on_neighbor_changed(&mut ctx, new_neighbor_wire_pos, Direction::North);
    }
    assert_eq!(
        wire.power(old_neighbor_wire_pos),
        15,
        "the old neighbor starts out powered directly by the redstone-block stand-in"
    );
    assert_eq!(
        wire.power(new_neighbor_wire_pos),
        0,
        "the new neighbor starts out unpowered -- head_dest_pos is still air"
    );
    // Discard whatever further cascade these two direct setup calls queued (this suite's own
    // `redstone_wire.rs`'s own identical "direct on_neighbor_changed call, then drain" pattern) —
    // a clean slate before the piston's own real trigger below, so the later `run_block_events`
    // call settles ONLY the piston's own cascade, never a leftover from this setup step.
    h.engine.drain(&mut |_eng, _item| {});

    qc_source.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston.on_neighbor_changed(&mut ctx, piston_pos, Direction::Down);
    }
    h.run_block_events(0);

    // The piston's own fan-out from `block_pos` (a full fan-out: this chain's sole `to_push`
    // entry, doubling as `armPos`) reaches `old_neighbor_wire_pos`'s own `on_neighbor_changed`
    // automatically, via REAL dispatch (`register_piston_range`'s own registration drives it) --
    // no manual recompute needed.
    assert_eq!(
        wire.power(old_neighbor_wire_pos),
        0,
        "the old neighbor loses power the SAME tick the extend accepts -- block_pos already \
         holds the non-conductor, non-signal moving_piston placeholder, permanently"
    );
    assert_eq!(
        wire.power(new_neighbor_wire_pos),
        0,
        "the new neighbor stays unpowered at accept time too -- head_dest_pos holds the SAME \
         placeholder, not the real redstone-block content, until the deferred commit (its own \
         shape-update-only fan-out never reaches new_neighbor_wire_pos's own on_neighbor_changed \
         at all -- write_extend_placeholders's own doc comment has the full derivation)"
    );

    h.run_scheduled(2);
    assert_eq!(
        h.world.get_block(head_dest_pos),
        Some(REDSTONE_BLOCK_ID),
        "the redstone-block stand-in's own real content finally lands at the deferred commit"
    );
    assert_eq!(
        wire.power(new_neighbor_wire_pos),
        15,
        "the new neighbor is powered only once the real content lands, two ticks after accept -- \
         commit_extend's own unchanged, full fan-out reached it automatically"
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

/// M3 field-report test-authoring (PLAN-D10, moving_piston placeholder): the composition test
/// above proves `classify`'s own rule against a SYNTHETIC `moving_piston` id planted directly
/// via `world.set_block`. This test proves the SAME rule against a REAL placeholder a real
/// piston's own accept-time dispatch just wrote — the genuine end-to-end scenario the manager's
/// own field report calls out ("make sure it now actually triggers, since the cell really
/// exists during the window"): before this changeset's fix, the withdrawn revert would have
/// already erased piston A's own placeholder by the time this same tick's dispatch settled, so
/// a second piston's own later trigger (even within the SAME tick, since `resolve_extend` reads
/// live world state at ITS OWN dispatch time, after piston A's own revert already ran) would
/// have seen piston A's own pre-push content again, not a real `moving_piston` — silently
/// permitting an extend that real vanilla, and this engine's own `classify`, must refuse.
#[test]
fn a_real_in_flight_placeholder_blocks_a_second_pistons_extend() {
    const PISTON_A_ID: BlockStateId = BlockStateId(100);
    const PISTON_B_ID: BlockStateId = BlockStateId(101);

    // Piston A: facing East, nothing in front -- a trivial `n == 0` extend whose sole
    // destination (`armPos == head_pos`) becomes the placeholder immediately.
    let piston_a_pos = BlockPos::new(0, 0, 0);
    let facing_a = Direction::East;
    let shared_cell = facing_a.apply(piston_a_pos); // piston A's own head cell.

    // Piston B: facing West, positioned so its OWN push direction points straight into piston
    // A's own placeholder cell.
    let piston_b_pos = Direction::East.apply(shared_cell);
    let facing_b = Direction::West;

    let (source_a, signals, source_a_id) = make_signal(0);
    let piston_a = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston_a.place(piston_a_pos, facing_a, false, false);
    let piston_b = Arc::new(PistonBehavior::new(Arc::clone(&signals)));
    piston_b.place(piston_b_pos, facing_b, false, false);

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        PISTON_A_ID,
        BlockStateId(PISTON_A_ID.0 + 1),
        Arc::clone(&piston_a) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        PISTON_B_ID,
        BlockStateId(PISTON_B_ID.0 + 1),
        Arc::clone(&piston_b) as Arc<dyn BlockBehavior>,
    );
    register_piston_range(&mut behaviors, &piston_a);

    let mut h = Harness::new(behaviors);
    h.world.set_block(piston_a_pos, PISTON_A_ID);
    h.world
        .set_block(Direction::Down.apply(piston_a_pos), source_a_id);
    h.world.set_block(piston_b_pos, PISTON_B_ID);

    // Piston A extends first, alone, and its own accept-time dispatch fully settles.
    source_a.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        piston_a.on_neighbor_changed(&mut ctx, piston_a_pos, Direction::Down);
    }
    h.run_block_events(0);
    assert_eq!(
        h.world.get_block(shared_cell),
        Some(moving_piston_id(facing_a, false)),
        "piston A's own head cell genuinely, permanently holds the placeholder"
    );

    // Piston B's own `resolve_extend`, run directly against this same, now-settled world (no
    // revert has run, or ever will, to erase piston A's own placeholder first) -- the real
    // in-flight cell must refuse it.
    let result = resolve_extend(&h.world, &h.ownership, piston_b_pos, facing_b);
    assert_eq!(
        result,
        Err(ExtendAbort::Blocked),
        "piston B must refuse to extend into piston A's own real, still in-flight moving_piston \
         cell"
    );
}
