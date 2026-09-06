//! M4-B05 acceptance tests: `FixedTierTwoLoot`, the bounded, hand-authored loot-drop seam
//! implementation.

use rc_mechanics::combat::death::{EntityLootProvider, FixedTierTwoLoot};
use rc_mechanics::entity::EntityKind;
use rc_mechanics::random::RcRandom;
use rc_registries::generated_v776::registries::item;

#[test]
fn fixed_tier_two_loot_zombie_bounded_range() {
    let provider = FixedTierTwoLoot;
    for seed in 1..=200_i64 {
        let mut rng = RcRandom::new(seed);
        let drops = provider.roll_death_loot(EntityKind::Zombie, &mut rng);
        assert!(
            drops.len() <= 1,
            "at most one rotten-flesh stack, seed {seed}"
        );
        for stack in &drops {
            assert_eq!(stack.item_id, item::ROTTEN_FLESH);
            assert!(
                stack.count >= 1 && stack.count <= 2,
                "seed {seed}: count {}",
                stack.count
            );
        }
    }
}

#[test]
fn fixed_tier_two_loot_villager_is_empty() {
    let provider = FixedTierTwoLoot;
    for seed in [1_i64, 42, 999] {
        let mut rng = RcRandom::new(seed);
        let drops = provider.roll_death_loot(EntityKind::Villager, &mut rng);
        assert!(drops.is_empty());
    }
}

#[test]
fn fixed_tier_two_loot_is_deterministic_given_seed() {
    let provider = FixedTierTwoLoot;
    let mut rng_a = RcRandom::new(7);
    let mut rng_b = RcRandom::new(7);
    let a = provider.roll_death_loot(EntityKind::Cow, &mut rng_a);
    let b = provider.roll_death_loot(EntityKind::Cow, &mut rng_b);
    assert_eq!(a, b);
}
