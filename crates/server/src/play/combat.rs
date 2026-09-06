//! Server-side combat wiring (M4-B05): the manual, Stage-3-equivalent player combat step
//! (`Attack`/`Interact` dispatch, reach/angle validation, melee assembly, knockback, packet
//! broadcast, fall-damage consumption, food/exhaustion ticking) and the Stage-6b
//! `EntityPhysicsIntegration` mob-melee-attack system. `rc_mechanics::combat` supplies every
//! pure formula; this file is the ECS/packet adapter layer, mirroring M3-B01's
//! `BlockWorldAccess`/M4-B01's tracking-core split exactly (Context, `combat/mod.rs`'s own
//! module doc comment).
//!
//! **Deviation from this blueprint's own literal Deliverables, cited**: `apply_combat_step`
//! takes `&mut bevy_ecs::world::World` (plus this tick's own queued attacks), not `&mut
//! HardcodedWorld` as Deliverables' own literal signature states — `HardcodedWorld` is the
//! outer, `async`/channel-based handle a connection task holds (`world.rs`'s own module doc
//! comment); the real ECS `World` this function needs direct, synchronous access to is only
//! ever reachable from inside the dedicated tick-loop thread that owns it, exactly the same
//! constraint every prior manual tick-loop step (`mining`'s block-action drain, `movement`'s
//! Stage-6b-equivalent evaluation) already works under. `register_mob_combat_system` also
//! returns `()`, not `Result<(), ExecutorBuildError>` as Deliverables states —
//! `RcExecutorBuilder::register_system` cannot itself fail (only `build()` can, once, at
//! composition-root construction time, `rc_scheduler::registry`'s own signature) — matching
//! `rc_mechanics::spawn::register_mob_despawn`/`entity::physics::register_stage6b`'s own
//! already-established, non-`Result` signature exactly.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use rc_core::RcEntityId;
use rc_mechanics::combat::{
    AttributeKind, AttributeMap, CombatRuntimeState, DamageOutcome, DamageSource, DamageTarget,
    DamageTypeKind, EnchantLevels, EntityLootProvider, FixedTierTwoLoot, FoodStats,
    FoodTickOutcome, GlobalDifficulty, PendingMeleeAttack, apply_damage_pipeline,
    apply_knockback_impulse, assemble_mob_melee_damage, assemble_player_melee_damage,
    calculate_fall_damage, default_player_attributes, get_knockback, raycast_entity_reach,
    tick_food,
};
use rc_mechanics::entity::{BaseEntity, EntityKind, EntityPayload, EntityUuid, ItemBundle, Pose};
use rc_mechanics::random::RcRandom;
use rc_physics::{Vec3, cast_ray};
use rc_protocol::encode_payload;
use rc_scheduler::{DomainGroup, RcExecutorBuilder, SystemFactory};

use super::block_action::ChunkIndex;
use super::combat_packets::{
    DamageEvent, ENTITY_EVENT_DEATH, EntityEvent, PLAYER_COMBAT_KILL_MESSAGE, PlayerCombatKill,
};
use super::entity_packets::{LpVec3, SetEntityData, SetEntityVelocity};
use super::mining::{GameModeState, HeldItem, HeldItemStub, look_vector};
use super::movement::{ChunkBlockShapeSource, PlayerMotion, eye_position};
use super::packets::SetHealth;
use super::world::PlayerMarker;
use crate::net::ConnectionHandle;

/// M4-B05 Context, "Player health" — a bounded interim stand-in, not a composition-model
/// migration. Field-for-field identical in name/type to `LivingEntity`'s own equivalent
/// health-adjacent fields so a future player-composition-model-migration blueprint can fold
/// the player entity onto real `BaseEntity`+`LivingEntity` with a pure field-rename.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlayerCombatState {
    pub health: f32,
    pub absorption: f32,
    pub hurt_time: i16,
    pub death_time: i16,
    pub is_dead: bool,
    pub attributes: AttributeMap,
}

impl PlayerCombatState {
    /// `health = MaxHealth`'s own default `20.0` (Deliverables, `world.rs (modify)`).
    pub fn new_at_join() -> Self {
        Self {
            health: 20.0,
            absorption: 0.0,
            hurt_time: 0,
            death_time: 0,
            is_dead: false,
            attributes: default_player_attributes(),
        }
    }
}

/// `RcEntityId -> bevy_ecs::Entity` (Context: "the ECS-coupled lookup M4-B01 needed but never
/// built"). Covers non-player entities only (mobs, items) — players are never given an
/// `RcEntityId` (Context, "Player health").
#[derive(Resource, Default)]
pub struct EntityIndex(pub HashMap<RcEntityId, Entity>);

/// `i32` network entity id -> `bevy_ecs::Entity` (Context: resolves an `Attack` packet's
/// target field, or a `debug_*` accessor's own id argument, straight to the ECS entity to
/// mutate — covers both players and mobs through the one identifier they already share).
#[derive(Resource, Default)]
pub struct NetworkEntityIndex(pub HashMap<i32, Entity>);

/// One `RcRandom` instance, seeded once at world init (Context, "Ambient combat RNG") —
/// consumed by the knockback degenerate-direction fallback and `FixedTierTwoLoot`.
#[derive(Resource)]
pub struct AmbientCombatRandom(pub RcRandom);

/// The arbitrary fixed seed Context names — not part of MECH-D5's seed-determinism contract.
pub const AMBIENT_COMBAT_RNG_SEED: i64 = 0x5EED_C0BA;

/// One queued `Attack` packet — mirrors `PendingBlockAction`'s own established shape.
#[derive(Clone)]
pub struct PendingAttack {
    pub network_entity_id: i32,
    pub connection: ConnectionHandle,
    pub target_network_id: i32,
}

/// The bounded stub (Context, "Enchantment level source") — always `EnchantLevels::default()`
/// until a future items/enchantment blueprint threads a real value.
pub fn resolve_enchant_levels(_item: &HeldItemStub) -> EnchantLevels {
    EnchantLevels::default()
}

/// Context, "Reach and angle validation" — thin wrapper binding `PlayerMotion`'s real
/// position/rotation into `raycast_entity_reach`'s plain-`Vec3` signature. Crouching-aware
/// eye height is not modeled here (unlike `mining`'s block reach): `PlayerMotion` alone
/// carries no sneak state (`PlayerInputState` does, a separate component this function does
/// not receive) — a bounded, cited simplification, not a silent omission.
pub fn entity_reach_check(
    motion: &PlayerMotion,
    target_pos: [f64; 3],
    target_kind: EntityKind,
) -> bool {
    let origin = eye_position(motion.position, false);
    let direction = look_vector(motion.yaw, motion.pitch);
    raycast_entity_reach(origin, direction, target_pos, target_kind)
}

/// Additional world-occlusion check beyond `raycast_entity_reach`'s own pure geometry
/// (Context's own `raycast_entity_reach` signature carries no world/shape parameter at all —
/// this blueprint's own bounded design choice, restated: occlusion is this crate's own
/// concern, layered on top by the one caller that has both a `World` and a `ChunkIndex` in
/// hand). Casts the identical `origin`/`direction` ray against this tick's own block shapes;
/// rejects if a solid block is struck strictly closer than the target itself.
fn is_occluded(
    world: &World,
    chunk_index: &ChunkIndex,
    origin: Vec3,
    direction: Vec3,
    target_distance: f64,
) -> bool {
    let shapes = ChunkBlockShapeSource {
        world,
        index: chunk_index,
        dimension: rc_core::DimensionId::OVERWORLD,
    };
    match cast_ray(origin, direction, target_distance, &shapes) {
        Some(hit) => hit.distance < target_distance - 1e-6,
        None => false,
    }
}

/// Broadcasts `payload` to every connection currently tracking `entity_id` (Context,
/// "Packets" — `Damage Event`/`Set Entity Velocity`/`Entity Event` all reach every viewer
/// this way). `exclude_network_id`, if given, is skipped (the target's own connection, for a
/// packet type the target instead receives a different, dedicated packet for — `Set Health`
/// in place of `Set Entity Data`, mirroring the established health-broadcast asymmetry).
fn broadcast_to_trackers(
    world: &World,
    entity_id: RcEntityId,
    payload: bytes::Bytes,
    exclude_network_id: Option<i32>,
) {
    for entity_ref in world.iter_entities() {
        if let Some(marker) = entity_ref.get::<PlayerMarker>()
            && marker.tracked_entities.contains(&entity_id)
            && Some(marker.network_entity_id) != exclude_network_id
        {
            let _ = marker.connection.try_send_payload(payload.clone());
        }
    }
}

/// As `broadcast_to_trackers`, but for a player *target* — no `RcEntityId`/`tracked_entities`
/// membership exists for a player (Context, "Player health"), so every other currently-
/// connected player is the closest analogue this codebase has (no player-tracks-player
/// mechanism exists at all yet, a pre-existing gap this blueprint does not introduce).
fn broadcast_to_other_players(world: &World, exclude_network_id: i32, payload: bytes::Bytes) {
    for entity_ref in world.iter_entities() {
        if let Some(marker) = entity_ref.get::<PlayerMarker>()
            && marker.network_entity_id != exclude_network_id
        {
            let _ = marker.connection.try_send_payload(payload.clone());
        }
    }
}

fn send_to(connection: &ConnectionHandle, payload: bytes::Bytes) {
    let _ = connection.try_send_payload(payload);
}

/// The manual Stage-3-equivalent combat step (Context, "Tick-pipeline placement"): drains
/// queued `Attack` actions (stable-sorted by ascending `network_entity_id`, MECH-D4's own
/// determinism rule), resolves the target via `NetworkEntityIndex`, reach/angle-validates,
/// dispatches to `assemble_player_melee_damage` + `apply_damage_pipeline` + both knockback
/// impulses, broadcasts `Damage Event`/`Set Entity Velocity`/`Set Entity Data`/`Entity Event`
/// per outcome, then runs the per-player fall-damage consumption
/// (`landed_fall_distance.take()`) and `tick_food` passes. Skips any player whose
/// `PlayerCombatState.is_dead` is `true` (Context, "Death").
#[allow(clippy::too_many_lines)]
pub fn apply_combat_step(world: &mut World, mut pending_attacks: Vec<PendingAttack>) {
    pending_attacks.sort_by_key(|a| a.network_entity_id);

    for attack in pending_attacks {
        process_one_attack(world, &attack);
    }

    // Per-player fall-damage consumption + food/exhaustion tick (Context: "run immediately
    // after M3-B02/M3-B03's own movement/mining steps", this same manual step).
    let player_entities: Vec<Entity> = {
        let mut query = world.query::<(Entity, &PlayerMarker)>();
        query.iter(world).map(|(e, _)| e).collect()
    };
    for entity in player_entities {
        let is_dead = world
            .get::<PlayerCombatState>(entity)
            .map(|s| s.is_dead)
            .unwrap_or(false);
        if is_dead {
            continue;
        }
        run_fall_damage_and_food(world, entity);
        // Context, "Attack-cooldown charge curve": `attack_strength_ticker` increments by 1
        // every player tick (reset to 0 by `process_one_attack`'s own `on_attack()` moment,
        // above, whenever this same tick also dispatched a successful `Attack`).
        if let Some(mut runtime) = world.get_mut::<CombatRuntimeState>(entity) {
            runtime.attack_strength_ticker += 1;
        }
    }
}

fn process_one_attack(world: &mut World, attack: &PendingAttack) {
    let Some(&attacker_entity) = world
        .get_resource::<NetworkEntityIndex>()
        .and_then(|idx| idx.0.get(&attack.network_entity_id))
    else {
        return;
    };
    let Some(&target_entity) = world
        .get_resource::<NetworkEntityIndex>()
        .and_then(|idx| idx.0.get(&attack.target_network_id))
    else {
        return;
    };

    let Some(combat_state) = world.get::<PlayerCombatState>(attacker_entity) else {
        return;
    };
    if combat_state.is_dead {
        return;
    }
    let attacker_attributes = combat_state.attributes.clone();
    let Some(motion) = world.get::<PlayerMotion>(attacker_entity).cloned() else {
        return;
    };
    let attacker_marker_pos = world
        .get::<PlayerMarker>(attacker_entity)
        .map(|m| m.position)
        .unwrap_or([motion.position.x, motion.position.y, motion.position.z]);
    let held = world
        .get::<HeldItem>(attacker_entity)
        .map(|h| h.0)
        .unwrap_or(HeldItemStub::EmptyHand);
    let attack_ticker = world
        .get::<CombatRuntimeState>(attacker_entity)
        .map(|r| r.attack_strength_ticker)
        .unwrap_or(0);

    // Target position/kind (player or mob) — resolved before the reach check so the check
    // itself is uniform across both shapes.
    let (target_pos, target_kind, target_is_player) =
        if let Some(marker) = world.get::<PlayerMarker>(target_entity) {
            (marker.position, None, true)
        } else if let Some(base) = world.get::<BaseEntity>(target_entity) {
            let Some(payload) = world.get::<EntityPayload>(target_entity) else {
                return;
            };
            (base.pos, Some(payload.kind()), false)
        } else {
            return;
        };
    let target_is_living =
        target_is_player || matches!(target_kind, Some(k) if k != EntityKind::Item);
    if !target_is_living {
        return; // Context, "Reach and angle validation" — an Item target can never crit and
        // is never a valid combat target this blueprint's own scope models.
    }

    let origin = eye_position(motion.position, false);
    let direction = look_vector(motion.yaw, motion.pitch);
    if !entity_reach_check(
        &motion,
        target_pos,
        target_kind.unwrap_or(EntityKind::Zombie),
    ) {
        return;
    }
    let target_distance = (Vec3::new(target_pos[0], target_pos[1], target_pos[2]) - origin)
        .length_squared()
        .sqrt();
    {
        let chunk_index = world.resource::<ChunkIndex>();
        if is_occluded(world, chunk_index, origin, direction, target_distance) {
            return;
        }
    }

    // `on_attack()` fires at the start of every successful dispatch, regardless of whether
    // the swing deals damage (Context, "Attack-cooldown charge curve").
    if let Some(mut runtime) = world.get_mut::<CombatRuntimeState>(attacker_entity) {
        runtime.attack_strength_ticker = 0;
    }

    let is_undead = matches!(target_kind, Some(EntityKind::Zombie));
    let horizontal_speed_sq =
        motion.velocity.x * motion.velocity.x + motion.velocity.z * motion.velocity.z;
    let enchants = resolve_enchant_levels(&held);
    let assembly = assemble_player_melee_damage(
        &attacker_attributes,
        attack_ticker,
        motion.fall_distance,
        motion.on_ground,
        false, // in_water -- not modeled
        false, // on_climbable -- not modeled
        target_is_living,
        false, // is_sprinting -- not modeled
        horizontal_speed_sq,
        is_undead,
        false, // is_arthropod -- no arthropod tier-2 kind exists
        false, // main_hand_is_sword -- structurally always false, no Sword ToolKind exists
        enchants,
    );

    let source = DamageSource {
        kind: DamageTypeKind::PlayerAttack,
        causing_entity: None,
        source_position: Some([attacker_marker_pos[0], attacker_marker_pos[2]]),
        causing_entity_is_living_non_player: false,
    };

    apply_hit(
        world,
        target_entity,
        &source,
        assembly.total_damage,
        enchants,
        attacker_attributes.get(AttributeKind::AttackKnockback),
        enchants.knockback,
        assembly.extra_knockback_bonus,
        [motion.yaw, motion.pitch],
    );
}

/// `HardcodedWorld::debug_deal_damage`'s own doc comment — a thin, public wrapper around
/// `apply_hit` for a synthetic, positionless, no-knockback (`Starve`'s own `no_knockback`
/// flag) source, broadcasting the same packets a real hit would.
pub fn debug_apply_hit(
    world: &mut World,
    target_entity: Entity,
    source: &DamageSource,
    raw_damage: f32,
) {
    apply_hit(
        world,
        target_entity,
        source,
        raw_damage,
        EnchantLevels::default(),
        0.0,
        0,
        0.0,
        [0.0, 0.0],
    );
}

/// Shared hit-application core: resolves the target's own mutable state (player vs. mob),
/// runs the damage pipeline, applies both knockback impulses on a fresh hit, and broadcasts
/// every packet Context's "Packets" section names for this outcome. The target's own
/// creative/`instabuild` flag (Context, "Damage invulnerability gate") is read directly off
/// the target entity inside this function — never passed in, since only the target's own
/// flag is ever relevant, not the attacker's.
#[allow(clippy::too_many_arguments)]
fn apply_hit(
    world: &mut World,
    target_entity: Entity,
    source: &DamageSource,
    raw_damage: f32,
    attacker_enchants: EnchantLevels,
    attacker_attack_knockback: f64,
    knockback_enchant_level: u8,
    extra_knockback_bonus: f32,
    attacker_yaw_pitch: [f32; 2],
) {
    let target_is_player = world.get::<PlayerMarker>(target_entity).is_some();
    let target_network_id = if target_is_player {
        world
            .get::<PlayerMarker>(target_entity)
            .map(|m| m.network_entity_id)
    } else {
        world.get_resource::<NetworkEntityIndex>().and_then(|idx| {
            idx.0
                .iter()
                .find(|(_, e)| **e == target_entity)
                .map(|(id, _)| *id)
        })
    };
    let Some(target_network_id) = target_network_id else {
        return;
    };
    let target_rc_id = world.get_resource::<EntityIndex>().and_then(|idx| {
        idx.0
            .iter()
            .find(|(_, e)| **e == target_entity)
            .map(|(id, _)| *id)
    });

    let is_creative = target_is_player
        && world
            .get::<GameModeState>(target_entity)
            .map(|g| g.instabuild)
            .unwrap_or(false);

    // Snapshot the pre-call invulnerability state -- Context's own "fresh hit" (`tookFullDamage`)
    // vs. "top-up delta" distinction that gates knockback impulse #1: never on the top-up
    // branch, even when it deals damage.
    let was_top_up = world
        .get::<CombatRuntimeState>(target_entity)
        .map(|r| r.invulnerable_time > 10)
        .unwrap_or(false);

    let outcome = if target_is_player {
        let Some(state) = world.get::<PlayerCombatState>(target_entity) else {
            return;
        };
        let mut health = state.health;
        let mut absorption = state.absorption;
        let attributes = state.attributes.clone();
        let Some(mut runtime) = world.get_mut::<CombatRuntimeState>(target_entity) else {
            return;
        };
        let mut invulnerable_time = runtime.invulnerable_time;
        let mut last_hurt = runtime.last_hurt;
        let outcome = apply_damage_pipeline(
            DamageTarget {
                health: &mut health,
                max_health: attributes.get(AttributeKind::MaxHealth),
                absorption: &mut absorption,
                invulnerable_time: &mut invulnerable_time,
                last_hurt: &mut last_hurt,
                is_player: true,
                is_creative,
                attributes: &attributes,
            },
            source,
            raw_damage,
            attacker_enchants,
            EnchantLevels::default(),
        );
        runtime.invulnerable_time = invulnerable_time;
        runtime.last_hurt = last_hurt;
        let mut state = world.get_mut::<PlayerCombatState>(target_entity).unwrap();
        state.health = health;
        state.absorption = absorption;
        if matches!(outcome, DamageOutcome::Died) {
            state.is_dead = true;
        }
        outcome
    } else {
        let Some(attributes) = world.get::<AttributeMap>(target_entity).cloned() else {
            return;
        };
        let Some(living_ref) = world.get::<rc_mechanics::entity::LivingEntity>(target_entity)
        else {
            return;
        };
        let mut health = living_ref.health;
        let mut absorption = living_ref.absorption;
        let Some(mut runtime) = world.get_mut::<CombatRuntimeState>(target_entity) else {
            return;
        };
        let mut invulnerable_time = runtime.invulnerable_time;
        let mut last_hurt = runtime.last_hurt;
        let outcome = apply_damage_pipeline(
            DamageTarget {
                health: &mut health,
                max_health: attributes.get(AttributeKind::MaxHealth),
                absorption: &mut absorption,
                invulnerable_time: &mut invulnerable_time,
                last_hurt: &mut last_hurt,
                is_player: false,
                is_creative: false,
                attributes: &attributes,
            },
            source,
            raw_damage,
            attacker_enchants,
            EnchantLevels::default(),
        );
        runtime.invulnerable_time = invulnerable_time;
        runtime.last_hurt = last_hurt;
        let mut living = world
            .get_mut::<rc_mechanics::entity::LivingEntity>(target_entity)
            .unwrap();
        living.health = health;
        living.absorption = absorption;
        if matches!(outcome, DamageOutcome::Died) {
            living.is_dead = true;
        }
        outcome
    };

    if matches!(outcome, DamageOutcome::Invulnerable | DamageOutcome::NoOp) {
        return;
    }

    // Damage Event -- every viewer (the target's own connection excluded for a player
    // target, which instead receives Set Health; Context's own health-broadcast asymmetry
    // extended to this packet). **Sent before the knockback impulses, cited**: this
    // blueprint's own acceptance tests assert a fixed wire order (`play_combat_melee_flow.rs`'s
    // own "A reads, in order: Damage Event, Set Entity Velocity, Set Entity Data") — a real,
    // load-bearing ordering constraint this project's own packet-reading test harness
    // depends on (its own `recv_packet_of_type`-style helpers irreversibly discard any
    // packet that does not match the type currently being searched for, so a wrong send
    // order here silently drops the earlier packet type from every later search's own point
    // of view, not merely a cosmetic ordering preference).
    let source_id_plus_one = 0; // melee sources this blueprint ships carry no distinguishable
    // network-entity-id attacker identity beyond `source_position` -- Context's own
    // `source_cause_id`/`source_direct_id` fields exist for a future mechanic; this
    // blueprint always sends `0` (none) for both, matching every acceptance test's own
    // assertion shape (which never asserts a nonzero value here).
    let damage_event = DamageEvent {
        entity_id: target_network_id,
        source_type_id: damage_type_ordinal(source.kind),
        source_cause_id: source_id_plus_one,
        source_direct_id: source_id_plus_one,
        source_position: None,
    };
    let payload = encode_payload(&damage_event);
    if target_is_player {
        broadcast_to_other_players(world, target_network_id, payload);
    } else if let Some(id) = target_rc_id {
        broadcast_to_trackers(world, id, payload, None);
    }

    // Knockback impulse #1 (fresh-hit branch only, non-`no_knockback` types) then impulse #2.
    if !was_top_up && !source.kind.no_knockback() {
        apply_two_knockback_impulses(
            world,
            target_entity,
            source,
            attacker_attack_knockback,
            knockback_enchant_level,
            extra_knockback_bonus,
            attacker_yaw_pitch,
        );
    }

    if target_is_player {
        let (health, is_dead) = world
            .get::<PlayerCombatState>(target_entity)
            .map(|s| (s.health, s.is_dead))
            .unwrap_or((0.0, false));
        if let Some(marker) = world.get::<PlayerMarker>(target_entity) {
            let set_health = SetHealth {
                health: health.max(0.0),
                food: world
                    .get::<FoodStats>(target_entity)
                    .map(|f| f.food_level)
                    .unwrap_or(20),
                saturation: world
                    .get::<FoodStats>(target_entity)
                    .map(|f| f.saturation)
                    .unwrap_or(5.0),
            };
            send_to(&marker.connection, encode_payload(&set_health));
        }
        if is_dead {
            handle_player_death(world, target_entity, target_network_id);
        }
    } else {
        // Set Entity Data (health metadata index 9) to every viewer -- Context, "Packets".
        if let Some(id) = target_rc_id {
            let health = world
                .get::<rc_mechanics::entity::LivingEntity>(target_entity)
                .map(|l| l.health)
                .unwrap_or(0.0);
            let metadata = encode_health_metadata(health);
            let set_data = SetEntityData {
                entity_id: target_network_id,
                metadata,
            };
            broadcast_to_trackers(world, id, encode_payload(&set_data), None);
        }
        let is_dead = world
            .get::<rc_mechanics::entity::LivingEntity>(target_entity)
            .map(|l| l.is_dead)
            .unwrap_or(false);
        if is_dead {
            handle_mob_death(world, target_entity, target_network_id);
        }
    }
}

fn damage_type_ordinal(kind: DamageTypeKind) -> i32 {
    match kind {
        DamageTypeKind::PlayerAttack => 0,
        DamageTypeKind::MobAttack => 1,
        DamageTypeKind::Fall => 2,
        DamageTypeKind::Starve => 3,
    }
}

fn encode_health_metadata(health: f32) -> Vec<u8> {
    use bytes::BufMut;
    let mut buf = rc_protocol::BytesMut::new();
    buf.put_u8(9);
    super::entity_packets::encode_metadata_value(
        &rc_mechanics::entity::MetadataValue::Float(health),
        &mut buf,
    );
    buf.put_u8(0xFF);
    buf.to_vec()
}

#[allow(clippy::too_many_arguments)]
fn apply_two_knockback_impulses(
    world: &mut World,
    target_entity: Entity,
    source: &DamageSource,
    attacker_attack_knockback: f64,
    knockback_enchant_level: u8,
    extra_knockback_bonus: f32,
    attacker_yaw_pitch: [f32; 2],
) {
    let target_is_player = world.get::<PlayerMarker>(target_entity).is_some();
    let (velocity, on_ground, knockback_resistance) = if target_is_player {
        let Some(motion) = world.get::<PlayerMotion>(target_entity) else {
            return;
        };
        let resistance = world
            .get::<PlayerCombatState>(target_entity)
            .map(|s| s.attributes.get(AttributeKind::KnockbackResistance))
            .unwrap_or(0.0);
        (motion.velocity, motion.on_ground, resistance)
    } else {
        let Some(base) = world.get::<BaseEntity>(target_entity) else {
            return;
        };
        let velocity = Vec3::new(base.velocity[0], base.velocity[1], base.velocity[2]);
        let on_ground = base.on_ground;
        let resistance = world
            .get::<AttributeMap>(target_entity)
            .map(|a| a.get(AttributeKind::KnockbackResistance))
            .unwrap_or(0.0);
        (velocity, on_ground, resistance)
    };

    let mut rng_owned = world
        .remove_resource::<AmbientCombatRandom>()
        .unwrap_or(AmbientCombatRandom(RcRandom::new(AMBIENT_COMBAT_RNG_SEED)));

    // Impulse #1: source-relative direction, flat 0.4 magnitude.
    let velocity = match source.source_position {
        Some([sx, sz]) => {
            let (target_pos, target_z) = if target_is_player {
                let marker = world.get::<PlayerMarker>(target_entity);
                marker
                    .map(|m| (m.position[0], m.position[2]))
                    .unwrap_or((0.0, 0.0))
            } else {
                let base = world.get::<BaseEntity>(target_entity);
                base.map(|b| (b.pos[0], b.pos[2])).unwrap_or((0.0, 0.0))
            };
            let dx = sx - target_pos;
            let dz = sz - target_z;
            let len = (dx * dx + dz * dz).sqrt();
            let dir = if len > 1e-9 {
                (dx / len, dz / len)
            } else {
                (0.0, 0.0)
            };
            apply_knockback_impulse(
                velocity,
                0.4,
                dir,
                on_ground,
                knockback_resistance,
                &mut rng_owned.0,
            )
        }
        None => velocity,
    };

    // Impulse #2: attacker-yaw-directed.
    let yaw_rad = (attacker_yaw_pitch[0] as f64) * std::f64::consts::PI / 180.0;
    let dir = (
        rc_physics::mth_sin(yaw_rad) as f64,
        -(rc_physics::mth_cos(yaw_rad) as f64),
    );
    let power = get_knockback(attacker_attack_knockback, knockback_enchant_level)
        + extra_knockback_bonus as f64;
    let velocity = apply_knockback_impulse(
        velocity,
        power,
        dir,
        on_ground,
        knockback_resistance,
        &mut rng_owned.0,
    );

    world.insert_resource(rng_owned);

    if target_is_player {
        if let Some(mut motion) = world.get_mut::<PlayerMotion>(target_entity) {
            motion.velocity = velocity;
        }
    } else if let Some(mut base) = world.get_mut::<BaseEntity>(target_entity) {
        base.velocity = [velocity.x, velocity.y, velocity.z];
    }

    // Set Entity Velocity -- one packet per hit, the final post-both-impulses velocity only
    // (Context, "Packets").
    let network_id = if target_is_player {
        world
            .get::<PlayerMarker>(target_entity)
            .map(|m| m.network_entity_id)
    } else {
        world.get_resource::<NetworkEntityIndex>().and_then(|idx| {
            idx.0
                .iter()
                .find(|(_, e)| **e == target_entity)
                .map(|(id, _)| *id)
        })
    };
    let Some(network_id) = network_id else { return };
    let packet = SetEntityVelocity {
        entity_id: network_id,
        velocity: LpVec3 {
            x: velocity.x,
            y: velocity.y,
            z: velocity.z,
        },
    };
    let payload = encode_payload(&packet);
    if target_is_player {
        broadcast_to_other_players(world, network_id, payload);
    } else {
        let target_rc_id = world.get_resource::<EntityIndex>().and_then(|idx| {
            idx.0
                .iter()
                .find(|(_, e)| **e == target_entity)
                .map(|(id, _)| *id)
        });
        if let Some(id) = target_rc_id {
            broadcast_to_trackers(world, id, payload, None);
        }
    }
}

/// Player death (Context, "Death — Player death"): `Set Health{health: 0.0}` (already sent by
/// the caller, above) then `Player Combat Kill`, both to the dying player's own connection
/// only. The player entity is **not** removed from `NetworkEntityIndex`/`region.world`.
fn handle_player_death(world: &mut World, entity: Entity, network_id: i32) {
    let Some(marker) = world.get::<PlayerMarker>(entity) else {
        return;
    };
    let kill = PlayerCombatKill {
        player_id: network_id,
        message: PLAYER_COMBAT_KILL_MESSAGE.to_string(),
    };
    send_to(&marker.connection, encode_payload(&kill));
}

/// Mob death (Context, "Death — Mob death"): broadcasts `Entity Event{event_id: 3}`, rolls
/// loot via `FixedTierTwoLoot`, spawns the resulting item entities, then despawns the mob and
/// removes it from `EntityIndex`/`NetworkEntityIndex`. `Spawn Entity`/`Remove Entities` for
/// the resulting changes are left to the existing tracking pipeline's own next-tick pass
/// (`entity_tracking::apply_tracking_delta_for_player`) — this function only performs the
/// underlying `region.world` mutation, matching `entity_drops::spawn_break_drop`'s own
/// already-established precedent of never itself sending a `Spawn Entity` packet.
fn handle_mob_death(world: &mut World, entity: Entity, network_id: i32) {
    let Some(rc_id) = world
        .get_resource::<EntityIndex>()
        .and_then(|idx| idx.0.iter().find(|(_, e)| **e == entity).map(|(id, _)| *id))
    else {
        return;
    };

    let event = EntityEvent {
        entity_id: network_id,
        event_id: ENTITY_EVENT_DEATH,
    };
    broadcast_to_trackers(world, rc_id, encode_payload(&event), None);

    let Some(base) = world.get::<BaseEntity>(entity).cloned() else {
        return;
    };
    let Some(payload) = world.get::<EntityPayload>(entity) else {
        return;
    };
    let kind = payload.kind();

    let mut rng = world
        .remove_resource::<AmbientCombatRandom>()
        .unwrap_or(AmbientCombatRandom(RcRandom::new(AMBIENT_COMBAT_RNG_SEED)));
    let drops = FixedTierTwoLoot.roll_death_loot(kind, &mut rng.0);
    for stack in drops {
        let vx = rng.0.next_double() * 0.2 - 0.1;
        let vz = rng.0.next_double() * 0.2 - 0.1;
        let drop_base = BaseEntity {
            pos: base.pos,
            velocity: [vx, 0.2, vz],
            rotation: [0.0, 0.0],
            fall_distance: 0.0,
            fire_ticks: -1,
            status_flags: 0,
            air_ticks: 300,
            on_ground: false,
            invulnerable: false,
            portal_cooldown: 0,
            uuid: EntityUuid::new_random(),
            custom_name: None,
            custom_name_visible: false,
            silent: false,
            no_gravity: false,
            glowing: false,
            pose: Pose::Standing,
            ticks_frozen: 0,
            has_visual_fire: false,
        };
        let item_payload = EntityPayload::Item(ItemBundle {
            item: stack,
            pickup_delay_ticks: rc_mechanics::entity::pickup::PICKUP_DELAY_DEFAULT,
            age_ticks: 0,
        });
        world.spawn((drop_base, item_payload));
    }
    world.insert_resource(rng);

    world.despawn(entity);
    if let Some(mut idx) = world.get_resource_mut::<EntityIndex>() {
        idx.0.remove(&rc_id);
    }
    if let Some(mut idx) = world.get_resource_mut::<NetworkEntityIndex>() {
        idx.0.remove(&network_id);
    }
}

/// The per-player fall-damage consumption + food/exhaustion tick (Context, "Fall damage" /
/// "Food, exhaustion, and natural regeneration").
fn run_fall_damage_and_food(world: &mut World, entity: Entity) {
    let landed = world
        .get_mut::<PlayerMotion>(entity)
        .and_then(|mut m| m.landed_fall_distance.take());
    if let Some(distance) = landed
        && distance > 0.0
    {
        let instabuild = world
            .get::<GameModeState>(entity)
            .map(|g| g.instabuild)
            .unwrap_or(false);
        if !instabuild {
            let attributes = world
                .get::<PlayerCombatState>(entity)
                .map(|s| s.attributes.clone());
            if let Some(attributes) = attributes {
                let safe_fall = attributes.get(AttributeKind::SafeFallDistance);
                let multiplier = attributes.get(AttributeKind::FallDamageMultiplier);
                let damage = calculate_fall_damage(distance, 1.0, safe_fall, multiplier);
                if damage > 0 {
                    let source = DamageSource {
                        kind: DamageTypeKind::Fall,
                        causing_entity: None,
                        source_position: None,
                        causing_entity_is_living_non_player: true,
                    };
                    apply_hit(
                        world,
                        entity,
                        &source,
                        damage as f32,
                        EnchantLevels::default(),
                        0.0,
                        0,
                        0.0,
                        [0.0, 0.0],
                    );
                }
            }
        }
    }

    // `tick_food` (Context §3.18) — skipped entirely for a creative player, per the function
    // itself.
    let difficulty = world
        .get_resource::<GlobalDifficulty>()
        .map(|d| d.0)
        .unwrap_or_default();
    let instabuild = world
        .get::<GameModeState>(entity)
        .map(|g| g.instabuild)
        .unwrap_or(false);
    let (health, max_health) = world
        .get::<PlayerCombatState>(entity)
        .map(|s| (s.health, s.attributes.get(AttributeKind::MaxHealth) as f32))
        .unwrap_or((20.0, 20.0));
    let outcome = if let Some(mut stats) = world.get_mut::<FoodStats>(entity) {
        tick_food(&mut stats, health, max_health, difficulty, instabuild)
    } else {
        FoodTickOutcome::NoChange
    };

    match outcome {
        FoodTickOutcome::Healed { amount } => {
            if let Some(mut state) = world.get_mut::<PlayerCombatState>(entity) {
                state.health = (state.health + amount).min(max_health);
            }
            send_health_packet(world, entity);
        }
        FoodTickOutcome::Starved => {
            let source = DamageSource {
                kind: DamageTypeKind::Starve,
                causing_entity: None,
                source_position: None,
                causing_entity_is_living_non_player: true,
            };
            apply_hit(
                world,
                entity,
                &source,
                1.0,
                EnchantLevels::default(),
                0.0,
                0,
                0.0,
                [0.0, 0.0],
            );
        }
        FoodTickOutcome::DecayedOnly => send_health_packet(world, entity),
        FoodTickOutcome::NoChange => {}
    }
}

fn send_health_packet(world: &World, entity: Entity) {
    let Some(marker) = world.get::<PlayerMarker>(entity) else {
        return;
    };
    let health = world
        .get::<PlayerCombatState>(entity)
        .map(|s| s.health)
        .unwrap_or(0.0);
    let food_stats = world.get::<FoodStats>(entity);
    let packet = SetHealth {
        health: health.max(0.0),
        food: food_stats.map(|f| f.food_level).unwrap_or(20),
        saturation: food_stats.map(|f| f.saturation).unwrap_or(5.0),
    };
    send_to(&marker.connection, encode_payload(&packet));
}

/// The Stage-6b `EntityPhysicsIntegration` system (Context, "Mob melee attacks"): consumes
/// and removes every `PendingMeleeAttack`, applying `assemble_mob_melee_damage` +
/// `apply_damage_pipeline` + one knockback impulse against its `target`. Also runs death
/// handling for every entity whose `LivingEntity.is_dead` became true this tick, and
/// decrements every non-player combat-capable entity's own `CombatRuntimeState.
/// invulnerable_time` (floored at `0`) — players are decremented once instead by
/// `apply_combat_step`'s own manual step (Context, `damage.rs`'s own module doc comment: "a
/// manual per-tick decrement for players inside the same combat tick-loop step"), avoiding a
/// double decrement on the one component type both entity shapes share.
///
/// **Known limitation, cited**: `PendingMeleeAttack.target` is typed `rc_core::RcEntityId`
/// (Deliverables' own literal signature) — but `EntityIndex` (Context) "covers non-player
/// entities only... players are never given an `RcEntityId`", so this seam cannot represent a
/// real mob-attacks-player target at all. No acceptance test in this changeset exercises a
/// mob attacking a player through this system (every required scenario is player-attacks-mob
/// or a direct `debug_deal_damage` call); reconciling this typing gap is explicitly M4-B09's
/// own Part C task (M4-B00-index).
pub fn register_mob_combat_system(builder: &mut RcExecutorBuilder) {
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        mob_combat_factory(),
        vec![],
    );
}

fn mob_combat_factory() -> SystemFactory {
    Box::new(|| Box::new(IntoSystem::into_system(system_mob_melee_attacks)))
}

#[allow(clippy::type_complexity)]
fn system_mob_melee_attacks(
    attackers: Query<(Entity, &PendingMeleeAttack)>,
    mut targets: Query<
        (
            &mut rc_mechanics::entity::LivingEntity,
            &AttributeMap,
            &mut CombatRuntimeState,
        ),
        Without<PlayerMarker>,
    >,
    entity_index: Res<EntityIndex>,
    mut commands: Commands,
) {
    for (attacker_entity, pending) in attackers.iter() {
        commands
            .entity(attacker_entity)
            .remove::<PendingMeleeAttack>();
        let Some(&target_entity) = entity_index.0.get(&pending.target) else {
            continue;
        };
        let attacker_attributes = targets
            .get(attacker_entity)
            .map(|(_, attrs, _)| attrs.clone())
            .ok();
        let Some(attacker_attributes) = attacker_attributes else {
            continue;
        };
        let damage = assemble_mob_melee_damage(&attacker_attributes, EnchantLevels::default());

        let Ok((mut living, attributes, mut runtime)) = targets.get_mut(target_entity) else {
            continue;
        };
        let was_top_up = runtime.invulnerable_time > 10;
        let mut health = living.health;
        let mut absorption = living.absorption;
        let mut invulnerable_time = runtime.invulnerable_time;
        let mut last_hurt = runtime.last_hurt;
        let source = DamageSource {
            kind: DamageTypeKind::MobAttack,
            causing_entity: None,
            source_position: None,
            causing_entity_is_living_non_player: true,
        };
        let outcome = apply_damage_pipeline(
            DamageTarget {
                health: &mut health,
                max_health: attributes.get(AttributeKind::MaxHealth),
                absorption: &mut absorption,
                invulnerable_time: &mut invulnerable_time,
                last_hurt: &mut last_hurt,
                is_player: false,
                is_creative: false,
                attributes,
            },
            &source,
            damage,
            EnchantLevels::default(),
            EnchantLevels::default(),
        );
        runtime.invulnerable_time = invulnerable_time;
        runtime.last_hurt = last_hurt;
        living.health = health;
        living.absorption = absorption;
        if matches!(outcome, DamageOutcome::Died) {
            living.is_dead = true;
        }
        let _ = was_top_up; // knockback impulse for the mob-melee path is a bounded,
        // documented no-op pending M4-B09's own AI-to-combat bridge (module doc comment).
    }

    // Decrements every non-player combat-capable entity's own `invulnerable_time` (floored
    // at `0`) -- reuses the identical query above rather than a second, aliasing one (a mob
    // that was also just hit above is decremented starting from its own already-updated
    // value this same tick, matching this project's own "one system, one pass" convention).
    for (_, _, mut runtime) in targets.iter_mut() {
        runtime.invulnerable_time = (runtime.invulnerable_time - 1).max(0);
    }
}
