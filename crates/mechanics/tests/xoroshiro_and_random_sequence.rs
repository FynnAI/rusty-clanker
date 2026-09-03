//! M4-B02 acceptance tests: `rc-rng`'s Xoroshiro128++ core and the `random_sequence` seeding
//! formula (Context §K), exercised through `rc_mechanics::random`'s own re-exports — the same
//! type/functions `rc_rng` itself defines (`crates/rng/src/xoroshiro.rs`).

use rc_mechanics::random::{RcRandomSource, XoroshiroRandom};
use rc_rng::{mix_stafford13, upgrade_seed_128_unmixed};

#[test]
fn xoroshiro_next_long_matches_published_vector() {
    let mut rng = XoroshiroRandom::new(0);
    let expected = [
        3038984756725240190i64,
        -3694039286755638414,
        4633751808701151732,
        2160572957309072155,
        1839370574944072389,
    ];
    for (i, want) in expected.into_iter().enumerate() {
        let got = rng.next_long();
        assert_eq!(got, want, "next_long() call {}", i + 1);
    }
}

#[test]
fn xoroshiro_seeded_42_matches_published_vector() {
    let mut rng = XoroshiroRandom::new(42);
    let expected = [
        -4695948378737616609i64,
        7341713790291473579,
        -7542733514721318211,
    ];
    for (i, want) in expected.into_iter().enumerate() {
        let got = rng.next_long();
        assert_eq!(got, want, "next_long() call {}", i + 1);
    }
}

#[test]
fn upgrade_seed_128_unmixed_then_mixed_matches_published_pair() {
    let (lo, hi) = upgrade_seed_128_unmixed(0);
    let (mixed_lo, mixed_hi) = (mix_stafford13(lo), mix_stafford13(hi));
    assert_eq!(mixed_lo, 3847398142028685078);
    assert_eq!(mixed_hi, 7192185014346937746);
}

#[test]
fn random_sequence_is_deterministic_and_stateful() {
    use rc_mechanics::entity::RandomSequenceStore;

    let mut store = RandomSequenceStore::default();
    let mut first_three = Vec::new();
    {
        let rng = store.get_or_create("test:seq_a", 12345);
        for _ in 0..3 {
            first_three.push(rng.next_int_bounded(100));
        }
    }
    assert_eq!(
        first_three.len(),
        3,
        "recorded exactly the first three draws"
    );
    let fourth_original = {
        let rng = store.get_or_create("test:seq_a", 12345);
        rng.next_int_bounded(100)
    };

    // A FRESH stream's own first draw is not the same as this stream's own fourth draw --
    // proving the stream continues rather than resets on the second `get_or_create` call.
    let fresh_first_draw =
        rc_rng::create_random_sequence_default("test:seq_a", 12345).next_int_bounded(100);
    assert_ne!(
        fourth_original, fresh_first_draw,
        "the stream must not have reset back to its own starting state"
    );

    // Replaying all four draws from scratch, in order, through a brand-new store reproduces
    // the same fourth value exactly -- full-history reproducibility.
    let mut replay_store = RandomSequenceStore::default();
    let fourth_replay = {
        let rng = replay_store.get_or_create("test:seq_a", 12345);
        for _ in 0..3 {
            let _ = rng.next_int_bounded(100);
        }
        rng.next_int_bounded(100)
    };
    assert_eq!(fourth_replay, fourth_original);
}

#[test]
fn random_sequence_with_different_ids_are_independent() {
    use rc_mechanics::entity::RandomSequenceStore;

    let mut store = RandomSequenceStore::default();
    let a = store.get_or_create("test:seq_a", 1).next_long();
    let b = store.get_or_create("test:seq_b", 1).next_long();
    assert_ne!(a, b);
}
