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
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SetPlayerRotation, pack_position,
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

/// Scans the clientbound stream once, bucketing every `AcknowledgeBlockChange` and every
/// `BlockUpdate` packet it sees -- each in the order its own bucket receives it (FIFO
/// receipt order) -- while discarding anything else, until `count` of both have been
/// collected. Bounded only by the surrounding `#[tokio::test]`'s own outer
/// `tokio::time::timeout` -- deliberately no second, independent deadline here: an inner
/// deadline shorter than that outer one would misfire under exactly the kind of heavy,
/// legitimate contention the outer 60s budget already exists to tolerate (confirmed
/// directly: an earlier draft of this helper with its own tighter 30s deadline produced a
/// new, synthetic-contention-only failure that also reproduced on an untouched sibling
/// test).
///
/// Interleaving contract: since the M2 live-chunk-streaming wiring, unrelated clientbound
/// packets (late chunk data, `SetHealth`, keep-alives, ...) may legally arrive interleaved
/// with the response packets under test -- they come from a different producer (the
/// connection's own keep-alive loop) than the region tick thread's `respond_to_action`,
/// which is the only thing that ever sends `AcknowledgeBlockChange`/`BlockUpdate`. A single
/// scan (rather than two separate ones) is required here specifically because
/// `AcknowledgeBlockChange` and `BlockUpdate` packets are themselves interleaved with each
/// other (one pair per action) -- a second, separate scan would have nowhere to recover a
/// `BlockUpdate` already discarded while an earlier scan was hunting for acks. Each
/// per-type bucket's own FIFO order is the real invariant under test (MECH-D4 /
/// `respond_to_action`'s single-threaded per-action response order) -- physical adjacency
/// between an ack and its own `BlockUpdate`, or freedom from unrelated packets in between,
/// was never the tested contract.
async fn collect_acks_and_updates(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    count: usize,
) -> (Vec<Bytes>, Vec<Bytes>) {
    let mut acks = Vec::with_capacity(count);
    let mut updates = Vec::with_capacity(count);
    while acks.len() < count || updates.len() < count {
        let (id, body) = recv_clientbound(socket, accumulator).await;
        if id == AcknowledgeBlockChange::ID && acks.len() < count {
            acks.push(body);
        } else if id == BlockUpdate::ID && updates.len() < count {
            updates.push(body);
        }
    }
    (acks, updates)
}

/// M3-B03 test-authoring addition: waits until `check` returns `true` -- mirrors
/// `play_movement_application.rs`'s own identical helper.
async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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
    tokio::time::timeout(std::time::Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
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

        // M3-B03 test-authoring update: reach is now a real voxel raycast (MECH-D62) --
        // looking straight down (`pitch: 90.0`) hits whatever is directly below the
        // player's own eye position, comfortably within the 5.0 creative reach bound.
        let sessions = world.player_sessions();
        send_packet(
            &mut client,
            &SetPlayerRotation {
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([0.0, 90.0]))
            .await;

        // Three breaks, sent back-to-back, before reading any response to any of them --
        // a straight-down dig column under spawn: each break exposes the next layer, which
        // is exactly what the very next break's own raycast then hits (Stone/Wood-pickaxe-
        // style column digging) -- unlike M2-B07's own three-different-diagonal-columns
        // shape, which a single fixed look direction could no longer all reach at once
        // under a real raycast.
        let targets = [
            (BlockPos::new(0, -61, 0), 10),
            (BlockPos::new(0, -62, 0), 11),
            (BlockPos::new(0, -63, 0), 12),
        ];
        for (location, sequence) in targets {
            send_packet(
                &mut client,
                &PlayerAction {
                    status: 0,
                    location: pack_position(location),
                    direction: 1,
                    sequence,
                },
            )
            .await;
        }

        // Scan-collect, not fixed-position reads (`collect_acks_and_updates`'s own doc
        // comment has the full interleaving contract) -- unrelated clientbound packets may
        // legally sit between any two of the six packets this burst owes. The semantic
        // contract itself (FIFO order, exact count, exact payloads for both the acks and
        // the block updates) stays fully asserted below, unweakened.
        let (acks, updates) =
            collect_acks_and_updates(&mut client, &mut accumulator, targets.len()).await;

        for (body, (_, sequence)) in acks.into_iter().zip(targets) {
            assert_eq!(
                decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
                sequence
            );
        }

        for (body, (location, _)) in updates.into_iter().zip(targets) {
            let update = decode_one::<BlockUpdate>(body).unwrap();
            assert_eq!(update.location, pack_position(location));
            assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
        }
    })
    .await
    .unwrap();
}
