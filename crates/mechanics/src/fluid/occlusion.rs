//! The shape-occlusion gate (Context §F): `can_pass_through_wall`/`can_maybe_pass_through`/
//! `can_hold_any_fluid`/`can_hold_specific_fluid` and the two solidity/sturdiness predicates
//! (`is_solid`/`is_solid_face`) they and `algorithm.rs`'s `get_new_liquid`/`get_flow` build on.
//! Deliberately implements only the two bounded shape fast-paths the research corpus itself
//! names (full-cube / empty), never the general per-face `mergedFaceOccludes` merge.
//!
//! Stub phase (test-authoring changeset, TEST-D45/D46): bodies are `todo!()`.
#![allow(unused_imports)]

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_physics::VoxelShape;

use super::state::FluidKind;
use super::tables::FluidTables;
use super::waterlog::WaterloggableRegistry;
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

/// `true` iff `pos`'s shape (via `rc_physics::shapes::tier1_shape_table()`) is exactly
/// `VoxelShape::full_cube()`. Unregistered/no-block-there ids default full-cube (`rc-physics`'s
/// own registry default) — matches vanilla's own "ordinary terrain is a conductor" default.
pub fn is_full_cube(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
    let _ = (world, pos);
    todo!()
}

/// `true` iff `pos`'s shape `.is_empty()`.
pub fn is_empty_shape(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
    let _ = (world, pos);
    todo!()
}

/// The pure geometry half of vanilla's `calculateSolid` (Context §F), directly testable against
/// a hand-constructed shape with no `BlockWorldAccess`/registry involved.
pub fn is_solid_shape(shape: &VoxelShape) -> bool {
    let _ = shape;
    todo!()
}

/// Context §F: vanilla's own solidity cache (`calculateSolid`) — `tables.force_solid_on`, then
/// `tables.force_solid_off` (both empty by default), are checked first, in that order, and win
/// unconditionally over the shape; failing both, resolves `pos`'s shape via `tier1_shape_table()`
/// and delegates to `is_solid_shape`. Used by §C's source-conversion floor check.
pub fn is_solid(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> bool {
    let _ = (world, tables, pos);
    todo!()
}

/// Context §F: `is_solid(pos)` further excluding cobweb and bamboo sapling by block kind. Used by
/// §H's `getFlow` redirect gate.
pub fn blocks_motion(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> bool {
    let _ = (world, tables, pos);
    todo!()
}

/// The pure per-face half of vanilla's real sturdiness test (`SupportType.FULL`, Context §F),
/// directly testable against a hand-constructed shape.
pub fn is_face_sturdy_shape(shape: &VoxelShape, dir: Direction) -> bool {
    let _ = (shape, dir);
    todo!()
}

/// Context §F: false if the fluid at `pos` is `kind` (same-fluid short-circuit); false if `pos`
/// is in `tables.solid_face_exceptions` (ice reservation); else resolves `pos`'s shape via
/// `tier1_shape_table()` and delegates to `is_face_sturdy_shape`. `dir` is never `Up` in any call
/// this blueprint makes.
pub fn is_solid_face(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    kind: FluidKind,
    pos: BlockPos,
    dir: Direction,
) -> bool {
    let _ = (world, tables, kind, pos, dir);
    todo!()
}

/// Context §F's two fast paths only; every other (non-full, non-empty) shape passes.
pub fn can_pass_through_wall(
    world: &dyn BlockWorldAccess,
    source_pos: BlockPos,
    target_pos: BlockPos,
    dir: Direction,
) -> bool {
    let _ = (world, source_pos, target_pos, dir);
    todo!()
}

/// Real vanilla `canHoldAnyFluid` (Context §F): unconditionally `true` when `pos` is registered
/// in `waterlog` (a container, regardless of shape); else `false` when `blocks_motion(pos)`; else
/// `!deny_hold_fluid.contains(pos)`.
pub fn can_hold_any_fluid(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
) -> bool {
    let _ = (world, tables, waterlog, pos);
    todo!()
}

/// Real vanilla `canHoldSpecificFluid` (Context §F): `LiquidBlockContainer` at target ⇒ delegate
/// to `can_place_liquid`, else `true` — carries no denylist of its own.
pub fn can_hold_specific_fluid(
    world: &dyn BlockWorldAccess,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    kind: FluidKind,
) -> bool {
    let _ = (world, waterlog, pos, kind);
    todo!()
}

/// `!is_source_of(target, kind) && can_hold_any_fluid(world, tables, waterlog, target_pos) &&
/// can_pass_through_wall(...)`.
#[allow(clippy::too_many_arguments)]
pub fn can_maybe_pass_through(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    source_pos: BlockPos,
    target_pos: BlockPos,
    dir: Direction,
    kind: FluidKind,
) -> bool {
    let _ = (world, tables, waterlog, source_pos, target_pos, dir, kind);
    todo!()
}

/// The slope-search variant (Context §E): `can_maybe_pass_through` plus
/// `can_hold_specific_fluid` against the abstract flowing type.
#[allow(clippy::too_many_arguments)]
pub fn can_pass_through(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    source_pos: BlockPos,
    target_pos: BlockPos,
    dir: Direction,
    kind: FluidKind,
) -> bool {
    let _ = (world, tables, waterlog, source_pos, target_pos, dir, kind);
    todo!()
}

/// `is_water_hole` (Context §E/§D) — fluid-agnostic name kept per vanilla's own (misleading)
/// naming. The "could structurally hold this kind at all" half is real vanilla's own
/// `canHoldFluid` combinator (`can_hold_any_fluid(below) && can_hold_specific_fluid(below,
/// kind)`), computed inline here since it has exactly one caller in this blueprint.
pub fn is_hole(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    kind: FluidKind,
) -> bool {
    let _ = (world, tables, waterlog, pos, kind);
    todo!()
}

/// Shared range-membership test for `tables`' four caller-supplied override/denylist fields
/// (`force_solid_on`/`force_solid_off`/`deny_hold_fluid`/`solid_face_exceptions`) — not part of
/// Deliverables' own public listing, a private helper local to this module.
fn range_contains(ranges: &[(BlockStateId, BlockStateId)], id: BlockStateId) -> bool {
    let _ = (ranges, id);
    todo!()
}

/// `true` iff the existing fluid at `pos` is already a `Source` of `kind` — the
/// `can_maybe_pass_through` precondition Deliverables' own doc comment names (`!is_source_of(...)`
/// ), not part of Deliverables' own public listing, a private helper local to this module.
fn is_source_of(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    kind: FluidKind,
) -> bool {
    let _ = (world, tables, pos, kind);
    todo!()
}
