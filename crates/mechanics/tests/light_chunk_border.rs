//! M4-B07 — cross-chunk-same-region propagation acceptance tests (Context §5/§8/§10),
//! via `bevy_ecs::World` directly (no `RcExecutor` needed).
//!
//! Every test chunk below is built with an opaque ceiling at `y=10` (heightmap seed
//! `11`), rather than the fully open, degenerate all-air/all-sky-source column a
//! bare `HeightmapSet::new_uniform(WORLD_MIN_Y)` would give every position: Stage
//! 8's own seeding step (Context §8 step 1) unconditionally calls `check_node_sky`
//! once for *every* dirty position's own `y`, regardless of whether the sky-source
//! boundary itself moved ("vanilla's own per-position branch runs unconditionally
//! on every block change") -- with no ceiling, the `y=0` block-light fixtures below
//! would each also seed a spurious sky-light source exactly at the emitter's own
//! position, which `light_border_update_emitted_and_applied_for_cross_region_case`'s
//! own `sky.is_none()` assertion depends on never happening. A ceiling well above
//! the tested `y=0` row keeps every one of this file's own block-light assertions
//! unaffected while keeping the sky channel genuinely untouched.

use bevy_ecs::prelude::*;
use rc_chunk_storage::{
    BlockStateColumn, BlockStateId, ChunkKeyTag, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::light::stage8::{ParallelDispatch, run_stage8_lighting};
use rc_mechanics::{
    LightDirtyQueue, LightPropagatorState, LightProperties, LightPropertiesRegistry,
    SkyLightSourceColumn,
};
use rc_messaging::{Address, LightBorderUpdate, RegionId, RegionMessage};
use rc_scheduler::{LightBorderInbox, RegionMessageOutbox};

const AIR: BlockStateId = BlockStateId(0);
const EMITTER: BlockStateId = BlockStateId(1);
const CEILING: BlockStateId = BlockStateId(99);
const CEILING_Y: i32 = 10;

/// Trivial sequential `ParallelDispatch` test double -- runs every task in `Vec`
/// order, single-threaded (Test 1's own doc comment note).
struct SequentialDispatch;
impl ParallelDispatch for SequentialDispatch {
    fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>) {
        for task in tasks {
            task();
        }
    }
}

/// The shared `LightPropertiesRegistry` every test builds: `EMITTER` (block
/// emission 14) plus `CEILING` (opaque) -- used both to build each chunk's own
/// `SkyLightSourceColumn` at spawn time and as the `bevy_ecs::World`'s own runtime
/// resource, so the two stay consistent (module doc comment).
fn properties_with_emitter_and_ceiling() -> LightPropertiesRegistry {
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
    properties
}

fn ceilinged_blocks() -> BlockStateColumn {
    let mut blocks = BlockStateColumn::new(AIR, PaletteThresholds::blocks(15));
    for x in 0u8..16 {
        for z in 0u8..16 {
            blocks.set(x, CEILING_Y, z, CEILING);
        }
    }
    blocks
}

/// Every section `Filled(0)` -- a structurally "already loaded, correctly all-dark"
/// `LightColumn` (Context §9's own chunk-load trust policy), rather than
/// `new_uninitialized()`'s own "freshly generated, needs a full recompute" trigger.
/// Sidesteps Stage 8's own step-2 bulk-recompute pass entirely for this file's own
/// chunks (module doc comment): a real full-chunk sky bulk-seed on a wide-open,
/// ceiling-less neighbor chunk would otherwise defer sky-light entries back across
/// the very same region boundary these tests are pinning the *block*-light behavior
/// of, tainting assertions this file's own tests never intend to exercise.
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

fn get_light_column(world: &World, entity: Entity) -> &LightColumn {
    world.get::<LightColumn>(entity).unwrap()
}

fn stored_block_at(column: &LightColumn, world_y: i32, local_x: u8, local_z: u8) -> u8 {
    let index = rc_mechanics::light::light_section_index_for_y(world_y);
    let local_y = rc_mechanics::light::light_local_y(world_y);
    let nibble_index = rc_mechanics::light::light_nibble_index(local_x, local_y, local_z);
    rc_mechanics::light::nibble_at(&column.section(index).block, nibble_index)
}

#[test]
fn light_crosses_a_same_region_chunk_boundary() {
    let mut world = World::new();
    let properties = properties_with_emitter_and_ceiling();
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
    world.insert_resource(rc_mechanics::RegionOwnership::always_local(
        Address::Region(RegionId(1)),
    ));
    world.insert_resource(RegionMessageOutbox::default());
    world.insert_resource(LightDirtyQueue::default());
    world.insert_resource(LightBorderInbox::default());

    // A block-light emitter (emission: 14) at local (15, 0, 0) of chunk (0,0) -- world
    // x=15 -- recorded into `LightDirtyQueue`, mimicking `UpdateContext::set_block`'s
    // own seam.
    world
        .resource_mut::<LightDirtyQueue>()
        .mark(BlockPos::new(15, 0, 0), AIR, EMITTER);
    world
        .get_mut::<BlockStateColumn>(chunk_a)
        .unwrap()
        .set(15, 0, 0, EMITTER);

    let _report = run_stage8_lighting(&mut world, &SequentialDispatch);

    let column_a = get_light_column(&world, chunk_a);
    assert_eq!(stored_block_at(column_a, 0, 15, 0), 14);

    let column_b = get_light_column(&world, chunk_b);
    assert_eq!(stored_block_at(column_b, 0, 0, 0), 13);
    assert_eq!(stored_block_at(column_b, 0, 1, 0), 12);
}

#[test]
fn light_border_update_emitted_and_applied_for_cross_region_case() {
    let mut world = World::new();
    let properties = properties_with_emitter_and_ceiling();
    let chunk_a = spawn_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        &properties,
    );
    let _chunk_b = spawn_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 1, 0),
        &properties,
    );

    world.insert_resource(properties);
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

    let _report = run_stage8_lighting(&mut world, &SequentialDispatch);

    // `RegionMessageBus`'s own contents are only readable via `RegionMessageState`'s
    // established merge/drain pair (its `pending` field is private, WS-D3 rule 3's
    // "no ad hoc read-back" convention) -- mirrors `RcExecutor::tick_region`'s own
    // Stage-10 bridging step exactly.
    let bus = world.resource_mut::<RegionMessageOutbox>().take();
    let mut message_state = rc_messaging::RegionMessageState::new();
    message_state.merge(bus);
    let sent = message_state.drain_outbox(RegionId(1), 0);
    let light_updates: Vec<(Address, Box<LightBorderUpdate>)> = sent
        .into_iter()
        .filter_map(|msg| match msg.payload {
            RegionMessage::LightBorderUpdate(ev) => Some((msg.to, ev)),
            _ => None,
        })
        .collect();

    // Two messages, not one: `check_node_block`'s own seed fans out in *all six*
    // directions (Context §2), so the emitter's light also spreads vertically --
    // downward from y=0 it crosses into the light section immediately below
    // (section index 4, y=-16..-1) one hop in, in addition to the main section (5,
    // y=0..15) the emitter's own position lives in and the horizontal spread stays
    // within. Both sections border the same remote chunk on the same East face, so
    // both get their own message. The section-5 one (this test's own real subject)
    // is picked out below by its own `section_index`.
    let section_index = rc_mechanics::light::light_section_index_for_y(0) as u8;
    assert_eq!(
        light_updates.len(),
        2,
        "one LightBorderUpdate per section touched on the East face"
    );
    let (to, ev) = light_updates
        .iter()
        .find(|(_, ev)| ev.section_index == section_index)
        .expect("a LightBorderUpdate for the emitter's own light section");
    assert_eq!(*to, Address::Chunk(remote_chunk));
    assert_eq!(ev.chunk, remote_chunk);
    assert_eq!(
        ev.edge_face, 0,
        "West -- the receiving chunk's own edge facing the sender"
    );
    // `Some([0; 128])`, not `None`: `spawn_chunk`'s own `already_loaded_light_column`
    // (module doc comment) starts every section `Filled(0)` -- structurally tracked,
    // all-zero -- rather than `Uninitialized`, and `extract_face_from_nibbles`
    // materializes a `Filled(v)` section's own face regardless of `v` (Context §10
    // states no `v == 0` special case, unlike `build_update_light_payload`'s own
    // explicit empty-mask handling for the wire payload, Context §12) -- this
    // chunk's own sky channel was never touched by anything this test does, so its
    // own tracked value is uniformly zero.
    assert_eq!(ev.sky, Some([0u8; 128]));
    let block_face = ev.block.expect("block face data present");
    // Face position (local_y=0, perp=0) decodes to 14 -- the converged value at world
    // x=15 (chunk (0,0)'s own East face, extracted from the sender's own column).
    assert_eq!(
        rc_mechanics::light::get_nibble(&expand_face(&block_face), 0),
        14
    );
}

/// Materializes a 128-byte face slice's own nibble at logical index 0 (local_y=0,
/// perp=0) for direct inspection -- a tiny local helper, not a re-test of
/// `section_ops` itself (already covered by `light_bits_and_faces.rs`).
fn expand_face(face: &[u8; 128]) -> [u8; 2048] {
    let mut out = [0u8; 2048];
    out[..128].copy_from_slice(face);
    out
}

#[test]
fn inbound_light_border_update_seeds_round_zero() {
    let mut world = World::new();
    let properties = properties_with_emitter_and_ceiling();
    let chunk = spawn_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 5, 5),
        &properties,
    );

    world.insert_resource(properties);
    world.insert_resource(rc_mechanics::RegionOwnership::always_local(
        Address::Region(RegionId(1)),
    ));
    world.insert_resource(RegionMessageOutbox::default());
    world.insert_resource(LightDirtyQueue::default());

    let section_index = rc_mechanics::light::light_section_index_for_y(0) as u8;
    // Byte pattern encoding 14 at face position (local_y=0, perp=0) (local (x=0,y=0,
    // z=0)) and 0 elsewhere. Face-local index 0 is the low nibble of byte 0.
    let mut block_face = [0u8; 128];
    block_face[0] = 14; // low nibble = 14, high nibble = 0

    let inbound = LightBorderUpdate {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 5, 5),
        section_index,
        edge_face: 0, // West -- this chunk's own edge that received the data.
        sky: None,
        block: Some(block_face),
    };
    world.insert_resource(LightBorderInbox(vec![inbound]));

    let _report = run_stage8_lighting(&mut world, &SequentialDispatch);

    let column = get_light_column(&world, chunk);
    assert_eq!(stored_block_at(column, 0, 0, 0), 13);
    assert_eq!(stored_block_at(column, 0, 1, 0), 12);
}
