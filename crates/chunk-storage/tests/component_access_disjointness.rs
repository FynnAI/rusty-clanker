//! The ARCH-D8 domain claim: each of the eight WORLD-D1 chunk components occupies its
//! own independent ECS storage slot, so declared `&mut` access to two different ones
//! never conflicts (M2-B01 Deliverables, Context's "Component decomposition" section).

use std::collections::HashSet;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::QueryState;
use bevy_ecs::world::World;
use rc_chunk_storage::{
    BiomeColumn, BiomeId, BlockEntityIndex, BlockStateColumn, BlockStateId, ChunkGenStatus,
    ChunkKeyTag, ChunkPersistenceState, ChunkStatus, HeightmapSet, LightColumn, PaletteThresholds,
};

#[test]
fn all_eight_components_register_to_distinct_component_ids() {
    let mut world = World::new();
    let ids: HashSet<_> = [
        world.register_component::<ChunkKeyTag>(),
        world.register_component::<BlockStateColumn>(),
        world.register_component::<BiomeColumn>(),
        world.register_component::<LightColumn>(),
        world.register_component::<HeightmapSet>(),
        world.register_component::<BlockEntityIndex>(),
        world.register_component::<ChunkStatus>(),
        world.register_component::<ChunkPersistenceState>(),
    ]
    .into_iter()
    .collect();
    assert_eq!(ids.len(), 8);
}

/// A full chunk entity carrying all eight components, mirroring how a future
/// worldgen/load blueprint will construct one.
fn spawn_full_chunk_entity(world: &mut World) -> Entity {
    world
        .spawn((
            ChunkKeyTag(rc_core::ChunkKey::new(
                rc_core::DimensionId::OVERWORLD,
                0,
                0,
            )),
            BlockStateColumn::new(BlockStateId(0), PaletteThresholds::blocks(15)),
            BiomeColumn::new(BiomeId(0), PaletteThresholds::biomes(4)),
            LightColumn::new_uninitialized(),
            HeightmapSet::new_uniform(-59),
            BlockEntityIndex::new(),
            ChunkStatus(ChunkGenStatus::Full),
            ChunkPersistenceState::new(),
        ))
        .id()
}

#[test]
fn block_state_and_light_queries_declare_disjoint_write_access() {
    let mut world = World::new();
    spawn_full_chunk_entity(&mut world);

    let block_state_query = QueryState::<&mut BlockStateColumn>::new(&mut world);
    let light_query = QueryState::<&mut LightColumn>::new(&mut world);

    assert!(
        block_state_query
            .component_access()
            .is_compatible(light_query.component_access())
    );
}

#[test]
fn block_state_and_persistence_queries_are_also_disjoint() {
    let mut world = World::new();
    spawn_full_chunk_entity(&mut world);

    let block_state_query = QueryState::<&mut BlockStateColumn>::new(&mut world);
    let persistence_query = QueryState::<&mut ChunkPersistenceState>::new(&mut world);

    assert!(
        block_state_query
            .component_access()
            .is_compatible(persistence_query.component_access())
    );
}
