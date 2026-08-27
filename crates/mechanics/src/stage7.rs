//! `system_block_entity_tick`'s ECS-agnostic core (Context: "one system, sequential
//! chunk+block-entity loop — ARCH-D17's cross-chunk-same-region collapse is therefore
//! automatic"). For each chunk in `world.region_chunks()` (already ascending `(x, z)`), for
//! each `(pos, kind)` in `world.block_entities_in_chunk(chunk)` (in `BlockEntityIndex`'s own
//! stored load order), dispatches by `kind`: `Hopper` calls `HopperBlockEntity::tick`;
//! `Furnace` calls `FurnaceBlockEntity::tick` then, if it returned a lit-state change, calls
//! `world.swap_furnace_lit_state`; `Chest` has no per-tick *transfer* behavior at M3. Every one
//! of the three kinds — including chest — additionally records its own `comparator_signal`
//! into `container_signals` once, after whatever kind-specific tick logic ran (Context:
//! "Wiring into M3-B04's `ContainerSignalSource`").

#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage7/ecs.rs

use crate::block_entity::container_signal_source::Tier1ContainerSignalSource;
use crate::block_entity::furnace::{FuelTable, FurnaceLitStateResolver, SmeltingRecipeTable};
use crate::block_entity::{BlockEntityKind, BlockEntityWorldAccess};
use crate::container::ItemMaxStackSize;

pub fn run_block_entity_tick(
    world: &mut dyn BlockEntityWorldAccess,
    recipes: &SmeltingRecipeTable,
    fuels: &FuelTable,
    max_stack: &dyn ItemMaxStackSize,
    lit_resolver: Option<&dyn FurnaceLitStateResolver>,
    container_signals: &Tier1ContainerSignalSource,
) {
    todo!()
}
