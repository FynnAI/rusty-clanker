//! The Stage-4 driver's ECS-agnostic core (Context/Deliverables): applies border events,
//! drains scheduled ticks (block-before-fluid, MECH-D1), and runs the block-event sub-phase —
//! settling `NeighborUpdateEngine` to a fixed point after every individual triggering event,
//! never batched (reproducing vanilla's synchronous per-tick settling).

#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage4/ecs.rs

use rc_messaging::{Address, BorderUpdateEvent, RegionMessage};

use crate::behavior::{BlockBehaviorRegistry, UpdateContext};
use crate::block_event::BlockEventQueue;
use crate::border::{apply_inbound_border_event, BorderHalo, RegionOwnership};
use crate::neighbor_update::{NeighborUpdateEngine, PendingUpdate};
use crate::scheduled_tick::ScheduledTickQueue;
use crate::world_access::BlockWorldAccess;

/// Bundles the seven `&mut`/`&` pieces `UpdateContext` needs into one value, freshly, at
/// every call site (Context: "the bundled-references pattern"). Each caller below passes its
/// own already-`&mut`-typed local parameters, which Rust implicitly reborrows at this
/// function-call boundary — so calling this repeatedly (once per loop iteration, or once per
/// `NeighborUpdateEngine::drain` handler invocation) never moves anything out from under the
/// caller.
#[allow(clippy::too_many_arguments)]
fn make_ctx<'a>(
    world: &'a mut dyn BlockWorldAccess,
    engine: &'a mut NeighborUpdateEngine,
    scheduled: &'a mut ScheduledTickQueue,
    events: &'a mut BlockEventQueue,
    outbound: &'a mut Vec<(Address, RegionMessage)>,
    ownership: &'a RegionOwnership,
    current_tick: u64,
) -> UpdateContext<'a> {
    todo!()
}

/// Drains `engine` to a fixed point (Context: one `NeighborUpdateEngine::drain` call already
/// settles a whole reentrant chain by itself), dispatching every popped `PendingUpdate` to the
/// matching `BlockBehavior` method via `dispatch_pending_update`.
#[allow(clippy::too_many_arguments)]
fn drain_engine(
    world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    ownership: &RegionOwnership,
    current_tick: u64,
    behaviors: &BlockBehaviorRegistry,
) {
    todo!()
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
fn dispatch_pending_update(ctx: &mut UpdateContext, behaviors: &BlockBehaviorRegistry, item: PendingUpdate) {
    todo!()
}

#[allow(clippy::too_many_arguments)]
fn dispatch_scheduled_tick(
    world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    ownership: &RegionOwnership,
    current_tick: u64,
    behaviors: &BlockBehaviorRegistry,
    pos: rc_core::BlockPos,
) {
    todo!()
}

/// `system_scheduled_phase`'s ECS-agnostic core: applies every inbound border event (ARCH-D11's
/// "first sub-step"), then drains due block ticks completely, then due fluid ticks completely
/// (MECH-D1's own order — Context), dispatching each to `behaviors.resolve(state).on_scheduled_tick`
/// and draining the neighbor-update engine to a fixed point after **each individual** due entry
/// (not batched) — reproducing vanilla's synchronous per-tick settling.
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
    current_tick: u64,
) {
    todo!()
}

/// `system_block_event_subphase`'s ECS-agnostic core: `events.begin_subphase()`, dispatch each
/// to `behaviors.resolve(event.block_state).on_block_event`, draining the neighbor-update
/// engine to a fixed point after each event (mirrors `run_scheduled_phase`'s per-item
/// settling). Anything emitted via `events.emit` during this call lands in the queue's fresh
/// `next` buffer, deferred to next tick's call (MECH-D9).
#[allow(clippy::too_many_arguments)]
pub fn run_block_event_subphase(
    world: &mut dyn BlockWorldAccess,
    ownership: &RegionOwnership,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    outbound: &mut Vec<(Address, RegionMessage)>,
    current_tick: u64,
) {
    todo!()
}
