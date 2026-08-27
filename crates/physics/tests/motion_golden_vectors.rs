//! M3-B02 acceptance tests: `step_living_entity_tick`'s free-fall, friction-stop, and
//! jump-impulse golden vectors (Context, "Gravity, drag, friction -- exact algorithm and
//! constants"), each hand-derived from that section's own pinned recurrence and checked to
//! `1e-9` absolute tolerance (floating-point noise only, never an algorithmic
//! approximation -- Constraints (d)).

use rc_core::BlockPos;
use rc_physics::{
    BlockPhysicsProperties, BlockShapeSource, LivingMotionState, MovementIntent, Vec3,
    step_living_entity_tick,
};

const TOLERANCE: f64 = 1e-9;

/// Open sky, no ground at all -- used only by the falling/jump-ascent tests.
struct EmptyWorld;
impl BlockShapeSource for EmptyWorld {
    fn properties_at(&self, _pos: BlockPos) -> BlockPhysicsProperties {
        BlockPhysicsProperties::air()
    }
}

/// A solid floor whose top face is exactly `y = top`; air everywhere else -- gives the
/// friction-stop/jump tests a real surface to rest on and push off from.
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
fn free_fall_velocity_and_position_sequence() {
    let shapes = EmptyWorld;
    let mut state = LivingMotionState {
        position: Vec3::new(0.0, 100.0, 0.0),
        velocity: Vec3::ZERO,
        on_ground: false,
        fall_distance: 0.0,
    };
    let input = MovementIntent::default();
    let expected = [
        (-0.0784, 100.0),
        (-0.155232, 99.9216),
        (-0.23052736, 99.766368),
        (-0.3043168128, 99.53584064),
    ];
    for (tick, (vy, py)) in expected.into_iter().enumerate() {
        state = step_living_entity_tick(state, input, 0.6, &shapes);
        assert_close(
            &format!("tick {} velocity.y", tick + 1),
            state.velocity.y,
            vy,
        );
        assert_close(
            &format!("tick {} position.y", tick + 1),
            state.position.y,
            py,
        );
    }
}

#[test]
fn friction_stop_decays_geometrically_at_default_friction() {
    let shapes = FlatFloorAt(0.0);
    let mut state = LivingMotionState {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(1.0, 0.0, 0.0),
        on_ground: true,
        fall_distance: 0.0,
    };
    let input = MovementIntent::default();
    let expected = [0.6, 0.36, 0.216];
    for (tick, vx) in expected.into_iter().enumerate() {
        state = step_living_entity_tick(state, input, 0.6, &shapes);
        assert_close(
            &format!("tick {} velocity.x", tick + 1),
            state.velocity.x,
            vx,
        );
        assert!(state.on_ground, "tick {} on_ground", tick + 1);
    }
}

#[test]
fn jump_impulse_then_gravity_decelerates_the_ascent() {
    let shapes = FlatFloorAt(0.0);
    let mut state = LivingMotionState {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::ZERO,
        on_ground: true,
        fall_distance: 0.0,
    };

    // Tick 1: a single jump key-press. Per Context's own algorithm, the jump impulse is set
    // on `velocity` *before* collision resolution (step 4, before step 6) -- unlike
    // gravity's own explicitly-delayed effect (step 7: "computed here but not yet applied
    // to position"), the jump impulse therefore already moves `position` this same tick.
    state = step_living_entity_tick(
        state,
        MovementIntent {
            jumping: true,
            ..Default::default()
        },
        0.6,
        &shapes,
    );
    assert_close("tick 1 velocity.y", state.velocity.y, 0.3332);
    assert_close("tick 1 position.y", state.position.y, 0.42);
    assert!(!state.on_ground, "tick 1 on_ground");

    // Ticks 2-3: no further jump input -- pure gravity/drag deceleration of the ascent.
    let expected = [(0.248136, 0.7532), (0.16477328, 1.001336)];
    for (tick, (vy, py)) in expected.into_iter().enumerate() {
        state = step_living_entity_tick(state, MovementIntent::default(), 0.6, &shapes);
        assert_close(
            &format!("tick {} velocity.y", tick + 2),
            state.velocity.y,
            vy,
        );
        assert_close(
            &format!("tick {} position.y", tick + 2),
            state.position.y,
            py,
        );
    }
}
