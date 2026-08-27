//! M3-B01 — `RcRandom`/`chunk_random_seed` acceptance tests (pure, no `bevy_ecs::World`).

use rc_mechanics::{RcRandom, chunk_random_seed};

#[test]
fn next_int_matches_known_java_sequence() {
    // `java.util.Random(42)`'s first three `nextInt()` values — a "known published value"
    // independently verifiable against any JDK (per the firewall notes' own §7 convention).
    let mut rng = RcRandom::new(42);
    assert_eq!(rng.next_int(), -1170105035);
    assert_eq!(rng.next_int(), 234785527);
    assert_eq!(rng.next_int(), -1360544799);
}

#[test]
fn next_int_bounded_power_of_two_uses_fast_path() {
    let mut rng = RcRandom::new(1);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        let v = rng.next_int_bounded(16);
        assert!((0..16).contains(&v), "value {v} out of range 0..16");
        seen.insert(v);
    }
    assert!(
        seen.len() > 1,
        "1000 draws should not all collapse to one value"
    );
}

#[test]
fn chunk_random_seed_is_deterministic() {
    let base = chunk_random_seed(1234, 5, -7, 100);
    assert_eq!(chunk_random_seed(1234, 5, -7, 100), base);

    assert_ne!(chunk_random_seed(9999, 5, -7, 100), base);
    assert_ne!(chunk_random_seed(1234, 6, -7, 100), base);
    assert_ne!(chunk_random_seed(1234, 5, -8, 100), base);
    assert_ne!(chunk_random_seed(1234, 5, -7, 101), base);
}

#[test]
fn chunk_random_seed_differs_across_ticks() {
    let seed0 = chunk_random_seed(42, 3, 3, 0);
    let seed1 = chunk_random_seed(42, 3, 3, 1);
    assert_ne!(seed0, seed1);

    let mut r0 = RcRandom::new(seed0);
    let mut r1 = RcRandom::new(seed1);
    let stream0 = [r0.next_int(), r0.next_int(), r0.next_int()];
    let stream1 = [r1.next_int(), r1.next_int(), r1.next_int()];
    assert_ne!(stream0, stream1);
}
