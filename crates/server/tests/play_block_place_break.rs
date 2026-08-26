//! M2-B07 acceptance test: a scripted place/break exchange over two real loopback
//! connections -- the actor (`A`) breaks then places a block, an uninvolved observer
//! (`B`) receives the identical broadcast `Block Update` for each change (Context: "The
//! M1-B05 interest/broadcast seam does not exist -- resolved here"), and the resulting
//! world state is queryable back out, dirty-marked -- criterion 1's own "persisted state"
//! half (in-memory, dirty-marked; the on-disk half is a separate, not-yet-written
//! blueprint's job).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, UseItemOn, pack_position,
};
use rusty_clanker_server::play::{DebugBlockInfo, HardcodedWorld, PlayerProfile, enter_play};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

/// Reads exactly one framed, uncompressed payload off `socket`, splits its leading
/// packet-id `VarInt` from the body, and returns both.
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

/// Drains this connection's own Play-entry clientbound sequence (M1-B05's `enter_play`, up
/// through and including `ChunkBatchFinished`) without decoding or asserting its content --
/// `play_chunk_set.rs` already owns that. Robust to the current chunk-send radius (does not
/// hardcode a chunk count).
async fn drain_play_entry(socket: &mut TcpStream, accumulator: &mut BytesMut) {
    // LoginPlay, SetDefaultSpawnPosition, SynchronizePlayerPosition, GameEvent,
    // SetChunkCacheCenter, ChunkBatchStart.
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

/// Reads the next clientbound packet like `recv_packet`, but transparently answers any
/// `KeepAliveClientbound` challenge it encounters with the matching `KeepAliveServerbound`
/// reply before returning it -- exactly what a real vanilla client does. Necessary for
/// correctness, not just politeness: the scan below is willing to wait as long as the
/// surrounding test's own outer deadline (up to 60s) allows, comfortably longer than the
/// server's own 15s `KEEPALIVE_INTERVAL` under heavy contention -- a scan that merely
/// skipped-and-discarded a keep-alive challenge instead of answering it would let
/// `KeepAliveDriver`'s own timeout close the connection out from under it.
async fn recv_clientbound(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, Bytes) {
    let (id, body) = recv_packet(socket, accumulator).await;
    if id == KeepAliveClientbound::ID {
        let challenge = decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
        send_packet(socket, &KeepAliveServerbound { id: challenge.id }).await;
    }
    (id, body)
}

/// Scans the clientbound stream for the next packet of `expected_id`, skipping (and
/// discarding) any other packet along the way. Bounded only by the surrounding
/// `#[tokio::test]`'s own outer `tokio::time::timeout` -- deliberately no second,
/// independent deadline here: an inner deadline shorter than that outer one would misfire
/// under exactly the kind of heavy, legitimate contention the outer 60s budget already
/// exists to tolerate (confirmed directly: an earlier draft of this helper with its own
/// tighter 30s deadline produced a new, synthetic-contention-only failure that also
/// reproduced on an untouched sibling test).
///
/// Interleaving contract: since the M2 live-chunk-streaming wiring, unrelated clientbound
/// packets (late chunk data, `SetHealth`, keep-alives, ...) may legally arrive interleaved
/// with the response packets under test -- a vanilla client tolerates arbitrary
/// interleaving, so this scan does too. Safe to call back-to-back for two different
/// expected ids on the same connection (as this file's own test does, for an actor's own
/// ack then its own resulting update) because `respond_to_action` always emits a given
/// action's own `AcknowledgeBlockChange` before its own `BlockUpdate`, in that
/// single-threaded order -- only *other* traffic can land in between, never those two out
/// of order.
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

#[tokio::test]
async fn break_and_place_broadcast_and_persist() {
    // M2 integration test-authoring fix: raised from `20` -- `enter_play` now awaits a
    // real, ticket-driven `RC-IoPool` chunk-grid load per join (`connection.rs`'s own
    // `request_chunk_grid` call), a genuinely asynchronous round trip absent when this
    // budget was first tuned against the old, instantly-synthesized placeholder chunk
    // blob. Two joins share one `IoPool` here; both real-server runs (isolated,
    // uncontended) complete in ~2.4s, but a real `cargo nextest run`'s own default
    // full-parallelism scheduling can multiply that well past `20`s under heavy
    // same-machine contention (confirmed directly: this exact test alone finished in
    // 2.379s, the same test inside a full-suite run exceeded 20s) -- `60` gives
    // comfortable headroom without masking a genuine hang.
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        // --- Break the grass block directly below A's own spawn column ---
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(0, -60, 0)),
                face: 1,
                sequence: 1,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            1
        );

        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(0, -60, 0)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);

        // B, uninvolved, receives the identical broadcast -- scanned for on its own
        // connection, since unrelated clientbound traffic (keep-alives, ...) may legally
        // interleave ahead of it there too.
        let body = recv_packet_of_type(&mut b, &mut b_acc, BlockUpdate::ID).await;
        let observed = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(observed, update);

        // --- Place above the still-intact grass block at (1, -60, 1) ---
        send_packet(
            &mut a,
            &UseItemOn {
                hand: 0,
                location: pack_position(BlockPos::new(1, -60, 1)),
                face: 1,
                cursor_x: 0.5,
                cursor_y: 0.0,
                cursor_z: 0.5,
                inside_block: false,
                hits_world_border: false,
                sequence: 2,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            2
        );

        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(1, -59, 1)));
        assert_eq!(update.block_state_id, blocks::STONE.0 as i32);

        let body = recv_packet_of_type(&mut b, &mut b_acc, BlockUpdate::ID).await;
        let observed = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(observed, update);

        // --- Criterion 1's own "persisted state" half: in-memory, dirty-marked ---
        assert_eq!(
            world.debug_query_block(BlockPos::new(0, -60, 0)).await,
            Some(DebugBlockInfo {
                raw_state: blocks::AIR.0,
                dirty: true,
            })
        );
        assert_eq!(
            world.debug_query_block(BlockPos::new(1, -59, 1)).await,
            Some(DebugBlockInfo {
                raw_state: blocks::STONE.0,
                dirty: true,
            })
        );
    })
    .await
    .unwrap();
}
