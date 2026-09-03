//! Waterlogging (Context §J) — substrate only, zero real content: the `LiquidBlockContainer`
//! two-method contract as `WaterloggableBehavior` plus a range-based `WaterloggableRegistry`
//! mirroring `BlockBehaviorRegistry`'s own shape exactly, and `SimpleWaterlogged`, a small
//! reference implementation mirroring `SimpleWaterloggedBlock`'s shared default (water only).
//!
//! Stub phase (test-authoring changeset, TEST-D45/D46): bodies are `todo!()`.
#![allow(unused_imports)]

use std::collections::HashMap;
use std::sync::Arc;

use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use super::state::FluidKind;
use crate::world_access::BlockWorldAccess;

/// Vanilla's `LiquidBlockContainer` two-method contract (Context §J). Zero real implementers
/// ship with this blueprint.
pub trait WaterloggableBehavior: Send + Sync {
    fn can_place_liquid(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        state: BlockStateId,
        kind: FluidKind,
    ) -> bool;

    /// The new (waterlogged) `BlockStateId`, or `None` if already waterlogged / `kind` rejected.
    fn waterlogged_state(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        state: BlockStateId,
        kind: FluidKind,
    ) -> Option<BlockStateId>;
}

/// Range-based dispatch, mirrors `BlockBehaviorRegistry`'s own shape exactly (M3-B01).
#[derive(Clone, Resource)]
pub struct WaterloggableRegistry {
    ranges: Vec<(BlockStateId, BlockStateId, Arc<dyn WaterloggableBehavior>)>,
}

impl WaterloggableRegistry {
    pub fn new() -> Self {
        todo!()
    }

    /// Panics on overlap with an already-registered range (mirrors `BlockBehaviorRegistry`).
    pub fn register_range(
        &mut self,
        start: BlockStateId,
        end_exclusive: BlockStateId,
        behavior: Arc<dyn WaterloggableBehavior>,
    ) {
        let _ = (start, end_exclusive, behavior);
        todo!()
    }

    /// `None` (not a `LiquidBlockContainer`) for any unregistered id — the correct default.
    pub fn resolve(&self, state: BlockStateId) -> Option<&Arc<dyn WaterloggableBehavior>> {
        let _ = state;
        todo!()
    }
}

impl Default for WaterloggableRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// `SimpleWaterloggedBlock`'s shared default (Context §J): accepts only water; the dry->wet
/// mapping is an explicit, caller-supplied pair list (no generated per-block boolean-property
/// encoding exists yet, mirroring M3-B04's own internal-store precedent for the same gap).
#[derive(Clone)]
pub struct SimpleWaterlogged {
    dry_to_wet: HashMap<BlockStateId, BlockStateId>,
}

impl SimpleWaterlogged {
    pub fn new(dry_to_wet: Vec<(BlockStateId, BlockStateId)>) -> Self {
        let _ = dry_to_wet;
        todo!()
    }
}

impl WaterloggableBehavior for SimpleWaterlogged {
    fn can_place_liquid(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        state: BlockStateId,
        kind: FluidKind,
    ) -> bool {
        let _ = (self, world, pos, state, kind);
        todo!()
    }

    fn waterlogged_state(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        state: BlockStateId,
        kind: FluidKind,
    ) -> Option<BlockStateId> {
        let _ = (self, world, pos, state, kind);
        todo!()
    }
}
