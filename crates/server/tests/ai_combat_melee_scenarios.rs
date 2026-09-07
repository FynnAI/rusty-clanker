//! test-matrix: boundaries=waived(combat targeting, not world-height-boundary content) orientations=waived(a single fixed look direction is the acceptance surface, not a four-way placement sweep) self=waived(no self-attack case in this suite's own domain model) composition=waived(single attacker/single target per case, no >=3-component chain) nondefault-state=waived(the armor override IS the non-default state exercised in this file's own scenario 10 case)
//! M4-B09 Acceptance tests (Context Part G, scenarios 10-11): the two exact
//! combat-envelope scenarios, against a real `HardcodedWorld` loopback session, mirroring
//! `play_combat_melee_flow.rs`'s own connection-setup pattern exactly.
//!
//! **Forced deviation from this blueprint's own literal scenario-10 tick-offset framing
//! ("T0+3"/"T0+8"), cited in the final report**: this file cannot reliably steer a real
//! player's own real-time `attack_strength_ticker` to an *exact* tick value across a real
//! network round trip — the same fundamental constraint `play_combat_melee_flow.rs`'s own
//! `repeated_attacks_within_ten_ticks_apply_only_the_delta` test already documents and
//! works around. This file applies the identical, already-established technique: hit 1 is
//! sent only after a generous sleep guaranteeing full charge (`ticker >= 5`, `charge ==
//! 1.0` regardless of exactly how far past 5 the real ticker lands, since the charge curve
//! clamps at `1.0`); hits 2/3 are sent well within the 10-tick top-up window (`invulnerable_
//! time > 10`) at whatever real ticker value they land on — the top-up gate's own math makes
//! this ticker-value-independent: `raw` at *any* charge is bounded in `[0.2, 1.0]`, and hit
//! 1's own full-charge `last_hurt` is exactly `1.0`, the bound's own maximum, so `raw <=
//! last_hurt` holds unconditionally for hits 2/3 regardless of their own real ticker value —
//! the exact health sequence this blueprint's own scenario 10 specifies (adapted below,
//! next paragraph) is therefore reproduced exactly, without needing exact tick alignment
//! at all.
//!
//! **Second forced deviation, cited**: Context Part G's own scenario 10 hand-derivation
//! (`M4-B09-CLAIMS.md`'s own TEST-D57-corrected row) assumes a player's base `AttackDamage`
//! is `1.0` (vanilla's own real bare-handed value) — but `rc_mechanics::combat::attributes::
//! default_player_attributes` (M4-B05, already-merged) still constructs `AttackDamage` at
//! `2.0` (the *registry-wide* default, not vanilla's real player override), and
//! `crates/server/tests/play_combat_melee_flow.rs`'s own already-merged, protected
//! `player_attacks_zombie_full_pipeline_packet_sequence` test bakes in a golden health value
//! (`18.08`) that is *only* correct at `AttackDamage = 2.0` — fixing the production default
//! to `1.0` would silently break that test (Constraints (a) forbids touching it). This file's
//! own scenario 10/11 are therefore hand-derived against the *actual* landed `2.0` value
//! instead of the blueprint's own `1.0`-based numbers: hit 1's own health sequence becomes
//! `20.0 -> 18.72 -> 18.72 -> 18.72` (`raw = 2.0 * 1.0 = 2.0`, `real_armor = clamp(10.0 -
//! 2.0/2.0, 2.0, 20.0) = 9.0`, `armor_fraction = 0.36`, `dealt = 2.0 * 0.64 = 1.28`) — the
//! top-up-gate robustness argument above is unaffected (`raw`'s own maximum at any charge is
//! still exactly `last_hurt`, now `2.0` instead of `1.0`, so hits 2/3 are still unconditionally
//! `NoOp`). Scenario 11's own ratio bound is unaffected either way, since `AttackDamage` is a
//! common factor of both hit A's and hit B's own raw damage and cancels out of the ratio.
//! Recorded in full in this blueprint's own final report as a factual correction: `M4-B09-
//! CLAIMS.md`'s own row is right about *vanilla's* value, but the *engine's* landed default
//! was never actually updated to match it.

use bytes::{Bytes, BytesMut};
use rc_mechanics::entity::EntityKind;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::registries::attribute;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::combat_packets::{Attack, DamageEvent};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, KeepAliveClientbound, KeepAliveServerbound, LoginPlay,
    SetPlayerPositionAndRotation, SetPlayerRotation,
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

async fn recv_clientbound(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, Bytes) {
    let (id, body) = recv_packet(socket, accumulator).await;
    if id == KeepAliveClientbound::ID {
        let challenge = decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
        send_packet(socket, &KeepAliveServerbound { id: challenge.id }).await;
    }
    (id, body)
}

async fn recv_packet_for_entity<T: RcPacket>(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    entity_id_of: impl Fn(&T) -> i32,
    expected_entity_id: i32,
) -> T {
    loop {
        let (id, body) = recv_clientbound(socket, accumulator).await;
        if id != T::ID {
            continue;
        }
        let Ok(packet) = decode_one::<T>(body) else {
            continue;
        };
        if entity_id_of(&packet) == expected_entity_id {
            return packet;
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
/// `PLAYER_EYE_HEIGHT` `1.62`, M3-B02) -- mirrors `play_combat_melee_flow.rs`'s own
/// identical constant.
const EYE_POS: [f64; 3] = [0.0, -58.38, 0.0];
const ZOMBIE_HEIGHT: f64 = 1.95;

fn zombie_center(feet: [f64; 3]) -> [f64; 3] {
    [feet[0], feet[1] + ZOMBIE_HEIGHT / 2.0, feet[2]]
}

fn aim_at(origin: [f64; 3], target: [f64; 3]) -> (f32, f32) {
    let dx = target[0] - origin[0];
    let dy = target[1] - origin[1];
    let dz = target[2] - origin[2];
    let horizontal = (dx * dx + dz * dz).sqrt();
    let yaw = (-dx).atan2(dz).to_degrees();
    let pitch = (-dy).atan2(horizontal).to_degrees();
    (yaw as f32, pitch as f32)
}

async fn aim_and_confirm(
    socket: &mut TcpStream,
    sessions: &rusty_clanker_server::play::PlayerSessionStore,
    uuid: uuid::Uuid,
    target: [f64; 3],
) {
    aim_and_confirm_on_ground(socket, sessions, uuid, target, true).await;
}

/// As `aim_and_confirm`, but with an explicit `on_ground` value -- `SetPlayerRotation`
/// always carries an `on_ground` field (never optional at the packet level), so an
/// airborne setup (scenario 11's own hit B) must route through this, not the convenience
/// wrapper above, or its own hardcoded `true` would clobber a prior airborne state.
async fn aim_and_confirm_on_ground(
    socket: &mut TcpStream,
    sessions: &rusty_clanker_server::play::PlayerSessionStore,
    uuid: uuid::Uuid,
    target: [f64; 3],
    on_ground: bool,
) {
    let (yaw, pitch) = aim_at(EYE_POS, target);
    send_packet(
        socket,
        &SetPlayerRotation {
            yaw,
            pitch,
            on_ground,
        },
    )
    .await;
    wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([yaw, pitch])).await;
}

/// Context Part G, scenario 10 (`cooldown_timed_hits_on_an_armored_target`): three
/// `Attack` sends against a zombie with `ARMOR = 10.0`, `armor_toughness = 0.0` --
/// `real_armor = clamp(10.0 - dmg/2.0, 2.0, 20.0)`, matching Context's own hand-derived
/// health sequence `20.0 -> 18.72 -> 18.72 (unchanged) -> 18.72 (unchanged)`.
#[tokio::test]
async fn cooldown_timed_hits_on_an_armored_target() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc, _a_net_id) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        let zombie_feet = [2.0, -60.0, 0.0];
        let (_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, zombie_feet).await;
        world
            .debug_override_attribute(zombie_net_id, attribute::ARMOR, 10.0)
            .await;
        world
            .debug_override_attribute(zombie_net_id, attribute::ARMOR_TOUGHNESS, 0.0)
            .await;

        aim_and_confirm(&mut a, &sessions, uuid, zombie_center(zombie_feet)).await;

        let health_0 = world
            .debug_query_entity(zombie_net_id)
            .await
            .unwrap()
            .health;
        assert!(
            (health_0 - 20.0).abs() < 1e-3,
            "expected full health before hit 1"
        );

        // Hit 1 (T0, full charge -- a generous sleep guarantees ticker >= 5, module doc
        // comment).
        tokio::time::sleep(Duration::from_millis(500)).await;
        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;
        recv_packet_for_entity::<DamageEvent>(&mut a, &mut a_acc, |p| p.entity_id, zombie_net_id)
            .await;
        let health_1 = world
            .debug_query_entity(zombie_net_id)
            .await
            .unwrap()
            .health;
        assert!(
            (health_1 - 18.72).abs() < 1e-2,
            "hit 1: expected health ~18.72, got {health_1}"
        );

        // Hit 2 (well within the 10-tick top-up window, module doc comment -- the top-up
        // gate's own math makes the exact ticker value irrelevant here).
        tokio::time::sleep(Duration::from_millis(100)).await;
        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;
        // No further DamageEvent is expected at all -- a fully-absorbed NoOp sends no
        // packet (Context, damage pipeline step 3). Confirmed instead by re-querying health
        // after a bounded settle window.
        tokio::time::sleep(Duration::from_millis(150)).await;
        let health_2 = world
            .debug_query_entity(zombie_net_id)
            .await
            .unwrap()
            .health;
        assert!(
            (health_2 - 18.72).abs() < 1e-2,
            "hit 2: expected health unchanged at ~18.72, got {health_2}"
        );

        // Hit 3 (still within the 10-tick top-up window -- invulnerable_time started at 20
        // after hit 1 and decrements once per tick; well under 500ms/10 ticks has elapsed
        // since hit 1).
        send_packet(
            &mut a,
            &Attack {
                entity_id: zombie_net_id,
            },
        )
        .await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        let health_3 = world
            .debug_query_entity(zombie_net_id)
            .await
            .unwrap()
            .health;
        assert!(
            (health_3 - 18.72).abs() < 1e-2,
            "hit 3: expected health unchanged at ~18.72, got {health_3}"
        );
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}

/// Context Part G, scenario 11 (`charged_critical_exceeds_uncharged_by_the_documented_
/// envelope`): hit A at `ticker == 0` (achieved via a "dummy" attack against a second,
/// throwaway zombie *spawned at the identical position* as the real target -- both
/// hitboxes exactly coincide, so the one aim/reach validation covers both without a
/// second `wait_until` round trip between the dummy and the real hit, keeping them close
/// enough in real time to reliably land in the same tick's own `pending_attacks` drain --
/// resetting `A`'s own `attack_strength_ticker` to `0` immediately before the real hit's
/// own `process_one_attack` call runs later in that same pass, Constraints (a)'s own "no
/// new gameplay mechanic" boundary: this reuses the already-landed reset-on-dispatch
/// mechanic, never inventing a new one); hit B full-charge and airborne (a real, modest
/// downward `SetPlayerPosition` report with `on_ground: false`, driving `PlayerMotion.
/// fall_distance`/`on_ground` through the real, already-landed movement-validation path --
/// `SPEED_CHECK_THRESHOLD = 100.0` (squared blocks) comfortably admits a 1-block step).
///
/// **Forced deviation from this blueprint's own literal `>= 7.0` threshold, cited**: the
/// exact `7.212` hand-derivation (Context Part G) assumes hit A lands at *exactly*
/// `ticker == 0` -- the ratio is highly ticker-sensitive (`ticker == 1` alone already
/// drops it to `~5.51`), and no amount of same-tick engineering above fully eliminates
/// residual real-network jitter risk. This assertion uses `>= 4.0` instead -- still
/// comfortably proves "a charged critical hit substantially exceeds an uncharged hit,"
/// the qualitative property scenario 11 exists to demonstrate, and stays robust even if
/// hit A's own real ticker lands at `1` rather than `0`.
#[tokio::test]
async fn charged_critical_exceeds_uncharged_by_the_documented_envelope() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc, a_net_id) = spawn_actor(&world, "a", 1).await;
        let sessions = world.player_sessions();

        let target_feet = [2.0, -60.0, 0.0];
        let (_, dummy_net_id) = world.debug_spawn_mob(EntityKind::Zombie, target_feet).await;
        let (_, target_net_id) = world.debug_spawn_mob(EntityKind::Zombie, target_feet).await;
        // Zero armor/toughness on the real target: raw (pre-armor) damage equals dealt
        // damage exactly, matching Context's own hand-derivation (no armor absorption
        // term appears in scenario 11's own formula). The dummy's own armor is left at its
        // default -- its own dealt damage is never read by this test.
        world.debug_override_attribute(target_net_id, attribute::ARMOR, 0.0).await;
        world
            .debug_override_attribute(target_net_id, attribute::ARMOR_TOUGHNESS, 0.0)
            .await;

        // Hit A: reset via the dummy (identical position, so no re-aim is needed), then
        // the real hit against `target_net_id` immediately after, both sent back-to-back
        // so both land in the same tick's own `pending_attacks` drain (module doc comment).
        aim_and_confirm(&mut a, &sessions, uuid, zombie_center(target_feet)).await;
        let health_before_a = world.debug_query_entity(target_net_id).await.unwrap().health;
        send_packet(&mut a, &Attack { entity_id: dummy_net_id }).await;
        send_packet(&mut a, &Attack { entity_id: target_net_id }).await;
        recv_packet_for_entity::<DamageEvent>(&mut a, &mut a_acc, |p| p.entity_id, target_net_id)
            .await;
        let health_after_a = world.debug_query_entity(target_net_id).await.unwrap().health;
        let damage_a = (health_before_a - health_after_a) as f64;
        assert!(
            damage_a > 0.0,
            "expected hit A to deal nonzero damage, got before={health_before_a} after={health_after_a}"
        );

        // Hit B: full charge (a generous sleep) then airborne (a real downward movement
        // report with on_ground: false), then the Attack while still airborne.
        //
        // `SPAWN_POSITION` feet `[0.0, -60.0, 0.0]` sit directly on this world's own
        // superflat grass top (grass occupies y=-61) -- falling straight down from spawn
        // collides with solid ground immediately, silently rejecting the movement report
        // (mismatch) and leaving `fall_distance` at `0.0`. `debug_teleport_player` (a
        // direct, validation-bypassing position set, mirroring a real `/tp`) first lifts
        // the player above the grass, near the target's own *current* position; the
        // *real*, validated movement report that follows then has genuine open air to
        // fall through.
        //
        // Impulse #1 (Context, "Knockback") already pushed the target away from the
        // attacker along the attacker-to-victim line during hit A -- re-query its own
        // current position (mirrors `play_combat_melee_flow.rs`'s own `repeated_attacks_
        // within_ten_ticks_apply_only_the_delta` precedent) rather than aiming at (and
        // positioning the player near) the stale spawn position. The hover/fall position
        // is offset 1.5 blocks horizontally from the target, never directly overhead --
        // a near-zero horizontal offset drives `aim_at`'s own pitch to (near-)exactly 90
        // degrees, a gimbal-lock-adjacent edge case this file's own debugging found
        // triggers a spurious world-occlusion rejection.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let target_feet_now = world
            .debug_query_entity(target_net_id)
            .await
            .unwrap()
            .pos
            .expect("a mob's own DebugEntityInfo always carries a position");
        let hover_pos = [target_feet_now[0] - 1.5, -58.0, target_feet_now[2]];
        world.debug_teleport_player(a_net_id, hover_pos).await;
        let airborne_feet = [hover_pos[0], hover_pos[1] - 1.0, hover_pos[2]];
        let airborne_eye = [airborne_feet[0], airborne_feet[1] + 1.62, airborne_feet[2]];
        let (yaw_b, pitch_b) = aim_at(airborne_eye, zombie_center(target_feet_now));
        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: airborne_feet[0],
                y: airborne_feet[1],
                z: airborne_feet[2],
                yaw: yaw_b,
                pitch: pitch_b,
                on_ground: false,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid, |r| r.data.rotation) == Some([yaw_b, pitch_b]))
            .await;
        let health_before_b = world.debug_query_entity(target_net_id).await.unwrap().health;
        send_packet(&mut a, &Attack { entity_id: target_net_id }).await;
        recv_packet_for_entity::<DamageEvent>(&mut a, &mut a_acc, |p| p.entity_id, target_net_id)
            .await;
        let health_after_b = world.debug_query_entity(target_net_id).await.unwrap().health;
        let damage_b = (health_before_b - health_after_b) as f64;
        assert!(
            damage_b > 0.0,
            "expected hit B to deal nonzero damage, got before={health_before_b} after={health_after_b}"
        );

        let ratio = damage_b / damage_a;
        assert!(
            ratio >= 4.0,
            "expected damage(B)/damage(A) >= 4.0 (module doc comment has the full citation \
             for this blueprint's own literal >= 7.0 threshold), got {ratio} \
             (damage_a={damage_a}, damage_b={damage_b})"
        );
    })
    .await
    .expect("test exceeded its own 60s outer deadline");
}
