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
use crate::ai::goal::{
    AiContext, FLAG_JUMP, FLAG_LOOK, FLAG_MOVE, FLAG_TARGET, Goal, GoalSelector,
};
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
    // M4-B09 Context Part B: four registry rows M4-B05's own combat-only `AttributeKind`
    // table needed and this table did not yet declare, added here so the one, registry-
    // keyed `AttributeMap` (this module's own) stays complete for every combat-relevant
    // attribute — every value copied verbatim from Part B's own table (TEST-D57 CONFIRMED,
    // `M4-B09-CLAIMS.md`). Consumed by M4-B05's own formulas only through the still-separate
    // `combat::attributes::AttributeMap` (Part B's own governance changeset keeps that type
    // alive rather than retiring it -- final report has the full citation), so these four
    // rows are registry-completeness content here, not yet read by any production system.
    map.insert(
        attribute::ATTACK_SPEED,
        AttributeInstance::new(4.0, 0.0, 1024.0),
    );
    map.insert(
        attribute::SAFE_FALL_DISTANCE,
        AttributeInstance::new(3.0, -1024.0, 1024.0),
    );
    map.insert(
        attribute::FALL_DAMAGE_MULTIPLIER,
        AttributeInstance::new(1.0, 0.0, 100.0),
    );
    map.insert(
        attribute::SWEEPING_DAMAGE_RATIO,
        AttributeInstance::new(0.0, 0.0, 1.0),
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
    (tick_count
        .wrapping_mul(2654435761)
        .wrapping_add(entity_id.0))
    .is_multiple_of(denom)
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

/// M4-B09 Context Part C.4: the melee-adjacency constant `ZombieAttackGoal::can_use`/
/// `tick` reads. Vanilla's real per-mob attack reach varies by hitbox and is not pinned
/// by any merged blueprint — this blueprint's own moderate-confidence value, flagged for
/// reconciliation (Context Part J).
pub const MELEE_ATTACK_RANGE: f64 = 1.5;
/// M4-B09 Context Part C.4: `HurtBySensor`'s own Villager `HurtBy`-memory expiry — no
/// merged blueprint pins vanilla's real value; flagged for reconciliation (Context Part J).
pub const HURT_BY_MEMORY_TTL_TICKS: u32 = 100;

/// `HurtByTargetGoal` (M4-B09 Context Part C.4, citing M4-B03's own `can_use` condition
/// verbatim): the Zombie target-selector row whose `can_use` reads the bounded `hurt_by`
/// seam directly, claiming `FLAG_TARGET` whenever `ctx.hurt_by.is_some()`. `start` needs
/// no body of its own — the adapter (`ScenarioWorld`/a future production Stage-6a
/// composition root) is what actually reads "which goal now holds `FLAG_TARGET`" and
/// derives `current_target`/`current_target_pos` from it (Context Part D's own
/// `running_goal_holding` introspection); this goal's own job is only to *claim* the flag
/// slot, overriding ordinary range-based targeting via `GoalSelector`'s own priority
/// eviction (this Goal's priority `1` beats `NearestAttackableTargetGoal`'s `2`,
/// `zombie_target_selector` below).
pub struct HurtByTargetGoal;
impl Goal for HurtByTargetGoal {
    fn flags(&self) -> u8 {
        FLAG_TARGET
    }
    fn can_use(&self, ctx: &AiContext) -> bool {
        ctx.hurt_by.is_some()
    }
}

/// `ZombieAttackGoal` (M4-B09 Context Part C.4, concrete body): claims `FLAG_MOVE|
/// FLAG_LOOK` and sets `ctx.melee_attack_signal` (backing `combat::PendingMeleeAttack.0`
/// directly) whenever `ctx.current_target` is within `MELEE_ATTACK_RANGE` — the 3D
/// Euclidean distance to `current_target`'s own adapter-supplied `current_target_pos`
/// (Context §D's "horizontal+vertical distance," i.e. full 3D, not a horizontal-only
/// check).
pub struct ZombieAttackGoal;
impl Goal for ZombieAttackGoal {
    fn flags(&self) -> u8 {
        FLAG_MOVE | FLAG_LOOK
    }
    fn can_use(&self, ctx: &AiContext) -> bool {
        within_melee_range(ctx)
    }
    fn can_continue_to_use(&self, ctx: &AiContext) -> bool {
        within_melee_range(ctx)
    }
    fn tick(&mut self, ctx: &mut AiContext) {
        *ctx.melee_attack_signal = if within_melee_range(ctx) {
            ctx.current_target
        } else {
            None
        };
    }
}

fn within_melee_range(ctx: &AiContext) -> bool {
    let (Some(_), Some(target_pos)) = (ctx.current_target, ctx.current_target_pos) else {
        return false;
    };
    let dx = target_pos[0] - ctx.self_pos[0];
    let dy = target_pos[1] - ctx.self_pos[1];
    let dz = target_pos[2] - ctx.self_pos[2];
    (dx * dx + dy * dy + dz * dz).sqrt() <= MELEE_ATTACK_RANGE
}

/// `PanicGoal` (Cow, M4-B09 Context Part C.4): `can_use: ctx.hurt_by.is_some()`, flees at
/// a `2.0` navigation-speed modifier away from `hurt_by`'s own last-known position
/// (`ctx.current_target_pos`, the adapter's own hurt_by-derived position — Context Part
/// C.3's own additive field). `2.0` is the Cow's own panic speed modifier (TEST-D57
/// CONFIRMED, `M4-B09-CLAIMS.md`) — this blueprint's own navigation-execution layer
/// (`MoveControl`) carries no per-instance speed-modifier parameter to thread that number
/// through structurally (Context §G's own already-cited bounded gap), so this Goal's own
/// `tick` sets a full-speed (`forward = 1.0`) flee heading directly, the same bounded
/// simplification `ScenarioWorld`'s own movement-realization step already applies
/// (Context Part D) — `2.0` is restated here as this Goal's own documented intent, not a
/// literal multiplier this engine's movement model can yet apply.
pub struct PanicGoal;
impl Goal for PanicGoal {
    fn flags(&self) -> u8 {
        FLAG_MOVE
    }
    fn can_use(&self, ctx: &AiContext) -> bool {
        ctx.hurt_by.is_some()
    }
    fn tick(&mut self, ctx: &mut AiContext) {
        flee_from_current_target(ctx);
    }
}

/// Shared flee-heading computation (`PanicGoal`/`FleeFromHostile`, Context Part C.4):
/// points `movement_intent`'s own `yaw_degrees` directly away from `ctx.current_target_pos`
/// (the same `atan2`-based convention `MoveControl`/`LookControl` already use, Context §G)
/// and drives `forward = 1.0`. A no-op when the adapter has no position to flee from this
/// tick (Context Part C.3's own field doc comment: `current_target_pos` is a one-tick
/// pulse, not sticky).
fn flee_from_current_target(ctx: &mut AiContext) {
    let Some(target_pos) = ctx.current_target_pos else {
        return;
    };
    let dx = ctx.self_pos[0] - target_pos[0];
    let dz = ctx.self_pos[2] - target_pos[2];
    if dx == 0.0 && dz == 0.0 {
        return;
    }
    let yaw = (dz.atan2(dx)).to_degrees() as f32 - 90.0;
    ctx.movement_intent.0.forward = 1.0;
    ctx.movement_intent.0.yaw_degrees = yaw;
}

/// `WaterAvoidingRandomStrollGoal` (Context §J: "no current `WalkTarget`, `1/120`-
/// per-tick chance to start").
struct WaterAvoidingRandomStrollGoal;
impl Goal for WaterAvoidingRandomStrollGoal {
    fn flags(&self) -> u8 {
        FLAG_MOVE
    }
    fn can_use(&self, ctx: &AiContext) -> bool {
        ctx.navigation.current_path.is_none()
            && pseudo_random_gate(ctx.tick_count, ctx.self_id, 120)
    }
}

/// Context §J's own Zombie goal-selector table.
pub fn zombie_goal_selector() -> GoalSelector {
    let mut selector = GoalSelector::new();
    selector.add_goal(3, Box::new(ZombieAttackGoal));
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
    selector.add_goal(1, Box::new(HurtByTargetGoal));
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
    selector.add_goal(1, Box::new(PanicGoal));
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
        use crate::ai::pathfinding::node::{PathType, tier1_path_type_table};
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
        if let Some(brain) = ctx.memory
            && let Some(&target) = brain.get::<[f64; 3]>(MemoryModuleType::LookTarget)
        {
            *ctx.look_target = Some(target);
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
/// flee behavior (this design carries no `NearestHostile`-equivalent memory). M4-B09
/// Context Part C.4/C.3: `tick` now drives real movement via `flee_from_current_target`
/// (the same shared heading computation `PanicGoal` uses) whenever `ctx.current_target_pos`
/// happens to carry the attacker's position this tick (a one-tick pulse, Context Part
/// C.3's own field doc comment — this Behavior's own gating memory, `HurtByEntity`, persists
/// for `HURT_BY_MEMORY_TTL_TICKS` ticks via `HurtBySensor`'s own TTL, longer than the one
/// tick `current_target_pos` itself stays populated; a no-op on every tick after the first
/// is the honest, bounded consequence, restated in the final report).
struct FleeFromHostile;
impl Behavior for FleeFromHostile {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        &[(MemoryModuleType::HurtByEntity, MemoryStatus::ValuePresent)]
    }
    fn start(&mut self, _ctx: &mut AiContext) {}
    fn tick(&mut self, ctx: &mut AiContext) {
        flee_from_current_target(ctx);
    }
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

/// `HurtBySensor` (Context §J, TTL added by M4-B09 Context Part C.4) — writes
/// `HurtByEntity` only, from the same bounded `hurt_by` seam Zombie/Cow use, with an
/// explicit `HURT_BY_MEMORY_TTL_TICKS`-tick expiry standing in for vanilla's own
/// damage-source-driven, TTL-argument-free expiry (M4-B09-CLAIMS.md's own corrected
/// row: vanilla's `HurtBySensor` sets no TTL, expiring instead when `getLastDamageSource`
/// returns null or the attacker dies/changes level — this engine's bounded seam has
/// neither signal, so a fixed TTL is this blueprint's own concrete substitute).
struct HurtBySensor;
impl Sensor for HurtBySensor {
    fn requires(&self) -> &'static [MemoryModuleType] {
        &[]
    }
    fn tick(&self, ctx: &AiContext, brain: &mut Brain) {
        if let Some(id) = ctx.hurt_by {
            brain.set(
                MemoryModuleType::HurtByEntity,
                id,
                Some(HURT_BY_MEMORY_TTL_TICKS),
            );
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
        // M4-B09 governance fix: `HurtBySensor` (above) only ever sets `HurtByEntity`, never
        // `HurtBy` (this engine's bounded `hurt_by` seam carries only a resolvable attacker
        // id, never a full damage-source value — Context Part C.2/C.4's own citation) — the
        // landed `Some(MemoryModuleType::HurtBy)` here could never actually fire, since no
        // sensor this crate ships ever writes that key. Corrected to the memory the sensor
        // really writes.
        panic_trigger_memory: Some(MemoryModuleType::HurtByEntity),
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
