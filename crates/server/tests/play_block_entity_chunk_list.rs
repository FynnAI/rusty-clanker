//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(single canonical value/facing asserted, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain; three kinds placed independently, not a chained transfer) nondefault-state=yes
//! M3-B0X test-authoring: production block entities end-to-end (owner's real-client field
//! report, "chest placed, rejoin -> invisible" -- `docs/findings-for-planning.md` §B: "Stage
//! 7's own production wiring is closed, but nothing yet spawns a real block entity for it to
//! tick"). Mirrors `play_block_state_orientation_real_client.rs`'s own established
//! real-connection shape (per-file helper duplication -- "no shared `tests/` support module
//! exists in this crate today").
//!
//! Every scenario below drives the exact client-visible symptom the owner reported: a chest
//! placed by one real connection must be visible (client-rendered, i.e. present in the chunk
//! packet's own block-entity list with the real `minecraft:block_entity_type` id) to a SECOND
//! real connection that joins afterward -- the chunk packet a rejoin/second-player receives is
//! this project's own only channel for a client to ever learn a block entity exists at all
//! (M4's own future scope adds the container-menu channel; no other production path exists at
//! M3). Breaking the block entity must remove it from a later joiner's own chunk packet.
//!
//! Coordinates: every actor stays at `HardcodedWorld`'s own default spawn (`SPAWN_POSITION =
//! (0, -60, 0)`, `connection.rs`), so every placement target here lands inside chunk `(0, 0)`
//! -- the one chunk `capture_chunk_zero`, below, extracts from each joiner's own Play-entry
//! chunk batch. No two placements in one test ever target the same cell (`TargetNotAir` would
//! otherwise reject the second one).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::registries::block_entity_type;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockEntityInfo, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, LevelChunkWithLight, PlayerAction, SetPlayerRotation, UseItemOn,
    pack_position, unpack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (mirrors `play_block_state_orientation_real_client.rs`'s own identical
// helpers -- this crate's own established per-file-duplication convention) ---

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

/// Drains Play-entry's own fixed 6-packet header (mirrors `drain_play_entry`'s own
/// established precedent) then every `LevelChunkWithLight` of this join's own chunk batch,
/// returning the ONE such packet whose `(chunk_x, chunk_z) == (0, 0)` -- the chunk every
/// placement target in this file lands inside (module doc comment). Panics if `(0, 0)` never
/// arrives (a real defect this file's own tests must not silently swallow).
async fn spawn_actor_capturing_chunk_zero(
    world: &HardcodedWorld,
    username: &str,
    uuid: u128,
) -> (TcpStream, BytesMut, LevelChunkWithLight) {
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
    for _ in 0..6 {
        recv_clientbound(&mut client, &mut accumulator).await;
    }
    let mut chunk_zero = None;
    loop {
        let (id, body) = recv_clientbound(&mut client, &mut accumulator).await;
        if id == LevelChunkWithLight::ID {
            let chunk = decode_one::<LevelChunkWithLight>(body).unwrap();
            if (chunk.chunk_x, chunk.chunk_z) == (0, 0) {
                chunk_zero = Some(chunk);
            }
            continue;
        }
        if id == ChunkBatchFinished::ID {
            break;
        }
    }
    let chunk_zero = chunk_zero.expect("chunk (0, 0) was never sent during this join's own batch");
    (client, accumulator, chunk_zero)
}

/// The placing actor's own join -- discards chunk content (mirrors `play_block_state_
/// orientation_real_client.rs`'s own identical `spawn_actor`, restated here since integration
/// tests cannot share code across files in this crate today).
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
    for _ in 0..6 {
        recv_clientbound(&mut client, &mut accumulator).await;
    }
    loop {
        let (id, _) = recv_clientbound(&mut client, &mut accumulator).await;
        if id == ChunkBatchFinished::ID {
            break;
        }
    }
    (client, accumulator)
}

async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Sets `yaw` and waits for it to land server-side -- `resolve_orientation`'s own horizontal
/// placement rule (repeater/comparator/chest/furnace-family) reads yaw, not clicked face.
async fn rotate(actor: &mut TcpStream, world: &HardcodedWorld, uuid: uuid::Uuid, yaw: f32) {
    send_packet(
        actor,
        &SetPlayerRotation {
            yaw,
            pitch: 90.0,
            on_ground: true,
        },
    )
    .await;
    wait_until(|| {
        world
            .player_sessions()
            .with_record_mut(uuid, |r| r.data.rotation)
            == Some([yaw, 90.0])
    })
    .await;
}

/// Sequence-numbered `Use Item On` + response scan (mirrors `place_and_read_id`'s own
/// established shape) -- returns the broadcast `Block Update`'s own `block_state_id`.
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
    // M3 field-report necessary exception (MECH-D78 -- `respond_place` now ALWAYS also
    // resends the CLICKED cell's own live state to the actor first, ahead of the
    // placement-direction cell's own value, on every outcome including a rejection --
    // this helper's own historical "the first Block Update is the answer" assumption held
    // only because a rejection used to send nothing but that one corrective update).
    // Skips any Block Update whose own position is exactly the clicked `location`; the
    // first one that is NOT is the placement-direction cell's own value, this helper's
    // real, unchanged contract.
    loop {
        let body = recv_packet_of_type(actor, acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        if unpack_position(update.location) != location {
            return update.block_state_id;
        }
    }
}

/// A creative-instant break (status `0` = `StartDestroyBlock`; every actor in this file is
/// creative by default, `GameModeState`'s own `instabuild: true` join default, `world.rs`) --
/// waits for the matching `AcknowledgeBlockChange` + `Block Update` (to `AIR`), mirroring
/// `play_block_place_break.rs`'s own established shape.
async fn break_block(actor: &mut TcpStream, acc: &mut BytesMut, seq: &mut i32, pos: BlockPos) {
    *seq += 1;
    send_packet(
        actor,
        &PlayerAction {
            status: 0,
            location: pack_position(pos),
            direction: 1,
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
    let update = decode_one::<BlockUpdate>(body).unwrap();
    assert_eq!(update.location, pack_position(pos));
    assert_eq!(update.block_state_id, 0, "broken block must become air");
}

/// The `BlockEntityInfo` entry (if any) `chunk` carries at absolute `pos` -- decodes
/// `packed_xz`/`y` back into a world position exactly as `packet_capture.rs`'s own established
/// `record_chunk_block_entities` does (module doc comment there: "minecraft.wiki's own
/// documented Chunk Data wire format"), restated here since this crate's own decode uses the
/// production `LevelChunkWithLight`/`BlockEntityInfo` types directly (`rc_protocol::decode_one`)
/// rather than azalea's.
fn block_entity_at(chunk: &LevelChunkWithLight, pos: BlockPos) -> Option<BlockEntityInfo> {
    chunk
        .block_entities
        .iter()
        .find(|entry| {
            let local_x = (entry.packed_xz >> 4) & 0x0F;
            let local_z = entry.packed_xz & 0x0F;
            let world_pos = BlockPos::new(
                chunk.chunk_x * 16 + local_x as i32,
                entry.y as i32,
                chunk.chunk_z * 16 + local_z as i32,
            );
            world_pos == pos
        })
        .copied()
}

// Eight distinct floor columns near spawn (mirrors `play_block_state_orientation_real_client.
// rs`'s own `FLOOR_COLS`), all within creative reach and all inside chunk `(0, 0)`.
const FLOOR_COLS: [(i32, i32); 4] = [(1, 0), (2, 0), (3, 0), (1, 1)];

fn floor(i: usize) -> BlockPos {
    let (dx, dz) = FLOOR_COLS[i];
    BlockPos::new(dx, -61, dz)
}

#[tokio::test]
async fn chest_furnace_hopper_appear_in_a_second_joiners_chunk_block_entity_list() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Chest))
            .await;
        let chest_pos = BlockPos::new(floor(0).x, floor(0).y + 1, floor(0).z);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(0), 1).await;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Furnace))
            .await;
        let furnace_pos = BlockPos::new(floor(1).x, floor(1).y + 1, floor(1).z);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(1), 1).await;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Hopper))
            .await;
        let hopper_pos = BlockPos::new(floor(2).x, floor(2).y + 1, floor(2).z);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(2), 1).await;

        // --- A second connection joins AFTER every placement above -- the exact real-client
        // symptom the owner reported (rejoin, not merely "the server's own internal state").
        let (mut b, _b_acc, chunk) = spawn_actor_capturing_chunk_zero(&world, "b", 2).await;

        let chest_entry = block_entity_at(&chunk, chest_pos)
            .expect("chest must appear in a second joiner's own chunk block-entity list");
        assert_eq!(
            chest_entry.type_id,
            block_entity_type::CHEST.0, // 1
            "chest's own minecraft:block_entity_type id"
        );

        let furnace_entry = block_entity_at(&chunk, furnace_pos)
            .expect("furnace must appear in a second joiner's own chunk block-entity list");
        assert_eq!(
            furnace_entry.type_id,
            block_entity_type::FURNACE.0, // 0
            "furnace's own minecraft:block_entity_type id"
        );

        let hopper_entry = block_entity_at(&chunk, hopper_pos)
            .expect("hopper must appear in a second joiner's own chunk block-entity list");
        assert_eq!(
            hopper_entry.type_id,
            block_entity_type::HOPPER.0, // 18
            "hopper's own minecraft:block_entity_type id"
        );

        // Exactly these three entries -- no stray/duplicate entries from anywhere else in
        // this chunk (the superflat floor's own bedrock/dirt/grass_block ids never match any
        // of the six `BlockEntityWireKind`s).
        assert_eq!(chunk.block_entities.len(), 3, "{:?}", chunk.block_entities);

        // --- Break the chest; a THIRD connection joining afterward must no longer see it,
        // while furnace and hopper (never broken) remain visible.
        break_block(&mut a, &mut a_acc, &mut seq, chest_pos).await;

        let (mut c, _c_acc, chunk_after_break) =
            spawn_actor_capturing_chunk_zero(&world, "c", 3).await;
        assert!(
            block_entity_at(&chunk_after_break, chest_pos).is_none(),
            "a broken chest must not appear in a later joiner's own chunk block-entity list: {:?}",
            chunk_after_break.block_entities
        );
        assert!(block_entity_at(&chunk_after_break, furnace_pos).is_some());
        assert!(block_entity_at(&chunk_after_break, hopper_pos).is_some());
        assert_eq!(chunk_after_break.block_entities.len(), 2);

        let _ = a.shutdown().await;
        let _ = b.shutdown().await;
        let _ = c.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn comparator_appears_in_the_chunk_block_entity_list_without_a_tracked_ecs_entity() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // Comparator resolves orientation from yaw (`resolve_orientation`'s own shared
        // horizontal-kind arm) -- yaw 0.0 -> FACING = South (`nearest_horizontal_direction4(0.0)
        // .opposite()`), irrelevant to this test beyond landing on a real, in-range id.
        rotate(&mut a, &world, uuid_a, 0.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Comparator))
            .await;
        let comparator_pos = BlockPos::new(floor(3).x, floor(3).y + 1, floor(3).z);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(3), 1).await;

        let (mut b, _b_acc, chunk) = spawn_actor_capturing_chunk_zero(&world, "b", 2).await;
        let entry = block_entity_at(&chunk, comparator_pos)
            .expect("comparator must appear in a second joiner's own chunk block-entity list");
        assert_eq!(entry.type_id, block_entity_type::COMPARATOR.0); // 19
        assert_eq!(chunk.block_entities.len(), 1, "{:?}", chunk.block_entities);

        let _ = a.shutdown().await;
        let _ = b.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn hopper_placed_beside_a_lit_redstone_torch_starts_disabled_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // Floor torch at `floor(0)` -> final position `(1, -60, 0)`, lit=true (a freshly
        // placed torch is always lit, unpowered support -- `play_block_state_orientation_
        // real_client.rs`'s own `wall_and_floor_redstone_torch_orientation_over_real_
        // connection` test proves this same id independently).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(0), 1).await;
        assert_eq!(id, 6885, "floor torch -> lit=true");

        // Hopper at `floor(1) = (2, -61, 0)`, Face::Up -> final position `(2, -60, 0)`,
        // horizontally adjacent (West neighbor is the torch) -- `TorchBehavior::
        // weak_signal_toward` gives `15` toward any direction except its own `Down` input
        // face, so this hopper's own `best_neighbor_signal` reads `> 0` (`redstone::signal`).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Hopper))
            .await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(1), 1).await;
        // source: blocks.json
        assert_eq!(
            id, 11318,
            "hopper placed beside a lit redstone torch -> enabled=false, facing=down \
             (11313 base + 5 enabled-false stride + 0 facing=down)"
        );

        let _ = a.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn hopper_placed_with_no_neighbor_signal_stays_enabled() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Hopper))
            .await;
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor(0), 1).await;
        // source: blocks.json
        assert_eq!(
            id, 11313,
            "hopper with no neighbor signal -> enabled=true, facing=down"
        );

        let _ = a.shutdown().await;
    })
    .await
    .unwrap();
}
