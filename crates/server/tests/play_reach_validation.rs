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
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SetPlayerPositionAndRotation, SetPlayerRotation, UseItemOn,
    pack_position,
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

/// Reads the next clientbound packet like `recv_packet`, but transparently answers any
/// `KeepAliveClientbound` challenge it encounters with the matching `KeepAliveServerbound`
/// reply before returning it -- exactly what a real vanilla client does. Necessary for
/// correctness, not just politeness: the scans below are willing to wait as long as the
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
/// with the response packet under test -- a vanilla client tolerates arbitrary
/// interleaving, so this scan does too. Safe to call back-to-back for two different
/// expected ids (as this file's own tests do, for the ack then the corrective update)
/// because `respond_to_action` always emits a given action's own `AcknowledgeBlockChange`
/// before its own `BlockUpdate`, in that single-threaded order, on the same connection --
/// only *other* traffic can land in between, never those two out of order.
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

/// Asserts no packet of `forbidden_id` arrives on `socket` within `window`, tolerating and
/// discarding any other legal clientbound traffic (late chunk data, `SetHealth`,
/// keep-alives, ...) that might interleave -- since the M2 live-chunk-streaming wiring,
/// only the specific packet type under test is the actual contract (`OutOfReach` owing no
/// corrective `Block Update`; a correction never broadcast to another player); unrelated
/// traffic is not evidence of either. Watches out the full window rather than returning on
/// the first benign packet, so silence must hold for the whole duration, not merely until
/// something (anything) shows up.
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

/// M3-B03 test-authoring addition: waits until `check` returns `true` -- mirrors
/// `play_movement_application.rs`'s own identical helper (that file's own doc comment has
/// the full reasoning: reach is now a real voxel raycast, MECH-D62, so a rotation/position
/// change must actually be applied before a dependent block action is sent).
async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// M2 integration test-authoring fix: every one of this file's four `timeout` budgets
// below raised from `20`s to `60`s -- `enter_play` now awaits a real, ticket-driven
// `RC-IoPool` chunk-grid load per join (`connection.rs`'s own `request_chunk_grid` call),
// a genuinely asynchronous round trip absent when these budgets were first tuned against
// the old, instantly-synthesized placeholder chunk blob. A real `cargo nextest run`'s own
// default full-parallelism scheduling can push that latency well past `20`s under heavy
// same-machine contention even though an isolated run of any one of these tests finishes
// in ~1.5s -- `60` gives comfortable headroom without masking a genuine hang.
#[tokio::test]
async fn reach_rejects_out_of_range_target_with_ack_only() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // (20, -60, 20) sits in chunk (1, 1) -- one of the nine... locally-seeded chunks,
        // grass at y=-60 -- but well outside the 5.0 creative reach bound (distance ~28.4,
        // and outside `cast_ray`'s own `max_distance` budget regardless of look direction --
        // no rotation setup needed here, unlike every other test below).
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(20, -60, 20)),
                direction: 1,
                sequence: 5,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            5
        );

        // `OutOfReach` owes no corrective `Block Update` at all (Context).
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            BlockUpdate::ID,
            Duration::from_millis(400),
        )
        .await;

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
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        // M3-B03 test-authoring update: reach is now a real voxel raycast (MECH-D62) --
        // looking straight down (`pitch: 90.0`) hits whatever is directly below A's own eye
        // position, distance ~1.62, comfortably within the 5.0 creative reach bound.
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([0.0, 90.0]))
            .await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(0, -60, 0)),
                direction: 1,
                sequence: 6,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            6
        );

        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(0, -60, 0)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn placement_into_non_air_target_is_rejected_with_correction() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let sessions = world.player_sessions();

        // M3-B03 test-authoring update: A moves above (2, -60, 2) and looks straight down,
        // so the raycast's own claimed target (the packet's raw, clicked `location`) is
        // exactly the block A clicked -- `inside_block: true` then targets that same clicked
        // cell itself (GRASS_BLOCK, not AIR) for the placement-mutation logic.
        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: 2.0,
                y: -59.0,
                z: 2.0,
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid_a, |r| r.data.pos) == Some([2.0, -59.0, 2.0]))
            .await;

        send_packet(
            &mut a,
            &UseItemOn {
                hand: 0,
                location: pack_position(BlockPos::new(2, -60, 2)),
                direction: 1,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside_block: true,
                hits_world_border: false,
                sequence: 7,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            7
        );

        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let correction = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(correction.location, pack_position(BlockPos::new(2, -60, 2)));
        assert_eq!(correction.block_state_id, blocks::GRASS_BLOCK.0 as i32);

        // The correction is actor-only, never broadcast.
        assert_no_packet_of_type(
            &mut b,
            &mut b_acc,
            BlockUpdate::ID,
            Duration::from_millis(400),
        )
        .await;
    })
    .await
    .unwrap();
}

/// M3-B03 test-authoring update (renamed and re-derived from M2-B07's own `breaking_air_
/// is_rejected_with_correction`): a real voxel raycast can never *hit* an air cell (`cast_
/// ray`'s own contract -- it only reports a hit against a non-empty shape), so a `Player
/// Action` naming an already-air `location` no longer reaches `mining::finalize_break`'s own
/// `TargetAlreadyAir` rejection path at all -- it fails the reach check first, exactly as a
/// real vanilla client's own local raycast would never let it send such a packet in the
/// first place (there is nothing under the crosshair to interact with). This is a genuine
/// parity improvement over M2-B07's own Euclidean-only check (which could not tell "no
/// target under the crosshair" from "a valid, if already-air, target"), not a weakened test
/// -- `mining::finalize_break`'s own `TargetAlreadyAir` arm is exercised directly, holding
/// its own hand-constructed pre-air `BlockWorldAccess`, by `mining_destroy_state_machine.rs`
/// sibling coverage is not needed here since that path is no longer reachable end-to-end
/// through this crate's own packet layer.
#[tokio::test]
async fn breaking_air_is_rejected_out_of_reach_not_with_a_correction() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        // Looks straight down from spawn -- the nearest solid block on that ray is the
        // grass at (0, -60, 0), not the air cell at (0, -59, 0) itself, so a claimed target
        // of (0, -59, 0) can never be the raycast's own hit position.
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([0.0, 90.0]))
            .await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(0, -59, 0)),
                direction: 1,
                sequence: 8,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            8
        );

        // `OutOfReach` owes no corrective `Block Update` at all (Context) -- unlike M2-B07's
        // own `TargetAlreadyAir`, which did.
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            BlockUpdate::ID,
            Duration::from_millis(400),
        )
        .await;
    })
    .await
    .unwrap();
}
