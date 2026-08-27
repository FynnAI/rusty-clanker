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
    // Defense in depth (M3 field-report Defect A), independent of every caller's own
    // upstream validation (block/place reach validation no longer calls this function at all
    // -- a later M3 field-report fix, MECH-D62 re-supersession, replaced it with a pure
    // box-distance predicate; `crates/testing/paritybot`'s own bot-aim self-tests are this
    // function's remaining real caller today, feeding it a `look_vector`-derived direction
    // that `play::movement::evaluate_movement` already rejects non-finite before it ever
    // reaches `motion.yaw`/`motion.pitch` -- but this function must stay safe on its own
    // terms for any caller, present or future): a non-finite direction makes
    // every `signum()`/`axis_t_delta`/`initial_t_max` value below either `NaN` or a
    // never-advancing `0` step, and a zero-length direction (every component `0.0`) makes
    // every `t_max_*` axis permanently `INFINITY` (`axis_t_delta`'s own "never advances"
    // case, on all three axes at once) -- either way `cell` would never change and every
    // `t_max_* > max_distance` exit check below would never fire, spinning forever. Bailing
    // out here is the only correct answer for a direction with no well-defined ray at all.
    if !direction.is_finite() || direction.length_squared() == 0.0 {
        return None;
    }

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

    // Defense in depth, independent of the `direction` guard above: a real DDA sweep with a
    // well-defined direction crosses at most one grid line per axis for every unit of
    // distance travelled, so the number of cells visited before `t` exceeds `max_distance`
    // is bounded by roughly `3 * max_distance` (one term per axis) -- `+ 4` covers the
    // starting cell and the loop's own off-by-one at each exit check. This cap never trusts
    // `max_distance` alone to end the loop, since a `NaN` `max_distance` would make every
    // `t_max_* > max_distance` comparison below false forever (the exact failure mode the
    // `direction` guard above exists to prevent for `direction` specifically -- this is the
    // same defense for `max_distance`). `f64::clamp` itself propagates a `NaN` input straight
    // through (its own doc comment: "returns NaN if the number is NaN," unlike the `.max(0.0)
    // .min(..)` chain this originally used) rather than folding it to `0` the way `direction`'s
    // own finite guard above does -- but that is still safe here, one step later: `as u64`
    // saturates a NaN operand to `0` (Rust's own documented, non-panicking float -> int cast
    // rule), so `max_steps` below ends up `4` either way, `NaN` or `0.0`. A merely huge or
    // `INFINITY` `max_distance` is clamped to a value already far past any real caller's own
    // reach (`play::mining::BLOCK_INTERACTION_RANGE_CREATIVE == 5.0`), so `max_steps` itself
    // is always a small, finite, real integer no matter what `max_distance` turns out to be.
    const MAX_TRUSTED_DISTANCE: f64 = 1.0e4;
    let safe_max_distance = max_distance.clamp(0.0, MAX_TRUSTED_DISTANCE);
    let max_steps = (3.0 * safe_max_distance).ceil() as u64 + 4;

    for _ in 0..max_steps {
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

        // M3 field-report fix (Defect B): the exit test must fire on the ENTRY distance into
        // the next cell (this axis's own `t_max_*` value BEFORE it advances), never on the
        // FAR/exit boundary of that not-yet-tested cell -- checking post-advance silently
        // discarded every cell whose own near face sat within `max_distance` but whose far
        // face did not, costing up to one full cell of reach per axis (compounding across
        // axes for an oblique ray, since each axis's own check fires independently as it
        // comes due). Checking here, before `cell`/`t_max_*` are touched, guarantees the next
        // cell is only skipped when the ray genuinely cannot reach it within `max_distance`.
        if t_max_x <= t_max_y && t_max_x <= t_max_z {
            if t_max_x > max_distance {
                return None;
            }
            cell.x += step_x;
            t_max_x += t_delta_x;
        } else if t_max_y <= t_max_z {
            if t_max_y > max_distance {
                return None;
            }
            cell.y += step_y;
            t_max_y += t_delta_y;
        } else {
            if t_max_z > max_distance {
                return None;
            }
            cell.z += step_z;
            t_max_z += t_delta_z;
        }
    }
    // The hard iteration cap above is a defensive backstop, never expected to bind for any
    // real `(direction, max_distance)` pair the `direction`-finite/non-zero guard and the
    // ordinary `t_max_* > max_distance` exit checks already let through -- reaching it means
    // `max_distance` itself was non-finite (the one case those ordinary checks cannot catch,
    // per this cap's own doc comment above), so `None` (no hit found within any well-defined
    // distance) is the only honest answer.
    None
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
