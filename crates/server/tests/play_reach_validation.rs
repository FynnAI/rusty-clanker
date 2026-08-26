//! M2-B07 acceptance test: MECH-D62's pinned reach bound (`BLOCK_INTERACTION_RANGE_CREATIVE
//! = 5.0`) and this blueprint's own bounded "only air is replaceable" placement/break
//! rejections, each with its own owed `Acknowledge Block Change` and, where applicable
//! (Context: never for `OutOfReach`), a corrective `Block Update`. Every test constructs
//! its own `HardcodedWorld::new()` -- no test shares state with any other.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, PlayerAction, UseItemOn, pack_position,
};
use rusty_clanker_server::play::{DebugBlockInfo, HardcodedWorld, PlayerProfile, enter_play};
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

/// Asserts no further packet arrives on `socket` within a short bounded timeout.
async fn assert_silence(socket: &mut TcpStream, accumulator: &mut BytesMut) {
    let outcome =
        tokio::time::timeout(Duration::from_millis(400), recv_packet(socket, accumulator)).await;
    assert!(
        outcome.is_err(),
        "expected no further packet, but received {:?}",
        outcome.ok()
    );
}

#[tokio::test]
async fn reach_rejects_out_of_range_target_with_ack_only() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // (20, -60, 20) sits in chunk (1, 1) -- one of the nine... locally-seeded chunks,
        // grass at y=-60 -- but well outside the 5.0 creative reach bound (distance ~28.4).
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(20, -60, 20)),
                face: 1,
                sequence: 5,
            },
        )
        .await;

        let (id, body) = recv_packet(&mut a, &mut a_acc).await;
        assert_eq!(id, AcknowledgeBlockChange::ID);
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            5
        );

        // `OutOfReach` owes no corrective `Block Update` at all (Context).
        assert_silence(&mut a, &mut a_acc).await;

        assert_eq!(
            world.debug_query_block(BlockPos::new(20, -60, 20)).await,
            Some(DebugBlockInfo {
                raw_state: blocks::GRASS_BLOCK.0,
                dirty: false,
            })
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn reach_accepts_in_range_target() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // distance ~2.12 -- comfortably within the 5.0 creative reach bound.
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(0, -60, 0)),
                face: 1,
                sequence: 6,
            },
        )
        .await;

        let (id, body) = recv_packet(&mut a, &mut a_acc).await;
        assert_eq!(id, AcknowledgeBlockChange::ID);
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            6
        );

        let (id, body) = recv_packet(&mut a, &mut a_acc).await;
        assert_eq!(id, BlockUpdate::ID);
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(0, -60, 0)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn placement_into_non_air_target_is_rejected_with_correction() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        // `inside_block: true` targets the clicked cell itself, (2, -60, 2), which is
        // GRASS_BLOCK -- distance ~3.54, within reach, but not AIR.
        send_packet(
            &mut a,
            &UseItemOn {
                hand: 0,
                location: pack_position(BlockPos::new(2, -60, 2)),
                face: 1,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside_block: true,
                hits_world_border: false,
                sequence: 7,
            },
        )
        .await;

        let (id, body) = recv_packet(&mut a, &mut a_acc).await;
        assert_eq!(id, AcknowledgeBlockChange::ID);
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            7
        );

        let (id, body) = recv_packet(&mut a, &mut a_acc).await;
        assert_eq!(id, BlockUpdate::ID);
        let correction = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(correction.location, pack_position(BlockPos::new(2, -60, 2)));
        assert_eq!(correction.block_state_id, blocks::GRASS_BLOCK.0 as i32);

        // The correction is actor-only, never broadcast.
        assert_silence(&mut b, &mut b_acc).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn breaking_air_is_rejected_with_correction() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // (2, -59, 2) -- distance ~3.04, within reach -- is already AIR (y=-59 is the
        // first all-air layer above the grass top, per the shared layer table).
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(2, -59, 2)),
                face: 1,
                sequence: 8,
            },
        )
        .await;

        let (id, body) = recv_packet(&mut a, &mut a_acc).await;
        assert_eq!(id, AcknowledgeBlockChange::ID);
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            8
        );

        let (id, body) = recv_packet(&mut a, &mut a_acc).await;
        assert_eq!(id, BlockUpdate::ID);
        let correction = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(correction.location, pack_position(BlockPos::new(2, -59, 2)));
        assert_eq!(correction.block_state_id, blocks::AIR.0 as i32);
    })
    .await
    .unwrap();
}
