//! M4-B08 — the `EntitySnapshot.component_data` discriminator-byte wrapper's own
//! acceptance tests (Acceptance tests, `entity_transfer_snapshot_wrapping.rs`).

use rc_mechanics::entity::{
    AiSystemKind, BaseEntity, CowBundle, EntityKind, EntityPayload, EntityUuid, ItemBundle,
    ItemStackRecord, LivingEntity, Pose, TRANSFER_PAYLOAD_KIND_MOB, VillagerBundle, ZombieBundle,
    build_mob_entity_snapshot, default_mob_marker, try_decode_mob_snapshot,
};
use rc_registries::generated_v776::registries::{item, villager_profession, villager_type};

fn sample_base() -> BaseEntity {
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
fn mob_snapshot_round_trips_through_the_discriminator_wrapper() {
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
        let base = sample_base();
        let snapshot = build_mob_entity_snapshot(
            rc_core::RcEntityId(7),
            rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
            123,
            kind,
            &base,
            living.as_ref(),
            &payload,
        );

        let decoded = try_decode_mob_snapshot(&snapshot.component_data)
            .expect("a TRANSFER_PAYLOAD_KIND_MOB leading byte must decode to Some")
            .expect("well-formed envelope bytes must decode without error");
        let (network_entity_id, decoded_payload) = decoded;
        assert_eq!(network_entity_id, 123);
        assert_eq!(decoded_payload.entity_kind, kind);

        for component in &decoded_payload.components {
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
fn non_mob_leading_byte_returns_none() {
    let component_data = vec![1u8, 9, 9, 9];
    assert!(try_decode_mob_snapshot(&component_data).is_none());
}

#[test]
fn malformed_mob_envelope_bytes_never_panic() {
    let component_data = vec![TRANSFER_PAYLOAD_KIND_MOB, 0xFF, 0x00];
    let result = try_decode_mob_snapshot(&component_data);
    assert!(matches!(result, Some(Err(_))));
}

#[test]
fn network_entity_id_survives_the_wrapper_unchanged() {
    let base = sample_base();
    let snapshot = build_mob_entity_snapshot(
        rc_core::RcEntityId(1),
        rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
        12345,
        EntityKind::Cow,
        &base,
        Some(&sample_living()),
        &EntityPayload::Cow(CowBundle),
    );
    let (network_entity_id, _) = try_decode_mob_snapshot(&snapshot.component_data)
        .expect("Some")
        .expect("Ok");
    assert_eq!(network_entity_id, 12345);
}

#[test]
fn default_mob_marker_matches_the_tier_2_kind_table() {
    let zombie = default_mob_marker(EntityKind::Zombie).expect("Zombie is a Mob-rung kind");
    assert_eq!(zombie.ai_system, AiSystemKind::GoalSelector);

    let villager = default_mob_marker(EntityKind::Villager).expect("Villager is a Mob-rung kind");
    assert_eq!(villager.ai_system, AiSystemKind::Brain);

    let cow = default_mob_marker(EntityKind::Cow).expect("Cow is a Mob-rung kind");
    assert_eq!(cow.ai_system, AiSystemKind::GoalSelector);

    assert_eq!(default_mob_marker(EntityKind::Item), None);
}
