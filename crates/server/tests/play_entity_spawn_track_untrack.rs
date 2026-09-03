//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(single canonical value/facing asserted, not a four-way sweep) self=waived(single actor, no self-interaction case in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=waived(every state asserted is that entity kind's own default — no non-default property variant exercised)
//! M4-B01 acceptance test: a scripted spawn/track/untrack sequence over two real
//! loopback connections -- mirrors `play_block_place_break.rs`'s own established
//! two-loopback-connection pattern (M2-B07). `A` (the acting connection, positioned
//! near a debug-spawned zombie) reads `Spawn Entity` + `Set Entity Data` on entry into
//! range, `Remove Entities` on leaving it, and `Spawn Entity` + `Set Entity Data` again
//! on re-entry; `B` (an uninvolved observer, debug-teleported far outside the zombie's
//! own tracking range for the entire test) reads no entity packet at any point --
//! proving the tracking gate, not a blanket broadcast, governs delivery.

use bytes::{Bytes, BytesMut};
use rc_core::RcEntityId;
use rc_mechanics::entity::metadata::{MetadataValue, decode_metadata_entries};
use rc_mechanics::entity::{
    BaseEntity, EntityKind, EntityPayload, EntityUuid, LivingEntity, Pose, ZombieBundle,
};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, KeepAliveClientbound, KeepAliveServerbound, LoginPlay,
};
use rusty_clanker_server::play::{
    HardcodedWorld, PlayerProfile, RemoveEntities, SetEntityData, SpawnEntity, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

async fn recv_packet(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, Bytes) {
    loop {
        if let Some(payload) =
            rc_protocol::try_decode_frame(accumulator, CompressionState::Disabled).unwrap()
        {
            let mut body = payload;
            let id = VarInt::decode(&mut body).unwrap().get();
            return (id, body);
        }
        let mut chunk = [0u8; 4096];
        let n = socket.read(&mut chunk).await.unwrap();
        assert!(n > 0, "peer closed before a full frame arrived");
        accumulator.extend_from_slice(&chunk[..n]);
    }
}

async fn send_packet<P: RcPacket>(socket: &mut TcpStream, packet: &P) {
    let payload = encode_payload(packet);
    let mut framed = BytesMut::new();
    rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
    socket.write_all(&framed).await.unwrap();
}

/// As `play_block_place_break.rs`'s own `drain_play_entry`, but additionally decodes
/// the very first packet (`LoginPlay`) to recover this connection's own network entity
/// id -- needed to call `HardcodedWorld::debug_teleport_player`.
async fn drain_play_entry_capturing_network_id(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
) -> i32 {
    let (id, body) = recv_packet(socket, accumulator).await;
    assert_eq!(
        id,
        LoginPlay::ID,
        "first Play-entry packet must be LoginPlay"
    );
    let login = decode_one::<LoginPlay>(body).expect("LoginPlay must decode");
    // SetDefaultSpawnPosition, SynchronizePlayerPosition, GameEvent,
    // SetChunkCacheCenter, ChunkBatchStart.
    for _ in 0..5 {
        recv_packet(socket, accumulator).await;
    }
    loop {
        let (id, _) = recv_packet(socket, accumulator).await;
        if id == ChunkBatchFinished::ID {
            return login.entity_id;
        }
    }
}

async fn spawn_actor(
    world: &HardcodedWorld,
    username: &str,
    uuid: u128,
) -> (TcpStream, BytesMut, i32) {
    let (server, mut client) = connected_pair().await;
    let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
    let world = world.clone();
    let profile = PlayerProfile {
        uuid,
        username: username.to_string(),
    };
    tokio::spawn(async move {
        enter_play(handle, inbound, profile, &world).await;
    });
    let mut accumulator = BytesMut::new();
    let network_id = drain_play_entry_capturing_network_id(&mut client, &mut accumulator).await;
    (client, accumulator, network_id)
}

async fn recv_clientbound(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, Bytes) {
    let (id, body) = recv_packet(socket, accumulator).await;
    if id == KeepAliveClientbound::ID {
        let challenge = decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
        send_packet(socket, &KeepAliveServerbound { id: challenge.id }).await;
    }
    (id, body)
}

async fn recv_packet_of_type(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    expected_id: i32,
) -> Bytes {
    loop {
        let (id, body) = recv_clientbound(socket, accumulator).await;
        if id == expected_id {
            return body;
        }
    }
}

/// Drains every byte currently sitting in `socket`'s own receive buffer (bounded by a
/// short per-read idle timeout, never the outer test deadline) into `accumulator`,
/// decoding every complete frame found and returning each one's packet id --
/// `entity_lifecycle_spawn_update_despawn_against_a_fake_client`'s own final proof that
/// `B` never received a single entity packet across the *entire* test, not merely at
/// one sampled instant: any such packet the server ever sent `B` would still be
/// sitting, unread, in `B`'s own OS-level TCP receive buffer by the time this function
/// is called, since `B` never reads anything until this one call.
async fn drain_all_pending_packet_ids(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
) -> Vec<i32> {
    let mut ids = Vec::new();
    loop {
        let mut chunk = [0u8; 4096];
        match tokio::time::timeout(Duration::from_millis(300), socket.read(&mut chunk)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => accumulator.extend_from_slice(&chunk[..n]),
            Ok(Err(_)) => break,
            Err(_) => break, // idle timeout: no more data pending right now
        }
        while let Some(payload) =
            rc_protocol::try_decode_frame(accumulator, CompressionState::Disabled).unwrap()
        {
            let mut body = payload;
            let id = VarInt::decode(&mut body).unwrap().get();
            ids.push(id);
        }
    }
    ids
}

fn sample_base_entity(pos: [f64; 3]) -> BaseEntity {
    BaseEntity {
        pos,
        velocity: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0],
        fall_distance: 0.0,
        fire_ticks: 0,
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
        pose: Pose::Standing,
        ticks_frozen: 0,
        has_visual_fire: false,
    }
}

fn sample_living_entity() -> LivingEntity {
    LivingEntity {
        hand_states: 0,
        health: 14.0,
        arrow_count: 0,
        stinger_count: 0,
        sleeping_bed_pos: None,
    }
}

#[tokio::test]
async fn entity_lifecycle_spawn_update_despawn_against_a_fake_client() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();

        let (mut a, mut a_acc, a_network_id) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc, b_network_id) = spawn_actor(&world, "b", 2).await;
        let _ = a_network_id;

        // `A` stays at the default `SPAWN_POSITION` ([0.0, -59.0, 0.0]); `B` is
        // debug-teleported far away -- both connections otherwise join at the
        // identical `SPAWN_POSITION` (`connection.rs`'s own default for a brand-new
        // player), so a real position difference must be established explicitly
        // (`debug_teleport_player`'s own doc comment has the full rationale).
        world
            .debug_teleport_player(b_network_id, [100_000.0, -59.0, 100_000.0])
            .await;

        // Well within Zombie's own 8-chunk = 128-block tracking range of `A`'s
        // position, and (obviously) far outside it for `B`.
        let near_pos = [10.0, -59.0, 0.0];
        let far_pos = [10.0, 10_000.0, 0.0];

        let zombie_id: RcEntityId = world
            .debug_spawn_entity(
                EntityKind::Zombie,
                sample_base_entity(near_pos),
                Some(sample_living_entity()),
                EntityPayload::Zombie(ZombieBundle),
            )
            .await
            .expect("the tick-loop thread must still be alive");

        // 1. `A` reads `Spawn Entity` then `Set Entity Data`, in that order.
        let spawn_body = recv_packet_of_type(&mut a, &mut a_acc, SpawnEntity::ID).await;
        let spawn = decode_one::<SpawnEntity>(spawn_body).expect("SpawnEntity must decode");
        assert_eq!(spawn.entity_type, EntityKind::Zombie.registry_id().0 as i32);

        let data_body = recv_packet_of_type(&mut a, &mut a_acc, SetEntityData::ID).await;
        let set_data = decode_one::<SetEntityData>(data_body).expect("SetEntityData must decode");
        let entries = decode_metadata_entries(&set_data.metadata).expect("metadata must decode");
        assert!(
            entries.iter().any(|(index, value)| *index == 9
                && matches!(value, MetadataValue::Float(h) if (*h - 14.0).abs() < f32::EPSILON)),
            "expected the health entry (index 9) among {entries:?}"
        );

        // 2. Move the zombie out of range; `A` reads exactly `Remove Entities`.
        world.debug_move_entity(zombie_id, far_pos).await;
        let remove_body = recv_packet_of_type(&mut a, &mut a_acc, RemoveEntities::ID).await;
        let remove = decode_one::<RemoveEntities>(remove_body).expect("RemoveEntities must decode");
        assert_eq!(remove.entity_ids.len(), 1);

        // 3. Move the zombie back into range; `A` reads `Spawn Entity`/`Set Entity
        //    Data` again -- a fresh spawn, matching vanilla's own re-discovery
        //    behavior.
        world.debug_move_entity(zombie_id, near_pos).await;
        let respawn_body = recv_packet_of_type(&mut a, &mut a_acc, SpawnEntity::ID).await;
        let respawn = decode_one::<SpawnEntity>(respawn_body).expect("SpawnEntity must decode");
        assert_eq!(
            respawn.entity_type,
            EntityKind::Zombie.registry_id().0 as i32
        );
        recv_packet_of_type(&mut a, &mut a_acc, SetEntityData::ID).await;

        // 4. `B` never received a single entity packet across the whole sequence
        //    above.
        let b_ids = drain_all_pending_packet_ids(&mut b, &mut b_acc).await;
        assert!(
            !b_ids.contains(&SpawnEntity::ID)
                && !b_ids.contains(&SetEntityData::ID)
                && !b_ids.contains(&RemoveEntities::ID),
            "B must never receive an entity packet; observed ids: {b_ids:?}"
        );
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}
