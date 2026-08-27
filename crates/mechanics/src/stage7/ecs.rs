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
use crate::block_entity::container_signal_source::ContainerSignalsResource;
use crate::block_entity::furnace::{
    FuelTable, FurnaceBlockEntity, FurnaceLitStateResolver, SmeltingRecipeTable,
};
use crate::block_entity::hopper::HopperBlockEntity;
use crate::block_entity::{BlockEntityHeader, BlockEntityKind, BlockEntityWorldAccess};
use crate::container::{DefaultMaxStackSize, MaxStackSizeResource, TierOneContainer};
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
        let mut entity_info = HashMap::new();
        let mut pos_to_entity = HashMap::new();
        for (entity, header, hopper, furnace, _chest) in headers.iter() {
            let kind = if hopper.is_some() {
                BlockEntityKind::Hopper
            } else if furnace.is_some() {
                BlockEntityKind::Furnace
            } else {
                BlockEntityKind::Chest
            };
            entity_info.insert(entity, (header.pos, kind));
            pos_to_entity.insert(header.pos, entity);
        }
        Self {
            headers,
            chunk_be_index,
            chunk_index,
            entity_info,
            pos_to_entity,
        }
    }
}

impl<'w, 's> BlockEntityWorldAccess for EcsBlockEntityWorld<'w, 's> {
    fn region_chunks(&self) -> Vec<ChunkKey> {
        let mut keys: Vec<ChunkKey> = self.chunk_index.0.keys().copied().collect();
        keys.sort_unstable_by_key(|k| (k.x, k.z));
        keys
    }

    fn block_entities_in_chunk(&self, chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)> {
        let Some(&chunk_entity) = self.chunk_index.0.get(&chunk) else {
            return Vec::new();
        };
        let Ok(index) = self.chunk_be_index.get(chunk_entity) else {
            return Vec::new();
        };
        index
            .entities()
            .iter()
            .filter_map(|e| self.entity_info.get(e).copied())
            .collect()
    }

    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer> {
        let &entity = self.pos_to_entity.get(&pos)?;
        let (_, _, hopper, furnace, chest) = self.headers.get_mut(entity).ok()?;
        if let Some(h) = hopper {
            return Some(h.into_inner());
        }
        if let Some(f) = furnace {
            return Some(f.into_inner());
        }
        if let Some(c) = chest {
            return Some(c.into_inner());
        }
        None
    }

    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut HopperBlockEntity> {
        let &entity = self.pos_to_entity.get(&pos)?;
        let (_, _, hopper, _, _) = self.headers.get_mut(entity).ok()?;
        hopper.map(Mut::into_inner)
    }

    fn get_furnace_mut(&mut self, pos: BlockPos) -> Option<&mut FurnaceBlockEntity> {
        let &entity = self.pos_to_entity.get(&pos)?;
        let (_, _, _, furnace, _) = self.headers.get_mut(entity).ok()?;
        furnace.map(Mut::into_inner)
    }

    fn get_chest_mut(&mut self, pos: BlockPos) -> Option<&mut ChestBlockEntity> {
        let &entity = self.pos_to_entity.get(&pos)?;
        let (_, _, _, _, chest) = self.headers.get_mut(entity).ok()?;
        chest.map(Mut::into_inner)
    }

    /// Not implemented by this blueprint's own shipped adapter (Context: "Redstone lock" — no
    /// comparator/wire/redstone-signal-strength query exists in `rc-mechanics` outside Stage
    /// 4's own internal state, which this Stage-7-scoped adapter has no access to). Always
    /// `false` — a documented, named gap, not silently wrong.
    fn is_locked_by_redstone(&self, _pos: BlockPos) -> bool {
        false
    }

    /// A no-op — this blueprint ships no real `FurnaceLitStateResolver` (Context: "Lit-state
    /// block swap"). `resolver` is always `None` from this blueprint's own `system_block_entity_
    /// tick`; a future blueprint with a legal path to a real generated block-state table
    /// supplies one, at which point this adapter resolves and writes the swapped state via
    /// `BlockWorldAccess::set_block` (not exposed to this Stage-7-scoped adapter today).
    fn swap_furnace_lit_state(
        &mut self,
        _pos: BlockPos,
        _now_lit: bool,
        _resolver: Option<&dyn FurnaceLitStateResolver>,
    ) {
    }
}

/// Registers `system_block_entity_tick` into `DomainGroup::BlockEntity` (`order_tag = 0`, the
/// only system ever registered there).
pub fn register_stage7(builder: &mut RcExecutorBuilder) {
    builder.register_system(
        DomainGroup::BlockEntity,
        block_entity_tick_factory(),
        vec![],
    );
}

#[allow(clippy::too_many_arguments)]
fn system_block_entity_tick(
    headers: Query<BlockEntityQueryData>,
    chunk_be_index: Query<&'static BlockEntityIndex>,
    chunk_index: Res<ChunkIndex>,
    recipes: Res<SmeltingRecipeTable>,
    fuels: Res<FuelTable>,
    max_stack: Res<MaxStackSizeResource>,
    container_signals: Res<ContainerSignalsResource>,
) {
    let mut world = EcsBlockEntityWorld::new(headers, chunk_be_index, &chunk_index);
    crate::stage7::run_block_entity_tick(
        &mut world,
        &recipes,
        &fuels,
        max_stack.0.as_ref(),
        None,
        container_signals.0.as_ref(),
    );
}

fn block_entity_tick_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(system_block_entity_tick))
            as Box<dyn System<In = (), Out = ()>>
    })
}

/// Inserts `SmeltingRecipeTable::minimal_tier1()`, `FuelTable::minimal_tier1()`, and
/// `MaxStackSizeResource(Arc::new(DefaultMaxStackSize))` as resources — the Stage-7 system's
/// own required-but-`Default`-able dependencies. Does **not** insert `ContainerSignalsResource`
/// — that resource has no sensible uniform default (Context), so the composition root inserts
/// it directly, the same status `WorldSeed` already has.
pub fn bootstrap_default_stage7_resources(world: &mut World) {
    world.insert_resource(SmeltingRecipeTable::minimal_tier1());
    world.insert_resource(FuelTable::minimal_tier1());
    world.insert_resource(MaxStackSizeResource(std::sync::Arc::new(
        DefaultMaxStackSize,
    )));
}
