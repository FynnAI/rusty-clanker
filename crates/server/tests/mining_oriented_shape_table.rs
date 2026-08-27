//! M3 field-report regression (Defect B): drives `mining::resolve_orientation` (the exact
//! function `mining::apply_placement` itself calls) through every real orientation a player can
//! place a repeater/comparator/wall-torch/chest/hopper in, resolves each one's real raw
//! block-state id via `mining::tier1_oriented_state_table()` (again, the exact table
//! `apply_placement` looks up), and asserts `rc_physics::tier1_shape_table()` -- the physics
//! crate's own, hand-kept-in-sync table (`crates/physics/src/shapes.rs`'s own doc comment on
//! `tier1_shape_table()`) -- resolves every one of those ids to its real, non-full shape.
//!
//! Before this fix, `tier1_shape_table()` registered only each of these five blocks' own
//! DEFAULT-orientation id (the one orientation `nearest_horizontal_direction4`'s own `North`
//! offset -- `0` in `mining.rs`'s own `direction_offset` -- happens to produce, or, for the wall
//! torch, the one `clicked_face` this file's own coverage below also happens to exercise
//! first): every OTHER orientation silently fell back to `ShapeTable::lookup`'s own full-cube
//! default, wrongly making, e.g., a South-facing repeater collide as a solid block AND (`rc_
//! mechanics::redstone::signal::is_conductor` reuses this same table) conduct redstone like one.
//! This file is the seam that would have caught that -- and the one that catches
//! `tier1_shape_table()`/`tier1_oriented_state_table()` drifting apart again in the future.

use std::collections::HashSet;

use rc_physics::{BlockPhysicsProperties, Vec3, tier1_shape_table};
use rusty_clanker_server::play::{
    Face, Orientation, PlaceableBlockKind, resolve_orientation, tier1_oriented_state_table,
};

/// `true` iff `props`'s own shape is exactly the single-box full unit cube -- the wrong answer
/// for every orientation this file checks.
fn is_full_cube(props: &BlockPhysicsProperties) -> bool {
    let boxes = props.shape.boxes();
    boxes.len() == 1
        && boxes[0].min == Vec3::new(0.0, 0.0, 0.0)
        && boxes[0].max == Vec3::new(1.0, 1.0, 1.0)
}

/// Resolves `kind`'s real orientation for this real `(clicked_face, yaw, pitch)` placement
/// input via `resolve_orientation` (unwrapped -- every input this file passes is a legal one,
/// per `resolve_orientation`'s own per-kind rules), then the real raw state id via
/// `tier1_oriented_state_table()` -- the identical two-step lookup `mining::apply_placement`
/// itself performs.
fn placed_shape(
    kind: PlaceableBlockKind,
    clicked_face: Face,
    yaw_degrees: f32,
) -> (Orientation, BlockPhysicsProperties) {
    let selection = resolve_orientation(kind, clicked_face, yaw_degrees, 0.0)
        .expect("every (kind, face, yaw) pair this test passes is a legal placement");
    let raw_id = tier1_oriented_state_table().lookup(selection.kind, selection.orientation);
    (selection.orientation, tier1_shape_table().lookup(raw_id))
}

/// Repeater/comparator/chest all resolve orientation from yaw alone (`resolve_orientation`'s
/// own shared match arm) -- `clicked_face` is irrelevant to them, so `Face::North` here is
/// arbitrary. Sweeping a full rotation's worth of yaw values must produce all four `Horizontal`
/// orientations (proving real coverage, not four repeats of the one orientation this table
/// happened to already register before this fix) and every one of them must be non-full.
fn assert_all_four_horizontal_orientations_are_non_full(kind: PlaceableBlockKind, label: &str) {
    let mut seen = HashSet::new();
    for yaw in [0.0_f32, 90.0, 180.0, 270.0] {
        let (orientation, props) = placed_shape(kind, Face::North, yaw);
        assert!(
            matches!(orientation, Orientation::Horizontal(_)),
            "{label} at yaw {yaw} must resolve to a Horizontal orientation, got {orientation:?}"
        );
        assert!(
            !is_full_cube(&props),
            "{label} at yaw {yaw} (orientation {orientation:?}) resolved to a full cube -- \
             Defect B: only this table's own default orientation was ever registered"
        );
        seen.insert(orientation);
    }
    assert_eq!(
        seen.len(),
        4,
        "{label}: the four yaw values above must produce four DISTINCT orientations, not \
         fewer -- got {seen:?}"
    );
}

#[test]
fn repeater_is_non_full_in_every_horizontal_orientation() {
    assert_all_four_horizontal_orientations_are_non_full(PlaceableBlockKind::Repeater, "repeater");
}

#[test]
fn comparator_is_non_full_in_every_horizontal_orientation() {
    assert_all_four_horizontal_orientations_are_non_full(
        PlaceableBlockKind::Comparator,
        "comparator",
    );
}

#[test]
fn chest_is_non_full_in_every_horizontal_orientation() {
    assert_all_four_horizontal_orientations_are_non_full(PlaceableBlockKind::Chest, "chest");
}

#[test]
fn wall_redstone_torch_is_non_full_in_every_horizontal_orientation() {
    // The torch's own orientation comes from `clicked_face`, not yaw (`resolve_orientation`'s
    // own `RedstoneTorch` match arm) -- `Face::Up`/`Face::Down` are the floor placement/
    // rejected cases respectively, already covered elsewhere; this sweeps the four horizontal
    // (wall-mounted) faces only.
    let mut seen = HashSet::new();
    for face in [Face::North, Face::South, Face::East, Face::West] {
        let (orientation, props) = placed_shape(PlaceableBlockKind::RedstoneTorch, face, 0.0);
        assert!(
            matches!(orientation, Orientation::Horizontal(_)),
            "wall torch on face {face:?} must resolve to a Horizontal orientation, got \
             {orientation:?}"
        );
        assert!(
            !is_full_cube(&props),
            "wall torch on face {face:?} (orientation {orientation:?}) resolved to a full \
             cube -- Defect B"
        );
        seen.insert(orientation);
    }
    assert_eq!(
        seen.len(),
        4,
        "the four faces above must produce four distinct orientations"
    );
}

#[test]
fn hopper_is_non_full_in_every_orientation_including_facing_down() {
    // `Face::Up` and `Face::Down` both collapse to `Full(Down)` (`resolve_orientation`'s own
    // Hopper rule -- vanilla hoppers can never face up) -- both are exercised here since either
    // one alone would already prove the `Full(Down)` id (`HOPPER.0 + 10`) is non-full, but
    // together they also prove that collapse itself still lands on the one registered id.
    let mut seen = HashSet::new();
    for face in [
        Face::North,
        Face::South,
        Face::East,
        Face::West,
        Face::Up,
        Face::Down,
    ] {
        let (orientation, props) = placed_shape(PlaceableBlockKind::Hopper, face, 0.0);
        assert!(
            !is_full_cube(&props),
            "hopper on face {face:?} (orientation {orientation:?}) resolved to a full cube -- \
             Defect B"
        );
        seen.insert(orientation);
    }
    assert_eq!(
        seen.len(),
        5,
        "six faces must collapse to exactly five distinct orientations (Up and Down both -> \
         Full(Down)), got {seen:?}"
    );
    assert!(seen.contains(&Orientation::Full(rc_mechanics::Direction::Down)));
}
