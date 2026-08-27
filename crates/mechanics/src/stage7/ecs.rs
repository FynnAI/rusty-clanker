//! `bevy_ecs`/`rc-scheduler` adapter for Stage 7 (feature `server-systems`). `EcsBlockEntityWorld`
//! implements `BlockEntityWorldAccess` over a `Query<(Entity, &BlockEntityHeader,
//! Option<&mut HopperBlockEntity>, Option<&mut FurnaceBlockEntity>, Option<&mut
//! ChestBlockEntity>)>` plus a `Query<&BlockEntityIndex>` (keyed by chunk entity) and
//! `ChunkIndex` (M3-B01's own resource, reused unmodified).

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use rc_chunk_storage::BlockEntityIndex;
use rc_core::{BlockPos, ChunkKey};
use rc_scheduler::{DomainGroup, RcExecutorBuilder, SystemFactory};

use crate::block_entity::chest::ChestBlockEntity;
use crate::block_entity::container_signal_source::{
    ContainerSignalsResource, Tier1ContainerSignalSource,
};
use crate::block_entity::furnace::{
    FuelTable, FurnaceBlockEntity, FurnaceLitStateResolver, SmeltingRecipeTable,
};
use crate::block_entity::hopper::HopperBlockEntity;
use crate::block_entity::{BlockEntityHeader, BlockEntityKind, BlockEntityWorldAccess};
use crate::container::{
    DefaultMaxStackSize, ItemMaxStackSize, MaxStackSizeResource, TierOneContainer,
};
use crate::stage4::ecs::ChunkIndex;

type BlockEntityQueryData = (
    Entity,
    &'static BlockEntityHeader,
    Option<&'static mut HopperBlockEntity>,
    Option<&'static mut FurnaceBlockEntity>,
    Option<&'static mut ChestBlockEntity>,
);

/// A `Query`-backed `BlockEntityWorldAccess` implementation, constructed fresh inside each
/// Stage-7 system call (mirroring `stage4::ecs::EcsBlockWorld`'s own "constructed fresh inside
/// each system call" convention). `entity_info`/`pos_to_entity` are built once, at
/// construction, from one read-only pass over `headers` (Deliverables: "building a
/// `HashMap<BlockPos, Entity>` once per call").
pub struct EcsBlockEntityWorld<'w, 's> {
    headers: Query<'w, 's, BlockEntityQueryData>,
    chunk_be_index: Query<'w, 's, &'static BlockEntityIndex>,
    chunk_index: &'w ChunkIndex,
    entity_info: HashMap<Entity, (BlockPos, BlockEntityKind)>,
    pos_to_entity: HashMap<BlockPos, Entity>,
}

impl<'w, 's> EcsBlockEntityWorld<'w, 's> {
    fn new(
        headers: Query<'w, 's, BlockEntityQueryData>,
        chunk_be_index: Query<'w, 's, &'static BlockEntityIndex>,
        chunk_index: &'w ChunkIndex,
    ) -> Self {
        todo!()
    }
}

impl<'w, 's> BlockEntityWorldAccess for EcsBlockEntityWorld<'w, 's> {
    fn region_chunks(&self) -> Vec<ChunkKey> {
        todo!()
    }

    fn block_entities_in_chunk(&self, chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)> {
        todo!()
    }

    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer> {
        todo!()
    }

    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut HopperBlockEntity> {
        todo!()
    }

    fn get_furnace_mut(&mut self, pos: BlockPos) -> Option<&mut FurnaceBlockEntity> {
        todo!()
    }

    fn get_chest_mut(&mut self, pos: BlockPos) -> Option<&mut ChestBlockEntity> {
        todo!()
    }

    fn is_locked_by_redstone(&self, pos: BlockPos) -> bool {
        todo!()
    }

    fn swap_furnace_lit_state(
        &mut self,
        pos: BlockPos,
        now_lit: bool,
        resolver: Option<&dyn FurnaceLitStateResolver>,
    ) {
        todo!()
    }
}

/// Registers `system_block_entity_tick` into `DomainGroup::BlockEntity` (`order_tag = 0`, the
/// only system ever registered there).
pub fn register_stage7(builder: &mut RcExecutorBuilder) {
    todo!()
}

/// Inserts `SmeltingRecipeTable::minimal_tier1()`, `FuelTable::minimal_tier1()`, and
/// `MaxStackSizeResource(Arc::new(DefaultMaxStackSize))` as resources — the Stage-7 system's
/// own required-but-`Default`-able dependencies. Does **not** insert `ContainerSignalsResource`
/// — that resource has no sensible uniform default (Context), so the composition root inserts
/// it directly, the same status `WorldSeed` already has.
pub fn bootstrap_default_stage7_resources(world: &mut World) {
    todo!()
}
