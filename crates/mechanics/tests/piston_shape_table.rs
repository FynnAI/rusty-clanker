//! M3-B05 — `piston_head` shape-table acceptance tests (Context §D), pure `rc-physics`.

use rc_mechanics::direction::Direction;
use rc_physics::{BlockPhysicsProperties, Vec3, tier1_shape_table};

/// The same six placeholder literals `crates/physics/src/shapes.rs`'s own six new
/// `tier1_shape_table()` entries key their rows by (Context §D) — kept in sync by hand, exactly
/// like every other placeholder-literal cross-reference this project's own tier-1 blueprints
/// already established (Constraints (b)).
fn piston_head_id_for(facing: Direction) -> u32 {
    match facing {
        Direction::West => 900_001,
        Direction::East => 900_002,
        Direction::North => 900_003,
        Direction::South => 900_004,
        Direction::Down => 900_005,
        Direction::Up => 900_006,
    }
}

const ALL_FACINGS: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::Down,
    Direction::Up,
];

#[test]
fn piston_head_shape_is_non_full_per_facing() {
    for facing in ALL_FACINGS {
        let props = tier1_shape_table().lookup(piston_head_id_for(facing));
        let boxes = props.shape.boxes();
        let is_full_cube = boxes.len() == 1
            && boxes[0].min == Vec3::new(0.0, 0.0, 0.0)
            && boxes[0].max == Vec3::new(1.0, 1.0, 1.0);
        assert!(
            !is_full_cube,
            "piston_head facing {facing:?} must not resolve to a full cube (Context §D — a \
             piston head is not a redstone conductor)"
        );
    }
}

#[test]
fn piston_head_face_plate_thickness_is_platform_thickness() {
    // `facing = Up` is Context §D's own worked reference case: the face plate is the box whose
    // `min.y == 0.75` (`PLATFORM_THICKNESS = 4/16` below the top face).
    let props = tier1_shape_table().lookup(piston_head_id_for(Direction::Up));
    let boxes = props.shape.boxes();
    let plate = boxes
        .iter()
        .find(|b| (b.min.y - 0.75).abs() < 1e-9)
        .expect("face plate box (min.y == 0.75) not found among piston_head's boxes");
    assert!(
        (plate.max.y - plate.min.y - 0.25).abs() < 1e-9,
        "face plate thickness must be exactly 0.25 (4/16, PLATFORM_THICKNESS)"
    );
}

#[test]
fn extended_piston_base_falls_through_to_default_full_cube() {
    // No explicit `tier1_shape_table()` entry exists for an extended piston base at all
    // (Context §D) — any id with no entry falls through to the default. `900_999` is not one
    // of this blueprint's own six `piston_head` ids, nor any other tier-1 entry.
    let props = tier1_shape_table().lookup(900_999);
    assert_eq!(props, BlockPhysicsProperties::default_full_cube());
}
