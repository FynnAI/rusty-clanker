//! Gravity/drag/friction integration (MECH-D37, Context: "Gravity, drag, friction -- exact
//! algorithm and constants").

use crate::collide::{collide_and_slide, sneak_edge_guard, would_still_be_supported};
use crate::trig::{mth_cos, mth_sin};
use crate::{BlockShapeSource, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, Vec3};

pub const GRAVITY_LIVING: f64 = 0.08;
pub const VERTICAL_DRAG: f64 = 0.98;
pub const AIRBORNE_HORIZONTAL_DRAG: f64 = 0.91;
pub const DEFAULT_BLOCK_FRICTION: f64 = 0.6;
pub const FRICTION_SPEED_COMPENSATION_BASE: f64 = 0.216;
pub const BASE_WALK_SPEED: f64 = 0.1;
pub const SPRINT_SPEED_MULTIPLIER: f64 = 1.3;
pub const SNEAK_SPEED_MULTIPLIER: f64 = 0.7;
pub const JUMP_STRENGTH: f64 = 0.42;
pub const JUMP_BOOST_PER_LEVEL: f64 = 0.1;
pub const STEP_HEIGHT: f64 = 0.6;
pub const SNEAK_EDGE_STEP: f64 = 0.05;

/// One tick's player-controlled horizontal/vertical intent (Context, `step_living_entity_tick`).
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct MovementIntent {
    /// `-1.0..=1.0`, matching vanilla's own strafe axis sign convention (positive = right).
    pub strafe: f64,
    /// `-1.0..=1.0`, positive = forward.
    pub forward: f64,
    pub yaw_degrees: f32,
    pub sprinting: bool,
    pub sneaking: bool,
    pub jumping: bool,
    pub jump_boost_amplifier: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LivingMotionState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fall_distance: f64,
}

/// `moveRelative`: rotates `(strafe, forward)` by `yaw_degrees` via the `Mth` sin/cos table,
/// scaled to `speed` (Context, exact formula).
fn get_input_vector(strafe: f64, forward: f64, speed: f64, yaw_degrees: f32) -> Vec3 {
    todo!()
}

/// The full gravity+drag+friction+moveRelative+collision tick (Context, full algorithm).
/// Reserved for entities the server itself fully simulates (a future blueprint's mobs,
/// falling blocks -- MECH-D28 -- and, in Phase 2, the client's own local prediction loop) --
/// **not** called by this blueprint's own network-player validation path, which uses
/// `rc_physics::collide::collide_and_slide` directly (Context: "Server-side movement
/// processing -- the exact reactive model").
pub fn step_living_entity_tick(
    state: LivingMotionState,
    input: MovementIntent,
    ground_friction: f64,
    shapes: &dyn BlockShapeSource,
) -> LivingMotionState {
    todo!()
}
