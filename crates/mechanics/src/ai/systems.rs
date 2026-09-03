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
use crate::ai::goal::{should_full_tick, AiContext};
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
        todo!()
    }
    fn set_block(&mut self, _pos: BlockPos, _state: rc_chunk_storage::BlockStateId) -> bool {
        todo!()
    }
    fn dimension(&self) -> DimensionId {
        todo!()
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        todo!()
    }
    fn local_identity(&self) -> Address {
        todo!()
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
    todo!()
}

/// A placeholder `self_id`/`self_pos`/`self_rotation` (Context: no stable per-entity
/// `RcEntityId`/live transform component is queryable from a `bevy_ecs::World` at M4
/// scope, restated at this file's own top doc comment) derived from the `Entity`
/// handle `bevy_ecs` itself already assigns.
fn placeholder_self_id(entity: Entity) -> RcEntityId {
    todo!()
}

/// `sensing_tick_system` — clears + repopulates `Sensing` for every entity.
pub fn sensing_tick_system(mut query: Query<&mut Sensing>) {
    todo!()
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
    todo!()
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
    todo!()
}

/// `navigation_and_movement_intent_system` — `PathNavigation` tick, `MoveControl`/
/// `LookControl`/`JumpControl`, writes `PendingMovementIntent` (Context §G/§K).
pub fn navigation_and_movement_intent_system(
    mut query: Query<(&mut AttributeMap, &mut PathNavigation, &mut PendingMovementIntent)>,
) {
    todo!()
}

/// Registers all four systems into `DomainGroup::EntityAiSelection` with
/// `structural_writes: vec![]` (Context §K). Never called by this blueprint's own
/// production code path — a future composition-root blueprint calls this against its
/// own real `RcExecutorBuilder`; this blueprint's own acceptance tests call it against a
/// throwaway builder/`World` directly.
pub fn register_ai_systems(builder: &mut rc_scheduler::RcExecutorBuilder) {
    todo!()
}
