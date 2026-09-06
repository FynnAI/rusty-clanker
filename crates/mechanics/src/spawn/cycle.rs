//! The natural per-tick spawn-cycle algorithm — blueprint Context, "The natural per-tick
//! spawn-cycle algorithm", restated field-precise from `docs/research/mc-26.2/
//! 23-spawning-math.md` §3.2 and cross-checked against `M4-B04-CLAIMS.md`.

use rc_core::{BlockPos, ChunkKey};

use crate::entity::{
    AiSystemKind, BaseEntity, CowBundle, EntityKind, EntityPayload, EntityUuid, LivingEntity,
    MobMarker, Pose, ZombieBundle,
};
use crate::random::RcRandom;
use crate::spawn::category::MobCategory;
use crate::spawn::census::RegionCensusState;
use crate::spawn::placement::{is_animal_light_ok, is_dark_enough_to_spawn, is_on_ground_legal};
use crate::spawn::tables::{SpawnerEntry, default_max_health, pick_weighted, spawn_list};

/// A candidate spawn position is rejected if the nearest player is within this many
/// blocks (M4-B04-CLAIMS.md row 36) — a full 3D distance, unlike the chunk-candidate/
/// local-cap rule's own horizontal-only 128-block check (`census::SPAWN_DISTANCE_BLOCKS`).
const MIN_SPAWN_DISTANCE_TO_PLAYER: f64 = 24.0;

/// Vanilla's `MAX_SPAWN_CLUSTER_SIZE` (M4-B04-CLAIMS.md row 40).
const MAX_SPAWN_CLUSTER_SIZE: u32 = 4;

/// This region's own persistent, engine-seeded spawn RNG (blueprint Context,
/// "`RcRandom`, reused unmodified..."). Never reseeded after bootstrap.
#[derive(bevy_ecs::prelude::Resource)]
pub struct SpawnCycleRandom(pub RcRandom);

impl SpawnCycleRandom {
    /// `seed`: an explicit engine seed supplied by the composition root at region
    /// bootstrap — never vanilla's own time-seeded stream (blueprint Context).
    pub fn new(seed: i64) -> Self {
        Self(RcRandom::new(seed))
    }
}

/// The ECS-agnostic core boundary (mirrors `rc-physics`'s and M3-B01's
/// `BlockWorldAccess`'s own already-established "plain data in/out, no `World`
/// reference crosses this boundary" shape). A production adapter (`ecs.rs`) implements
/// this over a real region `World`; acceptance tests use a small in-memory test double.
pub trait SpawnWorldAccess {
    fn min_y(&self) -> i32;
    /// This project's own simplified `WORLD_SURFACE`-equivalent probe (blueprint
    /// Context) — a direct per-column query, not a maintained heightmap structure;
    /// cheap and exact for the superflat world M4 ships.
    fn topmost_non_air_y(&self, x: i32, z: i32) -> i32;
    fn is_full_opaque_cube(&self, pos: BlockPos) -> bool;
    fn has_fluid(&self, pos: BlockPos) -> bool;
    fn sky_light(&self, pos: BlockPos) -> u8;
    fn block_light(&self, pos: BlockPos) -> u8;
    /// The level's own day/night sky-light darkening (blueprint Context, "Monster
    /// darkness gate"). Every implementation at this blueprint's own current scope
    /// returns `0` — no day/night cycle exists yet in this project.
    fn sky_darken(&self) -> i32;
    /// Every currently-loaded chunk with at least one player within 128 blocks of the
    /// chunk's own center — vanilla's own exact, definitive spawn-eligibility predicate,
    /// restated verbatim (M4-B04-CLAIMS.md row 49). This blueprint's own definition of
    /// both "spawn candidate chunk" and `spawnable_chunk_count`.
    fn spawn_candidate_chunks(&self) -> Vec<ChunkKey>;
    /// `(network_entity_id, position)` for every connected player in this region.
    fn players(&self) -> Vec<(i32, [f64; 3])>;
    /// Performs the real `bevy_ecs::Commands::spawn` (adapter-side only — the pure
    /// algorithm never touches `World`/`Commands` directly).
    fn spawn_mob(
        &mut self,
        kind: EntityKind,
        base: BaseEntity,
        living: Option<LivingEntity>,
        payload: EntityPayload,
        marker: MobMarker,
        category: MobCategory,
    );
}

/// Diagnostic/test-observable record of one successful spawn.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnedMob {
    pub kind: EntityKind,
    pub category: MobCategory,
    pub pos: [f64; 3],
    pub yaw: f32,
}

/// Full 3D nearest-player squared distance (M4-B04-CLAIMS.md row 36: vanilla's own
/// `nearestPlayer.distanceToSqr(xx, yStart, zz)`) — distinct from the chunk-candidate/
/// local-cap rule's own horizontal-only `census::is_within_spawn_distance`. `None` when
/// no player is loaded at all. `pub(crate)` — reused by `ecs.rs`'s own despawn system for
/// the identical full-3D nearest-player distance `check_despawn` needs.
pub(crate) fn nearest_player_dist_sqr(players: &[(i32, [f64; 3])], pos: [f64; 3]) -> Option<f64> {
    players
        .iter()
        .map(|&(_, p)| {
            let dx = p[0] - pos[0];
            let dy = p[1] - pos[1];
            let dz = p[2] - pos[2];
            dx * dx + dy * dy + dz * dz
        })
        .fold(None, |acc, d| Some(acc.map_or(d, |a: f64| a.min(d))))
}

/// Vanilla's own backward Fisher-Yates shuffle (M4-B04-CLAIMS.md row 30): `len - 1`
/// calls to `next_int_bounded`, always fully consumed.
fn fisher_yates_shuffle(chunks: &mut [ChunkKey], rng: &mut RcRandom) {
    let mut i = chunks.len();
    while i > 1 {
        let j = rng.next_int_bounded(i as i32) as usize;
        chunks.swap(i - 1, j);
        i -= 1;
    }
}

/// `finalizeSpawn`'s individuality-bonus RNG cost (M4-B04-CLAIMS.md row 7/39): a
/// triangle(mean=0.0, spread=0.11485000000000001) sample (2 calls) plus a left-handed
/// roll (1 call) — 3 calls total, always paid. The return value is computed and then
/// discarded by the caller (blueprint Scope boundary — no `Attribute` component exists
/// yet to store it in).
fn individuality_bonus(rng: &mut RcRandom) -> f64 {
    let a = rng.next_double();
    let b = rng.next_double();
    let _left_handed = rng.next_float() < 0.05;
    0.11485000000000001 * (a - b)
}

/// `is_spawn_position_legal` (blueprint Context) — the darkness gate for `Monster` (fed
/// `world.sky_darken()`), the light≥9 rule for `Creature`, restated as a single dispatch
/// point since this blueprint ships only these two categories with real content.
fn is_spawn_position_legal(
    world: &dyn SpawnWorldAccess,
    rng: &mut RcRandom,
    category: MobCategory,
    pos: BlockPos,
) -> bool {
    if !is_on_ground_legal(world, pos) {
        return false;
    }
    if category == MobCategory::Monster {
        is_dark_enough_to_spawn(
            rng,
            world.sky_light(pos),
            world.block_light(pos),
            world.sky_darken(),
        )
    } else {
        is_animal_light_ok(world.sky_light(pos), world.block_light(pos))
    }
}

/// The one point where the pure algorithm and the `SpawnWorldAccess` boundary meet
/// (blueprint Context). `MobMarker.can_pick_up_loot` is conservatively `false` — the
/// real per-species zombie roll (`0.55 * difficulty.getSpecialMultiplier()`) is
/// deferred, no pickup system consumes it yet (MECH-D51).
fn build_and_spawn(
    world: &mut dyn SpawnWorldAccess,
    kind: EntityKind,
    category: MobCategory,
    pos: BlockPos,
    yaw: f32,
) -> SpawnedMob {
    let spawn_pos = [pos.x as f64 + 0.5, pos.y as f64, pos.z as f64 + 0.5];
    let base = BaseEntity {
        pos: spawn_pos,
        velocity: [0.0; 3],
        rotation: [yaw, 0.0],
        fall_distance: 0.0,
        fire_ticks: -1,
        status_flags: 0,
        air_ticks: 300,
        on_ground: false,
        invulnerable: false,
        portal_cooldown: 0,
        uuid: EntityUuid::new_random(),
        custom_name: None,
        custom_name_visible: false,
        silent: false,
        no_gravity: false,
        glowing: false,
        pose: Pose::Standing,
        ticks_frozen: 0,
        has_visual_fire: false,
    };
    let living = LivingEntity {
        hand_states: 0,
        health: default_max_health(kind),
        arrow_count: 0,
        stinger_count: 0,
        sleeping_bed_pos: None,
        absorption: 0.0,
        hurt_time: 0,
        death_time: 0,
        is_dead: false,
    };
    let payload = match kind {
        EntityKind::Zombie => EntityPayload::Zombie(ZombieBundle),
        EntityKind::Cow => EntityPayload::Cow(CowBundle),
        // Never reached: this blueprint's own two shipped spawn lists (`tables::
        // spawn_list`) only ever produce `Zombie`/`Cow` entries (Scope boundary).
        EntityKind::Item | EntityKind::Villager => {
            unreachable!("natural spawning never picks a spawner entry for Item or Villager")
        }
    };
    let marker = MobMarker {
        ai_system: AiSystemKind::GoalSelector,
        persistence_required: false,
        can_pick_up_loot: false,
    };
    world.spawn_mob(kind, base, Some(living), payload, marker, category);
    SpawnedMob {
        kind,
        category,
        pos: spawn_pos,
        yaw,
    }
}

/// One `(category, chunk)` pack-spawn attempt (blueprint Context's own pseudocode,
/// restated as this function's real body). Exposed separately from `run_spawn_cycle` so
/// acceptance tests can exercise the pack algorithm's own RNG-call shape and
/// cluster-size cap in isolation, without a full chunk-shuffle/cap-filter harness.
pub fn spawn_category_for_chunk(
    category: MobCategory,
    chunk: ChunkKey,
    world: &mut dyn SpawnWorldAccess,
    rng: &mut RcRandom,
    census: &mut RegionCensusState,
) -> Vec<SpawnedMob> {
    let min_x = chunk.x * 16;
    let min_z = chunk.z * 16;
    let x0 = min_x + rng.next_int_bounded(16);
    let z0 = min_z + rng.next_int_bounded(16);
    let top_empty_y = world.topmost_non_air_y(x0, z0) + 1;
    let min_y = world.min_y();
    let y0 = min_y + rng.next_int_bounded(top_empty_y - min_y + 1);
    if y0 < min_y + 1 {
        return Vec::new();
    }

    let anchor = BlockPos::new(x0, y0, z0);
    if world.is_full_opaque_cube(anchor) {
        return Vec::new();
    }

    let players = world.players();
    // SHARED across all 3 group tries below — a cap on this WHOLE (category, chunk)
    // call, never reset between the 3 tries (M4-B04-CLAIMS.md row 33).
    let mut cluster_size: u32 = 0;
    let mut spawned = Vec::new();

    'groups: for _ in 0..3 {
        let mut x = x0;
        let mut z = z0;
        let mut current_species: Option<SpawnerEntry> = None;
        let mut max = (rng.next_float() * 4.0).ceil() as i32;
        let mut ll = 0;
        while ll < max {
            x += rng.next_int_bounded(6) - rng.next_int_bounded(6);
            z += rng.next_int_bounded(6) - rng.next_int_bounded(6);
            let pos = BlockPos::new(x, y0, z);
            let pos_f = [pos.x as f64, pos.y as f64, pos.z as f64];
            let Some(dist_sqr) = nearest_player_dist_sqr(&players, pos_f) else {
                ll += 1;
                continue;
            };
            if dist_sqr <= MIN_SPAWN_DISTANCE_TO_PLAYER * MIN_SPAWN_DISTANCE_TO_PLAYER {
                ll += 1;
                continue;
            }
            if current_species.is_none() {
                let Some(entry) = pick_weighted(spawn_list(category), rng) else {
                    break;
                };
                max = entry.min_count as i32
                    + rng.next_int_bounded(1 + entry.max_count as i32 - entry.min_count as i32);
                current_species = Some(entry);
            }
            let entry = current_species.expect("set immediately above on first iteration");
            if is_spawn_position_legal(world, rng, entry.category, pos) {
                let yaw = rng.next_float() * 360.0;
                let _ = individuality_bonus(rng);
                let mob = build_and_spawn(world, entry.kind, entry.category, pos, yaw);
                census.record_spawn(entry.category, mob.pos, &players);
                spawned.push(mob);
                cluster_size += 1;
                if cluster_size >= MAX_SPAWN_CLUSTER_SIZE {
                    break 'groups;
                }
                // `isMaxGroupSizeReached` is false by default for every tier-2 kind
                // (M4-B04-CLAIMS.md row 41) — never breaks here.
            }
            ll += 1;
        }
    }
    spawned
}

/// `docs/research/mc-26.2/23-spawning-math.md` §3.2's per-tick driver, restated and
/// scoped (blueprint Context). `global_cap_ok(category) -> bool` is supplied by the
/// caller (`ecs.rs`'s `system_mob_spawn_cycle`), which alone has access to
/// `GlobalMobCensus`'s cross-region aggregate — this pure function never reads that
/// aggregate itself, only the local half via `census.local`.
pub fn run_spawn_cycle(
    world: &mut dyn SpawnWorldAccess,
    rng: &mut RcRandom,
    census: &mut RegionCensusState,
    global_cap_ok: impl Fn(MobCategory) -> bool,
    current_tick: u64,
) -> Vec<SpawnedMob> {
    let spawn_persistent = current_tick.is_multiple_of(400);
    let candidate_categories: Vec<MobCategory> = MobCategory::ALL
        .into_iter()
        .filter(|category| {
            (spawn_persistent || !category.is_persistent()) && global_cap_ok(*category)
        })
        .collect();
    if candidate_categories.is_empty() {
        return Vec::new();
    }

    let mut chunks = world.spawn_candidate_chunks();
    fisher_yates_shuffle(&mut chunks, rng);

    let mut spawned = Vec::new();
    for chunk in &chunks {
        for category in MobCategory::ALL {
            if !candidate_categories.contains(&category) {
                continue;
            }
            let players = world.players();
            if !census.local.allows(
                category,
                crate::spawn::census::chunk_center(*chunk),
                &players,
            ) {
                continue;
            }
            spawned.extend(spawn_category_for_chunk(
                category, *chunk, world, rng, census,
            ));
        }
    }
    spawned
}
