//! Furnace — burn/cook state machine, fuel/recipe tables, lit-state block swap, comparator
//! (Context: "Furnace — burn/cook state machine, fuel/recipe tables, lit-state block swap,
//! comparator"). Fuel/recipe tables are a hand-authored, minimal tier-1 stand-in, explicitly
//! not MECH-D52's future data-driven pipeline.

use std::collections::HashMap;

use bevy_ecs::prelude::{Component, Resource};
use rc_chunk_storage::{BlockStateId, ItemStackRecord};
use rc_core::BlockPos;
use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{borrow, owned};

use crate::container::TierOneContainer;

pub const FURNACE_SLOT_INPUT: usize = 0;
pub const FURNACE_SLOT_FUEL: usize = 1;
pub const FURNACE_SLOT_OUTPUT: usize = 2;
pub const FURNACE_SLOT_COUNT: usize = 3;
pub const DEFAULT_COOK_TICKS: u16 = 200;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SmeltingRecipe {
    pub output_id: &'static str,
    pub output_count: i32,
    pub cook_ticks: u16,
}

/// Hand-authored, minimal tier-1 recipe table (Context — not MECH-D52's future data-driven
/// pipeline). `#[derive(Resource)]` so `bootstrap_default_stage7_resources` (`stage7/ecs.rs`)
/// can insert it directly.
#[derive(Resource)]
pub struct SmeltingRecipeTable(HashMap<&'static str, SmeltingRecipe>);

impl SmeltingRecipeTable {
    pub fn minimal_tier1() -> Self {
        todo!()
    }
    pub fn lookup(&self, input_item_id: &str) -> Option<SmeltingRecipe> {
        todo!()
    }
}

/// Hand-authored, minimal tier-1 fuel table (Context). `#[derive(Resource)]`, same reasoning
/// as `SmeltingRecipeTable` above.
#[derive(Resource)]
pub struct FuelTable(HashMap<&'static str, u16>);

impl FuelTable {
    pub fn minimal_tier1() -> Self {
        todo!()
    }
    pub fn lookup(&self, fuel_item_id: &str) -> Option<u16> {
        todo!()
    }
}

/// Injected block-state resolver (Context: "Lit-state block swap" — no real implementation
/// ships in this blueprint).
pub trait FurnaceLitStateResolver: Send + Sync {
    fn lit_variant(&self, unlit: BlockStateId) -> Option<BlockStateId>;
    fn unlit_variant(&self, lit: BlockStateId) -> Option<BlockStateId>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LitStateChange {
    Unchanged,
    NowLit,
    NowUnlit,
}

#[derive(Component, Clone, Debug, PartialEq)]
pub struct FurnaceBlockEntity {
    pub slots: [Option<ItemStackRecord>; FURNACE_SLOT_COUNT],
    pub lit_time_remaining: u16,
    pub lit_total_time: u16,
    pub cook_time: u16,
    pub cook_time_total: u16,
    pub cooking_recipe_output_id: Option<String>,
    pub custom_name: Option<String>,
    pub lock: Option<String>,
}

impl FurnaceBlockEntity {
    pub fn empty() -> Self {
        todo!()
    }

    /// Context's own binding pseudocode, implemented exactly. Returns whether the `lit`
    /// blockstate boolean should now be swapped (the caller — the Stage-7 driver — is
    /// responsible for actually calling `BlockEntityWorldAccess::swap_furnace_lit_state`).
    pub fn tick(
        &mut self,
        recipes: &SmeltingRecipeTable,
        fuels: &FuelTable,
        max_stack: &dyn crate::container::ItemMaxStackSize,
    ) -> LitStateChange {
        todo!()
    }

    pub fn comparator_signal(&self, max_stack: &dyn crate::container::ItemMaxStackSize) -> u8 {
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

impl TierOneContainer for FurnaceBlockEntity {
    fn slots(&self) -> &[Option<ItemStackRecord>] {
        &self.slots
    }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] {
        &mut self.slots
    }
    /// Context's "furnace face rule": from above -> input only; from any side -> fuel only.
    fn insertable_slots(&self, from_above: bool) -> Vec<usize> {
        vec![if from_above {
            FURNACE_SLOT_INPUT
        } else {
            FURNACE_SLOT_FUEL
        }]
    }
    /// Output only — extraction always means "hopper below, pulling up" (Context).
    fn extractable_slots(&self) -> Vec<usize> {
        vec![FURNACE_SLOT_OUTPUT]
    }
}
