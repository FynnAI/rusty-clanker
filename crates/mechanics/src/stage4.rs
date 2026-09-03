//! The Stage-4 driver's ECS-agnostic core (Context/Deliverables): applies border events,
//! drains scheduled ticks (block-before-fluid, MECH-D1), and runs the block-event sub-phase —
//! settling `NeighborUpdateEngine` to a fixed point after every individual triggering event,
//! never batched (reproducing vanilla's synchronous per-tick settling).

#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage4/ecs.rs

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_messaging::{Address, BorderUpdateEvent, RegionMessage};

use crate::behavior::{BlockBehaviorRegistry, UpdateContext};
use crate::block_event::BlockEventQueue;
use crate::border::{BorderHalo, RegionOwnership, apply_inbound_border_event};
use crate::light::LightDirtyQueue;
use crate::neighbor_update::{NeighborUpdateEngine, PendingUpdate};
use crate::scheduled_tick::ScheduledTickQueue;
use crate::world_access::BlockWorldAccess;

/// Bundles the eight `&mut`/`&` pieces `UpdateContext` needs into one value, freshly, at
/// every call site (Context: "the bundled-references pattern"). Each caller below passes its
/// own already-`&mut`-typed local parameters, which Rust implicitly reborrows at this
/// function-call boundary — so calling this repeatedly (once per loop iteration, or once per
/// `NeighborUpdateEngine::drain` handler invocation) never moves anything out from under the
/// caller. M4-B07: `light_dirty` is the newest of these, the enqueue seam into Stage 8's own
/// light recompute (`UpdateContext::set_block`'s extended body).
#[allow(clippy::too_many_arguments)]
fn make_ctx<'a>(
    world: &'a mut dyn BlockWorldAccess,
    engine: &'a mut NeighborUpdateEngine,
    scheduled: &'a mut ScheduledTickQueue,
    events: &'a mut BlockEventQueue,
    outbound: &'a mut Vec<(Address, RegionMessage)>,
    changed: &'a mut Vec<(BlockPos, BlockStateId)>,
    light_dirty: &'a mut LightDirtyQueue,
    ownership: &'a RegionOwnership,
    current_tick: u64,
) -> UpdateContext<'a> {
    UpdateContext {
        world,
        engine,
        scheduled,
        events,
        outbound,
        changed,
        ownership,
        current_tick,
        light_dirty,
    }
}

/// Drains `engine` to a fixed point (Context: one `NeighborUpdateEngine::drain` call already
/// settles a whole reentrant chain by itself), dispatching every popped `PendingUpdate` to the
/// matching `BlockBehavior` method via `dispatch_pending_update`.
///
/// `pub(crate)` (M3 field-report fix, Section C production half): `crate::stage7::run_container_
/// signal_notify` is this function's second call site -- the Stage-7 -> Stage-4 redstone notify
/// bridge reuses this exact dispatch/fixed-point-drain logic rather than duplicating it, mirroring
/// `crates/testing/gametest/src/replay.rs`'s own already-proven `engine.drain(&mut |eng, item| ...
/// dispatch_one(...))` composition.
#[allow(clippy::too_many_arguments)]
pub(crate) fn drain_engine(
    world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, BlockStateId)>,
    light_dirty: &mut LightDirtyQueue,
    ownership: &RegionOwnership,
    current_tick: u64,
    behaviors: &BlockBehaviorRegistry,
) {
    engine.drain(&mut |eng, item| {
        let mut ctx = make_ctx(
            world,
            eng,
            scheduled,
            events,
            outbound,
            changed,
            light_dirty,
            ownership,
            current_tick,
        );
        dispatch_pending_update(&mut ctx, behaviors, item);
    });
}

/// One popped `PendingUpdate`'s complete dispatch: resolves the target position's own
/// registered behavior and calls the matching trait method. A `ShapeUpdate` whose behavior
/// requests a state change (`Some(new_state)`) is written directly via `ctx.world.set_block`
/// (not `ctx.set_block` — a mid-cascade shape/connection-state write does not itself restart a
/// brand-new neighbor-changed + shape-update fan-out from this same position; only a
/// `BlockBehavior::on_neighbor_changed`/`on_scheduled_tick`/`on_block_event` handler's own
/// explicit `ctx.set_block` call, or this function's own explicit continuation below, ever
/// grows the chain), then continues the shape-update cascade one hop further (`remaining_depth
/// - 1`) if depth remains — reproducing vanilla's own decrementing-budget cascade without
/// requiring every future concrete `BlockBehavior` implementation to reimplement that
/// bookkeeping itself (`PendingUpdate::ShapeUpdate.remaining_depth` is not part of
/// `BlockBehavior::on_shape_update`'s own signature — only this dispatch glue sees it).
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

#[allow(clippy::too_many_arguments)]
fn dispatch_scheduled_tick(
    world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, BlockStateId)>,
    light_dirty: &mut LightDirtyQueue,
    ownership: &RegionOwnership,
    current_tick: u64,
    behaviors: &BlockBehaviorRegistry,
    pos: BlockPos,
) {
    let Some(state) = world.get_block(pos) else {
        return;
    };
    let behavior = behaviors.resolve(state);
    let mut ctx = make_ctx(
        world,
        engine,
        scheduled,
        events,
        outbound,
        changed,
        light_dirty,
        ownership,
        current_tick,
    );
    behavior.on_scheduled_tick(&mut ctx, pos);
}

/// `system_scheduled_phase`'s ECS-agnostic core: applies every inbound border event (ARCH-D11's
/// "first sub-step"), then drains due block ticks completely, then due fluid ticks completely
/// (MECH-D1's own order — Context), dispatching each to `behaviors.resolve(state).on_scheduled_tick`
/// and draining the neighbor-update engine to a fixed point after **each individual** due entry
/// (not batched) — reproducing vanilla's synchronous per-tick settling.
///
/// M3 field-report fix (Section B3): brackets the whole call with `events.begin_scheduled_phase_
/// dispatch()`/`end_scheduled_phase_dispatch()` (`block_event.rs`'s own doc comment has the full
/// rationale) — any `ctx.emit_block_event` reached from inside this function (typically a
/// piston's own finalization, `commit_extend`/`commit_retract`, fanning out to a neighbor
/// piston's own `on_neighbor_changed`) is held back a full extra `run_block_event_subphase`
/// call, landing on the *next* tick's own block-event pass rather than this same tick's.
#[allow(clippy::too_many_arguments)]
pub fn run_scheduled_phase(
    world: &mut dyn BlockWorldAccess,
    inbound: &[BorderUpdateEvent],
    halo: &mut BorderHalo,
    ownership: &RegionOwnership,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, BlockStateId)>,
    light_dirty: &mut LightDirtyQueue,
    current_tick: u64,
) {
    events.begin_scheduled_phase_dispatch();
    for ev in inbound {
        let mut ctx = make_ctx(
            world,
            engine,
            scheduled,
            events,
            outbound,
            changed,
            light_dirty,
            ownership,
            current_tick,
        );
        apply_inbound_border_event(&mut ctx, halo, ev);
        drain_engine(
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

    let due_block = scheduled.drain_due_block_ticks(current_tick);
    for entry in due_block {
        dispatch_scheduled_tick(
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
            entry.pos,
        );
        drain_engine(
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

    let due_fluid = scheduled.drain_due_fluid_ticks(current_tick);
    for entry in due_fluid {
        dispatch_scheduled_tick(
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
            entry.pos,
        );
        drain_engine(
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
    events.end_scheduled_phase_dispatch();
}

/// Defensive per-pass cap on how many block events one `run_block_event_subphase` call will
/// pop and dispatch (Context: mirrors `NeighborUpdateEngine::DEFAULT_CHAIN_LIMIT`'s identical
/// defensive role, at the same order of magnitude). Vanilla's own `while (!queue.is_empty())`
/// loop is naturally bounded by ordinary game logic -- no legal contraption re-queues a block
/// event forever -- so this cap is purely this project's own non-vanilla safety net against a
/// hypothetical runaway self-feeding loop; no legal contraption should ever reach it. Tripping
/// it stops the loop for *this* call only: whatever is still queued in `events` when that
/// happens is left exactly where it is (`BlockEventQueue` has no separate discard path) and is
/// simply picked up by the *next* tick's own `run_block_event_subphase` call instead.
const BLOCK_EVENT_PASS_CAP: u32 = 1_000_000;

/// `system_block_event_subphase`'s ECS-agnostic core (MECH-D9): pops and dispatches one event
/// at a time from `events`' own live queue, via `behaviors.resolve(event.block_state).
/// on_block_event`, draining the neighbor-update engine to a fixed point after each one
/// (mirrors `run_scheduled_phase`'s per-item settling). The loop keeps popping -- re-entrantly
/// picking up anything a handler's own `ctx.emit_block_event` call queues mid-loop, whether
/// directly or via a `ctx.set_block` fan-out that reaches another position's own
/// `on_neighbor_changed` -- until `events` is empty or `BLOCK_EVENT_PASS_CAP` trips, reproducing
/// vanilla's own same-tick, same-pass re-entrant cascade exactly: nothing emitted *during this
/// call* is ever deferred to a later tick purely because it was emitted during this call.
///
/// M3 field-report fix (Section B3): calls `events.begin_pass()` before the draining loop below
/// starts (`block_event.rs`'s own doc comment has the full rationale) -- advances the two-
/// generation rotation that holds back anything `run_scheduled_phase`'s own dispatch emitted
/// (typically a piston's own finalization fanning out to a neighbor piston) for a full extra
/// `run_block_event_subphase` call, so it lands on the *next* tick's own pass rather than this
/// same tick's.
#[allow(clippy::too_many_arguments)]
pub fn run_block_event_subphase(
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
) {
    events.begin_pass();
    let mut processed: u32 = 0;
    loop {
        if processed >= BLOCK_EVENT_PASS_CAP {
            tracing::warn!(
                cap = BLOCK_EVENT_PASS_CAP,
                still_queued = events.pending(),
                "run_block_event_subphase: per-pass block-event cap reached -- the remaining \
                 events queued this pass are left for next tick's own call (non-vanilla safety \
                 net; no legal contraption should ever hit this)"
            );
            break;
        }
        let Some(event) = events.pop_next() else {
            break;
        };
        processed += 1;
        {
            let behavior = behaviors.resolve(event.block_state);
            let mut ctx = make_ctx(
                world,
                engine,
                scheduled,
                events,
                outbound,
                changed,
                light_dirty,
                ownership,
                current_tick,
            );
            behavior.on_block_event(&mut ctx, event.pos, &event);
        }
        drain_engine(
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
}
