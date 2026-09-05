//! M4-B04 acceptance tests: despawn rules (`check_despawn`,
//! `remove_when_far_away_for_kind`) — restated exactly against `M4-B04-CLAIMS.md` rows
//! 45-48/61.

use rc_mechanics::entity::EntityKind;
use rc_mechanics::random::RcRandom;
use rc_mechanics::spawn::{
    DespawnDecision, DespawnTimer, MobCategory, check_despawn, remove_when_far_away_for_kind,
};

const MONSTER_DIST_SQR: f64 = 128.0 * 128.0;
const NO_DESPAWN_DIST_SQR: f64 = 32.0 * 32.0;

#[test]
fn instant_despawn_beyond_category_distance() {
    let mut rng = RcRandom::new(1);
    let mut reference = RcRandom::new(1);
    let mut timer = DespawnTimer::default();

    let decision = check_despawn(
        false,
        Some(MONSTER_DIST_SQR + 1.0),
        MobCategory::Monster,
        true,
        &mut rng,
        &mut timer,
    );
    assert_eq!(decision, DespawnDecision::Despawn);
    // No RNG consumed on the instant-distance path.
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn no_despawn_within_category_distance_and_inactive_less_than_600() {
    let mut rng = RcRandom::new(2);
    let mut reference = RcRandom::new(2);
    let mut timer = DespawnTimer {
        no_action_ticks: 100,
    };

    let dist_sqr = NO_DESPAWN_DIST_SQR + (MONSTER_DIST_SQR - NO_DESPAWN_DIST_SQR) / 2.0;
    let decision = check_despawn(
        false,
        Some(dist_sqr),
        MobCategory::Monster,
        true,
        &mut rng,
        &mut timer,
    );
    assert_eq!(decision, DespawnDecision::Keep);
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn random_despawn_rolls_only_past_600_ticks_and_beyond_32_blocks() {
    let seed = 3;
    let mut rng = RcRandom::new(seed);
    let mut reference = RcRandom::new(seed);
    let roll = reference.next_int_bounded(800); // the one call this branch always pays

    let mut timer = DespawnTimer {
        no_action_ticks: 601,
    };
    let dist_sqr = NO_DESPAWN_DIST_SQR + (MONSTER_DIST_SQR - NO_DESPAWN_DIST_SQR) / 2.0;
    let decision = check_despawn(
        false,
        Some(dist_sqr),
        MobCategory::Monster,
        true,
        &mut rng,
        &mut timer,
    );

    let expected = if roll == 0 {
        DespawnDecision::Despawn
    } else {
        DespawnDecision::Keep
    };
    assert_eq!(decision, expected);
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn random_despawn_never_rolls_before_600_ticks() {
    let mut rng = RcRandom::new(4);
    let mut reference = RcRandom::new(4);
    let mut timer = DespawnTimer {
        no_action_ticks: 599,
    };

    let dist_sqr = NO_DESPAWN_DIST_SQR + (MONSTER_DIST_SQR - NO_DESPAWN_DIST_SQR) / 2.0;
    let decision = check_despawn(
        false,
        Some(dist_sqr),
        MobCategory::Monster,
        true,
        &mut rng,
        &mut timer,
    );
    assert_eq!(decision, DespawnDecision::Keep);
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn persistence_required_always_keeps_and_resets_timer() {
    let mut rng = RcRandom::new(5);
    let mut reference = RcRandom::new(5);
    let mut timer = DespawnTimer {
        no_action_ticks: 12345,
    };

    let decision = check_despawn(
        true,
        Some(MONSTER_DIST_SQR + 1.0),
        MobCategory::Monster,
        true,
        &mut rng,
        &mut timer,
    );
    assert_eq!(decision, DespawnDecision::Keep);
    assert_eq!(timer.no_action_ticks, 0);
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn within_32_blocks_resets_inactivity_timer() {
    let mut rng = RcRandom::new(6);
    // Deliberately below 600 so the random-roll branch's own first conjunct is false and
    // no RNG is consumed, regardless of this test's own distance value -- keeps this
    // test's own outcome unambiguous.
    let mut timer = DespawnTimer { no_action_ticks: 5 };

    let decision = check_despawn(
        false,
        Some(NO_DESPAWN_DIST_SQR - 1.0),
        MobCategory::Monster,
        true,
        &mut rng,
        &mut timer,
    );
    assert_eq!(decision, DespawnDecision::Keep);
    assert_eq!(timer.no_action_ticks, 0);
}

#[test]
fn remove_when_far_away_false_prevents_all_despawn() {
    // Instant-distance case: zero RNG cost.
    let mut rng_instant = RcRandom::new(7);
    let mut reference_instant = RcRandom::new(7);
    let mut timer = DespawnTimer::default();
    let decision = check_despawn(
        false,
        Some(MONSTER_DIST_SQR + 1.0),
        MobCategory::Monster,
        false,
        &mut rng_instant,
        &mut timer,
    );
    assert_eq!(decision, DespawnDecision::Keep);
    assert_eq!(rng_instant.next_int(), reference_instant.next_int());

    // Random-roll-eligible case: the roll is still consumed even though the outcome is
    // forced to `Keep` by `remove_when_far_away == false` (the roll is evaluated before
    // the conjunct short-circuits).
    let seed = 8;
    let mut rng_roll = RcRandom::new(seed);
    let mut reference_roll = RcRandom::new(seed);
    reference_roll.next_int_bounded(800);

    let mut timer2 = DespawnTimer {
        no_action_ticks: 601,
    };
    let dist_sqr = NO_DESPAWN_DIST_SQR + (MONSTER_DIST_SQR - NO_DESPAWN_DIST_SQR) / 2.0;
    let decision = check_despawn(
        false,
        Some(dist_sqr),
        MobCategory::Monster,
        false,
        &mut rng_roll,
        &mut timer2,
    );
    assert_eq!(decision, DespawnDecision::Keep);
    assert_eq!(rng_roll.next_int(), reference_roll.next_int());
}

#[test]
fn remove_when_far_away_for_kind_matches_shipped_table() {
    assert!(remove_when_far_away_for_kind(EntityKind::Zombie));
    assert!(!remove_when_far_away_for_kind(EntityKind::Cow));
}

#[test]
fn cow_never_despawns_by_distance() {
    let remove_when_far_away = remove_when_far_away_for_kind(EntityKind::Cow);
    assert!(!remove_when_far_away);

    // Instant-distance scenario.
    let mut rng = RcRandom::new(9);
    let mut timer = DespawnTimer::default();
    let decision = check_despawn(
        false,
        Some(MONSTER_DIST_SQR + 1.0),
        MobCategory::Creature,
        remove_when_far_away,
        &mut rng,
        &mut timer,
    );
    assert_eq!(decision, DespawnDecision::Keep);

    // Random-roll-eligible scenario.
    let mut rng2 = RcRandom::new(10);
    let mut timer2 = DespawnTimer {
        no_action_ticks: 601,
    };
    let dist_sqr = NO_DESPAWN_DIST_SQR + (MONSTER_DIST_SQR - NO_DESPAWN_DIST_SQR) / 2.0;
    let decision2 = check_despawn(
        false,
        Some(dist_sqr),
        MobCategory::Creature,
        remove_when_far_away,
        &mut rng2,
        &mut timer2,
    );
    assert_eq!(decision2, DespawnDecision::Keep);
}
