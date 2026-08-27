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

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::stage4;
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo, NeighborUpdateEngine,
    PendingUpdate, RegionOwnership, ScheduledTickQueue, UpdateContext,
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

/// This blueprint's own baseline: every position resolves to `NoOpBehavior` (Context,
/// "Scope boundary" — this blueprint ships zero real component behaviors). Each
/// sibling M3 component-behavior blueprint extends this exact function with its own
/// `register_range` call.
pub fn tier1_registry() -> BlockBehaviorRegistry {
    BlockBehaviorRegistry::new()
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
