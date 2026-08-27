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
use crate::block_entity::furnace::{
    FuelTable, FurnaceLitStateResolver, LitStateChange, SmeltingRecipeTable,
};
use crate::block_entity::hopper::HopperBlockEntity;
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
    for chunk in world.region_chunks() {
        for (pos, kind) in world.block_entities_in_chunk(chunk) {
            match kind {
                BlockEntityKind::Hopper => {
                    let Some(hopper_ref) = world.get_hopper_mut(pos) else {
                        continue;
                    };
                    let facing = hopper_ref.facing;
                    // Detach the hopper being ticked from `world` before handing `world`
                    // itself to `tick` -- `HopperBlockEntity::tick`'s own signature needs
                    // `&mut dyn BlockEntityWorldAccess` to reach *other* containers
                    // (push/pull neighbours), which Rust cannot allow simultaneously with a
                    // live `&mut HopperBlockEntity` borrowed *from* that same `world` (two
                    // `&mut self` calls through one `&mut dyn Trait` can never alias).
                    // `HopperBlockEntity::empty(facing)` is a cheap, harmless placeholder
                    // that sits at `pos` only for the duration of this one tick call --
                    // nothing else in this single-threaded, sequential Stage-7 pass ever
                    // observes it, since a hopper's own `tick` never targets its own
                    // position via `world`.
                    let mut hopper =
                        std::mem::replace(hopper_ref, HopperBlockEntity::empty(facing));
                    hopper.tick(pos, world, max_stack);
                    let signal = hopper.comparator_signal(max_stack);
                    if let Some(slot) = world.get_hopper_mut(pos) {
                        *slot = hopper;
                    }
                    container_signals.record(pos, signal);
                }
                BlockEntityKind::Furnace => {
                    let mut lit_change = LitStateChange::Unchanged;
                    let mut signal = 0u8;
                    if let Some(furnace) = world.get_furnace_mut(pos) {
                        lit_change = furnace.tick(recipes, fuels, max_stack);
                        signal = furnace.comparator_signal(max_stack);
                    }
                    if lit_change != LitStateChange::Unchanged {
                        world.swap_furnace_lit_state(
                            pos,
                            lit_change == LitStateChange::NowLit,
                            lit_resolver,
                        );
                    }
                    container_signals.record(pos, signal);
                }
                BlockEntityKind::Chest => {
                    // No per-tick transfer behavior at M3 (Context) -- only the comparator
                    // query this fix wires in (Context: "Wiring into M3-B04's
                    // ContainerSignalSource").
                    if let Some(chest) = world.get_chest_mut(pos) {
                        let signal = chest.comparator_signal(max_stack);
                        container_signals.record(pos, signal);
                    }
                }
            }
        }
    }
}
