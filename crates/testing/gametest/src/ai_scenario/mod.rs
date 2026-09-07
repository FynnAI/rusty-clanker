//! `ai_scenario` — a lightweight, `bevy_ecs`-free replay world for Stage 6a/6b
//! (M4-B09 Context Part D), mirroring M3-B07's own established "pure core, lightweight
//! non-`bevy_ecs` replay world" `ReplayWorld` pattern for redstone, applied here to
//! AI/combat.

pub mod assertions;
pub mod spec;
pub mod world;

pub use assertions::{MobTick, ProgressReport, analyze_approach};
pub use spec::{AiScenarioSpec, BlockPlacement, SpecError, load_ai_scenario};
pub use world::{STEP_BLOCKS_PER_TICK, ScenarioMob, ScenarioPlayerProxy, ScenarioWorld};

// `MELEE_ATTACK_RANGE`/`HURT_BY_MEMORY_TTL_TICKS` live in `rc_mechanics::ai::mob_config`
// (Context Part C.4, their one production home — `ZombieAttackGoal`/`HurtBySensor` read
// them directly) and are re-exported here, not redefined, so this crate never carries a
// second, driftable copy of either value.
pub use rc_mechanics::ai::mob_config::{HURT_BY_MEMORY_TTL_TICKS, MELEE_ATTACK_RANGE};
