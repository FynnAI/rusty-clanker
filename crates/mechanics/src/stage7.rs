//! `system_block_entity_tick`'s ECS-agnostic core (Context: "one system, sequential
//! chunk+block-entity loop — ARCH-D17's cross-chunk-same-region collapse is therefore
//! automatic"). For each chunk in `world.region_chunks()` (already ascending `(x, z)`), for
//! each `(pos, kind)` in `world.block_entities_in_chunk(chunk)` (in `BlockEntityIndex`'s own
//! stored load order), dispatches by `kind`: `Hopper` calls `HopperBlockEntity::tick`;
//! `Furnace` calls `FurnaceBlockEntity::tick` then, if it returned a lit-state change, calls
//! `world.swap_furnace_lit_state`; `Chest` has no per-tick *transfer* behavior at M3.
//!
//! **Field-report correction (hopper-cadence wave, `docs/findings-for-planning.md`'s own
//! `comparator_clock_container_fill` residual, verified against the real oracle):** every kind's
//! own `comparator_signal` is recorded in a *second*, separate pass over the same visited
//! positions, only after every position's own kind-specific tick behavior (the loop above) has
//! finished mutating for this tick — never interleaved position-by-position the way a single
//! combined loop would. Real vanilla's `BlockEntity.setChanged` fires synchronously, at the
//! exact moment a container's content changes, regardless of which block entity's own turn it
//! happens to be. A single interleaved loop cannot reproduce that: a chest recorded (in load
//! order) *before* the hopper that feeds it still ticks this same pass captures its own
//! pre-push, stale content — the real push already landed in the chest's actual slots (`world`
//! is mutated immediately, `container_at_mut`/`move_one_item`), but that content is invisible to
//! `container_signals` until the *next* pass reaches the chest's own turn again, one whole game
//! tick later than vanilla's synchronous notify. Splitting signal-recording into its own,
//! later pass closes this regardless of load order — every recorded signal now reflects this
//! tick's own final content, exactly once, matching vanilla's per-tick synchronous notify
//! without this pass's own internal iteration order leaking into observable timing.

#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage7/ecs.rs

use std::collections::HashSet;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_messaging::{Address, RegionMessage};

use crate::behavior::{BlockBehaviorRegistry, UpdateContext};
use crate::block_entity::container_signal_source::Tier1ContainerSignalSource;
use crate::block_entity::furnace::{
    FuelTable, FurnaceLitStateResolver, LitStateChange, SmeltingRecipeTable,
};
use crate::block_entity::hopper::HopperBlockEntity;
use crate::block_entity::{BlockEntityKind, BlockEntityWorldAccess};
use crate::block_event::BlockEventQueue;
use crate::border::RegionOwnership;
use crate::container::ItemMaxStackSize;
use crate::light::LightDirtyQueue;
use crate::neighbor_update::NeighborUpdateEngine;
use crate::redstone::notify_neighbor_changed_only;
use crate::scheduled_tick::ScheduledTickQueue;
use crate::world_access::BlockWorldAccess;

pub fn run_block_entity_tick(
    world: &mut dyn BlockEntityWorldAccess,
    recipes: &SmeltingRecipeTable,
    fuels: &FuelTable,
    max_stack: &dyn ItemMaxStackSize,
    lit_resolver: Option<&dyn FurnaceLitStateResolver>,
    container_signals: &Tier1ContainerSignalSource,
) {
    // Positions of every hopper whose own `tick` has already run within *this* pass (spans
    // every chunk this call processes, not just the current one) -- `HopperBlockEntity::tick`'s
    // own chained-hopper cooldown quirk needs to know, for a push landing on another hopper,
    // whether that other hopper already had its own tick this same game tick (field-report
    // correction, `hopper.rs`'s own `tick` doc comment has the full citation).
    let mut already_ticked_hoppers: HashSet<BlockPos> = HashSet::new();
    // Every position this pass visits, in the same per-chunk load order `block_entities_in_
    // chunk` returns -- collected here so the second, signal-recording pass below (module doc
    // comment has the full rationale) can revisit them after every kind-specific tick behavior
    // in this loop has finished mutating for this tick.
    let mut visited: Vec<(BlockPos, BlockEntityKind)> = Vec::new();

    for chunk in world.region_chunks() {
        for (pos, kind) in world.block_entities_in_chunk(chunk) {
            visited.push((pos, kind));
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
                    hopper.tick(pos, world, max_stack, &already_ticked_hoppers);
                    already_ticked_hoppers.insert(pos);
                    if let Some(slot) = world.get_hopper_mut(pos) {
                        *slot = hopper;
                    }
                }
                BlockEntityKind::Furnace => {
                    let mut lit_change = LitStateChange::Unchanged;
                    if let Some(furnace) = world.get_furnace_mut(pos) {
                        lit_change = furnace.tick(recipes, fuels, max_stack);
                    }
                    if lit_change != LitStateChange::Unchanged {
                        world.swap_furnace_lit_state(
                            pos,
                            lit_change == LitStateChange::NowLit,
                            lit_resolver,
                        );
                    }
                }
                BlockEntityKind::Chest => {
                    // No per-tick transfer behavior at M3 (Context).
                }
            }
        }
    }

    // Second pass (module doc comment has the full rationale): every visited position's own
    // comparator_signal is queried fresh here, after every position in `visited` has already
    // finished its own kind-specific tick behavior above -- so a container mutated by another
    // entity's own push *later* in the first pass's own load order (e.g. a hopper feeding a
    // chest placed earlier in the fixture's own `blocks:` list) still records this tick's own
    // final, post-mutation content, not a stale pre-mutation snapshot taken mid-pass (Context:
    // "Wiring into M3-B04's `ContainerSignalSource`").
    for (pos, kind) in visited {
        let signal = match kind {
            BlockEntityKind::Hopper => world
                .get_hopper_mut(pos)
                .map(|hopper| hopper.comparator_signal(max_stack)),
            BlockEntityKind::Furnace => world
                .get_furnace_mut(pos)
                .map(|furnace| furnace.comparator_signal(max_stack)),
            BlockEntityKind::Chest => world
                .get_chest_mut(pos)
                .map(|chest| chest.comparator_signal(max_stack)),
        };
        if let Some(signal) = signal {
            container_signals.record(pos, signal);
        }
    }
}

/// M3 field-report fix (Section C, production half -- `docs/findings-for-planning.md`'s own
/// "Stage 7 has no path to trigger a Stage-4 redstone re-evaluation when a tier-1 container's
/// contents change" entry). The production counterpart of `crates/testing/gametest/src/
/// replay.rs`'s own per-tick `take_changed`/`notify_neighbor_changed_only`/drain loop (that
/// module's own doc comment has the full vanilla-parity rationale --
/// `BlockEntity.setChanged -> updateNeighbourForOutputSignal`, no counterpart anywhere in this
/// crate before this fix). Must run strictly after `run_block_entity_tick` within the same tick
/// (whose own `container_signals.record` calls are this function's read side, via `take_changed`)
/// and reuses that same tick's Stage-4 resources/dispatch (`stage4::drain_engine`, module-visible
/// for this exact call site) so a comparator adjacent to a container whose fullness changed this
/// tick re-evaluates before the tick ends -- reproducing the replay path's own already-proven
/// composition exactly: queue a `notify_neighbor_changed_only` fan-out for every position `take_
/// changed` reports, then drain the resulting `NeighborChanged`/`ShapeUpdate` cascade to a fixed
/// point once, after the whole batch (never per-position -- matches `replay.rs`'s own identical
/// two-phase shape, not a naive notify-then-drain-per-position loop).
#[allow(clippy::too_many_arguments)]
pub fn run_container_signal_notify(
    world: &mut dyn BlockWorldAccess,
    ownership: &RegionOwnership,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, BlockStateId)>,
    light_dirty: &mut LightDirtyQueue,
    current_tick: u64,
    container_signals: &Tier1ContainerSignalSource,
) {
    for pos in container_signals.take_changed() {
        let mut ctx = UpdateContext {
            world,
            engine,
            scheduled,
            events,
            outbound,
            changed,
            ownership,
            current_tick,
            light_dirty,
        };
        notify_neighbor_changed_only(&mut ctx, pos);
    }
    crate::stage4::drain_engine(
        world,
        engine,
        scheduled,
        events,
        outbound,
        changed,
        light_dirty,
        ownership,
        current_tick,
        behaviors,
    );
}
