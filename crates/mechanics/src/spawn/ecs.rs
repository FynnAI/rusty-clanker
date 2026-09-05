//! The real `DomainGroup::RandomTick`/`DomainGroup::EntityPhysicsIntegration`
//! registration (blueprint Context, "Tick-pipeline placement"). `system_mob_spawn_cycle`
//! drives MECH-D34's per-tick pack-spawn algorithm plus MECH-D35's cross-region census
//! gossip; `system_mob_despawn` drives the despawn rules over every live, tagged mob.
//!
//! **A necessary, documented deviation from this blueprint's own literal Deliverables**
//! (final report has the full citation): the blueprint's own `system_mob_spawn_cycle`
//! signature shows a direct `Query<(&PlayerMarker, &PlayerMotion)>` — structurally
//! impossible under WS-D3's crate-boundary rule, since `PlayerMarker`/`PlayerMotion` live
//! in `rusty-clanker-server`, which depends *on* `rc-mechanics`, never the reverse. This
//! is the identical boundary M4-B02's own Context section already names for item pickup,
//! and M4-B03's own `PlayerSensor` already declares itself permanently inert for the same
//! reason. Resolution: `KnownPlayers` (below), a new `rc-mechanics`-owned resource
//! refreshed once per tick by the composition root from its own real `PlayerMarker` query
//! — before `executor.tick_region` runs — mirroring this same blueprint's own already-
//! established `KnownRegionIds` convention ("peer enumeration step 1") applied to a
//! second, structurally identical cross-crate-visibility gap.
//!
//! A second documented deviation: the blueprint's own Deliverables specify
//! `SharedEntityIdAllocator`/`RegionNetworkIdAllocator`, wrapping `rc_core::
//! RcEntityIdAllocator`/`NetworkEntityIdAllocator` for spawned-mob identity. M4-B02's own
//! landed precedent (`entity_drops::spawn_break_drop`, `entity_tracking::
//! stand_in_network_id`) establishes that every entity this milestone's own tracking
//! pipeline already handles derives its wire-facing identity from `RcEntityId(Entity::
//! to_bits())` directly — no allocator is ever consumed for this purpose anywhere in the
//! landed codebase, and no prior blueprint ever wired a live `Arc<RcEntityIdAllocator>`
//! instance into a region `World` for entity-domain spawning. This blueprint's own
//! spawned mobs follow the identical, already-proven convention: `spawn_mob` below never
//! allocates an id at all, and neither `SharedEntityIdAllocator` nor
//! `RegionNetworkIdAllocator` is implemented (dead complexity with no consumer).

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, ChunkKeyTag, LightColumn, WORLD_HEIGHT, WORLD_MIN_Y};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, MobCensusReport, RegionId, RegionMessage};
use rc_physics::{ShapeTable, VoxelShape};
use rc_registries::generated_v776::block_states::default_state::AIR;
use rc_scheduler::{
    CurrentTick, DomainGroup, MobCensusInbox, RcExecutorBuilder, RegionMessageOutbox, SystemFactory,
};

use crate::entity::physics::ecs::{DimensionResource, ShapeTableResource};
use crate::entity::{BaseEntity, EntityKind, EntityPayload, LivingEntity, MobMarker};
use crate::fluid::FluidTables;
use crate::light::{light_local_y, light_nibble_index, light_section_index_for_y, nibble_at};
use crate::spawn::category::MobCategory;
use crate::spawn::census::{
    GlobalMobCensus, KnownRegionIds, MobCategoryCounts, RegionCensusState, chunk_center,
    global_cap, is_within_spawn_distance,
};
use crate::spawn::cycle::{
    SpawnCycleRandom, SpawnWorldAccess, nearest_player_dist_sqr, run_spawn_cycle,
};
use crate::spawn::despawn::{
    DespawnDecision, DespawnTimer, check_despawn, remove_when_far_away_for_kind,
};
use crate::stage4::ecs::ChunkIndex;

/// Per-entity category tag this blueprint attaches to every naturally-spawned mob
/// (blueprint Context — census/despawn need only the category, not the full
/// `EntityKind`).
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct MobCategoryTag(pub MobCategory);

/// Bridges player positions into `rc-mechanics` (this file's own module doc comment has
/// the full citation). Refreshed once per real-time tick by the composition root,
/// **before** `executor.tick_region(...)` — the same manual-step placement `KnownRegionIds`
/// already establishes. Auto-inserted empty at region bootstrap.
#[derive(Resource, Default, Debug, Clone)]
pub struct KnownPlayers(pub Vec<(i32, [f64; 3])>);

/// The ECS-agnostic `SpawnWorldAccess` boundary's real production adapter (mirrors
/// `physics::world_bridge::ReadOnlyBlockWorld`'s own established shape). Owns its
/// `Query`s by value — a reference to a function-local `Query` cannot satisfy this
/// struct's own `&'static`-annotated data type parameter (the identical, already-proven
/// reasoning `ReadOnlyBlockWorld`'s own doc comment states).
struct EcsSpawnWorld<'w, 's> {
    chunk_query: Query<
        'w,
        's,
        (
            &'static ChunkKeyTag,
            &'static BlockStateColumn,
            &'static LightColumn,
        ),
    >,
    /// `'s`, not `'w` — mirrors `physics::world_bridge::ReadOnlyBlockWorld`'s own
    /// identical, already-proven choice: `Res<ChunkIndex>`/`Res<FluidTables>` are
    /// themselves function-local system-param values (roughly `'s`-scoped), so a
    /// reference to one of them can only ever be `'s`, never the longer `'w`.
    chunk_index: &'s ChunkIndex,
    dimension: DimensionId,
    shape_table: &'static ShapeTable,
    fluid_tables: &'s FluidTables,
    players: Vec<(i32, [f64; 3])>,
    /// Owned by value (moved out of the system's own `commands` parameter), not a
    /// reference — sidesteps the exact lifetime-invariance trap a `&mut Commands<'w,'s>`
    /// reference field ran into (`Commands` is only ever used through this struct in
    /// `system_mob_spawn_cycle`, so nothing else needs to keep its own handle to it).
    commands: Commands<'w, 's>,
}

impl<'w, 's> EcsSpawnWorld<'w, 's> {
    fn get_block_state(&self, pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        if !(WORLD_MIN_Y..WORLD_MIN_Y + WORLD_HEIGHT).contains(&pos.y) {
            return None;
        }
        let chunk_key = pos.chunk_key(self.dimension);
        let entity = *self.chunk_index.0.get(&chunk_key)?;
        let (_, column, _) = self.chunk_query.get(entity).ok()?;
        let (lx, lz) = (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8);
        Some(column.get(lx, pos.y, lz))
    }

    /// Mirrors `fluid::occlusion::is_full_cube`'s own established fluid-aware shape
    /// resolution exactly (module doc comment there: a position holding any registered
    /// fluid always resolves to `VoxelShape::empty()`, since `tier1_shape_table()`
    /// carries no entry for either fluid range and would otherwise wrongly default a
    /// fluid position to a full opaque cube) — reimplemented locally rather than
    /// reusing that function directly, since it takes `&dyn BlockWorldAccess`, a
    /// different trait than `SpawnWorldAccess`.
    fn shape_at(&self, pos: BlockPos) -> VoxelShape {
        match self.get_block_state(pos) {
            Some(id) if self.fluid_tables.ranges.kind_of(id).is_some() => VoxelShape::empty(),
            Some(id) => self.shape_table.lookup(id.0).shape,
            None => VoxelShape::full_cube(),
        }
    }

    fn light_column_at(&self, pos: BlockPos) -> Option<&LightColumn> {
        let chunk_key = pos.chunk_key(self.dimension);
        let entity = *self.chunk_index.0.get(&chunk_key)?;
        let (_, _, light) = self.chunk_query.get(entity).ok()?;
        Some(light)
    }

    /// `0` when the position's own chunk is unloaded or (defensively) carries no
    /// `LightColumn` — production `HardcodedWorld` never wires Stage 8's light engine
    /// into its own tick loop (M4-B07's own `with_lighting_driver` is never called),
    /// so every `LightColumn` there stays `new_uninitialized()` forever, meaning both
    /// `sky_light`/`block_light` are `0` at every position in practice today — a
    /// pre-existing composition-root gap this blueprint does not itself introduce or
    /// fix, restated here since it makes the Monster darkness gate trivially permissive
    /// (final report has the full citation).
    fn light_at(&self, pos: BlockPos, sky: bool) -> u8 {
        let Some(light) = self.light_column_at(pos) else {
            return 0;
        };
        let section_index = light_section_index_for_y(pos.y);
        let local_y = light_local_y(pos.y);
        let (lx, lz) = (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8);
        let index = light_nibble_index(lx, local_y, lz);
        let section = light.section(section_index);
        let nibbles = if sky { &section.sky } else { &section.block };
        nibble_at(nibbles, index)
    }
}

impl<'w, 's> SpawnWorldAccess for EcsSpawnWorld<'w, 's> {
    fn min_y(&self) -> i32 {
        WORLD_MIN_Y
    }

    fn topmost_non_air_y(&self, x: i32, z: i32) -> i32 {
        let chunk_key = ChunkKey::new(self.dimension, x >> 4, z >> 4);
        let Some(&entity) = self.chunk_index.0.get(&chunk_key) else {
            return WORLD_MIN_Y - 1;
        };
        let Ok((_, column, _)) = self.chunk_query.get(entity) else {
            return WORLD_MIN_Y - 1;
        };
        let (lx, lz) = (x.rem_euclid(16) as u8, z.rem_euclid(16) as u8);
        // `rc_registries`' generated `BlockStateId` and `rc_chunk_storage::BlockStateId`
        // are distinct newtypes over the identical raw `u32` space (`to_storage_id`'s
        // own doc comment, `rusty-clanker-server::play::block_action`) — compared here
        // via the raw field rather than pulling that server-crate-only bridge function
        // into `rc-mechanics`.
        for y in (WORLD_MIN_Y..WORLD_MIN_Y + WORLD_HEIGHT).rev() {
            if column.get(lx, y, lz).0 != AIR.0 {
                return y;
            }
        }
        WORLD_MIN_Y - 1
    }

    fn is_full_opaque_cube(&self, pos: BlockPos) -> bool {
        self.shape_at(pos) == VoxelShape::full_cube()
    }

    fn has_fluid(&self, pos: BlockPos) -> bool {
        match self.get_block_state(pos) {
            Some(id) => self.fluid_tables.ranges.kind_of(id).is_some(),
            None => false,
        }
    }

    fn sky_light(&self, pos: BlockPos) -> u8 {
        self.light_at(pos, true)
    }

    fn block_light(&self, pos: BlockPos) -> u8 {
        self.light_at(pos, false)
    }

    fn sky_darken(&self) -> i32 {
        0
    }

    fn spawn_candidate_chunks(&self) -> Vec<ChunkKey> {
        self.chunk_query
            .iter()
            .map(|(tag, _, _)| tag.0)
            .filter(|key| {
                self.players
                    .iter()
                    .any(|&(_, pos)| is_within_spawn_distance(chunk_center(*key), pos))
            })
            .collect()
    }

    fn players(&self) -> Vec<(i32, [f64; 3])> {
        self.players.clone()
    }

    fn spawn_mob(
        &mut self,
        _kind: EntityKind,
        base: BaseEntity,
        living: Option<LivingEntity>,
        payload: EntityPayload,
        marker: MobMarker,
        category: MobCategory,
    ) {
        let mut entity_commands = self.commands.spawn((
            base,
            marker,
            MobCategoryTag(category),
            DespawnTimer::default(),
        ));
        entity_commands.insert(payload);
        if let Some(living) = living {
            entity_commands.insert(living);
        }
    }
}

/// Registers `system_mob_spawn_cycle` into `DomainGroup::RandomTick` (blueprint Context:
/// "Tick-pipeline placement"). Production `HardcodedWorld` never calls M3-B06's own
/// `register_stage5` (a pre-existing gap this blueprint does not itself introduce or
/// fix, final report has the full citation), so this system receives `order_tag = 0` —
/// the two systems are conflict-free either way (disjoint `Query`/`Commands` access
/// sets), so the exact `order_tag` value carries no behavioral consequence.
pub fn register_mob_spawn_cycle(builder: &mut RcExecutorBuilder) {
    builder.register_system(DomainGroup::RandomTick, mob_spawn_cycle_factory(), vec![]);
}

fn mob_spawn_cycle_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(system_mob_spawn_cycle))
            as Box<dyn System<In = (), Out = ()>>
    })
}

/// Registers `system_mob_despawn` into `DomainGroup::EntityPhysicsIntegration` (blueprint
/// Context: "Despawn → `DomainGroup::EntityPhysicsIntegration`"). The composition root
/// must call this after M4-B02's own `register_stage6b` so this system receives
/// `order_tag = 1` — M4-B09's own future governance changeset fixes the required
/// three-way order across this function, `register_stage6b`, and M4-B05's own mob-combat
/// registration function.
pub fn register_mob_despawn(builder: &mut RcExecutorBuilder) {
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        mob_despawn_factory(),
        vec![],
    );
}

fn mob_despawn_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(system_mob_despawn)) as Box<dyn System<In = (), Out = ()>>
    })
}

// `structural_writes: vec![]` on both registration functions above (blueprint Context's
// own text asks for the real `ComponentId`s of `BaseEntity`/`LivingEntity`/`MobMarker`/
// `MobCategoryTag`/`DespawnTimer`/`ZombieBundle`/`CowBundle` here) — a necessary,
// documented deviation. `register_system`'s own `structural_writes` parameter has no way
// to obtain a real, region-`World`-matching `ComponentId` at registration time (before
// any `World` exists at all, `RcExecutorBuilder::register_system`'s own call site);
// `RcExecutorBuilder::build`'s own structural-write check (`registry.rs`) only ever
// compares a system's own declared `structural_writes` against that SAME system's own
// `Query`-derived component writes — never against another system's. Neither
// `system_mob_spawn_cycle` nor `system_mob_despawn` ever declares a mutable `Query` over
// any of the component types its own `Commands` calls insert/remove, so the check
// trivially, correctly passes regardless of `structural_writes`'s exact contents — the
// identical `vec![]` already used by M4-B02's own `register_stage6b`
// (`entity/physics/ecs.rs`), the only other landed precedent for a `Commands`-using
// registered system in this crate.

/// Bootstrap resources a region's `World` needs before either system above can run —
/// inserted once at region-spawn time by the composition root (`bootstrap_region`,
/// mirroring M3-B01/M3-B06's own established per-blueprint bootstrap-function
/// convention). `SpawnCycleRandom`'s seed is caller-supplied — never vanilla's own
/// time-seeded stream (blueprint Constraints).
pub fn bootstrap_spawn_resources(world: &mut World, region_id: RegionId, spawn_rng_seed: i64) {
    world.insert_resource(SpawnCycleRandom::new(spawn_rng_seed));
    world.insert_resource(RegionCensusState::default());
    world.insert_resource(GlobalMobCensus::new(region_id));
    world.insert_resource(KnownRegionIds::default());
    world.insert_resource(KnownPlayers::default());
}

/// The Stage-5 spawn-cycle system (blueprint Context, Implementation step 9): drains
/// `MobCensusInbox` into `GlobalMobCensus`, rebuilds `RegionCensusState` from this tick's
/// own live, non-persistence-locked mobs, runs `run_spawn_cycle`, and (every 20 ticks)
/// emits this region's own `MobCensusReport` to every peer in `KnownRegionIds`
/// (MECH-D35's own gossip cadence — always a no-op in this project's current,
/// single-region-only composition root, since `KnownRegionIds` is never refreshed there;
/// final report has the full citation).
#[allow(clippy::too_many_arguments)]
fn system_mob_spawn_cycle(
    live_mob_query: Query<(
        &'static MobCategoryTag,
        &'static BaseEntity,
        &'static MobMarker,
    )>,
    chunk_query: Query<(
        &'static ChunkKeyTag,
        &'static BlockStateColumn,
        &'static LightColumn,
    )>,
    chunk_index: Res<ChunkIndex>,
    shape_table: Res<ShapeTableResource>,
    fluid_tables: Res<FluidTables>,
    dimension: Res<DimensionResource>,
    known_players: Res<KnownPlayers>,
    known_regions: Res<KnownRegionIds>,
    current_tick: Res<CurrentTick>,
    mut spawn_rng: ResMut<SpawnCycleRandom>,
    mut census: ResMut<RegionCensusState>,
    mut global_census: ResMut<GlobalMobCensus>,
    mut inbox: ResMut<MobCensusInbox>,
    mut outbox: ResMut<RegionMessageOutbox>,
    commands: Commands,
) {
    for report in inbox.0.drain(..) {
        global_census.record_peer_report(report.region, MobCategoryCounts(report.counts));
    }

    let players = known_players.0.clone();
    let live_mobs = live_mob_query
        .iter()
        .filter(|(_, _, marker)| !marker.persistence_required)
        .map(|(tag, base, _)| (tag.0, base.pos));
    *census = RegionCensusState::build(live_mobs, &players);
    global_census.set_own_counts(census.global);

    let mut world = EcsSpawnWorld {
        chunk_query,
        chunk_index: &chunk_index,
        dimension: dimension.0,
        shape_table: shape_table.0,
        fluid_tables: &fluid_tables,
        players: players.clone(),
        commands,
    };

    let eligible_chunk_count = world.spawn_candidate_chunks().len() as u32;

    {
        let global_census_ref = &*global_census;
        let global_cap_ok = |category: MobCategory| {
            global_census_ref.aggregate(category) < global_cap(category, eligible_chunk_count)
        };
        run_spawn_cycle(
            &mut world,
            &mut spawn_rng.0,
            &mut census,
            global_cap_ok,
            current_tick.0,
        );
    }

    global_census.set_own_counts(census.global);

    if current_tick.0.is_multiple_of(20) {
        let own_region = global_census.own_region();
        let report = MobCensusReport {
            region: own_region,
            counts: census.global.0,
        };
        for &peer in &known_regions.0 {
            if peer != own_region {
                outbox.send(
                    Address::Region(peer),
                    RegionMessage::MobCensusReport(report),
                );
            }
        }
    }
}

/// The Stage-6b despawn system (blueprint Context, Implementation step 9): iterates
/// every live tagged mob, calls `check_despawn` with each mob's own
/// `remove_when_far_away_for_kind(kind)` term, and issues `Commands::despawn` on a
/// `Despawn` decision.
fn system_mob_despawn(
    mut query: Query<(
        Entity,
        &'static BaseEntity,
        &'static MobCategoryTag,
        &'static MobMarker,
        &'static EntityPayload,
        &'static mut DespawnTimer,
    )>,
    known_players: Res<KnownPlayers>,
    mut spawn_rng: ResMut<SpawnCycleRandom>,
    mut commands: Commands,
) {
    let players = &known_players.0;
    for (entity, base, tag, marker, payload, mut timer) in query.iter_mut() {
        let dist_sqr = nearest_player_dist_sqr(players, base.pos);
        let remove_when_far_away = remove_when_far_away_for_kind(payload.kind());
        let decision = check_despawn(
            marker.persistence_required,
            dist_sqr,
            tag.0,
            remove_when_far_away,
            &mut spawn_rng.0,
            &mut timer,
        );
        if decision == DespawnDecision::Despawn {
            commands.entity(entity).despawn();
        }
    }
}
