//! test-matrix: boundaries=waived(codec round-trip suite, no world interaction) orientations=waived(codec round-trip suite, no placement) self=waived(codec round-trip suite, no actor) composition=waived(codec round-trip suite, single-record fixtures) nondefault-state=waived(codec round-trip suite, block-entity records carry no block state)
//! M3.5-B05 acceptance tests (Section 4): exercises the new `BlockEntityCodec` trait
//! wrapper specifically -- does not duplicate `block_entity_nbt_roundtrip.rs`'s existing
//! direct `to_nbt`/`from_nbt` coverage.

use std::sync::Arc;

use rc_chunk_storage::BlockEntityCodec;
use rc_core::BlockPos;
use rc_mechanics::block_entity::chest::ChestBlockEntity;
use rc_mechanics::block_entity::furnace::{
    FURNACE_SLOT_FUEL, FURNACE_SLOT_INPUT, FurnaceBlockEntity,
};
use rc_mechanics::block_entity::hopper::HopperBlockEntity;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::comparator::{self, ComparatorBehavior, NoContainers};

#[test]
fn chest_to_record_from_record_round_trips() {
    let pos = BlockPos::new(4, -60, 8);
    let mut original = ChestBlockEntity::empty();
    original.slots[0] = Some(rc_chunk_storage::ItemStackRecord {
        id: "minecraft:diamond".to_string(),
        count: 3,
        components: None,
    });
    original.slots[26] = Some(rc_chunk_storage::ItemStackRecord {
        id: "minecraft:stick".to_string(),
        count: 64,
        components: None,
    });
    original.custom_name = Some("Treasure Chest".to_string());
    original.lock = Some("minecraft:key".to_string());

    let record = original.to_record(pos);
    assert_eq!(record.id, "minecraft:chest");
    assert_eq!(record.pos, pos);

    let decoded = ChestBlockEntity::from_record(&record).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn furnace_to_record_from_record_round_trips() {
    let pos = BlockPos::new(-2, -60, 9);
    let mut original = FurnaceBlockEntity::empty();
    original.slots[FURNACE_SLOT_INPUT] = Some(rc_chunk_storage::ItemStackRecord {
        id: "minecraft:cobblestone".to_string(),
        count: 12,
        components: None,
    });
    original.slots[FURNACE_SLOT_FUEL] = Some(rc_chunk_storage::ItemStackRecord {
        id: "minecraft:coal".to_string(),
        count: 3,
        components: None,
    });
    original.lit_time_remaining = 800;
    original.lit_total_time = 1600;
    original.cook_time = 120;
    original.cook_time_total = 200;
    original.custom_name = Some("Smeltery".to_string());

    let record = original.to_record(pos);
    assert_eq!(record.id, "minecraft:furnace");
    assert_eq!(record.pos, pos);

    let decoded = FurnaceBlockEntity::from_record(&record).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn hopper_to_record_from_record_round_trips() {
    let pos = BlockPos::new(100, -60, -100);
    let mut original = HopperBlockEntity::empty(Direction::East);
    original.transfer_cooldown = 5;
    original.slots[0] = Some(rc_chunk_storage::ItemStackRecord {
        id: "minecraft:redstone".to_string(),
        count: 32,
        components: None,
    });
    original.custom_name = Some("Feeder".to_string());

    let record = original.to_record(pos);
    assert_eq!(record.id, "minecraft:hopper");
    assert_eq!(record.pos, pos);

    let decoded = HopperBlockEntity::from_record(&record).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn comparator_record_round_trips_output_signal() {
    let pos = BlockPos::new(1, -60, 1);
    let record = comparator::comparator_record(pos, 11);
    assert_eq!(record.id, "minecraft:comparator");
    assert_eq!(record.pos, pos);

    let output = comparator::comparator_output_from_record(&record).unwrap();
    assert_eq!(output, 11);
}

#[test]
fn comparator_seed_output_and_snapshot_outputs_round_trip() {
    let pos = BlockPos::new(2, -60, 2);
    let behavior = ComparatorBehavior::new(Arc::new(NoContainers));

    behavior.seed_output(pos, 7);
    assert!(behavior.snapshot_outputs().contains(&(pos, 7)));

    // Re-seeding updates the existing entry rather than duplicating it.
    behavior.seed_output(pos, 3);
    let outputs = behavior.snapshot_outputs();
    assert_eq!(outputs.iter().filter(|&&(p, _)| p == pos).count(), 1);
    assert!(outputs.contains(&(pos, 3)));
}
