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
        Self::default()
    }

    pub fn get(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.0.get(&pos).copied()
    }

    pub(crate) fn record(&mut self, pos: BlockPos, state: BlockStateId) {
        self.0.insert(pos, state);
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
        Self {
            local,
            resolve: Box::new(move |_| local),
        }
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
    match d {
        Direction::West => 0,
        Direction::East => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::Down => 4,
        Direction::Up => 5,
    }
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
    let dimension = ctx.world.dimension();

    let mut owners: [Option<Address>; 6] = [None; 6];
    for dir in ALL_DIRECTIONS {
        let npos = dir.apply(pos);
        let owner = (ctx.ownership.resolve)(npos.chunk_key(dimension));
        owners[direction_ordinal(dir)] = Some(owner);
    }
    let owner_of = |dir: Direction| {
        owners[direction_ordinal(dir)].expect("populated above for all 6 directions")
    };

    for dir in NEIGHBOR_CHANGED_ORDER {
        let npos = dir.apply(pos);
        if owner_of(dir) == ctx.ownership.local {
            ctx.engine.emit_single(PendingUpdate::NeighborChanged {
                pos: npos,
                from: dir.opposite(),
            });
        } else {
            let chunk = npos.chunk_key(dimension);
            ctx.outbound.push((
                Address::Chunk(chunk),
                RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                    chunk,
                    pos,
                    kind: BorderUpdateKind::BlockChanged {
                        new_state: new_state.to_raw(),
                    },
                }),
            ));
        }
    }

    for dir in SHAPE_UPDATE_ORDER {
        let npos = dir.apply(pos);
        if owner_of(dir) == ctx.ownership.local {
            ctx.engine.emit_single(PendingUpdate::ShapeUpdate {
                pos: npos,
                from: dir.opposite(),
                remaining_depth: NeighborUpdateEngine::SHAPE_DEPTH,
            });
        }
    }
}

/// M3 field-report wave 3 fix (PLAN-D10, moving_piston placeholder — corrected fan-out
/// mapping): the shape-update-only counterpart to `fan_out_from_changed_block`, needed because
/// vanilla's own piston `moveBlocks` placeholder writes (`PistonBaseBlock.moveBlocks`'s own
/// push-loop/`armPos` writes) carry a flag lacking `UPDATE_NEIGHBORS` but NOT `UPDATE_KNOWN_
/// SHAPE`, so the write's own automatic shape recompute fires immediately while no automatic
/// neighbor-changed notification ever does. This is what makes a wire directly above a
/// freshly-placeholdered cell pop immediately: `MovingPistonBlock.getShape`'s own empty shape
/// (MECH-D84) is detected by exactly this pass, never by a neighbor-changed/redstone recompute
/// — `crates/mechanics/src/redstone/piston.rs`'s own top-of-file doc comment has the complete
/// per-write flag citation. Same per-direction ownership resolution and `SHAPE_UPDATE_ORDER`
/// traversal as `fan_out_from_changed_block`'s own step 3, restated here since a single-signal
/// caller has no combined step 2 to piggyback a `BorderUpdateEvent` on: for a non-local
/// direction this pass dispatches nothing at all — a real limitation shared with the combined
/// function's own identical no-message-of-its-own convention for its shape-update half
/// (MECH-D15's own cross-region shape-update propagation gap predates this fix and stays
/// exactly as wide).
pub fn fan_out_shape_update_only(ctx: &mut UpdateContext, pos: BlockPos) {
    let dimension = ctx.world.dimension();
    for dir in SHAPE_UPDATE_ORDER {
        let npos = dir.apply(pos);
        let owner = (ctx.ownership.resolve)(npos.chunk_key(dimension));
        if owner == ctx.ownership.local {
            ctx.engine.emit_single(PendingUpdate::ShapeUpdate {
                pos: npos,
                from: dir.opposite(),
                remaining_depth: NeighborUpdateEngine::SHAPE_DEPTH,
            });
        }
    }
}

/// M3 field-report wave 3 fix (PLAN-D10, moving_piston placeholder — corrected fan-out
/// mapping): the neighbor-changed-only counterpart, needed for vanilla's own EXPLICIT, later
/// `updateNeighborsAt` calls `moveBlocks` issues once every placeholder write in the same
/// accept-time batch has already landed (`armPos`, unconditionally, whenever extending; every
/// `to_push` chain element's own position too — `PistonBaseBlock.moveBlocks`'s own doc-cited
/// two loops, `write_extend_placeholders`'s own doc comment has the full derivation). These
/// positions already received their own automatic shape-update pass from
/// `fan_out_shape_update_only` above (called earlier, at write time, for every one of these
/// same positions) — this second, later, SEPARATE call adds only the neighbor-changed half,
/// never a duplicate shape-update. For a non-local direction this DOES push its own
/// `BorderUpdateEvent` — mirrors `fan_out_from_changed_block`'s own combined step 2 exactly,
/// carrying `new_state` (this position's own now-current, live content — always the
/// placeholder id by the time this fires, since every write in the batch already landed).
pub fn fan_out_neighbor_changed_only(
    ctx: &mut UpdateContext,
    pos: BlockPos,
    new_state: BlockStateId,
) {
    let dimension = ctx.world.dimension();
    for dir in NEIGHBOR_CHANGED_ORDER {
        let npos = dir.apply(pos);
        let owner = (ctx.ownership.resolve)(npos.chunk_key(dimension));
        if owner == ctx.ownership.local {
            ctx.engine.emit_single(PendingUpdate::NeighborChanged {
                pos: npos,
                from: dir.opposite(),
            });
        } else {
            let chunk = npos.chunk_key(dimension);
            ctx.outbound.push((
                Address::Chunk(chunk),
                RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                    chunk,
                    pos,
                    kind: BorderUpdateKind::BlockChanged {
                        new_state: new_state.to_raw(),
                    },
                }),
            ));
        }
    }
}

/// Applies one inbound `BorderUpdateEvent` (Context: "applying an inbound event does the
/// mirror operation"), using `ctx.ownership` for the same per-direction routing check as
/// `fan_out_from_changed_block`. For `BlockChanged`, records `halo[ev.pos] = new_state` first
/// (skipped for `NeighborChanged`). Then, for each `dir` in `direction::NEIGHBOR_CHANGED_ORDER`:
/// if `ctx.ownership.resolve(chunk_of(dir.apply(ev.pos)))` is local, `ctx.engine.emit_single`; if
/// non-local, dispatch nothing and push **no** message (never re-forward — this is what
/// prevents an infinite cross-border ping-pong).
pub fn apply_inbound_border_event(
    ctx: &mut UpdateContext,
    halo: &mut BorderHalo,
    ev: &BorderUpdateEvent,
) {
    match ev.kind {
        BorderUpdateKind::BlockChanged { new_state } => {
            halo.record(ev.pos, BlockStateId::from_raw(new_state));
        }
        BorderUpdateKind::NeighborChanged => {}
    }

    let dimension = ctx.world.dimension();
    for dir in NEIGHBOR_CHANGED_ORDER {
        let npos = dir.apply(ev.pos);
        let owner = (ctx.ownership.resolve)(npos.chunk_key(dimension));
        if owner == ctx.ownership.local {
            ctx.engine.emit_single(PendingUpdate::NeighborChanged {
                pos: npos,
                from: dir.opposite(),
            });
        }
    }
}
