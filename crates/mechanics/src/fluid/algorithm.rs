//! The core fluid-spread algorithm (Context §C/§D/§E/§G/§H): `getNewLiquid`'s neighbor-driven
//! recompute, `getSpread`'s tie-preserving candidate search (with its own greedy-DFS
//! `getSlopeDistance` slope probe), `canBeReplacedWith`'s asymmetric water/lava rule, and
//! `getFlow`'s float/double-boundary-exact entity-push flow vector.

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
    world
        .get_block(pos)
        .and_then(|id| tables.ranges.state_of(id))
}

/// Context §C. `Ok` result is `None` for "should become empty/air", `Some(state)` otherwise.
pub fn get_new_liquid(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    kind: FluidKind,
) -> Option<FluidState> {
    let mut highest: u8 = 0;
    let mut sources: u32 = 0;

    for dir in FLUID_HORIZONTAL_ORDER {
        let npos = dir.apply(pos);
        if let Some(nstate) = fluid_state_at(world, tables, npos)
            && nstate.kind == kind
            && occlusion::can_pass_through_wall(world, tables, pos, npos, dir)
        {
            if nstate.is_source() {
                sources += 1;
            }
            highest = highest.max(nstate.amount());
        }
    }

    if sources >= 2 && tables.gamerules.allows_source_conversion(kind) {
        let below = Direction::Down.apply(pos);
        let below_solid = occlusion::is_solid(world, tables, below);
        let below_is_source = matches!(
            fluid_state_at(world, tables, below),
            Some(s) if s.kind == kind && s.is_source()
        );
        if below_solid || below_is_source {
            return Some(FluidState::source(kind));
        }
    }

    let above = Direction::Up.apply(pos);
    if let Some(above_state) = fluid_state_at(world, tables, above)
        && above_state.kind == kind
        && occlusion::can_pass_through_wall(world, tables, pos, above, Direction::Up)
    {
        return Some(FluidState::flowing(kind, 8, true));
    }

    let drop = tables.drop_off(kind);
    if highest <= drop {
        None
    } else {
        Some(FluidState::flowing(kind, highest - drop, false))
    }
}

/// The 4-direction source-count scan `spread`'s "boxed in by 3+" rule uses (Context §D). No
/// occlusion gating -- the same 4-direction scan, counting cells whose fluid `isSame(this) &&
/// isSource()` (research corpus §3.3, restated exactly).
pub fn source_neighbor_count(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    kind: FluidKind,
) -> u32 {
    let mut count = 0;
    for dir in FLUID_HORIZONTAL_ORDER {
        let npos = dir.apply(pos);
        if let Some(nstate) = fluid_state_at(world, tables, npos)
            && nstate.kind == kind
            && nstate.is_source()
        {
            count += 1;
        }
    }
    count
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
    match fluid_state_at(world, tables, existing_pos) {
        None => true,
        Some(existing) => match existing.kind {
            FluidKind::Water => {
                incoming_dir == Direction::Down && incoming_kind != FluidKind::Water
            }
            FluidKind::Lava => {
                get_height(world, tables, existing_pos, existing) >= 0.44444445f32
                    && incoming_kind == FluidKind::Water
            }
        },
    }
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
    let kind = state.kind;
    let mut lowest: u32 = 1000;
    let mut result: Vec<(Direction, Option<FluidState>)> = Vec::new();
    let mut hole_cache: HashMap<(i32, i32), bool> = HashMap::new();

    for dir in FLUID_HORIZONTAL_ORDER {
        let tpos = dir.apply(pos);
        if !occlusion::can_maybe_pass_through(world, tables, waterlog, pos, tpos, dir, kind) {
            continue;
        }
        let candidate = get_new_liquid(world, tables, tpos, kind);
        if !occlusion::can_hold_specific_fluid(world, waterlog, tpos, kind) {
            continue;
        }
        let distance = if is_hole_cached(world, tables, waterlog, tpos, kind, pos, &mut hole_cache)
        {
            0
        } else {
            get_slope_distance(
                world,
                tables,
                waterlog,
                tpos,
                1,
                dir.opposite(),
                kind,
                pos,
                &mut hole_cache,
            )
        };
        if distance < lowest {
            result.clear();
        }
        if distance <= lowest {
            if can_be_replaced_with(world, tables, tpos, kind, dir) {
                result.push((dir, candidate));
            }
            lowest = distance;
        }
    }
    result
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
    let key = (pos.x - origin.x, pos.z - origin.z);
    if let Some(&v) = cache.get(&key) {
        return v;
    }
    let v = occlusion::is_hole(world, tables, waterlog, pos, kind);
    cache.insert(key, v);
    v
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
    let mut lowest = 1000u32;
    for dir in FLUID_HORIZONTAL_ORDER {
        if dir == from {
            continue;
        }
        let tpos = dir.apply(pos);
        if !occlusion::can_pass_through(world, tables, waterlog, pos, tpos, dir, kind) {
            continue;
        }
        if is_hole_cached(world, tables, waterlog, tpos, kind, origin, cache) {
            return pass;
        }
        if pass < tables.slope_find_distance(kind) {
            let v = get_slope_distance(
                world,
                tables,
                waterlog,
                tpos,
                pass + 1,
                dir.opposite(),
                kind,
                origin,
                cache,
            );
            lowest = lowest.min(v);
        }
    }
    lowest
}

pub fn get_own_height(state: FluidState) -> f32 {
    state.own_height()
}

/// Context §A: `1.0` iff the cell directly above holds the same fluid kind (any variant), else
/// `get_own_height(state)`.
pub fn get_height(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    state: FluidState,
) -> f32 {
    let above = Direction::Up.apply(pos);
    if let Some(above_state) = fluid_state_at(world, tables, above)
        && above_state.kind == state.kind
    {
        return 1.0f32;
    }
    get_own_height(state)
}

/// Context §H — the complete entity-facing flow-field query.
pub fn get_flow(
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    pos: BlockPos,
    state: FluidState,
) -> Vec3 {
    let kind = state.kind;
    let mut flow_x: f64 = 0.0;
    let mut flow_z: f64 = 0.0;

    for dir in FLUID_HORIZONTAL_ORDER {
        let npos = dir.apply(pos);
        let nfluid = fluid_state_at(world, tables, npos);
        let affects_flow = match nfluid {
            None => true,
            Some(s) => s.kind == kind,
        };
        if !affects_flow {
            continue;
        }

        let neighbor_height_zero = nfluid.map(get_own_height).unwrap_or(0.0) == 0.0;
        let mut distance: f32 = 0.0;

        if neighbor_height_zero {
            if !occlusion::blocks_motion(world, tables, npos) {
                let below_npos = Direction::Down.apply(npos);
                let below_fluid = fluid_state_at(world, tables, below_npos);
                let below_ok = match below_fluid {
                    None => true,
                    Some(s) => s.kind == kind,
                };
                if below_ok {
                    let bh = below_fluid.map(get_own_height).unwrap_or(0.0);
                    if bh > 0.0 {
                        distance = get_own_height(state) - (bh - 0.8888889f32);
                    }
                }
            }
        } else {
            let neighbor_height = nfluid.map(get_own_height).unwrap_or(0.0);
            distance = get_own_height(state) - neighbor_height;
        }

        if distance != 0.0 {
            let (dx, _, dz) = dir.offset();
            flow_x += (dx as f32 * distance) as f64;
            flow_z += (dz as f32 * distance) as f64;
        }
    }

    let mut flow = Vec3::new(flow_x, 0.0, flow_z);

    if state.falling() {
        for dir in FLUID_HORIZONTAL_ORDER {
            let side_pos = dir.apply(pos);
            let above_side_pos = Direction::Up.apply(side_pos);
            if occlusion::is_solid_face(world, tables, kind, side_pos, dir)
                || occlusion::is_solid_face(world, tables, kind, above_side_pos, dir)
            {
                flow = normalize_or_zero(flow);
                flow = Vec3::new(flow.x, flow.y - 6.0, flow.z);
                break;
            }
        }
    }

    normalize_or_zero(flow)
}

/// Private helper (Context §H): length `< 1.0e-5f32` (widened to `f64`) ⇒ exactly `Vec3::ZERO`,
/// else each component divided by length. `rc-physics` ships no `normalize` method of its own
/// (Constraints (b)) — this stays local to this crate.
fn normalize_or_zero(v: Vec3) -> Vec3 {
    let length = v.length_squared().sqrt();
    if length < 1.0e-5f32 as f64 {
        Vec3::ZERO
    } else {
        Vec3::new(v.x / length, v.y / length, v.z / length)
    }
}
