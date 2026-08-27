use bevy_ecs::prelude::Resource;
use rc_chunk_storage::{BlockStateId, RegistryId};
use rc_core::{BlockPos, ChunkKey};
use rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, RegionMessage};
use std::collections::HashMap;

use crate::behavior::UpdateContext;
use crate::direction::{Direction, NEIGHBOR_CHANGED_ORDER, SHAPE_UPDATE_ORDER};
use crate::neighbor_update::{NeighborUpdateEngine, PendingUpdate};

/// Lazy, minimal cross-region read cache (Context: "a bounded, explicitly-scoped stand-in" —
/// not MECH-D18's full one-chunk halo). Populated only by inbound `BlockChanged` events.
#[derive(Debug, Default, Resource)]
pub struct BorderHalo(HashMap<BlockPos, BlockStateId>);

impl BorderHalo {
    pub fn new() -> Self {
        todo!()
    }

    pub fn get(&self, pos: BlockPos) -> Option<BlockStateId> {
        todo!()
    }

    pub(crate) fn record(&mut self, pos: BlockPos, state: BlockStateId) {
        todo!()
    }
}

/// This region's own identity plus the ARCH-D24-directory stand-in (Context; mirrors
/// M2-B07's/M0-B03's own identical-purpose stand-ins). Held by `UpdateContext::ownership`. Has
/// no `Default` (no sensible default `resolve` closure exists) — every region's bootstrap
/// function must insert one explicitly (Implementation steps).
#[derive(Resource)]
pub struct RegionOwnership {
    pub local: Address,
    pub resolve: Box<dyn Fn(ChunkKey) -> Address + Send + Sync>,
}

impl RegionOwnership {
    /// A `RegionOwnership` whose `resolve` always returns `local` — every position is
    /// considered local (this blueprint's own single-region test convenience; not a
    /// production default).
    pub fn always_local(local: Address) -> Self {
        todo!()
    }
}

/// All 6 `Direction` values, fixed order (used only to resolve each direction's owner exactly
/// once, up front — Deliverables step 1 of `fan_out_from_changed_block`; the pass itself is
/// explicitly order-independent).
const ALL_DIRECTIONS: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::Down,
    Direction::Up,
];

fn direction_ordinal(d: Direction) -> usize {
    todo!()
}

/// Fans both signals out from a block at `pos` that just changed to `new_state` (called only
/// from `UpdateContext::set_block`, which supplies `ctx`). `chunk_of(p) = p.chunk_key(ctx.world.
/// dimension())` throughout. Algorithm, precisely (the ownership check per direction happens
/// **once**, up front, shared by both passes below — this is what keeps a non-local direction
/// from producing two duplicate `BorderUpdateEvent`s, one per signal, since ownership never
/// depends on which signal is being fanned out, only on the neighbor position):
/// 1. For each of the 6 `Direction`s (any order — this pass is order-independent), resolve
///    `ctx.ownership.resolve(chunk_of(dir.apply(pos)))` once and remember it.
/// 2. **Neighbor-changed pass**, in `direction::NEIGHBOR_CHANGED_ORDER`: for each `dir`, if its
///    remembered owner is `ctx.ownership.local`, call `ctx.engine.emit_single` (per-direction
///    dispatch — not the bulk `emit_neighbor_changed_fanout` convenience method, since some
///    directions in this same pass may instead route cross-region); otherwise push exactly one
///    `RegionMessage::BorderUpdateEvent` onto `ctx.outbound`, addressed to
///    `Address::Chunk(dir.apply(pos))`'s chunk.
/// 3. **Shape-update pass**, in `direction::SHAPE_UPDATE_ORDER`: for each `dir`, if its
///    remembered owner is local, `emit_single` a `ShapeUpdate` item; if non-local, dispatch
///    **nothing** — step 2 already pushed that direction's one and only `BorderUpdateEvent`
///    (Context: "`MECH-D15`'s... distinction is not preserved across a region border" — one
///    message already covers it).
pub fn fan_out_from_changed_block(ctx: &mut UpdateContext, pos: BlockPos, new_state: BlockStateId) {
    todo!()
}

/// Applies one inbound `BorderUpdateEvent` (Context: "applying an inbound event does the
/// mirror operation"), using `ctx.ownership` for the same per-direction routing check as
/// `fan_out_from_changed_block`. For `BlockChanged`, records `halo[ev.pos] = new_state` first
/// (skipped for `NeighborChanged`). Then, for each `dir` in `direction::NEIGHBOR_CHANGED_ORDER`:
/// if `ctx.ownership.resolve(chunk_of(dir.apply(ev.pos)))` is local, `ctx.engine.emit_single`; if
/// non-local, dispatch nothing and push **no** message (never re-forward — this is what
/// prevents an infinite cross-border ping-pong).
pub fn apply_inbound_border_event(ctx: &mut UpdateContext, halo: &mut BorderHalo, ev: &BorderUpdateEvent) {
    todo!()
}
