//! M3-B06 — random-tick position-selection acceptance tests (Acceptance tests' own
//! `random_tick_positions.rs` section): the pure, ECS-agnostic `draw_random_tick_positions`/
//! `random_tick_chunk` algorithm.

use rc_core::BlockPos;
use rc_mechanics::random::{RcRandom, chunk_random_seed};
use rc_mechanics::random_tick::{
    RandomTickPosition, WorldSeed, draw_random_tick_positions, random_tick_chunk,
};

#[test]
fn known_seed_produces_exact_position_sequence() {
    let seed = WorldSeed(42);
    let result = random_tick_chunk(&seed, 0, 0, 0, 3);
    assert_eq!(result.len(), 72);

    let mut rng = RcRandom::new(chunk_random_seed(42, 0, 0, 0));
    let mut expected_first_three = Vec::new();
    for _ in 0..3 {
        let bits = rng.next_int() as u32;
        let x = (bits & 15) as i32;
        let z = ((bits >> 8) & 15) as i32;
        let y_local = ((bits >> 16) & 15) as i32;
        expected_first_three.push(RandomTickPosition {
            pos: BlockPos::new(x, -64 + y_local, z),
        });
    }

    assert_eq!(&result[0..3], expected_first_three.as_slice());
}

#[test]
fn every_draw_is_in_bounds() {
    let mut rng = RcRandom::new(chunk_random_seed(7, 3, -5, 12));
    let result = draw_random_tick_positions(&mut rng, 0, 0, 3);
    assert_eq!(result.len(), 72);
    for candidate in &result {
        assert!(
            (0..16).contains(&candidate.pos.x),
            "x out of bounds: {candidate:?}"
        );
        assert!(
            (0..16).contains(&candidate.pos.z),
            "z out of bounds: {candidate:?}"
        );
        assert!(
            (-64..320).contains(&candidate.pos.y),
            "y out of bounds: {candidate:?}"
        );
    }
}

#[test]
fn section_order_is_ascending() {
    let mut rng = RcRandom::new(chunk_random_seed(7, 3, -5, 12));
    let result = draw_random_tick_positions(&mut rng, 0, 0, 3);
    assert_eq!(result.len(), 72);
    for (i, run) in result.chunks(3).enumerate() {
        let section_min_y = -64 + i as i32 * 16;
        let section_max_y_exclusive = section_min_y + 16;
        for candidate in run {
            assert!(
                candidate.pos.y >= section_min_y && candidate.pos.y < section_max_y_exclusive,
                "run {i}: pos {candidate:?} outside expected section range [{section_min_y}, {section_max_y_exclusive})"
            );
        }
    }
}

#[test]
fn same_seed_same_speed_is_deterministic() {
    let seed = WorldSeed(999);
    let first = random_tick_chunk(&seed, 4, -2, 100, 3);
    for _ in 0..5 {
        let again = random_tick_chunk(&seed, 4, -2, 100, 3);
        assert_eq!(again, first);
    }
}

#[test]
fn different_tick_counter_changes_the_sequence() {
    let seed = WorldSeed(999);
    let at_tick_0 = random_tick_chunk(&seed, 4, -2, 0, 3);
    let at_tick_1 = random_tick_chunk(&seed, 4, -2, 1, 3);
    assert_ne!(at_tick_0, at_tick_1);
}

#[test]
fn random_tick_speed_scales_draw_count() {
    let seed = WorldSeed(1234);
    assert_eq!(random_tick_chunk(&seed, 0, 0, 0, 1).len(), 24);
    assert_eq!(random_tick_chunk(&seed, 0, 0, 0, 5).len(), 120);
}

#[test]
fn with_replacement_can_repeat_within_one_chunk_tick() {
    // Statistical alternative (Acceptance tests' own "if no such literal is convenient"
    // clause): across 50 different tick_counter values for one chunk, at least one exact
    // `pos` collision occurs somewhere in the combined draw set -- a near-certainty at
    // 24*3=72 draws per tick over a 16x16x384 space, and the only way this could fail is if
    // draws were deduplicated (which they are not, by design).
    let seed = WorldSeed(2024);
    let mut seen = std::collections::HashSet::new();
    let mut found_duplicate = false;
    for tick in 0..50u64 {
        for candidate in random_tick_chunk(&seed, 0, 0, tick, 3) {
            if !seen.insert(candidate.pos) {
                found_duplicate = true;
            }
        }
    }
    assert!(
        found_duplicate,
        "expected at least one repeated position across 50 chunk-ticks' worth of draws"
    );
}
