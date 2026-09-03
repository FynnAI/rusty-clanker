//! Item-entity tick shape (`14-physics-collision.md` §3.3, Context §C) — genuinely different
//! add-vs-multiply operation ORDER from the living-entity tick shape (`rc_physics::motion::
//! step_living_entity_tick`, reused completely unmodified by this blueprint's own Stage 6b
//! system for every `LivingEntity`-rung tier-2 kind).

use rc_physics::{BlockShapeSource, Vec3, collide_and_slide};

pub const ITEM_GRAVITY: f64 = 0.04;
pub const ITEM_AIR_DRAG: f64 = 0.98;
pub const ITEM_HALF_WIDTH: f64 = 0.125;
pub const ITEM_HEIGHT: f64 = 0.25;
/// Item entities do not step up onto low ledges the way a player does — this project's own
/// reasonable restatement (Context §C, flagged moderate-confidence).
pub const ITEM_STEP_HEIGHT: f64 = 0.0;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ItemMotionState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fall_distance: f64,
    pub no_gravity: bool,
}

/// Context §C — the complete item-entity tick (subtract-gravity, move, multiply-drag,
/// conditional halve-invert). `ground_friction` is the supporting block's own friction value
/// (`rc_physics::BlockPhysicsProperties::friction`, looked up by the caller exactly as
/// `evaluate_movement`/`step_living_entity_tick` already do).
pub fn step_item_entity_tick(
    state: ItemMotionState,
    shapes: &dyn BlockShapeSource,
    ground_friction: f64,
) -> ItemMotionState {
    let mut velocity = state.velocity;

    // Step 1: subtract gravity from vertical velocity, before collision resolution --
    // gated by `no_gravity` exactly as vanilla's `Entity.getGravity()` checks its own
    // `isNoGravity()` flag.
    if !state.no_gravity {
        velocity.y -= ITEM_GRAVITY;
    }

    // Step 2: collision resolution.
    let (resolved_delta, on_ground) = collide_and_slide(
        state.position,
        ITEM_HALF_WIDTH,
        ITEM_HEIGHT,
        velocity,
        shapes,
        ITEM_STEP_HEIGHT,
    );
    let new_position = state.position + resolved_delta;
    velocity = resolved_delta;

    // Step 3: drag multiply -- Y always by `ITEM_AIR_DRAG`; X/Z by `ITEM_AIR_DRAG *
    // ground_friction` when on the ground, else `ITEM_AIR_DRAG` alone.
    let h_drag = if on_ground {
        ITEM_AIR_DRAG * ground_friction
    } else {
        ITEM_AIR_DRAG
    };
    velocity.x *= h_drag;
    velocity.z *= h_drag;
    velocity.y *= ITEM_AIR_DRAG;

    // Step 4: on-ground, still-falling halve-and-invert -- fires only on the landing tick
    // itself, after the ordinary drag multiply.
    if on_ground && velocity.y < 0.0 {
        velocity.y *= -0.5;
    }

    let mut fall_distance = state.fall_distance;
    if resolved_delta.y < 0.0 {
        fall_distance -= resolved_delta.y;
    }
    if on_ground {
        fall_distance = 0.0;
    }

    ItemMotionState {
        position: new_position,
        velocity,
        on_ground,
        fall_distance,
        no_gravity: state.no_gravity,
    }
}
