//! M4-B07 field-report fix ("light bounce") -- cross-chunk sky/block-light
//! convergence acceptance tests (Context §5/§8, `docs/findings-for-planning.md`'s
//! own entry on this changeset has the full defect writeup). Unlike
//! `light_chunk_border.rs`/`light_determinism.rs`, the sky-light convergence test
//! below spawns its chunks with a genuinely fresh `LightColumn::new_uninitialized()`
//! (never the `already_loaded_light_column()` bypass those two files use)
//! specifically to exercise Stage 8's own step-2 bulk full-chunk recompute -- the
//! seeding path the diagnosed defect actually manifests on (a wide-open superflat
//! world, both sides of a chunk border independently seeded as sky sources at level
//! 15, mirroring each other's own cross-chunk deferral forever instead of
//! converging).

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
use rc_messaging::{Address, RegionId};
use rc_scheduler::{LightBorderInbox, RegionMessageOutbox};

const AIR: BlockStateId = BlockStateId(0);

/// Trivial sequential `ParallelDispatch` test double -- runs every task in `Vec`
/// order, single-threaded (mirrors every other light acceptance test file's own
/// identical double).
struct SequentialDispatch;
impl ParallelDispatch for SequentialDispatch {
    fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>) {
        for task in tasks {
            task();
        }
    }
}

fn stored_sky_at(column: &LightColumn, world_y: i32, local_x: u8, local_z: u8) -> u8 {
    let index = rc_mechanics::light::light_section_index_for_y(world_y);
    let local_y = rc_mechanics::light::light_local_y(world_y);
    let nibble_index = rc_mechanics::light::light_nibble_index(local_x, local_y, local_z);
    rc_mechanics::light::nibble_at(&column.section(index).sky, nibble_index)
}

fn stored_block_at(column: &LightColumn, world_y: i32, local_x: u8, local_z: u8) -> u8 {
    let index = rc_mechanics::light::light_section_index_for_y(world_y);
    let local_y = rc_mechanics::light::light_local_y(world_y);
    let nibble_index = rc_mechanics::light::light_nibble_index(local_x, local_y, local_z);
    rc_mechanics::light::nibble_at(&column.section(index).block, nibble_index)
}

fn get_light_column(world: &World, entity: Entity) -> &LightColumn {
    world.get::<LightColumn>(entity).unwrap()
}

fn is_idle(world: &World, entity: Entity) -> bool {
    world.get::<LightPropagatorState>(entity).unwrap().is_idle()
}

/// Every resource `run_stage8_lighting` needs beyond the properties registry
/// itself (which each test builds and inserts separately, since its content
/// varies per test) -- a same-region, single-region world with empty in/outboxes.
fn insert_common_resources(world: &mut World, properties: LightPropertiesRegistry) {
    world.insert_resource(properties);
    world.insert_resource(rc_mechanics::RegionOwnership::always_local(
        Address::Region(RegionId(1)),
    ));
    world.insert_resource(RegionMessageOutbox::default());
    world.insert_resource(LightDirtyQueue::default());
    world.insert_resource(LightBorderInbox::default());
}

// ---------------------------------------------------------------------------------------
// Test 1: a genuine open-sky superflat world (fresh `LightColumn`s, real step-2 bulk
// recompute -- the seeding path the diagnosed defect manifests on).
// ---------------------------------------------------------------------------------------

const FLOOR: BlockStateId = BlockStateId(1);
const FLOOR_Y: i32 = 300;
/// An arbitrary Y comfortably inside the open-sky span (`FLOOR_Y + 1 ..
/// WORLD_MIN_Y + WORLD_HEIGHT`) used for spot-checking real converged values.
const OPEN_SKY_SPOT_Y: i32 = 310;

fn open_sky_properties() -> LightPropertiesRegistry {
    let mut properties = LightPropertiesRegistry::new();
    properties.register_one(FLOOR, LightProperties::OPAQUE);
    properties
}

fn floor_blocks() -> BlockStateColumn {
    let mut blocks = BlockStateColumn::new(AIR, PaletteThresholds::blocks(15));
    for x in 0u8..16 {
        for z in 0u8..16 {
            blocks.set(x, FLOOR_Y, z, FLOOR);
        }
    }
    blocks
}

/// Spawns one fresh superflat chunk: an opaque floor at `FLOOR_Y` across the full
/// 16x16 column, open sky above it up to the world's own ceiling, and a genuinely
/// `new_uninitialized()` `LightColumn` -- triggers Stage 8's own step-2 bulk
/// full-chunk recompute (the seeding path the diagnosed defect manifests on).
fn spawn_open_chunk(
    world: &mut World,
    key: ChunkKey,
    properties: &LightPropertiesRegistry,
) -> Entity {
    let blocks = floor_blocks();
    let heightmap = HeightmapSet::new_uniform(FLOOR_Y + 1);
    let sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, properties);
    world
        .spawn((
            ChunkKeyTag(key),
            blocks,
            LightColumn::new_uninitialized(),
            sky_sources,
            heightmap,
            LightPropagatorState::new(),
        ))
        .id()
}

#[test]
fn sky_light_converges_across_a_fresh_chunk_boundary_and_stays_idle() {
    let mut world = World::new();
    let properties = open_sky_properties();
    let chunk_a = spawn_open_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        &properties,
    );
    let chunk_b = spawn_open_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 1, 0),
        &properties,
    );
    insert_common_resources(&mut world, properties);

    let report = run_stage8_lighting(&mut world, &SequentialDispatch);

    assert!(
        report.converged,
        "Stage 8 must reach a genuine fixed point, not exhaust its 16-round budget \
         (the diagnosed defect: mirrored border sky-decrease entries bouncing forever)"
    );
    assert!(
        is_idle(&world, chunk_a),
        "chunk A must have no pending queue entries"
    );
    assert!(
        is_idle(&world, chunk_b),
        "chunk B must have no pending queue entries"
    );

    // Spot-check real values: both chunks' own border cells sit in genuinely open
    // sky -- both should read the sky-light ceiling, 15, undisturbed by the fix.
    let column_a = get_light_column(&world, chunk_a);
    let column_b = get_light_column(&world, chunk_b);
    assert_eq!(stored_sky_at(column_a, OPEN_SKY_SPOT_Y, 15, 0), 15);
    assert_eq!(stored_sky_at(column_b, OPEN_SKY_SPOT_Y, 0, 0), 15);

    // A second invocation with nothing new to seed must be a genuine no-op.
    let report2 = run_stage8_lighting(&mut world, &SequentialDispatch);
    assert_eq!(report2.chunks_touched, 0);
    assert_eq!(report2.rounds_run, 0);
}

// ---------------------------------------------------------------------------------------
// Tests 2/3/4: block-light convergence across a chunk boundary. Mirrors
// `light_chunk_border.rs`'s own ceiling trick (a ceiling well above the tested y=0 row
// keeps Stage 8's own unconditional per-dirty-position `check_node_sky` call from
// seeding a spurious sky-light source at the emitter's own position) and its own
// `already_loaded_light_column` bypass of Stage 8's step-2 bulk sky recompute, isolating
// these three tests to block light alone -- which needs no per-column heightmap/boundary
// bookkeeping at all, so an opaque block's own occlusion is exercised directly, without
// the sky channel's own step-2 bulk-recompute optimization (whose own doc comment
// already scopes out "a neighbouring column's own obstruction reaching higher than this
// one's" as a *separate*, pre-existing, already-accepted limitation, unrelated to the
// cross-chunk light-bounce defect this changeset fixes, and which a sky-based version of
// test 2 below would otherwise confound with the fix actually under test).
// ---------------------------------------------------------------------------------------

const EMITTER: BlockStateId = BlockStateId(3);
const CEILING: BlockStateId = BlockStateId(4);
const WALL: BlockStateId = BlockStateId(5);
const CEILING_Y: i32 = 10;

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
    properties.register_one(WALL, LightProperties::OPAQUE);
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

fn already_loaded_light_column() -> LightColumn {
    let mut column = LightColumn::new_uninitialized();
    for i in 0..rc_chunk_storage::LIGHT_SECTION_COUNT {
        let section = column.section_mut(i);
        section.sky = rc_chunk_storage::LightNibbles::Filled(0);
        section.block = rc_chunk_storage::LightNibbles::Filled(0);
    }
    column
}

fn spawn_ceilinged_chunk(
    world: &mut World,
    key: ChunkKey,
    properties: &LightPropertiesRegistry,
) -> Entity {
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

#[test]
fn opaque_block_on_chunk_border_matches_intra_chunk_result() {
    // Two-chunk case: the emitter sits directly on chunk A's own border cell (local
    // x=15, mirroring `light_chunk_border.rs`'s own existing emitter-on-the-border
    // test), and an opaque `WALL` sits exactly at chunk B's own border cell (local
    // x=0) -- the exact position the cross-chunk deferral writes into. Bug (b)'s own
    // diagnosed shape ("an increase arrival with `increase_from_emission: true`
    // writes `from_level` into the neighbour cell without the receiver's opacity or
    // `shape_occludes` check") applies identically to the block channel, and reaches
    // *this* cell specifically -- unlike the sky channel, which needs no per-column
    // heightmap/boundary bookkeeping at all, so this cleanly isolates the fix from
    // the unrelated, pre-existing sky-side optimization caveat noted above.
    let (column_a, column_b) = {
        let mut world = World::new();
        let properties = properties_with_emitter_and_ceiling();
        let chunk_a = spawn_ceilinged_chunk(
            &mut world,
            ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
            &properties,
        );
        let chunk_b = spawn_ceilinged_chunk(
            &mut world,
            ChunkKey::new(DimensionId::OVERWORLD, 1, 0),
            &properties,
        );
        world
            .get_mut::<BlockStateColumn>(chunk_b)
            .unwrap()
            .set(0, 0, 0, WALL);
        insert_common_resources(&mut world, properties);

        world
            .resource_mut::<LightDirtyQueue>()
            .mark(BlockPos::new(15, 0, 0), AIR, EMITTER);
        world
            .get_mut::<BlockStateColumn>(chunk_a)
            .unwrap()
            .set(15, 0, 0, EMITTER);

        let report = run_stage8_lighting(&mut world, &SequentialDispatch);
        assert!(report.converged);
        assert!(is_idle(&world, chunk_a));
        assert!(is_idle(&world, chunk_b));

        (
            get_light_column(&world, chunk_a).clone(),
            get_light_column(&world, chunk_b).clone(),
        )
    };

    // Single-chunk case: the identical local geometry (emitter, wall one cell east
    // of it), entirely inside one chunk -- the reference this test's whole point is
    // to match bit-for-bit.
    let column_single = {
        let mut world = World::new();
        let properties = properties_with_emitter_and_ceiling();
        let chunk = spawn_ceilinged_chunk(
            &mut world,
            ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
            &properties,
        );
        world
            .get_mut::<BlockStateColumn>(chunk)
            .unwrap()
            .set(8, 0, 0, WALL);
        insert_common_resources(&mut world, properties);

        world
            .resource_mut::<LightDirtyQueue>()
            .mark(BlockPos::new(7, 0, 0), AIR, EMITTER);
        world
            .get_mut::<BlockStateColumn>(chunk)
            .unwrap()
            .set(7, 0, 0, EMITTER);

        let report = run_stage8_lighting(&mut world, &SequentialDispatch);
        assert!(report.converged);
        assert!(is_idle(&world, chunk));

        get_light_column(&world, chunk).clone()
    };

    // The wall's own interior: block light 0, in both the cross-chunk and the
    // intra-chunk case -- the bug this test pins wrote a stale, un-occluded value
    // (13) directly into this exact cell instead, letting light pass straight
    // through as if the wall were transparent.
    assert_eq!(
        stored_block_at(&column_b, 0, 0, 0),
        0,
        "cross-chunk: wall interior"
    );
    assert_eq!(
        stored_block_at(&column_single, 0, 8, 0),
        0,
        "intra-chunk: wall interior"
    );

    // The emitter's own exposed face still lights the air beside it per the normal
    // rules -- undisturbed by the wall one cell further east.
    assert_eq!(
        stored_block_at(&column_a, 0, 15, 0),
        14,
        "cross-chunk: emitter itself"
    );
    assert_eq!(
        stored_block_at(&column_single, 0, 7, 0),
        14,
        "intra-chunk: emitter itself"
    );

    // One cell past the wall: unreachable straight through it, but still lit via
    // the light that diffracts around the single-block obstruction -- must
    // converge to the identical value whether or not that diffusion crosses a
    // chunk boundary on its way there.
    let past_wall_cross_chunk = stored_block_at(&column_b, 0, 1, 0);
    let past_wall_intra_chunk = stored_block_at(&column_single, 0, 9, 0);
    assert_eq!(
        past_wall_cross_chunk, past_wall_intra_chunk,
        "cross-chunk and intra-chunk propagation must reach the identical value \
         just past the wall"
    );
}

#[test]
fn block_light_emitter_one_cell_from_border_matches_intra_chunk_result() {
    // Cross-chunk case: the emitter sits one cell *west* of chunk A's own border
    // cell (local x=14, not x=15 -- `light_chunk_border.rs`'s own existing test
    // already covers the emitter-directly-on-the-border case) -- propagation must
    // take one local hop before it even reaches the boundary, then continue for a
    // further two hops once inside chunk B.
    let mut world = World::new();
    let properties = properties_with_emitter_and_ceiling();
    let chunk_a = spawn_ceilinged_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        &properties,
    );
    let chunk_b = spawn_ceilinged_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 1, 0),
        &properties,
    );
    insert_common_resources(&mut world, properties);

    world
        .resource_mut::<LightDirtyQueue>()
        .mark(BlockPos::new(14, 0, 0), AIR, EMITTER);
    world
        .get_mut::<BlockStateColumn>(chunk_a)
        .unwrap()
        .set(14, 0, 0, EMITTER);

    let report = run_stage8_lighting(&mut world, &SequentialDispatch);
    assert!(report.converged);
    assert!(is_idle(&world, chunk_a));
    assert!(is_idle(&world, chunk_b));

    let column_a = get_light_column(&world, chunk_a);
    let column_b = get_light_column(&world, chunk_b);
    assert_eq!(stored_block_at(column_a, 0, 14, 0), 14, "emitter itself");
    assert_eq!(
        stored_block_at(column_a, 0, 15, 0),
        13,
        "one hop east, still chunk A"
    );
    assert_eq!(
        stored_block_at(column_b, 0, 0, 0),
        12,
        "two hops, crossed the border"
    );
    assert_eq!(stored_block_at(column_b, 0, 1, 0), 11, "three hops");

    // Intra-chunk reference: the identical relative pattern, entirely inside one
    // chunk -- must reach the same four values at the same relative offsets.
    let mut world2 = World::new();
    let properties2 = properties_with_emitter_and_ceiling();
    let chunk = spawn_ceilinged_chunk(
        &mut world2,
        ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        &properties2,
    );
    insert_common_resources(&mut world2, properties2);
    world2
        .resource_mut::<LightDirtyQueue>()
        .mark(BlockPos::new(7, 0, 0), AIR, EMITTER);
    world2
        .get_mut::<BlockStateColumn>(chunk)
        .unwrap()
        .set(7, 0, 0, EMITTER);

    let report2 = run_stage8_lighting(&mut world2, &SequentialDispatch);
    assert!(report2.converged);

    let column = get_light_column(&world2, chunk);
    assert_eq!(
        stored_block_at(column, 0, 7, 0),
        stored_block_at(column_a, 0, 14, 0)
    );
    assert_eq!(
        stored_block_at(column, 0, 8, 0),
        stored_block_at(column_a, 0, 15, 0)
    );
    assert_eq!(
        stored_block_at(column, 0, 9, 0),
        stored_block_at(column_b, 0, 0, 0)
    );
    assert_eq!(
        stored_block_at(column, 0, 10, 0),
        stored_block_at(column_b, 0, 1, 0)
    );
}

#[test]
fn removing_the_emitter_clears_the_neighbour_chunk_and_the_state_goes_idle() {
    let mut world = World::new();
    let properties = properties_with_emitter_and_ceiling();
    let chunk_a = spawn_ceilinged_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        &properties,
    );
    let chunk_b = spawn_ceilinged_chunk(
        &mut world,
        ChunkKey::new(DimensionId::OVERWORLD, 1, 0),
        &properties,
    );
    insert_common_resources(&mut world, properties);

    world
        .resource_mut::<LightDirtyQueue>()
        .mark(BlockPos::new(14, 0, 0), AIR, EMITTER);
    world
        .get_mut::<BlockStateColumn>(chunk_a)
        .unwrap()
        .set(14, 0, 0, EMITTER);

    let report = run_stage8_lighting(&mut world, &SequentialDispatch);
    assert!(report.converged);
    {
        let column_b = get_light_column(&world, chunk_b);
        assert_eq!(
            stored_block_at(column_b, 0, 0, 0),
            12,
            "sanity: light reached chunk B first"
        );
    }

    // Remove the emitter -- a decrease must now propagate across the same border,
    // clearing the previously-lit cells back to 0.
    world
        .resource_mut::<LightDirtyQueue>()
        .mark(BlockPos::new(14, 0, 0), EMITTER, AIR);
    world
        .get_mut::<BlockStateColumn>(chunk_a)
        .unwrap()
        .set(14, 0, 0, AIR);

    let report2 = run_stage8_lighting(&mut world, &SequentialDispatch);
    assert!(
        report2.converged,
        "the decrease must reach a genuine fixed point, not exhaust its round \
         budget (the diagnosed defect's own decrease-side counterpart: mirrored \
         border decrease entries bouncing forever)"
    );

    let column_a = get_light_column(&world, chunk_a);
    let column_b = get_light_column(&world, chunk_b);
    assert_eq!(stored_block_at(column_a, 0, 14, 0), 0);
    assert_eq!(stored_block_at(column_a, 0, 15, 0), 0);
    assert_eq!(stored_block_at(column_b, 0, 0, 0), 0);
    assert_eq!(stored_block_at(column_b, 0, 1, 0), 0);

    assert!(is_idle(&world, chunk_a));
    assert!(is_idle(&world, chunk_b));
}
