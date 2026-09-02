//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model; changes the actor's own pose/eye-height, not a build/break-into-self target) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only))
//! M3 field-report test-authoring (Symptom 2, end-to-end): a real serverbound `player_input`
//! packet (id `0x2B`, `packets::PlayerInput`) setting the shift bit must lower the acting
//! player's own eye position (`PLAYER_EYE_HEIGHT_CROUCHING`, `1.27`, vs the standing `1.62`)
//! and, through that, bring an otherwise-out-of-reach block into MECH-D62 reach. Fails today:
//! `player_input` is not yet recognized by `connection.rs`'s dispatch (falls into the
//! catch-all, silently dropped) and `world.rs`'s tick loop still validates reach via the
//! retired, pose-blind voxel raycast -- so the target below is rejected whether or not the
//! shift packet is sent. Mirrors `play_reach_validation.rs`'s own helper shape.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PLAYER_INPUT_SHIFT, PlayerAction, PlayerInput,
    SetPlayerPositionAndRotation, pack_position,
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

async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// The scripted geometry (Context, matching `mining_reach_validation.rs`'s own pure-function
/// case): feet directly above the grass at `(0, -60, 0)`, high enough that the standing eye
/// (`1.62`) sits just past the 5.5 survival threshold from the block's own nearest (top) face
/// and the crouching eye (`1.27`) sits just inside it. Deliberately left Creative (the
/// default -- no `debug_set_survival` call): a Creative break is always instant regardless of
/// hardness/tool, so a single `StartDestroy` either finalizes on the spot or is rejected --
/// unlike Survival, where `StartDestroy` alone only ever *begins* the dig-timing state
/// machine (`mining::begin_destroy`'s own `Tracking` outcome) and an active, non-delayed
/// destroy never auto-finalizes without an explicit `StopDestroy` packet, no matter how many
/// ticks pass (`mining.rs`'s own dig-packet-lifecycle doc comment) -- the wrong mode for a
/// test that wants one action to unambiguously succeed or fail.
const FEET_Y: f64 = -54.5;
const TARGET: BlockPos = BlockPos::new(0, -60, 0);

#[tokio::test]
async fn sneaking_makes_an_otherwise_out_of_reach_block_breakable() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: 0.5,
                y: FEET_Y,
                z: 0.5,
                yaw: 0.0,
                pitch: 90.0,
                on_ground: false,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid_a, |r| r.data.pos) == Some([0.5, FEET_Y, 0.5]))
            .await;

        // Sanity leg: standing, this target is still out of reach -- ack only, no correction.
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(TARGET),
                direction: 1,
                sequence: 1,
            },
        )
        .await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            1
        );
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            BlockUpdate::ID,
            Duration::from_millis(400),
        )
        .await;

        // Crouch (shift bit set) -- no direct hook to poll `PlayerInputState` from a test, so
        // a short fixed grace covers the tick this needs to land, mirroring the paritybot
        // bot's own `AIM_SETTLE_TICKS` precedent for the same kind of "wait for a decoded
        // packet to actually apply" gap.
        send_packet(
            &mut a,
            &PlayerInput {
                flags: PLAYER_INPUT_SHIFT,
            },
        )
        .await;
        tokio::time::sleep(Duration::from_millis(150)).await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(TARGET),
                direction: 1,
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
        assert_eq!(update.location, pack_position(TARGET));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
    })
    .await
    .unwrap();
}
