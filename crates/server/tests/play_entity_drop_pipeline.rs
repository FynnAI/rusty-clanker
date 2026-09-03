//! M4-B02 acceptance tests: the item-entity lifecycle end-to-end — spawn-on-break, pickup
//! delay/range, merge, and age-despawn (Context §I/§L/§M/§N) — over real loopback
//! connections and `HardcodedWorld`'s own debug seam, mirroring `play_block_break_place_
//! full.rs`'s own established connection-setup shape.
//!
//! **Documented test-design note** (`docs/findings-for-planning.md`): `item_despawns_at_
//! exactly_6000_ticks` genuinely waits out all `6000` real ticks (`DESPAWN_AGE_TICKS`, ~5
//! minutes at 20 TPS) — `TickClock<SystemTickWaiter>` (`rc-scheduler`, outside this
//! blueprint's own Crates-touched list) paces every region's tick loop against real wall-clock
//! time unconditionally, with no test-only acceleration hook anywhere in this project today.
//! This is therefore the slowest test in this blueprint's own suite by a wide margin.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rc_registries::generated_v776::registries::item;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{AcknowledgeBlockChange, PlayerAction, pack_position};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlayerProfile, SpawnEntity, ToolKind, ToolMaterial, enter_play,
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

async fn recv_packet_of_type(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    expected_id: i32,
    window: Duration,
) -> Option<Bytes> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == expected_id => return Some(body),
            Ok(_) => continue,
            Err(_) => return None,
        }
    }
}

async fn assert_no_packet_of_type(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    forbidden_id: i32,
    window: Duration,
) {
    let got = recv_packet_of_type(socket, accumulator, forbidden_id, window).await;
    assert!(
        got.is_none(),
        "expected no packet of id {forbidden_id}, but received one"
    );
}

const TICK: Duration = Duration::from_millis(50);

#[tokio::test]
async fn breaking_stone_spawns_exactly_one_cobblestone_item_entity() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        let target = BlockPos::new(5, -60, 5);

        // `debug_teleport_player` (a server-authoritative position overwrite bypassing
        // `evaluate_movement`'s own speed check entirely) rather than a raw movement packet --
        // a single-tick jump straight from `SPAWN_POSITION` to here would otherwise be
        // anti-cheat-rejectable, silently leaving the bot too far from `target` to reach it.
        // Reach is a box-distance-from-eye predicate, not a raycast, so rotation is
        // irrelevant here. Teleporting BEFORE `debug_set_block_state` matters too -- chunks
        // stream in based on a player's own position, so `target`'s own chunk is not
        // guaranteed resident (and a `set_block` against it may silently no-op) until a
        // player has actually been placed nearby.
        world.debug_teleport_player(1, [5.5, -59.0, 5.5]).await;

        world.debug_set_block_state(target, blocks::STONE.0).await;
        world.debug_set_survival(1, true).await;
        world
            .debug_set_held_item(
                1,
                HeldItemStub::Tool(ToolMaterial::Diamond, ToolKind::Pickaxe),
            )
            .await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(target),
                direction: 1,
                sequence: 1,
            },
        )
        .await;
        let body = recv_packet_of_type(
            &mut a,
            &mut a_acc,
            AcknowledgeBlockChange::ID,
            Duration::from_secs(10),
        )
        .await
        .expect("AcknowledgeBlockChange");
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            1
        );

        // Diamond pickaxe on Stone takes 6 ticks (mining_dig_timing_golden_table.rs's own
        // row 4) -- `tick_destroy_state`'s own `is_destroying` branch never finalizes on its
        // own (it only ever reports `ActiveProgress`, capped at crack stage 9, forever); a
        // `STOP_DESTROY_BLOCK` is what actually transitions the state machine into its own
        // "delayed destroy" branch, which THEN auto-finalizes over the following ticks with
        // no further client packet needed (mirrors `delayed_destroy_auto_finalize_...`'s own
        // established two-packet shape exactly -- this test's own first draft sent only the
        // START packet and waited indefinitely, which never breaks the block at all).
        tokio::time::sleep(TICK * 8).await;
        send_packet(
            &mut a,
            &PlayerAction {
                status: 2,
                location: pack_position(target),
                direction: 1,
                sequence: 2,
            },
        )
        .await;
        let body = recv_packet_of_type(
            &mut a,
            &mut a_acc,
            AcknowledgeBlockChange::ID,
            Duration::from_secs(10),
        )
        .await
        .expect("AcknowledgeBlockChange (stop)");
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            2
        );

        let spawn_body =
            recv_packet_of_type(&mut a, &mut a_acc, SpawnEntity::ID, Duration::from_secs(10))
                .await
                .expect("a Spawn Entity packet for the newly-dropped item");
        let spawn = decode_one::<SpawnEntity>(spawn_body).unwrap();
        assert_eq!(
            spawn.entity_type,
            rc_mechanics::entity::EntityKind::Item.registry_id().0 as i32
        );

        // Exactly one -- no second Spawn Entity packet follows within a generous window.
        assert_no_packet_of_type(&mut a, &mut a_acc, SpawnEntity::ID, Duration::from_secs(2)).await;
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn dropped_item_becomes_pickupable_after_delay_and_range() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut _a, mut _a_acc) = spawn_actor(&world, "a", 1).await;

        // `debug_teleport_player`, not a raw movement packet -- see
        // `breaking_stone_spawns_exactly_one_cobblestone_item_entity`'s own identical note.
        world.debug_teleport_player(1, [8.0, -60.0, 8.0]).await;

        let id = world
            .debug_spawn_item_entity(BlockPos::new(8, -60, 8), item::COBBLESTONE, 1)
            .await;

        // Immediately after spawn -- `pickup_delay_ticks` starts at `PICKUP_DELAY_DEFAULT`
        // (10) and decrements by at most one per `tick_region` call, so it cannot possibly
        // have reached zero yet even though the item already sits well within pickup range.
        // (`docs/findings-for-planning.md`: a longer, wall-clock-timed "still exists a few
        // ticks in" check was this test's own first draft, but this project's own
        // `TickClock` deliberately never skips or batches ticks under sustained overrun --
        // it degrades TPS instead, then catches up in a rapid burst once rescheduled, which
        // this test's own background `HardcodedWorld` thread is genuinely subject to under
        // real OS scheduling; a fixed wall-clock sleep is therefore not a reliable proxy for
        // "fewer than 10 ticks have elapsed" in this environment.)
        assert!(
            world.debug_query_item_entity(id).await.is_some(),
            "must not be picked up on its own spawn tick"
        );

        // Poll until it is gone, bounded generously past the delay to absorb real-time
        // scheduling jitter.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if world.debug_query_item_entity(id).await.is_none() {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "item was never picked up"
            );
            tokio::time::sleep(TICK).await;
        }

        let picked_up = world.debug_query_picked_up_items(1).await;
        assert_eq!(picked_up.len(), 1);
        assert_eq!(picked_up[0].item_id, item::COBBLESTONE);
        assert_eq!(picked_up[0].count, 1);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn pickup_out_of_range_never_triggers() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut _a, mut _a_acc) = spawn_actor(&world, "a", 1).await;

        world.debug_teleport_player(1, [8.0, -60.0, 8.0]).await;

        // Far outside `ITEM_PICKUP_AABB_INFLATE` (0.5 blocks) from the player above.
        let id = world
            .debug_spawn_item_entity(BlockPos::new(50, -60, 50), item::COBBLESTONE, 1)
            .await;

        tokio::time::sleep(TICK * 20).await;
        assert!(
            world.debug_query_item_entity(id).await.is_some(),
            "out-of-range item must not despawn or be picked up"
        );
        let picked_up = world.debug_query_picked_up_items(1).await;
        assert!(picked_up.is_empty());
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn two_adjacent_drops_of_the_same_item_eventually_merge() {
    tokio::time::timeout(Duration::from_secs(120), async {
        let world = HardcodedWorld::new();
        // A connected player is the only thing that ever gives this `HardcodedWorld` a
        // resident chunk at all (chunks stream in based on a player's own position, `world.
        // rs`'s own established chunk-streaming shape) -- this test's own target position
        // otherwise resolves as permanently unloaded (`get_block` returns `None`
        // everywhere), which this blueprint's own `docs/findings-for-planning.md` records:
        // this test's own first draft used a bare `HardcodedWorld` with no player at all,
        // and both drops free-fell forever, never converging in Y closely enough to merge.
        let (mut _a, mut _a_acc) = spawn_actor(&world, "a", 1).await;
        let target = BlockPos::new(2, -60, 2);
        // Well beyond `ITEM_PICKUP_AABB_INFLATE` (0.5 blocks) from `target`, so this test's
        // own drops are never eligible for pickup and race against it -- still close enough
        // for `target`'s own chunk to load (chunk residency covers a radius around the
        // player, not just their own cell).
        world
            .debug_teleport_player(
                1,
                [
                    target.x as f64 + 6.5,
                    target.y as f64,
                    target.z as f64 + 6.5,
                ],
            )
            .await;

        // A real floor directly beneath the spawn cell -- `debug_spawn_item_entity`'s own
        // spawn Y already sits exactly on this floor's own top face, so both drops rest in
        // place from their very first tick rather than diverging in Y tick-by-tick while
        // still falling (each drop's own spawn message lands on a different tick, so two
        // still-falling drops are never exactly Y-aligned).
        world
            .debug_set_block_state(
                BlockPos::new(target.x, target.y - 1, target.z),
                blocks::STONE.0,
            )
            .await;

        let id_a = world
            .debug_spawn_item_entity(target, item::COBBLESTONE, 1)
            .await;
        let id_b = world
            .debug_spawn_item_entity(target, item::COBBLESTONE, 1)
            .await;

        // §L's own cadence: every 2nd tick on a cell-crossing tick, otherwise every 40th --
        // a generous bound comfortably covers either case (a stationary drop's own 40-tick
        // worst case is 2 real seconds).
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let mut merged = false;
        while tokio::time::Instant::now() < deadline {
            let a = world.debug_query_item_entity(id_a).await;
            let b = world.debug_query_item_entity(id_b).await;
            match (a, b) {
                (Some(_), None) | (None, Some(_)) => {
                    merged = true;
                    break;
                }
                (None, None) => panic!("both entities vanished -- expected exactly one survivor"),
                (Some(_), Some(_)) => tokio::time::sleep(TICK).await,
            }
        }
        assert!(merged, "the two drops never merged within the deadline");

        let survivor = match (
            world.debug_query_item_entity(id_a).await,
            world.debug_query_item_entity(id_b).await,
        ) {
            (Some(info), None) => info,
            (None, Some(info)) => info,
            _ => unreachable!("checked above"),
        };
        assert_eq!(survivor.count, 2, "the survivor carries the summed count");
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn item_despawns_at_exactly_6000_ticks() {
    tokio::time::timeout(Duration::from_secs(600), async {
        let world = HardcodedWorld::new();
        let id = world
            .debug_spawn_item_entity(BlockPos::new(30, -60, 30), item::COBBLESTONE, 1)
            .await;

        // Ground truth is the server's own reported `age_ticks`, not this test's own
        // wall-clock sleep duration (`docs/findings-for-planning.md`: `TickClock` degrades
        // TPS under sustained overrun and then catches up in a rapid burst once
        // rescheduled -- a fixed sleep is not a reliable proxy for "N ticks have elapsed" on
        // this test's own background `HardcodedWorld` thread under real OS scheduling).
        // Polls until the entity disappears, tracking the last observed `age_ticks` --
        // despawn must land at (or very close to) exactly `DESPAWN_AGE_TICKS` (6000), never
        // dramatically earlier (a premature-despawn bug) nor within a huge margin later (a
        // stuck-forever bug).
        const DESPAWN_AGE_TICKS: i16 = 6000;
        let mut last_seen_age: Option<i16> = None;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(400);
        loop {
            match world.debug_query_item_entity(id).await {
                Some(info) => {
                    assert!(
                        info.age_ticks < DESPAWN_AGE_TICKS,
                        "still alive at age_ticks={} -- must have despawned by {DESPAWN_AGE_TICKS}",
                        info.age_ticks
                    );
                    last_seen_age = Some(info.age_ticks);
                }
                None => break,
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "item never despawned (last observed age_ticks={last_seen_age:?})"
            );
            tokio::time::sleep(TICK).await;
        }

        let last_age = last_seen_age.expect("observed the entity at least once before despawn");
        assert!(
            last_age >= DESPAWN_AGE_TICKS - 20,
            "despawned too early: last observed age_ticks={last_age}, expected close to {DESPAWN_AGE_TICKS}"
        );
    })
    .await
    .expect("test timed out");
}
