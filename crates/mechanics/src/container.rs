//! Container model shared by every tier-1 block entity (Context: "Container model — comparator-
//! fullness formula", MECH-D13/D48): the comparator-signal formula, slot-manipulation helpers
//! `hopper.rs`'s transfer algorithm and `furnace.rs`'s fuel/recipe consumption are both written
//! once against, and `TierOneContainer`, the seam every one of the three types implements.

use std::sync::Arc;

use bevy_ecs::prelude::Resource;
use rc_chunk_storage::ItemStackRecord;

pub const DEFAULT_MAX_STACK_SIZE: u32 = 64;

/// Injected item-registry seam (Context — mirrors M2-B04's `BlockStateNames`/`BiomeNames`
/// "no generated registry yet" pattern).
pub trait ItemMaxStackSize: Send + Sync {
    fn max_stack_size(&self, item_id: &str) -> u32;
}

/// Always `64` — correct for every non-tool/non-unique item (Context).
pub struct DefaultMaxStackSize;
impl ItemMaxStackSize for DefaultMaxStackSize {
    fn max_stack_size(&self, _item_id: &str) -> u32 {
        DEFAULT_MAX_STACK_SIZE
    }
}

/// The `bevy_ecs::Resource`-carrying wrapper around an injected `ItemMaxStackSize` (a bare
/// `Arc<dyn ItemMaxStackSize>` cannot itself derive `Resource` — a trait object needs a
/// concrete newtype to attach the derive to).
#[derive(Resource, Clone)]
pub struct MaxStackSizeResource(pub Arc<dyn ItemMaxStackSize>);

/// The comparator-fullness formula (Context, MECH-D13/D48), generic over any tier-1
/// container's own slot slice. `occupied == 0` returns exactly `0` (the empty-container edge
/// case the wiki's own simplified formula elides — Context's own hand-verified correction).
pub fn comparator_signal_from_slots(
    slots: &[Option<ItemStackRecord>],
    max_stack: &dyn ItemMaxStackSize,
) -> u8 {
    todo!()
}

/// First non-empty slot within `allowed_slots`, in the order `allowed_slots` itself lists
/// (Context: "leftmost," restricted — every real caller already supplies an ascending-index
/// list, `container/mod.rs`'s own default `(0..len).collect()` or a furnace override's
/// trivially-ascending single-element `Vec`).
pub fn find_leftmost_extract_slot(
    slots: &[Option<ItemStackRecord>],
    allowed_slots: &[usize],
) -> Option<usize> {
    todo!()
}

/// First slot within `allowed_slots` that already holds `item_id` with `count < cap`, or the
/// first empty slot within `allowed_slots` if no stackable match exists (leftmost-empty is
/// checked only after every allowed slot has been scanned for a stackable match — matching
/// vanilla's own "prefer merging over spreading" behavior).
pub fn find_leftmost_insert_slot(
    slots: &[Option<ItemStackRecord>],
    item_id: &str,
    cap: u32,
    allowed_slots: &[usize],
) -> Option<usize> {
    todo!()
}

/// Moves exactly one item unit from `src[src_slot]` to `dst[dst_slot]` (creating a fresh
/// 1-count stack in `dst` if it was empty, else incrementing its count by 1 and decrementing
/// `src`'s by 1 — clearing `src[src_slot]` to `None` if its count reaches 0). Panics
/// (`debug_assert!`) if `src[src_slot]` is `None` or `dst[dst_slot]` holds a different,
/// non-stackable item id — callers (hopper transfer, furnace fuel/input consumption) are
/// responsible for calling this only after `find_leftmost_*` confirms compatibility.
pub fn move_one_item(
    src: &mut [Option<ItemStackRecord>],
    src_slot: usize,
    dst: &mut [Option<ItemStackRecord>],
    dst_slot: usize,
) {
    todo!()
}

/// Decrements `slot`'s count by `n`, clearing it to `None` if the result is `0`. Panics
/// (`debug_assert!`) if `slot` is `None` or its count is `< n`.
pub fn decrement_or_clear(slot: &mut Option<ItemStackRecord>, n: i32) {
    todo!()
}

/// Places `count` units of `item_id` into `slot`: creates a fresh stack if `slot` is `None`,
/// else increments an existing same-`item_id` stack's count by `count`. Furnace-output-only
/// helper (a furnace recipe's own output never needs a leftmost-slot search — it always
/// targets exactly `FURNACE_SLOT_OUTPUT`).
pub fn place_or_stack_output(slot: &mut Option<ItemStackRecord>, item_id: &str, count: i32) {
    todo!()
}

/// The seam `hopper.rs`'s transfer algorithm is written once, generically, against (Context:
/// "Container model"). Implemented by `ChestBlockEntity`/`FurnaceBlockEntity`/
/// `HopperBlockEntity` (`block_entity/*.rs`).
pub trait TierOneContainer {
    fn slots(&self) -> &[Option<ItemStackRecord>];
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>];
    /// Default: every slot index, `from_above` ignored (chest, hopper-as-destination).
    /// `FurnaceBlockEntity` overrides this (Context's "furnace face rule").
    fn insertable_slots(&self, _from_above: bool) -> Vec<usize> {
        (0..self.slots().len()).collect()
    }
    /// Default: every slot index. `FurnaceBlockEntity` overrides this.
    fn extractable_slots(&self) -> Vec<usize> {
        (0..self.slots().len()).collect()
    }
}
