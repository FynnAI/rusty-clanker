//! The production adapter around `rc-mechanics`' pure `compute_tracking_delta`
//! (M4-B01, Context: "The production integration"). Called once per `PlayerMarker` per
//! tick, **after** the block-action drain-and-apply step and **before**
//! `executor.tick_region(...)` — mirroring M2-B07/M3-B02's own established manual-step
//! placement.

use bytes::{BufMut, BytesMut};
use rc_core::RcEntityId;
use rc_mechanics::entity::{
    BaseEntity, EntityKind, EntityMetadataFields, EntityPayload, LivingEntity,
    compute_tracking_delta,
};
use rc_protocol::{VarInt, encode_payload};

use crate::play::entity_packets::{
    LpVec3, RemoveEntities, SetEntityData, SpawnEntity, encode_angle, encode_metadata_value,
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
