//! M4-B02 acceptance tests: proves `rc_physics::step_living_entity_tick` (M3-B02) is reused
//! completely unmodified for the tier-2 `LivingEntity`-rung kinds (Context §A/§B) — these
//! tests exist to prove *reuse*, not to re-derive `step_living_entity_tick`'s own already-
//! established gravity/drag formula (`crates/physics/tests/motion_golden_vectors.rs` already
//! covers that in full).
//!
//! **Documented deviation** (`docs/findings-for-planning.md`): `step_living_entity_tick`
//! itself has no per-kind dimension parameter at all — it hardcodes `PLAYER_HALF_WIDTH`/
//! `PLAYER_HEIGHT`/`STEP_HEIGHT` as internal module constants (`crates/physics/src/motion.rs`),
//! not accepted as arguments. Context §D's own per-kind tier-2 dimension table (zombie/
//! villager `0.6×1.95`, cow `0.9×1.4`) therefore cannot literally apply to *this* call — every
//! tier-2 kind's own collision geometry during this one call is, in fact, the player's own
//! hitbox. Neither test below depends on the exact AABB width/height (zero horizontal drift,
//! a flat floor with no nearby obstruction), so this substitution does not affect either
//! test's own pass/fail outcome.

use rc_core::BlockPos;
use rc_physics::{
    BlockPhysicsProperties, BlockShapeSource, LivingMotionState, MovementIntent, Vec3,
    step_living_entity_tick,
};

const TOLERANCE: f64 = 1e-9;

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
fn zombie_with_default_intent_falls_straight_down() {
    let shapes = FlatFloorAt(0.0);
    let mut state = LivingMotionState {
        position: Vec3::new(0.5, 10.0, 0.5),
        velocity: Vec3::ZERO,
        on_ground: false,
        fall_distance: 0.0,
    };
    let intent = MovementIntent::default();

    let mut landed = false;
    for _ in 0..2000 {
        state = step_living_entity_tick(state, intent, 0.6, &shapes);
        // Zero strafe/forward -- `MovementIntent::default()` never produces a horizontal
        // input vector at all, so the X/Z position can never drift from spawn on any tick,
        // landed or not.
        assert_close("position.x", state.position.x, 0.5);
        assert_close("position.z", state.position.z, 0.5);
        if state.on_ground {
            landed = true;
            break;
        }
    }
    assert!(landed, "zombie never reached the ground within 2000 ticks");
}

#[test]
fn cow_on_ground_with_default_intent_stays_perfectly_still() {
    let shapes = FlatFloorAt(0.0);
    let mut state = LivingMotionState {
        position: Vec3::new(0.5, 0.0, 0.5),
        velocity: Vec3::ZERO,
        on_ground: true,
        fall_distance: 0.0,
    };
    let intent = MovementIntent::default();

    for tick in 0..10 {
        state = step_living_entity_tick(state, intent, 0.6, &shapes);
        assert_close(&format!("tick {tick} position.x"), state.position.x, 0.5);
        assert_close(&format!("tick {tick} position.y"), state.position.y, 0.0);
        assert_close(&format!("tick {tick} position.z"), state.position.z, 0.5);
    }
}
