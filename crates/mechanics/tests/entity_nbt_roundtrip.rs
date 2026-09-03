//! Entity NBT persistence acceptance tests (M4-B01 Deliverables, `entity::nbt`) — the
//! patch-over-original pattern, per tier-2 kind, restated for entities.

use rc_mechanics::entity::{
    AiSystemKind, BaseEntity, CowBundle, EntityKind, EntityPayload, EntityRecord, ItemBundle,
    ItemStackRecord, LivingEntity, MobMarker, Pose, VillagerBundle, ZombieBundle,
};
use rc_nbt::{NbtPath, borrow, owned};
use rc_registries::generated_v776::registries::{item, villager_profession, villager_type};

fn sample_base_entity(uuid: u128) -> BaseEntity {
    BaseEntity {
        pos: [10.5, 64.0, -20.25],
        velocity: [0.1, -0.05, 0.2],
        rotation: [90.0, 10.0],
        fall_distance: 1.5,
        fire_ticks: 0,
        status_flags: 0,
        air_ticks: 300,
        on_ground: true,
        invulnerable: false,
        portal_cooldown: 0,
        uuid: rc_mechanics::entity::EntityUuid(uuid),
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

fn sample_living_entity(health: f32) -> LivingEntity {
    LivingEntity {
        hand_states: 0,
        health,
        arrow_count: 0,
        stinger_count: 0,
        sleeping_bed_pos: None,
    }
}

fn round_trip(record: &EntityRecord, kind: EntityKind) -> EntityRecord {
    let compound = record.to_nbt(kind);
    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", compound));
    let read = rc_nbt::read_borrowed_strict(&bytes).expect("read_borrowed_strict must succeed");
    let base = match read {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected a non-empty document"),
    };
    let borrowed = base.as_compound();
    EntityRecord::from_nbt(&borrowed, &NbtPath::root(), kind).expect("from_nbt must succeed")
}

#[test]
fn zombie_round_trips() {
    let record = EntityRecord {
        base: None,
        entity: sample_base_entity(0x1111_2222_3333_4444_5555_6666_7777_8888),
        living: Some(sample_living_entity(14.0)),
        mob: Some(MobMarker {
            ai_system: AiSystemKind::GoalSelector,
            persistence_required: true,
            can_pick_up_loot: false,
        }),
        payload: EntityPayload::Zombie(ZombieBundle),
    };

    let reconstructed = round_trip(&record, EntityKind::Zombie);
    assert_eq!(reconstructed.entity, record.entity);
    assert_eq!(reconstructed.living, record.living);
    assert_eq!(reconstructed.mob, record.mob);
    assert_eq!(reconstructed.payload, record.payload);
}

#[test]
fn villager_round_trips() {
    let record = EntityRecord {
        base: None,
        entity: sample_base_entity(0x2222_3333_4444_5555_6666_7777_8888_9999),
        living: Some(sample_living_entity(20.0)),
        mob: Some(MobMarker {
            ai_system: AiSystemKind::Brain,
            persistence_required: false,
            can_pick_up_loot: true,
        }),
        payload: EntityPayload::Villager(VillagerBundle {
            villager_data: rc_mechanics::entity::metadata::VillagerData {
                villager_type: villager_type::PLAINS,
                profession: villager_profession::NONE,
                level: 1,
            },
        }),
    };

    let reconstructed = round_trip(&record, EntityKind::Villager);
    assert_eq!(reconstructed.entity, record.entity);
    assert_eq!(reconstructed.living, record.living);
    assert_eq!(reconstructed.mob, record.mob);
    assert_eq!(reconstructed.payload, record.payload);
}

#[test]
fn cow_round_trips() {
    let record = EntityRecord {
        base: None,
        entity: sample_base_entity(0x3333_4444_5555_6666_7777_8888_9999_aaaa),
        living: Some(sample_living_entity(10.0)),
        mob: Some(MobMarker {
            ai_system: AiSystemKind::GoalSelector,
            persistence_required: false,
            can_pick_up_loot: false,
        }),
        payload: EntityPayload::Cow(CowBundle),
    };

    let reconstructed = round_trip(&record, EntityKind::Cow);
    assert_eq!(reconstructed.entity, record.entity);
    assert_eq!(reconstructed.living, record.living);
    assert_eq!(reconstructed.mob, record.mob);
    assert_eq!(reconstructed.payload, record.payload);
}

#[test]
fn item_round_trips() {
    let record = EntityRecord {
        base: None,
        entity: sample_base_entity(0x4444_5555_6666_7777_8888_9999_aaaa_bbbb),
        living: None,
        mob: None,
        payload: EntityPayload::Item(ItemBundle {
            item: ItemStackRecord {
                item_id: item::STONE,
                count: 5,
                components: None,
            },
            pickup_delay_ticks: 10,
            age_ticks: 0,
        }),
    };

    let reconstructed = round_trip(&record, EntityKind::Item);
    assert_eq!(reconstructed.entity, record.entity);
    assert_eq!(reconstructed.living, None);
    assert_eq!(reconstructed.mob, None);
    assert_eq!(reconstructed.payload, record.payload);
}

#[test]
fn unmodeled_fields_survive_a_load_then_resave_cycle() {
    let mut hand_built =
        sample_base_entity(0x5555_6666_7777_8888_9999_aaaa_bbbb_cccc).to_nbt_hand_built();
    // `Tags` is not modeled by this blueprint (Context) -- an extra, unmodeled key.
    hand_built.insert(
        "Tags",
        owned::NbtTag::List(owned::NbtList::String(vec!["custom_tag".into()])),
    );
    hand_built.insert("id", "minecraft:zombie");
    // `Mob`-rung fields, since kind `Zombie` expects them.
    hand_built.insert("CanPickUpLoot", false);
    hand_built.insert("PersistenceRequired", false);
    // `LivingEntity`'s own required field.
    hand_built.insert("Health", 20.0f32);

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", hand_built));
    let read = rc_nbt::read_borrowed_strict(&bytes).expect("read_borrowed_strict must succeed");
    let base = match read {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected a non-empty document"),
    };
    let borrowed = base.as_compound();
    let record = EntityRecord::from_nbt(&borrowed, &NbtPath::root(), EntityKind::Zombie)
        .expect("from_nbt must succeed");
    assert!(record.base.is_some());

    let resaved = record.to_nbt(EntityKind::Zombie);
    let tags = resaved.get("Tags").expect("Tags must survive the resave");
    match tags {
        owned::NbtTag::List(owned::NbtList::String(values)) => {
            assert_eq!(values.len(), 1);
            assert_eq!(values[0].to_str().as_ref(), "custom_tag");
        }
        other => panic!("expected List<String>, got {other:?}"),
    }
}

#[test]
fn mob_persistence_fields_default_to_false_when_absent_from_a_loaded_compound() {
    let mut hand_built =
        sample_base_entity(0x6666_7777_8888_9999_aaaa_bbbb_cccc_dddd).to_nbt_hand_built();
    hand_built.insert("id", "minecraft:zombie");
    hand_built.insert("Health", 20.0f32);
    // `CanPickUpLoot`/`PersistenceRequired` deliberately omitted.

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", hand_built));
    let read = rc_nbt::read_borrowed_strict(&bytes).expect("read_borrowed_strict must succeed");
    let base = match read {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected a non-empty document"),
    };
    let borrowed = base.as_compound();
    let record = EntityRecord::from_nbt(&borrowed, &NbtPath::root(), EntityKind::Zombie)
        .expect("from_nbt must succeed");

    assert_eq!(
        record.mob,
        Some(MobMarker {
            ai_system: AiSystemKind::GoalSelector,
            persistence_required: false,
            can_pick_up_loot: false,
        })
    );
}

#[test]
fn custom_name_round_trips_the_compound_form_verbatim() {
    let mut rich_name = owned::NbtCompound::new();
    rich_name.insert("text", "Bob");
    rich_name.insert("bold", true);

    let mut hand_built =
        sample_base_entity(0x7777_8888_9999_aaaa_bbbb_cccc_dddd_eeee).to_nbt_hand_built();
    hand_built.insert("id", "minecraft:zombie");
    hand_built.insert("Health", 20.0f32);
    hand_built.insert("CanPickUpLoot", false);
    hand_built.insert("PersistenceRequired", false);
    hand_built.insert("CustomName", owned::NbtTag::Compound(rich_name.clone()));

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", hand_built));
    let read = rc_nbt::read_borrowed_strict(&bytes).expect("read_borrowed_strict must succeed");
    let base = match read {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected a non-empty document"),
    };
    let borrowed = base.as_compound();
    let record = EntityRecord::from_nbt(&borrowed, &NbtPath::root(), EntityKind::Zombie)
        .expect("from_nbt must succeed");

    let resaved = record.to_nbt(EntityKind::Zombie);
    let custom_name = resaved
        .get("CustomName")
        .expect("CustomName must round-trip");
    match custom_name {
        owned::NbtTag::Compound(c) => assert_eq!(c, &rich_name),
        other => panic!("expected Compound, got {other:?}"),
    }
}

#[test]
fn custom_name_plain_text_accessor_returns_some_only_for_the_bare_string_form() {
    let mut plain = sample_base_entity(0x8888_9999_aaaa_bbbb_cccc_dddd_eeee_ffff);
    plain.custom_name = Some(owned::NbtTag::String("Bob".into()));
    assert_eq!(plain.custom_name_text(), Some("Bob"));

    let mut rich = sample_base_entity(0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0000);
    let mut compound = owned::NbtCompound::new();
    compound.insert("text", "Bob");
    rich.custom_name = Some(owned::NbtTag::Compound(compound));
    assert_eq!(rich.custom_name_text(), None);
}

/// Test-local helper: `BaseEntity::write_nbt_fields` against a fresh compound, so
/// tests 5-7 can hand-build a compound that starts from this blueprint's own real
/// modeled-field encoding rather than re-deriving it by hand.
trait ToNbtHandBuilt {
    fn to_nbt_hand_built(&self) -> owned::NbtCompound;
}
impl ToNbtHandBuilt for BaseEntity {
    fn to_nbt_hand_built(&self) -> owned::NbtCompound {
        use rc_mechanics::entity::EntityNbtFields;
        let mut out = owned::NbtCompound::new();
        self.write_nbt_fields(&mut out);
        out
    }
}
