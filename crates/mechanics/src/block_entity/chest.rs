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
        todo!()
    }

    /// Increments `open_count`; if it transitioned `0 -> 1`, emits `CHEST_OPEN_EVENT_ID` via
    /// `queue` (Context). Returns the new count.
    pub fn add_viewer(
        &mut self,
        pos: BlockPos,
        block_state: rc_chunk_storage::BlockStateId,
        queue: &mut BlockEventQueue,
    ) -> u8 {
        todo!()
    }

    /// Decrements `open_count` (floored at 0); if it transitioned `1 -> 0`, emits the same
    /// event. Returns the new count.
    pub fn remove_viewer(
        &mut self,
        pos: BlockPos,
        block_state: rc_chunk_storage::BlockStateId,
        queue: &mut BlockEventQueue,
    ) -> u8 {
        todo!()
    }

    pub fn comparator_signal(&self, max_stack: &dyn crate::container::ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    /// `id: "minecraft:chest"`, `x`/`y`/`z` from `pos`, `Items` (only occupied slots, each with
    /// its own `Slot: Byte`), `CustomName`/`Lock` if present. DataVersion is the caller's own
    /// responsibility (this is the block-entity-local compound only, not a full document).
    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound {
        todo!()
    }

    pub fn from_nbt(
        compound: &borrow::NbtCompound<'_, '_>,
    ) -> Result<(BlockPos, Self), SchemaError> {
        todo!()
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
