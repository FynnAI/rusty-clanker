//! M4-B04 acceptance tests: the natural per-tick spawn-cycle algorithm
//! (`run_spawn_cycle`/`spawn_category_for_chunk`) — RNG-call-shape and cap-enforcement
//! scenarios, restated field-precise against `M4-B04-CLAIMS.md`.
//!
//! Uses a synthetic test-double `SpawnWorldAccess`, deliberately more permissive than the
//! real superflat world (a single solid Y layer with open air immediately above it, so
//! exactly one of the anchor's own candidate Y values is legal — avoids needing thousands
//! of iterations to observe a spawn, per the blueprint's own note about the real
//! superflat world's low yield ratio).

use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::entity::{BaseEntity, EntityKind, EntityPayload, LivingEntity, MobMarker};
use rc_mechanics::random::RcRandom;
use rc_mechanics::spawn::{
    MobCategory, RegionCensusState, SpawnWorldAccess, run_spawn_cycle, spawn_category_for_chunk,
};

const MIN_Y: i32 = -64;
/// The only solid Y layer — open air immediately above it, matching this file's own
/// module doc comment.
const FLOOR_Y: i32 = -61;
/// The Y value one above `FLOOR_Y` — the sole legal `y0` this test double's own anchor
/// roll can land on.
const LEGAL_Y: i32 = FLOOR_Y + 1;
/// `top_empty_y - MIN_Y + 1` for this test double (`topmost_non_air_y` always returns
/// `FLOOR_Y`, so `top_empty_y = FLOOR_Y + 1`).
const Y_ROLL_BOUND: i32 = FLOOR_Y + 1 - MIN_Y + 1;

#[derive(Clone)]
struct CycleTestWorld {
    always_opaque: bool,
    sky_light: u8,
    block_light: u8,
    players: Vec<(i32, [f64; 3])>,
    candidate_chunks: Vec<ChunkKey>,
}

impl CycleTestWorld {
    fn permissive() -> Self {
        Self {
            always_opaque: false,
            sky_light: 15,
            block_light: 0,
            // Within the chunk-candidate/local-cap rule's own 128-block horizontal radius
            // of chunk (0,0)'s own center (8.0, _, 8.0), but far enough (in full 3D) from
            // every candidate position near that chunk's own anchor to never trigger the
            // pack-spawn algorithm's own 24-block player-exclusion check.
            players: vec![(1, [100.0, LEGAL_Y as f64, 8.0])],
            candidate_chunks: vec![ChunkKey::new(DimensionId::OVERWORLD, 0, 0)],
        }
    }
}

impl SpawnWorldAccess for CycleTestWorld {
    fn min_y(&self) -> i32 {
        MIN_Y
    }
    fn topmost_non_air_y(&self, _x: i32, _z: i32) -> i32 {
        FLOOR_Y
    }
    fn is_full_opaque_cube(&self, pos: BlockPos) -> bool {
        self.always_opaque || pos.y == FLOOR_Y
    }
    fn has_fluid(&self, _pos: BlockPos) -> bool {
        false
    }
    fn sky_light(&self, _pos: BlockPos) -> u8 {
        self.sky_light
    }
    fn block_light(&self, _pos: BlockPos) -> u8 {
        self.block_light
    }
    fn sky_darken(&self) -> i32 {
        0
    }
    fn spawn_candidate_chunks(&self) -> Vec<ChunkKey> {
        self.candidate_chunks.clone()
    }
    fn players(&self) -> Vec<(i32, [f64; 3])> {
        self.players.clone()
    }
    fn spawn_mob(
        &mut self,
        _kind: EntityKind,
        _base: BaseEntity,
        _living: Option<LivingEntity>,
        _payload: EntityPayload,
        _marker: MobMarker,
        _category: MobCategory,
    ) {
        // The pure algorithm's own `Vec<SpawnedMob>` return value is this test suite's
        // only observation point — no further bookkeeping needed here.
    }
}

#[test]
fn seeded_spawn_cycle_is_deterministic() {
    let chunk_a = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let chunk_b = ChunkKey::new(DimensionId::OVERWORLD, 1, 0);
    let mut world = CycleTestWorld::permissive();
    world.candidate_chunks = vec![chunk_a, chunk_b];

    let run = |seed: i64| {
        let mut world = world.clone();
        let mut rng = RcRandom::new(seed);
        let mut census = RegionCensusState::default();
        run_spawn_cycle(&mut world, &mut rng, &mut census, |_| true, 0)
    };

    let first = run(123456);
    let second = run(123456);
    assert_eq!(first, second);
}

#[test]
fn pack_spawn_cluster_size_never_exceeds_four() {
    let mut world = CycleTestWorld::permissive();
    let chunk = world.candidate_chunks[0];

    for seed in 0..500i64 {
        let mut rng = RcRandom::new(seed);
        let mut census = RegionCensusState::default();
        let spawned = spawn_category_for_chunk(
            MobCategory::Creature,
            chunk,
            &mut world,
            &mut rng,
            &mut census,
        );
        assert!(
            spawned.len() <= 4,
            "seed {seed} produced {} spawns, exceeding MAX_SPAWN_CLUSTER_SIZE",
            spawned.len()
        );
    }
}

#[test]
fn redstone_conductor_anchor_skips_whole_attempt() {
    let mut world = CycleTestWorld::permissive();
    world.always_opaque = true;
    let chunk = world.candidate_chunks[0];

    let seed = 42;
    let mut rng = RcRandom::new(seed);
    let mut census = RegionCensusState::default();
    let spawned = spawn_category_for_chunk(
        MobCategory::Monster,
        chunk,
        &mut world,
        &mut rng,
        &mut census,
    );
    assert!(spawned.is_empty());

    // Exactly 3 RNG calls consumed (the anchor roll only): x0, z0, y0.
    let mut reference = RcRandom::new(seed);
    reference.next_int_bounded(16);
    reference.next_int_bounded(16);
    reference.next_int_bounded(Y_ROLL_BOUND);
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn global_cap_at_limit_prevents_any_attempt_for_that_category() {
    let mut world = CycleTestWorld::permissive();
    let mut rng = RcRandom::new(777);
    let mut census = RegionCensusState::default();

    let spawned = run_spawn_cycle(
        &mut world,
        &mut rng,
        &mut census,
        |category| category != MobCategory::Monster,
        0,
    );
    assert!(
        spawned.iter().all(|m| m.category != MobCategory::Monster),
        "Monster must never spawn once its own global cap denies the whole tick"
    );
}

#[test]
fn local_cap_multiplayer_scaling() {
    let chunk = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    // Both players sit well outside the 24-block pack-spawn exclusion radius from any
    // candidate near the chunk's own anchor, but within the chunk-candidate/local-cap
    // rule's own 128-block horizontal radius of the chunk center (8.0, _, 8.0).
    let player_1 = (1, [58.0, LEGAL_Y as f64, 8.0]);
    let player_2 = (2, [8.0, LEGAL_Y as f64, 58.0]);

    let mut world = CycleTestWorld::permissive();
    world.candidate_chunks = vec![chunk];
    world.players = vec![player_1, player_2];

    let mut census = RegionCensusState::default();
    // Player 1 is at its own local cap for `Creature` (10); player 2 is under.
    for _ in 0..10 {
        census
            .local
            .bump_near(MobCategory::Creature, player_1.1, &[player_1]);
    }

    let mut rng = RcRandom::new(2024);
    let spawned = run_spawn_cycle(&mut world, &mut rng, &mut census, |_| true, 0);
    assert!(
        spawned.iter().any(|m| m.category == MobCategory::Creature),
        "Creature spawns must still proceed while at least one nearby player has room"
    );
}

#[test]
fn chunk_shuffle_is_fisher_yates_and_consumes_len_minus_one_calls() {
    let mut world = CycleTestWorld::permissive();
    // No players at all -- `LocalCapCounts::allows` is unconditionally `false`, so
    // nothing beyond the shuffle itself ever consumes RNG.
    world.players = Vec::new();
    world.candidate_chunks = (0..5)
        .map(|x| ChunkKey::new(DimensionId::OVERWORLD, x, 0))
        .collect();
    let len = world.candidate_chunks.len();

    let seed = 909;
    let mut rng = RcRandom::new(seed);
    let mut census = RegionCensusState::default();
    let spawned = run_spawn_cycle(&mut world, &mut rng, &mut census, |_| true, 0);
    assert!(spawned.is_empty());

    let mut reference = RcRandom::new(seed);
    for i in (2..=len as i32).rev() {
        reference.next_int_bounded(i);
    }
    assert_eq!(rng.next_int(), reference.next_int());
}

/// Cross-checks `finalize_spawn`'s individuality-bonus RNG cost (3 calls per successful
/// spawn, M4-B04-CLAIMS.md rows 7/39) against a manually replayed reference `RcRandom`
/// sequence. Searches for a seed where this test double's own maximally-permissive
/// scenario (a single, always-legal `y0`, a distant player never triggering the 24-block
/// exclusion) drives `spawn_category_for_chunk` to fill the shared cluster-size cap
/// (`MAX_SPAWN_CLUSTER_SIZE = 4`) entirely within the first group try, then verifies both
/// that outcome AND that a hand-replayed sequence of the exact same primitive `RcRandom`
/// calls (anchor, one group-try max roll, then four sub-spawn iterations — the first
/// paying the species-pick/group-size-reroll cost the later three don't — each followed
/// by a yaw draw and the 3-call individuality bonus) reaches an identical subsequent RNG
/// state. A seed where either check fails is skipped rather than asserted against,
/// keeping this test's own correctness independent of exactly which seed it lands on.
#[test]
fn finalize_spawn_individuality_bonus_consumes_exactly_three_calls_per_successful_spawn() {
    let mut world = CycleTestWorld::permissive();
    let chunk = world.candidate_chunks[0];

    for seed in 0..2000i64 {
        let mut rng = RcRandom::new(seed);
        let mut census = RegionCensusState::default();
        let spawned = spawn_category_for_chunk(
            MobCategory::Creature,
            chunk,
            &mut world,
            &mut rng,
            &mut census,
        );
        if spawned.len() != 4 {
            continue;
        }

        let mut reference = RcRandom::new(seed);
        reference.next_int_bounded(16); // x0
        reference.next_int_bounded(16); // z0
        reference.next_int_bounded(Y_ROLL_BOUND); // y0
        reference.next_float(); // this group try's own initial max-count roll

        for sub_spawn in 0..4 {
            reference.next_int_bounded(6); // x offset (positive term)
            reference.next_int_bounded(6); // x offset (negative term)
            reference.next_int_bounded(6); // z offset (positive term)
            reference.next_int_bounded(6); // z offset (negative term)
            if sub_spawn == 0 {
                reference.next_int_bounded(8); // pick_weighted over CREATURE_LIST (weight 8)
                reference.next_int_bounded(1); // group-size reroll (min_count == max_count == 4)
            }
            // `is_on_ground_legal`/`is_animal_light_ok` cost zero RNG calls.
            reference.next_float(); // yaw
            reference.next_double(); // individuality-bonus triangle sample, term 1
            reference.next_double(); // individuality-bonus triangle sample, term 2
            reference.next_float(); // individuality-bonus left-handed roll
        }

        if rng.next_int() == reference.next_int() {
            return; // found a seed matching this exact call-sequence assumption
        }
    }
    panic!("no seed in the search range matched the expected 4-spawns-in-one-group-try shape");
}
