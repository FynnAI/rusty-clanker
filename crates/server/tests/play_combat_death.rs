//! test-matrix: boundaries=waived(combat death handling, not world-height-boundary content) orientations=waived(no placement/facing content in this file's own domain model) self=waived(no self-kill case in this suite's own domain model) composition=waived(single mob or single player death per case, no >=3-component chain) nondefault-state=waived(every attribute asserted is that mob kind's own default -- no non-default override exercised in this file)
//! M4-B05 acceptance tests: death detection, the mob loot-drop/despawn sequence (both
//! observers), and the player-death packet pair without despawn.

use bytes::{Bytes, BytesMut};
use rc_mechanics::entity::EntityKind;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::combat_packets::{DamageEvent, EntityEvent, PlayerCombatKill};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, KeepAliveClientbound, KeepAliveServerbound, LoginPlay, SetHealth,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, RemoveEntities, enter_play};
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

/// As `play_combat_melee_flow.rs`'s own `recv_packet_for_entity`: filters by `entity_id` so
/// M4-B04's own unconditional natural mob-spawning cycle cannot be mistaken for this file's
/// own combat-triggered packets.
async fn recv_packet_for_entity<T: RcPacket>(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    entity_id_of: impl Fn(&T) -> i32,
    expected_entity_id: i32,
) -> T {
    loop {
        let body = recv_packet_of_type(socket, accumulator, T::ID).await;
        let Ok(packet) = decode_one::<T>(body) else {
            continue;
        };
        if entity_id_of(&packet) == expected_entity_id {
            return packet;
        }
    }
}

async fn recv_remove_for(socket: &mut TcpStream, accumulator: &mut BytesMut, expected_id: i32) {
    loop {
        let body = recv_packet_of_type(socket, accumulator, RemoveEntities::ID).await;
        let Ok(packet) = decode_one::<RemoveEntities>(body) else {
            continue;
        };
        if packet.entity_ids.iter().any(|id| id.0 == expected_id) {
            return;
        }
    }
}

async fn drain_play_entry_capturing_network_id(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
) -> i32 {
    let (id, body) = recv_packet(socket, accumulator).await;
    assert_eq!(
        id,
        LoginPlay::ID,
        "first Play-entry packet must be LoginPlay"
    );
    let login = decode_one::<LoginPlay>(body).expect("LoginPlay must decode");
    for _ in 0..5 {
        recv_packet(socket, accumulator).await;
    }
    loop {
        let (id, _) = recv_packet(socket, accumulator).await;
        if id == ChunkBatchFinished::ID {
            return login.entity_id;
        }
    }
}

async fn spawn_actor(
    world: &HardcodedWorld,
    username: &str,
    uuid: u128,
) -> (TcpStream, BytesMut, i32) {
    let (server, mut client) = connected_pair().await;
    // See `play_combat_melee_flow.rs`'s own identical rationale: headroom against M4-B04's
    // own unconditional natural mob-spawning traffic on this same connection.
    let (inbound, handle) = spawn_connection(
        server,
        ConnectionConfig {
            outbound_capacity: 16_384,
            ..ConnectionConfig::default()
        },
    );
    let world = world.clone();
    let profile = PlayerProfile {
        uuid,
        username: username.to_string(),
    };
    tokio::spawn(async move {
        enter_play(handle, inbound, profile, &world).await;
    });
    let mut accumulator = BytesMut::new();
    let network_id = drain_play_entry_capturing_network_id(&mut client, &mut accumulator).await;
    (client, accumulator, network_id)
}

#[tokio::test]
async fn zombie_death_drops_loot_and_despawns() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc, _a_net_id) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc, _b_net_id) = spawn_actor(&world, "b", 2).await;

        let (_, zombie_net_id) = world
            .debug_spawn_mob(EntityKind::Zombie, [2.0, -60.0, 0.0])
            .await;

        // Both A and B are within the zombie's own tracking range from the moment it spawns
        // (both join at `SPAWN_POSITION`) -- give the tracking pipeline a moment to discover
        // it before dealing the killing blow, so both observers are already tracking it when
        // death fires.
        tokio::time::sleep(Duration::from_millis(300)).await;

        // A single, directly-lethal hit (Context, "Mob spawning -- a debug-only entry
        // point": `debug_deal_damage`'s own synthetic `Starve`-typed source bypasses armor
        // entirely) -- a cited, necessary deviation from this blueprint's own literal test
        // setup ("override MaxHealth to 2.0 then deal 2.0 damage"): `debug_override_attribute`
        // only ever mutates the live `AttributeMap`, never retroactively resets the already-
        // spawned `LivingEntity.health` field to match a newly-lowered `MaxHealth` (no
        // production code path ties the two together), so that construction would only ever
        // bring health from 20.0 to 18.0, never to a lethal <= 0.0. A single, sufficiently
        // large direct hit achieves the same acceptance goal (a real death-detection/loot/
        // despawn sequence) without relying on that unimplemented linkage.
        let ok = world.debug_deal_damage(zombie_net_id, 25.0).await;
        assert!(ok, "debug_deal_damage must resolve the zombie");

        for (socket, acc) in [(&mut a, &mut a_acc), (&mut b, &mut b_acc)] {
            recv_packet_for_entity::<DamageEvent>(socket, acc, |p| p.entity_id, zombie_net_id)
                .await;
            let event =
                recv_packet_for_entity::<EntityEvent>(socket, acc, |p| p.entity_id, zombie_net_id)
                    .await;
            assert_eq!(event.event_id, 3, "event_id 3 is the death animation");

            // **Deviation from this blueprint's own literal test prose, cited**: the
            // blueprint's own text asserts "one or more `Spawn Entity` packets" for the
            // dropped rotten flesh, but `FixedTierTwoLoot`'s own zombie roll is `0..=2`
            // (`combat_death_loot.rs`'s own unit-level acceptance range, `fixed_tier_two_
            // loot_zombie_bounded_range`) -- a real, honest one-in-three chance of a ZERO-item
            // roll on any given death, which a hard "at least one Spawn Entity" assertion here
            // would make this test flaky against. This test does not assert on the loot
            // Spawn Entity packet(s) at all (`recv_remove_for`, below, transparently skips
            // over any that do arrive while searching for the zombie's own `Remove Entities`
            // specifically) -- the loot-roll math itself is already the unit tests' own job.
            recv_remove_for(socket, acc, zombie_net_id).await;
        }

        let info = world.debug_query_entity(zombie_net_id).await;
        assert!(
            info.is_none(),
            "the zombie must be fully despawned (EntityIndex/NetworkEntityIndex no longer \
             resolve it): {info:?}"
        );
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}

#[tokio::test]
async fn player_death_sends_combat_kill_and_marks_dead_without_despawn() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc, a_net_id) = spawn_actor(&world, "a", 1).await;

        // A joining player defaults to Creative (`GameModeState{instabuild: true}`, M1-B05's
        // own hardcoded default) -- `apply_damage_pipeline`'s own very first check (Context,
        // "Damage invulnerability gate") makes a creative player fully invulnerable, so this
        // test must switch A to survival first or `debug_deal_damage` would silently resolve
        // to `DamageOutcome::Invulnerable` and never send anything at all.
        world.debug_set_survival(a_net_id, true).await;

        let ok = world.debug_deal_damage(a_net_id, 25.0).await;
        assert!(ok, "debug_deal_damage must resolve the player");

        let health_body = recv_packet_of_type(&mut a, &mut a_acc, SetHealth::ID).await;
        let set_health = decode_one::<SetHealth>(health_body).expect("SetHealth must decode");
        assert_eq!(set_health.health, 0.0);

        let kill_body = recv_packet_of_type(&mut a, &mut a_acc, PlayerCombatKill::ID).await;
        let kill = decode_one::<PlayerCombatKill>(kill_body).expect("PlayerCombatKill must decode");
        assert_eq!(kill.player_id, a_net_id);

        // The connection remains open and `debug_query_entity` still resolves the (now dead)
        // player (Context, "Death -- Player death": never removed from `NetworkEntityIndex`).
        let info = world
            .debug_query_entity(a_net_id)
            .await
            .expect("a dead player still resolves");
        assert!(info.is_dead);
        assert!(
            info.rc_entity_id.is_none(),
            "a player never carries an RcEntityId"
        );

        // A subsequent Attack from the dead player produces no further combat packets.
        send_packet(
            &mut a,
            &rusty_clanker_server::play::combat_packets::Attack {
                entity_id: a_net_id,
            },
        )
        .await;
        let mut saw_more = false;
        loop {
            let mut chunk = [0u8; 4096];
            match tokio::time::timeout(Duration::from_millis(400), a.read(&mut chunk)).await {
                Ok(Ok(0)) => break,
                Ok(Ok(n)) => {
                    a_acc.extend_from_slice(&chunk[..n]);
                    while let Some(payload) =
                        rc_protocol::try_decode_frame(&mut a_acc, CompressionState::Disabled)
                            .unwrap()
                    {
                        let mut body = payload;
                        let id = VarInt::decode(&mut body).unwrap().get();
                        if id == KeepAliveClientbound::ID {
                            let challenge =
                                decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
                            send_packet(&mut a, &KeepAliveServerbound { id: challenge.id }).await;
                            continue;
                        }
                        if id == DamageEvent::ID || id == SetHealth::ID {
                            saw_more = true;
                        }
                    }
                }
                Ok(Err(_)) => break,
                Err(_) => break,
            }
        }
        assert!(
            !saw_more,
            "a dead player's own further Attack must never produce a combat packet"
        );
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}
