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
    /// The tier-1 minimal recipe table (Context, cited with sources): `cobblestone ->
    /// stone`, `iron_ore -> iron_ingot`, `sand -> glass`, all 200 cook ticks, output count 1.
    pub fn minimal_tier1() -> Self {
        let mut map = HashMap::new();
        map.insert(
            "minecraft:cobblestone",
            SmeltingRecipe {
                output_id: "minecraft:stone",
                output_count: 1,
                cook_ticks: 200,
            },
        );
        map.insert(
            "minecraft:iron_ore",
            SmeltingRecipe {
                output_id: "minecraft:iron_ingot",
                output_count: 1,
                cook_ticks: 200,
            },
        );
        map.insert(
            "minecraft:sand",
            SmeltingRecipe {
                output_id: "minecraft:glass",
                output_count: 1,
                cook_ticks: 200,
            },
        );
        Self(map)
    }
    pub fn lookup(&self, input_item_id: &str) -> Option<SmeltingRecipe> {
        self.0.get(input_item_id).copied()
    }
}

/// Hand-authored, minimal tier-1 fuel table (Context). `#[derive(Resource)]`, same reasoning
/// as `SmeltingRecipeTable` above.
#[derive(Resource)]
pub struct FuelTable(HashMap<&'static str, u16>);

impl FuelTable {
    /// The tier-1 minimal fuel table (Context, cited with sources): `coal`/`charcoal` 1600,
    /// `coal_block` 16000, `blaze_rod` 2400, `lava_bucket` 20000, `oak_planks` 300, `stick`
    /// 100 (all in ticks).
    pub fn minimal_tier1() -> Self {
        let mut map = HashMap::new();
        map.insert("minecraft:coal", 1600);
        map.insert("minecraft:charcoal", 1600);
        map.insert("minecraft:coal_block", 16000);
        map.insert("minecraft:blaze_rod", 2400);
        map.insert("minecraft:lava_bucket", 20000);
        map.insert("minecraft:oak_planks", 300);
        map.insert("minecraft:stick", 100);
        Self(map)
    }
    pub fn lookup(&self, fuel_item_id: &str) -> Option<u16> {
        self.0.get(fuel_item_id).copied()
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
        Self {
            slots: std::array::from_fn(|_| None),
            lit_time_remaining: 0,
            lit_total_time: 0,
            cook_time: 0,
            cook_time_total: 0,
            cooking_recipe_output_id: None,
            custom_name: None,
            lock: None,
        }
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
        let was_lit = self.lit_time_remaining > 0;
        if self.lit_time_remaining > 0 {
            self.lit_time_remaining -= 1;
        }

        let recipe: Option<SmeltingRecipe> = self.slots[FURNACE_SLOT_INPUT]
            .as_ref()
            .and_then(|s| recipes.lookup(&s.id));
        let output_compatible = match (&self.slots[FURNACE_SLOT_OUTPUT], recipe) {
            (None, Some(_)) => true,
            (Some(existing), Some(r)) => {
                existing.id == r.output_id
                    && (existing.count + r.output_count) as u32
                        <= max_stack.max_stack_size(r.output_id).min(64)
            }
            (_, None) => false,
        };
        let can_smelt = recipe.is_some() && output_compatible;

        if self.lit_time_remaining == 0
            && can_smelt
            && let Some(fuel_stack) = &self.slots[FURNACE_SLOT_FUEL]
            && let Some(burn_ticks) = fuels.lookup(&fuel_stack.id)
        {
            crate::container::decrement_or_clear(&mut self.slots[FURNACE_SLOT_FUEL], 1);
            self.lit_time_remaining = burn_ticks;
            self.lit_total_time = burn_ticks;
        }

        let now_lit = self.lit_time_remaining > 0;
        if can_smelt && now_lit {
            // `can_smelt` already established `recipe.is_some()`.
            let r = recipe.unwrap();
            if self.cooking_recipe_output_id.as_deref() != Some(r.output_id) {
                self.cook_time = 0;
            }
            self.cooking_recipe_output_id = Some(r.output_id.to_string());
            self.cook_time_total = r.cook_ticks;
            self.cook_time += 1;
            if self.cook_time >= self.cook_time_total {
                self.cook_time = 0;
                crate::container::decrement_or_clear(&mut self.slots[FURNACE_SLOT_INPUT], 1);
                crate::container::place_or_stack_output(
                    &mut self.slots[FURNACE_SLOT_OUTPUT],
                    r.output_id,
                    r.output_count,
                );
            }
        } else {
            self.cook_time = self.cook_time.saturating_sub(2);
        }

        if was_lit != now_lit {
            return if now_lit {
                LitStateChange::NowLit
            } else {
                LitStateChange::NowUnlit
            };
        }
        LitStateChange::Unchanged
    }

    pub fn comparator_signal(&self, max_stack: &dyn crate::container::ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    /// M3.5-B05 (TEST-D57 pass, `M3.5-B05-CLAIMS.md`): real vanilla 26.2's furnace
    /// block-entity NBT keys its four timing fields `lit_time_remaining`/`lit_total_time`/
    /// `cooking_time_spent`/`cooking_total_time` (all `Short`) -- the pre-rename
    /// `BurnTime`/`CookTime`/`CookTimeTotal` spellings this crate wrote through M3 are
    /// corrected here; `lit_total_time` (tracked in memory since M3-B06 but never
    /// persisted) is now written for real.
    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound {
        let mut out = owned::NbtCompound::new();
        out.insert("id", "minecraft:furnace");
        out.insert("x", pos.x);
        out.insert("y", pos.y);
        out.insert("z", pos.z);
        out.insert("Items", crate::item_stack::slots_to_items_list(&self.slots));
        out.insert("lit_time_remaining", self.lit_time_remaining as i16);
        out.insert("lit_total_time", self.lit_total_time as i16);
        out.insert("cooking_time_spent", self.cook_time as i16);
        out.insert("cooking_total_time", self.cook_time_total as i16);
        // `RecipesUsed` (the furnace's real experience-bookkeeping map, a
        // `Map<ResourceKey<Recipe<?>>, Integer>` in real vanilla) is deliberately not
        // modeled at M3.5 (no smelting/experience until M4, Context 2.4) -- written as
        // an empty compound so a vanilla reader still finds the key.
        out.insert(
            "RecipesUsed",
            owned::NbtTag::Compound(owned::NbtCompound::new()),
        );
        if let Some(name) = &self.custom_name {
            out.insert("CustomName", name.as_str());
        }
        if let Some(lock) = &self.lock {
            out.insert("Lock", lock.as_str());
        }
        out
    }

    /// `cooking_recipe_output_id` is not part of vanilla's own furnace block-entity NBT
    /// schema (real vanilla derives its analog, `RecipesUsed`, from a
    /// `Map<ResourceKey<Recipe<?>>, Integer>` this engine does not model at M3.5 --
    /// `M3.5-B05-CLAIMS.md`) and is therefore not written by `to_nbt` above; `from_nbt`
    /// always decodes it as `None`. A furnace loaded mid-cook can therefore see one
    /// spurious cook-progress reset on the very next `tick()` call if the input item's
    /// own recipe output id differs from what was cooking before save (a real, if minor,
    /// vanilla-parity gap this crate's own NBT schema inherits — flagged in the M3-B06
    /// field report, not silently accepted).
    pub fn from_nbt(
        compound: &borrow::NbtCompound<'_, '_>,
    ) -> Result<(BlockPos, Self), SchemaError> {
        let path = NbtPath::root();
        let x = compound.require_int(&path, "x")?;
        let y = compound.require_int(&path, "y")?;
        let z = compound.require_int(&path, "z")?;
        let pos = BlockPos::new(x, y, z);

        let mut slots: [Option<ItemStackRecord>; FURNACE_SLOT_COUNT] =
            std::array::from_fn(|_| None);
        crate::item_stack::items_list_from_nbt(compound, &path, &mut slots)?;

        let lit_time_remaining = compound.require_short(&path, "lit_time_remaining")? as u16;
        let lit_total_time = compound.require_short(&path, "lit_total_time")? as u16;
        let cook_time = compound.require_short(&path, "cooking_time_spent")? as u16;
        let cook_time_total = compound.require_short(&path, "cooking_total_time")? as u16;
        let custom_name = compound
            .string("CustomName")
            .map(|s| s.to_str().into_owned());
        let lock = compound.string("Lock").map(|s| s.to_str().into_owned());

        Ok((
            pos,
            Self {
                slots,
                lit_time_remaining,
                lit_total_time,
                cook_time,
                cook_time_total,
                cooking_recipe_output_id: None,
                custom_name,
                lock,
            },
        ))
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

/// M3.5-B05 (WORLD-D6): a thin wrapper over `to_nbt`/`from_nbt` above, exposing them
/// through `rc-chunk-storage`'s generic persistence contract. Always writes/dispatches
/// `id: "minecraft:furnace"` — `blast_furnace`/`smoker` share this same component at the
/// ECS level (`world.rs`'s own `spawn_block_entity_for_placement` precedent); a real
/// furnace's own actual block kind is derivable from its raw block-state id if a later
/// blueprint needs it, out of this one's scope.
impl rc_chunk_storage::BlockEntityCodec for FurnaceBlockEntity {
    fn to_record(&self, pos: BlockPos) -> rc_chunk_storage::BlockEntityRecord {
        rc_chunk_storage::BlockEntityRecord {
            pos,
            id: "minecraft:furnace".to_string(),
            data: self.to_nbt(pos),
        }
    }

    fn from_record(
        record: &rc_chunk_storage::BlockEntityRecord,
    ) -> Result<Self, rc_chunk_storage::BlockEntityCodecError> {
        let bytes = rc_nbt::write_owned(&rc_nbt::owned::BaseNbt::new("", record.data.clone()));
        let nbt = rc_nbt::read_borrowed_strict(&bytes)?;
        let base = match nbt {
            rc_nbt::borrow::Nbt::Some(base) => base,
            rc_nbt::borrow::Nbt::None => {
                return Err(rc_nbt::NbtError::from(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "BlockEntityRecord::data round-tripped to an empty NBT document",
                ))
                .into());
            }
        };
        let compound = base.as_compound();
        let (_pos, value) = Self::from_nbt(&compound)?;
        Ok(value)
    }
}
