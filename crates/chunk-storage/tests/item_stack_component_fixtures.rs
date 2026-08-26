//! M2-B06 Acceptance tests: a synthetic golden fixture proving a realistically-shaped
//! `components` compound (nested compounds, not just flat primitives) round-trips
//! fully opaque through a real `LoadedPlayerRecord` cycle — this crate never needs to
//! know these keys mean "damage" or "sharpness 2" for the test to be meaningful.

use rc_core::DimensionId;
use rc_chunk_storage::{InventorySlotEntry, ItemStackRecord, LoadedPlayerRecord};
use rc_nbt::{borrow, owned};

fn decode(bytes: &[u8]) -> LoadedPlayerRecord {
    let nbt = rc_nbt::read_borrowed(bytes).unwrap();
    let base = match nbt {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    };
    LoadedPlayerRecord::from_nbt(&base.as_compound()).unwrap()
}

#[test]
fn diamond_sword_with_enchantments_and_damage_round_trips() {
    let mut levels = owned::NbtCompound::new();
    levels.insert("minecraft:sharpness", 2i32);

    let mut enchantments = owned::NbtCompound::new();
    enchantments.insert("levels", owned::NbtTag::Compound(levels));

    let mut components = owned::NbtCompound::new();
    components.insert("minecraft:damage", 3i32);
    components.insert("minecraft:enchantments", owned::NbtTag::Compound(enchantments));

    let item = ItemStackRecord {
        id: "minecraft:diamond_sword".into(),
        count: 1,
        components: Some(components.clone()),
    };

    let mut record = LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [0.0, 0.0, 0.0]);
    record.data.inventory = vec![InventorySlotEntry { slot: 0, item }];

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", record.to_nbt()));
    let decoded = decode(&bytes);

    let decoded_components = decoded.data.inventory[0]
        .item
        .components
        .as_ref()
        .expect("components must survive round trip");
    assert_eq!(decoded_components, &components);
}

#[ignore = "requires a vanilla-produced players/data/<uuid>.dat sample from rc-test-harness (TEST-D7), not yet implemented — see the M2-B06 implementation report's open problems"]
#[test]
fn decodes_real_vanilla_player_dat_without_error() {
    let path = std::path::Path::new("oracle/26.2/harness/samples/players/data/sample.dat");
    let bytes = std::fs::read(path).expect("sample not present — see #[ignore] reason");
    let nbt = rc_nbt::read_gzip_owned(&bytes).expect("must decode a real vanilla player record cleanly");
    let _ = nbt;
    // Further field-level assertions deferred to whichever future blueprint first
    // wires rc-test-harness — this test's own job, today, is to exist and be
    // honestly skipped, exactly M2-B02's own precedent.
}
