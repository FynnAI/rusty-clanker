//! M4-B02 acceptance tests: `step_item_entity_tick`'s free-fall/landing/drag golden vectors
//! (Context §C), hand-derived from that section's own pinned recurrence and checked to
//! `1e-9` absolute tolerance (floating-point noise only, never an algorithmic approximation).
//!
//! `FlatFloorAt`/`EmptyWorld` mirror `crates/physics/tests/motion_golden_vectors.rs`'s own
//! established test-double shape exactly: a resting entity's own *feet* position (this
//! crate's — and vanilla's own — position convention, `rc_physics::Aabb::from_position`'s own
//! `min.y = position.y`) settles exactly at the floor's own top surface, never at
//! `top + half_height`.

use rc_core::BlockPos;
use rc_mechanics::entity::physics::item::{
    ITEM_AIR_DRAG, ITEM_GRAVITY, ItemMotionState, step_item_entity_tick,
};
use rc_physics::{BlockPhysicsProperties, BlockShapeSource, Vec3};

const TOLERANCE: f64 = 1e-9;

struct EmptyWorld;
impl BlockShapeSource for EmptyWorld {
    fn properties_at(&self, _pos: BlockPos) -> BlockPhysicsProperties {
        BlockPhysicsProperties::air()
    }
}

/// A solid floor whose top face is exactly `y = top`; air everywhere else.
struct FlatFloorAt(f64);
impl BlockShapeSource for FlatFloorAt {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        if pos.y as f64 == self.0 - 1.0 {
            BlockPhysicsProperties::default_full_cube()
        } else {
            BlockPhysicsProperties::air()
        }
    }
}

fn assert_close(label: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() < TOLERANCE,
        "{label}: got {got}, want {want} (diff {})",
        (got - want).abs()
    );
}

#[test]
fn item_falls_and_lands_on_flat_ground() {
    let shapes = FlatFloorAt(0.0);
    let mut state = ItemMotionState {
        position: Vec3::new(0.5, 10.0, 0.5),
        velocity: Vec3::ZERO,
        on_ground: false,
        fall_distance: 0.0,
        no_gravity: false,
    };

    let mut landed = false;
    for tick in 0..2000 {
        let prev_velocity_y = state.velocity.y;
        state = step_item_entity_tick(state, &shapes, 0.6);

        if state.on_ground {
            landed = true;
            assert_close("landing position.y", state.position.y, 0.0);
            break;
        }

        // Context §C: subtract gravity, then multiply the (unclipped, in open air) result by
        // air drag -- the exact chain every still-airborne tick must match.
        let expected_vy = (prev_velocity_y - ITEM_GRAVITY) * ITEM_AIR_DRAG;
        assert_close(
            &format!("tick {tick} velocity.y"),
            state.velocity.y,
            expected_vy,
        );
    }
    assert!(landed, "item never reached the ground within 2000 ticks");
}

#[test]
fn item_on_ground_horizontal_velocity_decays_by_air_drag_times_friction() {
    let shapes = FlatFloorAt(0.0);
    let state = ItemMotionState {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(1.0, 0.0, 0.0),
        on_ground: true,
        fall_distance: 0.0,
        no_gravity: false,
    };
    let next = step_item_entity_tick(state, &shapes, 0.6);
    assert_close("velocity.x", next.velocity.x, 1.0 * ITEM_AIR_DRAG * 0.6);
    assert!(next.on_ground, "still resting on the floor");
}

#[test]
fn item_bounces_on_landing_tick() {
    let shapes = FlatFloorAt(0.0);
    // Starts exactly `0.34` above the floor's own top -- falling by `-(0.3 + 0.04)` this
    // tick lands it exactly on the floor, so the halve-invert branch fires on this very
    // landing tick (Context §C step 4), after the ordinary drag multiply (step 3).
    let state = ItemMotionState {
        position: Vec3::new(0.0, 0.34, 0.0),
        velocity: Vec3::new(0.0, -0.3, 0.0),
        on_ground: false,
        fall_distance: 0.34,
        no_gravity: false,
    };
    let next = step_item_entity_tick(state, &shapes, 0.6);
    assert!(next.on_ground, "expected to land exactly this tick");
    let expected_vy = ((-0.3 - ITEM_GRAVITY) * ITEM_AIR_DRAG) * -0.5;
    assert_close("landing-tick velocity.y", next.velocity.y, expected_vy);
}

#[test]
fn no_gravity_item_does_not_fall() {
    let shapes = EmptyWorld;
    let state = ItemMotionState {
        position: Vec3::new(0.0, 50.0, 0.0),
        velocity: Vec3::ZERO,
        on_ground: false,
        fall_distance: 0.0,
        no_gravity: true,
    };
    let next = step_item_entity_tick(state, &shapes, 0.6);
    assert_close("position.y", next.position.y, 50.0);
    assert_close("velocity.x", next.velocity.x, 0.0);
    assert_close("velocity.y", next.velocity.y, 0.0);
    assert_close("velocity.z", next.velocity.z, 0.0);
}
