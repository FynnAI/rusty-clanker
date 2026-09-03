//! The core fluid-spread algorithm (Context §C/§D/§E/§G/§H): `getNewLiquid`'s neighbor-driven
//! recompute, `getSpread`'s tie-preserving candidate search (with its own greedy-DFS
//! `getSlopeDistance` slope probe), `canBeReplacedWith`'s asymmetric water/lava rule, and
//! `getFlow`'s float/double-boundary-exact entity-push flow vector.
//!
//! Stub phase (test-authoring changeset, TEST-D45/D46): bodies are `todo!()`.
#![allow(unused_imports)]

use std::collections::HashMap;

use rc_core::BlockPos;
use rc_physics::Vec3;

use super::occlusion;
use super::state::{FLUID_HORIZONTAL_ORDER, FluidKind, FluidState};
use super::tables::FluidTables;
use super::waterlog::WaterloggableRegistry;
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

/// `world.get_block(pos)` resolved through `tables.ranges` — `None` if `pos` holds no fluid.
pub fn fluid_state_at(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
) -> Option<FluidState> {
    let _ = (world, tables, pos);
    todo!()
}

/// Context §C. `Ok` result is `None` for "should become empty/air", `Some(state)` otherwise.
pub fn get_new_liquid(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    kind: FluidKind,
) -> Option<FluidState> {
    let _ = (world, tables, pos, kind);
    todo!()
}

/// The 4-direction source-count scan `spread`'s "boxed in by 3+" rule uses (Context §D).
pub fn source_neighbor_count(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    kind: FluidKind,
) -> u32 {
    let _ = (world, tables, pos, kind);
    todo!()
}

/// Context §G. `existing_pos` is the position whose *current* fluid is being asked whether it
/// can be replaced by `incoming_kind` arriving from `incoming_dir`.
pub fn can_be_replaced_with(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    existing_pos: BlockPos,
    incoming_kind: FluidKind,
    incoming_dir: Direction,
) -> bool {
    let _ = (world, tables, existing_pos, incoming_kind, incoming_dir);
    todo!()
}

/// Context §E — the tie-preserving candidate search. Returns `(direction, candidate)` pairs;
/// `candidate: None` means "this candidate resolved to empty" (Context §K step 3's own
/// faithfully-preserved edge case), still a real map entry, not filtered out here.
pub fn get_spread(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    state: FluidState,
) -> Vec<(Direction, Option<FluidState>)> {
    let _ = (world, tables, waterlog, pos, state);
    todo!()
}

/// Private per-call memo lookup for `is_hole`, keyed by `(dx, dz)` relative to `origin` (Context
/// §E's `hole_cache`, scoped to one `get_spread` call tree).
fn is_hole_cached(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    kind: FluidKind,
    origin: BlockPos,
    cache: &mut HashMap<(i32, i32), bool>,
) -> bool {
    let _ = (world, tables, waterlog, pos, kind, origin, cache);
    todo!()
}

/// Context §E's greedy depth-first slope probe, private (only `get_spread` calls it).
#[allow(clippy::too_many_arguments)]
fn get_slope_distance(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    pass: u32,
    from: Direction,
    kind: FluidKind,
    origin: BlockPos,
    cache: &mut HashMap<(i32, i32), bool>,
) -> u32 {
    let _ = (
        world, tables, waterlog, pos, pass, from, kind, origin, cache,
    );
    todo!()
}

pub fn get_own_height(state: FluidState) -> f32 {
    let _ = state;
    todo!()
}

/// Context §A: `1.0` iff the cell directly above holds the same fluid kind (any variant), else
/// `get_own_height(state)`.
pub fn get_height(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    state: FluidState,
) -> f32 {
    let _ = (world, tables, pos, state);
    todo!()
}

/// Context §H — the complete entity-facing flow-field query.
pub fn get_flow(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    state: FluidState,
) -> Vec3 {
    let _ = (world, tables, pos, state);
    todo!()
}

/// Private helper (Context §H): length `< 1.0e-5f32` (widened to `f64`) ⇒ exactly `Vec3::ZERO`,
/// else each component divided by length. `rc-physics` ships no `normalize` method of its own
/// (Constraints (b)) — this stays local to this crate.
fn normalize_or_zero(v: Vec3) -> Vec3 {
    let _ = v;
    todo!()
}
