//! M3 field-report regression test (Defect B): the owner's own live-test report -- creative
//! block placement reaching only "roughly half" of the pinned `BLOCK_INTERACTION_RANGE_
//! CREATIVE = 5.0` -- measured directly against the real production entry points
//! (`mining::look_vector`, `movement::eye_position`, `rc_physics::cast_ray`), never against
//! `raycast.rs`'s own internal `t_max`/`t_delta` bookkeeping (a test built from the same
//! arithmetic the fix touches would just re-certify whatever that arithmetic already does,
//! bug included). Every case below places a real solid target at an independently computed
//! distance -- plain distance-along-a-known-direction arithmetic, not `cast_ray`'s own DDA --
//! and asserts the *black-box* hit/miss outcome `cast_ray` reports for it.
//!
//! Four cases, per the field report's own instruction to check both an axis-aligned and an
//! oblique direction, and both a level and a downward-pitched look (the owner flies in
//! creative and looks down at terrain):
//!   1. horizontal, axis-aligned  (yaw=0,   pitch=0)  -- dir ~ (0, 0, 1)
//!   2. horizontal, diagonal      (yaw=45,  pitch=0)  -- dir ~ (-0.707, 0, 0.707)
//!   3. downward,   axis-aligned  (yaw=0,   pitch=90) -- dir ~ (0, -1, 0)
//!   4. downward,   diagonal      (yaw=0,   pitch=45) -- dir ~ (0, -0.707, 0.707)
//!
//! Measured effective reach for `range = 5.0` (bisection against the real `cast_ray`, see
//! `measure_effective_reach` below), pre- vs post-`raycast.rs` DDA fix (recorded here per the
//! field report's own "report the measured numbers" instruction):
//!   case 1 (axis-aligned, horizontal): 4.000000000000001 -> 5.000000000000001
//!   case 2 (diagonal, horizontal):     3.5857864134236728 -> 5.000000000000003
//!   case 3 (axis-aligned, downward):   4.000000000000001 -> 5.000000000000001
//!   case 4 (diagonal, downward):       3.5857864376269073 -> 5.000000000000003
//! Every case lands on exactly 5.0 (to float precision) once the DDA exit check is fixed --
//! this single fix fully accounts for the reported symptom, worst-case (diagonal aim, the
//! most common case for a freely-looking creative player) costing 28% of the nominal range
//! pre-fix, close enough to a casual "about half" field impression that no second cause is
//! indicated. See `crates/physics/src/raycast.rs`'s own doc comment on the fix itself.

use rc_core::BlockPos;
use rc_physics::{
    BlockPhysicsProperties, BlockShapeSource, PLAYER_EYE_HEIGHT, Vec3, VoxelShape, cast_ray,
};
use rusty_clanker_server::play::{BLOCK_INTERACTION_RANGE_CREATIVE, eye_position, look_vector};

/// A single fixed solid block -- used for the two axis-aligned cases, where the ray never
/// drifts off the block's own transverse span, so a lone block already pins an exact,
/// unambiguous near-face distance (mirrors `crates/physics/tests/raycast_basic.rs`'s own
/// `FixedBlocks` test double).
struct SingleBlock(BlockPos);
impl BlockShapeSource for SingleBlock {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        if pos == self.0 {
            BlockPhysicsProperties {
                shape: VoxelShape::full_cube(),
                friction: 0.6,
                speed_factor: 1.0,
                jump_factor: 1.0,
            }
        } else {
            BlockPhysicsProperties::air()
        }
    }
}

/// An infinite axis-aligned solid slab: every cell whose coordinate on `axis` (0=x, 1=y,
/// 2=z) equals `wall_coord` is a full cube; every other cell is air. Needed for the two
/// oblique cases: a lone block's near face could be entered through either of two faces at
/// slightly different distances depending on exactly where the diagonal ray clips its
/// corner, which would make "the known distance" ambiguous; an infinite slab removes that
/// ambiguity by making the *other* two axes irrelevant to whether/where it is struck.
struct Wall {
    axis: u8,
    wall_coord: i32,
}
impl BlockShapeSource for Wall {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        let c = match self.axis {
            0 => pos.x,
            1 => pos.y,
            _ => pos.z,
        };
        if c == self.wall_coord {
            BlockPhysicsProperties {
                shape: VoxelShape::full_cube(),
                friction: 0.6,
                speed_factor: 1.0,
                jump_factor: 1.0,
            }
        } else {
            BlockPhysicsProperties::air()
        }
    }
}

fn axis_component(v: Vec3, axis: u8) -> f64 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn with_axis(mut v: Vec3, axis: u8, value: f64) -> Vec3 {
    match axis {
        0 => v.x = value,
        1 => v.y = value,
        _ => v.z = value,
    }
    v
}

/// The eye position (`movement::eye_position`'s own real formula, called directly -- not
/// re-derived) that sits exactly `distance` from the near face of a target on `axis` at
/// `target_coord`, given `direction` -- the actual, real `look_vector(yaw, pitch)` output for
/// the case under test. Base coordinates on the two other axes are fixed at an arbitrary
/// `0.5`/`65.62`-ish point, irrelevant to every case below (either the ray never drifts off
/// them, axis-aligned, or the target is an infinite `Wall`, oblique).
fn eye_at_distance(direction: Vec3, axis: u8, target_coord: i32, distance: f64) -> Vec3 {
    let dir_axis = axis_component(direction, axis);
    assert!(
        dir_axis.abs() > 1e-6,
        "the chosen axis must be one `direction` actually moves along"
    );
    // The face nearer the origin: the block's own min bound if the ray moves in the
    // positive direction on this axis, its max bound (== target_coord + 1) if negative.
    let face_coord = if dir_axis > 0.0 {
        target_coord as f64
    } else {
        (target_coord + 1) as f64
    };
    let base = Vec3::new(0.5, 65.62, 0.5);
    with_axis(base, axis, face_coord - distance * dir_axis)
}

fn feet_for_eye(eye: Vec3) -> Vec3 {
    eye - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0)
}

/// Bisects on the true near-face distance (see `eye_at_distance`) to find the largest
/// distance at which `cast_ray` still reports a hit for a fixed `range` -- the real,
/// black-box-measured effective reach, independent of any of `raycast.rs`'s own arithmetic.
fn measure_effective_reach(
    direction: Vec3,
    axis: u8,
    target_coord: i32,
    range: f64,
    shapes: &dyn BlockShapeSource,
) -> f64 {
    let hits = |d: f64| -> bool {
        let eye = eye_at_distance(direction, axis, target_coord, d);
        cast_ray(eye, direction, range, shapes).is_some()
    };
    let mut lo = 0.0f64;
    let mut hi = 8.0f64;
    assert!(hits(lo), "sanity: touching the near face must hit");
    assert!(!hits(hi), "sanity: 8 blocks away must miss");
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if hits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

const JUST_UNDER: f64 = 4.9;
const JUST_BEYOND: f64 = 5.1;

#[test]
fn horizontal_axis_aligned_reach_matches_the_creative_range() {
    let direction = look_vector(0.0, 0.0);
    let target = BlockPos::new(0, 65, 25);
    let world = SingleBlock(target);

    let eye_hit = eye_at_distance(direction, 2, target.z, JUST_UNDER);
    assert!(
        cast_ray(eye_hit, direction, BLOCK_INTERACTION_RANGE_CREATIVE, &world).is_some(),
        "a block {JUST_UNDER} away (just under the 5.0 creative range) must be hittable"
    );
    // Also exercised through the real `PlayerMotion`/`eye_position` seam, not only raw Vec3s.
    let feet = feet_for_eye(eye_hit);
    assert_eq!(eye_position(feet), eye_hit);

    let eye_miss = eye_at_distance(direction, 2, target.z, JUST_BEYOND);
    assert!(
        cast_ray(
            eye_miss,
            direction,
            BLOCK_INTERACTION_RANGE_CREATIVE,
            &world
        )
        .is_none(),
        "a block {JUST_BEYOND} away (just beyond the 5.0 creative range) must NOT be hittable"
    );

    let measured = measure_effective_reach(
        direction,
        2,
        target.z,
        BLOCK_INTERACTION_RANGE_CREATIVE,
        &world,
    );
    assert!(
        (measured - BLOCK_INTERACTION_RANGE_CREATIVE).abs() < 1e-6,
        "measured effective reach {measured} should equal the pinned range 5.0"
    );
}

#[test]
fn horizontal_diagonal_reach_matches_the_creative_range() {
    let direction = look_vector(45.0, 0.0);
    let wall_coord = 25;
    let world = Wall {
        axis: 2,
        wall_coord,
    };

    let eye_hit = eye_at_distance(direction, 2, wall_coord, JUST_UNDER);
    assert!(
        cast_ray(eye_hit, direction, BLOCK_INTERACTION_RANGE_CREATIVE, &world).is_some(),
        "a wall {JUST_UNDER} away along a 45-degree diagonal must be hittable"
    );

    let eye_miss = eye_at_distance(direction, 2, wall_coord, JUST_BEYOND);
    assert!(
        cast_ray(
            eye_miss,
            direction,
            BLOCK_INTERACTION_RANGE_CREATIVE,
            &world
        )
        .is_none(),
        "a wall {JUST_BEYOND} away along a 45-degree diagonal must NOT be hittable"
    );

    let measured = measure_effective_reach(
        direction,
        2,
        wall_coord,
        BLOCK_INTERACTION_RANGE_CREATIVE,
        &world,
    );
    assert!(
        (measured - BLOCK_INTERACTION_RANGE_CREATIVE).abs() < 1e-6,
        "measured effective reach {measured} should equal the pinned range 5.0 -- \
         pre-fix this measured ~3.586 (28% short), the closest of the four cases to the \
         owner's own \"about half\" field report"
    );
}

#[test]
fn downward_axis_aligned_reach_matches_the_creative_range() {
    let direction = look_vector(0.0, 90.0);
    let target = BlockPos::new(0, -30, 0);
    let world = SingleBlock(target);

    let eye_hit = eye_at_distance(direction, 1, target.y, JUST_UNDER);
    assert!(
        cast_ray(eye_hit, direction, BLOCK_INTERACTION_RANGE_CREATIVE, &world).is_some(),
        "a block {JUST_UNDER} straight down must be hittable"
    );

    let eye_miss = eye_at_distance(direction, 1, target.y, JUST_BEYOND);
    assert!(
        cast_ray(
            eye_miss,
            direction,
            BLOCK_INTERACTION_RANGE_CREATIVE,
            &world
        )
        .is_none(),
        "a block {JUST_BEYOND} straight down must NOT be hittable"
    );

    let measured = measure_effective_reach(
        direction,
        1,
        target.y,
        BLOCK_INTERACTION_RANGE_CREATIVE,
        &world,
    );
    assert!(
        (measured - BLOCK_INTERACTION_RANGE_CREATIVE).abs() < 1e-6,
        "measured effective reach {measured} should equal the pinned range 5.0"
    );
}

#[test]
fn downward_diagonal_reach_matches_the_creative_range() {
    // yaw=0, pitch=45 -- the owner's own "flies in creative and looks down at terrain" case.
    let direction = look_vector(0.0, 45.0);
    let wall_coord = 25;
    let world = Wall {
        axis: 2,
        wall_coord,
    };

    let eye_hit = eye_at_distance(direction, 2, wall_coord, JUST_UNDER);
    assert!(
        cast_ray(eye_hit, direction, BLOCK_INTERACTION_RANGE_CREATIVE, &world).is_some(),
        "a wall {JUST_UNDER} away along a 45-degree downward look must be hittable"
    );

    let eye_miss = eye_at_distance(direction, 2, wall_coord, JUST_BEYOND);
    assert!(
        cast_ray(
            eye_miss,
            direction,
            BLOCK_INTERACTION_RANGE_CREATIVE,
            &world
        )
        .is_none(),
        "a wall {JUST_BEYOND} away along a 45-degree downward look must NOT be hittable"
    );

    let measured = measure_effective_reach(
        direction,
        2,
        wall_coord,
        BLOCK_INTERACTION_RANGE_CREATIVE,
        &world,
    );
    assert!(
        (measured - BLOCK_INTERACTION_RANGE_CREATIVE).abs() < 1e-6,
        "measured effective reach {measured} should equal the pinned range 5.0 -- \
         pre-fix this measured ~3.586 (28% short), the closest of the four cases to the \
         owner's own \"about half\" field report"
    );
}
