//! Navigation execution: `PathNavigation`, `MoveControl`, `LookControl`, `JumpControl`
//! (MECH-D33, M4-B03 blueprint Context §G) — the produce-side of the Stage-6a→Stage-6b
//! seam M4-B01 opened.

use std::collections::HashMap;

use rc_core::BlockPos;

use crate::ai::pathfinding::node::{NodeEvaluator, PathType};
use crate::ai::pathfinding::path::Path;
use crate::world_access::BlockWorldAccess;

pub const MAX_TURN_DEGREES_PER_TICK: f32 = 90.0;
pub const MOVE_CONTROL_ARRIVAL_EPSILON_SQ: f64 = 2.5e-4;

#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Component))]
#[derive(Clone, Debug, Default)]
pub struct PathNavigation {
    pub current_path: Option<Path>,
    /// Counts down from 20; recompute only allowed at 0.
    pub recompute_cooldown_ticks: u32,
    /// Counts down from 100.
    pub stuck_check_countdown: u32,
    pub position_at_last_stuck_check: Option<[f64; 3]>,
    pub is_stuck: bool,
    /// Per-instance override — empty by default for every tier-2 kind at M4 scope.
    pub malus_overrides: HashMap<PathType, f32>,
}

impl PathNavigation {
    /// One navigation tick: recompute throttle + stuck detection (Context §G,
    /// restated field-precise).
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        entity_pos: [f64; 3],
        goal_pos: Option<BlockPos>,
        movement_speed_attr: f64,
        evaluator: &dyn NodeEvaluator,
        world: &dyn BlockWorldAccess,
        entity_height: f32,
        max_visited_nodes: u32,
    ) -> Option<u32> {
        todo!()
    }
}

/// `Jumping` added to vanilla's own `MoveControl.Operation.JUMPING` state (Context §G).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MoveControlOperation {
    Wait,
    MoveTo,
    Jumping,
}

pub struct MoveControl {
    pub operation: MoveControlOperation,
    pub wanted_pos: [f64; 3],
    pub speed_modifier: f64,
}

impl MoveControl {
    /// Context §G, restated field-precise: the `Wait`/arrival-epsilon zero-forward
    /// case, and the `MoveTo`/`Jumping` turn-and-drive case with its own real jump
    /// trigger.
    pub fn tick(
        &mut self,
        current_pos: [f64; 3],
        current_yaw: f32,
        on_ground: bool,
        step_height: f64,
        entity_width: f32,
    ) -> (f64, f32, bool) {
        todo!()
    }
}

/// Straightforward shortest-angle rotation clamped to `max_degrees_per_tick` —
/// normalizes the raw `target - current` delta into `(-180, 180]` before clamping, so
/// a mob never spins the "long way around."
pub fn rotate_towards(current_degrees: f32, target_degrees: f32, max_degrees_per_tick: f32) -> f32 {
    todo!()
}

pub struct LookControl;

impl LookControl {
    /// `desired_yaw`/`desired_pitch` from `atan2` toward `target`, both axes
    /// independently clamped via `rotate_towards` (Context §G).
    pub fn tick(
        &self,
        current_yaw: f32,
        current_pitch: f32,
        target: Option<[f64; 3]>,
        eye_pos: [f64; 3],
    ) -> (f32, f32) {
        todo!()
    }
}

pub struct JumpControl;

impl JumpControl {
    /// `rise_to_target > step_height && horizontal_dist_sq < f64::max(1.0,
    /// entity_width as f64)` — vanilla's own literal `MoveControl.tick` trigger
    /// condition, restated exactly, including its own unsquared-width comparison.
    pub fn should_jump(
        rise_to_target: f64,
        horizontal_dist_sq: f64,
        step_height: f64,
        entity_width: f32,
    ) -> bool {
        rise_to_target > step_height && horizontal_dist_sq < f64::max(1.0, entity_width as f64)
    }
}

/// One per Stage-6a-ticked entity, overwritten every tick (Context §G).
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Component))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct PendingMovementIntent(pub rc_physics::MovementIntent);
