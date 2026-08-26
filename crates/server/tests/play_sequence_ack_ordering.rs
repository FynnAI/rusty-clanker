//! M2-B07 acceptance test: MECH-D4's "deterministic merge by ascending player id" applied
//! to a single player's own burst of actions -- the manual per-tick queue drain
//! (`HardcodedWorld`'s own Stage-3-equivalent step) must preserve that one player's own
//! original receipt (FIFO) order for both the `Acknowledge Block Change` and the resulting
//! `Block Update` of each action, never interleaving or reordering them.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, PlayerAction, pack_position,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

// M2 integration test-authoring fix: raised from `20` -- `enter_play` now awaits a real,
// ticket-driven `RC-IoPool` chunk-grid load per join (`connection.rs`'s own
// `request_chunk_grid` call), a genuinely asynchronous round trip absent when this budget
// was first tuned against the old, instantly-synthesized placeholder chunk blob (matches
// the identical fix and reasoning in `play_reach_validation.rs`/
// `play_block_place_break.rs`, both hit the same real, `cargo nextest`-confirmed
// full-suite contention).
#[tokio::test]
async fn sequence_acks_preserve_fifo_order_under_a_burst() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let world_for_task = world.clone();
        let profile = PlayerProfile {
            uuid: 1,
            username: "a".to_string(),
        };
        tokio::spawn(async move {
            enter_play(handle, inbound, profile, &world_for_task).await;
        });

        let mut accumulator = BytesMut::new();
        drain_play_entry(&mut client, &mut accumulator).await;

        // Three breaks, sent back-to-back, before reading any response to any of them --
        // every target comfortably within the 5.0 creative reach bound.
        let targets = [
            (BlockPos::new(1, -60, 1), 10),
            (BlockPos::new(2, -60, 1), 11),
            (BlockPos::new(2, -60, 2), 12),
        ];
        for (location, sequence) in targets {
            send_packet(
                &mut client,
                &PlayerAction {
                    status: 0,
                    location: pack_position(location),
                    face: 1,
                    sequence,
                },
            )
            .await;
        }

        for (location, sequence) in targets {
            let (id, body) = recv_packet(&mut client, &mut accumulator).await;
            assert_eq!(id, AcknowledgeBlockChange::ID);
            assert_eq!(
                decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
                sequence
            );

            let (id, body) = recv_packet(&mut client, &mut accumulator).await;
            assert_eq!(id, BlockUpdate::ID);
            let update = decode_one::<BlockUpdate>(body).unwrap();
            assert_eq!(update.location, pack_position(location));
            assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
        }
    })
    .await
    .unwrap();
}
