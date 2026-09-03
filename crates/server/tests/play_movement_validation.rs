//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only))
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
    SetPlayerPosition, SetPlayerPositionAndRotation, SetPlayerRotation, SynchronizePlayerPosition,
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
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // A 0.1-block nudge from `SPAWN_POSITION`, well within the speed-check threshold
        // from a resting `velocity == Vec3::ZERO` state.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 0.1,
                y: -60.0,
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
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 2).await;

        // An obviously-impossible single-tick jump -- rejected by the speed check
        // (`SPEED_CHECK_THRESHOLD = 100.0`), issuing a teleport correction back to the
        // player's own last-known-good position (`SPAWN_POSITION`).
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 5000.0,
                y: -60.0,
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
        assert_eq!(sync.y, -60.0);
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
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 3).await;

        // Replicates the prior test's own setup: an out-of-range move issues a correction
        // awaiting teleport id `2`.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 5000.0,
                y: -60.0,
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
                y: -60.0,
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
                y: -60.0,
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
    tokio::time::timeout(Duration::from_secs(300), async {
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
                y: -60.0,
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
            match tokio::time::timeout(Duration::from_secs(120), a.read(&mut chunk)).await {
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

/// M3 field-report regression (Defect A): mirrors `nan_position_disconnects_the_connection`
/// exactly, but for the *rotation* half of the blueprint's own jointly-stated "any reported
/// position OR rotation coordinate that is NaN or non-finite is rejected outright" rule
/// (Context, "Server-side movement validation") -- a bare `SetPlayerRotation` carries no
/// position claim at all, so this exercises `evaluate_movement`'s rotation check as the sole
/// path to `MovementOutcome::Disconnect`, independent of the position check a few lines below
/// it.
#[tokio::test]
async fn nan_rotation_disconnects_the_connection() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 5).await;

        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw: 0.0,
                pitch: f32::NAN,
                on_ground: true,
            },
        )
        .await;

        // A bare `SetPlayerRotation` carries no position claim at all -- without this fix,
        // `evaluate_movement` applies the NaN unconditionally and then returns
        // `NoPositionClaim` (not `Disconnect`), which sends nothing and closes nothing.
        // `KEEPALIVE_INTERVAL` (`play::keepalive`, 15 s) plus its own grace window would
        // *eventually* close a connection like that anyway once the never-answered keep-alive
        // challenge this raw read loop ignores times out -- a real close, but for the wrong
        // reason, which would make a generous, `nan_position_disconnects_the_connection`-style
        // 30 s wait here pass either way and prove nothing. A tight bound is used instead,
        // comfortably wide for the real `Disconnect` path (well under one region tick) and
        // comfortably narrower than any keep-alive-driven close could ever land inside.
        let mut chunk = [0u8; 4096];
        loop {
            match tokio::time::timeout(Duration::from_secs(2), a.read(&mut chunk)).await {
                Ok(Ok(0)) => return, // EOF -- connection closed, as expected.
                Ok(Ok(n)) => a_acc.extend_from_slice(&chunk[..n]),
                Ok(Err(_)) => return, // connection reset -- also closed.
                Err(_) => panic!(
                    "connection did not close within the bounded timeout -- a real Disconnect \
                     closes almost immediately, well under this bound"
                ),
            }
        }
    })
    .await
    .unwrap();
}

/// M3 field-report regression (Defect A, "the correction packet path"): pairs a NaN pitch
/// with an otherwise-ordinary speed violation (`wildly_out_of_range_move_triggers_a_teleport_
/// correction`'s own `x: 5000.0`) in a single `SetPlayerPositionAndRotation`. Before this fix,
/// `evaluate_movement` wrote the unvalidated rotation into `motion` *before* the speed check
/// ran, so the speed violation's own `RejectSpeed` correction echoed that same NaN pitch
/// straight back onto the wire in a real `SynchronizePlayerPosition` packet (`respond_to_
/// movement`'s own `pitch: motion.pitch` field) -- malformed input reaching a real client
/// instead of the connection simply closing. This asserts the fixed, correct behavior: no
/// `SynchronizePlayerPosition` of any kind is ever sent (the rotation check disconnects before
/// the speed check is even reached), and the connection actually closes.
#[tokio::test]
async fn nan_rotation_paired_with_a_speed_violation_disconnects_before_any_correction_is_sent() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 6).await;

        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: 5000.0,
                y: -60.0,
                z: 0.0,
                yaw: 0.0,
                pitch: f32::NAN,
                on_ground: true,
            },
        )
        .await;

        // A single combined read loop, deliberately not `assert_no_packet_of_type` followed
        // by a separate EOF-wait (that helper's own inner `recv_packet` asserts `n > 0` on
        // every read, i.e. treats EOF as a hard failure -- exactly the *expected*, passing
        // outcome here, since the fix disconnects promptly rather than merely staying quiet
        // for the window). Reads until either a `SynchronizePlayerPosition` arrives (a
        // leaked, NaN-carrying correction -- fails the test immediately, the exact bug this
        // guards against) or the connection closes (EOF/reset -- the expected `Disconnect`
        // outcome, confirming the correction was never built in the first place, not merely
        // raced past). Any other well-framed packet (e.g. a keep-alive challenge) is
        // tolerated and answered, as `recv_clientbound` does elsewhere in this file.
        let mut chunk = [0u8; 4096];
        loop {
            if let Some(payload) =
                rc_protocol::try_decode_frame(&mut a_acc, CompressionState::Disabled).unwrap()
            {
                let mut body = payload;
                let id = VarInt::decode(&mut body).unwrap().get();
                assert_ne!(
                    id,
                    SynchronizePlayerPosition::ID,
                    "a SynchronizePlayerPosition must never be sent -- the rotation check must \
                     disconnect before the speed check's own correction is ever built"
                );
                if id == KeepAliveClientbound::ID {
                    let challenge = decode_one::<KeepAliveClientbound>(body).unwrap();
                    send_packet(&mut a, &KeepAliveServerbound { id: challenge.id }).await;
                }
                continue;
            }
            match tokio::time::timeout(Duration::from_secs(120), a.read(&mut chunk)).await {
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
