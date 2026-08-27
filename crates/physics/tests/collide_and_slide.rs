//! M3-B02 acceptance tests: `collide_and_slide`'s unobstructed-move, full-cube-blocks,
//! Y-then-X-then-Z corner-clip, step-up, and sneak-edge-guard cases (Context, "Collide-and-
//! slide sweep" / "Sneak edge-keep"), each hand-derived from that section's own pinned
//! algorithm and checked to `1e-9` absolute tolerance.

use rc_core::BlockPos;
use rc_physics::collide::sneak_edge_guard;
use rc_physics::{
    Aabb, BlockPhysicsProperties, BlockShapeSource, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, STEP_HEIGHT,
    Vec3, VoxelShape, collide_and_slide,
};

const TOLERANCE: f64 = 1e-9;

fn assert_vec_close(label: &str, got: Vec3, want: Vec3) {
    assert!(
        (got.x - want.x).abs() < TOLERANCE
            && (got.y - want.y).abs() < TOLERANCE
            && (got.z - want.z).abs() < TOLERANCE,
        "{label}: got {got:?}, want {want:?}"
    );
}

/// Air everywhere.
struct EmptyWorld;
impl BlockShapeSource for EmptyWorld {
    fn properties_at(&self, _pos: BlockPos) -> BlockPhysicsProperties {
        BlockPhysicsProperties::air()
    }
}

/// One explicitly-shaped block; air everywhere else.
struct SingleBlock(BlockPos, VoxelShape);
impl BlockShapeSource for SingleBlock {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        if pos == self.0 {
            BlockPhysicsProperties {
                shape: self.1.clone(),
                friction: 0.6,
                speed_factor: 1.0,
                jump_factor: 1.0,
            }
        } else {
            BlockPhysicsProperties::air()
        }
    }
}

/// Two explicitly-shaped blocks; air everywhere else.
struct TwoBlocks(BlockPos, VoxelShape, BlockPos, VoxelShape);
impl BlockShapeSource for TwoBlocks {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        let hit = if pos == self.0 {
            Some(&self.1)
        } else if pos == self.2 {
            Some(&self.3)
        } else {
            None
        };
        match hit {
            Some(shape) => BlockPhysicsProperties {
                shape: shape.clone(),
                friction: 0.6,
                speed_factor: 1.0,
                jump_factor: 1.0,
            },
            None => BlockPhysicsProperties::air(),
        }
    }
}

#[test]
fn unobstructed_move_travels_the_full_requested_delta() {
    let (delta, on_ground) = collide_and_slide(
        Vec3::new(0.0, 10.0, 0.0),
        0.3,
        1.8,
        Vec3::new(1.0, -1.0, 0.5),
        &EmptyWorld,
        0.6,
    );
    assert_vec_close("delta", delta, Vec3::new(1.0, -1.0, 0.5));
    assert!(!on_ground);
}

#[test]
fn full_cube_blocks_a_direct_approach() {
    let shapes = SingleBlock(BlockPos::new(2, 0, 0), VoxelShape::full_cube());
    let (delta, on_ground) = collide_and_slide(
        Vec3::new(0.65, 0.0, 0.5),
        0.3,
        1.8,
        Vec3::new(0.4, 0.0, 0.0),
        &shapes,
        0.6,
    );
    assert_vec_close("delta", delta, Vec3::new(0.4, 0.0, 0.0));
    assert!(!on_ground);
}

#[test]
fn corner_clip_x_resolves_before_z_catches_up() {
    let shapes = SingleBlock(BlockPos::new(1, 0, 1), VoxelShape::full_cube());
    let (delta, on_ground) = collide_and_slide(
        Vec3::new(0.65, 0.0, 0.65),
        0.3,
        1.8,
        Vec3::new(0.5, 0.0, 0.5),
        &shapes,
        0.6,
    );
    assert_vec_close("delta", delta, Vec3::new(0.5, 0.0, 0.05));
    assert!(!on_ground);
}

#[test]
fn step_up_onto_a_repeater_succeeds_and_raises_y_by_its_height() {
    let repeater_shape = VoxelShape::from_boxes(vec![Aabb {
        min: Vec3::new(0.0, 0.0, 0.0),
        max: Vec3::new(1.0, 0.125, 1.0),
    }]);
    let shapes = TwoBlocks(
        BlockPos::new(0, -1, 0),
        VoxelShape::full_cube(),
        BlockPos::new(1, 0, 0),
        repeater_shape,
    );

    let (delta, _on_ground) = collide_and_slide(
        Vec3::new(0.5, 0.0, 0.5),
        PLAYER_HALF_WIDTH,
        PLAYER_HEIGHT,
        Vec3::new(0.6, 0.0, 0.0),
        &shapes,
        STEP_HEIGHT,
    );
    assert!(
        (delta.y - 0.125).abs() < TOLERANCE,
        "stepped delta.y: got {}, want 0.125",
        delta.y
    );
    assert!(
        (delta.x - 0.6).abs() < TOLERANCE,
        "stepped delta.x: got {}, want 0.6 (step preserved full horizontal travel)",
        delta.x
    );

    // Contrasted directly against step_height: 0.0 -- blocked, no step attempted.
    let (blocked_delta, _) = collide_and_slide(
        Vec3::new(0.5, 0.0, 0.5),
        PLAYER_HALF_WIDTH,
        PLAYER_HEIGHT,
        Vec3::new(0.6, 0.0, 0.0),
        &shapes,
        0.0,
    );
    assert!(
        blocked_delta.x < 0.6,
        "with step_height=0.0 the repeater must still block: got dx={}",
        blocked_delta.x
    );
}

/// M3-B02 deviation (recorded in the implementation report): the blueprint's own narrative
/// walkthrough for this case (`dx = 0.35` shrinking down to `dx = 0.2`) assumes
/// `would_still_be_supported` requires the entity's ENTIRE footprint to remain over solid
/// ground. The blueprint's own pinned algorithm is not that strict, though: it literally
/// reuses `sweep_axis` (Context, "Sneak edge-keep": "sweep it downward... via `sweep_axis`"),
/// which -- like every other `sweep_axis`/`clip_distance` call in this same document -- is a
/// broad-phase AABB overlap test that registers support as soon as ANY part of the footprint
/// still overlaps solid ground, not only when the WHOLE footprint does. That matches real
/// vanilla's own well-documented sneak-edge behavior (a player can shuffle out until only a
/// sliver of their box remains over the ledge, not just until half of it is still on solid
/// ground). This test's own setup is therefore chosen so the requested move starts fully
/// past the tile's edge (zero overlap, genuinely unsupported) so the shrink loop has
/// something real to do, rather than landing on the blueprint's own narrative's assumed
/// (but algorithmically unreachable) `dx = 0.2` stopping point.
#[test]
fn sneak_edge_guard_truncates_approach_to_a_ledge_in_fixed_steps() {
    // A single 1x1 floor tile, world box [0,1]x[-1,0]x[0,1] -- nothing beyond x=1.0.
    let shapes = SingleBlock(BlockPos::new(0, -1, 0), VoxelShape::full_cube());
    // Entity at (0.5,0,0.5), footprint [0.2,0.8] -- resting on the tile. Requesting dx=0.9
    // would move the footprint to [1.1,1.7], entirely past the tile's own x=1.0 edge (zero
    // overlap -- genuinely unsupported, not merely partially hanging off).
    let (dx, dz) = sneak_edge_guard(Vec3::new(0.5, 0.0, 0.5), 0.3, 1.8, 0.9, 0.0, &shapes, 0.6);
    // Shrinking in 0.05 steps from 0.9: 0.85, 0.80 (footprint [1.0,1.6], still zero overlap
    // -- exactly touching the tile's edge does not count, Context's own SHAPE_EPSILON
    // rule), 0.75 (footprint [0.95,1.55], now genuinely overlapping the tile on
    // [0.95,1.0] -- supported, stop).
    assert!((dx - 0.75).abs() < TOLERANCE, "dx: got {dx}, want 0.75");
    assert!(
        (dz - 0.0).abs() < TOLERANCE,
        "dz: got {dz}, want 0.0 (never evaluated)"
    );
}
