//! Chest — open-count/viewer tracking, comparator, container-menu boundary (Context: "Chest —
//! open-count/viewer tracking, comparator, container-menu boundary"). Double-chest merging and
//! the full container-menu system are explicitly out of scope (Constraints (e)).

use bevy_ecs::prelude::Component;
use rc_chunk_storage::ItemStackRecord;
use rc_core::BlockPos;
use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{borrow, owned};

use crate::block_event::{BlockEvent, BlockEventQueue};
use crate::container::TierOneContainer;

pub const CHEST_SLOT_COUNT: usize = 27;
/// Vanilla's own chest-open-count block-event id (Context, MECH-D9).
pub const CHEST_OPEN_EVENT_ID: u8 = 1;

#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChestBlockEntity {
    pub slots: [Option<ItemStackRecord>; CHEST_SLOT_COUNT],
    pub open_count: u8,
    pub custom_name: Option<String>,
    pub lock: Option<String>,
}

impl ChestBlockEntity {
    pub fn empty() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            open_count: 0,
            custom_name: None,
            lock: None,
        }
    }

    /// Increments `open_count`; if it transitioned `0 -> 1`, emits `CHEST_OPEN_EVENT_ID` via
    /// `queue` (Context). Returns the new count.
    pub fn add_viewer(
        &mut self,
        pos: BlockPos,
        block_state: rc_chunk_storage::BlockStateId,
        queue: &mut BlockEventQueue,
    ) -> u8 {
        let was_zero = self.open_count == 0;
        self.open_count += 1;
        if was_zero {
            queue.emit(BlockEvent {
                pos,
                event_id: CHEST_OPEN_EVENT_ID,
                event_param: self.open_count,
                block_state,
            });
        }
        self.open_count
    }

    /// Decrements `open_count` (floored at 0); if it transitioned `1 -> 0`, emits the same
    /// event. Returns the new count.
    pub fn remove_viewer(
        &mut self,
        pos: BlockPos,
        block_state: rc_chunk_storage::BlockStateId,
        queue: &mut BlockEventQueue,
    ) -> u8 {
        if self.open_count == 0 {
            return 0;
        }
        self.open_count -= 1;
        if self.open_count == 0 {
            queue.emit(BlockEvent {
                pos,
                event_id: CHEST_OPEN_EVENT_ID,
                event_param: 0,
                block_state,
            });
        }
        self.open_count
    }

    pub fn comparator_signal(&self, max_stack: &dyn crate::container::ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    /// `id: "minecraft:chest"`, `x`/`y`/`z` from `pos`, `Items` (only occupied slots, each with
    /// its own `Slot: Byte`), `CustomName`/`Lock` if present. DataVersion is the caller's own
    /// responsibility (this is the block-entity-local compound only, not a full document).
    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound {
        let mut out = owned::NbtCompound::new();
        out.insert("id", "minecraft:chest");
        out.insert("x", pos.x);
        out.insert("y", pos.y);
        out.insert("z", pos.z);
        out.insert("Items", crate::item_stack::slots_to_items_list(&self.slots));
        if let Some(name) = &self.custom_name {
            out.insert("CustomName", name.as_str());
        }
        if let Some(lock) = &self.lock {
            out.insert("Lock", lock.as_str());
        }
        out
    }

    pub fn from_nbt(
        compound: &borrow::NbtCompound<'_, '_>,
    ) -> Result<(BlockPos, Self), SchemaError> {
        let path = NbtPath::root();
        let x = compound.require_int(&path, "x")?;
        let y = compound.require_int(&path, "y")?;
        let z = compound.require_int(&path, "z")?;
        let pos = BlockPos::new(x, y, z);

        let mut slots: [Option<ItemStackRecord>; CHEST_SLOT_COUNT] = std::array::from_fn(|_| None);
        crate::item_stack::items_list_from_nbt(compound, &path, &mut slots)?;

        let custom_name = compound
            .string("CustomName")
            .map(|s| s.to_str().into_owned());
        let lock = compound.string("Lock").map(|s| s.to_str().into_owned());

        Ok((
            pos,
            Self {
                slots,
                open_count: 0,
                custom_name,
                lock,
            },
        ))
    }
}

impl TierOneContainer for ChestBlockEntity {
    fn slots(&self) -> &[Option<ItemStackRecord>] {
        &self.slots
    }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] {
        &mut self.slots
    }
}
