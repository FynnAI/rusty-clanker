//! M4-B07 — cross-worker-count determinism acceptance tests (PERF-D3): the same
//! final `LightColumn` state and the same emitted `LightBorderUpdate` sequence,
//! regardless of `RcWorkerPool::new(n)`'s own `n`.

use bevy_ecs::prelude::*;
use rc_chunk_storage::{
    BlockStateColumn, BlockStateId, ChunkKeyTag, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::{
    LightDirtyQueue, LightPropagatorState, LightProperties, LightPropertiesRegistry,
    SkyLightSourceColumn,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{LightBorderInbox, RegionMessageOutbox};

const AIR: BlockStateId = BlockStateId(0);
const EMITTER: BlockStateId = BlockStateId(1);
const CEILING: BlockStateId = BlockStateId(99);
const CEILING_Y: i32 = 10;

/// A ceiling well above the tested `y=0` row (see `light_chunk_border.rs`'s own
/// identical module doc comment for the full "why" -- keeps Stage 8's own
/// unconditional per-dirty-position `check_node_sky` call from seeding a spurious
/// sky-light source exactly at the emitter's own position).
fn ceilinged_blocks() -> BlockStateColumn {
    let mut blocks = BlockStateColumn::new(AIR, PaletteThresholds::blocks(15));
    for x in 0u8..16 {
        for z in 0u8..16 {
            blocks.set(x, CEILING_Y, z, CEILING);
        }
    }
    blocks
}

/// Every section `Filled(0)` -- see `light_chunk_border.rs`'s own identical helper
/// for the full "why" (sidesteps Stage 8's own step-2 bulk-recompute pass, which
/// would otherwise defer sky-light entries across the region boundary these tests
/// are not exercising).
fn already_loaded_light_column() -> LightColumn {
    let mut column = LightColumn::new_uninitialized();
    for i in 0..rc_chunk_storage::LIGHT_SECTION_COUNT {
        let section = column.section_mut(i);
        section.sky = rc_chunk_storage::LightNibbles::Filled(0);
        section.block = rc_chunk_storage::LightNibbles::Filled(0);
    }
    column
}

fn spawn_chunk(world: &mut World, key: ChunkKey, properties: &LightPropertiesRegistry) -> Entity {
    let blocks = ceilinged_blocks();
    let heightmap = HeightmapSet::new_uniform(CEILING_Y + 1);
    let sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, properties);
    world
        .spawn((
            ChunkKeyTag(key),
            blocks,
            already_loaded_light_column(),
            sky_sources,
            heightmap,
            LightPropagatorState::new(),
        ))
        .id()
}

fn build_world(cross_region: bool) -> (World, Entity, Entity) {
    let mut world = World::new();
    let mut properties = LightPropertiesRegistry::new();
    properties.register_one(
        EMITTER,
        LightProperties {
            block_emission: 14,
            opacity: 0,
            occludes_face: [false; 6],
        },
    );
    properties.register_one(CEILING, LightProperties::OPAQUE);

    let chunk_a = spawn_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        &properties,
    );
    let chunk_b = spawn_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 1, 0),
        &properties,
    );

    world.insert_resource(properties);

    if cross_region {
        let remote_chunk = ChunkKey::new(DimensionId::OVERWORLD, 1, 0);
        world.insert_resource(rc_mechanics::RegionOwnership {
            local: Address::Region(RegionId(1)),
            resolve: Box::new(move |chunk: ChunkKey| {
                if chunk == remote_chunk {
                    Address::Region(RegionId(2))
                } else {
                    Address::Region(RegionId(1))
                }
            }),
        });
    } else {
        world.insert_resource(rc_mechanics::RegionOwnership::always_local(
            Address::Region(RegionId(1)),
        ));
    }
    world.insert_resource(RegionMessageOutbox::default());
    world.insert_resource(LightDirtyQueue::default());
    world.insert_resource(LightBorderInbox::default());

    world
        .resource_mut::<LightDirtyQueue>()
        .mark(BlockPos::new(15, 0, 0), AIR, EMITTER);
    world
        .get_mut::<BlockStateColumn>(chunk_a)
        .unwrap()
        .set(15, 0, 0, EMITTER);

    (world, chunk_a, chunk_b)
}

fn light_columns_equal(a: &LightColumn, b: &LightColumn) -> bool {
    for i in 0..rc_chunk_storage::LIGHT_SECTION_COUNT {
        let sa = a.section(i);
        let sb = b.section(i);
        if sa.sky != sb.sky || sa.block != sb.block {
            return false;
        }
    }
    true
}

#[test]
fn stage8_final_state_identical_across_worker_counts() {
    let mut reference: Option<(LightColumn, LightColumn)> = None;

    for &n in &[1usize, 2, 8] {
        let (mut world, chunk_a, chunk_b) = build_world(false);
        let pool = RcWorkerPool::new(n);
        rc_mechanics::light::stage8_ecs::lighting_stage_driver(&mut world, &pool);

        let column_a = world.get::<LightColumn>(chunk_a).unwrap().clone();
        let column_b = world.get::<LightColumn>(chunk_b).unwrap().clone();

        match &reference {
            None => reference = Some((column_a, column_b)),
            Some((ref_a, ref_b)) => {
                assert!(
                    light_columns_equal(&column_a, ref_a),
                    "chunk A state diverged at n={n}"
                );
                assert!(
                    light_columns_equal(&column_b, ref_b),
                    "chunk B state diverged at n={n}"
                );
            }
        }
    }
}

#[test]
fn stage8_emitted_light_border_update_sequence_identical_across_worker_counts() {
    let mut reference: Option<Vec<(Address, RegionMessage)>> = None;

    for &n in &[1usize, 2, 8] {
        let (mut world, _chunk_a, _chunk_b) = build_world(true);
        let pool = RcWorkerPool::new(n);
        rc_mechanics::light::stage8_ecs::lighting_stage_driver(&mut world, &pool);

        let bus = world.resource_mut::<RegionMessageOutbox>().take();
        let mut message_state = rc_messaging::RegionMessageState::new();
        message_state.merge(bus);
        let sent = message_state.drain_outbox(RegionId(1), 0);
        let sequence: Vec<(Address, RegionMessage)> =
            sent.into_iter().map(|m| (m.to, m.payload)).collect();

        match &reference {
            None => reference = Some(sequence),
            Some(ref_seq) => {
                assert_eq!(&sequence, ref_seq, "emitted sequence diverged at n={n}");
            }
        }
    }
}
