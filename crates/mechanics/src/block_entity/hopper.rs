//! Hopper — transfer semantics, restated exactly (Context: "Hopper — transfer semantics,
//! restated exactly"). Cross-region hopper chains, hopper minecarts, and item-entity
//! collection are explicitly out of scope (Constraints (g)).

use bevy_ecs::prelude::Component;
use rc_chunk_storage::ItemStackRecord;
use rc_core::BlockPos;
use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{borrow, owned};

use crate::block_entity::BlockEntityWorldAccess;
use crate::container::ItemMaxStackSize;
use crate::direction::Direction;

pub const HOPPER_SLOT_COUNT: usize = 5;
pub const ALL_HOPPER_SLOTS: [usize; HOPPER_SLOT_COUNT] = [0, 1, 2, 3, 4];

#[derive(Component, Clone, Debug, PartialEq)]
pub struct HopperBlockEntity {
    pub slots: [Option<ItemStackRecord>; HOPPER_SLOT_COUNT],
    pub transfer_cooldown: u8,
    /// One of `{Down, North, South, East, West}` — never `Up` (Context; not structurally
    /// enforced, restated as a caller invariant).
    pub facing: Direction,
    pub custom_name: Option<String>,
    pub lock: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HopperTickOutcome {
    OnCooldown,
    Locked,
    Pushed,
    Pulled,
    Idle,
}

impl HopperBlockEntity {
    pub fn empty(facing: Direction) -> Self {
        todo!()
    }

    /// Context's own binding pseudocode, implemented exactly (cooldown gate, lock gate,
    /// push-then-pull, the 8/7-tick cooldown rule, furnace-face-aware insertion/extraction via
    /// `TierOneContainer`). `pos` is this hopper's own absolute position (needed to compute
    /// `facing.apply(pos)`/`Direction::Up.apply(pos)` and to query `world.is_locked_by_redstone`).
    pub fn tick(
        &mut self,
        pos: BlockPos,
        world: &mut dyn BlockEntityWorldAccess,
        max_stack: &dyn ItemMaxStackSize,
    ) -> HopperTickOutcome {
        todo!()
    }

    pub fn comparator_signal(&self, max_stack: &dyn ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound {
        todo!()
    }

    pub fn from_nbt(
        compound: &borrow::NbtCompound<'_, '_>,
    ) -> Result<(BlockPos, Self), SchemaError> {
        todo!()
    }
}

impl crate::container::TierOneContainer for HopperBlockEntity {
    fn slots(&self) -> &[Option<ItemStackRecord>] {
        &self.slots
    }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] {
        &mut self.slots
    }
}
