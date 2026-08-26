//! M2 field-report regression test: reach validation (`M2-B07`'s own gate,
//! `blueprints/M2/M2-B07-block-interaction-minimal.md`) must key off the acting player's
//! own live position, kept current by the movement-application fix in
//! `play::movement`/`play::world`'s tick loop -- not the hardcoded `SPAWN_POSITION`
//! constant every prior version of this check used unconditionally, the exact root cause of
//! the reported "place/break only works in a sphere around spawn" symptom. Every test
//! constructs its own `HardcodedWorld::new()` -- no test shares state with any other.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SetPlayerPosition, pack_position,
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

/// As `play_reach_validation.rs`'s own identical helper -- transparently answers any
/// keep-alive challenge so a generous scan/wait budget never trips `KeepAliveDriver`'s own
/// timeout.
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

async fn assert_no_packet_of_type(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    forbidden_id: i32,
    window: Duration,
) {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, _)) => assert_ne!(
                id, forbidden_id,
                "expected no packet of id {forbidden_id}, but received one"
            ),
            Err(_) => return,
        }
    }
}

/// Polls `check` (a closure reading `PlayerSessionStore` directly, never TCP) until it
/// returns `true`, bounded only by the surrounding test's own outer `tokio::time::timeout` --
/// the movement-application fix syncs a live position/rotation into the player's own
/// session record on every applied tick (`world.rs`'s own tick loop, "M2 field-report
/// persistence fix"), so this is a deterministic, race-free way to know a sent movement
/// packet has actually been applied before sending a follow-up packet whose own outcome
/// depends on it -- avoids relying on fragile cross-thread timing between two back-to-back
/// packet sends.
async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// Generous timeouts matching every sibling `play_*.rs` acceptance test's own established
// budget (`play_reach_validation.rs`'s own doc comment has the full reasoning: a real,
// ticket-driven `RC-IoPool` chunk-grid load per join needs comfortable headroom under
// `cargo nextest`'s full-parallelism scheduling).

#[tokio::test]
async fn reach_check_uses_the_live_position_after_a_movement_packet() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(101);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 101).await;

        // (20, -60, 20) sits in chunk (1, 1) -- one of the eleven-by-eleven locally-seeded
        // chunks, grass at y=-60 -- ~28.4 blocks from the fixed `SPAWN_POSITION`, well
        // outside the 5.0 creative reach bound: before this fix, every reach check used
        // `SPAWN_POSITION` unconditionally, so this would always be rejected `OutOfReach`
        // no matter where the player actually claimed to stand.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 20.0,
                y: -60.0,
                z: 20.0,
                on_ground: true,
            },
        )
        .await;

        let sessions = world.player_sessions();
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.pos) == Some([20.0, -60.0, 20.0]))
            .await;

        // Now within ~2.1 blocks of the moved position -- accepted.
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(20, -60, 20)),
                face: 1,
                sequence: 30,
            },
        )
        .await;

        let ack = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(ack).unwrap().sequence,
            30
        );

        let update = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(update).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(20, -60, 20)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);

        assert_eq!(
            world.debug_query_block(BlockPos::new(20, -60, 20)).await,
            Some(DebugBlockInfo {
                raw_state: blocks::AIR.0,
                dirty: true,
            })
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn moving_away_from_a_target_makes_it_go_out_of_reach() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(102);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 102).await;

        // (0, -60, 0) sits ~2.1 blocks from `SPAWN_POSITION` -- comfortably in reach at
        // join time. Moving to (40, -60, 0) first puts the player's own live eye position
        // ~40 blocks away, well past the 5.0 creative bound.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 40.0,
                y: -60.0,
                z: 0.0,
                on_ground: true,
            },
        )
        .await;

        let sessions = world.player_sessions();
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.pos) == Some([40.0, -60.0, 0.0]))
            .await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(0, -60, 0)),
                face: 1,
                sequence: 31,
            },
        )
        .await;

        let ack = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(ack).unwrap().sequence,
            31
        );

        // `OutOfReach` owes no corrective `Block Update` at all (M2-B07's own established
        // contract, `play_reach_validation.rs`'s identical assertion).
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            BlockUpdate::ID,
            Duration::from_millis(400),
        )
        .await;

        assert_eq!(
            world.debug_query_block(BlockPos::new(0, -60, 0)).await,
            Some(DebugBlockInfo {
                raw_state: blocks::GRASS_BLOCK.0,
                dirty: false,
            })
        );
    })
    .await
    .unwrap();
}
