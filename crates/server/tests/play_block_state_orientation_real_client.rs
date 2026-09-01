//! M3 field-report test-authoring (Root Cause 1, "placeholder id table" -- end-to-end over a
//! real loopback connection, mirroring `play_block_break_place_full.rs`'s own established
//! shape): drives `PlaceableBlockKind`'s own oriented-placeable set through real `Use Item On`
//! packets and asserts the exact `Block Update` state id the server broadcasts, per kind, per
//! orientation. `mining_block_state_ids.rs` (this same directory) pins the underlying id
//! arithmetic in isolation, pure and fast; this file additionally proves the real
//! packet-decode -> `resolve_orientation` -> `tier1_oriented_state_table` -> broadcast pipeline
//! produces the identical ids over an actual client connection, not merely in a unit call.
//! Every literal id below is the same blocks.json-decoded (protocol 776) value
//! `mining_block_state_ids.rs`'s own per-block comments already cite.
//!
//! Coordinates: every actor here stays at `HardcodedWorld`'s own default spawn (never sends
//! `SetPlayerPositionAndRotation`, mirroring `play_block_break_place_full.rs`'s own
//! `creative_break_is_still_instant...` test, which targets `(0, -60, 0)` -- the column
//! directly below spawn -- unmoved). Every target column below is a small offset from that
//! same spawn column, comfortably inside creative reach (`BLOCK_INTERACTION_RANGE_CREATIVE`
//! `5.0` + the `1.0` verification buffer, `mining.rs`), and no two placements in the same test
//! function ever target the same cell (`TargetNotAir` would otherwise reject the second one).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, SetPlayerRotation, UseItemOn, pack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (mirrors `play_block_break_place_full.rs`'s own identical helpers --
// this crate's own established per-file-duplication convention, no shared `tests/` support
// module exists in this crate today) ---

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

/// Sets `yaw`/`pitch` and waits for it to land server-side -- every yaw/pitch-driven
/// placement below needs this before its own `place_and_read_id` call. Never touches position
/// (this file's own top-of-file doc comment: every actor stays at spawn).
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
/// `&mut i32` the caller owns and increments across every call in one test (every real client
/// packet needs a distinct, increasing sequence number, `packets.rs`'s own protocol contract).
/// Returns the broadcast `Block Update`'s own `block_state_id`.
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

// Yaw values producing each cardinal `FACING` (`nearest_horizontal_direction4(yaw).opposite()`
// -- `mining_block_state_ids.rs`'s own identical constants/derivation, restated here since
// integration tests cannot share code across files in this crate today).
const YAW_FACING_NORTH: f32 = 0.0;
const YAW_FACING_SOUTH: f32 = 180.0;
const YAW_FACING_EAST: f32 = 90.0;
const YAW_FACING_WEST: f32 = 270.0;

/// Eight distinct floor columns, all within a couple of blocks of spawn (well inside creative
/// reach) and never overlapping the player's own standing column -- shared by every test below
/// (each test gets a fresh `HardcodedWorld`, so no cross-test collision either).
const FLOOR_COLS: [(i32, i32); 8] = [
    (1, 0),
    (2, 0),
    (3, 0),
    (1, 1),
    (2, 1),
    (3, 1),
    (1, -1),
    (2, -1),
];

fn floor(i: usize) -> BlockPos {
    let (dx, dz) = FLOOR_COLS[i];
    BlockPos::new(dx, -60, dz)
}

/// A reference-block floor column set well clear of every `FLOOR_COLS` entry above AND of
/// every one of that column's own 4 horizontal neighbors (the two wall-mounted tests below --
/// hopper-on-stone, torch-on-stone -- each place a reference Stone here, then click all 4 of
/// ITS OWN horizontal faces; every one of those 4 target cells must land somewhere `FLOOR_COLS`
/// itself never separately occupies, or the second placement would hit `TargetNotAir`). Still
/// within creative reach of spawn (`(0, -59, 0)`-ish, per this file's own top-of-file doc
/// comment): the farthest of the 4 neighbor cells this produces is at Euclidean distance
/// `sqrt(1^2 + 3^2) ~= 3.16` from spawn's own column, comfortably inside the `5.0 + 1.0`
/// creative reach budget.
const STONE_FLOOR: BlockPos = BlockPos::new(0, -60, 3);

#[tokio::test]
async fn furnace_orientation_and_lit_state_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Furnace))
            .await;
        let mut seq = 0;

        for (i, (yaw, expected, label)) in [
            (YAW_FACING_NORTH, 5328, "facing=north, lit=false"),
            (YAW_FACING_SOUTH, 5330, "facing=south, lit=false"),
            (YAW_FACING_WEST, 5332, "facing=west, lit=false"),
            (YAW_FACING_EAST, 5334, "facing=east, lit=false"),
        ]
        .into_iter()
        .enumerate()
        {
            rotate(&mut a, &world, uuid_a, yaw, 90.0).await;
            let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(i), 1).await;
            assert_eq!(id, expected, "furnace at yaw {yaw} ({label})");
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn blast_furnace_and_smoker_orientation_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::BlastFurnace))
            .await;
        for (i, (yaw, expected)) in [
            (YAW_FACING_NORTH, 20763),
            (YAW_FACING_SOUTH, 20765),
            (YAW_FACING_WEST, 20767),
            (YAW_FACING_EAST, 20769),
        ]
        .into_iter()
        .enumerate()
        {
            rotate(&mut a, &world, uuid_a, yaw, 90.0).await;
            let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(i), 1).await;
            assert_eq!(id, expected, "blast_furnace at yaw {yaw}");
        }

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Smoker))
            .await;
        for (i, (yaw, expected)) in [
            (YAW_FACING_NORTH, 20755),
            (YAW_FACING_SOUTH, 20757),
            (YAW_FACING_WEST, 20759),
            (YAW_FACING_EAST, 20761),
        ]
        .into_iter()
        .enumerate()
        {
            rotate(&mut a, &world, uuid_a, yaw, 90.0).await;
            let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(i + 4), 1).await;
            assert_eq!(id, expected, "smoker at yaw {yaw}");
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn chest_orientation_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Chest))
            .await;
        let mut seq = 0;

        for (i, (yaw, expected, label)) in [
            (
                YAW_FACING_NORTH,
                3988,
                "facing=north, type=single, waterlogged=false",
            ),
            (
                YAW_FACING_SOUTH,
                3994,
                "facing=south, type=single, waterlogged=false",
            ),
            (
                YAW_FACING_WEST,
                4000,
                "facing=west, type=single, waterlogged=false",
            ),
            (
                YAW_FACING_EAST,
                4006,
                "facing=east, type=single, waterlogged=false",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            rotate(&mut a, &world, uuid_a, yaw, 90.0).await;
            let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(i), 1).await;
            assert_eq!(id, expected, "chest at yaw {yaw} ({label})");
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn repeater_and_comparator_orientation_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Repeater))
            .await;
        for (i, (yaw, expected)) in [
            (YAW_FACING_NORTH, 7037),
            (YAW_FACING_SOUTH, 7041),
            (YAW_FACING_WEST, 7045),
            (YAW_FACING_EAST, 7049),
        ]
        .into_iter()
        .enumerate()
        {
            rotate(&mut a, &world, uuid_a, yaw, 90.0).await;
            let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(i), 1).await;
            assert_eq!(
                id, expected,
                "repeater at yaw {yaw} (delay=1, locked=false, powered=false)"
            );
        }

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Comparator))
            .await;
        for (i, (yaw, expected)) in [
            (YAW_FACING_NORTH, 11264),
            (YAW_FACING_SOUTH, 11268),
            (YAW_FACING_WEST, 11272),
            (YAW_FACING_EAST, 11276),
        ]
        .into_iter()
        .enumerate()
        {
            rotate(&mut a, &world, uuid_a, yaw, 90.0).await;
            let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(i + 4), 1).await;
            assert_eq!(
                id, expected,
                "comparator at yaw {yaw} (mode=compare, powered=false)"
            );
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn piston_and_sticky_piston_orientation_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        // Two representative non-default horizontal directions plus both vertical facings
        // (`pitch` steers `nearest_direction6` -- `mining_block_state_ids.rs`'s own identical
        // derivation) -- full 6-way `full6_piston_index` coverage across this one function.
        // `pitch = 0.0` (level look) here, deliberately NOT `90.0`: unlike every yaw-driven
        // horizontal-only block above, piston's own `nearest_direction6` reads the FULL 3-axis
        // look vector -- a steep pitch would make the vertical component dominate regardless of
        // yaw, exactly the mistake this file's own first draft made (piston.rs's own field
        // report: asserted `facing=south` at `pitch=90.0`, silently got `facing=up` instead).
        rotate(&mut a, &world, uuid_a, YAW_FACING_SOUTH, 0.0).await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(0), 1).await;
        assert_eq!(id, 2265, "piston facing=south");

        rotate(&mut a, &world, uuid_a, YAW_FACING_WEST, 0.0).await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(1), 1).await;
        assert_eq!(id, 2266, "piston facing=west");

        // Steeply-down / steeply-up looks select the vertical facings (`Up`/`Down`) regardless
        // of yaw (`look.y < 0 -> Down` per `mining.rs`'s own doc comment: pitch=90 looking down
        // selects `Down`, whose `.opposite()` is `Up`).
        rotate(&mut a, &world, uuid_a, 0.0, 90.0).await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(2), 1).await;
        assert_eq!(id, 2267, "piston facing=up (player looked steeply down)");

        rotate(&mut a, &world, uuid_a, 0.0, -90.0).await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(3), 1).await;
        assert_eq!(id, 2268, "piston facing=down (player looked steeply up)");

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::StickyPiston))
            .await;
        rotate(&mut a, &world, uuid_a, YAW_FACING_EAST, 0.0).await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(4), 1).await;
        assert_eq!(id, 2242, "sticky_piston facing=east");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn hopper_orientation_over_real_connection_including_the_down_case() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Hopper))
            .await;
        let mut seq = 0;

        // Face::Up (direction = 1) clicked on an ordinary floor tile -- the reported "hopper
        // placed facing down becomes QUARTZ" bug: the pre-fix `HOPPER.0 + 10` landed on
        // `minecraft:quartz_block`'s own id range (`mining_block_state_ids.rs`'s own doc
        // comment has the full citation).
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(0), 1).await;
        assert_eq!(
            id, 11313,
            "hopper clicked on floor -> enabled=true, facing=down"
        );

        // A reference Stone block's own 4 horizontal faces -- placement target = clicked
        // location + face offset, so clicking the reference's own South/North/East/West face
        // lands the hopper one cell further in that same direction; `hopper`'s own FACING =
        // clicked face's OPPOSITE.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let stone_floor = STONE_FLOOR;
        let stone_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_floor, 1).await;
        assert_ne!(stone_id, 0);
        let stone_pos = BlockPos::new(stone_floor.x, stone_floor.y + 1, stone_floor.z);

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Hopper))
            .await;
        // direction 3 = South face of the reference stone -> hopper lands south of it,
        // FACING = South.getOpposite() = North.
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 3).await;
        assert_eq!(
            id, 11314,
            "hopper on stone's south face -> enabled=true, facing=north"
        );
        // direction 2 = North face -> FACING = South.
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 2).await;
        assert_eq!(
            id, 11315,
            "hopper on stone's north face -> enabled=true, facing=south"
        );
        // direction 5 = East face -> FACING = West.
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 5).await;
        assert_eq!(
            id, 11316,
            "hopper on stone's east face -> enabled=true, facing=west"
        );
        // direction 4 = West face -> FACING = East.
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 4).await;
        assert_eq!(
            id, 11317,
            "hopper on stone's west face -> enabled=true, facing=east"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn wall_and_floor_redstone_torch_orientation_over_real_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // Floor torch: click Face::Up (1) on an ordinary floor tile.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(0), 1).await;
        assert_eq!(id, 6885, "floor torch -> lit=true");

        // Wall torch: a reference Stone's own 4 horizontal faces. `resolve_orientation`'s own
        // torch rule sets `FACING = clicked_face` directly (the direction the torch points
        // AWAY from the wall it's attached to) -- distinct from hopper's own `.opposite()` rule
        // above.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let stone_floor = STONE_FLOOR;
        let stone_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_floor, 1).await;
        assert_ne!(stone_id, 0);
        let stone_pos = BlockPos::new(stone_floor.x, stone_floor.y + 1, stone_floor.z);

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        // Before this fix, `REDSTONE_WALL_TORCH.0 + direction_offset(dir)` (no `lit` stride at
        // all) put South's own old `offset=1` at id 6888 = facing=north, lit=FALSE -- wrong
        // facing AND wrong (unlit) `lit` (`mining_block_state_ids.rs`'s own doc comment has the
        // full citation).
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 3).await;
        assert_eq!(
            id, 6889,
            "torch on stone's south face -> facing=south, lit=true"
        );
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 2).await;
        assert_eq!(
            id, 6887,
            "torch on stone's north face -> facing=north, lit=true"
        );
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 4).await;
        assert_eq!(
            id, 6891,
            "torch on stone's west face -> facing=west, lit=true"
        );
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 5).await;
        assert_eq!(
            id, 6893,
            "torch on stone's east face -> facing=east, lit=true"
        );
    })
    .await
    .unwrap();
}
