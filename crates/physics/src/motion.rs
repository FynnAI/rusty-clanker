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
    let len_sq = strafe * strafe + forward * forward;
    if len_sq < 1e-7 {
        return Vec3::ZERO;
    }
    let (s, f) = if len_sq > 1.0 {
        let n = len_sq.sqrt();
        (strafe / n, forward / n)
    } else {
        (strafe, forward)
    };
    let (s, f) = (s * speed, f * speed);
    let angle = yaw_degrees as f64 * std::f64::consts::PI / 180.0;
    let sin = mth_sin(angle) as f64;
    let cos = mth_cos(angle) as f64;
    Vec3::new(s * cos - f * sin, 0.0, s * sin + f * cos)
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
    // 1. Effective speed from sprint/sneak flags.
    let mut speed = BASE_WALK_SPEED;
    if input.sprinting {
        speed *= SPRINT_SPEED_MULTIPLIER;
    }
    if input.sneaking {
        speed *= SNEAK_SPEED_MULTIPLIER;
    }

    // 2. Friction-influenced speed (14 §3.5) -- only matters when grounded on a
    //    more-slippery-than-default block.
    let move_speed = if state.on_ground && ground_friction > DEFAULT_BLOCK_FRICTION {
        speed * (FRICTION_SPEED_COMPENSATION_BASE / ground_friction.powi(3))
    } else {
        speed
    };

    // 3. moveRelative: rotate (strafe, forward) by yaw, ADD to the entity's existing
    //    (this-tick's, i.e. last tick's post-drag) velocity.
    let input_vec = get_input_vector(input.strafe, input.forward, move_speed, input.yaw_degrees);
    let mut velocity = state.velocity + input_vec;

    // 4. Jump impulse -- an instantaneous Y-velocity SET (not an add), applied only when
    //    grounded and requested, BEFORE collision resolution.
    if input.jumping && state.on_ground {
        velocity.y = JUMP_STRENGTH + JUMP_BOOST_PER_LEVEL * input.jump_boost_amplifier as f64;
    }

    // 5. Sneak edge-keep (14 §3.2.1) -- truncates the HORIZONTAL components of `velocity`
    //    toward zero BEFORE collision, only when sneaking, not moving upward net, and
    //    currently on-ground-or-within-STEP_HEIGHT-of-it.
    let currently_grounded = state.on_ground
        || would_still_be_supported(
            state.position,
            PLAYER_HALF_WIDTH,
            PLAYER_HEIGHT,
            0.0,
            0.0,
            shapes,
            STEP_HEIGHT,
        );
    if input.sneaking && velocity.y <= 0.0 && currently_grounded {
        let (dx, dz) = sneak_edge_guard(
            state.position,
            PLAYER_HALF_WIDTH,
            PLAYER_HEIGHT,
            velocity.x,
            velocity.z,
            shapes,
            STEP_HEIGHT,
        );
        velocity.x = dx;
        velocity.z = dz;
    }

    // 6. Collision resolution (Y, then X, then Z).
    let (resolved_delta, new_on_ground) = collide_and_slide(
        state.position,
        PLAYER_HALF_WIDTH,
        PLAYER_HEIGHT,
        velocity,
        shapes,
        STEP_HEIGHT,
    );
    let new_position = state.position + resolved_delta;

    // 7. Gravity subtracted from, then drag multiplied into, the ALREADY-COLLISION-RESOLVED
    //    delta -- stored as next tick's velocity, never applied to THIS tick's position.
    let mut next_velocity = resolved_delta;
    next_velocity.y -= GRAVITY_LIVING;
    let h_drag = if new_on_ground {
        ground_friction
    } else {
        AIRBORNE_HORIZONTAL_DRAG
    };
    next_velocity.x *= h_drag;
    next_velocity.z *= h_drag;
    next_velocity.y *= VERTICAL_DRAG;

    // 8. Fall-distance bookkeeping.
    let mut fall_distance = state.fall_distance;
    if resolved_delta.y < 0.0 {
        fall_distance -= resolved_delta.y;
    }
    if new_on_ground {
        fall_distance = 0.0;
    }

    LivingMotionState {
        position: new_position,
        velocity: next_velocity,
        on_ground: new_on_ground,
        fall_distance,
    }
}
