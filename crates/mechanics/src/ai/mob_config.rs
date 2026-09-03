//! Tier-2 mob AI configurations (M4-B03 blueprint Context §I/§J): per-kind default
//! attribute tables, entity dimensions, and each kind's own `GoalSelector`/
//! `BrainProgram` wiring.

use rc_core::RcEntityId;
use rc_registries::generated_v776::registries::attribute;

use crate::ai::attributes::{AttributeInstance, AttributeMap};
use crate::ai::brain::{
    Activity, ActivityPackage, ActivityRequirement, Behavior, Brain, BrainProgram,
    MemoryModuleType, MemoryStatus, Sensor,
};
use crate::ai::goal::{AiContext, Goal, GoalSelector, FLAG_JUMP, FLAG_LOOK, FLAG_MOVE, FLAG_TARGET};
use crate::entity::EntityKind;

/// Context §I's own per-kind table.
pub fn default_attribute_map(kind: EntityKind) -> AttributeMap {
    let mut map = AttributeMap::default();
    let (max_health, movement_speed, follow_range, attack_damage) = match kind {
        EntityKind::Zombie => (20.0, 0.23, 35.0, Some(3.0)),
        EntityKind::Villager => (20.0, 0.5, 16.0, None),
        EntityKind::Cow => (10.0, 0.2, 16.0, None),
        // Never a real Mob-rung kind; this row is inert data no system consumes.
        EntityKind::Item => (20.0, 0.7, 32.0, None),
    };
    map.insert(
        attribute::MAX_HEALTH,
        AttributeInstance::new(max_health, 1.0, 1024.0),
    );
    map.insert(
        attribute::MOVEMENT_SPEED,
        AttributeInstance::new(movement_speed, 0.0, 1024.0),
    );
    map.insert(
        attribute::FOLLOW_RANGE,
        AttributeInstance::new(follow_range, 0.0, 2048.0),
    );
    if let Some(damage) = attack_damage {
        map.insert(
            attribute::ATTACK_DAMAGE,
            AttributeInstance::new(damage, 0.0, 2048.0),
        );
    }
    map.insert(
        attribute::ATTACK_KNOCKBACK,
        AttributeInstance::new(0.0, 0.0, 5.0),
    );
    map.insert(
        attribute::KNOCKBACK_RESISTANCE,
        AttributeInstance::new(0.0, -2.0, 1.0),
    );
    map.insert(attribute::ARMOR, AttributeInstance::new(0.0, 0.0, 30.0));
    map.insert(
        attribute::ARMOR_TOUGHNESS,
        AttributeInstance::new(0.0, 0.0, 20.0),
    );
    map.insert(
        attribute::STEP_HEIGHT,
        AttributeInstance::new(0.6, 0.0, 10.0),
    );
    map.insert(
        attribute::JUMP_STRENGTH,
        AttributeInstance::new(0.42, 0.0, 32.0),
    );
    map
}

/// Context §J's own hand-typed, moderate-confidence dimension table.
pub fn entity_dimensions(kind: EntityKind) -> (f32, f32) {
    match kind {
        EntityKind::Zombie => (0.6, 1.95),
        EntityKind::Villager => (0.6, 1.95),
        EntityKind::Cow => (0.9, 1.4),
        EntityKind::Item => (0.25, 0.25),
    }
}

/// A deterministic, bounded stand-in for vanilla's own real per-region RNG-consuming
/// interval-goal chance rolls (Context §D/§J: no per-region RNG seam exists in this
/// blueprint's own dependencies) — used by `WaterAvoidingRandomStrollGoal`'s own
/// `1/denom`-per-tick chance.
fn pseudo_random_gate(tick_count: u64, entity_id: RcEntityId, denom: u64) -> bool {
    (tick_count.wrapping_mul(2654435761).wrapping_add(entity_id.0)) % denom == 0
}

/// A goal whose `can_use` is a fixed constant — covers every "declared for
/// priority-slot completeness only... can_use `false`, always" table row (Context §J:
/// `BreedGoal`/`TemptGoal`/`FollowParentGoal`), and every goal whose real `can_use`
/// condition needs a live candidate/target list this blueprint's own `AiContext` does
/// not carry (`ZombieAttackGoal`, `LookAtPlayerGoal`, `NearestAttackableTargetGoal`) —
/// a genuine, bounded infrastructure gap (no target-selector→goal-selector shared
/// target storage and no live player-candidate feed exist anywhere in this blueprint's
/// own Deliverables), restated in `docs/findings-for-planning.md`, not a shortcut
/// taken silently.
struct ConstantGoal {
    flags: u8,
    can_use: bool,
}
impl Goal for ConstantGoal {
    fn flags(&self) -> u8 {
        self.flags
    }
    fn can_use(&self, _ctx: &AiContext) -> bool {
        self.can_use
    }
}

/// `can_use` reads the bounded `hurt_by` seam every tier-2 kind's own `HurtByTargetGoal`
/// (Zombie target selector)/`PanicGoal` (Cow) shares (Context §J).
struct HurtByGoal {
    flags: u8,
}
impl Goal for HurtByGoal {
    fn flags(&self) -> u8 {
        self.flags
    }
    fn can_use(&self, ctx: &AiContext) -> bool {
        ctx.hurt_by.is_some()
    }
}

/// `WaterAvoidingRandomStrollGoal` (Context §J: "no current `WalkTarget`, `1/120`-
/// per-tick chance to start").
struct WaterAvoidingRandomStrollGoal;
impl Goal for WaterAvoidingRandomStrollGoal {
    fn flags(&self) -> u8 {
        FLAG_MOVE
    }
    fn can_use(&self, ctx: &AiContext) -> bool {
        ctx.navigation.current_path.is_none() && pseudo_random_gate(ctx.tick_count, ctx.self_id, 120)
    }
}

/// Context §J's own Zombie goal-selector table.
pub fn zombie_goal_selector() -> GoalSelector {
    let mut selector = GoalSelector::new();
    selector.add_goal(
        3,
        Box::new(ConstantGoal {
            flags: FLAG_MOVE | FLAG_LOOK,
            can_use: false,
        }),
    ); // ZombieAttackGoal
    selector.add_goal(7, Box::new(WaterAvoidingRandomStrollGoal));
    selector.add_goal(
        8,
        Box::new(ConstantGoal {
            flags: FLAG_LOOK,
            can_use: false,
        }),
    ); // LookAtPlayerGoal
    selector.add_goal(
        8,
        Box::new(ConstantGoal {
            flags: FLAG_MOVE | FLAG_LOOK,
            can_use: true,
        }),
    ); // RandomLookAroundGoal
    selector
}

/// Context §J's own Zombie target-selector table.
pub fn zombie_target_selector() -> GoalSelector {
    let mut selector = GoalSelector::new();
    selector.add_goal(1, Box::new(HurtByGoal { flags: FLAG_TARGET })); // HurtByTargetGoal
    selector.add_goal(
        2,
        Box::new(ConstantGoal {
            flags: FLAG_TARGET,
            can_use: false,
        }),
    ); // NearestAttackableTargetGoal<Player>
    selector
}

/// Context §J's own Cow goal-selector table (`target_selector` is `GoalSelector::new()`,
/// empty).
pub fn cow_goal_selector() -> GoalSelector {
    let mut selector = GoalSelector::new();
    selector.add_goal(
        0,
        Box::new(ConstantGoal {
            flags: FLAG_JUMP,
            can_use: true,
        }),
    ); // FloatGoal
    selector.add_goal(1, Box::new(HurtByGoal { flags: FLAG_MOVE })); // PanicGoal
    selector.add_goal(
        2,
        Box::new(ConstantGoal {
            flags: FLAG_MOVE | FLAG_LOOK,
            can_use: false,
        }),
    ); // BreedGoal
    selector.add_goal(
        3,
        Box::new(ConstantGoal {
            flags: FLAG_MOVE | FLAG_LOOK,
            can_use: false,
        }),
    ); // TemptGoal
    selector.add_goal(
        4,
        Box::new(ConstantGoal {
            flags: 0,
            can_use: false,
        }),
    ); // FollowParentGoal
    selector.add_goal(5, Box::new(WaterAvoidingRandomStrollGoal));
    selector.add_goal(
        6,
        Box::new(ConstantGoal {
            flags: FLAG_LOOK,
            can_use: false,
        }),
    ); // LookAtPlayerGoal
    selector.add_goal(
        7,
        Box::new(ConstantGoal {
            flags: FLAG_MOVE | FLAG_LOOK,
            can_use: true,
        }),
    ); // RandomLookAroundGoal
    selector
}

/// `SwimBehavior` (Context §J's own Core package) — floats up while submerged.
struct SwimBehavior;
impl Behavior for SwimBehavior {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[]
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, ctx: &mut AiContext) {
        use crate::ai::pathfinding::node::{tier1_path_type_table, PathType};
        let pos = rc_core::BlockPos::new(
            ctx.self_pos[0].floor() as i32,
            ctx.self_pos[1].floor() as i32,
            ctx.self_pos[2].floor() as i32,
        );
        if tier1_path_type_table().classify(ctx.world, pos) == PathType::Water {
            ctx.movement_intent.0.jumping = true;
        }
    }
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

/// `LookAtTargetSink` (Context §J's own Core package) — drives `look_target` toward the
/// `LookTarget` memory when present.
struct LookAtTargetSink;
impl Behavior for LookAtTargetSink {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[(MemoryModuleType::LookTarget, MemoryStatus::ValuePresent)]
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, ctx: &mut AiContext) {
        if let Some(brain) = ctx.memory {
            if let Some(&target) = brain.get::<[f64; 3]>(MemoryModuleType::LookTarget) {
                *ctx.look_target = Some(target);
            }
        }
    }
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

/// `VillagerPanicTrigger` (Context §J/§E's own Core package) — declared for framework
/// completeness/behavior-registration symmetry only; the real push mechanism is
/// `BrainProgram.panic_trigger_memory`, evaluated as `tick`'s own dedicated pre-phase-3
/// step (Context §E). This entry's own `start`/`tick`/`stop` bodies are therefore
/// no-ops, never actually driving the transition themselves.
struct VillagerPanicTrigger;
impl Behavior for VillagerPanicTrigger {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[]
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, _ctx: &mut AiContext) {}
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

/// `WalkToRandomPoiOrStroll` (Context §J's own Idle package) — this blueprint's own
/// reduced stand-in for vanilla's real village-bound stroll (no POI/village-bounds
/// system exists).
struct WalkToRandomPoiOrStroll;
impl Behavior for WalkToRandomPoiOrStroll {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[]
    }
    fn check_extra_start_conditions(&self, ctx: &AiContext) -> bool {
        pseudo_random_gate(ctx.tick_count, ctx.self_id, 120)
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, _ctx: &mut AiContext) {}
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

/// `InteractWithNearestVillager` (Context §J's own Idle package) — declared,
/// `check_extra_start_conditions` returns `false` (no second villager modeled in this
/// blueprint's own test fixtures; framework-ready, inert at M4 scope).
struct InteractWithNearestVillager;
impl Behavior for InteractWithNearestVillager {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[]
    }
    fn check_extra_start_conditions(&self, _ctx: &AiContext) -> bool {
        false
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, _ctx: &mut AiContext) {}
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

/// `VillagerCalmDown` (Context §J's own Panic package) — declared-but-inert (vanilla's
/// own real behavior name); it drives nothing itself, `select_activity`'s own general,
/// unconditional call recovers a Villager from `Panic` instead (Context §E).
struct VillagerCalmDown;
impl Behavior for VillagerCalmDown {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[]
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, _ctx: &mut AiContext) {}
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

/// `FleeFromHostile` (Context §J's own Panic package) — models only the `HurtByEntity`
/// flee behavior (this design carries no `NearestHostile`-equivalent memory). No
/// position is associated with the `HurtByEntity` memory's own `RcEntityId` value
/// anywhere in this blueprint's own `AiContext` (a bounded gap, restated in
/// `docs/findings-for-planning.md`), so this behavior's own `tick` gates correctly on
/// the real memory requirement but drives no real movement yet.
struct FleeFromHostile;
impl Behavior for FleeFromHostile {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[(MemoryModuleType::HurtByEntity, MemoryStatus::ValuePresent)]
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, _ctx: &mut AiContext) {}
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

/// `PlayerSensor` (Context §J) — vanilla's own real body needs a live player-candidate
/// feed no `Sensor::tick(&self, ctx: &AiContext, brain: &mut Brain)` call receives
/// anywhere in this blueprint's own Deliverables (identical bounded gap to
/// `NearestAttackableTargetGoal`'s own, restated in `docs/findings-for-planning.md`) —
/// declared, empty `tick`, alongside the seven other declared-inert sensors below.
struct PlayerSensor;
impl Sensor for PlayerSensor {
    fn requires(&self) -> &'static [MemoryModuleType] {
        &[]
    }
    fn tick(&self, _ctx: &AiContext, _brain: &mut Brain) {}
}

/// `HurtBySensor` (Context §J) — writes `HurtByEntity` only, from the same bounded
/// `hurt_by` seam Zombie/Cow use.
struct HurtBySensor;
impl Sensor for HurtBySensor {
    fn requires(&self) -> &'static [MemoryModuleType] {
        &[]
    }
    fn tick(&self, ctx: &AiContext, brain: &mut Brain) {
        if let Some(id) = ctx.hurt_by {
            brain.set(MemoryModuleType::HurtByEntity, id, None);
        }
    }
}

/// The remaining 7 vanilla sensor types (`NearestLivingEntitySensor`,
/// `VillagerHostilesSensor`, `SecondaryPoiSensor`, `GolemSensor`, `NearestBedSensor`,
/// `VillagerBabiesSensor`, `NearestItemSensor`) — declared, empty `tick`, documented
/// as inactive at M4 scope (Context §J).
struct InertSensor;
impl Sensor for InertSensor {
    fn requires(&self) -> &'static [MemoryModuleType] {
        &[]
    }
    fn tick(&self, _ctx: &AiContext, _brain: &mut Brain) {}
}

/// Context §J's own Villager `BrainProgram` (3 active + 3 declared-inert activities, 2
/// "real" sensors + 7 inert ones).
pub fn villager_brain_program() -> BrainProgram {
    let core = ActivityPackage {
        activity: Activity::Core,
        requirements: vec![],
        behaviors: vec![
            (0, Box::new(SwimBehavior) as Box<dyn Behavior>),
            (1, Box::new(LookAtTargetSink)),
            (2, Box::new(VillagerPanicTrigger)),
        ],
        erase_on_stop: vec![],
    };
    let idle = ActivityPackage {
        activity: Activity::Idle,
        requirements: vec![],
        behaviors: vec![
            (0, Box::new(WalkToRandomPoiOrStroll) as Box<dyn Behavior>),
            (1, Box::new(InteractWithNearestVillager)),
        ],
        erase_on_stop: vec![MemoryModuleType::WalkTarget],
    };
    let work = ActivityPackage {
        activity: Activity::Work,
        requirements: vec![ActivityRequirement {
            memory: MemoryModuleType::JobSite,
            status: MemoryStatus::ValuePresent,
        }],
        behaviors: vec![],
        erase_on_stop: vec![],
    };
    let meet = ActivityPackage {
        activity: Activity::Meet,
        requirements: vec![ActivityRequirement {
            memory: MemoryModuleType::MeetingPoint,
            status: MemoryStatus::ValuePresent,
        }],
        behaviors: vec![],
        erase_on_stop: vec![],
    };
    let rest = ActivityPackage {
        activity: Activity::Rest,
        requirements: vec![],
        behaviors: vec![],
        erase_on_stop: vec![],
    };
    let panic = ActivityPackage {
        activity: Activity::Panic,
        requirements: vec![],
        behaviors: vec![
            (0, Box::new(VillagerCalmDown) as Box<dyn Behavior>),
            (1, Box::new(FleeFromHostile)),
        ],
        erase_on_stop: vec![],
    };

    BrainProgram {
        sensors: vec![
            Box::new(PlayerSensor),
            Box::new(HurtBySensor),
            Box::new(InertSensor), // NearestLivingEntitySensor
            Box::new(InertSensor), // VillagerHostilesSensor
            Box::new(InertSensor), // SecondaryPoiSensor
            Box::new(InertSensor), // GolemSensor
            Box::new(InertSensor), // NearestBedSensor
            Box::new(InertSensor), // VillagerBabiesSensor
            Box::new(InertSensor), // NearestItemSensor
        ],
        packages: vec![core, idle, work, meet, rest, panic],
        schedule_candidates: vec![Activity::Work, Activity::Meet, Activity::Idle],
        panic_trigger_memory: Some(MemoryModuleType::HurtBy),
        schedule_update_delay_ticks: 20,
    }
}

/// Every field a future spawning blueprint needs to attach this blueprint's own AI
/// substrate to one freshly-spawned entity — a plain data bag, not a `bevy_ecs::Bundle`
/// (a Brain-driven kind and a GoalSelector-driven kind need different component sets,
/// so a single static `#[derive(Bundle)]` cannot represent both — Context §K).
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
    let (goal_selector, target_selector, brain) = match kind {
        EntityKind::Zombie => (
            Some(zombie_goal_selector()),
            Some(zombie_target_selector()),
            None,
        ),
        EntityKind::Cow => (Some(cow_goal_selector()), Some(GoalSelector::new()), None),
        EntityKind::Villager => (
            None,
            None,
            Some((Brain::new([Activity::Core]), villager_brain_program())),
        ),
        EntityKind::Item => (None, None, None),
    };
    MobAiLoadout {
        attributes: default_attribute_map(kind),
        sensing: crate::ai::sensing::Sensing::default(),
        navigation: crate::ai::navigation::PathNavigation::default(),
        movement_intent: crate::ai::navigation::PendingMovementIntent::default(),
        goal_selector,
        target_selector,
        brain,
    }
}
