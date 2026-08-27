//! M3-B06 — block-entity NBT round-trip acceptance tests (Acceptance tests' own
//! `block_entity_nbt_roundtrip.rs` section, the task's own required acceptance category).

use rc_chunk_storage::ItemStackRecord;
use rc_core::BlockPos;
use rc_mechanics::block_entity::chest::ChestBlockEntity;
use rc_mechanics::block_entity::furnace::{
    FURNACE_SLOT_FUEL, FURNACE_SLOT_INPUT, FurnaceBlockEntity,
};
use rc_mechanics::block_entity::hopper::HopperBlockEntity;
use rc_mechanics::direction::Direction;
use rc_nbt::{SchemaError, borrow, owned};

fn to_bytes(compound: owned::NbtCompound) -> Vec<u8> {
    rc_nbt::write_owned(&owned::BaseNbt::new("", compound))
}

fn with_borrowed_compound<R>(bytes: &[u8], f: impl FnOnce(&borrow::NbtCompound<'_, '_>) -> R) -> R {
    let nbt = rc_nbt::read_borrowed(bytes).unwrap();
    let base = match nbt {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    };
    f(&base.as_compound())
}

fn item(id: &str, count: i32, components: Option<owned::NbtCompound>) -> Option<ItemStackRecord> {
    Some(ItemStackRecord {
        id: id.to_string(),
        count,
        components,
    })
}

#[test]
fn chest_empty_round_trips() {
    let pos = BlockPos::new(10, -20, 30);
    let original = ChestBlockEntity::empty();
    let bytes = to_bytes(original.to_nbt(pos));

    let (decoded_pos, decoded) =
        with_borrowed_compound(&bytes, |c| ChestBlockEntity::from_nbt(c).unwrap());
    assert_eq!(decoded_pos, pos);
    assert_eq!(decoded, original);
}

#[test]
fn chest_with_items_and_custom_name_round_trips() {
    let pos = BlockPos::new(1, 2, 3);
    let mut original = ChestBlockEntity::empty();
    original.slots[0] = item("minecraft:diamond", 3, None);
    original.slots[14] = item(
        "minecraft:enchanted_book",
        1,
        Some(owned::NbtCompound::from_values(vec![(
            "damage".into(),
            owned::NbtTag::Int(5),
        )])),
    );
    original.slots[26] = item("minecraft:stick", 64, None);
    original.custom_name = Some("Treasure Chest".to_string());
    original.lock = Some("minecraft:key".to_string());

    let bytes = to_bytes(original.to_nbt(pos));
    let (decoded_pos, decoded) =
        with_borrowed_compound(&bytes, |c| ChestBlockEntity::from_nbt(c).unwrap());

    assert_eq!(decoded_pos, pos);
    assert_eq!(decoded, original);
    // The mid-array slot round-trips to exactly the same index, not renumbered.
    assert_eq!(decoded.slots[14], original.slots[14]);
}

#[test]
fn furnace_with_active_burn_round_trips() {
    let pos = BlockPos::new(-5, 64, 5);
    let mut original = FurnaceBlockEntity::empty();
    original.slots[FURNACE_SLOT_INPUT] = item("minecraft:cobblestone", 12, None);
    original.slots[FURNACE_SLOT_FUEL] = item("minecraft:coal", 3, None);
    original.lit_time_remaining = 800;
    // `lit_total_time` is not part of vanilla's own furnace block-entity NBT schema (Context)
    // and is therefore not written; `from_nbt` reconstructs it as equal to the freshly-decoded
    // `lit_time_remaining` (a safe, harmless placeholder -- see `furnace.rs`'s own doc comment)
    // -- this fixture sets the same value up front so the round-trip equality below is exact
    // under that documented convention, not by coincidence.
    original.lit_total_time = 800;
    original.cook_time = 120;
    original.cook_time_total = 200;
    // Likewise not persisted (Context's own NBT tag-name table lists only `BurnTime`/
    // `CookTime`/`CookTimeTotal`) -- `from_nbt` always decodes `None`.
    original.cooking_recipe_output_id = None;

    let bytes = to_bytes(original.to_nbt(pos));
    let (decoded_pos, decoded) =
        with_borrowed_compound(&bytes, |c| FurnaceBlockEntity::from_nbt(c).unwrap());

    assert_eq!(decoded_pos, pos);
    assert_eq!(decoded, original);
}

#[test]
fn hopper_with_cooldown_and_facing_round_trips() {
    let pos = BlockPos::new(100, 0, -100);
    let mut original = HopperBlockEntity::empty(Direction::East);
    original.transfer_cooldown = 5;
    original.slots[0] = item("minecraft:redstone", 32, None);
    original.slots[4] = item("minecraft:hopper", 1, None);
    original.custom_name = Some("Feeder".to_string());

    let bytes = to_bytes(original.to_nbt(pos));
    let (decoded_pos, decoded) =
        with_borrowed_compound(&bytes, |c| HopperBlockEntity::from_nbt(c).unwrap());

    assert_eq!(decoded_pos, pos);
    assert_eq!(decoded, original);
}

#[test]
fn malformed_items_entry_is_rejected_not_silently_dropped() {
    // Hand-constructed: an Items entry missing the required `id` field.
    let mut bad_entry = owned::NbtCompound::new();
    bad_entry.insert("Slot", 0i8);
    bad_entry.insert("count", 1i32);

    let mut compound = owned::NbtCompound::new();
    compound.insert("id", "minecraft:chest");
    compound.insert("x", 0i32);
    compound.insert("y", 0i32);
    compound.insert("z", 0i32);
    compound.insert(
        "Items",
        owned::NbtTag::List(owned::NbtList::Compound(vec![bad_entry])),
    );

    let bytes = to_bytes(compound);
    let result: Result<_, SchemaError> = with_borrowed_compound(&bytes, ChestBlockEntity::from_nbt);

    assert!(matches!(
        result,
        Err(SchemaError::MissingField { field: "id", .. })
    ));
}
