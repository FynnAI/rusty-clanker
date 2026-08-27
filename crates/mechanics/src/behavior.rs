use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_messaging::{Address, RegionMessage};
use std::sync::Arc;

use crate::block_event::{BlockEvent, BlockEventQueue};
use crate::border::{self, RegionOwnership};
use crate::direction::Direction;
use crate::neighbor_update::NeighborUpdateEngine;
use crate::scheduled_tick::{ScheduledTickQueue, TickPriority};
use crate::world_access::BlockWorldAccess;

/// Everything a `BlockBehavior` callback may read/mutate during Stage 4 (Context: the
/// bundled-references pattern; every field is a plain borrow, no `bevy_ecs` type appears
/// here). `set_block` is the **only** way a behavior mutates block state — it performs the
/// full ARCH-D13 neighbor-changed + shape-update fan-out (local dispatch or cross-region
/// routing per-neighbor, `border.rs`) automatically; a behavior never calls
/// `BlockWorldAccess::set_block` directly. `ownership` is set once, at construction (by
/// `run_scheduled_phase`/`run_block_event_subphase` in `stage4.rs`, or directly by a test),
/// and never reassigned mid-context — `border.rs`'s functions read it from here rather than
/// taking it as a separate parameter, so there is exactly one place a caller supplies it.
pub struct UpdateContext<'a> {
    pub world: &'a mut dyn BlockWorldAccess,
    pub engine: &'a mut NeighborUpdateEngine,
    pub scheduled: &'a mut ScheduledTickQueue,
    pub events: &'a mut BlockEventQueue,
    pub outbound: &'a mut Vec<(Address, RegionMessage)>,
    pub ownership: &'a RegionOwnership,
    pub current_tick: u64,
}

impl<'a> UpdateContext<'a> {
    pub fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        todo!()
    }

    /// Writes `new_state` at `pos` (must be local — Context), then fans out both signals from
    /// `pos` (`border.rs`'s `fan_out_from_changed_block`). Returns `true` iff the stored value
    /// actually changed (a no-op write still fans out — matches vanilla's own unconditional
    /// `updateNeighborsAt` behavior after any `setBlock` call with `UPDATE_NEIGHBORS` set).
    pub fn set_block(&mut self, pos: BlockPos, new_state: BlockStateId) -> bool {
        todo!()
    }

    pub fn schedule_block_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority) {
        todo!()
    }

    pub fn schedule_fluid_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority) {
        todo!()
    }

    pub fn emit_block_event(&mut self, pos: BlockPos, event_id: u8, event_param: u8, block_state: BlockStateId) {
        todo!()
    }
}

/// The dispatch target for one block-state range (Context: "tier-1 registry"). Every method
/// has a no-op default — a behavior overrides only what it needs.
pub trait BlockBehavior: Send + Sync {
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, _pos: BlockPos, _from: Direction) {}
    /// Returning `Some(new_state)` requests this block's own state be replaced (vanilla's
    /// `updateShape` return-value contract). Returning `None` (the default) means no change.
    fn on_shape_update(
        &self,
        _ctx: &mut UpdateContext,
        _pos: BlockPos,
        _from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        None
    }
    fn on_scheduled_tick(&self, _ctx: &mut UpdateContext, _pos: BlockPos) {}
    fn on_block_event(&self, _ctx: &mut UpdateContext, _pos: BlockPos, _event: &BlockEvent) {}
}

/// The tier-1 default: every method's default no-op body, shared by every unregistered
/// block-state id.
pub struct NoOpBehavior;
impl BlockBehavior for NoOpBehavior {}

/// Range-based dispatch (Context: "no generated registry available yet"). Ranges must be
/// non-overlapping; `register_range` panics on overlap with an already-registered range.
#[derive(Clone, Resource)]
pub struct BlockBehaviorRegistry {
    ranges: Vec<(BlockStateId, BlockStateId, Arc<dyn BlockBehavior>)>,
    default: Arc<dyn BlockBehavior>,
}

impl BlockBehaviorRegistry {
    pub fn new() -> Self {
        todo!()
    }

    pub fn register_range(&mut self, start: BlockStateId, end_exclusive: BlockStateId, behavior: Arc<dyn BlockBehavior>) {
        todo!()
    }

    pub fn register_one(&mut self, state: BlockStateId, behavior: Arc<dyn BlockBehavior>) {
        todo!()
    }

    /// Returns the matching range's behavior, or the shared `NoOpBehavior` default.
    pub fn resolve(&self, state: BlockStateId) -> &Arc<dyn BlockBehavior> {
        todo!()
    }
}

impl Default for BlockBehaviorRegistry {
    fn default() -> Self {
        todo!()
    }
}
