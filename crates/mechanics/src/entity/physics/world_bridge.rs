//! A minimal, **read-only** `BlockWorldAccess` adapter for Stage 6b's own physics/fluid
//! queries — deliberately NOT `stage4::ecs::EcsBlockWorld` (M3-B01), which requires a
//! `&mut BlockStateColumn` `Query`, forcing an unnecessary write-conflict declaration on a
//! system (this one) that only ever reads block state. Reuses `stage4::ecs::ChunkIndex`
//! (M3-B01, unmodified) for the position→chunk-entity lookup.

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, BlockStateId, ChunkKeyTag, WORLD_HEIGHT, WORLD_MIN_Y};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::Address;

use crate::stage4::ecs::ChunkIndex;
use crate::world_access::BlockWorldAccess;

fn local_pos(pos: BlockPos) -> (u8, u8) {
    (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8)
}

fn y_in_world_bounds(world_y: i32) -> bool {
    (WORLD_MIN_Y..WORLD_MIN_Y + WORLD_HEIGHT).contains(&world_y)
}

/// `set_block`/`owner_of`/`local_identity` are part of the shared `BlockWorldAccess` trait
/// but are never called by any of this blueprint's own code paths against this adapter —
/// each panics with an explanatory message rather than silently no-opping, so a future
/// accidental call is a loud bug, not a silent one.
///
/// **Owns** its `Query` (by value), mirroring `stage4::ecs::EcsBlockWorld`'s own proven
/// shape exactly — not `&'s Query<...>` (`docs/findings-for-planning.md`): `Query<'world,
/// 'state, D>`'s data type parameter `D` is invariant, so a REFERENCE to a function-local
/// `Query` cannot satisfy this struct's own `&'static`-annotated `D` (a genuine Rust
/// lifetime error, not a style choice) — every one of this crate's own established
/// `BlockWorldAccess` adapters (`stage4::ecs::EcsBlockWorld`, `stage5::ecs::Stage5BlockWorld`)
/// already takes its `Query` by value for exactly this reason.
pub struct ReadOnlyBlockWorld<'w, 's> {
    query: Query<'w, 's, (&'static ChunkKeyTag, &'static BlockStateColumn)>,
    index: &'s ChunkIndex,
    dimension: DimensionId,
}

impl<'w, 's> ReadOnlyBlockWorld<'w, 's> {
    pub fn new(
        query: Query<'w, 's, (&'static ChunkKeyTag, &'static BlockStateColumn)>,
        index: &'s ChunkIndex,
        dimension: DimensionId,
    ) -> Self {
        Self {
            query,
            index,
            dimension,
        }
    }
}

impl<'w, 's> BlockWorldAccess for ReadOnlyBlockWorld<'w, 's> {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        if !y_in_world_bounds(pos.y) {
            return None;
        }
        let chunk_key = pos.chunk_key(self.dimension);
        let entity = *self.index.0.get(&chunk_key)?;
        let (_, column) = self.query.get(entity).ok()?;
        let (lx, lz) = local_pos(pos);
        Some(column.get(lx, pos.y, lz))
    }

    fn set_block(&mut self, _pos: BlockPos, _state: BlockStateId) -> bool {
        panic!(
            "ReadOnlyBlockWorld::set_block called — Stage 6b's own physics/fluid queries \
             never write block state"
        );
    }

    fn dimension(&self) -> DimensionId {
        self.dimension
    }

    /// Panics: this adapter never crosses a region border by construction (every entity this
    /// blueprint ticks stays inside its own owning region within one tick).
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        panic!(
            "ReadOnlyBlockWorld::owner_of called — Stage 6b's own physics/fluid queries never \
             cross a region border within one tick"
        );
    }

    fn local_identity(&self) -> Address {
        panic!(
            "ReadOnlyBlockWorld::local_identity called — Stage 6b's own physics/fluid queries \
             never need this adapter's own local address"
        );
    }
}
