//! M2 field-report regression test: a real serverbound movement packet's claimed
//! position/rotation must survive a disconnect and a genuine rejoin -- the fix for the
//! reported "player position is not persisted across rejoin -- always back at spawn"
//! symptom. Before this fix nothing ever wrote a live position into `PlayerSessionStore`
//! (the session record's `pos`/`rotation` stayed at whatever `load_or_create` produced at
//! join time forever), so `SaveOnDisconnect`'s own disconnect-time save
//! (`play::connection`) always persisted the untouched join-time value.
//!
//! M3-B02 test-authoring fix: the single `SetPlayerPositionAndRotation` packet this test
//! originally sent claimed a ~32.75-block move in one tick -- legal under M2's own raw
//! decode-and-apply movement path, but well past M3-B02's own server-authoritative speed
//! check (`SPEED_CHECK_THRESHOLD = 100.0` blocks^2 per tick, `evaluate_movement`), which now
//! rejects it with a teleport correction instead of applying it. Restated as a short walk
//! of six equal steps along the straight line from `SPAWN_POSITION` to the same final
//! target (each step's own squared length is ~29.8, comfortably under the per-tick budget),
//! confirming each step landed (`wait_until`, mirroring `play_movement_application.rs`'s
//! own established pattern) before sending the next -- the final step's own claimed
//! position is `(12.5, -58.0, -30.25)` exactly, unchanged from this test's original target.

use bytes::{Bytes, BytesMut};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, KeepAliveClientbound, KeepAliveServerbound, LoginPlay,
    SetDefaultSpawnPosition, SetPlayerPositionAndRotation, SynchronizePlayerPosition,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
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

async fn recv_clientbound(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, Bytes) {
    let (id, body) = recv_packet(socket, accumulator).await;
    if id == KeepAliveClientbound::ID {
        let challenge = decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
        send_packet(socket, &KeepAliveServerbound { id: challenge.id }).await;
    }
    (id, body)
}

async fn drain_play_entry(socket: &mut TcpStream, accumulator: &mut BytesMut) {
    for _ in 0..6 {
        recv_clientbound(socket, accumulator).await;
    }
    loop {
        let (id, _) = recv_clientbound(socket, accumulator).await;
        if id == ChunkBatchFinished::ID {
            return;
        }
    }
}

/// As `play_movement_application.rs`'s own identical helper.
async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn position_and_rotation_persist_across_a_real_disconnect_and_rejoin() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_raw: u128 = 9001;
        let uuid = uuid::Uuid::from_u128(uuid_raw);
        let sessions = world.player_sessions();

        // First session: join, move, then disconnect.
        {
            let (server, mut client) = connected_pair().await;
            let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
            let world_for_task = world.clone();
            let profile = PlayerProfile {
                uuid: uuid_raw,
                username: "a".to_string(),
            };
            tokio::spawn(async move {
                enter_play(handle, inbound, profile, &world_for_task).await;
            });

            let mut acc = BytesMut::new();
            drain_play_entry(&mut client, &mut acc).await;

            // Walks the straight line from `SPAWN_POSITION` (0, -59, 0) to the target
            // (12.5, -58, -30.25) in six equal, speed-check-legal steps (M3-B02 test-
            // authoring fix, module doc comment) rather than one large jump.
            const STEPS: u32 = 6;
            const TARGET: (f64, f64, f64) = (12.5, -58.0, -30.25);
            for step in 1..=STEPS {
                let t = step as f64 / STEPS as f64;
                let pos = (TARGET.0 * t, -59.0 + (TARGET.1 - -59.0) * t, TARGET.2 * t);
                send_packet(
                    &mut client,
                    &SetPlayerPositionAndRotation {
                        x: pos.0,
                        y: pos.1,
                        z: pos.2,
                        yaw: 91.0,
                        pitch: -12.0,
                        on_ground: true,
                    },
                )
                .await;
                wait_until(|| {
                    sessions.with_record_mut(uuid, |r| r.data.pos) == Some([pos.0, pos.1, pos.2])
                })
                .await;
            }
            assert_eq!(
                sessions.with_record_mut(uuid, |r| r.data.rotation),
                Some([91.0, -12.0])
            );

            // A real disconnect: dropping the client socket closes the connection, which
            // (once the server's own reader task notices EOF) makes `enter_play`'s loop
            // exit and its `SaveOnDisconnect` guard fire -- `save_and_remove` both persists
            // this record and removes it from the live session set atomically, so polling
            // for its removal from the live set is a deterministic, race-free signal that
            // the disconnect-time save has actually completed.
            drop(client);
            wait_until(|| sessions.with_record_mut(uuid, |_| ()).is_none()).await;
        }

        // Second session: a genuine rejoin. `SynchronizePlayerPosition` must reflect the
        // real, disk-persisted position/rotation from the first session, not the hardcoded
        // `SPAWN_POSITION`/`0.0` defaults.
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let world_for_task = world.clone();
        let profile = PlayerProfile {
            uuid: uuid_raw,
            username: "a".to_string(),
        };
        tokio::spawn(async move {
            enter_play(handle, inbound, profile, &world_for_task).await;
        });

        let mut acc = BytesMut::new();
        let (login_id, _) = recv_packet(&mut client, &mut acc).await;
        assert_eq!(login_id, LoginPlay::ID);
        let (spawn_id, _) = recv_packet(&mut client, &mut acc).await;
        assert_eq!(spawn_id, SetDefaultSpawnPosition::ID);
        let (sync_id, sync_body) = recv_packet(&mut client, &mut acc).await;
        assert_eq!(sync_id, SynchronizePlayerPosition::ID);

        let sync = decode_one::<SynchronizePlayerPosition>(sync_body).unwrap();
        assert_eq!(sync.x, 12.5);
        assert_eq!(sync.y, -58.0);
        assert_eq!(sync.z, -30.25);
        assert_eq!(sync.yaw, 91.0);
        assert_eq!(sync.pitch, -12.0);
    })
    .await
    .unwrap();
}
