//! `rc-physics` -- no-ECS-dependency movement/collision physics (MECH-D36-D39), shared
//! unmodified between `rusty-clanker-server`'s Stage 6b simulation and, in Phase 2,
//! `rusty-clanker-client`'s local prediction/reconciliation loop. Every public function
//! takes plain position/velocity/`f32` rotation/bounding-box/world-shape-query inputs and
//! returns a new position/velocity -- no `bevy_ecs::World` reference, no I/O, ever crosses
//! this crate's boundary. Complete normal-dependency set: `{rc-core}` (12-workspace-
//! structure.md's WS-D3 rule 1).

pub mod aabb;
pub mod collide;
pub mod motion;
pub mod raycast;
pub mod shapes;
pub mod trig;
pub mod vec3;

pub use aabb::Aabb;
pub use collide::{collide_and_slide, has_new_collision, overlaps_any_solid, sweep_axis};
pub use motion::{
    AIRBORNE_HORIZONTAL_DRAG, BASE_WALK_SPEED, DEFAULT_BLOCK_FRICTION,
    FRICTION_SPEED_COMPENSATION_BASE, GRAVITY_LIVING, JUMP_BOOST_PER_LEVEL, JUMP_STRENGTH,
    LivingMotionState, MovementIntent, SNEAK_EDGE_STEP, SNEAK_SPEED_MULTIPLIER,
    SPRINT_SPEED_MULTIPLIER, STEP_HEIGHT, step_living_entity_tick,
};
pub use raycast::{RayHit, cast_ray};
pub use shapes::{
    BlockPhysicsProperties, BlockShapeSource, ShapeTable, VoxelShape, tier1_shape_table,
};
pub use trig::{mth_cos, mth_sin};
pub use vec3::Vec3;

/// `Shapes.EPSILON` (14-physics-collision.md §5) -- the collision-geometry epsilon family,
/// distinct from `Mth.EPSILON`/`Vec3.normalize`'s `1e-5`-family constants (18-float-
/// determinism.md §3.12/§4's own explicit warning not to conflate the two).
pub const SHAPE_EPSILON: f64 = 1e-7;

/// Standing player hitbox (Context: "Player dimensions").
pub const PLAYER_HALF_WIDTH: f64 = 0.3;
pub const PLAYER_HEIGHT: f64 = 1.8;
pub const PLAYER_HEIGHT_SNEAKING: f64 = 1.5;
pub const PLAYER_EYE_HEIGHT: f64 = 1.62;
