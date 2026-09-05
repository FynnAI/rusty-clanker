//! M4-B08 — `detect_mob_crossings`' own pure acceptance tests (Acceptance tests,
//! `entity_crossing_detection.rs`): no `bevy_ecs::World`, `detect_mob_crossings` called
//! directly.

use bevy_ecs::entity::Entity;
use rc_core::{ChunkKey, DimensionId, RcEntityId};
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::entity::{
    BaseEntity, CowBundle, EntityKind, EntityPayload, EntityUuid, ItemBundle, ItemStackRecord,
    LivingEntity, Pose, VillagerBundle, ZombieBundle, detect_mob_crossings,
};
use rc_messaging::{Address, RegionId};
use rc_registries::generated_v776::registries::item;

fn sample_base(pos: [f64; 3]) -> BaseEntity {
    BaseEntity {
        pos,
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

/// `resolve` returns `Address::Region(2)` for every chunk with `chunk_x >= 0`, and
/// `Address::Region(1)` (== `local`) otherwise.
fn two_region_ownership() -> RegionOwnership {
    RegionOwnership {
        local: Address::Region(RegionId(1)),
        resolve: Box::new(|chunk: ChunkKey| {
            if chunk.x >= 0 {
                Address::Region(RegionId(2))
            } else {
                Address::Region(RegionId(1))
            }
        }),
    }
}

#[test]
fn entity_leaving_local_chunks_is_detected() {
    let ownership = two_region_ownership();
    let entity = Entity::from_bits(1);
    let entities = vec![(
        entity,
        RcEntityId(1),
        10,
        EntityKind::Zombie,
        sample_base([5.0, 64.0, 0.0]),
        Some(sample_living()),
        EntityPayload::Zombie(ZombieBundle),
    )];

    let crossings = detect_mob_crossings(entities, DimensionId::OVERWORLD, &ownership);
    assert_eq!(crossings.len(), 1);
    assert_eq!(crossings[0].destination, RegionId(2));
}

#[test]
fn entity_staying_local_is_never_detected() {
    let ownership = two_region_ownership();
    let entity = Entity::from_bits(1);
    let entities = vec![(
        entity,
        RcEntityId(1),
        10,
        EntityKind::Zombie,
        sample_base([-5.0, 64.0, 0.0]),
        Some(sample_living()),
        EntityPayload::Zombie(ZombieBundle),
    )];

    let crossings = detect_mob_crossings(entities, DimensionId::OVERWORLD, &ownership);
    assert!(crossings.is_empty());
}

#[test]
fn every_field_of_a_detected_crossing_matches_its_source_input() {
    let ownership = two_region_ownership();
    let entity = Entity::from_bits(42);
    let base = sample_base([8.0, 70.0, 3.0]);
    let living = sample_living();
    let payload = EntityPayload::Zombie(ZombieBundle);

    let entities = vec![(
        entity,
        RcEntityId(99),
        42,
        EntityKind::Zombie,
        base.clone(),
        Some(living.clone()),
        payload.clone(),
    )];

    let crossings = detect_mob_crossings(entities, DimensionId::OVERWORLD, &ownership);
    assert_eq!(crossings.len(), 1);
    let crossing = &crossings[0];
    assert_eq!(crossing.entity, entity);
    assert_eq!(crossing.rc_entity_id, RcEntityId(99));
    assert_eq!(crossing.network_entity_id, 42);
    assert_eq!(crossing.kind, EntityKind::Zombie);
    assert_eq!(
        crossing.source_chunk,
        ChunkKey::new(DimensionId::OVERWORLD, 0, 0)
    );
    assert_eq!(crossing.base, base);
    assert_eq!(crossing.living, Some(living));
    assert_eq!(crossing.payload, payload);
}

#[test]
fn non_region_resolved_chunks_are_skipped_not_panicked_on() {
    let ownership = RegionOwnership {
        local: Address::Region(RegionId(1)),
        resolve: Box::new(|chunk: ChunkKey| Address::Chunk(chunk)),
    };
    let entity = Entity::from_bits(1);
    let entities = vec![(
        entity,
        RcEntityId(1),
        10,
        EntityKind::Cow,
        sample_base([5.0, 64.0, 0.0]),
        Some(sample_living()),
        EntityPayload::Cow(CowBundle),
    )];

    let crossings = detect_mob_crossings(entities, DimensionId::OVERWORLD, &ownership);
    assert!(
        crossings.is_empty(),
        "non-Region resolutions must never panic and must never produce a crossing"
    );
}

#[test]
fn multiple_crossing_entities_are_all_detected_independently() {
    let ownership = RegionOwnership {
        local: Address::Region(RegionId(1)),
        resolve: Box::new(|chunk: ChunkKey| {
            if chunk.x >= 16 {
                Address::Region(RegionId(3))
            } else if chunk.x >= 0 {
                Address::Region(RegionId(2))
            } else {
                Address::Region(RegionId(1))
            }
        }),
    };

    let local_entity = Entity::from_bits(1);
    let crossing_to_2 = Entity::from_bits(2);
    let crossing_to_3 = Entity::from_bits(3);

    let entities = vec![
        (
            local_entity,
            RcEntityId(1),
            1,
            EntityKind::Cow,
            sample_base([-5.0, 64.0, 0.0]),
            Some(sample_living()),
            EntityPayload::Cow(CowBundle),
        ),
        (
            crossing_to_2,
            RcEntityId(2),
            2,
            EntityKind::Villager,
            sample_base([5.0, 64.0, 0.0]),
            Some(sample_living()),
            EntityPayload::Villager(VillagerBundle {
                villager_data: rc_mechanics::entity::metadata::VillagerData {
                    villager_type: rc_registries::generated_v776::registries::villager_type::PLAINS,
                    profession:
                        rc_registries::generated_v776::registries::villager_profession::NONE,
                    level: 1,
                },
            }),
        ),
        (
            crossing_to_3,
            RcEntityId(3),
            3,
            EntityKind::Item,
            sample_base([300.0, 64.0, 0.0]),
            None,
            EntityPayload::Item(ItemBundle {
                item: ItemStackRecord {
                    item_id: item::STONE,
                    count: 1,
                    components: None,
                },
                pickup_delay_ticks: 0,
                age_ticks: 0,
            }),
        ),
    ];

    let crossings = detect_mob_crossings(entities, DimensionId::OVERWORLD, &ownership);
    assert_eq!(crossings.len(), 2);
    let dest_by_entity: std::collections::HashMap<Entity, RegionId> = crossings
        .iter()
        .map(|c| (c.entity, c.destination))
        .collect();
    assert_eq!(dest_by_entity.get(&crossing_to_2), Some(&RegionId(2)));
    assert_eq!(dest_by_entity.get(&crossing_to_3), Some(&RegionId(3)));
    assert!(!dest_by_entity.contains_key(&local_entity));
}
