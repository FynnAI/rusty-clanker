//! Goal/GoalSelector + Brain AI (MECH-D31), Stage-6a/6b access-set discipline
//! (MECH-D32/ARCH-D15), A* pathfinding (MECH-D33), navigation execution, sensing, and
//! the attribute system, at M4 scope. Produces `PendingMovementIntent`
//! (`rc_physics::MovementIntent`-shaped) per entity per tick — Stage 6a's own half of
//! the seam M4-B01 opened; Stage 6b's consumer is a future, unnamed blueprint's job.

pub mod attributes;
pub mod brain;
pub mod goal;
pub mod mob_config;
pub mod navigation;
pub mod pathfinding;
pub mod sensing;
#[cfg(feature = "server-systems")]
pub mod systems;

pub use attributes::{
    AttributeInstance, AttributeMap, AttributeModifier, AttributeModifierId,
    AttributeModifierOperation,
};
pub use brain::{
    ActivityPackage, ActivityRequirement, Behavior, BehaviorStatus, Brain, BrainProgram,
    ExpirableValue, MemoryModuleType, MemoryStatus, Sensor,
};
pub use goal::{
    AiContext, FLAG_JUMP, FLAG_LOOK, FLAG_MOVE, FLAG_TARGET, Goal, GoalSelector, should_full_tick,
};
pub use navigation::{
    JumpControl, LookControl, MAX_TURN_DEGREES_PER_TICK, MoveControl, MoveControlOperation,
    PathNavigation, PendingMovementIntent, rotate_towards,
};
pub use pathfinding::{
    astar::{FUDGING, PathSearchOutcome, find_path},
    node::{NodeEvaluator, PathType, PathTypeTable, WalkNodeEvaluator, tier1_path_type_table},
    path::Path,
};
pub use sensing::{Sensing, nearest_within_range, raycast_line_of_sight};
