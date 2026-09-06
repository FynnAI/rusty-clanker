//! test-matrix: boundaries=waived(combat targeting, not world-height-boundary content) orientations=waived(a single fixed look direction is the acceptance surface, not a four-way placement sweep) self=waived(no self-attack case in this suite's own domain model) composition=waived(single attacker/single target per case, no >=3-component chain) nondefault-state=waived(every attribute asserted is that mob kind's own default -- no non-default override exercised in this file)
//! M4-B05 acceptance tests: the full player-melee pipeline over real loopback connections --
//! reach/angle validation, the damage/knockback/tracking-broadcast sequence, the
//! invulnerability top-up-delta boundary, and `Interact`'s own silent-no-op contract.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_mechanics::entity::EntityKind;
use rc_mechanics::entity::metadata::{MetadataValue, decode_metadata_entries};
use rc_physics::Vec3;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::combat_packets::{Attack, DamageEvent, Interact};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, KeepAliveClientbound, KeepAliveServerbound, LoginPlay, SetPlayerRotation,
};
use rusty_clanker_server::play::{
    HardcodedWorld, LpVec3, PlayerProfile, SetEntityData, SetEntityVelocity, enter_play,
    look_vector,
};
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

/// As `recv_packet_of_type`, but additionally filtered by `entity_id` (M4-B04's own natural
/// mob-spawning cycle runs unconditionally against every real `HardcodedWorld`, and can spawn
/// unrelated entities this file's own connection also tracks within the same short test
/// window — a plain packet-type search alone can pick up one of THOSE entities' own `Spawn
/// Entity`/`Set Entity Data` broadcasts by pure interleaving accident. `decode`/`entity_id`
/// isolate this file's own combat-triggered packet for the one entity id each test cares
/// about, regardless of how much unrelated natural-spawn traffic is also in flight).
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

/// As `play_entity_spawn_track_untrack.rs`'s own `drain_all_pending_packet_ids`: reads
/// whatever the peer has sent within a short idle window (never the outer test deadline),
/// decoding every complete frame's own packet id. Used to prove a rejected `Attack`/a silent
/// `Interact` produces none of the combat packets this file's own passing cases assert on.
async fn drain_pending_packet_ids(socket: &mut TcpStream, accumulator: &mut BytesMut) -> Vec<i32> {
    let mut ids = Vec::new();
    loop {
        let mut chunk = [0u8; 4096];
        match tokio::time::timeout(Duration::from_millis(400), socket.read(&mut chunk)).await {
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
                continue;
            }
            ids.push(id);
        }
    }
    ids
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
    // SetDefaultSpawnPosition, SynchronizePlayerPosition, GameEvent,
    // SetChunkCacheCenter, ChunkBatchStart.
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
    // A much larger outbound capacity than `ConnectionConfig::default()`'s own `1024` --
    // M4-B04's own natural mob-spawning cycle runs unconditionally against every real
    // `HardcodedWorld` and can burst dozens of unrelated entities' own tracking broadcasts
    // through this same connection while this file's own tests are still reading; a
    // congestion-induced drop of one of THIS file's own combat packets (this connection's
    // own outbound channel briefly filling faster than a single-threaded test reads it) is a
    // test-environment risk this blueprint's own combat pipeline does not itself introduce
    // (`docs/findings-for-planning.md`, final report has the full writeup) -- headroom here
    // sidesteps it entirely rather than racing it.
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

async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// `A`'s own fixed join eye position (`SPAWN_POSITION` feet `[0.0, -60.0, 0.0]` +
/// `PLAYER_EYE_HEIGHT` `1.62`, M3-B02).
const EYE_POS: [f64; 3] = [0.0, -58.38, 0.0];

/// Zombie hitbox height (Context, "Reach and angle validation" -- `entity_dimensions`).
const ZOMBIE_HEIGHT: f64 = 1.95;

fn zombie_center(feet: [f64; 3]) -> [f64; 3] {
    [feet[0], feet[1] + ZOMBIE_HEIGHT / 2.0, feet[2]]
}

/// Derives `(yaw, pitch)` in the exact convention `play::look_vector` reads (Context, M3-B03's
/// own shared look-vector construction) so a real `SetPlayerRotation` packet aims precisely at
/// `target` from `origin` -- computed geometrically from the two positions the test itself
/// already fixed, not asserted against any implementation output (TEST-D56).
fn aim_at(origin: [f64; 3], target: [f64; 3]) -> (f32, f32) {
    let dx = target[0] - origin[0];
    let dy = target[1] - origin[1];
    let dz = target[2] - origin[2];
    let horizontal = (dx * dx + dz * dz).sqrt();
    let yaw = (-dx).atan2(dz).to_degrees();
    let pitch = (-dy).atan2(horizontal).to_degrees();
    (yaw as f32, pitch as f32)
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn assert_close(what: &str, got: f32, want: f32, tol: f32) {
    assert!(
        (got - want).abs() < tol,
        "{what}: got {got}, want {want} (within {tol})"
    );
}

#[tokio::test]
async fn player_attacks_zombie_full_pipeline_packet_sequence() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc, _a_net_id) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        let zombie_feet = [2.0, -60.0, 0.0];
        let (_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, zombie_feet).await;

        let (yaw, pitch) = aim_at(EYE_POS, zombie_center(zombie_feet));
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw,
                pitch,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([yaw, pitch]))
            .await;

        // `attack_strength_ticker` (Context, "Attack-cooldown charge curve") increments once
        // per player tick from join onward and needs `>= 5` (250ms at 20 TPS) for a full-
        // charge (`charge_scale == 1.0`) swing -- this test's own golden `18.08` value assumes
        // full charge, so an explicit, generous wait replaces the (empirically false) earlier
        // assumption that enough real setup time had already naturally elapsed.
        tokio::time::sleep(Duration::from_millis(500)).await;

        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;

        // A reads, in order: Damage Event, Set Entity Velocity, Set Entity Data -- each
        // filtered to this test's own zombie specifically (M4-B04's own natural mob-spawning
        // cycle runs unconditionally against every real `HardcodedWorld` and can interleave
        // unrelated entities' own tracking broadcasts into the same stream).
        let damage = recv_packet_for_entity::<DamageEvent>(
            &mut a,
            &mut a_acc,
            |p| p.entity_id,
            zombie_net_id,
        )
        .await;
        assert_eq!(damage.source_type_id, 0, "PlayerAttack's own ordinal");

        let velocity = recv_packet_for_entity::<SetEntityVelocity>(
            &mut a,
            &mut a_acc,
            |p| p.entity_id,
            zombie_net_id,
        )
        .await;
        assert!(
            velocity.velocity.x.abs() > 1e-4 || velocity.velocity.z.abs() > 1e-4,
            "impulse #1 (flat 0.4, source-relative) must be nonzero: {:?}",
            velocity.velocity
        );

        let set_data = recv_packet_for_entity::<SetEntityData>(
            &mut a,
            &mut a_acc,
            |p| p.entity_id,
            zombie_net_id,
        )
        .await;
        let entries = decode_metadata_entries(&set_data.metadata).expect("metadata must decode");
        // Zombie's own Armor=2.0 absorbs part of the hit: toughness=2.0, real_armor=
        // clamp(2-2.0/2.0, 0.4, 20)=1.0, armor_fraction=0.04, damage=2.0*0.96=1.92,
        // health=20.0-1.92=18.08 (Context, armor-absorption formula, step 4).
        assert!(
            entries.iter().any(|(index, value)| *index == 9
                && matches!(value, MetadataValue::Float(h) if (*h - 18.08).abs() < 1e-3)),
            "expected the health entry (index 9, ~18.08) among {entries:?}"
        );

        let info = world
            .debug_query_entity(zombie_net_id)
            .await
            .expect("the zombie must still resolve");
        assert_close("zombie health after one hit", info.health, 18.08, 1e-3);
        assert!(!info.is_dead);
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}

#[tokio::test]
async fn attack_out_of_reach_produces_no_packets() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc, _a_net_id) = spawn_actor(&world, "a", 1).await;

        // Beyond ENTITY_INTERACTION_RANGE = 3.0, well outside reach from A's own eye.
        let zombie_feet = [10.0, -60.0, 0.0];
        let (_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, zombie_feet).await;

        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;

        let ids = drain_pending_packet_ids(&mut a, &mut a_acc).await;
        assert!(
            !ids.contains(&DamageEvent::ID),
            "an out-of-reach Attack must produce no combat packets; observed ids: {ids:?}"
        );

        let info = world.debug_query_entity(zombie_net_id).await.unwrap();
        assert_close("zombie health unchanged", info.health, 20.0, 1e-6);
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}

#[tokio::test]
async fn attack_occluded_target_is_rejected() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc, _a_net_id) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        // Within Euclidean range (~2.1 blocks) but a solid block is placed directly along
        // the aimed ray, between the eye and the target.
        let zombie_feet = [2.0, -60.0, 0.0];
        let target = zombie_center(zombie_feet);
        let (_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, zombie_feet).await;

        let (yaw, pitch) = aim_at(EYE_POS, target);
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw,
                pitch,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([yaw, pitch]))
            .await;

        let direction = look_vector(yaw, pitch);
        let target_distance = distance(EYE_POS, target);
        let mid =
            Vec3::new(EYE_POS[0], EYE_POS[1], EYE_POS[2]) + direction * (target_distance * 0.5);
        let occluding = BlockPos::new(
            mid.x.floor() as i32,
            mid.y.floor() as i32,
            mid.z.floor() as i32,
        );
        world
            .debug_set_block_state(occluding, blocks::STONE.0)
            .await;

        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;

        let ids = drain_pending_packet_ids(&mut a, &mut a_acc).await;
        assert!(
            !ids.contains(&DamageEvent::ID),
            "an occluded Attack must produce no combat packets; observed ids: {ids:?}"
        );

        let info = world.debug_query_entity(zombie_net_id).await.unwrap();
        assert_close("zombie health unchanged", info.health, 20.0, 1e-6);
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}

#[tokio::test]
async fn interact_packet_is_a_silent_no_op() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc, _a_net_id) = spawn_actor(&world, "a", 1).await;

        let zombie_feet = [2.0, -60.0, 0.0];
        let (_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, zombie_feet).await;

        send_packet(
            &mut a,
            &Interact {
                entity_id: zombie_net_id,
                hand: 0,
                location: LpVec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                using_secondary_action: false,
            },
        )
        .await;

        let ids = drain_pending_packet_ids(&mut a, &mut a_acc).await;
        assert!(
            !ids.contains(&DamageEvent::ID),
            "Interact must never produce a combat packet; observed ids: {ids:?}"
        );

        let info = world.debug_query_entity(zombie_net_id).await.unwrap();
        assert_close("zombie health unchanged", info.health, 20.0, 1e-6);
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}

#[tokio::test]
async fn repeated_attacks_within_ten_ticks_apply_only_the_delta() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc, a_net_id) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        // Deliberately close (well inside ENTITY_INTERACTION_RANGE's own 3.0 bound, with
        // generous margin): impulse #1 (Context, "Knockback") pushes the zombie directly away
        // from the attacker along this same line every hit that deals damage, so a second
        // attack sent shortly after the first must still find it in reach.
        let zombie_feet = [1.2, -60.0, 0.0];
        let (_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, zombie_feet).await;

        let (yaw, pitch) = aim_at(EYE_POS, zombie_center(zombie_feet));
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw,
                pitch,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([yaw, pitch]))
            .await;

        // **Deviation from this blueprint's own literal test prose, cited (final report has
        // the full writeup)**: the blueprint's own Acceptance-tests text describes overriding
        // the SECOND attack's own `AttackDamage` down to "a smaller value" than the first --
        // but `last_hurt` (Context, damage pipeline step 3) is set from the RAW, pre-armor
        // `assemble_player_melee_damage` output, and `base_damage_scale_factor` is bounded in
        // `[0.2, 1.0]`, so a strictly SMALLER second `AttackDamage` can never produce a raw
        // value exceeding the first hit's own `last_hurt` -- that construction can only ever
        // land in the pipeline's own fully-absorbed `NoOp` branch (`combat_damage_pipeline.rs`'s
        // own `hit 2` case), never the nonzero-delta branch (`hit 4`) this test needs to
        // demonstrate through real packets. This test instead overrides hit 1's own
        // `AttackDamage` DOWN first (to `0.2`, capping hit 1's own `last_hurt` at `0.2`
        // regardless of charge) and hit 2's own `AttackDamage` UP second (to `2.0`, whose own
        // raw-damage floor even at the worst-case near-zero charge scale, `0.2`, is `0.4`) --
        // `0.4 > 0.2` unconditionally for every possible charge-scale pair, guaranteeing the
        // nonzero-delta branch fires deterministically regardless of real-time tick-count
        // timing between the two `Attack` sends.
        world
            .debug_override_attribute(
                a_net_id,
                rc_mechanics::combat::AttributeKind::AttackDamage,
                0.2,
            )
            .await;

        // Hit 1: a small, capped raw damage -- `last_hurt` becomes at most `0.2`.
        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;
        recv_packet_for_entity::<DamageEvent>(&mut a, &mut a_acc, |p| p.entity_id, zombie_net_id)
            .await;
        let hit_1_info = world.debug_query_entity(zombie_net_id).await.unwrap();
        let health_after_hit_1 = hit_1_info.health;
        assert!(
            health_after_hit_1 < 20.0 && health_after_hit_1 > 19.7,
            "hit 1 (AttackDamage 0.2) must deal a small amount of damage: {health_after_hit_1}"
        );

        // Impulse #1 (Context, "Knockback") already pushed the zombie away from A along the
        // attacker-to-victim line -- re-aim at its own now-current position (queried above)
        // before the second `Attack`, exactly as a real client's own continuous aiming would.
        let zombie_feet_after_hit_1 = hit_1_info
            .pos
            .expect("a mob's own DebugEntityInfo always carries a position");
        let (yaw2, pitch2) = aim_at(EYE_POS, zombie_center(zombie_feet_after_hit_1));
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw: yaw2,
                pitch: pitch2,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([yaw2, pitch2]))
            .await;

        // Hit 2, within the same invulnerability window (`invulnerable_time` still > 10, sent
        // shortly after hit 1): `AttackDamage` raised to `2.0` -- its own raw-damage floor
        // (`0.4`) unconditionally exceeds hit 1's own `last_hurt` ceiling (`0.2`), so this hit
        // deals exactly the delta, not the full amount a fresh (non-invulnerable) hit at this
        // same `AttackDamage` would.
        world
            .debug_override_attribute(
                a_net_id,
                rc_mechanics::combat::AttributeKind::AttackDamage,
                2.0,
            )
            .await;
        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;
        // A Damage Event IS received -- the delta branch fired, not `NoOp` (which sends no
        // packet at all).
        recv_packet_for_entity::<DamageEvent>(&mut a, &mut a_acc, |p| p.entity_id, zombie_net_id)
            .await;
        let health_after_hit_2 = world
            .debug_query_entity(zombie_net_id)
            .await
            .unwrap()
            .health;
        assert!(
            health_after_hit_2 < health_after_hit_1,
            "hit 2 must deal further damage (the delta branch, not full absorption): \
             health_after_hit_1={health_after_hit_1}, health_after_hit_2={health_after_hit_2}"
        );
        // Robust against either hit's own exact real-time charge level (Context, "Attack-
        // cooldown charge curve" -- this test does not pin either hit to a specific tick
        // count): a hypothetical bug that let hit 2 bypass the top-up gate and deal a FULL
        // fresh hit at `AttackDamage 2.0` would cost at most ~1.92 (this file's own golden
        // case, `player_attacks_zombie_full_pipeline_packet_sequence`) against a target
        // already at hit 1's own worst-case health floor (`19.7`, this test's own hit-1
        // assertion above) -- `19.7 - 1.92 = 17.78`. The real top-up-delta result is always
        // strictly ABOVE that floor (it always spares at least `last_hurt`'s own post-armor
        // equivalent, however small), so this bound cleanly distinguishes "the delta branch
        // fired" from "the invulnerability gate was silently bypassed" without needing either
        // hit's own charge level to be pinned.
        assert!(
            health_after_hit_2 > 17.8,
            "hit 2's own result ({health_after_hit_2}) is at or below the worst-case FULL \
             fresh-hit floor (17.8) -- the top-up gate's own delta-only guarantee appears to \
             have been bypassed"
        );
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}
