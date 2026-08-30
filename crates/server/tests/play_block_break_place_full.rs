//! M3-B03 acceptance test: the full survival dig-timing state machine, creative instant
//! break, and held-item-driven placement orientation, all exercised end-to-end over real
//! loopback connections. Mirrors `M2-B07`'s own `play_block_place_break.rs` shape, extended.
//! See `blueprints/M3/M3-B03-breaking-placing.md`, Acceptance tests,
//! "`crates/server/tests/play_block_break_place_full.rs`".
//!
//! M3 field-report test-authoring update (MECH-D62 re-supersession, Symptom 1): this file's
//! own former `raycast_reach_rejects_an_occluded_target_even_within_euclidean_range` test
//! pinned the now-retired voxel raycast's own occlusion behavior, which vanilla's real reach
//! predicate never had (Context, AUTHORITATIVE RESEARCH VERDICT: no line-of-sight component
//! whatsoever) -- rewritten below as `distance_based_reach_ignores_occlusion_and_accepts_a_
//! block_behind_another`, asserting the corrected acceptance instead. Fails today: the old
//! raycast's own closest-hit-only DDA still resolves this exact scenario to the nearer,
//! occluding block, never reaching the claimed (farther, occluded) one, so the pre-fix server
//! still rejects it.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_mechanics::Direction;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, LEVEL_EVENT_BLOCK_BREAK, LevelEvent, PlayerAction, SetBlockDestroyStage,
    SetPlayerPositionAndRotation, SetPlayerRotation, UseItemOn, pack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, Orientation, PlaceableBlockKind, PlayerProfile, ToolKind,
    ToolMaterial, enter_play, tier1_oriented_state_table,
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

/// `HardcodedWorld::alloc_network_entity_id` is a single, monotonic per-`HardcodedWorld`
/// counter starting at `1`; `enter_play` allocates one, synchronously, before its very
/// first `.await` (before it ever queues a join or sends a packet). Every test in this file
/// awaits one `spawn_actor` call to completion before starting the next, so the Nth
/// `spawn_actor` call against a fresh `HardcodedWorld` always gets network-entity id `N` --
/// exactly what this file's own `debug_set_held_item`/`debug_set_survival` call sites (which
/// need a concrete id, not a connection handle) rely on.
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
) -> Bytes {
    loop {
        let (id, body) = recv_clientbound(socket, accumulator).await;
        if id == expected_id {
            return body;
        }
    }
}

async fn assert_no_packet_of_type(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    forbidden_id: i32,
    window: Duration,
) {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, _)) => assert_ne!(
                id, forbidden_id,
                "expected no packet of id {forbidden_id}, but received one"
            ),
            Err(_) => return,
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

#[tokio::test]
async fn creative_break_is_still_instant_and_excludes_the_breaker_from_the_level_event() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let sessions = world.player_sessions();

        // Looking straight down hits the grass block directly below spawn -- every M3
        // player is Creative by default (M1-B05's own hardcoded default, preserved).
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
                location: pack_position(BlockPos::new(0, -60, 0)),
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
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(0, -60, 0)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);

        // M3 field-report fix (Defect 2, "the breaking player hears the block-break effect
        // twice"): A, the breaker, never receives the Level Event at all -- their own client
        // already plays the break effect locally as prediction (Context, AUTHORITATIVE
        // RESEARCH VERDICT). Checked before draining B's own traffic below, so a wrongly-sent
        // packet to A cannot be masked by B's own scan racing ahead of it.
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            LevelEvent::ID,
            Duration::from_millis(400),
        )
        .await;

        // B, the bystander, still receives both the Block Update and the Level Event.
        let body = recv_packet_of_type(&mut b, &mut b_acc, BlockUpdate::ID).await;
        assert_eq!(decode_one::<BlockUpdate>(body).unwrap(), update);
        let body = recv_packet_of_type(&mut b, &mut b_acc, LevelEvent::ID).await;
        let level_event = decode_one::<LevelEvent>(body).unwrap();
        assert_eq!(level_event.event_id, LEVEL_EVENT_BLOCK_BREAK);
        assert_eq!(
            level_event.location,
            pack_position(BlockPos::new(0, -60, 0))
        );
        assert_eq!(level_event.data, blocks::GRASS_BLOCK.0 as i32);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn survival_multi_tick_break_shows_rising_crack_stages_then_finalizes_on_stop() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let sessions = world.player_sessions();

        // A moves to (1, -59, 1), looking straight down, then places a Stone block in the
        // NEIGHBOURING column at x=2 (still Creative -- the default held item is already
        // `Block(Stone)`) so this test has a known-Stone target independent of the fixed
        // superflat layer table. Not the column A is standing in: `mining::apply_placement`'s
        // own obstruction gate (M3 field-report fix, Defect 1) now correctly rejects a
        // placement into a cell A's own body occupies, A's own body included -- one column
        // over keeps this test's own "known-Stone target" intent without tripping that gate.
        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: 1.0,
                y: -59.0,
                z: 1.0,
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid_a, |r| r.data.pos) == Some([1.0, -59.0, 1.0]))
            .await;

        send_packet(
            &mut a,
            &UseItemOn {
                hand: 0,
                location: pack_position(BlockPos::new(2, -60, 1)),
                direction: 1,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside_block: false,
                hits_world_border: false,
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
        let placed = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(placed.location, pack_position(BlockPos::new(2, -59, 1)));
        assert_eq!(placed.block_state_id, blocks::STONE.0 as i32);
        // B also observes the placement broadcast -- drained so it does not interfere with
        // this test's own later `BlockUpdate` scan.
        let body = recv_packet_of_type(&mut b, &mut b_acc, BlockUpdate::ID).await;
        assert_eq!(decode_one::<BlockUpdate>(body).unwrap(), placed);

        // `debug_set_survival`/`debug_set_held_item` are themselves `async fn`s (a
        // deliberate deviation from this blueprint's own literal synchronous signature,
        // `world.rs`'s own doc comment on them) -- each `.await` only resolves once that
        // exact mutation has actually landed, a real synchronization guarantee rather than
        // a fixed-timing guess.
        world.debug_set_survival(1, true).await;
        world
            .debug_set_held_item(1, HeldItemStub::Tool(ToolMaterial::Wood, ToolKind::Pickaxe))
            .await;

        // Golden-table row 2: Stone / Wood pickaxe -> 23 ticks.
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(2, -59, 1)),
                direction: 1,
                sequence: 2,
            },
        )
        .await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            2
        );

        // B observes a strictly non-decreasing crack-stage sequence, reaching 9; A itself
        // never receives one (Context: excludes the digging player).
        let mut last_stage: i8 = -1;
        loop {
            let (id, body) = recv_clientbound(&mut b, &mut b_acc).await;
            if id != SetBlockDestroyStage::ID {
                continue;
            }
            let packet = decode_one::<SetBlockDestroyStage>(body).unwrap();
            assert_eq!(packet.location, pack_position(BlockPos::new(2, -59, 1)));
            assert!(
                packet.destroy_stage >= last_stage,
                "stage must never decrease"
            );
            last_stage = packet.destroy_stage;
            if last_stage == 9 {
                break;
            }
        }
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            SetBlockDestroyStage::ID,
            Duration::from_millis(300),
        )
        .await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 2,
                location: pack_position(BlockPos::new(2, -59, 1)),
                direction: 1,
                sequence: 3,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            3
        );
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(2, -59, 1)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
        // M3 field-report fix (Defect 2): A, the breaker, never receives the Level Event.
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            LevelEvent::ID,
            Duration::from_millis(400),
        )
        .await;

        let body = recv_packet_of_type(&mut b, &mut b_acc, BlockUpdate::ID).await;
        assert_eq!(decode_one::<BlockUpdate>(body).unwrap(), update);
        let body = recv_packet_of_type(&mut b, &mut b_acc, LevelEvent::ID).await;
        assert_eq!(
            decode_one::<LevelEvent>(body).unwrap().data,
            blocks::STONE.0 as i32
        );
    })
    .await
    .unwrap();
}

/// M3 field-report test-authoring (Defect 2, `world.rs`'s own `broadcast_break` call site --
/// the delayed-destroy tick-driven finalize, distinct from `respond_break`'s own immediate-
/// finalize path the two tests above already cover): A stops digging well short of the 0.7
/// progress threshold (`stop_destroy`'s own `DelayedQueued` outcome), sends no further packet,
/// and the resulting delayed destroy auto-finalizes several ticks later entirely on its own
/// (`mining::tick_destroy_state`'s own per-tick drain) -- proving the exclusion holds on THIS
/// call site too, not only `respond_break`'s.
#[tokio::test]
async fn delayed_destroy_auto_finalize_excludes_the_breaker_from_the_level_event() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let sessions = world.player_sessions();

        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: 3.0,
                y: -59.0,
                z: 3.0,
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions.with_record_mut(uuid_a, |r| r.data.pos) == Some([3.0, -59.0, 3.0]))
            .await;

        // Survival + Wood pickaxe on Stone: golden-table row 2 -> 23 ticks total, so stopping
        // after the very first tick (progress ~= 1/23 ~= 0.043) sits well under the 0.7
        // finalize-now threshold and queues a delayed destroy instead.
        world.debug_set_survival(1, true).await;
        world
            .debug_set_held_item(1, HeldItemStub::Tool(ToolMaterial::Wood, ToolKind::Pickaxe))
            .await;

        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(3, -60, 3)),
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

        send_packet(
            &mut a,
            &PlayerAction {
                status: 2,
                location: pack_position(BlockPos::new(3, -60, 3)),
                direction: 1,
                sequence: 2,
            },
        )
        .await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            2
        );

        // No further packet from A -- the delayed destroy finalizes on its own, ~23 ticks
        // (~1.15s at 20 TPS) after the original `START_DESTROY_BLOCK`.
        let body = recv_packet_of_type(&mut b, &mut b_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(3, -60, 3)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);
        let body = recv_packet_of_type(&mut b, &mut b_acc, LevelEvent::ID).await;
        let level_event = decode_one::<LevelEvent>(body).unwrap();
        assert_eq!(level_event.event_id, LEVEL_EVENT_BLOCK_BREAK);
        assert_eq!(level_event.data, blocks::GRASS_BLOCK.0 as i32);

        // A receives the identical Block Update (unconditional resync) but never a Level
        // Event -- M3 field-report fix, Defect 2, this call site.
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        assert_eq!(decode_one::<BlockUpdate>(body).unwrap(), update);
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            LevelEvent::ID,
            Duration::from_millis(500),
        )
        .await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn distance_based_reach_ignores_occlusion_and_accepts_a_block_behind_another() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // M3 field-report test-authoring fix (Defect 2): A never receives its own break's own
        // Level Event any more (`broadcast_to_others` excludes the breaker) -- this file's
        // own former single-actor version of this test waited on `recv_packet_of_type(&mut
        // a, ..., LevelEvent::ID)`, which now blocks forever with no bystander to ever
        // deliver one, hanging until this test's own outer 60s timeout. B, added here purely
        // as that bystander, is this test's own only change beyond that.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions_rotation_ready(&world, uuid_a)).await;

        // (0, -61, 0) (dirt) sits directly behind (0, -60, 0) (grass) along A's own
        // straight-down look line -- vanilla's real reach predicate has no line-of-sight
        // component at all (Context, AUTHORITATIVE RESEARCH VERDICT): only the box-distance
        // from A's own standing eye to (0, -61, 0)'s own nearest point (~2.62 blocks, well
        // inside the 6.0 creative threshold) decides this, regardless of the solid grass
        // block in between.
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

        // Every M3 player is Creative by default -- this succeeds as an instant break.
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(0, -61, 0)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);

        // M3 field-report fix (Defect 2): A, the breaker, never receives the Level Event; B,
        // the bystander, still does.
        assert_no_packet_of_type(
            &mut a,
            &mut a_acc,
            LevelEvent::ID,
            Duration::from_millis(400),
        )
        .await;
        let body = recv_packet_of_type(&mut b, &mut b_acc, BlockUpdate::ID).await;
        assert_eq!(decode_one::<BlockUpdate>(body).unwrap(), update);
        let body = recv_packet_of_type(&mut b, &mut b_acc, LevelEvent::ID).await;
        let level_event = decode_one::<LevelEvent>(body).unwrap();
        assert_eq!(level_event.event_id, LEVEL_EVENT_BLOCK_BREAK);
        assert_eq!(level_event.data, blocks::DIRT.0 as i32);
    })
    .await
    .unwrap();
}

fn sessions_rotation_ready(world: &HardcodedWorld, uuid: uuid::Uuid) -> bool {
    world
        .player_sessions()
        .with_record_mut(uuid, |r| r.data.rotation)
        == Some([0.0, 90.0])
}

#[tokio::test]
async fn placement_selects_the_held_items_own_block_and_orientation() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Repeater))
            .await;

        // yaw = 0.0 -> looking South (`look_vector`'s own convention); a repeater faces
        // *away* from the player -- North. A never moves off spawn (0, -59, 0) in this test,
        // so the click below targets the NEIGHBOURING column at x=1, not spawn's own column
        // directly underfoot -- `mining::apply_placement`'s own obstruction gate (M3
        // field-report fix, Defect 1) would otherwise reject a repeater placed into the cell
        // A's own body occupies (orientation depends only on yaw/pitch, never position, so
        // this has no effect on the North-facing assertion below).
        send_packet(
            &mut a,
            &SetPlayerRotation {
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        wait_until(|| sessions_rotation_ready(&world, uuid_a)).await;

        send_packet(
            &mut a,
            &UseItemOn {
                hand: 0,
                location: pack_position(BlockPos::new(1, -60, 0)),
                direction: 1,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside_block: false,
                hits_world_border: false,
                sequence: 2,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            2
        );

        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(1, -59, 0)));
        let expected_raw = tier1_oriented_state_table().lookup(
            PlaceableBlockKind::Repeater,
            Orientation::Horizontal(Direction::North),
        );
        assert_eq!(update.block_state_id, expected_raw as i32);
    })
    .await
    .unwrap();
}

/// M3 field-report regression test (CI run 33319009203, `placement_selects_the_held_items_
/// own_block_and_orientation` above): reproduces, deterministically rather than by chance,
/// the exact race between `debug_set_held_item` and its own target actor's join that CI
/// caught only under parallel load. Root cause (verified locally with an instrumented
/// build, ~35% failure rate under contention, always the identical shape): `spawn_actor`'s
/// own completion (`ChunkBatchFinished` receipt) proves only that `connection.rs`'s own
/// `enter_play` is *about* to call `queue_join`, never that the join has actually reached
/// `HardcodedWorld`'s own tick-loop thread and been drained into a spawned `PlayerMarker` --
/// the same join/action mpsc-ordering race `carried_movement_updates`'s own doc comment
/// (`world.rs`) already documents for ordinary movement reports. The pre-fix held-item/
/// survival drain handled a not-yet-spawned target differently and wrongly: it still
/// unconditionally acked, so a caller's `.await` returned believing the mutation had
/// landed while `HeldItem` silently kept its join-time default forever after.
///
/// Races many independent `HardcodedWorld`s concurrently (each gets its own OS tick-loop
/// thread) rather than looping one at a time -- real OS-thread contention, not mere
/// iteration count, is what widens this race's own window enough to hit reliably; a
/// sequential loop on an otherwise-idle machine reproduced it far less often locally. Every
/// one of these actors calls `debug_set_held_item` with no `.await` in between `spawn_actor`
/// being kicked off (via `tokio::spawn`, not `.await`ed yet) and the call itself -- the
/// tightest window this file's own public API can create between "join queued" (or not
/// even that far yet) and "held-item mutation queued".
#[tokio::test]
async fn debug_set_held_item_survives_a_join_still_racing_it() {
    tokio::time::timeout(Duration::from_secs(120), async {
        let mut tasks = Vec::new();
        for _ in 0..16 {
            tasks.push(tokio::spawn(async move {
                let world = HardcodedWorld::new();
                let uuid_a = uuid::Uuid::from_u128(1);
                // `enter_play`'s own `alloc_network_entity_id` call is the connection
                // task's very first synchronous step (`connection.rs`, before any
                // `.await`) -- id 1 is deterministic for a fresh `HardcodedWorld`'s
                // first-ever spawn (`spawn_actor`'s own doc comment, this file).
                let world_for_spawn = world.clone();
                let spawn_task =
                    tokio::spawn(async move { spawn_actor(&world_for_spawn, "a", 1).await });
                // No `.await` precedes this call -- races `debug_set_held_item`'s own
                // channel send directly against `connection.rs`'s own `queue_join` call
                // for this exact actor, deliberately not waiting for the join to land
                // first the way a well-behaved caller normally would (this test's own
                // point: the API contract must hold regardless).
                world
                    .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Repeater))
                    .await;
                let (mut a, mut a_acc) = spawn_task.await.unwrap();

                send_packet(
                    &mut a,
                    &SetPlayerRotation {
                        yaw: 0.0,
                        pitch: 90.0,
                        on_ground: true,
                    },
                )
                .await;
                wait_until(|| sessions_rotation_ready(&world, uuid_a)).await;

                send_packet(
                    &mut a,
                    &UseItemOn {
                        hand: 0,
                        location: pack_position(BlockPos::new(1, -60, 0)),
                        direction: 1,
                        cursor_x: 0.5,
                        cursor_y: 1.0,
                        cursor_z: 0.5,
                        inside_block: false,
                        hits_world_border: false,
                        sequence: 2,
                    },
                )
                .await;

                let body =
                    recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
                assert_eq!(
                    decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
                    2
                );

                let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
                let update = decode_one::<BlockUpdate>(body).unwrap();
                assert_eq!(update.location, pack_position(BlockPos::new(1, -59, 0)));
                let expected_raw = tier1_oriented_state_table().lookup(
                    PlaceableBlockKind::Repeater,
                    Orientation::Horizontal(Direction::North),
                );
                assert_eq!(
                    update.block_state_id, expected_raw as i32,
                    "held-item mutation queued before this actor's own join must still \
                     apply once the join lands, never silently keep the join-time default"
                );
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
    })
    .await
    .unwrap();
}
