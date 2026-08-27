//! `bevy_ecs`/`rc-scheduler` adapter for Stage 5 (feature `server-systems`). `Stage5BlockWorld`
//! is a fresh, locally-scoped `BlockWorldAccess` implementation, structurally identical to
//! M3-B01's own `stage4::ecs::EcsBlockWorld` but declared here rather than imported: that
//! type's own fields are private to its defining module (M3-B01's own Deliverables show no
//! public constructor), so a sibling module cannot construct one from parts — only the type
//! *name* would be importable, not a way to build a new instance of it. Reproducing the same
//! few-line wrapper here (rather than modifying M3-B01's own file to add a cross-module
//! constructor, which this blueprint's own Prerequisites commit to leaving unmodified except
//! `behavior.rs`) is the smaller, safer edit.

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, BlockStateId, ChunkKeyTag};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, RegionMessage};
use rc_scheduler::{
    BorderUpdateInbox, CurrentTick, DomainGroup, RcExecutorBuilder, RegionMessageOutbox,
    SystemFactory,
};

use crate::behavior::BlockBehaviorRegistry;
use crate::block_event::BlockEventQueue;
use crate::border::RegionOwnership;
use crate::neighbor_update::NeighborUpdateEngine;
use crate::random_tick::WorldSeed;
use crate::scheduled_tick::ScheduledTickQueue;
use crate::stage4::ecs::ChunkIndex;
use crate::world_access::BlockWorldAccess;

struct Stage5BlockWorld<'w, 's> {
    query: Query<'w, 's, (&'static ChunkKeyTag, &'static mut BlockStateColumn)>,
    chunk_index: &'w ChunkIndex,
    ownership: &'w RegionOwnership,
}

impl<'w, 's> Stage5BlockWorld<'w, 's> {
    fn local_pos(pos: BlockPos) -> (u8, u8) {
        (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8)
    }
}

impl<'w, 's> BlockWorldAccess for Stage5BlockWorld<'w, 's> {
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

/// Registers `system_random_tick` into `DomainGroup::RandomTick` (`order_tag = 0`, the only
/// system this blueprint ever registers there — Context). Gathers this region's own loaded
/// `ChunkKeyTag` list from `ChunkIndex` (reused from M3-B01's own Stage-4 adapter, unmodified
/// — its own field is `pub`, so reading it cross-module is fine even though `EcsBlockWorld`
/// itself is not constructible cross-module), sorts by `(x, z)` ascending, builds a
/// `Stage5BlockWorld` (above), and calls `stage5::run_random_tick_phase`. Requires `WorldSeed`
/// to be present as a resource (inserted by the composition root — no sensible uniform default
/// exists, mirroring `RegionOwnership`'s own identical per-region-data status in M3-B01).
pub fn register_stage5(builder: &mut RcExecutorBuilder, random_tick_speed: u32) {
    todo!()
}
