//! Y-then-X-then-Z sequential collide-and-slide with step-up and sneak edge-keep (MECH-D38,
//! Context: "Collide-and-slide sweep" / "Sneak edge-keep").

use crate::motion::SNEAK_EDGE_STEP;
use crate::{Aabb, BlockShapeSource, SHAPE_EPSILON, Vec3, aabb::Axis};

/// Sweeps `aabb` along `axis` by `delta`, clipped against every block whose shape overlaps
/// the motion-swept broad-phase box -- the maximum distance actually travelable before the
/// first obstruction (Context: exact algorithm).
pub fn sweep_axis(aabb: Aabb, axis: Axis, delta: f64, shapes: &dyn BlockShapeSource) -> f64 {
    if delta == 0.0 {
        return 0.0;
    }
    let broad = aabb.extended_along(axis, delta);
    let mut result = delta;
    for block_pos in broad.overlapped_block_positions() {
        let props = shapes.properties_at(block_pos);
        for sub_box in props.shape.boxes() {
            let world_box = sub_box.offset_by(block_pos);
            result = clip_distance(aabb, world_box, axis, result);
        }
    }
    result
}

/// Clips `distance` (the still-attempted travel along `axis`) against one fixed obstacle box
/// -- no interaction at all if the two OTHER axes' current extents (before this axis moves)
/// don't overlap, within `SHAPE_EPSILON` (Context: exact algorithm).
fn clip_distance(moving: Aabb, fixed: Aabb, axis: Axis, distance: f64) -> f64 {
    let (a1, a2) = axis.other_two();
    if !moving.overlaps_on(a1, fixed, SHAPE_EPSILON)
        || !moving.overlaps_on(a2, fixed, SHAPE_EPSILON)
    {
        return distance;
    }
    if distance > 0.0 {
        let gap = fixed.min(axis) - moving.max(axis);
        if gap >= -SHAPE_EPSILON {
            distance.min(gap.max(0.0))
        } else {
            distance
        }
    } else if distance < 0.0 {
        let gap = fixed.max(axis) - moving.min(axis);
        if gap <= SHAPE_EPSILON {
            distance.max(gap.min(0.0))
        } else {
            distance
        }
    } else {
        0.0
    }
}

/// Y-then-X-then-Z sequential collide-and-slide with step-up (Context, full algorithm).
/// Returns the resolved delta and whether the entity ends the sweep on solid ground.
pub fn collide_and_slide(
    position: Vec3,
    half_width: f64,
    height: f64,
    requested: Vec3,
    shapes: &dyn BlockShapeSource,
    step_height: f64,
) -> (Vec3, bool) {
    let mut aabb = Aabb::from_position(position, half_width, height);
    let dy = sweep_axis(aabb, Axis::Y, requested.y, shapes);
    aabb = aabb.translated(0.0, dy, 0.0);
    let dx = sweep_axis(aabb, Axis::X, requested.x, shapes);
    aabb = aabb.translated(dx, 0.0, 0.0);
    let dz = sweep_axis(aabb, Axis::Z, requested.z, shapes);
    aabb = aabb.translated(0.0, 0.0, dz);

    let horizontal_blocked =
        (dx.abs() + 1e-9 < requested.x.abs()) || (dz.abs() + 1e-9 < requested.z.abs());
    let on_ground_now = sweep_axis(aabb, Axis::Y, -1e-3, shapes).abs() < 1e-3;
    let on_ground_after_collision = dy < 0.0 && on_ground_now;

    if step_height > 0.0
        && horizontal_blocked
        && (on_ground_after_collision || on_ground_now)
        && let Some((stepped_dx, stepped_dy, stepped_dz)) = try_step_up(
            position,
            half_width,
            height,
            requested,
            shapes,
            step_height,
            dx,
            dz,
        )
    {
        let final_aabb = Aabb::from_position(position, half_width, height)
            .translated(stepped_dx, stepped_dy, stepped_dz);
        let final_on_ground = sweep_axis(final_aabb, Axis::Y, -1e-3, shapes).abs() < 1e-3;
        return (
            Vec3::new(stepped_dx, stepped_dy, stepped_dz),
            final_on_ground,
        );
    }
    (Vec3::new(dx, dy, dz), on_ground_now)
}

/// Step-up (14 §3.2.2): tries every candidate surface height in ascending order, taking the
/// first one that actually improves horizontal travel distance over the plain (unstepped)
/// result -- never merely "the highest reachable surface within `step_height`" (Context).
#[allow(clippy::too_many_arguments)]
fn try_step_up(
    position: Vec3,
    half_width: f64,
    height: f64,
    requested: Vec3,
    shapes: &dyn BlockShapeSource,
    step_height: f64,
    plain_dx: f64,
    plain_dz: f64,
) -> Option<(f64, f64, f64)> {
    let plain_horiz = (plain_dx * plain_dx + plain_dz * plain_dz).sqrt();
    let grounded = Aabb::from_position(position, half_width, height);
    let probe = grounded
        .extended_along(Axis::X, requested.x)
        .extended_along(Axis::Z, requested.z)
        .extended_along(Axis::Y, step_height);

    let mut candidates: Vec<f64> = probe
        .overlapped_block_positions()
        .into_iter()
        .flat_map(|pos| {
            let props = shapes.properties_at(pos);
            props
                .shape
                .boxes()
                .iter()
                .flat_map(|b| {
                    let world = b.offset_by(pos);
                    [world.min.y - grounded.min.y, world.max.y - grounded.min.y]
                })
                .collect::<Vec<_>>()
        })
        .filter(|&h| h > 0.0 && h <= step_height)
        .collect();
    candidates.sort_by(|a, b| a.partial_cmp(b).expect("step-up heights are always finite"));
    candidates.dedup_by(|a, b| (*a - *b).abs() < SHAPE_EPSILON);

    let mut best: Option<(f64, f64, f64, f64)> = None; // (dx, dy, dz, horiz_len)
    for h in candidates {
        let raised = grounded.translated(0.0, h, 0.0);
        let dx = sweep_axis(raised, Axis::X, requested.x, shapes);
        let stepped_x = raised.translated(dx, 0.0, 0.0);
        let dz = sweep_axis(stepped_x, Axis::Z, requested.z, shapes);
        let horiz = (dx * dx + dz * dz).sqrt();
        if horiz > plain_horiz && best.as_ref().is_none_or(|b| horiz > b.3) {
            best = Some((dx, h, dz, horiz));
        }
    }
    best.map(|(dx, h, dz, _)| (dx, h, dz))
}

/// `true` iff the entity's AABB at `position` overlaps any solid (non-empty-shape) block at
/// all -- used by `evaluate_movement`'s mismatch-rejection gate (Context, 14 §3.15 step 5).
pub fn overlaps_any_solid(
    position: Vec3,
    half_width: f64,
    height: f64,
    shapes: &dyn BlockShapeSource,
) -> bool {
    let aabb = Aabb::from_position(position, half_width, height);
    aabb.overlapped_block_positions().into_iter().any(|pos| {
        let props = shapes.properties_at(pos);
        props
            .shape
            .boxes()
            .iter()
            .any(|b| box_overlaps(aabb, b.offset_by(pos)))
    })
}

/// `true` iff any block overlapping the entity's AABB at `new_position` did **not** already
/// overlap it at `old_position` (14 §3.15 step 5's "new collision not already present at the
/// old position" check).
pub fn has_new_collision(
    old_position: Vec3,
    new_position: Vec3,
    half_width: f64,
    height: f64,
    shapes: &dyn BlockShapeSource,
) -> bool {
    let old_aabb = Aabb::from_position(old_position, half_width, height);
    let new_aabb = Aabb::from_position(new_position, half_width, height);
    new_aabb
        .overlapped_block_positions()
        .into_iter()
        .any(|pos| {
            let props = shapes.properties_at(pos);
            props.shape.boxes().iter().any(|b| {
                let world = b.offset_by(pos);
                box_overlaps(new_aabb, world) && !box_overlaps(old_aabb, world)
            })
        })
}

fn box_overlaps(a: Aabb, b: Aabb) -> bool {
    a.overlaps_on(Axis::X, b, SHAPE_EPSILON)
        && a.overlaps_on(Axis::Y, b, SHAPE_EPSILON)
        && a.overlaps_on(Axis::Z, b, SHAPE_EPSILON)
}

/// Context: "Sneak edge-keep" -- shrinks `dx` toward zero first (holding `dz` at its
/// original value) until either `dx == 0.0` or the probe at `(dx, dz_original)` reports
/// support; then shrinks `dz` toward zero (holding the now-final `dx`) the same way.
pub fn sneak_edge_guard(
    position: Vec3,
    half_width: f64,
    height: f64,
    dx: f64,
    dz: f64,
    shapes: &dyn BlockShapeSource,
    max_up_step: f64,
) -> (f64, f64) {
    fn shrink(v: f64, step: f64) -> f64 {
        if v.abs() <= step {
            0.0
        } else {
            v - step * v.signum()
        }
    }
    let mut dx = dx;
    while dx != 0.0
        && !would_still_be_supported(position, half_width, height, dx, dz, shapes, max_up_step)
    {
        dx = shrink(dx, SNEAK_EDGE_STEP);
    }
    let mut dz = dz;
    while dz != 0.0
        && !would_still_be_supported(position, half_width, height, dx, dz, shapes, max_up_step)
    {
        dz = shrink(dz, SNEAK_EDGE_STEP);
    }
    (dx, dz)
}

/// Builds the AABB at `position` translated by `(dx, 0, dz)`; sweeps it downward by
/// `max_up_step` via `sweep_axis`; returns `true` iff the returned distance is strictly less
/// than `max_up_step` (something stops the fall within that range -- solid ground is
/// present).
pub fn would_still_be_supported(
    position: Vec3,
    half_width: f64,
    height: f64,
    dx: f64,
    dz: f64,
    shapes: &dyn BlockShapeSource,
    max_up_step: f64,
) -> bool {
    let aabb = Aabb::from_position(position, half_width, height).translated(dx, 0.0, dz);
    sweep_axis(aabb, Axis::Y, -max_up_step, shapes).abs() < max_up_step
}
