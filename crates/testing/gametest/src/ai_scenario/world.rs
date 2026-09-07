//! `ScenarioWorld` — a lightweight, `bevy_ecs`-free replay world for Stage 6a/6b (M4-B09
//! Context Part D), mirroring M3-B07's own established `ReplayWorld` "pure core +
//! lightweight non-`bevy_ecs` replay world" pattern for redstone, applied here to
//! AI/combat.

use std::collections::HashMap;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, RcEntityId};
use rc_mechanics::ai::brain::{Brain, BrainProgram};
use rc_mechanics::ai::goal::AiContext;
use rc_mechanics::ai::mob_config;
use rc_mechanics::ai::navigation::PendingMovementIntent;
use rc_mechanics::ai::pathfinding::node::WalkNodeEvaluator;
use rc_mechanics::ai::sensing::{Sensing, nearest_within_range};
use rc_mechanics::ai::{AttributeMap, GoalSelector, PathNavigation};
use rc_mechanics::combat::{PendingMeleeAttack, RecentDamage};
use rc_mechanics::entity::{BaseEntity, EntityKind, EntityUuid, LivingEntity};
use rc_mechanics::world_access::BlockWorldAccess;
use rc_messaging::Address;
use rc_registries::generated_v776::registries::attribute;

/// Scenario-harness-local only (Context Part D's own "movement realization" — never a
/// vanilla speed claim, never read by any `rc-mechanics` production code). Re-exported
/// from `ai_scenario::mod` for the crate-level constant table.
pub const STEP_BLOCKS_PER_TICK: f64 = 0.2;

/// A rough, this-suite-only eye-height fraction of an entity's own full height (Context
/// Part D: no exact vanilla eye-height table is pinned anywhere in this project's merged
/// blueprints) — used only for this harness's own line-of-sight/range checks, never a
/// parity claim.
const EYE_HEIGHT_FRACTION: f64 = 0.85;

/// A trivial, `HashMap`-backed `BlockWorldAccess` — every block this suite's own scenarios
/// need (a wall, a pit, open air) fits a hand-populated map; no real chunk/Anvil storage
/// is exercised (M4's own roadmap boundary: "world content remains superflat filler until
/// M5" — restated). Unlisted positions default to air (`BlockStateId::AIR`). `owner_of`/
/// `local_identity` are both fixed to one local placeholder region — every scenario in
/// this suite is single-region by construction, no cross-region traffic is ever exercised
/// here (that is M4-B08's own, already-integrated, job — Context Part A).
pub struct ScenarioWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    pub mobs: HashMap<RcEntityId, ScenarioMob>,
    pub players: HashMap<RcEntityId, ScenarioPlayerProxy>,
    pub tick_count: u64,
    next_id: u64,
}

/// A minimal, generic target stand-in — deliberately *not* `PlayerMarker` (`rc-mechanics`
/// must never depend on `rusty-clanker-server`-only types, WS-D3 rule 2, restated) and
/// deliberately just an `(RcEntityId, position)` pair, matching `nearest_within_range`'s
/// own already-generic candidate-list signature (M4-B03) exactly — this scenario harness's
/// own concrete resolution of the "how does a Stage-6a adapter learn where players are"
/// question M4-B03's own text leaves unanswered (a real production adapter's own answer,
/// e.g. a shared resource a composition root populates, is out of this blueprint's scope
/// — restated, Constraints).
pub struct ScenarioPlayerProxy {
    pub id: RcEntityId,
    pub pos: [f64; 3],
}

pub struct ScenarioMob {
    pub id: RcEntityId,
    pub kind: EntityKind,
    pub base: BaseEntity,
    pub living: LivingEntity,
    pub attributes: AttributeMap,
    pub sensing: Sensing,
    pub navigation: PathNavigation,
    pub movement_intent: PendingMovementIntent,
    pub goal_selector: GoalSelector,
    pub target_selector: GoalSelector,
    /// This tick's `target_selector` output (Context Part D/C.3's own adapter rule,
    /// restated at `ScenarioWorld::tick`'s own doc comment).
    pub current_target: Option<RcEntityId>,
    pub brain: Option<(Brain, BrainProgram)>,
    pub pending_melee_attack: PendingMeleeAttack,
    pub recent_damage: RecentDamage,
}

impl Default for ScenarioWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioWorld {
    pub fn new() -> Self {
        ScenarioWorld {
            blocks: HashMap::new(),
            mobs: HashMap::new(),
            players: HashMap::new(),
            tick_count: 0,
            next_id: 1,
        }
    }

    /// Populates one block (this blueprint's own scenario-spec loader, `spec.rs`, calls
    /// this once per RON-declared block entry).
    pub fn set_block(&mut self, pos: BlockPos, state: BlockStateId) {
        self.blocks.insert(pos, state);
    }

    fn alloc_id(&mut self) -> RcEntityId {
        let id = RcEntityId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Spawns one tier-2 mob with a full, correctly-defaulted component set — reuses
    /// `rc_mechanics::ai::mob_config::ai_loadout_for` unmodified (the "union of M4-B03's
    /// AI components and M4-B05's combat components" helper this blueprint's own Part C
    /// closes at the tick-time signal path is closed here too, at spawn time, via the
    /// same already-shipped per-kind table). `pending_melee_attack`/`recent_damage` both
    /// default to `None` (Context Part C.1/C.2).
    pub fn spawn_mob(&mut self, kind: EntityKind, pos: [f64; 3]) -> RcEntityId {
        let id = self.alloc_id();
        let loadout = mob_config::ai_loadout_for(kind);
        let (width, height) = mob_config::entity_dimensions(kind);
        let base = BaseEntity {
            pos,
            velocity: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0],
            fall_distance: 0.0,
            fire_ticks: -1,
            status_flags: 0,
            air_ticks: 300,
            on_ground: true,
            invulnerable: false,
            portal_cooldown: 0,
            uuid: EntityUuid::new_random(),
            custom_name: None,
            custom_name_visible: false,
            silent: false,
            no_gravity: false,
            glowing: false,
            pose: rc_mechanics::entity::metadata::Pose::Standing,
            ticks_frozen: 0,
            has_visual_fire: false,
        };
        let mut attributes = loadout.attributes;
        let max_health = attributes.value_or(attribute::MAX_HEALTH, 20.0) as f32;
        let living = LivingEntity {
            hand_states: 0,
            health: max_health,
            arrow_count: 0,
            stinger_count: 0,
            sleeping_bed_pos: None,
            absorption: 0.0,
            hurt_time: 0,
            death_time: 0,
            is_dead: false,
        };
        let _ = (width, height); // read by `entity_dimensions` call sites elsewhere in this module
        let mob = ScenarioMob {
            id,
            kind,
            base,
            living,
            attributes,
            sensing: loadout.sensing,
            navigation: loadout.navigation,
            movement_intent: loadout.movement_intent,
            goal_selector: loadout.goal_selector.unwrap_or_default(),
            target_selector: loadout.target_selector.unwrap_or_default(),
            current_target: None,
            brain: loadout.brain,
            pending_melee_attack: PendingMeleeAttack::default(),
            recent_damage: RecentDamage::default(),
        };
        self.mobs.insert(id, mob);
        id
    }

    pub fn spawn_player_proxy(&mut self, pos: [f64; 3]) -> RcEntityId {
        let id = self.alloc_id();
        self.players.insert(id, ScenarioPlayerProxy { id, pos });
        id
    }

    /// Test/debug-only, mirrors every `debug_query_*` precedent in this project: current
    /// position of a live mob, `None` if despawned/unknown.
    pub fn mob_pos(&self, id: RcEntityId) -> Option<[f64; 3]> {
        self.mobs.get(&id).map(|m| m.base.pos)
    }

    fn entity_pos(&self, id: RcEntityId) -> Option<[f64; 3]> {
        self.mobs
            .get(&id)
            .map(|m| m.base.pos)
            .or_else(|| self.players.get(&id).map(|p| p.pos))
    }

    fn eye_pos_of(pos: [f64; 3], kind: EntityKind) -> [f64; 3] {
        let (_, height) = mob_config::entity_dimensions(kind);
        [pos[0], pos[1] + height as f64 * EYE_HEIGHT_FRACTION, pos[2]]
    }

    /// The "ordinary" nearest-in-`FOLLOW_RANGE`-with-line-of-sight target (Context Part
    /// D's own concrete resolution of "how does a Stage-6a adapter learn where players
    /// are"), reusing M4-B03's already-shipped `nearest_within_range`/
    /// `has_line_of_sight` pure functions directly — never a new gameplay mechanic
    /// (Constraints (e)), only adapter-level composition of already-existing primitives.
    /// Restricted to `EntityKind::Zombie` (the one tier-2 kind whose own target-selector
    /// table carries a `NearestAttackableTargetGoal` row at all, even inert — Cow's own
    /// target-selector is empty by design, M4-B03 Context §J).
    fn ordinary_target(&mut self, mob_id: RcEntityId) -> (Option<RcEntityId>, Option<[f64; 3]>) {
        let Some(mob) = self.mobs.get(&mob_id) else {
            return (None, None);
        };
        if mob.kind != EntityKind::Zombie {
            return (None, None);
        }
        let follow_range = self
            .mobs
            .get_mut(&mob_id)
            .expect("checked Some above")
            .attributes
            .value_or(attribute::FOLLOW_RANGE, 32.0);

        let self_pos = self.mobs[&mob_id].base.pos;
        let candidates: Vec<(RcEntityId, [f64; 3])> =
            self.players.values().map(|p| (p.id, p.pos)).collect();
        let Some(nearest_id) = nearest_within_range(self_pos, candidates, follow_range) else {
            return (None, None);
        };
        let Some(target_pos) = self.players.get(&nearest_id).map(|p| p.pos) else {
            return (None, None);
        };

        let self_eye = Self::eye_pos_of(self_pos, EntityKind::Zombie);
        // Players have no `EntityKind` of their own; this suite's own player-eye-height
        // approximation reuses the identical `EYE_HEIGHT_FRACTION` against a fixed 1.8
        // stand-in full height (vanilla's own real player height), since no proxy carries
        // a `mob_config::entity_dimensions` entry.
        let target_eye = [
            target_pos[0],
            target_pos[1] + 1.8 * EYE_HEIGHT_FRACTION,
            target_pos[2],
        ];

        // `has_line_of_sight` needs `&mut Sensing` (its own per-tick cache) and `&dyn
        // BlockWorldAccess` (`self`) simultaneously -- temporarily lifting `sensing` out
        // of `self.mobs` (rather than out of the whole `ScenarioMob`, which `navigation`/
        // `movement_intent` also live on and this function never touches) avoids
        // aliasing `self` against itself.
        let mut sensing = std::mem::take(&mut self.mobs.get_mut(&mob_id).unwrap().sensing);
        let visible = sensing.has_line_of_sight(self_eye, nearest_id, target_eye, self);
        self.mobs.get_mut(&mob_id).unwrap().sensing = sensing;
        if visible {
            (Some(nearest_id), Some(target_pos))
        } else {
            (None, None)
        }
    }

    /// One full simulated tick (Context Part D's own fixed ordering, restated): (1)
    /// sensing, (2a) `target_selector.tick` then (2b) `goal_selector.tick`, (3)
    /// navigation, (4) `brain`, (5) movement-intent application, then this blueprint's
    /// own bounded, explicitly-approximate position-integration step.
    ///
    /// **Adapter rule for `current_target`/`current_target_pos`** (Context Part C.3's own
    /// field doc comments, restated concretely here): whenever this mob's own
    /// `recent_damage` pulse is `Some` this tick, `current_target`/`current_target_pos`
    /// are set from it directly (the attacker's own id/last-known position), regardless
    /// of the mob's own kind — `HurtByTargetGoal`'s job in `target_selector.tick` is only
    /// to *claim* `FLAG_TARGET` (observable, but not itself queried here); this adapter
    /// derives the actual target value from the same `hurt_by` input the Goal itself
    /// reads, which is simpler and equivalent since `HurtByTargetGoal` is this mob's own
    /// target_selector's only ever-`can_use`-true entry (M4-B03's own already-shipped
    /// `NearestAttackableTargetGoal` placeholder never fires, Context Part J). Otherwise,
    /// `ordinary_target` supplies it (Zombie only).
    pub fn tick(&mut self) {
        self.tick_count += 1;
        let tick_count = self.tick_count;
        let mob_ids: Vec<RcEntityId> = self.mobs.keys().copied().collect();

        for mob_id in mob_ids {
            // (1) sensing.
            if let Some(mob) = self.mobs.get_mut(&mob_id) {
                mob.sensing.clear();
            }

            let self_id = mob_id;
            let self_pos = self.mobs[&mob_id].base.pos;
            let self_kind = self.mobs[&mob_id].kind;
            let full_tick = rc_mechanics::ai::should_full_tick(tick_count, self_id);

            let hurt_by = self
                .mobs
                .get_mut(&mob_id)
                .and_then(|m| m.recent_damage.0.take());
            let hurt_by_pos = hurt_by.and_then(|id| self.entity_pos(id));

            // (2a) target_selector.tick — the real `GoalSelector`/`Goal` mechanism,
            // driven by `hurt_by` alone (`HurtByTargetGoal`'s own `can_use`).
            {
                let mut mob = self.mobs.remove(&mob_id).expect("mob present");
                let mut melee_scratch: Option<RcEntityId> = None;
                let mut look_target: Option<[f64; 3]> = None;
                let mut ctx = AiContext {
                    self_id,
                    self_pos,
                    self_rotation: mob.base.rotation,
                    self_kind,
                    attributes: &mob.attributes,
                    sensing: &mob.sensing,
                    memory: mob.brain.as_ref().map(|(brain, _)| brain),
                    world: self,
                    tick_count,
                    navigation: &mut mob.navigation,
                    movement_intent: &mut mob.movement_intent,
                    look_target: &mut look_target,
                    hurt_by,
                    melee_attack_signal: &mut melee_scratch,
                    current_target: None,
                    current_target_pos: None,
                };
                mob.target_selector.tick(&mut ctx, full_tick);
                self.mobs.insert(mob_id, mob);
            }

            let (current_target, current_target_pos) = if hurt_by.is_some() {
                (hurt_by, hurt_by_pos)
            } else {
                self.ordinary_target(mob_id)
            };
            if let Some(mob) = self.mobs.get_mut(&mob_id) {
                mob.current_target = current_target;
            }

            // (2b) goal_selector.tick.
            {
                let mut mob = self.mobs.remove(&mob_id).expect("mob present");
                let mut look_target: Option<[f64; 3]> = None;
                let mut ctx = AiContext {
                    self_id,
                    self_pos,
                    self_rotation: mob.base.rotation,
                    self_kind,
                    attributes: &mob.attributes,
                    sensing: &mob.sensing,
                    memory: mob.brain.as_ref().map(|(brain, _)| brain),
                    world: self,
                    tick_count,
                    navigation: &mut mob.navigation,
                    movement_intent: &mut mob.movement_intent,
                    look_target: &mut look_target,
                    hurt_by,
                    melee_attack_signal: &mut mob.pending_melee_attack.0,
                    current_target,
                    current_target_pos,
                };
                mob.goal_selector.tick(&mut ctx, full_tick);
                self.mobs.insert(mob_id, mob);
            }

            // (3) navigation — bookkeeping only in this suite (Context Part D's own
            // "movement realization" note): no Goal in this suite's own scenario set
            // drives real `find_path`-based movement through `PathNavigation` (scenarios
            // 1/2 call `find_path` directly, bypassing `tick()` entirely).
            {
                let mut mob = self.mobs.remove(&mob_id).expect("mob present");
                let evaluator = WalkNodeEvaluator;
                let movement_speed = mob.attributes.value_or(attribute::MOVEMENT_SPEED, 0.7);
                let follow_range = mob.attributes.value_or(attribute::FOLLOW_RANGE, 32.0);
                let max_visited_nodes = (follow_range * 16.0).floor().max(0.0) as u32;
                let (_, height) = mob_config::entity_dimensions(mob.kind);
                mob.navigation.tick(
                    mob.base.pos,
                    None,
                    movement_speed,
                    &evaluator,
                    self,
                    height,
                    max_visited_nodes,
                );
                self.mobs.insert(mob_id, mob);
            }

            // (4) brain.
            if let Some(mob) = self.mobs.get_mut(&mob_id)
                && mob.brain.is_some()
            {
                let mut mob = self.mobs.remove(&mob_id).expect("mob present");
                let mut look_target: Option<[f64; 3]> = None;
                {
                    let (brain, program) = mob.brain.as_mut().expect("checked Some above");
                    let mut rng = deterministic_rng(tick_count, self_id);
                    let mut ctx = AiContext {
                        self_id,
                        self_pos,
                        self_rotation: mob.base.rotation,
                        self_kind,
                        attributes: &mob.attributes,
                        sensing: &mob.sensing,
                        memory: None,
                        world: self,
                        tick_count,
                        navigation: &mut mob.navigation,
                        movement_intent: &mut mob.movement_intent,
                        look_target: &mut look_target,
                        hurt_by,
                        melee_attack_signal: &mut mob.pending_melee_attack.0,
                        current_target,
                        current_target_pos,
                    };
                    program.tick(&mut ctx, brain, tick_count, &mut rng);
                    // The panic-vs-schedule interaction (M4-B09 Context Part C.4's own
                    // final-report citation): `select_activity`'s own periodic re-evaluation
                    // would otherwise unconditionally override an active `Panic` with
                    // `Idle`/`Work`/`Meet` on its very next scheduled check, since it carries
                    // no special-case for the currently-active activity at all. Guarded here,
                    // at the adapter, to skip that call while `Panic` is active AND its own
                    // trigger memory is still present — resuming exactly once that memory's
                    // TTL (`HURT_BY_MEMORY_TTL_TICKS`) expires.
                    let panic_active = brain
                        .active_activities
                        .contains(&rc_mechanics::ai::brain::Activity::Panic);
                    let trigger_present = program
                        .panic_trigger_memory
                        .map(|m| {
                            brain.status(m) == rc_mechanics::ai::brain::MemoryStatus::ValuePresent
                        })
                        .unwrap_or(false);
                    if !(panic_active && trigger_present) {
                        program.select_activity(brain, tick_count);
                    }
                }
                self.mobs.insert(mob_id, mob);
            }

            // (5) movement-intent application, then this blueprint's own bounded
            // position-integration step (Context Part D's own "movement realization").
            if let Some(mob) = self.mobs.get_mut(&mob_id) {
                let intent = mob.movement_intent.0;
                if intent.forward > 0.0 {
                    let yaw_radians = ((intent.yaw_degrees as f64) + 90.0).to_radians();
                    mob.base.pos[0] += yaw_radians.cos() * STEP_BLOCKS_PER_TICK;
                    mob.base.pos[2] += yaw_radians.sin() * STEP_BLOCKS_PER_TICK;
                }
            }
        }
    }
}

/// A deterministic, bounded stand-in for a real per-region RNG stream (mirrors
/// `rc_mechanics::ai::systems::deterministic_rng`'s own identical, `pub(crate)`-scoped
/// generator, restated here since this crate cannot reach that private item).
fn deterministic_rng(tick_count: u64, entity_id: RcEntityId) -> impl FnMut() -> u32 {
    let mut state = tick_count
        .wrapping_mul(2654435761)
        .wrapping_add(entity_id.0);
    move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as u32
    }
}

impl BlockWorldAccess for ScenarioWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        // id 0 is the registry's own AIR state (rc_registries::generated_v776::
        // block_states::default_state::AIR.0) -- structural, not a guessed/hand-typed
        // block-state value; every position this suite's own scenarios do not
        // explicitly place a block at defaults to it (Context Part D's own doc
        // comment, above).
        // block-state-id-lint-waiver: see the citation immediately above.
        Some(self.blocks.get(&pos).copied().unwrap_or(BlockStateId(0)))
    }

    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos).copied() != Some(state);
        self.blocks.insert(pos, state);
        changed
    }

    fn dimension(&self) -> rc_core::DimensionId {
        rc_core::DimensionId::OVERWORLD
    }

    fn owner_of(&self, _chunk: rc_core::ChunkKey) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }

    fn local_identity(&self) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }
}
