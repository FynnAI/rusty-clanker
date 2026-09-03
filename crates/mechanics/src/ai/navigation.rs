//! Navigation execution: `PathNavigation`, `MoveControl`, `LookControl`, `JumpControl`
//! (MECH-D33, M4-B03 blueprint Context §G) — the produce-side of the Stage-6a→Stage-6b
//! seam M4-B01 opened.

use std::collections::HashMap;

use rc_core::BlockPos;

use crate::ai::pathfinding::astar::find_path;
use crate::ai::pathfinding::node::{NodeEvaluator, PathType};
use crate::ai::pathfinding::path::Path;
use crate::world_access::BlockWorldAccess;

pub const MAX_TURN_DEGREES_PER_TICK: f32 = 90.0;
pub const MOVE_CONTROL_ARRIVAL_EPSILON_SQ: f64 = 2.5e-4;

/// `PathNavigation`'s own `FOLLOW_RANGE`-derived reach range (Context §F: A*'s own
/// Manhattan `reach_range` parameter) — a fixed, modest "close enough to the goal
/// block" radius, since no per-call caller-supplied override exists in this
/// blueprint's own `PathNavigation::tick` signature.
const NAVIGATION_REACH_RANGE: f64 = 0.0;
/// Vanilla's own `MAX_TIME_RECOMPUTE` (Context §G).
const MAX_TIME_RECOMPUTE: u32 = 20;
/// Vanilla's own `STUCK_CHECK_INTERVAL` (Context §G).
const STUCK_CHECK_INTERVAL: u32 = 100;
/// Vanilla's own `STUCK_CHECK_INTERVAL * STUCK_THRESHOLD_DISTANCE_FACTOR` (Context §G).
const STUCK_THRESHOLD_MULTIPLIER: f64 = 25.0;

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
        self.recompute_cooldown_ticks = self.recompute_cooldown_ticks.saturating_sub(1);
        self.stuck_check_countdown = self.stuck_check_countdown.saturating_sub(1);

        let mut ran_search = None;

        if self.current_path.is_none() && goal_pos.is_some() && self.recompute_cooldown_ticks == 0 {
            let goal = goal_pos.expect("checked Some above");
            let start = BlockPos::new(
                entity_pos[0].floor() as i32,
                entity_pos[1].round() as i32,
                entity_pos[2].floor() as i32,
            );
            let outcome = find_path(
                start,
                &[goal],
                NAVIGATION_REACH_RANGE,
                evaluator,
                world,
                entity_height,
                &self.malus_overrides,
                max_visited_nodes,
            );
            self.current_path = outcome.path;
            self.recompute_cooldown_ticks = MAX_TIME_RECOMPUTE;
            ran_search = Some(outcome.nodes_visited);
        }

        if self.stuck_check_countdown == 0 {
            match self.position_at_last_stuck_check {
                None => {
                    self.position_at_last_stuck_check = Some(entity_pos);
                }
                Some(last) => {
                    let effective_speed = if movement_speed_attr >= 1.0 {
                        movement_speed_attr
                    } else {
                        movement_speed_attr * movement_speed_attr
                    };
                    let threshold = effective_speed * STUCK_THRESHOLD_MULTIPLIER;
                    let dx = entity_pos[0] - last[0];
                    let dy = entity_pos[1] - last[1];
                    let dz = entity_pos[2] - last[2];
                    let dist_sq = dx * dx + dy * dy + dz * dz;
                    if dist_sq < threshold * threshold {
                        self.is_stuck = true;
                        self.current_path = None;
                    }
                    self.position_at_last_stuck_check = Some(entity_pos);
                }
            }
            self.stuck_check_countdown = STUCK_CHECK_INTERVAL;
        }

        ran_search
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
        let dx = self.wanted_pos[0] - current_pos[0];
        let dy = self.wanted_pos[1] - current_pos[1];
        let dz = self.wanted_pos[2] - current_pos[2];
        let horizontal_dist_sq = dx * dx + dz * dz;

        if matches!(self.operation, MoveControlOperation::Wait)
            || horizontal_dist_sq < MOVE_CONTROL_ARRIVAL_EPSILON_SQ
        {
            return (0.0, current_yaw, false);
        }

        let desired_yaw = (dz.atan2(dx)).to_degrees() as f32 - 90.0;
        let new_yaw = rotate_towards(current_yaw, desired_yaw, MAX_TURN_DEGREES_PER_TICK);
        let forward = 1.0;

        let jumping = if matches!(self.operation, MoveControlOperation::Jumping) {
            if on_ground {
                self.operation = MoveControlOperation::MoveTo;
                false
            } else {
                true
            }
        } else if JumpControl::should_jump(dy, horizontal_dist_sq, step_height, entity_width) {
            self.operation = MoveControlOperation::Jumping;
            true
        } else {
            self.operation = MoveControlOperation::MoveTo;
            false
        };

        (forward, new_yaw, jumping)
    }
}

/// Straightforward shortest-angle rotation clamped to `max_degrees_per_tick` —
/// normalizes the raw `target - current` delta into `(-180, 180]` before clamping, so
/// a mob never spins the "long way around."
pub fn rotate_towards(current_degrees: f32, target_degrees: f32, max_degrees_per_tick: f32) -> f32 {
    let mut delta = (target_degrees - current_degrees) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    let clamped = delta.clamp(-max_degrees_per_tick, max_degrees_per_tick);
    current_degrees + clamped
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
        match target {
            None => (current_yaw, current_pitch),
            Some(target) => {
                let dx = target[0] - eye_pos[0];
                let dy = target[1] - eye_pos[1];
                let dz = target[2] - eye_pos[2];
                let horizontal = (dx * dx + dz * dz).sqrt();
                let desired_yaw = (dz.atan2(dx)).to_degrees() as f32 - 90.0;
                let desired_pitch = -(dy.atan2(horizontal)).to_degrees() as f32;
                let new_yaw = rotate_towards(current_yaw, desired_yaw, MAX_TURN_DEGREES_PER_TICK);
                let new_pitch =
                    rotate_towards(current_pitch, desired_pitch, MAX_TURN_DEGREES_PER_TICK);
                (new_yaw, new_pitch)
            }
        }
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
