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
        todo!()
    }
}

impl BlockWorldAccess for ReplayWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        todo!()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        todo!()
    }
    fn dimension(&self) -> DimensionId {
        todo!()
    }
    fn owner_of(&self, chunk: ChunkKey) -> Address {
        todo!()
    }
    fn local_identity(&self) -> Address {
        todo!()
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
    todo!()
}

/// Reads every position in `[bounds_min, bounds_max]` from `world` (plus
/// `analog_reader`, if supplied, at every position), sorted per `TickSnapshot::
/// blocks`'s own documented `(y, z, x)` ascending order.
fn snapshot_volume(
    world: &dyn BlockWorldAccess,
    bounds_min: (i32, i32, i32),
    bounds_max: (i32, i32, i32),
    analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>,
) -> Vec<BlockObservation> {
    todo!()
}

/// This blueprint's own baseline: every position resolves to `NoOpBehavior` (Context,
/// "Scope boundary" — this blueprint ships zero real component behaviors). Each
/// sibling M3 component-behavior blueprint extends this exact function with its own
/// `register_range` call.
pub fn tier1_registry() -> BlockBehaviorRegistry {
    todo!()
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
    todo!()
}

/// One popped `PendingUpdate`'s dispatch — module doc comment (a necessary
/// restatement of `stage4.rs`'s own private `dispatch_pending_update`, which this
/// crate cannot call directly, including its identical `ShapeUpdate` handling: a
/// state-change request is written directly via `ctx.world.set_block` (never
/// `ctx.set_block`, which would restart a brand-new fan-out from this position), then
/// the cascade continues one hop further if depth remains).
fn dispatch_one(ctx: &mut UpdateContext, behaviors: &BlockBehaviorRegistry, item: PendingUpdate) {
    todo!()
}
