//! M4-B04 acceptance tests: `MobCategory`'s seven cap categories (MECH-D34), restated
//! field-precise from `M4-B04-CLAIMS.md`, and `mob_category_for_kind`'s tier-2 mapping.

use rc_mechanics::entity::EntityKind;
use rc_mechanics::spawn::{MobCategory, mob_category_for_kind};

struct Expected {
    category: MobCategory,
    max_instances_per_chunk: u32,
    is_friendly: bool,
    is_persistent: bool,
    despawn_distance_blocks: f64,
}

#[test]
fn mob_category_constants_match_mech_d34() {
    let table = [
        Expected {
            category: MobCategory::Monster,
            max_instances_per_chunk: 70,
            is_friendly: false,
            is_persistent: false,
            despawn_distance_blocks: 128.0,
        },
        Expected {
            category: MobCategory::Creature,
            max_instances_per_chunk: 10,
            is_friendly: true,
            is_persistent: true,
            despawn_distance_blocks: 128.0,
        },
        Expected {
            category: MobCategory::Ambient,
            max_instances_per_chunk: 15,
            is_friendly: true,
            is_persistent: false,
            despawn_distance_blocks: 128.0,
        },
        Expected {
            category: MobCategory::Axolotls,
            max_instances_per_chunk: 5,
            is_friendly: true,
            is_persistent: false,
            despawn_distance_blocks: 128.0,
        },
        Expected {
            category: MobCategory::UndergroundWaterCreature,
            max_instances_per_chunk: 5,
            is_friendly: true,
            is_persistent: false,
            despawn_distance_blocks: 128.0,
        },
        Expected {
            category: MobCategory::WaterCreature,
            max_instances_per_chunk: 5,
            is_friendly: true,
            is_persistent: false,
            despawn_distance_blocks: 128.0,
        },
        Expected {
            category: MobCategory::WaterAmbient,
            max_instances_per_chunk: 20,
            is_friendly: true,
            // M4-B04-CLAIMS.md row 16's own correction: `WaterAmbient` is NOT
            // persistent, contrary to the blueprint's own uncorrected table.
            is_persistent: false,
            despawn_distance_blocks: 64.0,
        },
    ];

    for expected in table {
        assert_eq!(
            expected.category.max_instances_per_chunk(),
            expected.max_instances_per_chunk,
            "{:?} max_instances_per_chunk",
            expected.category
        );
        assert_eq!(
            expected.category.is_friendly(),
            expected.is_friendly,
            "{:?} is_friendly",
            expected.category
        );
        assert_eq!(
            expected.category.is_persistent(),
            expected.is_persistent,
            "{:?} is_persistent",
            expected.category
        );
        assert_eq!(
            expected.category.despawn_distance_blocks(),
            expected.despawn_distance_blocks,
            "{:?} despawn_distance_blocks",
            expected.category
        );
    }
}

#[test]
fn no_despawn_distance_is_32_for_every_category() {
    assert_eq!(MobCategory::no_despawn_distance_blocks(), 32.0);
}

#[test]
fn global_cap_magic_number_is_289() {
    assert_eq!(MobCategory::global_cap_magic_number(), 289);
}

#[test]
fn mob_category_for_kind_matches_tier2_table() {
    assert_eq!(mob_category_for_kind(EntityKind::Item), None);
    assert_eq!(
        mob_category_for_kind(EntityKind::Zombie),
        Some(MobCategory::Monster)
    );
    assert_eq!(
        mob_category_for_kind(EntityKind::Villager),
        Some(MobCategory::Misc)
    );
    assert_eq!(
        mob_category_for_kind(EntityKind::Cow),
        Some(MobCategory::Creature)
    );
}

#[test]
fn misc_is_excluded_from_mob_category_all() {
    assert_eq!(MobCategory::ALL.len(), 7);
    assert!(!MobCategory::ALL.contains(&MobCategory::Misc));
}
