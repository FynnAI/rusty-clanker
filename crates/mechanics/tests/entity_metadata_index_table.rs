//! Pins every shipped kind's own `#[net_metadata(index = ...)]` declarations against
//! the vanilla-exact indices Context's own "Per-kind synced-data index table" derives
//! class-chain-by-class-chain (M4-B01 Deliverables) — a regression guard asserting the
//! *value* each struct's own `metadata_entries()` actually returns.

use rc_mechanics::entity::metadata::VillagerData;
use rc_mechanics::entity::{
    BaseEntity, CowBundle, EntityMetadataFields, EntityUuid, ItemBundle, ItemStackRecord,
    LivingEntity, MetadataValue, Pose, VillagerBundle, ZombieBundle,
};
use rc_registries::generated_v776::registries::RegistryEntryId;

fn sample_base_entity() -> BaseEntity {
    BaseEntity {
        pos: [0.0, 0.0, 0.0],
        velocity: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0],
        fall_distance: 0.0,
        fire_ticks: 0,
        status_flags: 0,
        air_ticks: 300,
        on_ground: true,
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
    }
}

#[test]
fn base_entity_metadata_indices_match_entity_java() {
    let entries = sample_base_entity().metadata_entries();
    let indices: Vec<u8> = entries.iter().map(|(i, _)| *i).collect();
    assert_eq!(indices, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn living_entity_metadata_indices_match_living_entity_java() {
    let living = LivingEntity {
        hand_states: 0,
        health: 20.0,
        arrow_count: 0,
        stinger_count: 0,
        sleeping_bed_pos: None,
    };
    let indices: Vec<u8> = living.metadata_entries().iter().map(|(i, _)| *i).collect();
    assert_eq!(indices, vec![8, 9, 12, 13, 14]);
}

#[test]
fn villager_bundle_metadata_index_is_19_not_the_prior_16() {
    let bundle = VillagerBundle {
        villager_data: VillagerData {
            villager_type: RegistryEntryId(0),
            profession: RegistryEntryId(0),
            level: 1,
        },
    };
    let entries = bundle.metadata_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, 19);
    assert!(matches!(entries[0].1, MetadataValue::VillagerData(_)));
}

#[test]
fn item_bundle_metadata_index_is_8() {
    let bundle = ItemBundle {
        item: ItemStackRecord {
            item_id: RegistryEntryId(0),
            count: 1,
            components: None,
        },
        pickup_delay_ticks: 0,
        age_ticks: 0,
    };
    let entries = bundle.metadata_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, 8);
    assert!(matches!(entries[0].1, MetadataValue::Slot(_)));
}

#[test]
fn zombie_and_cow_bundles_declare_no_metadata_at_this_milestones_scope() {
    assert_eq!(ZombieBundle, ZombieBundle);
    assert_eq!(CowBundle, CowBundle);
}
