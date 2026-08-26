//! M2-B06 Acceptance tests: `LoadedPlayerRecord`'s `to_nbt`/`from_nbt` round trip —
//! fresh defaults, every modeled field under distinctive values, the three-way
//! `Dimension` mapping, an unrecognized dimension string's schema error, unknown-field
//! preservation across a load-then-save cycle, and byte-level idempotency on an
//! untouched reload.

use rc_chunk_storage::{
    InventorySlotEntry, ItemStackRecord, LoadedPlayerRecord, PlayerAbilities, PlayerSaveData,
};
use rc_core::DimensionId;
use rc_nbt::{SchemaError, borrow, owned};

fn decode(bytes: &[u8]) -> LoadedPlayerRecord {
    let nbt = rc_nbt::read_borrowed(bytes).unwrap();
    let base = match nbt {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    };
    LoadedPlayerRecord::from_nbt(&base.as_compound()).unwrap()
}

fn try_decode(bytes: &[u8]) -> Result<LoadedPlayerRecord, SchemaError> {
    let nbt = rc_nbt::read_borrowed(bytes).unwrap();
    let base = match nbt {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    };
    LoadedPlayerRecord::from_nbt(&base.as_compound())
}

#[test]
fn fresh_default_round_trips_through_save_then_load() {
    let original = LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [8.5, -59.0, -3.25]);
    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", original.to_nbt()));
    let decoded = decode(&bytes);

    assert_eq!(decoded.data, original.data);
}

#[test]
fn every_modeled_field_survives_a_hand_populated_round_trip() {
    let inventory = vec![
        InventorySlotEntry {
            slot: 0,
            item: ItemStackRecord {
                id: "minecraft:diamond_sword".into(),
                count: 1,
                components: Some(owned::NbtCompound::from_values(vec![(
                    "damage".into(),
                    owned::NbtTag::Int(5),
                )])),
            },
        },
        InventorySlotEntry {
            slot: 35,
            item: ItemStackRecord {
                id: "minecraft:cobblestone".into(),
                count: 64,
                components: None,
            },
        },
    ];

    let data = PlayerSaveData {
        pos: [123.5, 64.0, -77.25],
        motion: [0.1, -0.2, 0.3],
        rotation: [45.0, -12.5],
        health: 13.5,
        food_level: 17,
        food_saturation_level: 2.5,
        food_exhaustion_level: 1.75,
        xp_level: 9,
        xp_p: 0.42,
        xp_total: 315,
        inventory,
        selected_item_slot: 4,
        dimension: DimensionId::THE_NETHER,
        player_game_type: 2,
        previous_player_game_type: 0,
        abilities: PlayerAbilities {
            flying: true,
            fly_speed: 0.1,
            instabuild: true,
            invulnerable: true,
            may_build: true,
            may_fly: true,
            walk_speed: 0.2,
        },
    };

    // A `LoadedPlayerRecord` with an empty `base`: `fresh_default` is the only public
    // constructor, and its `base` is always empty; `data` is a plain public field, so
    // overwriting it here is the standard way to get an empty-`base` record carrying
    // arbitrary field values.
    let mut record = LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [0.0, 0.0, 0.0]);
    record.data = data;

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", record.to_nbt()));
    let decoded = decode(&bytes);

    assert_eq!(decoded.data, record.data);
}

#[test]
fn dimension_round_trips_for_all_three_vanilla_values() {
    let cases = [
        (DimensionId::OVERWORLD, "minecraft:overworld"),
        (DimensionId::THE_NETHER, "minecraft:the_nether"),
        (DimensionId::THE_END, "minecraft:the_end"),
    ];

    for (dimension, expected_str) in cases {
        let record = LoadedPlayerRecord::fresh_default(dimension, [0.0, 0.0, 0.0]);
        let nbt = record.to_nbt();

        match nbt.get("Dimension") {
            Some(owned::NbtTag::String(s)) => assert_eq!(s.to_str().as_ref(), expected_str),
            other => panic!("expected String `Dimension`, found {other:?}"),
        }

        let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", nbt));
        let decoded = decode(&bytes);
        assert_eq!(decoded.data.dimension, dimension);
    }
}

#[test]
fn unrecognized_dimension_string_is_a_schema_error() {
    let record = LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [0.0, 0.0, 0.0]);
    let mut nbt = record.to_nbt();
    nbt.remove("Dimension");
    nbt.insert("Dimension", "minecraft:my_custom_dim");

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", nbt));
    let err = try_decode(&bytes).unwrap_err();

    match err {
        SchemaError::InvalidValue { field, .. } => assert_eq!(field, "Dimension"),
        other => panic!("expected SchemaError::InvalidValue, got {other:?}"),
    }
}

#[test]
fn unknown_field_preservation_survives_a_full_load_then_save_cycle() {
    let base_record = LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [0.0, 0.0, 0.0]);
    let mut nbt = base_record.to_nbt();
    nbt.insert("foodTickTimer", 12i32);
    nbt.insert(
        "recipeBook",
        owned::NbtTag::Compound(owned::NbtCompound::new()),
    );
    nbt.insert("Fire", owned::NbtTag::Short(-20));

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", nbt));
    let decoded = decode(&bytes);
    let resaved = decoded.to_nbt();

    assert_eq!(resaved.get("foodTickTimer"), Some(&owned::NbtTag::Int(12)));
    assert_eq!(
        resaved.get("recipeBook"),
        Some(&owned::NbtTag::Compound(owned::NbtCompound::new()))
    );
    assert_eq!(resaved.get("Fire"), Some(&owned::NbtTag::Short(-20)));
}

#[test]
fn byte_level_idempotency_on_an_untouched_reload() {
    let original = LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [1.0, 2.0, 3.0]);
    let first_bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", original.to_nbt()));

    let decoded = decode(&first_bytes);
    let second_bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", decoded.to_nbt()));

    assert_eq!(first_bytes, second_bytes);
}
