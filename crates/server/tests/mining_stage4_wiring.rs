//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only))
//! M3-B03 acceptance test: proves Stage 4 (M3-B01's substrate, wired into `HardcodedWorld`'s
//! live tick loop for the first time by this blueprint) is inert in the steady state under
//! this milestone's own tier-1 scope, and settles fully within the same tick an ordinary
//! break is processed. See `blueprints/M3/M3-B03-breaking-placing.md`, Acceptance tests,
//! "`crates/server/tests/mining_stage4_wiring.rs`".

use bytes::BytesMut;
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SetPlayerRotation, pack_position,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, Stage4Counters, enter_play};
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

async fn recv_packet(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, bytes::Bytes) {
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

async fn recv_clientbound(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
) -> (i32, bytes::Bytes) {
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
    let world_for_task = world.clone();
    let profile = PlayerProfile {
        uuid,
        username: username.to_string(),
    };
    tokio::spawn(async move {
        enter_play(handle, inbound, profile, &world_for_task).await;
    });
    let mut accumulator = BytesMut::new();
    drain_play_entry(&mut client, &mut accumulator).await;
    (client, accumulator)
}

async fn recv_packet_of_type(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    expected_id: i32,
) -> bytes::Bytes {
    loop {
        let (id, body) = recv_clientbound(socket, accumulator).await;
        if id == expected_id {
            return body;
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

fn assert_idle(counters: Stage4Counters) {
    assert!(
        counters.neighbor_engine_idle,
        "NeighborUpdateEngine must be idle: {counters:?}"
    );
    assert_eq!(counters.block_ticks_pending, 0);
    assert_eq!(counters.fluid_ticks_pending, 0);
    assert_eq!(counters.block_events_pending_next_tick, 0);
}

#[tokio::test]
async fn stage4_is_inert_with_no_registered_behavior() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();

        // Ten ordinary ticks, no block actions sent at all -- Stage 4 now runs for real
        // every tick (this blueprint's own wiring), but under this milestone's own tier-1
        // scope (no fluids, no leaf decay, no real redstone behavior registered) it has
        // nothing to do.
        for _ in 0..10 {
            assert_idle(
                world
                    .debug_stage4_counters()
                    .await
                    .expect("the hardcoded region stays alive for this test's own duration"),
            );
        }

        // One ordinary break, then the same counters must return to idle again within the
        // same tick the break was processed -- `mining::settle_neighbor_updates`'s own
        // fixed-point drain never leaves residual pending work for Stage 4 to discover.
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid_a, |r| r.data.rotation) == Some([0.0, 90.0]))
            .await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(0, -61, 0)),
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
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        assert_eq!(
            decode_one::<BlockUpdate>(body).unwrap().location,
            pack_position(BlockPos::new(0, -61, 0))
        );

        assert_idle(
            world
                .debug_stage4_counters()
                .await
                .expect("the hardcoded region stays alive for this test's own duration"),
        );
    })
    .await
    .unwrap();
}
