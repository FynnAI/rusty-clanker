//! This blueprint's own entry point into the Play state (M1-B05 blueprint Context,
//! "Assumed hand-off from the connection driver" / "Play-entry clientbound packet
//! sequence -- exact order" / "Inbound Play-state dispatch"). Reachable, and fully
//! exercised, from a bare M1-B01 connection alone -- no dependency on M1-B02/B03/B04's
//! packet catalogs.

use std::time::{Duration, Instant};

use rc_core::{BlockPos, DimensionId};
use rc_physics::Vec3;
use rc_protocol::{ConnectionState, RawPacket, RcPacket, decode_one, encode_payload};
use tokio::sync::mpsc;

use super::block_action::{BlockActionKind, Face, PendingBlockAction};
use super::chunk;
use super::keepalive::{KeepAliveAction, KeepAliveDriver};
use super::movement::{PendingMoveReport, PendingMovementPacket, feet_block_pos};
use super::packets::{
    ChunkBatchFinished, ChunkBatchReceived, ChunkBatchStart, ConfirmTeleportation, GameEvent,
    KeepAliveClientbound, KeepAliveServerbound, LevelChunkWithLight, LoginPlay, PlayerAction,
    SetChunkCacheCenter, SetDefaultSpawnPosition, SetHealth, SetPlayerMovementFlags,
    SetPlayerPosition, SetPlayerPositionAndRotation, SetPlayerRotation, SynchronizePlayerPosition,
    UseItemOn, pack_position, unpack_position,
};
use super::persistence::PlayerSessionStore;
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

    // M2 integration addition: player persistence (M2-B06's own "Composition-root
    // integration" recipe, restated here) -- load (or freshly default) this player's
    // saved record before building any position/health-carrying Play-entry packet.
    // `_save_guard`'s own doc comment (below) is what actually guarantees the record is
    // saved back on every exit from this function from this point onward.
    let uuid = uuid::Uuid::from_u128(profile.uuid);
    let sessions = world.player_sessions();
    let (pos, rotation) = match sessions.load_or_create(
        uuid,
        DimensionId::OVERWORLD,
        [
            SPAWN_POSITION.x as f64,
            SPAWN_POSITION.y as f64,
            SPAWN_POSITION.z as f64,
        ],
    ) {
        Ok(pair) => pair,
        Err(err) => {
            tracing::error!(%uuid, error = %err, "failed to load player data; disconnecting");
            return;
        }
    };
    let (health, food_level, food_saturation_level) = sessions
        .with_record_mut(uuid, |record| {
            (
                record.data.health,
                record.data.food_level,
                record.data.food_saturation_level,
            )
        })
        .unwrap_or((20.0, 20, 5.0));
    // Guarantees `sessions.save_and_remove(uuid)` runs on every exit from this function
    // from this point onward -- every one of `enter_play`'s many early `return`s on send
    // failure, the keep-alive-timeout `return`, and the loop's own natural `inbound.recv()
    // == None` exit (M2-B06's own "at every exit path of the connection's own driving
    // loop" requirement) -- without needing to edit every one of those existing return
    // sites individually.
    struct SaveOnDisconnect {
        sessions: PlayerSessionStore,
        uuid: uuid::Uuid,
    }
    impl Drop for SaveOnDisconnect {
        fn drop(&mut self) {
            if let Err(err) = self.sessions.save_and_remove(self.uuid) {
                tracing::warn!(
                    uuid = %self.uuid,
                    error = %err,
                    "failed to save player data on disconnect"
                );
            }
        }
    }
    let _save_guard = SaveOnDisconnect {
        sessions: sessions.clone(),
        uuid,
    };

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

    // M2 integration addition: `pos`/`rotation` (above) are the just-loaded (or freshly
    // defaulted) player's own persisted position/rotation, replacing the hardcoded
    // `SPAWN_POSITION`/`0.0` literals -- AC1d's own "player rejoins at the position they
    // left at" assertion.
    let sync_position = SynchronizePlayerPosition {
        teleport_id: 1,
        x: pos[0],
        y: pos[1],
        z: pos[2],
        delta_x: 0.0,
        delta_y: 0.0,
        delta_z: 0.0,
        yaw: rotation[0],
        pitch: rotation[1],
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

    // M2 integration addition: sent here, immediately after `GameEvent` and before any
    // chunk data -- deliberately NOT after `ChunkBatchFinished` (this project's own first
    // attempt at this placement broke every `drain_play_entry`-style acceptance test
    // that reads exactly through `ChunkBatchFinished` then asserts the very next packet
    // is a specific response, e.g. `play_reach_validation.rs`'s own `assert_eq!(id,
    // AcknowledgeBlockChange::ID)` immediately after `spawn_actor` returns -- a leftover
    // `SetHealth` sitting unread past `ChunkBatchFinished` was consumed by that next
    // `recv_packet` call instead, a real regression confirmed by a real `cargo nextest`
    // run before this placement was corrected). `play_chunk_set.rs`'s own strict,
    // packet-by-packet Play-entry assertion needed a matching test-authoring fix for
    // this exact insertion point (see that commit).
    let set_health = SetHealth {
        health,
        food: food_level,
        saturation: food_saturation_level,
    };
    if handle
        .try_send_payload(encode_payload(&set_health))
        .is_err()
    {
        return;
    }

    // M2 integration addition: the joining player's own chunk, derived from their real
    // loaded/defaulted `pos` (above) instead of a hardcoded `(0, 0)` literal.
    //
    // M2 field-report fix: uses `feet_block_pos` (floor on every axis) rather than a plain
    // `as i32` truncation -- the two disagree for a negative-fractional `pos` (e.g.
    // `x == -0.5` truncates to chunk `0`, but floors to the correct chunk `-1`), which
    // matters now that a rejoining player's own persisted `pos` can carry a genuine
    // fractional part (this same fix's own movement-application/persistence consumer,
    // `world.rs`'s own per-tick chunk-crossing detection uses the identical helper so the
    // two can never disagree about which chunk a given live position belongs to).
    let player_chunk = feet_block_pos(pos).chunk_key(DimensionId::OVERWORLD);

    let chunk_cache_center = SetChunkCacheCenter {
        chunk_x: player_chunk.x,
        chunk_z: player_chunk.z,
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
    //
    // M2 integration addition: coordinates are now absolute, centered on the joining
    // player's own chunk (`player_chunk`, above) rather than always on world origin --
    // identical to the old behavior whenever `player_chunk == (0, 0)` (every case any
    // currently-committed test exercises, M2's own no-movement-mechanics scope), but
    // correct in general.
    let coords: Vec<(i32, i32)> = chunk::placeholder_chunk_coords()
        .into_iter()
        .map(|(dx, dz)| (player_chunk.x + dx, player_chunk.z + dz))
        .collect();
    let chunk_count = coords.len();

    // M2 integration addition: real, storage-backed chunk content (`M2-COMPLETION-
    // REPORT.md`'s own diagnosed gap) -- registers a real ticket at this player's own
    // chunk and waits for every one of `coords`' columns to actually become resident
    // (superflat-filled or disk-restored, M2-B05's own async load path), returning each
    // one's real, currently-live block/biome content already wire-encoded. Replaces
    // `chunk::build_placeholder_chunk_data()`'s static, always-identical blob -- block
    // changes M2-B07 already applies to live chunk storage are now visible in a freshly
    // sent chunk (AC1b's own assertion).
    let encoded_data = world
        .request_chunk_grid(
            network_entity_id,
            player_chunk,
            chunk::PLACEHOLDER_RADIUS_CHUNKS as u8,
            coords.clone(),
        )
        .await;

    for ((chunk_x, chunk_z), data) in coords.into_iter().zip(encoded_data) {
        let level_chunk = LevelChunkWithLight {
            chunk_x,
            chunk_z,
            heightmaps: heightmaps.clone(),
            data,
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
        // M2 field-report fix: this player's own real, just-loaded (or freshly defaulted)
        // uuid/position/rotation -- previously lost the moment `PendingJoin` crossed this
        // channel boundary, forcing the tick-loop's own `PlayerMarker` (before this fix,
        // one that did not even carry a position field at all) and its own ticket
        // registration to fall back on the hardcoded `SPAWN_POSITION` unconditionally.
        uuid,
        position: pos,
        rotation,
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
                // M3-B02: preserves M1-B05's own accept-and-log behavior for this packet
                // unchanged, additionally queuing it for the region's own per-tick
                // `evaluate_movement` step (Context: "Teleport / position-sync protocol" --
                // "On ConfirmTeleportation{teleport_id}").
                world.queue_movement_packet(PendingMovementPacket {
                    network_entity_id,
                    report: PendingMoveReport {
                        confirm_teleport_id: Some(packet.teleport_id),
                        ..Default::default()
                    },
                });
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
        // M3-B02: the four serverbound movement packets, each decoded and enqueued as a
        // `PendingMovementPacket` for the region's own per-tick `evaluate_movement` step
        // (`world.rs`'s own tick loop, Context: "Which pipeline stage") -- never applies
        // anything here directly, matching `PlayerAction`/`UseItemOn`'s own established
        // "decode and enqueue, apply once per tick" pattern below. Superseded the M2
        // field-report fix's own minimal decode-and-apply path (`play::movement`'s own
        // module doc comment has the full prior root-cause writeup for why these ids were
        // ever unrecognized in the first place).
        SetPlayerPosition::ID => {
            if let Ok(packet) = decode_one::<SetPlayerPosition>(raw.body) {
                world.queue_movement_packet(PendingMovementPacket {
                    network_entity_id,
                    report: PendingMoveReport {
                        position: Some(Vec3::new(packet.x, packet.y, packet.z)),
                        on_ground: Some(packet.on_ground),
                        ..Default::default()
                    },
                });
            }
        }
        SetPlayerPositionAndRotation::ID => {
            if let Ok(packet) = decode_one::<SetPlayerPositionAndRotation>(raw.body) {
                world.queue_movement_packet(PendingMovementPacket {
                    network_entity_id,
                    report: PendingMoveReport {
                        position: Some(Vec3::new(packet.x, packet.y, packet.z)),
                        rotation: Some((packet.yaw, packet.pitch)),
                        on_ground: Some(packet.on_ground),
                        confirm_teleport_id: None,
                    },
                });
            }
        }
        SetPlayerRotation::ID => {
            if let Ok(packet) = decode_one::<SetPlayerRotation>(raw.body) {
                world.queue_movement_packet(PendingMovementPacket {
                    network_entity_id,
                    report: PendingMoveReport {
                        rotation: Some((packet.yaw, packet.pitch)),
                        on_ground: Some(packet.on_ground),
                        ..Default::default()
                    },
                });
            }
        }
        SetPlayerMovementFlags::ID => {
            if let Ok(packet) = decode_one::<SetPlayerMovementFlags>(raw.body) {
                world.queue_movement_packet(PendingMovementPacket {
                    network_entity_id,
                    report: PendingMoveReport {
                        on_ground: Some(packet.on_ground),
                        ..Default::default()
                    },
                });
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
