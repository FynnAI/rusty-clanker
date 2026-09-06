//! Reach and angle validation for entity targets (M4-B05 Context, "Reach and angle
//! validation for entity targets", MECH-D62 extended from block targets). A per-player-
//! position, per-target-entity raycast against the target's own bounding box, reusing
//! M3-B03's own shared slab-method ray/AABB-intersection style (`rc_physics::raycast`'s own
//! module doc comment) applied here to an entity's box instead of a block's `VoxelShape`.
//! Pure geometry only — world/block occlusion is a separate concern the caller
//! (`rusty-clanker-server::play::combat`) layers on top via `rc_physics::raycast::cast_ray`
//! against the same origin/direction, since this function has no world access at all
//! (Deliverables' own literal signature carries none).

use rc_physics::Vec3;

use crate::entity::EntityKind;

/// A fixed constant (Context: "`entity_interaction_range` attribute does not exist in this
/// blueprint's own `AttributeKind` table... restated here as a fixed constant"), moderate
/// confidence, matching this blueprint's own pinning discipline for every other numeric
/// constant restated in Context.
pub const ENTITY_INTERACTION_RANGE: f64 = 3.0;

/// Per-kind hitbox width/height, moderate confidence (well-established, unverified against a
/// real entity-dimensions data dump — reconciliation item). `Item` is never a combat target
/// in practice — included only for match-exhaustiveness.
pub fn entity_dimensions(kind: EntityKind) -> (f64, f64) {
    match kind {
        EntityKind::Zombie | EntityKind::Villager => (0.6, 1.95),
        EntityKind::Cow => (0.9, 1.4),
        EntityKind::Item => (0.25, 0.25),
    }
}

/// `origin`/`direction` per M3-B03's own shared look-vector construction, reused unmodified
/// by the caller. Builds the target's AABB from `entity_dimensions` centered on `target_pos`,
/// tests via the standard slab method (min/max `t` per axis), accepts only if the ray enters
/// the box within `[0, ENTITY_INTERACTION_RANGE]` **and** the straight-line Euclidean
/// distance from `origin` to the target's own center is also `<= ENTITY_INTERACTION_RANGE`
/// (belt-and-suspenders — Context's own stated rationale: the slab test alone would accept a
/// graze along a box edge at extreme range if `direction` were not exactly normalized, and
/// this function does not re-normalize its input).
pub fn raycast_entity_reach(
    origin: Vec3,
    direction: Vec3,
    target_pos: [f64; 3],
    target_kind: EntityKind,
) -> bool {
    // `target_pos` is the entity's own natural position — feet, matching `BaseEntity.pos`/
    // `PlayerMarker.position`'s already-established convention (horizontally centered,
    // vertically the AABB's own bottom, exactly like every other AABB this project builds,
    // e.g. `Aabb::from_position`). Context's own "straight-line Euclidean distance... to the
    // target's own center" is the vertical MIDPOINT specifically — a real, cited bug fix
    // beyond this blueprint's own literal Deliverables text: the earlier implementation
    // compared distance-to-feet, which rejects a perfectly in-range, dead-center-aimed target
    // whenever its own height alone pushes the feet-to-eye distance just past
    // `ENTITY_INTERACTION_RANGE` (a zombie standing at the same Y as the attacker's own feet
    // is `~1.95/2` blocks closer to eye height at its center than at its feet).
    let (half_width, height) = entity_dimensions(target_kind);
    let center = Vec3::new(target_pos[0], target_pos[1] + height / 2.0, target_pos[2]);
    let straight_line_distance = (center - origin).length_squared().sqrt();
    if straight_line_distance > ENTITY_INTERACTION_RANGE {
        return false;
    }

    let min = Vec3::new(
        target_pos[0] - half_width,
        target_pos[1],
        target_pos[2] - half_width,
    );
    let max = Vec3::new(
        target_pos[0] + half_width,
        target_pos[1] + height,
        target_pos[2] + half_width,
    );

    let Some((t_enter, t_exit)) = slab_intersect(origin, direction, min, max) else {
        return false;
    };
    t_enter <= t_exit && (0.0..=ENTITY_INTERACTION_RANGE).contains(&t_enter)
}

/// The standard slab method (the identical generic algorithm `rc_physics::raycast`'s own
/// private `ray_aabb_intersect` implements — restated here, independently, since that helper
/// is private to its own crate/module and this function needs no other part of that file).
/// `None` if the ray, extended infinitely in both directions, never enters the box at all.
fn slab_intersect(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<(f64, f64)> {
    let (enter_x, exit_x) = slab_axis(origin.x, direction.x, min.x, max.x)?;
    let (enter_y, exit_y) = slab_axis(origin.y, direction.y, min.y, max.y)?;
    let (enter_z, exit_z) = slab_axis(origin.z, direction.z, min.z, max.z)?;
    let t_enter = enter_x.max(enter_y).max(enter_z);
    let t_exit = exit_x.min(exit_y).min(exit_z);
    Some((t_enter, t_exit))
}

fn slab_axis(origin_i: f64, direction_i: f64, lo: f64, hi: f64) -> Option<(f64, f64)> {
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
