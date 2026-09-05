//! Player-side cross-region transfer: `PlayerTransferPayload`, `PlayerRouting`'s
//! connection-redirect mechanism, and the combined `EntityArrivalDriver` `TwoRegionWorld`
//! registers (M4-B08 Context, Part 1.3/1.5).

use std::collections::HashMap;
use std::sync::Arc;

use bevy_ecs::prelude::Resource;
use parking_lot::{Mutex, RwLock};
use rc_core::{ChunkKey, RcEntityId};
use rc_messaging::EntitySnapshot;
use tokio::sync::mpsc::UnboundedSender;

use crate::net::ConnectionHandle;

pub const TRANSFER_PAYLOAD_KIND_PLAYER: u8 = 1;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlayerTransferPayload {
    pub uuid: u128,
    pub username: String,
    pub network_entity_id: i32,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub fall_distance: f64,
    /// `RcEntityId`'s raw `u64` values of every entity this player was tracking
    /// (M4-B01's `PlayerMarker.tracked_entities`) at the moment of transfer — carried so
    /// the destination region's own tracking pass does not immediately re-send `Spawn
    /// Entity` for something the client already has rendered; entities the player can no
    /// longer see (left the new region's own tracking range) are naturally dropped by the
    /// destination region's very next ordinary tracking pass (M4-B01, unchanged).
    pub tracked_entities: Vec<u64>,
}

/// Builds a player `EntitySnapshot` (Context, Part 1.5). `entity_id` is the player's own
/// `RcEntityId`.
pub fn build_player_entity_snapshot(
    entity_id: RcEntityId,
    source_chunk: ChunkKey,
    payload: &PlayerTransferPayload,
) -> EntitySnapshot {
    let mut component_data = vec![TRANSFER_PAYLOAD_KIND_PLAYER];
    component_data.extend(
        postcard::to_allocvec(payload)
            .expect("PlayerTransferPayload is always postcard-serializable"),
    );
    EntitySnapshot {
        entity_id,
        source_chunk,
        component_data,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlayerTransferDecodeError {
    #[error("postcard decode of the player-transfer payload failed: {0}")]
    Payload(String),
}

/// `None` if the leading byte is not `TRANSFER_PAYLOAD_KIND_PLAYER`; `Some(Err(...))` for
/// malformed remaining bytes (never a panic).
pub fn try_decode_player_snapshot(
    component_data: &[u8],
) -> Option<Result<PlayerTransferPayload, PlayerTransferDecodeError>> {
    let (&kind_byte, rest) = component_data.split_first()?;
    if kind_byte != TRANSFER_PAYLOAD_KIND_PLAYER {
        return None;
    }
    match postcard::from_bytes(rest) {
        Ok(payload) => Some(Ok(payload)),
        Err(err) => Some(Err(PlayerTransferDecodeError::Payload(err.to_string()))),
    }
}

/// One region's own set of `enter_play`-facing inbound queues — a plain bundle of the
/// `UnboundedSender` halves `HardcodedWorld`'s own established pattern already uses per
/// region; `Clone` (every field is a `Clone`-able `UnboundedSender`).
#[derive(Clone)]
pub struct RegionQueueHandles {
    pub block_action_tx: UnboundedSender<crate::play::block_action::PendingBlockAction>,
    pub movement_tx: UnboundedSender<crate::play::movement::PendingMovementPacket>,
}

/// One player's currently-live set of per-region inbound queue senders (Context, Part
/// 1.5). `Arc<parking_lot::RwLock<...>>`-guarded so the owning connection's async task and
/// *both* regions' own tick-loop threads can read/replace it without a channel round-trip
/// — the redirect is a plain, uncontended shared-memory update.
pub struct PlayerRouting {
    current: RwLock<RegionQueueHandles>,
}

impl PlayerRouting {
    pub fn new(initial: RegionQueueHandles) -> Self {
        Self {
            current: RwLock::new(initial),
        }
    }

    /// Read the currently-live queue set (cloned — cheap, every field is a `Sender`).
    /// Called by `enter_play`'s own inbound-dispatch loop on every decoded packet, so a
    /// mid-flight redirect takes effect on the very next packet, never a stale one already
    /// in a local variable.
    pub fn current(&self) -> RegionQueueHandles {
        self.current.read().clone()
    }

    /// Called by the *source* region's own crossing-detection system, at the exact moment
    /// it decides to transfer this player — before, or atomically with, its own
    /// `RegionMessageOutbox::send` call for the same entity.
    pub fn redirect_to(&self, new_target: RegionQueueHandles) {
        *self.current.write() = new_target;
    }
}

/// This blueprint's own resolution of the one gap `PlayerTransferPayload` deliberately
/// leaves open (Context, Part 1.5: `ConnectionHandle` cannot travel through a
/// `serde`-serialized wire payload): the *destination* region's own arrival driver needs
/// both a live `ConnectionHandle` (to resume sending this player packets) and this
/// player's own `Arc<PlayerRouting>` handle (to attach to the freshly-spawned
/// `PlayerMarker`) — neither of which the wire payload carries. `TwoRegionWorld`'s own
/// join path (`queue_join`) populates one entry per connected player, keyed by uuid;
/// `combined_arrival_driver` reads it back on arrival. A process-wide, `uuid`-keyed side
/// table, mirroring `SharedNetworkEntityIdAllocator`'s own "one shared `Arc`, inserted
/// into every region's own `World`" precedent.
#[derive(Clone)]
pub struct PlayerConnectionState {
    pub connection: ConnectionHandle,
    pub routing: Arc<PlayerRouting>,
}

#[derive(Resource, Clone, Default)]
pub struct PlayerRoutingRedirectTable(pub Arc<Mutex<HashMap<u128, PlayerConnectionState>>>);

impl PlayerRoutingRedirectTable {
    pub fn insert(&self, uuid: u128, state: PlayerConnectionState) {
        self.0.lock().insert(uuid, state);
    }

    pub fn get(&self, uuid: u128) -> Option<PlayerConnectionState> {
        self.0.lock().get(&uuid).cloned()
    }

    pub fn remove(&self, uuid: u128) {
        self.0.lock().remove(&uuid);
    }
}

/// The `EntityArrivalDriver` `TwoRegionWorld` registers on its single shared
/// `RcExecutorBuilder` (Context, Part 1.3): tries
/// `rc_mechanics::entity::try_decode_mob_snapshot` first (via the leading discriminator
/// byte, checked directly here to avoid decoding twice); on a mob-kind byte, delegates the
/// whole batch of mob-kind arrivals to `rc_mechanics::entity::mob_arrival_driver` unchanged;
/// on `TRANSFER_PAYLOAD_KIND_PLAYER`, decodes via `try_decode_player_snapshot` and spawns
/// `(PlayerMarker { .. }, PlayerMotion { .. }, TeleportState { .. })` — `routing`/
/// `connection` are re-attached immediately from `PlayerRoutingRedirectTable`, keyed by
/// this player's own `uuid` (Context: "the destination region does not yet know this
/// player's own `PlayerRouting` handle... responsible for re-attaching it").
pub fn combined_arrival_driver(world: &mut bevy_ecs::world::World, arrivals: Vec<EntitySnapshot>) {
    let mut mob_arrivals = Vec::new();
    let mut player_payloads = Vec::new();

    for snapshot in arrivals {
        match snapshot.component_data.first().copied() {
            Some(rc_mechanics::entity::TRANSFER_PAYLOAD_KIND_MOB) => mob_arrivals.push(snapshot),
            Some(TRANSFER_PAYLOAD_KIND_PLAYER) => {
                if let Some(Ok(payload)) = try_decode_player_snapshot(&snapshot.component_data) {
                    player_payloads.push(payload);
                }
                // A malformed player envelope, or a decode failure, is silently skipped --
                // never a panic (Context's own "never a panic" discipline).
            }
            _ => {
                // An unrecognized leading byte is silently skipped -- a future blueprint
                // adding a third transferable-entity family extends this table.
            }
        }
    }

    if !mob_arrivals.is_empty() {
        rc_mechanics::entity::mob_arrival_driver(world, mob_arrivals);
    }

    for payload in player_payloads {
        spawn_arrived_player(world, payload);
    }
}

fn spawn_arrived_player(world: &mut bevy_ecs::world::World, payload: PlayerTransferPayload) {
    let Some(state) = world
        .get_resource::<PlayerRoutingRedirectTable>()
        .and_then(|table| table.get(payload.uuid))
    else {
        // No connection/routing handle registered for this uuid -- this player's own
        // connection must already be gone (or this is a test that never registered one);
        // there is nothing this driver can usefully spawn without a live
        // `ConnectionHandle`, so the arrival is dropped rather than spawning a
        // permanently-unreachable `PlayerMarker`.
        return;
    };

    let dimension = rc_core::DimensionId::OVERWORLD;
    let last_streamed_center =
        crate::play::movement::feet_block_pos(payload.position).chunk_key(dimension);
    let tracked_entities: std::collections::HashSet<RcEntityId> = payload
        .tracked_entities
        .iter()
        .map(|&raw| RcEntityId(raw))
        .collect();

    let marker = crate::play::world::PlayerMarker {
        network_entity_id: payload.network_entity_id,
        username: payload.username,
        connection: state.connection,
        uuid: uuid::Uuid::from_u128(payload.uuid),
        position: payload.position,
        rotation: [payload.yaw, payload.pitch],
        on_ground: payload.on_ground,
        last_streamed_center,
        sent_chunks: std::collections::HashSet::new(),
        tracked_entities,
        last_sent_entity_state: HashMap::new(),
        routing: Some(state.routing),
    };
    let motion = crate::play::movement::PlayerMotion {
        position: rc_physics::Vec3::new(
            payload.position[0],
            payload.position[1],
            payload.position[2],
        ),
        velocity: rc_physics::Vec3::new(
            payload.velocity[0],
            payload.velocity[1],
            payload.velocity[2],
        ),
        yaw: payload.yaw,
        pitch: payload.pitch,
        on_ground: payload.on_ground,
        fall_distance: payload.fall_distance,
    };
    let teleport = crate::play::movement::TeleportState {
        awaiting_teleport_id: None,
        next_teleport_id: 2,
    };

    world.spawn((marker, motion, teleport));
}
