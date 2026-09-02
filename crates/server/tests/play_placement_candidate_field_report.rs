//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=yes self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(single canonical value/facing asserted, not a four-way sweep; one-neighbor chest merge only) nondefault-state=yes
//! M3 field-report test-authoring (the torch-candidate loop, the placement-time survival
//! refusal, and the chest-merge fixes -- end-to-end over a real loopback connection, mirroring
//! `play_block_state_orientation_real_client.rs`'s/`play_redstone_field_report.rs`'s own
//! established shape): the exact scenario the closed placement-diff case names
//! (`redstone_torch/dir_north/face_bottom_of_ceiling/pitch_level` -- oracle places a FLOOR
//! torch, ours placed nothing) plus the remaining vanilla placement rules the previous wave
//! left as residuals: wire/repeater placement-time survival refusal, and the chest
//! double-merge algorithm (`ChestBlock.getStateForPlacement`), both the non-sneak
//! clockwise/counter-clockwise neighbor case and the sneak-adopt-facing case.
//!
//! `wall_and_floor_redstone_torch_orientation_over_real_connection`
//! (`play_block_state_orientation_real_client.rs`, unmodified by this fix) already proves the
//! new torch-candidate loop reproduces every one of its own pre-existing wall/floor
//! assertions unchanged -- this file adds only the genuinely NEW candidate-loop behavior (a
//! ceiling click that used to be an unconditional `Face::Down` rejection) plus the wire/
//! repeater refusal and chest-merge coverage the previous wave's own residual note flagged.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PLAYER_INPUT_SHIFT, PlayerInput, SetPlayerRotation, UseItemOn,
    pack_position,
};
use rusty_clanker_server::play::{
    ChestType, HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, chest_state_id,
    enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (mirrors `play_block_state_orientation_real_client.rs`'s own identical
// helpers -- this crate's own established per-file-duplication convention, no shared `tests/`
// support module exists in this crate today) ---

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

async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Sets `yaw`/`pitch` and waits for it to land server-side. Never touches position (every
/// actor here stays at spawn, mirroring `play_block_state_orientation_real_client.rs`'s own
/// top-of-file doc comment).
async fn rotate(
    actor: &mut TcpStream,
    world: &HardcodedWorld,
    uuid: uuid::Uuid,
    yaw: f32,
    pitch: f32,
) {
    send_packet(
        actor,
        &SetPlayerRotation {
            yaw,
            pitch,
            on_ground: true,
        },
    )
    .await;
    wait_until(|| {
        world
            .player_sessions()
            .with_record_mut(uuid, |r| r.data.rotation)
            == Some([yaw, pitch])
    })
    .await;
}

/// Sequence-numbered `Use Item On` + response scan, shared by every test below. `seq` is a
/// `&mut i32` the caller owns and increments across every call in one test. Returns the
/// broadcast (or, on rejection, the corrective) `Block Update`'s own `block_state_id` --
/// `respond_place`'s own doc comment (`world.rs`) is why a rejection also always yields
/// exactly one `Block Update` here whenever `current_state` is populated, which is every
/// rejection reason this file ever exercises.
async fn place_and_read_id(
    actor: &mut TcpStream,
    acc: &mut BytesMut,
    seq: &mut i32,
    location: BlockPos,
    direction: i32,
) -> i32 {
    *seq += 1;
    send_packet(
        actor,
        &UseItemOn {
            hand: 0,
            location: pack_position(location),
            direction,
            cursor_x: 0.5,
            cursor_y: 0.5,
            cursor_z: 0.5,
            inside_block: false,
            hits_world_border: false,
            sequence: *seq,
        },
    )
    .await;
    let body = recv_packet_of_type(actor, acc, AcknowledgeBlockChange::ID).await;
    assert_eq!(
        decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
        *seq
    );
    let body = recv_packet_of_type(actor, acc, BlockUpdate::ID).await;
    decode_one::<BlockUpdate>(body).unwrap().block_state_id
}

// Yaw values producing each cardinal `FACING`
// (`nearest_horizontal_direction4(yaw).opposite()`), restated here since integration tests
// cannot share code across files in this crate today
// (`play_block_state_orientation_real_client.rs`'s own identical constants).
const YAW_FACING_NORTH: f32 = 0.0;
const YAW_FACING_EAST: f32 = 90.0;

#[tokio::test]
async fn torch_ceiling_click_falls_back_to_floor_torch_over_real_connection_facing_case() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // A floating "ceiling" block: `location`'s own occupancy is never itself validated
        // (only `target`, the offset cell, is -- `mining::apply_placement`'s own `TargetNotAir`
        // check), so a single `Face::Down` (direction 0) click at an arbitrary currently-air
        // `location` places Stone directly at `location + Down offset`, no reference block
        // needed first.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let ceiling_location = BlockPos::new(2, -57, 0);
        let ceiling_pos = BlockPos::new(2, -58, 0);
        let stone_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, ceiling_location, 0).await;
        assert_eq!(stone_id, blocks::STONE.0 as i32, "reference ceiling block");

        // The regression case itself: click the underside (`Face::Down`) of that ceiling
        // block. The clicked face's own opposite (`Up`) is front-inserted into the
        // torch-candidate order but is always invalid (`resolve_orientation`'s own
        // `RedstoneTorch` arm skips `Up` outright); no horizontal neighbor of the gap cell is
        // solid (nothing placed there); the natural grass floor two cells below the ceiling
        // IS solid, so the `Down` candidate succeeds -- a FLOOR torch, never a rejection,
        // despite the clicked face being `Down`. Before this fix: the old clicked-face-only
        // rule rejected every `Face::Down` click unconditionally (`InvalidTorchFace`), placing
        // nothing.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(2, -59, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, ceiling_pos, 0).await;
        assert_eq!(
            id, 6885,
            "ceiling click must fall back to a floor torch (lit=true), not a rejection"
        );
        let info = world.debug_query_block(torch_pos).await.unwrap();
        assert_eq!(info.raw_state, 6885);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn redstone_wire_on_air_below_is_refused_over_real_connection_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // Same "arbitrary currently-air `location`" technique as the ceiling test above:
        // places the target cell floating in open air, air below it too.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let location = BlockPos::new(2, -57, 0);
        let target = BlockPos::new(2, -58, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, location, 0).await;
        assert_eq!(
            id,
            blocks::AIR.0 as i32,
            "wire placement over air must be refused -- nothing placed"
        );
        let info = world.debug_query_block(target).await.unwrap();
        assert_eq!(info.raw_state, blocks::AIR.0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn repeater_on_non_full_block_is_refused_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // A real non-full block to place onto: a floor torch (Face::Up on the natural grass
        // floor).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let floor = BlockPos::new(2, -60, 0);
        let torch_pos = BlockPos::new(2, -59, 0);
        let torch_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor, 1).await;
        assert_eq!(torch_id, 6885);

        // Repeater placed directly on top of the torch (Face::Up click on the torch itself) --
        // the torch's own non-full shape must refuse this placement (M3 field-report fix,
        // placement-time survival refusal: repeater/comparator never had this check before).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Repeater))
            .await;
        let target = BlockPos::new(2, -58, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, torch_pos, 1).await;
        assert_eq!(
            id,
            blocks::AIR.0 as i32,
            "repeater on a non-full block below must be refused -- nothing placed"
        );
        let info = world.debug_query_block(target).await.unwrap();
        assert_eq!(info.raw_state, blocks::AIR.0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn chest_beside_chest_same_facing_merges_into_left_right_pair_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Chest))
            .await;
        let mut seq = 0;

        rotate(&mut a, &world, uuid_a, YAW_FACING_NORTH, 90.0).await;

        // Chest 1, facing North.
        let chest1_pos = BlockPos::new(2, -59, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -60, 0), 1).await;
        assert_eq!(
            id,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Single) as i32,
            "chest 1: facing=north, single"
        );

        // Chest 2, west of chest 1, same yaw (same base FACING North). Chest 2's own
        // clockwise-from-North neighbor direction is East -- exactly where chest 1 sits --
        // so this merges: chest 2 becomes LEFT, chest 1 flips to RIGHT.
        let chest2_pos = BlockPos::new(1, -59, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -60, 0), 1).await;
        assert_eq!(
            id,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Left) as i32,
            "chest 2: facing=north, left (merged)"
        );

        let info = world.debug_query_block(chest1_pos).await.unwrap();
        assert_eq!(
            info.raw_state,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Right),
            "chest 1 must flip to RIGHT once chest 2 merges beside it"
        );
        let info = world.debug_query_block(chest2_pos).await.unwrap();
        assert_eq!(
            info.raw_state,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Left)
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn sneak_place_beside_perpendicular_chest_adopts_its_facing_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Chest))
            .await;
        let mut seq = 0;

        // Chest 1, facing North.
        rotate(&mut a, &world, uuid_a, YAW_FACING_NORTH, 90.0).await;
        let chest1_pos = BlockPos::new(2, -59, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -60, 0), 1).await;
        assert_eq!(
            id,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Single) as i32,
            "chest 1: facing=north, single"
        );

        // Rotate to a yaw whose own non-merge default would resolve FACING = East (yaw 90 ->
        // look West -> opposite East) -- deliberately different from chest 1's own North, so
        // the assertion below proves the sneak-merge really ADOPTS the neighbor's facing
        // rather than coincidentally already agreeing with the player's own yaw-based default.
        rotate(&mut a, &world, uuid_a, YAW_FACING_EAST, 0.0).await;

        // Sneak (shift bit set) -- no direct hook to poll `PlayerInputState` from a test, so a
        // short fixed grace covers the tick this needs to land (`play_sneak_reach.rs`'s own
        // established precedent for this exact "wait for a decoded packet to actually apply"
        // gap).
        send_packet(
            &mut a,
            &PlayerInput {
                flags: PLAYER_INPUT_SHIFT,
            },
        )
        .await;
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Click chest 1's own East face directly (direction 5) -- a horizontal face
        // perpendicular to chest 1's own North/South facing axis, so the sneak-merge's own
        // axis-eligibility check passes. The new chest lands at chest1_pos + East.
        let chest2_pos = BlockPos::new(3, -59, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, chest1_pos, 5).await;
        assert_eq!(
            id,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Left) as i32,
            "chest 2 must ADOPT chest 1's own North facing (not the player's own yaw-based \
             East default) and merge as LEFT"
        );

        let info = world.debug_query_block(chest1_pos).await.unwrap();
        assert_eq!(
            info.raw_state,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Right),
            "chest 1 must flip to RIGHT once chest 2 merges beside it"
        );
        let info = world.debug_query_block(chest2_pos).await.unwrap();
        assert_eq!(
            info.raw_state,
            chest_state_id(rc_mechanics::Direction::North, ChestType::Left)
        );
    })
    .await
    .unwrap();
}
