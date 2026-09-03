//! `EntitySnapshot` acceptance tests (M4-B01 Deliverables, `entity::snapshot`).

use rc_mechanics::entity::{
    BaseEntity, CowBundle, ENTITY_SNAPSHOT_FORMAT_VERSION, EntityKind, EntityPayload, EntityUuid,
    ItemBundle, ItemStackRecord, LivingEntity, Pose, SnapshotError, VillagerBundle, ZombieBundle,
    deserialize_entity_snapshot, serialize_entity_snapshot,
};
use rc_registries::generated_v776::registries::{item, villager_profession, villager_type};

fn sample_base_entity() -> BaseEntity {
    BaseEntity {
        pos: [1.0, 2.0, 3.0],
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

fn sample_living() -> LivingEntity {
    LivingEntity {
        hand_states: 0,
        health: 20.0,
        arrow_count: 0,
        stinger_count: 0,
        sleeping_bed_pos: None,
    }
}

#[test]
fn snapshot_round_trips_for_every_tier_2_kind() {
    let cases: Vec<(EntityKind, Option<LivingEntity>, EntityPayload)> = vec![
        (
            EntityKind::Item,
            None,
            EntityPayload::Item(ItemBundle {
                item: ItemStackRecord {
                    item_id: item::STONE,
                    count: 3,
                    components: None,
                },
                pickup_delay_ticks: 0,
                age_ticks: 0,
            }),
        ),
        (
            EntityKind::Zombie,
            Some(sample_living()),
            EntityPayload::Zombie(ZombieBundle),
        ),
        (
            EntityKind::Villager,
            Some(sample_living()),
            EntityPayload::Villager(VillagerBundle {
                villager_data: rc_mechanics::entity::metadata::VillagerData {
                    villager_type: villager_type::PLAINS,
                    profession: villager_profession::NONE,
                    level: 1,
                },
            }),
        ),
        (
            EntityKind::Cow,
            Some(sample_living()),
            EntityPayload::Cow(CowBundle),
        ),
    ];

    for (kind, living, payload) in cases {
        let base = sample_base_entity();
        let bytes = serialize_entity_snapshot(kind, &base, living.as_ref(), &payload);
        let decoded = deserialize_entity_snapshot(&bytes).expect("decode must succeed");

        assert_eq!(decoded.entity_kind, kind);
        assert_eq!(decoded.format_version, ENTITY_SNAPSHOT_FORMAT_VERSION);

        for component in &decoded.components {
            match component.kind {
                rc_mechanics::entity::ComponentKind::Base => {
                    let decoded_base: BaseEntity =
                        postcard::from_bytes(&component.bytes).expect("BaseEntity decode");
                    assert_eq!(decoded_base, base);
                }
                rc_mechanics::entity::ComponentKind::Living => {
                    let decoded_living: LivingEntity =
                        postcard::from_bytes(&component.bytes).expect("LivingEntity decode");
                    assert_eq!(Some(decoded_living), living);
                }
                rc_mechanics::entity::ComponentKind::Item => {
                    let decoded_item: ItemBundle =
                        postcard::from_bytes(&component.bytes).expect("ItemBundle decode");
                    assert_eq!(EntityPayload::Item(decoded_item), payload);
                }
                rc_mechanics::entity::ComponentKind::Zombie => {
                    let decoded_zombie: ZombieBundle =
                        postcard::from_bytes(&component.bytes).expect("ZombieBundle decode");
                    assert_eq!(EntityPayload::Zombie(decoded_zombie), payload);
                }
                rc_mechanics::entity::ComponentKind::Villager => {
                    let decoded_villager: VillagerBundle =
                        postcard::from_bytes(&component.bytes).expect("VillagerBundle decode");
                    assert_eq!(EntityPayload::Villager(decoded_villager), payload);
                }
                rc_mechanics::entity::ComponentKind::Cow => {
                    let decoded_cow: CowBundle =
                        postcard::from_bytes(&component.bytes).expect("CowBundle decode");
                    assert_eq!(EntityPayload::Cow(decoded_cow), payload);
                }
            }
        }
    }
}

#[test]
fn unsupported_format_version_is_rejected_not_silently_misread() {
    let payload = rc_mechanics::entity::SnapshotPayload {
        format_version: ENTITY_SNAPSHOT_FORMAT_VERSION + 1,
        entity_kind: EntityKind::Cow,
        components: Vec::new(),
    };
    let bytes = postcard::to_allocvec(&payload).expect("postcard encode must succeed");

    let result = deserialize_entity_snapshot(&bytes);
    match result {
        Err(SnapshotError::UnsupportedFormatVersion { found, supported }) => {
            assert_eq!(found, ENTITY_SNAPSHOT_FORMAT_VERSION + 1);
            assert_eq!(supported, ENTITY_SNAPSHOT_FORMAT_VERSION);
        }
        other => panic!("expected UnsupportedFormatVersion, got {other:?}"),
    }
}

#[test]
fn malformed_bytes_never_panic() {
    let result = deserialize_entity_snapshot(&[0xFF, 0x00, 0x01]);
    assert!(result.is_err());
}
