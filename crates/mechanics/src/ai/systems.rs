//! Stage-6a system registration glue (M4-B03 blueprint Context §K): four systems
//! registered into `DomainGroup::EntityAiSelection`, none of which ever uses
//! `Commands` — MECH-D32's "Stage 6a never mutates authoritative World state" rule,
//! made structural.
//!
//! **Code-vs-blueprint mismatch (recorded in `docs/findings-for-planning.md`):**
//! Context §K's own query shape names `&BaseEntity`/`&LivingEntity` as two of this
//! blueprint's own Stage-6a query terms, but neither type derives `bevy_ecs::
//! Component` in the merged M4-B01 code (only `EntityNbtFields`/`EntityMetadataFields`
//! — M4-B01's own Context explicitly states it "never attached any of its own
//! component structs to a live `bevy_ecs::World`"). Adding that derive is M4-B01's own
//! deliverable, not this blueprint's, so the four systems below query only the six
//! AI-owned component types this blueprint itself defines and read a placeholder
//! `self_pos`/`self_rotation` — restated below at each system's own doc comment.
//!
//! This blueprint's own Goal & Done definition is explicit that wiring any of this
//! into `HardcodedWorld`'s live tick loop, or connecting a real `BlockWorldAccess`
//! view onto live chunk data, is a future composition-root blueprint's job (mirroring
//! `crate::stage4::ecs::EcsBlockWorld`'s own established `Query`-backed pattern, which
//! this blueprint deliberately does not reuse yet — no acceptance test here spawns a
//! real, chunk-backed region). The four systems below are therefore real, correctly
//! access-set-declared `bevy_ecs` systems (proven by `ai_stage_registration.rs`'s own
//! `RcExecutorBuilder::build` success), but read world blocks through `NullBlockWorld`
//! (below) — a `BlockWorldAccess` stand-in that reports every position as unloaded —
//! until that future blueprint swaps in a real `EcsBlockWorld`-style adapter.

use bevy_ecs::prelude::*;

use rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId};
use rc_messaging::{Address, RegionId};
use rc_registries::generated_v776::registries::attribute;

use crate::ai::attributes::AttributeMap;
use crate::ai::brain::{Brain, BrainProgram};
use crate::ai::goal::GoalSelector;
use crate::ai::goal::{AiContext, should_full_tick};
use crate::ai::navigation::PathNavigation;
use crate::ai::navigation::PendingMovementIntent;
use crate::ai::navigation::{MoveControl, MoveControlOperation};
use crate::ai::pathfinding::node::WalkNodeEvaluator;
use crate::ai::sensing::Sensing;
use crate::entity::EntityKind;
use crate::world_access::BlockWorldAccess;

/// Thin `Component` wrapper around `crate::ai::goal::GoalSelector` (behavior goals).
#[derive(Component)]
pub struct GoalSelectorComponent(pub GoalSelector);
/// Thin `Component` wrapper around `crate::ai::goal::GoalSelector` (attack/interaction
/// target-only goals).
#[derive(Component)]
pub struct TargetSelectorComponent(pub GoalSelector);
/// Thin `Component` wrapper around `(Brain, BrainProgram)`.
#[derive(Component)]
pub struct BrainComponent(pub Brain, pub BrainProgram);

/// A `bevy_ecs::Resource` carrying the current region tick counter — read by the
/// half-tick throttle (Context §D) and the Brain schedule-update throttle (Context §E).
/// A future blueprint's own real per-region tick counter resource supersedes this one at
/// composition-root wiring time; this blueprint's own tests construct it directly.
#[derive(Resource, Copy, Clone, Debug, Default)]
pub struct AiTickCounter(pub u64);

/// Reports every position as unloaded (`None`) — this module's own placeholder
/// `BlockWorldAccess`, restated at the top of this file's own doc comment.
struct NullBlockWorld;
impl BlockWorldAccess for NullBlockWorld {
    fn get_block(&self, _pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        None
    }
    fn set_block(&mut self, _pos: BlockPos, _state: rc_chunk_storage::BlockStateId) -> bool {
        false
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        Address::Region(RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(RegionId(0))
    }
}

/// Registers every `Component`/`Resource` type this blueprint's own four Stage-6a
/// systems query, against a fresh `World` -- the `RcExecutorBuilder::new` bootstrap
/// function a caller (this blueprint's own acceptance tests, and eventually a future
/// composition-root blueprint) passes so `RcExecutorBuilder`'s prototype `World` and
/// every real region's own `World` register these components in the identical order
/// (Context §K / `rc-scheduler`'s own "`ComponentId` consistency across regions"
/// invariant).
pub fn ai_bootstrap(world: &mut World) {
    world.register_component::<GoalSelectorComponent>();
    world.register_component::<TargetSelectorComponent>();
    world.register_component::<BrainComponent>();
    world.register_component::<AttributeMap>();
    world.register_component::<Sensing>();
    world.register_component::<PathNavigation>();
    world.register_component::<PendingMovementIntent>();
    world.insert_resource(AiTickCounter::default());
}

/// A placeholder `self_id`/`self_pos`/`self_rotation` (Context: no stable per-entity
/// `RcEntityId`/live transform component is queryable from a `bevy_ecs::World` at M4
/// scope, restated at this file's own top doc comment) derived from the `Entity`
/// handle `bevy_ecs` itself already assigns.
fn placeholder_self_id(entity: Entity) -> RcEntityId {
    RcEntityId(entity.index_u32() as u64)
}

/// `sensing_tick_system` — clears + repopulates `Sensing` for every entity (Context
/// §H: cleared every tick, before any `has_line_of_sight` call that tick — this
/// blueprint's own systems never call it themselves, so clearing is this system's
/// entire job).
pub fn sensing_tick_system(mut query: Query<&mut Sensing>) {
    for mut sensing in query.iter_mut() {
        sensing.clear();
    }
}

/// `goal_selector_tick_system` — `GoalSelectorComponent`/`TargetSelectorComponent`
/// (Context §D/§K).
#[allow(clippy::type_complexity)]
pub fn goal_selector_tick_system(
    tick: Res<AiTickCounter>,
    mut query: Query<(
        Entity,
        Option<&mut GoalSelectorComponent>,
        Option<&mut TargetSelectorComponent>,
        &mut AttributeMap,
        &mut Sensing,
        &mut PathNavigation,
        &mut PendingMovementIntent,
    )>,
) {
    let world = NullBlockWorld;
    for (
        entity,
        goal_selector,
        target_selector,
        attributes,
        sensing,
        mut navigation,
        mut movement_intent,
    ) in query.iter_mut()
    {
        if goal_selector.is_none() && target_selector.is_none() {
            continue;
        }
        let self_id = placeholder_self_id(entity);
        let full_tick = should_full_tick(tick.0, self_id);
        let mut look_target: Option<[f64; 3]> = None;
        let mut ctx = AiContext {
            self_id,
            self_pos: [0.0, 0.0, 0.0],
            self_rotation: [0.0, 0.0],
            self_kind: EntityKind::Zombie,
            attributes: &attributes,
            sensing: &sensing,
            memory: None,
            world: &world,
            tick_count: tick.0,
            navigation: &mut navigation,
            movement_intent: &mut movement_intent,
            look_target: &mut look_target,
            hurt_by: None,
        };
        if let Some(mut goal_selector) = goal_selector {
            goal_selector.0.tick(&mut ctx, full_tick);
        }
        if let Some(mut target_selector) = target_selector {
            target_selector.0.tick(&mut ctx, full_tick);
        }
    }
}

/// `brain_tick_system` — `BrainComponent` (Context §E/§K).
#[allow(clippy::type_complexity)]
pub fn brain_tick_system(
    tick: Res<AiTickCounter>,
    mut query: Query<(
        Entity,
        &mut BrainComponent,
        &mut AttributeMap,
        &mut Sensing,
        &mut PathNavigation,
        &mut PendingMovementIntent,
    )>,
) {
    let world = NullBlockWorld;
    for (entity, mut brain_component, attributes, sensing, mut navigation, mut movement_intent) in
        query.iter_mut()
    {
        let self_id = placeholder_self_id(entity);
        let mut look_target: Option<[f64; 3]> = None;
        let mut ctx = AiContext {
            self_id,
            self_pos: [0.0, 0.0, 0.0],
            self_rotation: [0.0, 0.0],
            self_kind: EntityKind::Villager,
            attributes: &attributes,
            sensing: &sensing,
            memory: None,
            world: &world,
            tick_count: tick.0,
            navigation: &mut navigation,
            movement_intent: &mut movement_intent,
            look_target: &mut look_target,
            hurt_by: None,
        };
        let mut rng = deterministic_rng(tick.0, self_id);
        let BrainComponent(ref mut brain, ref mut program) = *brain_component;
        program.tick(&mut ctx, brain, tick.0, &mut rng);
        program.select_activity(brain, tick.0);
    }
}

/// A deterministic, bounded stand-in for a real per-region RNG stream (Context §E: no
/// such seam exists in this blueprint's own dependency set) — used only for a
/// `Behavior`'s own randomized `min..=max` duration roll.
fn deterministic_rng(tick_count: u64, entity_id: RcEntityId) -> impl FnMut() -> u32 {
    let mut state = tick_count
        .wrapping_mul(2654435761)
        .wrapping_add(entity_id.0);
    move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as u32
    }
}

/// `navigation_and_movement_intent_system` — `PathNavigation` tick, `MoveControl`/
/// `LookControl`/`JumpControl`, writes `PendingMovementIntent` (Context §G/§K).
pub fn navigation_and_movement_intent_system(
    mut query: Query<(
        &mut AttributeMap,
        &mut PathNavigation,
        &mut PendingMovementIntent,
    )>,
) {
    let world = NullBlockWorld;
    let evaluator = WalkNodeEvaluator;
    // No live transform component is queryable at M4 scope (this file's own top doc
    // comment) — every entity is navigated relative to the world origin until a
    // future composition-root blueprint supplies a real position.
    let entity_pos = [0.0, 0.0, 0.0];

    for (mut attributes, mut navigation, mut movement_intent) in query.iter_mut() {
        let movement_speed = attributes.value_or(attribute::MOVEMENT_SPEED, 0.7);
        let step_height = attributes.value_or(attribute::STEP_HEIGHT, 0.6);
        let follow_range = attributes.value_or(attribute::FOLLOW_RANGE, 32.0);
        let max_visited_nodes = (follow_range * 16.0).floor().max(0.0) as u32;

        let goal_pos = navigation
            .current_path
            .as_ref()
            .and_then(|p| p.nodes().last().copied());

        navigation.tick(
            entity_pos,
            goal_pos,
            movement_speed,
            &evaluator,
            &world,
            1.95,
            max_visited_nodes,
        );

        let wanted_pos = navigation
            .current_path
            .as_mut()
            .map(|p| {
                p.advance_if_reached(entity_pos);
                p.current_target()
            })
            .and_then(|t| t)
            .map(|p| [p.x as f64 + 0.5, p.y as f64, p.z as f64 + 0.5])
            .unwrap_or(entity_pos);

        let mut move_control = MoveControl {
            operation: if wanted_pos == entity_pos {
                MoveControlOperation::Wait
            } else {
                MoveControlOperation::MoveTo
            },
            wanted_pos,
            speed_modifier: 1.0,
        };
        let (forward, new_yaw, jumping) =
            move_control.tick(entity_pos, 0.0, true, step_height, 0.6);

        movement_intent.0 = rc_physics::MovementIntent {
            strafe: 0.0,
            forward,
            yaw_degrees: new_yaw,
            sprinting: false,
            sneaking: false,
            jumping,
            jump_boost_amplifier: 0,
        };
    }
}

/// Registers all four systems into `DomainGroup::EntityAiSelection` with
/// `structural_writes: vec![]` (Context §K). Never called by this blueprint's own
/// production code path — a future composition-root blueprint calls this against its
/// own real `RcExecutorBuilder`; this blueprint's own acceptance tests call it against a
/// throwaway builder/`World` directly.
pub fn register_ai_systems(builder: &mut rc_scheduler::RcExecutorBuilder) {
    builder.register_system(
        rc_scheduler::DomainGroup::EntityAiSelection,
        Box::new(|| {
            Box::new(IntoSystem::into_system(sensing_tick_system))
                as Box<dyn System<In = (), Out = ()>>
        }),
        vec![],
    );
    builder.register_system(
        rc_scheduler::DomainGroup::EntityAiSelection,
        Box::new(|| {
            Box::new(IntoSystem::into_system(goal_selector_tick_system))
                as Box<dyn System<In = (), Out = ()>>
        }),
        vec![],
    );
    builder.register_system(
        rc_scheduler::DomainGroup::EntityAiSelection,
        Box::new(|| {
            Box::new(IntoSystem::into_system(brain_tick_system))
                as Box<dyn System<In = (), Out = ()>>
        }),
        vec![],
    );
    builder.register_system(
        rc_scheduler::DomainGroup::EntityAiSelection,
        Box::new(|| {
            Box::new(IntoSystem::into_system(
                navigation_and_movement_intent_system,
            )) as Box<dyn System<In = (), Out = ()>>
        }),
        vec![],
    );
}
