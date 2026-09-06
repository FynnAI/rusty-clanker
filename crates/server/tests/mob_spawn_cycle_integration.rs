//! M4-B04 acceptance test: the natural mob-spawning cycle produces real, ECS-spawned
//! entities that reach a real connected client through M4-B01's already-shipped
//! tracking pipeline, unmodified — over a real loopback connection and
//! `HardcodedWorld`'s own real tick loop, mirroring `play_entity_drop_pipeline.rs`'s own
//! established connection-setup shape.
//!
//! **Documented test-design note** (final report has the full citation): the blueprint's
//! own acceptance-test text names "a fixed seed known via a short pre-computed trace to
//! produce at least one spawn within 50 ticks." Hand-deriving such a trace is
//! impractical without an executable reference; this test instead polls generously for
//! the first `Spawn Entity` packet naming a naturally spawnable tier-2 kind. With Stage
//! 8's light engine wired into `HardcodedWorld` (M4-B07 field-report changeset) the
//! superflat surface is fully sunlit, so the Monster darkness gate keeps Zombies off it
//! and the Creature path is the one that fires: Cows are legal on the lit grass but the
//! persistent categories only spawn on the `tick % 400 == 0` cadence, so the poll window
//! covers more than two of those ticks (45 s); a Zombie is still accepted should the RNG
//! land one in an unlit cell.
use bytes::{Bytes, BytesMut};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, SpawnEntity, enter_play};
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
    use rusty_clanker_server::play::packets::{KeepAliveClientbound, KeepAliveServerbound};
    let (id, body) = recv_packet(socket, accumulator).await;
    if id == KeepAliveClientbound::ID {
        let challenge = decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
        send_packet(socket, &KeepAliveServerbound { id: challenge.id }).await;
    }
    (id, body)
}

async fn drain_play_entry(socket: &mut TcpStream, accumulator: &mut BytesMut) {
    use rusty_clanker_server::play::packets::ChunkBatchFinished;
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

const TICK: Duration = Duration::from_millis(50);

#[tokio::test]
async fn natural_spawn_cycle_produces_tracked_entities_end_to_end() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // Superflat ground (bedrock@-64, dirt -63..=-62, grass@-61, open air from -60
        // upward, M1-B05's own layer table) already gives this test a real, legal
        // `ON_GROUND` placement band -- no `debug_set_block_state` needed.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
        let zombie_id = rc_mechanics::entity::EntityKind::Zombie.registry_id().0 as i32;
        let cow_id = rc_mechanics::entity::EntityKind::Cow.registry_id().0 as i32;
        let mut natural_spawn_seen = false;
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(TICK, recv_clientbound(&mut a, &mut a_acc)).await {
                Ok((id, body)) if id == SpawnEntity::ID => {
                    let spawn = decode_one::<SpawnEntity>(body).unwrap();
                    if spawn.entity_type == zombie_id || spawn.entity_type == cow_id {
                        natural_spawn_seen = true;
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }

        assert!(
            natural_spawn_seen,
            "expected at least one naturally-spawned Cow's or Zombie's own Spawn Entity \
             packet to reach the client within the poll window"
        );
    })
    .await
    .expect("test timed out");
}
