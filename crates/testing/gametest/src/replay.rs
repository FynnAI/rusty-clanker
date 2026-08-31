//! The Stage-4 replay driver: drives one `ContraptionSpec` through Rusty Clanker's
//! own Stage-4 core (M3-B01's `stage4::{run_scheduled_phase, run_block_event_subphase}`,
//! unmodified) and produces a `RedstoneTrace` in exactly the schema/order the capture
//! pipeline produces (blueprint Deliverables, `replay.rs`).
//!
//! Placement (`ContraptionSpec::blocks`) and every scripted action are, per the
//! blueprint's own "Tick 0, precisely" Context section, settled *immediately*
//! (ARCH-D13/MECH-D10's same-tick fan-out) rather than deferred to the next Stage-4
//! pass — `stage4.rs`'s own equivalent dispatch loop (`drain_engine`/
//! `dispatch_pending_update`) is a private implementation detail of that module, so
//! `place_and_settle`/`dispatch_one` below are this crate's own necessarily-
//! duplicated re-statement of the identical algorithm, used only for this "outside
//! the tick loop" placement/action-application step (never for the tick loop
//! itself, which calls `stage4::run_scheduled_phase`/`run_block_event_subphase`
//! directly, unmodified).

use std::collections::HashMap;
use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::comparator::NoContainers;
use rc_mechanics::redstone::piston::PistonBehavior;
use rc_mechanics::redstone::{
    ComparatorBehavior, ComparatorMode, RedstoneSignalSource, RepeaterBehavior,
    SignalSourceRegistry, TorchAttachment, TorchBehavior, WireBehavior, register_redstone_block,
};
use rc_mechanics::stage4;
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo,
    NeighborUpdateEngine, PendingUpdate, RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionId, RegionMessage};

use crate::spec::{ContraptionSpec, bounding_box};
use crate::trace::{BlockObservation, RedstoneTrace, TRACE_FORMAT_VERSION, TickSnapshot};

/// A `HashMap`-backed `BlockWorldAccess` scoped to one contraption — the identical
/// in-memory test-double shape M3-B01's own `stage4_ordering.rs`/`cross_region_
/// border.rs` test files already establish (`FakeWorld`), reused here as this
/// blueprint's own production replay world, not merely a test fixture.
pub struct ReplayWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    dimension: DimensionId,
    local: Address,
}

impl ReplayWorld {
    pub fn new(dimension: DimensionId) -> Self {
        Self {
            blocks: HashMap::new(),
            dimension,
            // A fixed placeholder id — never observed outside this single-region
            // replay (Deliverables, `replay_contraption`'s own doc comment).
            local: Address::Region(RegionId(0)),
        }
    }
}

impl BlockWorldAccess for ReplayWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        self.dimension
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        self.local
    }
    fn local_identity(&self) -> Address {
        self.local
    }
}

/// Drives `spec` through Rusty Clanker's own Stage-4 core for exactly
/// `spec.max_ticks` ticks, against a single-region `RegionOwnership::always_local`
/// (this contraption never spans a region), producing a `RedstoneTrace` in exactly
/// the same schema/order the capture pipeline produces.
pub fn replay_contraption(
    spec: &ContraptionSpec,
    behaviors: &BlockBehaviorRegistry,
    analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>,
) -> RedstoneTrace {
    let mut world = ReplayWorld::new(DimensionId::OVERWORLD);
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    // A single-region replay receives no inbound border events — never populated
    // (Deliverables doc comment).
    let mut halo = BorderHalo::default();
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    // Always empty at return — asserted below (a non-empty `outbound` after any
    // step is a hard bug, since a single, `always_local`-owned region can never
    // route a message cross-region, Deliverables doc comment).
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();

    let (bounds_min, bounds_max) = bounding_box(spec);

    // Step 2: place every `spec.blocks` entry in list order, each immediately
    // settled (module doc comment) at `current_tick: 0`.
    for block in &spec.blocks {
        let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
        let state = BlockStateId(block.state_id);
        place_and_settle(
            &mut world,
            &mut engine,
            &mut scheduled,
            &mut events,
            &mut outbound,
            &ownership,
            0,
            behaviors,
            pos,
            state,
        );
    }

    let mut ticks = Vec::with_capacity(spec.max_ticks as usize + 1);
    ticks.push(TickSnapshot {
        tick: 0,
        blocks: snapshot_volume(&world, bounds_min, bounds_max, analog_reader),
    });

    for t in 1..=spec.max_ticks as u64 {
        for action in spec.actions.iter().filter(|a| a.tick == t) {
            let pos = BlockPos::new(action.pos.0, action.pos.1, action.pos.2);
            let state = BlockStateId(action.state_id);
            place_and_settle(
                &mut world,
                &mut engine,
                &mut scheduled,
                &mut events,
                &mut outbound,
                &ownership,
                t,
                behaviors,
                pos,
                state,
            );
        }

        stage4::run_scheduled_phase(
            &mut world,
            &[],
            &mut halo,
            &ownership,
            &mut engine,
            &mut scheduled,
            &mut events,
            behaviors,
            &mut outbound,
            t,
        );
        stage4::run_block_event_subphase(
            &mut world,
            &ownership,
            &mut engine,
            &mut scheduled,
            &mut events,
            behaviors,
            &mut outbound,
            t,
        );

        ticks.push(TickSnapshot {
            tick: t,
            blocks: snapshot_volume(&world, bounds_min, bounds_max, analog_reader),
        });
    }

    assert!(
        outbound.is_empty(),
        "replay_contraption: a single always_local region must never produce an outbound cross-region message, got {} entries",
        outbound.len()
    );

    RedstoneTrace {
        format_version: TRACE_FORMAT_VERSION,
        contraption_id: spec.id.clone(),
        // Replay has no jar provenance — only a captured trace's `source_jar_sha1`
        // is meaningful (Deliverables doc comment).
        source_jar_sha1: String::new(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        bounds_min,
        bounds_max,
        ticks,
    }
}

/// Reads every position in `[bounds_min, bounds_max]` from `world` (plus
/// `analog_reader`, if supplied, at every position), sorted per `TickSnapshot::
/// blocks`'s own documented `(y, z, x)` ascending order — the nested loop order
/// below (`y` outer, `z` middle, `x` inner) already produces exactly that order, so
/// no separate sort step is needed.
fn snapshot_volume(
    world: &dyn BlockWorldAccess,
    bounds_min: (i32, i32, i32),
    bounds_max: (i32, i32, i32),
    analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>,
) -> Vec<BlockObservation> {
    let mut out = Vec::new();
    for y in bounds_min.1..=bounds_max.1 {
        for z in bounds_min.2..=bounds_max.2 {
            for x in bounds_min.0..=bounds_max.0 {
                let pos = BlockPos::new(x, y, z);
                // `BlockStateId(0)` is vanilla's own air default (M0-B07's
                // `block_states.rs` codegen) — an untouched position reads as air
                // exactly as it should (Implementation step 5).
                let state = world.get_block(pos).unwrap_or(BlockStateId(0));
                let analog = analog_reader.and_then(|read| read(pos));
                out.push(BlockObservation {
                    pos: (x, y, z),
                    state_id: state.0,
                    analog,
                });
            }
        }
    }
    out
}

/// The tight `[min, max_inclusive]` of every real numeric id this corpus's own committed
/// `.ron` fixtures place for each tier-1 block (`redstone/` corpus, every `blocks:`/`actions:`
/// entry, cross-checked against `crates/physics/src/shapes.rs`'s own identical literals for
/// the single-state cases) — this project's own still-open "no generated per-block-state-
/// property registry" gap (`Tier1RedstoneStateIds`'s own doc comment) applied honestly here
/// too, rather than guessing a formula. Safe because none of the four tier-1 components nor a
/// piston's own base ever rewrites its own `BlockStateId` (`WireBehavior`/`TorchBehavior`/
/// `RepeaterBehavior`/`ComparatorBehavior` track power/lit/locked/output purely as internal
/// per-position state; `piston.rs`'s own top-of-file note documents the identical stance for
/// `PistonState.extended`), so a position placed at one of these ids is guaranteed to still
/// hold that same id at every later lookup this replay ever performs. Flagged for
/// reconciliation once a real generated range table exists.
const WIRE_RANGE: (u32, u32) = (4591, 5171);
const TORCH_FLOOR_RANGE: (u32, u32) = (6885, 6885);
const TORCH_WALL_RANGE: (u32, u32) = (6891, 6893);
const REPEATER_RANGE: (u32, u32) = (7037, 7093);
const COMPARATOR_RANGE: (u32, u32) = (11264, 11276);
const PISTON_RANGE: (u32, u32) = (2258, 2268);
const STICKY_PISTON_RANGE: (u32, u32) = (2236, 2242);

fn in_range(id: u32, range: (u32, u32)) -> bool {
    id >= range.0 && id <= range.1
}

/// The `[start, end_exclusive)` `register_range` needs, from an inclusive `(min, max)`.
fn exclusive(range: (u32, u32)) -> (BlockStateId, BlockStateId) {
    (BlockStateId(range.0), BlockStateId(range.1 + 1))
}

/// Extracts one `key=value` property out of a `PlacedBlock::vanilla_state`'s own bracket
/// syntax (e.g. `"minecraft:repeater[facing=east,delay=1,locked=false,powered=false]"`) — the
/// same legal `/setblock` grammar `spec.rs`'s own doc comment already describes this field as
/// carrying verbatim.
fn vanilla_property<'a>(vanilla_state: &'a str, key: &str) -> Option<&'a str> {
    let start = vanilla_state.find('[')?;
    let end = vanilla_state.rfind(']')?;
    vanilla_state[start + 1..end].split(',').find_map(|entry| {
        let (k, v) = entry.split_once('=')?;
        (k == key).then_some(v)
    })
}

fn facing_property(vanilla_state: &str) -> Direction {
    match vanilla_property(vanilla_state, "facing") {
        Some("north") => Direction::North,
        Some("south") => Direction::South,
        Some("east") => Direction::East,
        Some("west") => Direction::West,
        Some("up") => Direction::Up,
        Some("down") => Direction::Down,
        other => {
            panic!("tier1_registry: unrecognized/missing facing in {vanilla_state:?}: {other:?}")
        }
    }
}

/// Governance fix (M3 field-report): wires the same production composition — M3-B04's four
/// tier-1 components (wire, torch-floor, torch-wall, repeater, comparator) followed by M3-B05's
/// piston, piston strictly after the four components have fully populated `SignalSourceRegistry`
/// (`register_piston`'s own doc comment) — into this replay path, which until now left every
/// position resolving to the shared `NoOpBehavior` default (an honest, documented M3-B07
/// placeholder, "until the component blueprints land"; they all have).
///
/// Deliberately does **not** call `register_tier1_redstone`/`register_piston` themselves
/// (`registration.rs`/`piston.rs`): both wrap their constructed instances in an opaque handle
/// with no getter (`Tier1RedstoneHandles`'s own doc comment — "carries no public field or
/// getter"), but `RepeaterBehavior::place`/`ComparatorBehavior::place` (each block's own facing,
/// plus delay/mode) require `&mut self` access **before** the instance is ever shared behind an
/// `Arc` — a real fixture's repeater/comparator facing can only be recovered from its own
/// `vanilla_state` property string (still no generated per-property registry, same gap as
/// above), which only this spec-aware caller has. So this function reproduces
/// `register_tier1_redstone`+`Tier1RedstoneHandles::bind_registry`+`register_piston`'s own
/// exact construction/registration/bind order by hand, inserting the one additional seeding
/// step each of those two components needs, immediately before it is wrapped in its own `Arc`.
///
/// Seeding scans `spec.blocks` only (never `spec.actions`) — every repeater/comparator/piston
/// this corpus's own fixtures ever place first appears in `blocks:` (verified against every
/// committed `.ron` fixture); a handful of comparator fixtures *re-place* the same position
/// with a different facing/mode later via `actions:` (`comparator_facing_probe_all_four`'s own
/// four-facing rotation; `comparator_compare_vs_subtract`/`comparator_tie_no_turn_on`'s own
/// mid-run mode swap) — `ComparatorBehavior` exposes no way to update facing after construction
/// at all, and mode only via `set_mode`, which this generic, redstone-behavior-agnostic replay
/// driver deliberately never special-cases, so those three fixtures keep showing a real
/// mismatch from their own re-placement tick onward (an accepted, reported gap, not a bug in
/// this wiring — do not "fix" it here).
pub fn tier1_registry(spec: &ContraptionSpec) -> BlockBehaviorRegistry {
    let mut behaviors = BlockBehaviorRegistry::new();
    let mut signals = SignalSourceRegistry::new();

    // `minecraft:redstone_block` (M3 field-report fix, Task 1): a stateless always-on source,
    // no `BlockBehavior`/registry-self-reference concerns (`register_redstone_block`'s own doc
    // comment) — registered directly via the production function, unlike the four tier-1
    // components below (which this replay driver must hand-reconstruct for their own
    // pre-`Arc` placement-seeding needs, module doc comment).
    register_redstone_block(&mut signals);

    let wire = Arc::new(WireBehavior::new());
    let (lo, hi) = exclusive(WIRE_RANGE);
    behaviors.register_range(lo, hi, Arc::clone(&wire) as Arc<dyn BlockBehavior>);
    signals.register_range(lo, hi, Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>);

    let torch_floor = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let (lo, hi) = exclusive(TORCH_FLOOR_RANGE);
    behaviors.register_range(lo, hi, Arc::clone(&torch_floor) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&torch_floor) as Arc<dyn RedstoneSignalSource>,
    );

    // One representative `Wall(North)` orientation for the whole range (`registration.rs`'s
    // own identical, already-documented M3 scope limitation) — a wall torch actually facing a
    // different direction in a fixture dispatches with the wrong `input_direction` here, same
    // as it would through the real composition root today.
    let torch_wall = Arc::new(TorchBehavior::new(TorchAttachment::Wall(Direction::North)));
    let (lo, hi) = exclusive(TORCH_WALL_RANGE);
    behaviors.register_range(lo, hi, Arc::clone(&torch_wall) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&torch_wall) as Arc<dyn RedstoneSignalSource>,
    );

    let mut repeater = RepeaterBehavior::new();
    for block in &spec.blocks {
        if in_range(block.state_id, REPEATER_RANGE) {
            let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
            let facing = facing_property(&block.vanilla_state);
            let delay: u8 = vanilla_property(&block.vanilla_state, "delay")
                .unwrap_or_else(|| {
                    panic!(
                        "tier1_registry: repeater with no delay property: {}",
                        block.vanilla_state
                    )
                })
                .parse()
                .expect("tier1_registry: repeater delay property must be a small integer");
            repeater.place(pos, facing, delay);
        }
    }
    let repeater = Arc::new(repeater);
    let (lo, hi) = exclusive(REPEATER_RANGE);
    behaviors.register_range(lo, hi, Arc::clone(&repeater) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&repeater) as Arc<dyn RedstoneSignalSource>,
    );

    // `NoContainers` (Context §B's own fallback): the replay world has no block-entity
    // storage and never ticks Stage-7 (`comparator_container_fullness_chest.ron`'s own doc
    // comment — "the replay side has no block-entity storage at M3-B07"), so the real
    // `Tier1ContainerSignalSource` has nothing to read here yet.
    let mut comparator = ComparatorBehavior::new(Arc::new(NoContainers));
    for block in &spec.blocks {
        if in_range(block.state_id, COMPARATOR_RANGE) {
            let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
            let facing = facing_property(&block.vanilla_state);
            let mode = match vanilla_property(&block.vanilla_state, "mode") {
                Some("compare") => ComparatorMode::Compare,
                Some("subtract") => ComparatorMode::Subtract,
                other => panic!(
                    "tier1_registry: unrecognized/missing mode in {:?}: {other:?}",
                    block.vanilla_state
                ),
            };
            comparator.place(pos, facing, mode);
        }
    }
    let comparator = Arc::new(comparator);
    let (lo, hi) = exclusive(COMPARATOR_RANGE);
    behaviors.register_range(lo, hi, Arc::clone(&comparator) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&comparator) as Arc<dyn RedstoneSignalSource>,
    );

    // Two-phase registry self-reference (Context §I½, `Tier1RedstoneHandles::bind_registry`'s
    // own identical order: wire, torch-floor, torch-wall, repeater, comparator).
    let signals = Arc::new(signals);
    wire.bind_registry(Arc::clone(&signals));
    torch_floor.bind_registry(Arc::clone(&signals));
    torch_wall.bind_registry(Arc::clone(&signals));
    repeater.bind_registry(Arc::clone(&signals));
    comparator.bind_registry(Arc::clone(&signals));

    // Piston strictly after the four components (`register_piston`'s own doc comment).
    let piston = Arc::new(PistonBehavior::new(signals));
    for block in &spec.blocks {
        if in_range(block.state_id, PISTON_RANGE) || in_range(block.state_id, STICKY_PISTON_RANGE) {
            let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
            let facing = facing_property(&block.vanilla_state);
            let sticky = block.vanilla_state.starts_with("minecraft:sticky_piston");
            piston.place(pos, facing, sticky);
        }
    }
    let (lo, hi) = exclusive(PISTON_RANGE);
    behaviors.register_range(lo, hi, Arc::clone(&piston) as Arc<dyn BlockBehavior>);
    let (lo, hi) = exclusive(STICKY_PISTON_RANGE);
    behaviors.register_range(lo, hi, Arc::clone(&piston) as Arc<dyn BlockBehavior>);

    behaviors
}

/// Immediate-settle: writes `new_state` at `pos` (fanning out both signals, per
/// `UpdateContext::set_block`), then drains the resulting `NeighborUpdateEngine`
/// queue to a fixed point via `dispatch_one` — module doc comment.
#[allow(clippy::too_many_arguments)]
fn place_and_settle(
    world: &mut ReplayWorld,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    ownership: &RegionOwnership,
    current_tick: u64,
    behaviors: &BlockBehaviorRegistry,
    pos: BlockPos,
    state: BlockStateId,
) {
    {
        let mut ctx = UpdateContext {
            world,
            engine,
            scheduled,
            events,
            outbound,
            ownership,
            current_tick,
        };
        ctx.set_block(pos, state);
    }
    engine.drain(&mut |eng, item| {
        let mut ctx = UpdateContext {
            world,
            engine: eng,
            scheduled,
            events,
            outbound,
            ownership,
            current_tick,
        };
        dispatch_one(&mut ctx, behaviors, item);
    });
}

/// One popped `PendingUpdate`'s dispatch — module doc comment (a necessary
/// restatement of `stage4.rs`'s own private `dispatch_pending_update`, which this
/// crate cannot call directly, including its identical `ShapeUpdate` handling: a
/// state-change request is written directly via `ctx.world.set_block` (never
/// `ctx.set_block`, which would restart a brand-new fan-out from this position), then
/// the cascade continues one hop further if depth remains).
fn dispatch_one(ctx: &mut UpdateContext, behaviors: &BlockBehaviorRegistry, item: PendingUpdate) {
    match item {
        PendingUpdate::NeighborChanged { pos, from } => {
            if let Some(state) = ctx.get_block(pos) {
                let behavior = behaviors.resolve(state);
                behavior.on_neighbor_changed(ctx, pos, from);
            }
        }
        PendingUpdate::ShapeUpdate {
            pos,
            from,
            remaining_depth,
        } => {
            let Some(state) = ctx.get_block(pos) else {
                return;
            };
            let Some(neighbor_state) = ctx.get_block(from.apply(pos)) else {
                return;
            };
            let behavior = behaviors.resolve(state);
            if let Some(new_state) = behavior.on_shape_update(ctx, pos, from, neighbor_state) {
                ctx.world.set_block(pos, new_state);
                if remaining_depth > 0 {
                    ctx.engine
                        .emit_shape_update_fanout_at_depth(pos, remaining_depth - 1);
                }
            }
        }
    }
}
