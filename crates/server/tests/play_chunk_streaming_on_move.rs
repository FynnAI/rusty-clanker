//! M2 field-report regression test: crossing a chunk boundary must re-center this player's
//! own `SetChunkCacheCenter`/streaming ticket (`rc_scheduler::chunk_ticket::TicketManager::
//! move_player` -- that method's own doc comment: "no production call site exists at M2...
//! exposed for a future mechanics blueprint") and stream every newly-visible chunk that was
//! not already sent at Play-entry -- the fix for the reported "walking 4 chunks in any
//! direction, no new chunks mesh" symptom. Every test constructs its own
//! `HardcodedWorld::new()` -- no test shares state with any other.
//!
//! M3-B02 test-authoring fix: the single 96-block `SetPlayerPositionAndRotation` jump this
//! test originally sent is no longer legal under M3-B02's own server-authoritative speed
//! check (`SPEED_CHECK_THRESHOLD = 100.0` blocks^2 per tick, `evaluate_movement`) -- it is
//! now rejected with a teleport correction instead of applied, so the `SetChunkCacheCenter`
//! this test waits for never arrives. Restated as a sequence of 8-block steps (`8^2 = 64 <=
//! 100`, comfortably under the per-tick budget even as `PlayerMotion.velocity` -- this
//! test's own repeated-identical-step walk keeps "expected" and "moved" essentially equal
//! every step) -- `wait_until` (mirroring `play_movement_application.rs`'s own established
//! pattern) confirms each step actually landed before the next is sent, so the walk cannot
//! outrun the region's own per-tick movement evaluation.

use bytes::{Bytes, BytesMut};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, KeepAliveClientbound, KeepAliveServerbound, LevelChunkWithLight,
    SetChunkCacheCenter, SetPlayerPositionAndRotation,
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
        recv_packet(socket, accumulator).await;
    }
    loop {
        let (id, _) = recv_packet(socket, accumulator).await;
        if id == ChunkBatchFinished::ID {
            return;
        }
    }
}

async fn spawn_actor(world: &HardcodedWorld, username: &str, uuid: u128) -> (TcpStream, BytesMut) {
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
    drain_play_entry(&mut client, &mut accumulator).await;
    (client, accumulator)
}

/// Scans until a packet of `expected_id` satisfies `matches`, discarding everything else
/// (including other packets of the same id that don't match) -- necessary here because
/// multiple `LevelChunkWithLight` packets legally arrive in a row and only one specific
/// coordinate is under test. Bounded only by the surrounding test's own outer
/// `tokio::time::timeout` (`play_reach_validation.rs`'s own established convention: no
/// second, independent inner deadline).
/// As `play_movement_application.rs`'s own identical helper -- polls `check` until it
/// returns `true`, bounded only by the surrounding test's own outer `tokio::time::timeout`.
async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn recv_matching<T: RcPacket, F: Fn(&T) -> bool>(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    matches: F,
) -> T {
    loop {
        let (id, body) = recv_clientbound(socket, accumulator).await;
        if id == T::ID
            && let Ok(packet) = decode_one::<T>(body)
            && matches(&packet)
        {
            return packet;
        }
    }
}

#[tokio::test]
async fn crossing_a_chunk_boundary_streams_newly_visible_chunks_and_updates_cache_center() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let sessions = world.player_sessions();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // Play-entry's own initial grid is an 11x11 disc (radius 5) centered on chunk
        // (0, 0) -- x in -5..=5. Walking to world x=96 (chunk 6) in 8-block steps (M3-B02's
        // own speed-check budget) crosses the boundary at least once and re-centers the
        // disc on chunk (6, 0): x in 1..=11. Chunk x=11 is strictly outside the original
        // grid, so receiving it proves genuinely new content streamed in, not merely a
        // resend of something Play-entry already sent.
        let mut x = 0.0_f64;
        while x < 96.0 {
            x += 8.0;
            send_packet(
                &mut a,
                &SetPlayerPositionAndRotation {
                    x,
                    y: -59.0,
                    z: 0.0,
                    yaw: 0.0,
                    pitch: 0.0,
                    on_ground: true,
                },
            )
            .await;
            wait_until(|| sessions.with_record_mut(uuid, |r| r.data.pos) == Some([x, -59.0, 0.0]))
                .await;
        }

        // The walk crosses several chunk boundaries en route (one `SetChunkCacheCenter` per
        // crossing) -- `recv_matching` scans past every earlier one to the final center,
        // exactly as its own doc comment describes for `LevelChunkWithLight` below.
        let center = recv_matching::<SetChunkCacheCenter, _>(&mut a, &mut a_acc, |c| {
            (c.chunk_x, c.chunk_z) == (6, 0)
        })
        .await;
        assert_eq!((center.chunk_x, center.chunk_z), (6, 0));

        let chunk = recv_matching::<LevelChunkWithLight, _>(&mut a, &mut a_acc, |c| {
            (c.chunk_x, c.chunk_z) == (11, 0)
        })
        .await;
        assert_eq!((chunk.chunk_x, chunk.chunk_z), (11, 0));
    })
    .await
    .unwrap();
}
