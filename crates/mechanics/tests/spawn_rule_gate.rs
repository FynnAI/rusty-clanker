//! M4-B04 field-report test-authoring: the `spawn_mobs` game-rule gate. Proves, at the
//! real ECS-system level (`register_mob_spawn_cycle`/`system_mob_spawn_cycle` — the gate
//! lives one level above the pure `run_spawn_cycle` function `spawn_cycle.rs` exercises
//! directly, mirroring the pinned reference's own `ServerChunkCache.tickChunks`/
//! `NaturalSpawner.spawnForChunk` split, `crate::game_rules::GameRules`'s own doc comment
//! has the full citation), that setting `GameRules.spawn_mobs = false` suppresses every
//! natural spawn attempt on a world that demonstrably spawns when the rule is left at its
//! vanilla-default `true`.
//!
//! Reuses `spawn_cycle.rs`'s own established "single legal anchor Y, zero light"
//! permissive-world trick (that file's own module doc comment) — a real
//! `BlockStateColumn`/`LightColumn`-backed chunk (rather than a hand-rolled
//! `SpawnWorldAccess` double) is needed here, since the gate under test is wired into
//! `system_mob_spawn_cycle` itself, one level above the pure function `spawn_cycle.rs`
//! tests. Every chunk fills exactly one Y layer (`FLOOR_Y`) solid and leaves every other Y
//! open air, so `topmost_non_air_y` returns `FLOOR_Y` everywhere in the chunk and only
//! `FLOOR_Y + 1` is a legal `SpawnPlacementType::OnGround` anchor — deterministic, no
//! search-for-a-lucky-seed needed. Every `LightColumn` is `new_uninitialized()` (both
//! `sky_light`/`block_light` read `0` everywhere), which `crate::spawn::ecs`'s own module
//! doc comment already notes "makes the Monster darkness gate trivially permissive" — the
//! reason this file targets `MobCategory::Monster` (Zombie) rather than `Creature`, whose
//! own light-sufficiency rule (`is_animal_light_ok`) would never pass under zero light.
//! Six candidate chunks in a row give the Monster category a nonzero global cap
//! (`census.rs`'s own `global_cap(Monster, 6) == 70 * 6 / 289 == 1`, the smallest chunk
//! count for which that integer division is nonzero) — Monster is also not a "persistent"
//! category (`category.rs`'s own `is_persistent`), so its spawn attempt runs every tick,
//! not only on the `tick % 400 == 0` cadence `Creature` needs.
#![cfg(feature = "server-systems")]

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, BlockStateId, ChunkKeyTag, LightColumn, PaletteThresholds};
use rc_core::{ChunkKey, DimensionId};
use rc_mechanics::entity::physics::ecs::{DimensionResource, ShapeTableResource};
use rc_mechanics::entity::{BaseEntity, EntityPayload, LivingEntity, MobMarker};
use rc_mechanics::fluid::{FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks};
use rc_mechanics::game_rules::GameRules;
use rc_mechanics::spawn::{
    DespawnTimer, KnownPlayers, MobCategoryTag, bootstrap_spawn_resources, register_mob_spawn_cycle,
};
use rc_mechanics::stage4::ecs::ChunkIndex;
use rc_messaging::{Message, RegionId, RegionMessage, Transport, TransportError};
use rc_registries::generated_v776::block_states::default_state;
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{RcExecutor, RcExecutorBuilder, RegionState};

const REGION: RegionId = RegionId(1);
/// The sole solid Y layer in every test chunk below — matches `spawn_cycle.rs`'s own
/// `FLOOR_Y` constant and reasoning exactly (this file's own module doc comment).
const FLOOR_Y: i32 = -61;

/// A no-op `Transport` — `RcExecutor::tick_region` requires one, but this single-region
/// fixture never crosses a region boundary (mirrors `world_bounds_fan_out.rs`'s own
/// identical `MockTransport`).
struct MockTransport;
impl Transport for MockTransport {
    fn send(&self, _msg: Message<RegionMessage>) -> Result<(), TransportError> {
        Ok(())
    }
    fn try_recv(&self, _into: RegionId) -> Option<Message<RegionMessage>> {
        None
    }
}

/// `RcExecutorBuilder::new` requires a plain `fn(&mut World)` pointer (no captures) —
/// registers every component `system_mob_spawn_cycle`'s own queries/`Commands` calls
/// touch (`ecs.rs`'s own `EcsSpawnWorld::spawn_mob`/`chunk_query` field list), mirroring
/// `mob_region_transfer_integration.rs`'s own identical bootstrap-fn convention.
fn bootstrap(world: &mut World) {
    world.register_component::<BaseEntity>();
    world.register_component::<LivingEntity>();
    world.register_component::<EntityPayload>();
    world.register_component::<MobMarker>();
    world.register_component::<MobCategoryTag>();
    world.register_component::<DespawnTimer>();
    world.register_component::<ChunkKeyTag>();
    world.register_component::<BlockStateColumn>();
    world.register_component::<LightColumn>();
}

/// A `FluidTables` with fabricated, deliberately-out-of-range water/lava id ranges
/// (`500_000..500_016`/`500_100..500_116` — real generated block states top out at
/// `BLOCK_STATE_COUNT == 32_366`) — fluids play no role in this suite, so any two
/// disjoint, exactly-16-wide ranges that never collide with `AIR`/`STONE` satisfy
/// `EcsSpawnWorld::shape_at`/`has_fluid`'s own `fluid_tables.ranges.kind_of(id)` calls.
fn fluidless_tables() -> FluidTables {
    let ranges = FluidBlockRanges::new(
        (BlockStateId(500_000), BlockStateId(500_016)),
        (BlockStateId(500_100), BlockStateId(500_116)),
    )
    .expect("both fabricated ranges are exactly 16 wide");
    FluidTables::new(
        ranges,
        ReactionBlocks {
            obsidian: BlockStateId(0),
            cobblestone: BlockStateId(0),
            stone: BlockStateId(0),
            basalt_conversion: None,
        },
        FluidDimensionProfile { fast_lava: false },
        BlockStateId(default_state::AIR.0),
    )
}

/// Builds a single-region fixture with `register_mob_spawn_cycle` wired in, `GameRules.
/// spawn_mobs` set to `spawn_mobs`, one player near six candidate chunks, and every
/// chunk's own `FLOOR_Y`-only permissive ground (this file's own module doc comment has
/// the full setup rationale).
fn build_region(spawn_mobs: bool) -> (RcExecutor, RegionState) {
    let mut builder = RcExecutorBuilder::new(bootstrap);
    register_mob_spawn_cycle(&mut builder);
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(REGION);

    // `SpawnCycleRandom`/`RegionCensusState`/`GlobalMobCensus`/`KnownRegionIds`/
    // `KnownPlayers` (the last overwritten with a real player just below) — mirrors
    // `crates/server/src/play/world.rs`'s own `bootstrap_region` precedent exactly.
    bootstrap_spawn_resources(&mut region.world, REGION, 42);
    region.world.insert_resource(ChunkIndex::default());
    region
        .world
        .insert_resource(ShapeTableResource(rc_physics::tier1_shape_table()));
    region
        .world
        .insert_resource(DimensionResource(DimensionId::OVERWORLD));
    region.world.insert_resource(fluidless_tables());
    region.world.insert_resource(GameRules {
        spawn_mobs,
        ..GameRules::default()
    });
    // Chunk (0,0)'s own center is (8.0, _, 8.0) -- close enough that its own candidate
    // anchor positions fall inside the pack-spawn algorithm's 24-block player-exclusion
    // radius, but chunks (2,0)..=(5,0)'s own candidates (32..88 blocks away) sit outside
    // that radius while every one of the six chunk centers (8..88) stays within
    // `SPAWN_DISTANCE_BLOCKS` (128) for candidacy.
    region
        .world
        .insert_resource(KnownPlayers(vec![(1, [8.0, (FLOOR_Y + 1) as f64, 8.0])]));

    let air = BlockStateId(default_state::AIR.0);
    let solid = BlockStateId(default_state::STONE.0);
    for x in 0..6i32 {
        let chunk = ChunkKey::new(DimensionId::OVERWORLD, x, 0);
        let mut column = BlockStateColumn::new(air, PaletteThresholds::blocks(8));
        for lx in 0..16u8 {
            for lz in 0..16u8 {
                column.set(lx, FLOOR_Y, lz, solid);
            }
        }
        let entity = region
            .world
            .spawn((ChunkKeyTag(chunk), column, LightColumn::new_uninitialized()))
            .id();
        region
            .world
            .resource_mut::<ChunkIndex>()
            .0
            .insert(chunk, entity);
    }

    (executor, region)
}

fn count_naturally_spawned_mobs(world: &mut World) -> usize {
    world.query::<&MobCategoryTag>().iter(world).count()
}

fn run_ticks(executor: &RcExecutor, region: &mut RegionState, ticks: u32) {
    let transport = MockTransport;
    let pool = RcWorkerPool::new(1);
    for _ in 0..ticks {
        executor.tick_region(region, &pool, &transport);
    }
}

/// Positive control: proves the fixture itself is genuinely permissive (`GameRules.
/// spawn_mobs` left at its vanilla-default `true`) before the sibling test below relies
/// on "this same world would otherwise spawn."
#[test]
fn spawn_mobs_true_produces_at_least_one_spawn_on_a_permissive_world() {
    let (executor, mut region) = build_region(true);
    run_ticks(&executor, &mut region, 300);
    assert!(
        count_naturally_spawned_mobs(&mut region.world) > 0,
        "expected at least one natural spawn within 300 ticks on a permissive world with \
         GameRules.spawn_mobs left at its vanilla-default true"
    );
}

#[test]
fn spawn_mobs_false_produces_zero_spawns_on_the_identical_permissive_world() {
    let (executor, mut region) = build_region(false);
    run_ticks(&executor, &mut region, 300);
    assert_eq!(
        count_naturally_spawned_mobs(&mut region.world),
        0,
        "GameRules.spawn_mobs = false must suppress every natural spawn attempt, even on \
         a world the sibling test proves would otherwise spawn"
    );
}
