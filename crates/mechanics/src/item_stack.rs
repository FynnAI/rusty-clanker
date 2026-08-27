//! One occupied inventory slot's NBT shape, reused verbatim from M2-B06 (Context: "Item-stack
//! shape reused verbatim from M2-B06"). `Slot` is deliberately not written here — every caller
//! (chest/furnace/hopper `to_nbt`) wraps this compound's output with its own `Slot: Byte`
//! sibling entry at the call site.

use rc_chunk_storage::ItemStackRecord;
use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{borrow, owned};

/// `{ id: String, count: Int, components: Compound (omitted if absent) }`.
pub fn item_stack_to_nbt(item: &ItemStackRecord) -> owned::NbtCompound {
    let mut c = owned::NbtCompound::new();
    c.insert("id", item.id.as_str());
    c.insert("count", item.count);
    if let Some(components) = &item.components {
        c.insert("components", owned::NbtTag::Compound(components.clone()));
    }
    c
}

/// Inverse of `item_stack_to_nbt`. `components`'s absence is not an error (`None`).
pub fn item_stack_from_nbt(
    compound: &borrow::NbtCompound<'_, '_>,
    path: &NbtPath,
) -> Result<ItemStackRecord, SchemaError> {
    let id = compound.require_string(path, "id")?.to_str().into_owned();
    let count = compound.require_int(path, "count")?;
    let components = compound.compound("components").map(|c| c.to_owned());
    Ok(ItemStackRecord {
        id,
        count,
        components,
    })
}

/// Shared by chest/furnace/hopper `to_nbt` (Deliverables' own item-stack-shape reuse note,
/// extended here — not part of Context's literal listing, needed to avoid tripling the
/// identical Items-list encode loop across the three block-entity types): builds the vanilla
/// `Items: List<Compound>` tag from only the occupied slots of `slots`, each carrying its own
/// `Slot: Byte` sibling entry matching its array index.
pub(crate) fn slots_to_items_list(slots: &[Option<ItemStackRecord>]) -> owned::NbtTag {
    let items: Vec<owned::NbtCompound> = slots
        .iter()
        .enumerate()
        .filter_map(|(i, slot)| {
            slot.as_ref().map(|item| {
                let mut c = item_stack_to_nbt(item);
                c.insert("Slot", i as i8);
                c
            })
        })
        .collect();
    owned::NbtTag::List(owned::NbtList::Compound(items))
}

/// Shared by chest/furnace/hopper `from_nbt` (the decode half of `slots_to_items_list` above):
/// decodes `compound`'s `Items` list into `out` (already sized to the container's own slot
/// count) — an entry whose `Slot` index falls outside `out`'s bounds is silently ignored
/// (defensive; no fixture in this blueprint's own suite produces one). Errors precisely as
/// `item_stack_from_nbt`/`require_byte` do on a malformed entry.
pub(crate) fn items_list_from_nbt(
    compound: &borrow::NbtCompound<'_, '_>,
    path: &NbtPath,
    out: &mut [Option<ItemStackRecord>],
) -> Result<(), SchemaError> {
    let items_list = compound.require_list(path, "Items")?;
    let items_path = path.field("Items");
    let entries = items_list
        .compounds()
        .ok_or_else(|| SchemaError::WrongType {
            path: items_path.clone(),
            field: "Items",
            expected: "List<Compound>",
            actual_id: items_list.id(),
        })?;
    for (i, entry) in entries.into_iter().enumerate() {
        let entry_path = items_path.index(i);
        let slot_index = entry.require_byte(&entry_path, "Slot")? as usize;
        let item = item_stack_from_nbt(&entry, &entry_path)?;
        if slot_index < out.len() {
            out[slot_index] = Some(item);
        }
    }
    Ok(())
}
