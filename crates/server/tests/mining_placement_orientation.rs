//! M3-B03 acceptance test: `resolve_orientation`'s per-block-type table -- pure, no sockets.
//! See `blueprints/M3/M3-B03-breaking-placing.md`, Acceptance tests,
//! "`crates/server/tests/mining_placement_orientation.rs`".
//!
//! Test 1's own worked example (`repeater_faces_away_from_player`) is this file's own
//! ground truth for `nearest_horizontal_direction4`'s yaw convention; test 2
//! (`piston_faces_up_when_player_looks_steeply_down`) is the ground truth for `nearest_
//! direction6`'s pitch-sign mapping -- both hand-derived directly from `mining.rs`'s own
//! `look_vector` formula (that module's own top-of-file doc comment has the full
//! reasoning for why the sign this file asserts differs from the blueprint's own inverted
//! restatement of its own formula).

use rc_mechanics::Direction;
use rusty_clanker_server::play::{
    Face, Orientation, PlaceableBlockKind, RejectReason, resolve_orientation,
};

#[test]
fn repeater_faces_away_from_player() {
    // yaw = 0.0 -> looking South (this project's own `look_vector` convention); a repeater
    // faces *away* from the player, i.e. North.
    let selection = resolve_orientation(PlaceableBlockKind::Repeater, Face::Up, 0.0, 0.0).unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Horizontal(Direction::North)
    );
    assert!(!selection.is_wall_variant);
}

#[test]
fn piston_faces_up_when_player_looks_steeply_down() {
    let selection = resolve_orientation(PlaceableBlockKind::Piston, Face::Up, 0.0, 80.0).unwrap();
    assert_eq!(selection.orientation, Orientation::Full(Direction::Up));
}

#[test]
fn torch_on_top_face_is_standing() {
    let selection =
        resolve_orientation(PlaceableBlockKind::RedstoneTorch, Face::Up, 0.0, 0.0).unwrap();
    assert_eq!(selection.kind, PlaceableBlockKind::RedstoneTorch);
    assert_eq!(selection.orientation, Orientation::None);
    assert!(!selection.is_wall_variant);
}

#[test]
fn torch_on_side_face_is_wall_variant_facing_that_side() {
    let selection =
        resolve_orientation(PlaceableBlockKind::RedstoneTorch, Face::North, 0.0, 0.0).unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Horizontal(Direction::North)
    );
    assert!(selection.is_wall_variant);
}

#[test]
fn torch_on_bottom_face_is_rejected() {
    let result = resolve_orientation(PlaceableBlockKind::RedstoneTorch, Face::Down, 0.0, 0.0);
    assert_eq!(result, Err(RejectReason::InvalidTorchFace));
}

#[test]
fn hopper_faces_opposite_the_clicked_side_face() {
    let selection = resolve_orientation(PlaceableBlockKind::Hopper, Face::North, 0.0, 0.0).unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Horizontal(Direction::South)
    );
}

#[test]
fn hopper_clicked_on_top_defaults_to_facing_down_never_up() {
    let selection = resolve_orientation(PlaceableBlockKind::Hopper, Face::Up, 0.0, 0.0).unwrap();
    assert_eq!(selection.orientation, Orientation::Full(Direction::Down));

    // The clamp itself: naive opposite of `Down` would be `Up`, clamped back to `Down`.
    let selection = resolve_orientation(PlaceableBlockKind::Hopper, Face::Down, 0.0, 0.0).unwrap();
    assert_eq!(selection.orientation, Orientation::Full(Direction::Down));
}

#[test]
fn chest_and_furnace_share_the_same_horizontal_away_from_player_rule() {
    let chest = resolve_orientation(PlaceableBlockKind::Chest, Face::Up, 90.0, 0.0).unwrap();
    let furnace = resolve_orientation(PlaceableBlockKind::Furnace, Face::Up, 90.0, 0.0).unwrap();
    assert_eq!(chest.orientation, furnace.orientation);
    assert!(matches!(chest.orientation, Orientation::Horizontal(_)));
}
