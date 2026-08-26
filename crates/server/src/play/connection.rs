//! This blueprint's own entry point into the Play state (M1-B05 blueprint Context,
//! "Assumed hand-off from the connection driver" / "Play-entry clientbound packet
//! sequence -- exact order" / "Inbound Play-state dispatch"). Reachable, and fully
//! exercised, from a bare M1-B01 connection alone -- no dependency on M1-B02/B03/B04's
//! packet catalogs.

use std::time::{Duration, Instant};

use rc_core::BlockPos;
use rc_protocol::{ConnectionState, RawPacket, RcPacket, decode_one, encode_payload};
use tokio::sync::mpsc;

use super::block_action::{BlockActionKind, Face, PendingBlockAction};
use super::chunk;
use super::keepalive::{KeepAliveAction, KeepAliveDriver};
use super::packets::{
    ChunkBatchFinished, ChunkBatchReceived, ChunkBatchStart, ConfirmTeleportation, GameEvent,
    KeepAliveClientbound, KeepAliveServerbound, LevelChunkWithLight, LoginPlay, PlayerAction,
    SetChunkCacheCenter, SetDefaultSpawnPosition, SynchronizePlayerPosition, UseItemOn,
    pack_position, unpack_position,
};
use super::world::{HardcodedWorld, PendingJoin};
use crate::net::ConnectionHandle;

pub struct PlayerProfile {
    pub uuid: u128,
    pub username: String,
}

pub const SPAWN_POSITION: BlockPos = BlockPos::new(0, -59, 0);

/// How often the keep-alive driver is polled while idling in the inbound-dispatch loop.
/// `KeepAliveDriver::on_tick` itself gates on `KEEPALIVE_INTERVAL`, so any poll cadence
/// finer or coarser than exactly 15s never changes observed behavior (Context).
const KEEPALIVE_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// This blueprint's own entry point (Context: "Assumed hand-off"). Sends the full
/// Play-entry sequence, then drives the keep-alive + inbound-dispatch loop for the
/// connection's remaining lifetime (returns only once the connection closes -- spawn this
/// as its own Tokio task; it never blocks the caller beyond that task-spawn point).
pub async fn enter_play(
    handle: ConnectionHandle,
    mut inbound: mpsc::Receiver<RawPacket>,
    profile: PlayerProfile,
    world: &HardcodedWorld,
) {
    handle.set_inbound_state(ConnectionState::Play);
    handle.set_outbound_state(ConnectionState::Play);

    // The wire `entity_id` is allocated once and reused both in `LoginPlay` and in the
    // `PlayerMarker` this connection queues below -- the two must agree so a future
    // mechanics blueprint can resolve "my own entity" consistently.
    let network_entity_id = world.alloc_network_entity_id();

    let login_play = LoginPlay {
        entity_id: network_entity_id,
        is_hardcore: false,
        dimension_names: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        // M1 integration fix, round 5: raised from `2` to `5` alongside `chunk::
        // PLACEHOLDER_RADIUS_CHUNKS` (that constant's own doc comment has the full
        // writeup) -- large enough for a real client's own chunk-cache array to hold the
        // new 11x11 send grid.
        view_distance: 5,
        simulation_distance: 2,
        reduced_debug_info: false,
        enable_respawn_screen: true,
        do_limited_crafting: false,
        dimension_type: 0,
        dimension_name: "minecraft:overworld".to_string(),
        hashed_seed: 0,
        game_mode: 1,
        previous_game_mode: -1,
        is_debug: false,
        is_flat: true,
        has_death_location: false,
        portal_cooldown: 0,
        sea_level: 63,
        // Purely informational to a real client (never gates `Event::Spawn`); this
        // blueprint's own `enter_play` has no route to the real `ServerLoginConfig::
        // online_mode` flag a much earlier connection stage already resolved (threading it
        // through is left to whichever later blueprint wires real Play-state session
        // plumbing) -- `false` matches every automated test and manual-verification path
        // this milestone actually exercises (M1-B04's own scope: "Every test uses
        // ServerLoginConfig{ online_mode: false, .. }").
        online_mode: false,
        enforces_secure_chat: false,
    };
    if handle
        .try_send_payload(encode_payload(&login_play))
        .is_err()
    {
        return;
    }

    let spawn_position = SetDefaultSpawnPosition {
        dimension: "minecraft:overworld".to_string(),
        location: pack_position(SPAWN_POSITION),
        yaw: 0.0,
        pitch: 0.0,
    };
    if handle
        .try_send_payload(encode_payload(&spawn_position))
        .is_err()
    {
        return;
    }

    let sync_position = SynchronizePlayerPosition {
        teleport_id: 1,
        x: SPAWN_POSITION.x as f64,
        y: SPAWN_POSITION.y as f64,
        z: SPAWN_POSITION.z as f64,
        delta_x: 0.0,
        delta_y: 0.0,
        delta_z: 0.0,
        yaw: 0.0,
        pitch: 0.0,
        relative_arguments: 0x00,
    };
    if handle
        .try_send_payload(encode_payload(&sync_position))
        .is_err()
    {
        return;
    }

    let game_event = GameEvent {
        event: 13,
        value: 0.0,
    };
    if handle
        .try_send_payload(encode_payload(&game_event))
        .is_err()
    {
        return;
    }

    let chunk_cache_center = SetChunkCacheCenter {
        chunk_x: 0,
        chunk_z: 0,
    };
    if handle
        .try_send_payload(encode_payload(&chunk_cache_center))
        .is_err()
    {
        return;
    }

    if handle
        .try_send_payload(encode_payload(&ChunkBatchStart {}))
        .is_err()
    {
        return;
    }

    let heightmaps = chunk::build_placeholder_heightmaps();
    let data = chunk::build_placeholder_chunk_data();
    let (
        sky_light_mask,
        block_light_mask,
        empty_sky_light_mask,
        empty_block_light_mask,
        sky_light_arrays,
        block_light_arrays,
    ) = chunk::build_placeholder_light();

    // M1 integration fix, round 5: `batch_size` (below) used to be a literal number kept
    // in sync with `play::chunk::PLACEHOLDER_RADIUS_CHUNKS` by hand across two separate
    // fixes (round 4 alone needed 9 -> 25) -- computed directly from the coordinate list's
    // own real length instead, so it can never drift from what this loop actually sends
    // again, no matter how the radius changes in the future.
    let coords = chunk::placeholder_chunk_coords();
    let chunk_count = coords.len();

    for (chunk_x, chunk_z) in coords {
        let level_chunk = LevelChunkWithLight {
            chunk_x,
            chunk_z,
            heightmaps: heightmaps.clone(),
            data: data.clone(),
            block_entities: Vec::new(),
            sky_light_mask: sky_light_mask.clone(),
            block_light_mask: block_light_mask.clone(),
            empty_sky_light_mask: empty_sky_light_mask.clone(),
            empty_block_light_mask: empty_block_light_mask.clone(),
            sky_light_arrays: sky_light_arrays.clone(),
            block_light_arrays: block_light_arrays.clone(),
        };
        if handle
            .try_send_payload(encode_payload(&level_chunk))
            .is_err()
        {
            return;
        }
    }

    if handle
        .try_send_payload(encode_payload(&ChunkBatchFinished {
            batch_size: chunk_count as i32,
        }))
        .is_err()
    {
        return;
    }

    world.queue_join(PendingJoin {
        network_entity_id,
        username: profile.username.clone(),
        connection: handle.clone(),
    });

    let mut keepalive = KeepAliveDriver::new(Instant::now());
    let mut poll = tokio::time::interval(KEEPALIVE_POLL_INTERVAL);

    loop {
        tokio::select! {
            _ = poll.tick() => {
                match keepalive.on_tick(Instant::now()) {
                    KeepAliveAction::None => {}
                    KeepAliveAction::SendChallenge(id) => {
                        if handle
                            .try_send_payload(encode_payload(&KeepAliveClientbound { id }))
                            .is_err()
                        {
                            return;
                        }
                    }
                    KeepAliveAction::Disconnect(reason) => {
                        tracing::debug!(?reason, "keep-alive timeout; closing connection");
                        handle.close();
                        return;
                    }
                }
            }
            maybe_raw = inbound.recv() => {
                let Some(raw) = maybe_raw else {
                    return;
                };
                dispatch_inbound(raw, &mut keepalive, &handle, world, network_entity_id);
            }
        }
    }
}

/// Recognizes the handful of serverbound Play packets this blueprint's own sequence
/// provokes; every other well-framed serverbound Play packet id is silently dropped,
/// unread (Context: "Inbound Play-state dispatch -- recognize a few, tolerate everything
/// else").
///
/// M2-B07: gains the two block-modifying arms (`PlayerAction`/`UseItemOn`) -- each decodes,
/// builds a `BlockActionKind`, and enqueues a `PendingBlockAction` for the region's own
/// manual Stage-3-equivalent drain step (Context, "Which pipeline stage"). Neither arm
/// validates reach or touches `world`'s chunk state directly -- that happens once per tick,
/// batched, in `HardcodedWorld`'s own tick loop (Context, "Where this check runs,
/// precisely").
fn dispatch_inbound(
    raw: RawPacket,
    keepalive: &mut KeepAliveDriver,
    handle: &ConnectionHandle,
    world: &HardcodedWorld,
    network_entity_id: i32,
) {
    match raw.id {
        ConfirmTeleportation::ID => {
            if let Ok(packet) = decode_one::<ConfirmTeleportation>(raw.body) {
                tracing::trace!(teleport_id = packet.teleport_id, "confirm teleportation");
            }
        }
        KeepAliveServerbound::ID => {
            if let Ok(packet) = decode_one::<KeepAliveServerbound>(raw.body) {
                let _ = keepalive.on_client_response(packet.id);
            }
        }
        ChunkBatchReceived::ID => {
            if let Ok(packet) = decode_one::<ChunkBatchReceived>(raw.body) {
                tracing::trace!(
                    chunks_per_tick = packet.chunks_per_tick,
                    "chunk batch received"
                );
            }
        }
        PlayerAction::ID => {
            if let Ok(packet) = decode_one::<PlayerAction>(raw.body) {
                // Only `status == 0` (StartDestroyBlock) ever turns into a break --
                // creative-mode instant break fires on the start action alone (MECH-D61,
                // Context); `1`/`2` (Abort/StopDestroyBlock) and every other status
                // (`3..=6`) are `Ignored` -- still owed exactly one ack (MECH-D63), never a
                // `Block Update`.
                let kind = match packet.status {
                    0 => BlockActionKind::Break {
                        location: unpack_position(packet.location),
                    },
                    _ => BlockActionKind::Ignored,
                };
                world.queue_block_action(PendingBlockAction {
                    network_entity_id,
                    connection: handle.clone(),
                    kind,
                    sequence: packet.sequence,
                });
            }
        }
        UseItemOn::ID => {
            if let Ok(packet) = decode_one::<UseItemOn>(raw.body) {
                // An out-of-range `face` value is decodable-but-nonsensical input --
                // clamped to a harmless default rather than disconnecting (this project's
                // own established "tolerate everything not explicitly gated" dispatch
                // philosophy, M1-B05's Context).
                let face = Face::from_ordinal(packet.face).unwrap_or(Face::Up);
                let kind = BlockActionKind::Place {
                    location: unpack_position(packet.location),
                    face,
                    inside_block: packet.inside_block,
                };
                world.queue_block_action(PendingBlockAction {
                    network_entity_id,
                    connection: handle.clone(),
                    kind,
                    sequence: packet.sequence,
                });
            }
        }
        other => {
            tracing::trace!(
                id = other,
                "dropping unrecognized Play-state serverbound packet"
            );
        }
    }
}
