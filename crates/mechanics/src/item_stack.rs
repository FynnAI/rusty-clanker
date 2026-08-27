//! One occupied inventory slot's NBT shape, reused verbatim from M2-B06 (Context: "Item-stack
//! shape reused verbatim from M2-B06"). `Slot` is deliberately not written here — every caller
//! (chest/furnace/hopper `to_nbt`) wraps this compound's output with its own `Slot: Byte`
//! sibling entry at the call site.

use rc_chunk_storage::ItemStackRecord;
use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{borrow, owned};

/// `{ id: String, count: Int, components: Compound (omitted if absent) }`.
pub fn item_stack_to_nbt(item: &ItemStackRecord) -> owned::NbtCompound {
    todo!()
}

/// Inverse of `item_stack_to_nbt`. `components`'s absence is not an error (`None`).
pub fn item_stack_from_nbt(
    compound: &borrow::NbtCompound<'_, '_>,
    path: &NbtPath,
) -> Result<ItemStackRecord, SchemaError> {
    todo!()
}

/// Shared by chest/furnace/hopper `to_nbt` (Deliverables' own item-stack-shape reuse note,
/// extended here — not part of Context's literal listing, needed to avoid tripling the
/// identical Items-list encode loop across the three block-entity types): builds the vanilla
/// `Items: List<Compound>` tag from only the occupied slots of `slots`, each carrying its own
/// `Slot: Byte` sibling entry matching its array index.
pub(crate) fn slots_to_items_list(slots: &[Option<ItemStackRecord>]) -> owned::NbtTag {
    todo!()
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
    todo!()
}
