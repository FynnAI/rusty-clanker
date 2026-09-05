//! M4-B04 acceptance tests: `SpawnPlacementType::OnGround` legality, the Monster
//! darkness gate, and the Creature (animal) light rule.

use std::collections::HashSet;

use rc_core::{BlockPos, ChunkKey};
use rc_mechanics::entity::{BaseEntity, EntityKind, EntityPayload, LivingEntity, MobMarker};
use rc_mechanics::random::RcRandom;
use rc_mechanics::spawn::{
    MobCategory, SpawnWorldAccess, is_animal_light_ok, is_dark_enough_to_spawn, is_on_ground_legal,
};

/// A small, fixed in-memory `SpawnWorldAccess` test double: `solid` names every position
/// that is a full opaque cube; `fluid` names every position that holds a fluid. Every
/// other position is open air.
#[derive(Default)]
struct TestWorld {
    solid: HashSet<(i32, i32, i32)>,
    fluid: HashSet<(i32, i32, i32)>,
}

impl TestWorld {
    fn with_solid(mut self, pos: BlockPos) -> Self {
        self.solid.insert((pos.x, pos.y, pos.z));
        self
    }
    fn with_fluid(mut self, pos: BlockPos) -> Self {
        self.fluid.insert((pos.x, pos.y, pos.z));
        self
    }
}

impl SpawnWorldAccess for TestWorld {
    fn min_y(&self) -> i32 {
        -64
    }
    fn topmost_non_air_y(&self, _x: i32, _z: i32) -> i32 {
        -65
    }
    fn is_full_opaque_cube(&self, pos: BlockPos) -> bool {
        self.solid.contains(&(pos.x, pos.y, pos.z))
    }
    fn has_fluid(&self, pos: BlockPos) -> bool {
        self.fluid.contains(&(pos.x, pos.y, pos.z))
    }
    fn sky_light(&self, _pos: BlockPos) -> u8 {
        0
    }
    fn block_light(&self, _pos: BlockPos) -> u8 {
        0
    }
    fn sky_darken(&self) -> i32 {
        0
    }
    fn spawn_candidate_chunks(&self) -> Vec<ChunkKey> {
        Vec::new()
    }
    fn players(&self) -> Vec<(i32, [f64; 3])> {
        Vec::new()
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
        unreachable!("this test file never spawns a mob")
    }
}

const POS: BlockPos = BlockPos::new(0, 0, 0);

#[test]
fn on_ground_legal_when_solid_below_and_open_above() {
    let world = TestWorld::default().with_solid(BlockPos::new(0, -1, 0));
    assert!(is_on_ground_legal(&world, POS));
}

#[test]
fn illegal_when_pos_itself_is_solid() {
    let world = TestWorld::default()
        .with_solid(BlockPos::new(0, -1, 0))
        .with_solid(POS);
    assert!(!is_on_ground_legal(&world, POS));
}

#[test]
fn illegal_when_block_above_is_solid() {
    let world = TestWorld::default()
        .with_solid(BlockPos::new(0, -1, 0))
        .with_solid(BlockPos::new(0, 1, 0));
    assert!(!is_on_ground_legal(&world, POS));
}

#[test]
fn illegal_when_support_block_is_not_full_opaque() {
    // No solid block below at all -- the support-block check fails outright.
    let world = TestWorld::default();
    assert!(!is_on_ground_legal(&world, POS));
}

#[test]
fn illegal_when_pos_itself_holds_a_fluid() {
    let world = TestWorld::default()
        .with_solid(BlockPos::new(0, -1, 0))
        .with_fluid(POS);
    assert!(!is_on_ground_legal(&world, POS));
}

#[test]
fn darkness_gate_costs_one_call_when_sky_term_fails() {
    // `sky_light = 15` fails against most rolls of `next_int_bounded(32)` -- find a seed
    // whose very first roll is small enough that 15 > roll.
    let seed = find_seed_where_first_roll_lt(15, 32);
    let mut rng = RcRandom::new(seed);
    let mut reference = RcRandom::new(seed);
    reference.next_int_bounded(32); // the one call this branch always pays

    let result = is_dark_enough_to_spawn(&mut rng, 15, 0, 0);
    assert!(!result);
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn darkness_gate_costs_two_calls_when_sky_term_passes() {
    // `sky_light = 0` never exceeds any non-negative roll -- the sky term always passes,
    // so the second (brightness) roll is always paid too.
    let seed = 99;
    let mut rng = RcRandom::new(seed);
    let mut reference = RcRandom::new(seed);
    reference.next_int_bounded(32);
    reference.next_int_bounded(8);

    let _ = is_dark_enough_to_spawn(&mut rng, 0, 0, 0);
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn darkness_gate_is_deterministic_for_fixed_seed_and_inputs() {
    let mut a = RcRandom::new(555);
    let mut b = RcRandom::new(555);
    assert_eq!(
        is_dark_enough_to_spawn(&mut a, 4, 0, 0),
        is_dark_enough_to_spawn(&mut b, 4, 0, 0)
    );
}

#[test]
fn darkness_gate_formula_subtracts_sky_darken_from_sky_light() {
    // A fixed seed whose sky-light-term roll (against sky_light=10) is pre-verified to
    // pass (10 <= roll(0..32)), then two calls differing only in `sky_darken`, compared
    // against a manually-computed `max(block_light, sky_light - sky_darken)` reference
    // and a fixed final-roll draw.
    let seed = find_seed_where_first_roll_geq(10, 32);

    // sky_darken = 0: brightness = max(0, 10 - 0) = 10, never <= a next_int_bounded(8)
    // draw (0..=7) -- always false.
    let mut rng_a = RcRandom::new(seed);
    assert!(!is_dark_enough_to_spawn(&mut rng_a, 10, 0, 0));

    // sky_darken = 5: brightness = max(0, 10 - 5) = 5, which CAN be <= a next_int_bounded(8)
    // draw -- pins that the formula is the real vanilla one (`max(block_light, sky_light -
    // sky_darken)`), not the current-scope-simplified `max(sky_light, block_light)` (which
    // would ignore `sky_darken` entirely and always compute brightness = 10 regardless).
    let mut rng_b = RcRandom::new(seed);
    let mut reference = RcRandom::new(seed);
    reference.next_int_bounded(32);
    let final_roll = reference.next_int_bounded(8);
    let expected = 5 <= final_roll;
    assert_eq!(is_dark_enough_to_spawn(&mut rng_b, 10, 0, 5), expected);
}

#[test]
fn animal_light_rule_boundary_at_9() {
    let mut rng = RcRandom::new(1);
    let mut reference = RcRandom::new(1);

    assert!(!is_animal_light_ok(8, 0));
    assert!(is_animal_light_ok(9, 0));
    assert!(is_animal_light_ok(0, 9));

    // Zero RNG cost.
    assert_eq!(rng.next_int(), reference.next_int());
}

/// Brute-force search for a seed whose very first `next_int_bounded(bound)` draw is
/// strictly less than `threshold` -- a small test-only helper, not part of the
/// production API.
fn find_seed_where_first_roll_lt(threshold: i32, bound: i32) -> i64 {
    for seed in 0..10_000i64 {
        let mut rng = RcRandom::new(seed);
        if rng.next_int_bounded(bound) < threshold {
            return seed;
        }
    }
    panic!("no seed found in search range");
}

fn find_seed_where_first_roll_geq(threshold: i32, bound: i32) -> i64 {
    for seed in 0..10_000i64 {
        let mut rng = RcRandom::new(seed);
        if rng.next_int_bounded(bound) >= threshold {
            return seed;
        }
    }
    panic!("no seed found in search range");
}
