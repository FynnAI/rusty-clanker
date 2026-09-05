//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=waived(single canonical value/facing asserted, not a four-way sweep; one-neighbor chest merge only) nondefault-state=yes
//! M3-B03 acceptance test: `resolve_orientation`'s per-block-type table -- pure, no sockets
//! (every world query it needs is an injected closure, never a real `BlockWorldAccess`/ECS
//! dependency -- `resolve_orientation`'s own doc comment in `mining.rs`).
//!
//! Test 1's own worked example (`repeater_faces_away_from_player_facing_case`) is this file's own
//! ground truth for `nearest_horizontal_direction4`'s yaw convention; test 2
//! (`piston_faces_up_when_player_looks_steeply_down`) is the ground truth for `nearest_
//! direction6`'s pitch-sign mapping -- both hand-derived directly from `mining.rs`'s own
//! `look_vector` formula (that module's own top-of-file doc comment has the full
//! reasoning for why the sign this file asserts differs from the blueprint's own inverted
//! restatement of its own formula).
//!
//! M3 field-report test-authoring (torch-candidate loop + chest-merge, superseding the former
//! clicked-face-only torch approximation): every call below now also passes `sneaking` and the
//! two injected world-query closures `resolve_orientation` takes (`is_full_cube_at`/
//! `chest_neighbor_at`) -- `&mut |_| true`/`&mut |_| false`/`&mut |_| None` stand in for "every
//! neighbor is a full-cube conductor" / "nothing supports anything" / "no chest anywhere"
//! respectively, chosen per test to exercise the specific candidate/merge branch under test.
//! `torch_on_bottom_face_is_rejected` (the old test asserting a bottom-face click is ALWAYS
//! rejected) is replaced by two tests below reflecting the real, world-conditional rule: reject
//! only when no candidate direction has support, and fall back to a floor torch when the block
//! below the target cell is solid -- the exact "redstone_torch/dir_north/face_bottom_of_ceiling/
//! pitch_level" placement-diff regression this fix closes.

use rc_mechanics::Direction;
use rc_physics::Vec3;
use rusty_clanker_server::play::{
    ChestType, Face, Orientation, PlaceableBlockKind, RejectReason, resolve_orientation,
};

#[test]
fn repeater_faces_away_from_player_facing_case() {
    // yaw = 0.0 -> looking South (this project's own `look_vector` convention); a repeater
    // faces *away* from the player, i.e. North.
    let selection = resolve_orientation(
        PlaceableBlockKind::Repeater,
        Face::Up,
        0.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Horizontal(Direction::North)
    );
    assert!(!selection.is_wall_variant);
}

#[test]
fn piston_faces_up_when_player_looks_steeply_down() {
    let selection = resolve_orientation(
        PlaceableBlockKind::Piston,
        Face::Up,
        0.0,
        80.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(selection.orientation, Orientation::Full(Direction::Up));
}

#[test]
fn torch_on_top_face_is_standing() {
    // Clicked face's opposite (Down) is front-inserted and, with every neighbor reported
    // solid, is the very first candidate tried -- a floor torch, matching the pre-fix rule's
    // own result for this exact input (this is the "already worked, stays working" case the
    // real-connection `wall_and_floor_redstone_torch_orientation_over_real_connection` test
    // also exercises end-to-end).
    let selection = resolve_orientation(
        PlaceableBlockKind::RedstoneTorch,
        Face::Up,
        0.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(selection.kind, PlaceableBlockKind::RedstoneTorch);
    assert_eq!(selection.orientation, Orientation::None);
    assert!(!selection.is_wall_variant);
}

#[test]
fn torch_on_side_face_is_wall_variant_facing_that_side() {
    // Clicked face's opposite (South) is front-inserted and, with every neighbor reported
    // solid, is the very first candidate tried -- FACING = South.opposite() = North, matching
    // the pre-fix rule's own result for this exact input.
    let selection = resolve_orientation(
        PlaceableBlockKind::RedstoneTorch,
        Face::North,
        0.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Horizontal(Direction::North)
    );
    assert!(selection.is_wall_variant);
}

#[test]
fn torch_on_bottom_face_is_rejected_when_no_candidate_has_support() {
    // Nothing anywhere is solid (`&mut |_| false`) -- every one of the six candidates
    // (Up excluded outright) fails its own support check, so placement fails outright,
    // exactly like vanilla's own "no valid `getStateForPlacement` result" refusal.
    let result = resolve_orientation(
        PlaceableBlockKind::RedstoneTorch,
        Face::Down,
        0.0,
        0.0,
        false,
        &mut |_, _| false,
        &mut |_| None,
    );
    assert_eq!(result, Err(RejectReason::InvalidTorchFace));
}

#[test]
fn torch_on_bottom_face_falls_back_to_floor_torch_when_the_floor_below_is_solid_nondefault_case() {
    // The exact placement-diff regression this fix closes
    // (`redstone_torch/dir_north/face_bottom_of_ceiling/pitch_level`): clicking the underside
    // of a ceiling block front-inserts `Up` (always invalid, skipped), then every horizontal
    // candidate fails (no walls reported solid) until `Down` is reached -- valid, since the
    // floor below the target cell IS reported solid -- landing a floor torch, never a
    // rejection, despite the clicked face being `Down`.
    let selection = resolve_orientation(
        PlaceableBlockKind::RedstoneTorch,
        Face::Down,
        0.0,
        0.0,
        false,
        &mut |dir, _kind| dir == Direction::Down,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(selection.orientation, Orientation::None);
    assert!(!selection.is_wall_variant);
}

#[test]
fn hopper_faces_opposite_the_clicked_side_face() {
    let selection = resolve_orientation(
        PlaceableBlockKind::Hopper,
        Face::North,
        0.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Horizontal(Direction::South)
    );
}

#[test]
fn hopper_clicked_on_top_defaults_to_facing_down_never_up() {
    let selection = resolve_orientation(
        PlaceableBlockKind::Hopper,
        Face::Up,
        0.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(selection.orientation, Orientation::Full(Direction::Down));

    // The clamp itself: naive opposite of `Down` would be `Up`, clamped back to `Down`.
    let selection = resolve_orientation(
        PlaceableBlockKind::Hopper,
        Face::Down,
        0.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    assert_eq!(selection.orientation, Orientation::Full(Direction::Down));
}

#[test]
fn chest_and_furnace_share_the_same_horizontal_away_from_player_rule() {
    // No neighbor anywhere (`&mut |_| None`) -- chest resolves to a plain `Single`, whose own
    // FACING must still match furnace's identical yaw-driven rule.
    let chest = resolve_orientation(
        PlaceableBlockKind::Chest,
        Face::Up,
        90.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    let furnace = resolve_orientation(
        PlaceableBlockKind::Furnace,
        Face::Up,
        90.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |_| None,
    )
    .unwrap();
    let chest_facing = match chest.orientation {
        Orientation::Chest(dir, ChestType::Single) => dir,
        other => panic!("expected Orientation::Chest(_, Single), got {other:?}"),
    };
    let furnace_facing = match furnace.orientation {
        Orientation::Horizontal(dir) => dir,
        other => panic!("expected Orientation::Horizontal, got {other:?}"),
    };
    assert_eq!(chest_facing, furnace_facing);
    assert!(chest.chest_merge.is_none());
}

// --- Chest-merge (M3 field-report test-authoring, pure coverage alongside the real-connection
// tests in `play_placement_candidate_field_report.rs`) ---

#[test]
fn chest_merges_left_with_a_same_facing_clockwise_neighbor() {
    use rusty_clanker_server::play::ChestNeighbor;

    // yaw 0.0 -> base FACING North (this file's own established convention). North's own
    // clockwise neighbor direction is East -- reporting a SINGLE, North-facing chest there
    // (and nothing at the counter-clockwise direction) must merge as LEFT.
    let selection = resolve_orientation(
        PlaceableBlockKind::Chest,
        Face::Up,
        0.0,
        0.0,
        false,
        &mut |_, _| true,
        &mut |dir| {
            if dir == Direction::East {
                Some(ChestNeighbor {
                    facing: Direction::North,
                    is_single: true,
                })
            } else {
                None
            }
        },
    )
    .unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Chest(Direction::North, ChestType::Left)
    );
    let merge = selection.chest_merge.expect("expected a chest merge");
    assert_eq!(merge.neighbor_direction, Direction::East);
    assert_eq!(merge.neighbor_facing, Direction::North);
    assert_eq!(merge.neighbor_new_type, ChestType::Right);
}

#[test]
fn sneak_click_on_a_perpendicular_chest_adopts_its_facing() {
    use rusty_clanker_server::play::ChestNeighbor;

    // Player's own yaw would otherwise resolve FACING = East (yaw 90 -> look West -> opposite
    // East, this file's own established convention) -- but sneak-clicking a North-facing
    // chest's own East face adopts that chest's own North facing instead, proving the
    // adoption really overrides the player-yaw-based default rather than coincidentally
    // agreeing with it.
    let selection = resolve_orientation(
        PlaceableBlockKind::Chest,
        Face::East,
        90.0,
        0.0,
        true,
        &mut |_, _| true,
        &mut |dir| {
            // `clicked_face.opposite()` (East.opposite() = West) is the direction FROM the
            // new chest's own target TO the clicked (existing) chest.
            if dir == Direction::West {
                Some(ChestNeighbor {
                    facing: Direction::North,
                    is_single: true,
                })
            } else {
                None
            }
        },
    )
    .unwrap();
    assert_eq!(
        selection.orientation,
        Orientation::Chest(Direction::North, ChestType::Left)
    );
    let merge = selection.chest_merge.expect("expected a chest merge");
    assert_eq!(merge.neighbor_direction, Direction::West);
    assert_eq!(merge.neighbor_facing, Direction::North);
    assert_eq!(merge.neighbor_new_type, ChestType::Right);
}

// --- `ordered_by_nearest` (M3 field-report test-authoring, torch-candidate loop: "write a
// unit test for a few look vectors") ---

#[test]
fn ordered_by_nearest_matches_a_south_look() {
    // South (0,0,1): South is the sole positive-dot direction (dot 1); East/West/Up/Down all
    // tie at dot 0, broken by the N,E,S,W,U,D tie order; North is the sole negative (dot -1).
    let order = rusty_clanker_server::play::ordered_by_nearest(Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(
        order,
        [
            Direction::South,
            Direction::East,
            Direction::West,
            Direction::Up,
            Direction::Down,
            Direction::North,
        ]
    );
}

#[test]
fn ordered_by_nearest_matches_a_straight_down_look() {
    // Down (0,-1,0): Down alone at dot 1; every horizontal direction ties at dot 0 (N,E,S,W
    // tie order); Up alone at dot -1.
    let order = rusty_clanker_server::play::ordered_by_nearest(Vec3::new(0.0, -1.0, 0.0));
    assert_eq!(
        order,
        [
            Direction::Down,
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
            Direction::Up,
        ]
    );
}

#[test]
fn ordered_by_nearest_matches_an_east_look() {
    // East (1,0,0): East alone at dot 1; North/South/Up/Down tie at dot 0 (N,S,U,D tie
    // order -- East and West are excluded from that tied group, each having its own distinct
    // nonzero dot); West alone at dot -1.
    let order = rusty_clanker_server::play::ordered_by_nearest(Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(
        order,
        [
            Direction::East,
            Direction::North,
            Direction::South,
            Direction::Up,
            Direction::Down,
            Direction::West,
        ]
    );
}
