//! M4-B04 acceptance tests: the superflat spawn list, `pick_weighted`
//! (`WeightedList::getRandom`, M4-B04-CLAIMS.md row 42), and `default_max_health`.

use rc_mechanics::entity::EntityKind;
use rc_mechanics::random::RcRandom;
use rc_mechanics::spawn::{SpawnerEntry, default_max_health, pick_weighted};

#[test]
fn pick_weighted_returns_none_for_empty_list() {
    let mut rng = RcRandom::new(42);
    let mut reference = RcRandom::new(42);

    let result = pick_weighted(&[], &mut rng);
    assert!(result.is_none());

    // Zero RNG calls consumed: `rng`'s state must still match a freshly-seeded reference
    // instance that has drawn nothing.
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn pick_weighted_single_entry_always_selected_consumes_exactly_one_call() {
    let entry = SpawnerEntry {
        kind: EntityKind::Zombie,
        category: rc_mechanics::spawn::MobCategory::Monster,
        weight: 100,
        min_count: 4,
        max_count: 4,
    };
    let mut rng = RcRandom::new(7);
    let mut reference = RcRandom::new(7);
    reference.next_int_bounded(100); // the one call `pick_weighted` itself consumes

    let picked = pick_weighted(&[entry], &mut rng).expect("single entry always selected");
    assert_eq!(picked, entry);
    // Exactly one call consumed -- both streams now agree on the very next draw.
    assert_eq!(rng.next_int(), reference.next_int());
}

#[test]
fn pick_weighted_is_deterministic_for_a_fixed_seed() {
    let entries = [
        SpawnerEntry {
            kind: EntityKind::Zombie,
            category: rc_mechanics::spawn::MobCategory::Monster,
            weight: 100,
            min_count: 4,
            max_count: 4,
        },
        SpawnerEntry {
            kind: EntityKind::Cow,
            category: rc_mechanics::spawn::MobCategory::Creature,
            weight: 8,
            min_count: 4,
            max_count: 4,
        },
    ];

    let mut a = RcRandom::new(1234);
    let mut b = RcRandom::new(1234);
    assert_eq!(
        pick_weighted(&entries, &mut a),
        pick_weighted(&entries, &mut b)
    );
}

#[test]
fn default_max_health_zombie_and_cow() {
    assert_eq!(default_max_health(EntityKind::Zombie), 20.0);
    assert_eq!(default_max_health(EntityKind::Cow), 10.0);
}
