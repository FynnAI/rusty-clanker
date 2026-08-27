//! Voxel-grid ray traversal (M3-B03, MECH-D62's real per-player reach validation --
//! Context: "Reach validation -- superseded to a real per-player-position voxel raycast").
//! Standard 3D DDA (Amanatidis-Woo style): visits every integer block cell a ray passes
//! through, in strictly ascending-distance order, testing each visited cell's own
//! `VoxelShape` sub-boxes for an exact ray/AABB intersection via the standard slab method.
//! Not a byte-exact reproduction of vanilla's own `BlockGetter.clip`/`ClipContext` -- this
//! crate's own reasonable general-shape implementation, sufficient for a boolean reach
//! accept/reject (Open Questions, restated from the blueprint this file implements).

use crate::aabb::Aabb;
use crate::shapes::BlockShapeSource;
use crate::vec3::Vec3;
use rc_core::BlockPos;

/// One `cast_ray` hit: the block it landed in, the exact world-space point struck, and the
/// ray-parameter distance from `origin` (in units of `direction`'s own length -- the caller
/// is responsible for passing a pre-normalized `direction`, per this function's own doc
/// comment below).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RayHit {
    pub block_pos: BlockPos,
    pub hit_point: Vec3,
    pub distance: f64,
}

/// Casts a ray from `origin` along `direction` (assumed pre-normalized by the caller --
/// undefined distance units otherwise, this function never normalizes) up to `max_distance`,
/// returning the *closest* block whose `shapes.properties_at` shape is non-empty and whose
/// world-space sub-box the ray actually intersects within `[0, max_distance]`, or `None` if
/// the ray exits `max_distance` without touching one. Cells are visited in strictly
/// ascending-distance DDA order, so the first cell with any hit is the closest hit overall.
pub fn cast_ray(
    origin: Vec3,
    direction: Vec3,
    max_distance: f64,
    shapes: &dyn BlockShapeSource,
) -> Option<RayHit> {
    let mut cell = BlockPos::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );

    let step_x = direction.x.signum() as i32;
    let step_y = direction.y.signum() as i32;
    let step_z = direction.z.signum() as i32;

    let t_delta_x = axis_t_delta(direction.x);
    let t_delta_y = axis_t_delta(direction.y);
    let t_delta_z = axis_t_delta(direction.z);

    let mut t_max_x = initial_t_max(origin.x, direction.x, cell.x);
    let mut t_max_y = initial_t_max(origin.y, direction.y, cell.y);
    let mut t_max_z = initial_t_max(origin.z, direction.z, cell.z);

    loop {
        let properties = shapes.properties_at(cell);
        if !properties.shape.is_empty()
            && let Some(hit) = closest_sub_box_hit(
                cell,
                origin,
                direction,
                max_distance,
                properties.shape.boxes(),
            )
        {
            return Some(hit);
        }

        if t_max_x <= t_max_y && t_max_x <= t_max_z {
            cell.x += step_x;
            t_max_x += t_delta_x;
            if t_max_x > max_distance {
                return None;
            }
        } else if t_max_y <= t_max_z {
            cell.y += step_y;
            t_max_y += t_delta_y;
            if t_max_y > max_distance {
                return None;
            }
        } else {
            cell.z += step_z;
            t_max_z += t_delta_z;
            if t_max_z > max_distance {
                return None;
            }
        }
    }
}

/// `t_delta_i`: distance along the ray to cross one full cell on this axis. `INFINITY` for
/// an axis the ray never advances on (`direction_i == 0.0`) -- that axis's own `t_max_i`
/// then also stays `INFINITY` forever (`initial_t_max`), so it is never chosen as the
/// smallest-`t_max` axis to step (guaranteed as long as at least one other axis is nonzero,
/// always true for a real, non-degenerate direction vector).
fn axis_t_delta(direction_i: f64) -> f64 {
    if direction_i == 0.0 {
        f64::INFINITY
    } else {
        (1.0 / direction_i).abs()
    }
}

/// The ray-parameter distance from `origin` to the first grid-line crossing on this axis,
/// starting from `cell_i` (`origin_i`'s own containing cell).
fn initial_t_max(origin_i: f64, direction_i: f64, cell_i: i32) -> f64 {
    if direction_i > 0.0 {
        (cell_i as f64 + 1.0 - origin_i) / direction_i
    } else if direction_i < 0.0 {
        (cell_i as f64 - origin_i) / direction_i
    } else {
        f64::INFINITY
    }
}

/// Tests every sub-box of `cell`'s own shape for a ray/AABB intersection, keeping the one
/// with the smallest `t_enter` (Context: "among all sub-boxes hit at this cell, keep the
/// smallest `t_enter`"). Only a hit within `[0, max_distance]` counts.
fn closest_sub_box_hit(
    cell: BlockPos,
    origin: Vec3,
    direction: Vec3,
    max_distance: f64,
    sub_boxes: &[Aabb],
) -> Option<RayHit> {
    let mut best: Option<RayHit> = None;
    for sub_box in sub_boxes {
        let world_box = sub_box.offset_by(cell);
        let Some((t_enter, t_exit)) = ray_aabb_intersect(origin, direction, world_box) else {
            continue;
        };
        if t_enter > t_exit || t_enter < 0.0 || t_enter > max_distance {
            continue;
        }
        if best.is_none_or(|b| t_enter < b.distance) {
            best = Some(RayHit {
                block_pos: cell,
                hit_point: origin + direction * t_enter,
                distance: t_enter,
            });
        }
    }
    best
}

/// The standard slab method: `t_enter`/`t_exit` such that `origin + direction * t` lies
/// inside `box_` for every `t` in `[t_enter, t_exit]`. `None` if the ray (extended
/// infinitely in both directions) never enters the box at all -- distinct from the caller's
/// own further "`t_enter <= t_exit` and within range" acceptance check, which handles the
/// case where the infinite line crosses the box only in [t_enter, t_exit] but this ray's own
/// bounded segment does not.
fn ray_aabb_intersect(origin: Vec3, direction: Vec3, box_: Aabb) -> Option<(f64, f64)> {
    let (enter_x, exit_x) = slab(origin.x, direction.x, box_.min.x, box_.max.x)?;
    let (enter_y, exit_y) = slab(origin.y, direction.y, box_.min.y, box_.max.y)?;
    let (enter_z, exit_z) = slab(origin.z, direction.z, box_.min.z, box_.max.z)?;
    let t_enter = enter_x.max(enter_y).max(enter_z);
    let t_exit = exit_x.min(exit_y).min(exit_z);
    Some((t_enter, t_exit))
}

/// One axis' own slab test: `None` iff the ray is parallel to this slab (`d == 0.0`) and
/// `origin` sits outside `[lo, hi]` on this axis (never enters, regardless of the other two
/// axes). Otherwise returns `(near, far)` sorted ascending -- a parallel ray whose origin
/// *is* within `[lo, hi]` imposes no bound on this axis at all (`(-INFINITY, INFINITY)`),
/// deferring entirely to the other two axes.
fn slab(origin_i: f64, direction_i: f64, lo: f64, hi: f64) -> Option<(f64, f64)> {
    if direction_i == 0.0 {
        if origin_i < lo || origin_i > hi {
            None
        } else {
            Some((f64::NEG_INFINITY, f64::INFINITY))
        }
    } else {
        let t1 = (lo - origin_i) / direction_i;
        let t2 = (hi - origin_i) / direction_i;
        if t1 <= t2 {
            Some((t1, t2))
        } else {
            Some((t2, t1))
        }
    }
}
