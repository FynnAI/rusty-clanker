//! The production adapter around `rc-mechanics`' pure `compute_tracking_delta`
//! (M4-B01, Context: "The production integration"). Called once per `PlayerMarker` per
//! tick, **after** the block-action drain-and-apply step and **before**
//! `executor.tick_region(...)` — mirroring M2-B07/M3-B02's own established manual-step
//! placement.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bytes::{BufMut, BytesMut};
use rc_core::RcEntityId;
use rc_mechanics::entity::physics::{ITEM_HALF_WIDTH, ITEM_HEIGHT};
use rc_mechanics::entity::pickup::ITEM_PICKUP_AABB_INFLATE;
use rc_mechanics::entity::{
    BaseEntity, EntityKind, EntityMetadataFields, EntityPayload, ItemStackRecord, LivingEntity,
    PickedUpItems, compute_tracking_delta,
};
use rc_physics::{Aabb, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, Vec3};
use rc_protocol::{VarInt, encode_payload};

use crate::play::entity_packets::{
    LpVec3, RemoveEntities, SetEntityData, SetEntityVelocity, SpawnEntity, TakeItemEntity,
    TeleportEntity, UpdateEntityPosition, encode_angle, encode_metadata_value,
    encode_position_delta, java_round,
};
use crate::play::world::PlayerMarker;

/// This blueprint's own bounded, explicitly-cited simplification (Context, "The
/// production integration" and Implementation step 13): a real `RcEntityId ->
/// network_entity_id` directory is out of this blueprint's own composition-root
/// scope (ARCH-D24's directory is explicitly not built here) — every tier-2 entity
/// this milestone's own debug-spawn seam ever constructs draws its `RcEntityId` from a
/// fresh, small, sequential `RcEntityIdAllocator`, so truncating the id's own low 32
/// bits is a safe, collision-free stand-in network id for this milestone's own test/
/// debug scope. A future blueprint supplying the real directory plugs it in here
/// without changing this function's own signature.
fn stand_in_network_id(id: RcEntityId) -> i32 {
    id.0 as i32
}

/// Every metadata entry `base`/`living`/`payload` together contribute, in that
/// declaration order (base first, then living when present, then the kind-specific
/// bundle) — mirrors `rc_mechanics::entity::snapshot::serialize_entity_snapshot`'s own
/// identical three-part component ordering.
fn collect_metadata_entries(
    base: &BaseEntity,
    living: Option<&LivingEntity>,
    payload: &EntityPayload,
) -> Vec<(u8, rc_mechanics::entity::MetadataValue)> {
    let mut entries = base.metadata_entries();
    if let Some(living) = living {
        entries.extend(living.metadata_entries());
    }
    entries.extend(match payload {
        EntityPayload::Item(item) => item.metadata_entries(),
        EntityPayload::Zombie(_) => Vec::new(),
        EntityPayload::Villager(villager) => villager.metadata_entries(),
        EntityPayload::Cow(_) => Vec::new(),
    });
    entries
}

/// Encodes the framed `(index, type, value)*` sequence, `0xFF`-terminated (Context:
/// "Entity metadata protocol... Framing"), via this crate's own `rc-protocol`-backed
/// `encode_metadata_value` — the production counterpart to `rc_mechanics::entity::
/// metadata::encode_metadata_entries`'s own independent, `rc-protocol`-free
/// reimplementation (WS-D3 rule 2 bars `rc-mechanics` from depending on `rc-protocol`
/// itself, Context).
fn encode_metadata_entries_wire(entries: &[(u8, rc_mechanics::entity::MetadataValue)]) -> Vec<u8> {
    let mut buf = BytesMut::new();
    for (index, value) in entries {
        buf.put_u8(*index);
        encode_metadata_value(value, &mut buf);
    }
    buf.put_u8(0xFF);
    buf.to_vec()
}

/// The production adapter around `rc-mechanics`' pure `compute_tracking_delta`
/// (Context: "The production integration"). Called once per `PlayerMarker` per tick,
/// **after** the block-action drain-and-apply step and **before**
/// `executor.tick_region(...)` — mirroring M2-B07/M3-B02's own established manual-step
/// placement, restated. Mutates `marker.tracked_entities` in place; sends `Spawn
/// Entity` + `Set Entity Data` for each newly-in-range entity and `Remove Entities`
/// for each newly-out-of-range one, over `marker.connection`, via
/// `ConnectionHandle::try_send_payload` (never blocking, matching every prior
/// broadcast call site's own established non-async-context calling convention).
pub fn apply_tracking_delta_for_player(
    marker: &mut PlayerMarker,
    viewer_pos: [f64; 3],
    live_entities: impl IntoIterator<
        Item = (
            RcEntityId,
            EntityKind,
            BaseEntity,
            Option<LivingEntity>,
            EntityPayload,
        ),
    > + Clone,
) {
    let tracking_view = live_entities
        .clone()
        .into_iter()
        .map(|(id, kind, base, _living, _payload)| (id, kind, base.pos));
    let delta = compute_tracking_delta(viewer_pos, &marker.tracked_entities, tracking_view);

    for &id in &delta.to_spawn {
        let Some((_, kind, base, living, payload)) = live_entities
            .clone()
            .into_iter()
            .find(|(entity_id, ..)| *entity_id == id)
        else {
            continue;
        };

        let network_id = stand_in_network_id(id);
        let spawn = SpawnEntity {
            entity_id: network_id,
            uuid: base.uuid.0,
            entity_type: kind.registry_id().0 as i32,
            x: base.pos[0],
            y: base.pos[1],
            z: base.pos[2],
            movement: LpVec3 {
                x: base.velocity[0],
                y: base.velocity[1],
                z: base.velocity[2],
            },
            pitch: encode_angle(base.rotation[1]),
            yaw: encode_angle(base.rotation[0]),
            head_yaw: encode_angle(base.rotation[0]),
            data: 0,
        };
        let _ = marker.connection.try_send_payload(encode_payload(&spawn));

        let entries = collect_metadata_entries(&base, living.as_ref(), &payload);
        let set_data = SetEntityData {
            entity_id: network_id,
            metadata: encode_metadata_entries_wire(&entries),
        };
        let _ = marker
            .connection
            .try_send_payload(encode_payload(&set_data));
    }

    for &id in &delta.to_despawn {
        let remove = RemoveEntities {
            entity_ids: vec![VarInt::new(stand_in_network_id(id))],
        };
        let _ = marker.connection.try_send_payload(encode_payload(&remove));
    }

    for &id in &delta.to_spawn {
        marker.tracked_entities.insert(id);
    }
    for &id in &delta.to_despawn {
        marker.tracked_entities.remove(&id);
    }
}

/// `09-entities-ai.md` §3.2's own documented `updateInterval` default (Context §O), applied
/// uniformly across every tier-2 kind.
pub const ENTITY_UPDATE_INTERVAL_TICKS: u64 = 3;

/// A small, non-zero epsilon (Context §O) — avoids re-sending a bit-for-bit-idle entity every
/// three ticks forever.
const RESYNC_EPSILON: f64 = 1e-4;

/// The `±8`-block per-axis range the delta-encoded `UpdateEntityPosition` packet can express
/// (`encode_position_delta`'s own `i16`, `/4096.0` fixed-point) — beyond it, `TeleportEntity`
/// (absolute) is required instead.
fn axis_delta_fits_i16(old: f64, new: f64) -> bool {
    i16::try_from(java_round(new * 4096.0) - java_round(old * 4096.0)).is_ok()
}

/// Context §O — the post-`tick_region` resync step, positioned after `executor.tick_region`
/// returns so it observes this tick's own Stage 6b physics output (unlike `still_tracked`'s
/// own membership, computed pre-tick by `apply_tracking_delta_for_player`). Reads each
/// tracked entity's current `BaseEntity.pos`/`velocity` (real ECS entities this milestone's
/// own `entity_drops::spawn_break_drop` spawns, bridged back from a tracked `RcEntityId` via
/// `Entity::from_bits` — the identical `RcEntityId(Entity::to_bits())` convention `entity_
/// tracking.rs`'s own `stand_in_network_id` already establishes, `docs/findings-for-planning.
/// md`), compares against `PlayerMarker.last_sent_entity_state`, and sends `UpdateEntity
/// Position`/`TeleportEntity` + `SetEntityVelocity` for anything that changed beyond `1e-4`,
/// gated to fire only when `current_tick % ENTITY_UPDATE_INTERVAL_TICKS == 0`.
pub fn entity_resync_step(world: &mut bevy_ecs::world::World, current_tick: u64) {
    if !current_tick.is_multiple_of(ENTITY_UPDATE_INTERVAL_TICKS) {
        return;
    }

    // Pass 1: snapshot every live tracked-kind entity's current pos/velocity/on_ground,
    // keyed by its own derived `RcEntityId` — a plain owned map, so pass 2's mutable
    // `PlayerMarker` query below never needs a second, conflicting borrow of `world`.
    let mut live: HashMap<RcEntityId, ([f64; 3], [f64; 3], bool)> = HashMap::new();
    {
        let mut snapshot_query = world.query::<(Entity, &BaseEntity)>();
        for (entity, base) in snapshot_query.iter(world) {
            live.insert(
                RcEntityId(entity.to_bits()),
                (base.pos, base.velocity, base.on_ground),
            );
        }
    }

    let mut marker_query = world.query::<&mut PlayerMarker>();
    for mut marker in marker_query.iter_mut(world) {
        let tracked_ids: Vec<RcEntityId> = marker.tracked_entities.iter().copied().collect();
        for id in tracked_ids {
            let Some(&(pos, velocity, on_ground)) = live.get(&id) else {
                continue;
            };
            let last = marker.last_sent_entity_state.get(&id).copied();
            let (pos_changed, vel_changed) = match last {
                None => (true, true),
                Some((last_pos, last_vel)) => (
                    (0..3).any(|i| (pos[i] - last_pos[i]).abs() > RESYNC_EPSILON),
                    (0..3).any(|i| (velocity[i] - last_vel[i]).abs() > RESYNC_EPSILON),
                ),
            };
            if !pos_changed && !vel_changed {
                continue;
            }

            let network_id = stand_in_network_id(id);
            let (old_pos, _old_vel) = last.unwrap_or((pos, velocity));

            if pos_changed {
                let fits = (0..3).all(|i| axis_delta_fits_i16(old_pos[i], pos[i]));
                if fits {
                    let update = UpdateEntityPosition {
                        entity_id: network_id,
                        delta_x: encode_position_delta(old_pos[0], pos[0]),
                        delta_y: encode_position_delta(old_pos[1], pos[1]),
                        delta_z: encode_position_delta(old_pos[2], pos[2]),
                        on_ground,
                    };
                    let _ = marker.connection.try_send_payload(encode_payload(&update));
                } else {
                    let teleport = TeleportEntity {
                        entity_id: network_id,
                        x: pos[0],
                        y: pos[1],
                        z: pos[2],
                        velocity_x: velocity[0],
                        velocity_y: velocity[1],
                        velocity_z: velocity[2],
                        yaw: 0.0,
                        pitch: 0.0,
                        on_ground,
                    };
                    let _ = marker
                        .connection
                        .try_send_payload(encode_payload(&teleport));
                }
            }
            if vel_changed {
                let set_velocity = SetEntityVelocity {
                    entity_id: network_id,
                    velocity: LpVec3 {
                        x: velocity[0],
                        y: velocity[1],
                        z: velocity[2],
                    },
                };
                let _ = marker
                    .connection
                    .try_send_payload(encode_payload(&set_velocity));
            }

            marker.last_sent_entity_state.insert(id, (pos, velocity));
        }
    }
}

/// One decided pickup this tick — `entity_pickup_step`'s own Pass-1/Pass-2 split (Context §M's
/// own player-touching half, `docs/findings-for-planning.md`: `rc-mechanics`' Stage 6b system
/// structurally cannot see `PlayerMarker`, so this half of pickup lives here instead).
struct PickupDecision {
    item_entity: Entity,
    collector_entity: Entity,
    collector_network_id: i32,
    stack: ItemStackRecord,
}

/// Context §M's own player-touching half of pickup: eligibility (`pickup_delay_ticks == 0`
/// AND a `PlayerMarker`'s own collision `Aabb` intersects the item entity's own collision
/// `Aabb` inflated by `ITEM_PICKUP_AABB_INFLATE` on every axis), the item entity's own
/// despawn-on-pickup, the `Take Item Entity` broadcast to every player currently tracking
/// either entity, and the `PickedUpItems` append. Positioned alongside `entity_resync_step`
/// (after `executor.tick_region` returns, so it observes this tick's own fresh positions).
pub fn entity_pickup_step(world: &mut bevy_ecs::world::World) {
    struct ItemCandidate {
        entity: Entity,
        position: [f64; 3],
        stack: ItemStackRecord,
    }

    let mut candidates: Vec<ItemCandidate> = Vec::new();
    {
        let mut item_query = world.query::<(Entity, &BaseEntity, &EntityPayload)>();
        for (entity, base, payload) in item_query.iter(world) {
            if let EntityPayload::Item(item) = payload
                && item.pickup_delay_ticks == 0
            {
                candidates.push(ItemCandidate {
                    entity,
                    position: base.pos,
                    stack: item.item.clone(),
                });
            }
        }
    }
    if candidates.is_empty() {
        return;
    }

    let mut decisions: Vec<PickupDecision> = Vec::new();
    let mut claimed: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    {
        let mut marker_query = world.query::<(Entity, &PlayerMarker)>();
        for (marker_entity, marker) in marker_query.iter(world) {
            let player_aabb = Aabb::from_position(
                Vec3::new(marker.position[0], marker.position[1], marker.position[2]),
                PLAYER_HALF_WIDTH,
                PLAYER_HEIGHT,
            );
            for candidate in &candidates {
                if claimed.contains(&candidate.entity) {
                    continue;
                }
                let item_aabb = Aabb::from_position(
                    Vec3::new(
                        candidate.position[0],
                        candidate.position[1],
                        candidate.position[2],
                    ),
                    ITEM_HALF_WIDTH,
                    ITEM_HEIGHT,
                );
                let inflated = Aabb {
                    min: Vec3::new(
                        item_aabb.min.x - ITEM_PICKUP_AABB_INFLATE,
                        item_aabb.min.y - ITEM_PICKUP_AABB_INFLATE,
                        item_aabb.min.z - ITEM_PICKUP_AABB_INFLATE,
                    ),
                    max: Vec3::new(
                        item_aabb.max.x + ITEM_PICKUP_AABB_INFLATE,
                        item_aabb.max.y + ITEM_PICKUP_AABB_INFLATE,
                        item_aabb.max.z + ITEM_PICKUP_AABB_INFLATE,
                    ),
                };
                let overlaps = player_aabb.overlaps_on(rc_physics::aabb::Axis::X, inflated, 0.0)
                    && player_aabb.overlaps_on(rc_physics::aabb::Axis::Y, inflated, 0.0)
                    && player_aabb.overlaps_on(rc_physics::aabb::Axis::Z, inflated, 0.0);
                if overlaps {
                    claimed.insert(candidate.entity);
                    decisions.push(PickupDecision {
                        item_entity: candidate.entity,
                        collector_entity: marker_entity,
                        collector_network_id: marker.network_entity_id,
                        stack: candidate.stack.clone(),
                    });
                }
            }
        }
    }

    for decision in decisions {
        if let Some(mut picked_up) = world.get_mut::<PickedUpItems>(decision.collector_entity) {
            picked_up.0.push(decision.stack.clone());
        }

        let item_id = RcEntityId(decision.item_entity.to_bits());
        let take_payload = encode_payload(&TakeItemEntity {
            collected_entity_id: stand_in_network_id(item_id),
            collector_entity_id: decision.collector_network_id,
            pickup_item_count: decision.stack.count as i32,
        });
        for entity_ref in world.iter_entities() {
            if let Some(marker) = entity_ref.get::<PlayerMarker>()
                && (marker.network_entity_id == decision.collector_network_id
                    || marker.tracked_entities.contains(&item_id))
            {
                let _ = marker.connection.try_send_payload(take_payload.clone());
            }
        }

        world.despawn(decision.item_entity);
    }
}
