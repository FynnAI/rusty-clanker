//! The shape-occlusion gate (Context §F): `can_pass_through_wall`/`can_maybe_pass_through`/
//! `can_hold_any_fluid`/`can_hold_specific_fluid` and the two solidity/sturdiness predicates
//! (`is_solid`/`is_solid_face`) they and `algorithm.rs`'s `get_new_liquid`/`get_flow` build on.
//! Deliberately implements only the two bounded shape fast-paths the research corpus itself
//! names (full-cube / empty), never the general per-face `mergedFaceOccludes` merge.

use rc_chunk_storage::{BlockStateId, RegistryId};
use rc_core::BlockPos;
use rc_physics::{VoxelShape, tier1_shape_table};

use super::state::FluidKind;
use super::tables::FluidTables;
use super::waterlog::WaterloggableRegistry;
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

/// Resolves `pos`'s own shape for every occlusion/solidity predicate in this module. A position
/// currently holding *any* registered fluid (`tables.ranges.kind_of`) always resolves to
/// `VoxelShape::empty()` — real vanilla's `LiquidBlock.getCollisionShape()` is unconditionally
/// `Shapes.empty()` regardless of level, and `rc_physics::tier1_shape_table()` (a fixed,
/// hand-authored table this blueprint must not modify, Constraints (b)) carries no entry for
/// either fluid range at all — its own unregistered-id default is a full cube, the exact
/// opposite of a fluid's real shape, which would otherwise make every already-fluid neighbor
/// read as an impassable wall to `can_pass_through_wall`, breaking the spread algorithm's own
/// same-kind-neighbor reads outright. A position outside `world`'s own knowledge entirely (no
/// block loaded/indexed there — `world.get_block` returns `None`) resolves to
/// `VoxelShape::full_cube()`, the conservative "ordinary terrain is a conductor" default this
/// crate's own occlusion checks apply everywhere else, matching vanilla's own
/// unloaded-is-solid-until-proven-otherwise posture and, structurally, preventing `spread_to`
/// from ever attempting a write to a position outside the caller's own local `BlockWorldAccess`
/// (Context §N's own documented, bounded cross-region read gap).
fn shape_at(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> VoxelShape {
    match world.get_block(pos) {
        Some(id) if tables.ranges.kind_of(id).is_some() => VoxelShape::empty(),
        Some(id) => tier1_shape_table().lookup(id.to_raw()).shape,
        None => VoxelShape::full_cube(),
    }
}

/// `true` iff `pos`'s shape (via `rc_physics::shapes::tier1_shape_table()`) is exactly
/// `VoxelShape::full_cube()`. Unregistered/no-block-there ids default full-cube (`rc-physics`'s
/// own registry default) — matches vanilla's own "ordinary terrain is a conductor" default.
pub fn is_full_cube(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> bool {
    shape_at(world, tables, pos) == VoxelShape::full_cube()
}

/// `true` iff `pos`'s shape `.is_empty()`.
pub fn is_empty_shape(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> bool {
    shape_at(world, tables, pos).is_empty()
}

/// The pure geometry half of vanilla's `calculateSolid` (Context §F), directly testable against
/// a hand-constructed shape with no `BlockWorldAccess`/registry involved.
pub fn is_solid_shape(shape: &VoxelShape) -> bool {
    if shape.is_empty() {
        return false;
    }
    let boxes = shape.boxes();
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for b in boxes {
        let b_min = [b.min.x, b.min.y, b.min.z];
        let b_max = [b.max.x, b.max.y, b.max.z];
        for axis in 0..3 {
            min[axis] = min[axis].min(b_min[axis]);
            max[axis] = max[axis].max(b_max[axis]);
        }
    }
    let x_ext = max[0] - min[0];
    let y_ext = max[1] - min[1];
    let z_ext = max[2] - min[2];
    let mean_edge = (x_ext + y_ext + z_ext) / 3.0;
    mean_edge >= 0.7291666666666666 || y_ext >= 1.0
}

fn range_contains(ranges: &[(BlockStateId, BlockStateId)], id: BlockStateId) -> bool {
    ranges
        .iter()
        .any(|(start, end)| id.0 >= start.0 && id.0 < end.0)
}

/// Context §F: vanilla's own solidity cache (`calculateSolid`) — `tables.force_solid_on`, then
/// `tables.force_solid_off` (both empty by default), are checked first, in that order, and win
/// unconditionally over the shape; failing both, resolves `pos`'s shape via `tier1_shape_table()`
/// and delegates to `is_solid_shape`. Used by §C's source-conversion floor check.
pub fn is_solid(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> bool {
    if let Some(id) = world.get_block(pos) {
        if range_contains(&tables.force_solid_on, id) {
            return true;
        }
        if range_contains(&tables.force_solid_off, id) {
            return false;
        }
    }
    is_solid_shape(&shape_at(world, tables, pos))
}

/// Context §F: `is_solid(pos)` further excluding cobweb and bamboo sapling by block kind. Used by
/// §H's `getFlow` redirect gate. Neither block kind exists in this blueprint's own tier-1/tier-2
/// placeable set (Context §F), so this reduces to `is_solid` for every block this blueprint's
/// own acceptance tests can construct today — the kind-exclusion is kept only for definitional
/// accuracy, matching real vanilla's own `blocksMotion()` contract.
pub fn blocks_motion(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> bool {
    is_solid(world, tables, pos)
}

/// The pure per-face half of vanilla's real sturdiness test (`SupportType.FULL`, Context §F),
/// directly testable against a hand-constructed shape.
pub fn is_face_sturdy_shape(shape: &VoxelShape, dir: Direction) -> bool {
    let (axis, boundary_is_min): (usize, bool) = match dir {
        Direction::West => (0, true),
        Direction::East => (0, false),
        Direction::Down => (1, true),
        Direction::Up => (1, false),
        Direction::North => (2, true),
        Direction::South => (2, false),
    };
    for b in shape.boxes() {
        let min = [b.min.x, b.min.y, b.min.z];
        let max = [b.max.x, b.max.y, b.max.z];
        let touches_boundary = if boundary_is_min {
            min[axis] <= 0.0
        } else {
            max[axis] >= 1.0
        };
        if !touches_boundary {
            continue;
        }
        let mut spans_full = true;
        for other in 0..3 {
            if other == axis {
                continue;
            }
            if min[other] > 0.0 || max[other] < 1.0 {
                spans_full = false;
                break;
            }
        }
        if spans_full {
            return true;
        }
    }
    false
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
    if let Some(id) = world.get_block(pos) {
        if tables.ranges.kind_of(id) == Some(kind) {
            return false;
        }
        if range_contains(&tables.solid_face_exceptions, id) {
            return false;
        }
    }
    is_face_sturdy_shape(&shape_at(world, tables, pos), dir)
}

/// Context §F's two fast paths only; every other (non-full, non-empty) shape passes. `dir` is
/// not consulted by these fast paths (the general per-face `mergedFaceOccludes` merge, which
/// would need it, is out of this blueprint's own bounded scope) — kept in the signature to match
/// vanilla's own `canPassThroughWall(direction, ...)` shape and to leave the seam open for
/// whichever future blueprint completes it.
pub fn can_pass_through_wall(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    source_pos: BlockPos,
    target_pos: BlockPos,
    _dir: Direction,
) -> bool {
    if is_full_cube(world, tables, source_pos) {
        return false;
    }
    if is_full_cube(world, tables, target_pos) {
        return false;
    }
    if is_empty_shape(world, tables, source_pos) && is_empty_shape(world, tables, target_pos) {
        return true;
    }
    true
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
    if let Some(id) = world.get_block(pos)
        && waterlog.resolve(id).is_some()
    {
        return true;
    }
    if blocks_motion(world, tables, pos) {
        return false;
    }
    match world.get_block(pos) {
        Some(id) => !range_contains(&tables.deny_hold_fluid, id),
        None => true,
    }
}

/// Real vanilla `canHoldSpecificFluid` (Context §F): `LiquidBlockContainer` at target ⇒ delegate
/// to `can_place_liquid`, else `true` — carries no denylist of its own.
pub fn can_hold_specific_fluid(
    world: &dyn BlockWorldAccess,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    kind: FluidKind,
) -> bool {
    let Some(id) = world.get_block(pos) else {
        return true;
    };
    match waterlog.resolve(id) {
        Some(behavior) => behavior.can_place_liquid(world, pos, id, kind),
        None => true,
    }
}

/// `true` iff the existing fluid at `pos` is already a `Source` of `kind` — the
/// `can_maybe_pass_through` precondition Deliverables' own doc comment names
/// (`!is_source_of(...)`), a private helper local to this module.
fn is_source_of(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    kind: FluidKind,
) -> bool {
    match world
        .get_block(pos)
        .and_then(|id| tables.ranges.state_of(id))
    {
        Some(state) => state.kind == kind && state.is_source(),
        None => false,
    }
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
    if is_source_of(world, tables, target_pos, kind) {
        return false;
    }
    can_hold_any_fluid(world, tables, waterlog, target_pos)
        && can_pass_through_wall(world, tables, source_pos, target_pos, dir)
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
    can_maybe_pass_through(world, tables, waterlog, source_pos, target_pos, dir, kind)
        && can_hold_specific_fluid(world, waterlog, target_pos, kind)
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
    let below = Direction::Down.apply(pos);
    if !can_pass_through_wall(world, tables, pos, below, Direction::Down) {
        return false;
    }
    let same_kind_already = matches!(
        world.get_block(below).and_then(|id| tables.ranges.state_of(id)),
        Some(state) if state.kind == kind
    );
    if same_kind_already {
        return true;
    }
    can_hold_any_fluid(world, tables, waterlog, below)
        && can_hold_specific_fluid(world, waterlog, below, kind)
}
