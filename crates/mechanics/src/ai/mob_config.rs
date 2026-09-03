//! Tier-2 mob AI configurations (M4-B03 blueprint Context §I/§J): per-kind default
//! attribute tables, entity dimensions, and each kind's own `GoalSelector`/
//! `BrainProgram` wiring.

use crate::ai::attributes::AttributeMap;
use crate::ai::brain::{Brain, BrainProgram};
use crate::ai::goal::GoalSelector;
use crate::entity::EntityKind;

/// Context §I's own per-kind table.
pub fn default_attribute_map(kind: EntityKind) -> AttributeMap {
    todo!()
}

/// Context §J's own hand-typed, moderate-confidence dimension table.
pub fn entity_dimensions(kind: EntityKind) -> (f32, f32) {
    todo!()
}

/// Context §J's own Zombie goal-selector table.
pub fn zombie_goal_selector() -> GoalSelector {
    todo!()
}

/// Context §J's own Zombie target-selector table.
pub fn zombie_target_selector() -> GoalSelector {
    todo!()
}

/// Context §J's own Cow goal-selector table (`target_selector` is empty).
pub fn cow_goal_selector() -> GoalSelector {
    todo!()
}

/// Context §J's own Villager `BrainProgram`.
pub fn villager_brain_program() -> BrainProgram {
    todo!()
}

/// Every field a future spawning blueprint needs to attach this blueprint's own AI
/// substrate to one freshly-spawned entity — a plain data bag, not a `bevy_ecs::Bundle`.
#[cfg(feature = "server-systems")]
pub struct MobAiLoadout {
    pub attributes: AttributeMap,
    pub sensing: crate::ai::sensing::Sensing,
    pub navigation: crate::ai::navigation::PathNavigation,
    pub movement_intent: crate::ai::navigation::PendingMovementIntent,
    pub goal_selector: Option<GoalSelector>,
    pub target_selector: Option<GoalSelector>,
    pub brain: Option<(Brain, BrainProgram)>,
}

#[cfg(feature = "server-systems")]
pub fn ai_loadout_for(kind: EntityKind) -> MobAiLoadout {
    todo!()
}
