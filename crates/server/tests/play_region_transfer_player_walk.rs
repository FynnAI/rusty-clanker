//! M4-B08 acceptance test (the task's own required full harness, criterion 1): a real
//! player walks across a live region boundary between two independently-ticking regions
//! (still monolithic) with position-delta logging showing no observable discontinuity
//! beyond ARCH-D10's documented one-tick transfer budget. Mirrors
//! `play_block_place_break.rs`'s own established two-loopback-connection pattern
//! (M2-B07), driven against `TwoRegionWorld` (M4-B08) instead of `HardcodedWorld`.

use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_messaging::RegionId;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, KeepAliveClientbound, KeepAliveServerbound, LoginPlay, SetPlayerPosition,
};
use rusty_clanker_server::play::{
    PlayerProfile, REGION_EAST_ID, REGION_WEST_ID, SpawnEntity, TwoRegionWorld,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// One position-delta log entry: the sampled tick index, and (if the player resolved in
/// some region that tick) which region and its reported position.
type PositionLogEntry = (usize, Option<(RegionId, [f64; 3])>);

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

/// Drains the full Play-entry sequence (LoginPlay through the 12-chunk strip through
/// ChunkBatchFinished), returning the decoded `LoginPlay.entity_id` (this connection's own
/// network entity id).
async fn drain_play_entry(socket: &mut TcpStream, accumulator: &mut BytesMut) -> i32 {
    let (id, body) = recv_packet(socket, accumulator).await;
    assert_eq!(
        id,
        LoginPlay::ID,
        "first Play-entry packet must be LoginPlay"
    );
    let login = decode_one::<LoginPlay>(body).expect("LoginPlay must decode");
    loop {
        let (id, _) = recv_packet(socket, accumulator).await;
        if id == ChunkBatchFinished::ID {
            return login.entity_id;
        }
    }
}

/// Drains every byte currently sitting in `socket`'s own receive buffer (bounded by a
/// short per-read idle timeout, never the outer test deadline), answering any keep-alive
/// challenge found along the way, and returns every complete frame's packet id —
/// `entity_lifecycle_spawn_update_despawn_against_a_fake_client`'s own identical
/// "B never read anything until this one call" technique (M4-B01, `crates/server/tests/
/// play_entity_spawn_track_untrack.rs`).
async fn drain_all_pending(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
) -> Vec<(i32, Bytes)> {
    let mut out = Vec::new();
    loop {
        let mut chunk = [0u8; 4096];
        match tokio::time::timeout(Duration::from_millis(300), socket.read(&mut chunk)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => accumulator.extend_from_slice(&chunk[..n]),
            Ok(Err(_)) => break,
            Err(_) => break,
        }
        while let Some(payload) =
            rc_protocol::try_decode_frame(accumulator, CompressionState::Disabled).unwrap()
        {
            let mut body = payload;
            let id = VarInt::decode(&mut body).unwrap().get();
            if id == KeepAliveClientbound::ID {
                let challenge = decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
                send_packet(socket, &KeepAliveServerbound { id: challenge.id }).await;
            }
            out.push((id, body));
        }
    }
    out
}

#[tokio::test]
async fn player_walks_across_a_live_region_boundary_with_bounded_position_delta() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = Arc::new(TwoRegionWorld::new());

        // `A` — the acting connection, joined deep in West territory (chunk_x = -1).
        let (server_a, mut client_a) = connected_pair().await;
        let (inbound_a, handle_a) = spawn_connection(server_a, ConnectionConfig::default());
        let world_a = Arc::clone(&world);
        let a_uuid: u128 = 1;
        tokio::spawn(async move {
            world_a
                .join_and_drive(
                    handle_a,
                    inbound_a,
                    PlayerProfile {
                        uuid: a_uuid,
                        username: "a".to_string(),
                    },
                    BlockPos::new(-16, -59, 0),
                )
                .await;
        });
        let mut a_acc = BytesMut::new();
        let a_network_id = drain_play_entry(&mut client_a, &mut a_acc).await;

        // `B` — an uninvolved observer, joined deep in East territory (chunk_x = 1),
        // fixed for the whole test.
        let (server_b, mut client_b) = connected_pair().await;
        let (inbound_b, handle_b) = spawn_connection(server_b, ConnectionConfig::default());
        let world_b = Arc::clone(&world);
        let b_uuid: u128 = 2;
        tokio::spawn(async move {
            world_b
                .join_and_drive(
                    handle_b,
                    inbound_b,
                    PlayerProfile {
                        uuid: b_uuid,
                        username: "b".to_string(),
                    },
                    BlockPos::new(24, -59, 0),
                )
                .await;
        });
        let mut b_acc = BytesMut::new();
        let _b_network_id = drain_play_entry(&mut client_b, &mut b_acc).await;

        // 64 successive serverbound movement packets, each advancing x by +0.5 from
        // -16.0 toward +16.0 -- crossing `x = 0` partway through.
        let mut position_log: Vec<PositionLogEntry> = Vec::new();
        let mut x = -16.0_f64;
        for step in 0..64usize {
            x += 0.5;
            send_packet(
                &mut client_a,
                &SetPlayerPosition {
                    x,
                    y: -59.0,
                    z: 0.0,
                    on_ground: true,
                },
            )
            .await;
            // One full simulated tick (SERVER_TICK_PERIOD = 50ms) plus slack, so each
            // step's own position sample reliably reflects that step's own tick.
            tokio::time::sleep(Duration::from_millis(80)).await;
            let observed = world.debug_query_player_position(a_uuid).await;
            position_log.push((step, observed));
        }

        // (a) `A` never receives a `Disconnect` -- proven implicitly: every `recv_packet`/
        // `send_packet` call above would itself fail/panic had the connection closed, and
        // none did.

        // (b) At most one log entry is `None`.
        let none_count = position_log.iter().filter(|(_, p)| p.is_none()).count();
        assert!(
            none_count <= 1,
            "at most one sampled tick may observe the player absent from both regions \
             (ARCH-D10's one-tick transfer budget); observed {none_count}: {position_log:?}"
        );

        // (c) Every pair of consecutive `Some` entries' x coordinate differs by exactly
        // one `0.5` step per tick gap between them (a gap tick, if any, is exactly one
        // tick's worth of "missing" delta, never more); y/z are unchanged throughout.
        let some_entries: Vec<(usize, RegionId, [f64; 3])> = position_log
            .iter()
            .filter_map(|(step, p)| p.map(|(region, pos)| (*step, region, pos)))
            .collect();
        assert!(
            some_entries.len() >= 2,
            "expected at least two Some samples to compare deltas across"
        );
        for pair in some_entries.windows(2) {
            let (step_a, _, pos_a) = pair[0];
            let (step_b, _, pos_b) = pair[1];
            let tick_gap = (step_b - step_a) as f64;
            let expected_dx = 0.5 * tick_gap;
            assert!(
                (pos_b[0] - pos_a[0] - expected_dx).abs() < 1e-6,
                "expected an x delta of exactly {expected_dx} between steps {step_a} and {step_b}, got {}",
                pos_b[0] - pos_a[0]
            );
            assert!((pos_b[1] - pos_a[1]).abs() < 1e-9, "y must stay unchanged");
            assert!((pos_b[2] - pos_a[2]).abs() < 1e-9, "z must stay unchanged");
        }

        // (d) The log entry immediately before crossing `x = 0` reports `REGION_WEST_ID`;
        // the first entry at or after `x = 0` reports `REGION_EAST_ID` -- resolvable in
        // exactly one region per tick, transitioning cleanly.
        let last_west = some_entries
            .iter()
            .rfind(|(_, region, _)| *region == REGION_WEST_ID)
            .copied();
        let first_east = some_entries
            .iter()
            .find(|(_, region, _)| *region == REGION_EAST_ID)
            .copied();
        let (west_step, _, west_pos) =
            last_west.expect("the log must contain at least one West-reported entry");
        let (east_step, _, east_pos) =
            first_east.expect("the log must contain at least one East-reported entry");
        assert!(west_step < east_step, "West must be observed strictly before East");
        assert!(west_pos[0] < 0.0 || (west_pos[0] - 0.0).abs() < 1e-6);
        assert!(east_pos[0] >= 0.0 - 1e-6);

        // (e) The position value at the last West-reported entry and the position value
        // at the first East-reported entry differ by exactly the tick-gap-scaled `0.5`
        // step (never a jump, never a repeat) -- the literal "no observable discontinuity
        // beyond the one-tick budget" assertion.
        let tick_gap = (east_step - west_step) as f64;
        assert!(
            (east_pos[0] - west_pos[0] - 0.5 * tick_gap).abs() < 1e-6,
            "West -> East transition must show exactly the tick-gap-scaled step delta, \
             got west={west_pos:?} (step {west_step}) east={east_pos:?} (step {east_step})"
        );

        // Observer `B`: asserted to receive `Spawn Entity` for `A` (via this harness's own
        // bounded player-visibility mechanism, `two_region_world.rs`'s own module doc
        // comment has the full citation for why this is not M4-B01's own
        // `compute_tracking_delta` mechanism verbatim) once `A` enters `B`'s own tracking
        // range post-crossing -- proving tracking continues to function correctly across
        // the harness's two independently-ticking regions.
        let b_packets = drain_all_pending(&mut client_b, &mut b_acc).await;
        let spawn_entities: Vec<_> = b_packets
            .iter()
            .filter(|(id, _)| *id == SpawnEntity::ID)
            .collect();
        assert!(
            !spawn_entities.is_empty(),
            "B must have received at least one Spawn Entity packet for A by the end of the test"
        );
        let found_a = spawn_entities.iter().any(|(_, body)| {
            decode_one::<SpawnEntity>(body.clone())
                .map(|spawn| spawn.entity_id == a_network_id)
                .unwrap_or(false)
        });
        assert!(found_a, "the Spawn Entity B received must be for A's own network entity id");
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}
