//! M3-B02 acceptance tests: server-authoritative movement validation (Context, "Server-side
//! movement validation" / "Teleport / position-sync protocol") -- the speed check, the
//! teleport-correction state machine, and the NaN-position disconnect rule. Every test
//! constructs its own `HardcodedWorld::new()` -- no test shares state with any other
//! (mirrors every sibling `play_*.rs` acceptance test's own established convention).

use bytes::{Bytes, BytesMut};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, ConfirmTeleportation, KeepAliveClientbound, KeepAliveServerbound,
    SetPlayerPosition, SynchronizePlayerPosition,
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

/// As every sibling `play_*.rs` acceptance test's own identical helper -- transparently
/// answers any keep-alive challenge so a bounded wait/scan never trips `KeepAliveDriver`'s
/// own timeout.
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

// Generous timeouts matching every sibling `play_*.rs` acceptance test's own established
// budget (`play_reach_validation.rs`'s own doc comment has the full reasoning).

#[tokio::test]
async fn small_in_range_move_is_accepted_silently() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // A 0.1-block nudge from `SPAWN_POSITION`, well within the speed-check threshold
        // from a resting `velocity == Vec3::ZERO` state.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 0.1,
                y: -59.0,
                z: 0.0,
                on_ground: true,
            },
        )
        .await;

        // An accepted move is silent -- no ack, no correction (Context: "the server says
        // nothing when it agrees").
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            SynchronizePlayerPosition::ID,
            Duration::from_millis(500),
        )
        .await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn wildly_out_of_range_move_triggers_a_teleport_correction() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 2).await;

        // An obviously-impossible single-tick jump -- rejected by the speed check
        // (`SPEED_CHECK_THRESHOLD = 100.0`), issuing a teleport correction back to the
        // player's own last-known-good position (`SPAWN_POSITION`).
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 5000.0,
                y: -59.0,
                z: 0.0,
                on_ground: true,
            },
        )
        .await;

        let (id, body) = loop {
            let (id, body) = recv_clientbound(&mut a, &mut a_acc).await;
            if id == SynchronizePlayerPosition::ID {
                break (id, body);
            }
        };
        assert_eq!(id, SynchronizePlayerPosition::ID);
        let sync = decode_one::<SynchronizePlayerPosition>(body).unwrap();
        // id `2` -- `M1-B05`'s own join-time `SynchronizePlayerPosition` already consumed
        // id `1` (Context: `TeleportState::next_teleport_id`'s own starting value).
        assert_eq!(sync.teleport_id, 2);
        assert_eq!(sync.x, 0.0);
        assert_eq!(sync.y, -59.0);
        assert_eq!(sync.z, 0.0);
        assert_eq!(sync.yaw, 0.0);
        assert_eq!(sync.pitch, 0.0);
        assert_eq!(sync.relative_arguments, 0x00);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn movement_is_ignored_while_awaiting_a_teleport_ack() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 3).await;

        // Replicates the prior test's own setup: an out-of-range move issues a correction
        // awaiting teleport id `2`.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 5000.0,
                y: -59.0,
                z: 0.0,
                on_ground: true,
            },
        )
        .await;
        let sync_body = loop {
            let (id, body) = recv_clientbound(&mut a, &mut a_acc).await;
            if id == SynchronizePlayerPosition::ID {
                break body;
            }
        };
        let sync = decode_one::<SynchronizePlayerPosition>(sync_body).unwrap();
        assert_eq!(sync.teleport_id, 2);

        // A second, otherwise-plausible small move sent before acknowledging the pending
        // teleport -- discarded without running speed-check/replay (`MovementOutcome::
        // IgnoredAwaitingTeleport`): no second correction, no acceptance.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 0.05,
                y: -59.0,
                z: 0.0,
                on_ground: true,
            },
        )
        .await;
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            SynchronizePlayerPosition::ID,
            Duration::from_millis(500),
        )
        .await;

        // Acknowledges the pending teleport -- normal validation resumes next tick.
        send_packet(&mut a, &ConfirmTeleportation { teleport_id: 2 }).await;

        // A subsequent small move is now accepted silently.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 0.05,
                y: -59.0,
                z: 0.0,
                on_ground: true,
            },
        )
        .await;
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            SynchronizePlayerPosition::ID,
            Duration::from_millis(500),
        )
        .await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn nan_position_disconnects_the_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 4).await;

        // `f64::NAN`'s own IEEE-754 bit pattern, written by the same `WireWrite for f64`
        // impl every other `f64` field on this packet uses (Deliverables' own "bypassing
        // the normal packet-struct constructor's [non-existent] validation" -- a plain
        // Rust struct offers none to bypass; this is the literal NaN payload either way).
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: f64::NAN,
                y: -59.0,
                z: 0.0,
                on_ground: true,
            },
        )
        .await;

        // The connection closes -- a subsequent socket read returns EOF within a bounded
        // timeout (every other well-framed packet this connection might still emit before
        // closing, e.g. a keep-alive challenge, is tolerated by `recv_clientbound` above,
        // but a raw read loop here is simpler and sufficient: any further bytes are either
        // a keep-alive challenge this test never answers -- itself eventually timing the
        // connection out and closing it -- or nothing at all).
        let mut chunk = [0u8; 4096];
        loop {
            match tokio::time::timeout(Duration::from_secs(30), a.read(&mut chunk)).await {
                Ok(Ok(0)) => return, // EOF -- connection closed, as expected.
                Ok(Ok(n)) => a_acc.extend_from_slice(&chunk[..n]),
                Ok(Err(_)) => return, // connection reset -- also closed.
                Err(_) => panic!("connection did not close within the bounded timeout"),
            }
        }
    })
    .await
    .unwrap();
}
