//! Regression test for the production race flagged as `task_9ce21947`: `world.rs`'s tick
//! loop drains two independent, unsynchronized mpsc channels (`join_tx`/`block_action_tx`)
//! -- a block action can legitimately be processed on a tick before the acting player's own
//! `PlayerMarker` entity has been spawned into `region.world`. `respond_to_action`'s own
//! `Applied`/`RoutedCrossRegion` broadcast previously iterated only `world`'s own spawned
//! `PlayerMarker`s, silently dropping exactly the acting player's own copy in that case
//! (every *other* already-connected player still received theirs correctly, since only the
//! actor's own entity could possibly be missing).
//!
//! Reproduced deterministically (not by racing real timing) by directly queuing a
//! `PendingBlockAction` for a `network_entity_id` that is never joined at all -- the
//! strongest, permanent form of "this actor's own `PlayerMarker` does not exist in
//! `region.world` this tick," guaranteed reproducible on every run. A normally-joined
//! bystander player is also present, matching the real production scenario (at least one
//! other already-connected player) and proving chunk (0, 0) is resident so the phantom
//! action can actually resolve.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, pack_position,
};
use rusty_clanker_server::play::{
    BlockActionKind, HardcodedWorld, PendingBlockAction, PlayerProfile, enter_play,
};

// M3-B03 test-authoring update: `BlockActionKind::Break` (M2-B07) is retired --
// `StartDestroy` is its replacement (Deliverables, `block_action.rs`). This phantom actor
// never joins at all, so `world.rs`'s own reach-check fallback (no `PlayerMotion` found)
// applies: a synthetic motion at `SPAWN_POSITION` looking straight down (`pitch: 90.0`) --
// the only fallback orientation that can honestly resolve a reach check against this
// hardcoded world's own always-below-spawn content (`world.rs`'s own doc comment on that
// fallback). `(0, -60, 0)` (grass directly below spawn) is exactly what that fallback ray
// hits, so this test's own target needs no change beyond the variant rename. The same
// fallback also has no `GameModeState`, defaulting to `instabuild: true` (creative) --
// matching this test's own expectation of an immediate, unconditional break.
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
        recv_clientbound(socket, accumulator).await;
    }
    loop {
        let (id, _) = recv_clientbound(socket, accumulator).await;
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

/// Stage budgets are pure HANG GUARDS, deliberately sized for a starved shared CI
/// runner (a 2-core host running the rest of the suite concurrently) -- the same
/// reasoning as login_configuration_flow's outer budgets. Two structural races this
/// test used to carry (bystander keep-alive starvation; the join/action mpsc race
/// itself) are fixed and pinned by the assertions below, which are load-independent;
/// after those fixes the only remaining CI failures were plain budget exhaustion
/// under runner contention, never wrong packets.
/// Per-stage timeout wrapper: a hang names its exact stage instead of collapsing into
/// one opaque whole-test deadline. Added after CI's `windows-2025` runner hit that
/// opaque deadline twice (runs 33022662264 / 33023447974) while the same code stayed
/// green on `ubuntu-24.04`, on a local run, and on one earlier `windows-2025` run —
/// the next CI failure must say *which* wait hung, not just that one did.
async fn staged<T>(
    stage: &'static str,
    limit: Duration,
    fut: impl std::future::Future<Output = T>,
) -> T {
    match tokio::time::timeout(limit, fut).await {
        Ok(value) => value,
        Err(_) => panic!("stage {stage:?} timed out after {limit:?}"),
    }
}

#[tokio::test]
async fn broadcast_reaches_the_actor_even_when_its_own_player_marker_was_never_spawned() {
    let world = HardcodedWorld::new();

    // Bystander: a normal join, so chunk (0, 0) becomes resident (needed for the
    // phantom action below to ever resolve) and so `respond_to_action`'s broadcast
    // loop has at least one real `PlayerMarker` to iterate -- exactly the "at least one
    // other already-connected player" shape of the real production race.
    let (mut bystander, mut bystander_acc) = staged(
        "bystander-join",
        Duration::from_secs(60),
        spawn_actor(&world, "bystander", 1),
    )
    .await;

    // Phantom actor: a real connection and a fresh `network_entity_id`, deliberately
    // never joined via `enter_play`/`queue_join` at all -- guarantees, permanently,
    // that this id's own `PlayerMarker` is never spawned into `region.world`.
    let (server, mut phantom) = connected_pair().await;
    let (_inbound, handle) = spawn_connection(server, ConnectionConfig::default());
    let phantom_id = world.alloc_network_entity_id();
    let mut phantom_acc = BytesMut::new();

    // `queue_block_action` now returns `Result<(), RegionUnavailable>` (M3 field-report fix,
    // symptom 2, `world.rs`'s own doc comment) -- the region is alive throughout this test,
    // so this call always succeeds; explicitly discarded rather than left `#[must_use]`.
    let _ = world.queue_block_action(PendingBlockAction {
        network_entity_id: phantom_id,
        connection: handle.clone(),
        kind: BlockActionKind::StartDestroy {
            location: BlockPos::new(0, -60, 0),
        },
        sequence: 42,
    });

    // M3 field-report test-authoring fix: the phantom's own ack and block-update, and the
    // bystander's own copy of the same broadcast, are all paced by the region's own real,
    // non-drift-compounding tick clock (`TickClock::await_next_tick` -- proven load-
    // stretchable under host contention in this repo already, commit 2fadab1's own doc
    // comment on `play_block_break_place_full.rs`'s `delayed_destroy_auto_finalize_
    // excludes_the_breaker_from_the_level_event`, the identical root-cause class). This
    // test's own former shape read `phantom` to completion across two whole stages before
    // ever touching `bystander`'s socket again, leaving `bystander` unread -- so unable to
    // auto-answer any `KeepAliveClientbound` challenge, `recv_clientbound`'s own per-call
    // side effect -- for however long that stretch actually took in real time; under load,
    // `KeepAliveDriver`'s own ~30s nominal disconnect schedule (running on `bystander`'s own
    // connection task, independently of the region tick thread's speed) can fire mid-wait,
    // and the later `bystander` read then panics on the server's own already-closed
    // connection instead of ever seeing the broadcast. Fixed the same way as 2fadab1: `
    // tokio::join!` reads `phantom`'s own two-stage sequence and `bystander`'s own single
    // stage concurrently, so neither socket is ever left unread for a load-sensitive
    // stretch, while each of the three stages below keeps its own named timeout budget
    // (unchanged) so a genuine hang still names exactly which wait stalled.
    tokio::join!(
        async {
            // A hang here means the action never resolved at all (e.g. chunk (0, 0) not
            // resident, or lost residency, so the action is carried tick over tick
            // forever).
            let ack = staged(
                "phantom-ack",
                Duration::from_secs(45),
                recv_packet_of_type(&mut phantom, &mut phantom_acc, AcknowledgeBlockChange::ID),
            )
            .await;
            assert_eq!(
                decode_one::<AcknowledgeBlockChange>(ack).unwrap().sequence,
                42
            );

            // The regression under test: before the fix, this would never arrive (the
            // phantom actor's own `PlayerMarker` never existed for `respond_to_action`'s
            // broadcast loop to find), and the wait would hang. A hang here with the ack
            // already received means the action resolved but its actor-directed broadcast
            // was lost again.
            let update = staged(
                "phantom-block-update",
                Duration::from_secs(45),
                recv_packet_of_type(&mut phantom, &mut phantom_acc, BlockUpdate::ID),
            )
            .await;
            let update = decode_one::<BlockUpdate>(update).unwrap();
            assert_eq!(update.location, pack_position(BlockPos::new(0, -60, 0)));
            assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
        },
        async {
            // The already-connected bystander is a normal broadcast recipient too, read
            // concurrently with `phantom`'s own two stages above (not after) so its socket
            // is never left silently unread for however long those actually take.
            let bystander_update = staged(
                "bystander-block-update",
                Duration::from_secs(45),
                recv_packet_of_type(&mut bystander, &mut bystander_acc, BlockUpdate::ID),
            )
            .await;
            let bystander_update = decode_one::<BlockUpdate>(bystander_update).unwrap();
            assert_eq!(
                bystander_update.location,
                pack_position(BlockPos::new(0, -60, 0))
            );
            assert_eq!(bystander_update.block_state_id, blocks::AIR.0 as i32);
        },
    );
}
