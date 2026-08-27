//! M3-B03 acceptance test: `rc_physics::raycast::cast_ray`'s own DDA traversal against a
//! handful of synthetic, single/double-block `BlockShapeSource` test doubles -- no chunk
//! storage, no server crate. See `blueprints/M3/M3-B03-breaking-placing.md`, Acceptance
//! tests, "`crates/physics/tests/raycast_basic.rs`".

use rc_core::BlockPos;
use rc_physics::{BlockPhysicsProperties, BlockShapeSource, Vec3, VoxelShape, cast_ray};

/// No blocks anywhere -- every `properties_at` call returns `air()`.
struct EmptyWorld;
impl BlockShapeSource for EmptyWorld {
    fn properties_at(&self, _pos: BlockPos) -> BlockPhysicsProperties {
        BlockPhysicsProperties::air()
    }
}

/// A fixed, closed set of non-air positions, each with its own `VoxelShape` -- every other
/// position is air.
struct FixedBlocks {
    entries: Vec<(BlockPos, VoxelShape)>,
}

impl FixedBlocks {
    fn single(pos: BlockPos, shape: VoxelShape) -> Self {
        FixedBlocks {
            entries: vec![(pos, shape)],
        }
    }

    fn two(a: (BlockPos, VoxelShape), b: (BlockPos, VoxelShape)) -> Self {
        FixedBlocks {
            entries: vec![a, b],
        }
    }
}

impl BlockShapeSource for FixedBlocks {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        for (entry_pos, shape) in &self.entries {
            if *entry_pos == pos {
                return BlockPhysicsProperties {
                    shape: shape.clone(),
                    friction: 0.6,
                    speed_factor: 1.0,
                    jump_factor: 1.0,
                };
            }
        }
        BlockPhysicsProperties::air()
    }
}

/// Normalizes a `Vec3` -- `rc_physics::Vec3` itself exposes no `normalize` (M3-B03's own
/// Deliverables add no method to it, Constraints (b): `rc-physics` gains no new
/// dependency and this file's Constraints leave `vec3.rs` untouched); `cast_ray` itself
/// assumes a pre-normalized `direction` (its own doc comment), so this is the caller's job.
fn normalize(v: Vec3) -> Vec3 {
    let len = v.length_squared().sqrt();
    Vec3::new(v.x / len, v.y / len, v.z / len)
}

#[test]
fn unobstructed_ray_returns_none_within_air() {
    let hit = cast_ray(
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        10.0,
        &EmptyWorld,
    );
    assert_eq!(hit, None);
}

#[test]
fn ray_hits_adjacent_full_cube_immediately() {
    let world = FixedBlocks::single(BlockPos::new(0, 0, 1), VoxelShape::full_cube());
    let hit = cast_ray(
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        10.0,
        &world,
    )
    .expect("the ray must hit the adjacent full cube");
    assert_eq!(hit.block_pos, BlockPos::new(0, 0, 1));
    assert!(
        (hit.distance - 0.5).abs() < 1e-9,
        "expected distance ~0.5, got {}",
        hit.distance
    );
}

#[test]
fn ray_stops_at_first_non_empty_cell_not_a_farther_one() {
    let world = FixedBlocks::two(
        (BlockPos::new(0, 0, 1), VoxelShape::full_cube()),
        (BlockPos::new(0, 0, 2), VoxelShape::full_cube()),
    );
    let hit = cast_ray(
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        10.0,
        &world,
    )
    .expect("the ray must hit the nearer cube");
    assert_eq!(hit.block_pos, BlockPos::new(0, 0, 1));
}

#[test]
fn ray_exceeding_max_distance_returns_none() {
    let world = FixedBlocks::single(BlockPos::new(0, 0, 5), VoxelShape::full_cube());
    let hit = cast_ray(
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        2.0,
        &world,
    );
    assert_eq!(hit, None);
}

#[test]
fn diagonal_ray_visits_cells_in_correct_dda_order() {
    // Full cube at (2, 0, 2) only -- air everywhere else, including at (1, 0, 1), (2, 0, 1),
    // (1, 0, 2), the cells a naive diagonal step might visit out of order.
    let world = FixedBlocks::single(BlockPos::new(2, 0, 2), VoxelShape::full_cube());
    let direction = normalize(Vec3::new(1.0, 0.0, 1.0));
    let hit = cast_ray(Vec3::new(0.5, 0.5, 0.5), direction, 10.0, &world)
        .expect("the diagonal ray must eventually hit (2, 0, 2)");
    assert_eq!(hit.block_pos, BlockPos::new(2, 0, 2));
}
