//! M3 field-report test-authoring (MECH-D62 re-supersession): drives `mining::is_within_
//! block_interaction_range` -- vanilla's own box-distance-from-eye reach predicate, replacing
//! the retired per-player voxel raycast (Context, AUTHORITATIVE RESEARCH VERDICT) -- and
//! `movement::eye_position`'s pose-aware height selection directly, no live world/socket
//! involved. Every case below fails today against `is_within_block_interaction_range`'s own
//! `todo!()` stub (and, for the sneaking cases, `eye_position`'s crouching-branch stub); the
//! matching implementation changeset fills both in.

use rc_core::BlockPos;
use rc_physics::Vec3;
use rusty_clanker_server::play::{
    BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER, BLOCK_INTERACTION_RANGE_CREATIVE,
    BLOCK_INTERACTION_RANGE_SURVIVAL, eye_position, is_within_block_interaction_range,
};

#[test]
fn survival_threshold_sits_at_the_nearest_point_of_the_box_plus_the_buffer() {
    let target = BlockPos::new(0, 0, 0); // box [0,1] x [0,1] x [0,1]
    let threshold =
        BLOCK_INTERACTION_RANGE_SURVIVAL + BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER;
    assert_eq!(threshold, 5.5);

    // Axis-aligned along +x from the box's own far face (x == 1.0) keeps "nearest point"
    // unambiguous -- y/z sit inside the box's own [0,1] span on both axes, contributing 0.
    let just_inside = Vec3::new(1.0 + threshold - 0.01, 0.5, 0.5);
    let just_outside = Vec3::new(1.0 + threshold + 0.01, 0.5, 0.5);
    assert!(is_within_block_interaction_range(
        just_inside,
        target,
        BLOCK_INTERACTION_RANGE_SURVIVAL
    ));
    assert!(!is_within_block_interaction_range(
        just_outside,
        target,
        BLOCK_INTERACTION_RANGE_SURVIVAL
    ));
}

#[test]
fn creative_threshold_sits_at_the_nearest_point_of_the_box_plus_the_buffer() {
    let target = BlockPos::new(0, 0, 0);
    let threshold =
        BLOCK_INTERACTION_RANGE_CREATIVE + BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER;
    assert_eq!(threshold, 6.0);

    let just_inside = Vec3::new(1.0 + threshold - 0.01, 0.5, 0.5);
    let just_outside = Vec3::new(1.0 + threshold + 0.01, 0.5, 0.5);
    assert!(is_within_block_interaction_range(
        just_inside,
        target,
        BLOCK_INTERACTION_RANGE_CREATIVE
    ));
    assert!(!is_within_block_interaction_range(
        just_outside,
        target,
        BLOCK_INTERACTION_RANGE_CREATIVE
    ));
}

/// The predicate's own defining property (Context, AUTHORITATIVE RESEARCH VERDICT): the
/// boundary is the NEAREST POINT of the block's own full unit box, never its centre. `eye`
/// below sits off the block's own centred axis on y/z (aligned with the box's own min corner
/// on both), so a naive centre-distance model and this nearest-point model disagree about
/// whether this exact point is in survival range -- locking in that this predicate is the
/// latter, not the former.
#[test]
fn the_boundary_is_the_nearest_point_of_the_box_not_its_centre() {
    let target = BlockPos::new(0, 0, 0); // box [0,1]^3, centre (0.5, 0.5, 0.5)
    let eye = Vec3::new(6.4, 0.0, 0.0);

    // Nearest point: (1.0, 0.0, 0.0) -- distance 5.4, inside the 5.5 survival threshold.
    assert!(is_within_block_interaction_range(
        eye,
        target,
        BLOCK_INTERACTION_RANGE_SURVIVAL
    ));

    // A centre-distance model would compute sqrt(5.9^2 + 0.5^2 + 0.5^2) ~= 5.94, OUTSIDE the
    // 5.5 threshold -- this locks in that this predicate does not do that.
    let centre_distance =
        ((eye.x - 0.5).powi(2) + (eye.y - 0.5).powi(2) + (eye.z - 0.5).powi(2)).sqrt();
    assert!(
        centre_distance
            > BLOCK_INTERACTION_RANGE_SURVIVAL + BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER
    );
}

#[test]
fn a_genuinely_distant_block_is_rejected_at_both_gamemode_thresholds() {
    let target = BlockPos::new(0, 0, 0);
    let far_eye = Vec3::new(50.0, 0.5, 0.5);
    assert!(!is_within_block_interaction_range(
        far_eye,
        target,
        BLOCK_INTERACTION_RANGE_SURVIVAL
    ));
    assert!(!is_within_block_interaction_range(
        far_eye,
        target,
        BLOCK_INTERACTION_RANGE_CREATIVE
    ));
}

/// M3 field-report regression (Symptom 1): the exact-cell-hit voxel raycast this predicate
/// replaces resolved a claimed edge/grazing target to a neighboring cell and rejected it even
/// though the block was well within Euclidean range -- this predicate has no such
/// cell-resolution step at all, so a point diagonally offset from a block's own axis (an
/// "edge aim," geometrically the same class of case) is accepted purely on distance.
#[test]
fn a_diagonally_offset_edge_aim_is_accepted_purely_on_distance() {
    let target = BlockPos::new(10, 0, 10); // box [10,11] x [0,1] x [10,11]
    // Offset toward one corner of the block rather than centred on either horizontal axis --
    // exactly the kind of grazing aim the old exact-cell DDA raycast could resolve to a
    // different neighboring cell and wrongly reject.
    let eye = Vec3::new(10.0, 0.5, 10.0 - 4.0);
    assert!(is_within_block_interaction_range(
        eye,
        target,
        BLOCK_INTERACTION_RANGE_SURVIVAL
    ));
}

/// M3 field-report regression (Symptom 2): a block reachable only from the lower, crouching
/// eye height is accepted while sneaking and rejected while standing, at the same feet
/// position. `feet` is directly below `target`, `PLAYER_EYE_HEIGHT`/`_CROUCHING`'s own 0.35
/// block difference is exactly what straddles the survival threshold here.
#[test]
fn sneaking_brings_an_otherwise_out_of_reach_block_into_reach() {
    let feet = Vec3::new(0.5, 0.0, 0.5);
    let target = BlockPos::new(0, -5, 0); // box y = [-5, -4]

    let standing_eye = eye_position(feet, false);
    let crouching_eye = eye_position(feet, true);

    assert!(
        !is_within_block_interaction_range(standing_eye, target, BLOCK_INTERACTION_RANGE_SURVIVAL),
        "standing eye (1.62) must still be out of survival reach of this target"
    );
    assert!(
        is_within_block_interaction_range(crouching_eye, target, BLOCK_INTERACTION_RANGE_SURVIVAL),
        "crouching eye (1.27), 0.35 blocks lower, must bring this same target into reach"
    );
}
