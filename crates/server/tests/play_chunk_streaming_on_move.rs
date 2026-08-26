//! M2 field-report regression test: crossing a chunk boundary must re-center this player's
//! own `SetChunkCacheCenter`/streaming ticket (`rc_scheduler::chunk_ticket::TicketManager::
//! move_player` -- that method's own doc comment: "no production call site exists at M2...
//! exposed for a future mechanics blueprint") and stream every newly-visible chunk that was
//! not already sent at Play-entry -- the fix for the reported "walking 4 chunks in any
//! direction, no new chunks mesh" symptom. Every test constructs its own
//! `HardcodedWorld::new()` -- no test shares state with any other.

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
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // Play-entry's own initial grid is an 11x11 disc (radius 5) centered on chunk
        // (0, 0) -- x in -5..=5. Moving to world x=96 (chunk 6) crosses the boundary at
        // least once and re-centers the disc on chunk (6, 0): x in 1..=11. Chunk x=11 is
        // strictly outside the original grid, so receiving it proves genuinely new content
        // streamed in, not merely a resend of something Play-entry already sent.
        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: 96.0,
                y: -59.0,
                z: 0.0,
                yaw: 0.0,
                pitch: 0.0,
                on_ground: true,
            },
        )
        .await;

        let center = recv_matching::<SetChunkCacheCenter, _>(&mut a, &mut a_acc, |_| true).await;
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
