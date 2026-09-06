//! M4-B04 field-report test-authoring: the negative-case counterpart to
//! `mob_spawn_cycle_integration.rs` (which must stay green, unmodified, and proves the
//! positive case — this file's own final report has the full citation). A real loopback
//! player joins `HardcodedWorld` configured with `WorldConfig.game_rules.spawn_mobs =
//! false`; over a real tick loop and connection, no `Spawn Entity` packet naming a Cow or
//! Zombie may arrive within the identical 45 s poll window
//! `natural_spawn_cycle_produces_tracked_entities_end_to_end` uses to prove the positive
//! case — connection setup, packet plumbing, and poll shape are otherwise copied from that
//! file verbatim so the only behavioral variable between the two tests is the game rule
//! itself.
use bytes::{Bytes, BytesMut};
use rc_mechanics::game_rules::GameRules;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::config::WorldConfig;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, SpawnEntity, enter_play};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

fn temp_world_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rc-spawn-rule-field-report-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ))
}

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
async fn spawn_mobs_false_stops_every_natural_spawn_from_reaching_the_client() {
    tokio::time::timeout(Duration::from_secs(90), async {
        let config = WorldConfig {
            world_dir: temp_world_dir("main"),
            game_rules: GameRules {
                spawn_mobs: false,
                ..GameRules::default()
            },
            ..WorldConfig::default()
        };
        let world = HardcodedWorld::with_config(config);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // Same superflat ground `natural_spawn_cycle_produces_tracked_entities_end_to_end`
        // relies on to make the positive case's own spawns possible in the first place —
        // reused unmodified here so this test proves the game rule itself is what
        // suppresses spawning, not a world that happens to never spawn anyway.
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
            !natural_spawn_seen,
            "GameRules.spawn_mobs = false must suppress every naturally-spawned Cow's or \
             Zombie's own Spawn Entity packet for the full 45 s poll window"
        );
    })
    .await
    .expect("test timed out");
}
