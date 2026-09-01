//! The Stage-5 (random block tick, ARCH-D14) driver's ECS-agnostic core: for each of `chunks`
//! (already in the caller's own fixed, deterministic order), calls `random_tick_chunk`, then
//! dispatches every drawn position to `behaviors.resolve(state).on_random_tick`, draining the
//! neighbor-update engine to a fixed point after each dispatch (mirrors M3-B01's own Stage-4
//! per-item settling discipline).

#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage5/ecs.rs

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_messaging::{Address, RegionMessage};

use crate::behavior::{BlockBehaviorRegistry, RandomTickContext, UpdateContext};
use crate::block_event::BlockEventQueue;
use crate::border::RegionOwnership;
use crate::neighbor_update::{NeighborUpdateEngine, PendingUpdate};
use crate::random::RcRandom;
use crate::random_tick::{WorldSeed, draw_random_tick_positions};
use crate::scheduled_tick::ScheduledTickQueue;
use crate::world_access::BlockWorldAccess;

/// `system_random_tick`'s ECS-agnostic core (Context: "one system, sequential chunk loop").
/// For each of `chunks` (already in the caller's own fixed, deterministic order — this
/// function does not itself sort, since sorting needs `ChunkKey`'s own ordering and the ECS
/// adapter is the natural place to gather+sort the live chunk-entity list), calls
/// `random_tick_chunk`, then dispatches every drawn position to
/// `behaviors.resolve(state).on_random_tick`, draining the neighbor-update engine to a fixed
/// point after each dispatch (mirrors M3-B01's own Stage-4 per-item settling discipline,
/// reused for consistency even though no tier-1 random-tick behavior in this blueprint ever
/// calls `RandomTickContext::set_block`).
#[allow(clippy::too_many_arguments)]
pub fn run_random_tick_phase(
    world: &mut dyn BlockWorldAccess,
    chunks: &[(i32, i32)],
    seed: &WorldSeed,
    tick_counter: u64,
    random_tick_speed: u32,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, BlockStateId)>,
    ownership: &RegionOwnership,
) {
    for &(chunk_x, chunk_z) in chunks {
        // Constructed directly here (rather than via `random_tick::random_tick_chunk`'s own
        // convenience wrapper) so this same `rng` instance stays reachable afterward, for
        // every one of this chunk's own dispatch calls' `RandomTickContext::rng` field --
        // "further draws from the *same* per-chunk-per-tick stream the position-selection
        // loop itself already consumes" (behavior.rs's own doc comment).
        let mut rng = RcRandom::new(crate::random::chunk_random_seed(
            seed.0,
            chunk_x,
            chunk_z,
            tick_counter,
        ));
        let positions =
            draw_random_tick_positions(&mut rng, chunk_x * 16, chunk_z * 16, random_tick_speed);

        for candidate in positions {
            let pos: BlockPos = candidate.pos;
            let Some(state) = world.get_block(pos) else {
                continue;
            };
            let behavior = behaviors.resolve(state);

            let base = UpdateContext {
                world,
                engine,
                scheduled,
                events,
                outbound,
                changed,
                ownership,
                current_tick: tick_counter,
            };
            let mut ctx = RandomTickContext {
                base,
                rng: &mut rng,
            };
            behavior.on_random_tick(&mut ctx, pos);

            engine.drain(&mut |eng, item| {
                let mut item_ctx = UpdateContext {
                    world,
                    engine: eng,
                    scheduled,
                    events,
                    outbound,
                    changed,
                    ownership,
                    current_tick: tick_counter,
                };
                dispatch_pending_update(&mut item_ctx, behaviors, item);
            });
        }
    }
}

/// Duplicates `stage4.rs`'s own private `dispatch_pending_update` (that function is not `pub`
/// or `pub(crate)`, and Prerequisites forbid modifying `stage4.rs` — the one other file this
/// blueprint depends on but does not touch). Resolves the target position's own registered
/// behavior and calls the matching `on_neighbor_changed`/`on_shape_update` trait method,
/// identically to Stage 4's own dispatch.
fn dispatch_pending_update(
    ctx: &mut UpdateContext,
    behaviors: &BlockBehaviorRegistry,
    item: PendingUpdate,
) {
    match item {
        PendingUpdate::NeighborChanged { pos, from } => {
            if let Some(state) = ctx.get_block(pos) {
                let behavior = behaviors.resolve(state);
                behavior.on_neighbor_changed(ctx, pos, from);
            }
        }
        PendingUpdate::ShapeUpdate {
            pos,
            from,
            remaining_depth,
        } => {
            let Some(state) = ctx.get_block(pos) else {
                return;
            };
            let Some(neighbor_state) = ctx.get_block(from.apply(pos)) else {
                return;
            };
            let behavior = behaviors.resolve(state);
            if let Some(new_state) = behavior.on_shape_update(ctx, pos, from, neighbor_state) {
                ctx.write_block_state(pos, new_state);
                if remaining_depth > 0 {
                    ctx.engine
                        .emit_shape_update_fanout_at_depth(pos, remaining_depth - 1);
                }
            }
        }
    }
}
