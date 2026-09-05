//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=yes self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(two components chained over a real connection at most -- lever + wire/piston -- see redstone_wire.rs for the pure ≥3-component chain) nondefault-state=yes
//! M3 field-report test-authoring (PLAN-D10/MECH-D13, wave 3, finding 2): the lever end-to-end
//! over real loopback connections -- mirrors `play_block_use_field_report.rs`'s own established
//! harness. Placement across all three attach faces (floor/wall/ceiling), the MECH-D82 toggle
//! (wire chain read, the bystander's own sound, the actor's own silence), the MECH-D84
//! support-loss pop (support block broken; a piston base's own face losing sturdiness on
//! extend), and the sneak-places-stone-instead override.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, VarLong, decode_one, encode_payload};
use rc_registries::block_state_properties::{properties, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::block_states::default_state::STONE;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PLAYER_INPUT_SHIFT, PlayerAction, PlayerInput, SectionBlocksUpdate,
    SetPlayerRotation, Sound, UseItemOn, pack_position, unpack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (mirrors `play_block_use_field_report.rs`'s own identical helpers) ---

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

/// Places `held` at `(location, direction)` and returns the direct `Block Update`'s own
/// `block_state_id` (mirrors `play_hopper_enabled_field_report.rs`'s own identical helper).
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

/// A plain block-use click (no placement expected) -- sends `UseItemOn` and consumes the
/// unconditional `Acknowledge Block Change` (mirrors `play_block_use_field_report.rs`'s own
/// identical helper).
async fn click(
    actor: &mut TcpStream,
    acc: &mut BytesMut,
    seq: &mut i32,
    location: BlockPos,
    direction: i32,
) {
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
}

async fn set_sneaking(actor: &mut TcpStream, sneaking: bool) {
    send_packet(
        actor,
        &PlayerInput {
            flags: if sneaking { PLAYER_INPUT_SHIFT } else { 0 },
        },
    )
    .await;
}

/// Waits until `check` returns `true` (mirrors `play_block_place_break.rs`'s own identical
/// `wait_until` helper).
async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Sends `SetPlayerRotation` and waits for it to actually be APPLIED (visible in the player's
/// own session record) before returning -- `mining_reach_validation.rs`'s/`play_block_place_
/// break.rs`'s own established convention: `resolve_orientation`'s own yaw/pitch inputs come
/// from the server's own tracked `PlayerMotion`, not from the `UseItemOn` packet itself, so a
/// dependent placement sent immediately after an un-applied rotation would still resolve
/// against the player's PRIOR orientation.
async fn set_yaw(world: &HardcodedWorld, uuid: u128, actor: &mut TcpStream, yaw: f32) {
    send_packet(
        actor,
        &SetPlayerRotation {
            yaw,
            pitch: 0.0,
            on_ground: true,
        },
    )
    .await;
    let sessions = world.player_sessions();
    let target_uuid = uuid::Uuid::from_u128(uuid);
    wait_until(|| sessions.with_record_mut(target_uuid, |r| r.data.rotation[0]) == Some(yaw)).await;
}

/// Scans clientbound traffic on `socket` for up to `window`, collecting every `Block Update`
/// whose own `location` matches one of `wanted` (mirrors `play_hopper_enabled_field_report.rs`'s
/// own identical helper).
fn sign_extend(value: i64, bits: u32) -> i64 {
    let sign_bit = 1i64 << (bits - 1);
    if value >= sign_bit {
        value - (1i64 << bits)
    } else {
        value
    }
}

/// M3.5-B06 (Context §3.1, mirrored from `play_redstone_field_report.rs`'s own identical
/// helper): `packets::pack_section_position`'s own exact inverse.
fn unpack_section_position(packed: i64) -> (i32, i32, i32) {
    let raw_x = (packed >> 42) & 0x3F_FFFF;
    let raw_y = packed & 0xF_FFFF;
    let raw_z = (packed >> 20) & 0x3F_FFFF;
    (
        sign_extend(raw_x, 22) as i32,
        sign_extend(raw_y, 20) as i32,
        sign_extend(raw_z, 22) as i32,
    )
}

/// `packets::pack_block_in_section`'s own exact inverse.
fn unpack_block_in_section(entry: VarLong) -> (u32, u8, u8, u8) {
    let packed = entry.get() as u64;
    let state_id = (packed >> 12) as u32;
    let local_x = ((packed >> 8) & 0xF) as u8;
    let local_z = ((packed >> 4) & 0xF) as u8;
    let local_y = (packed & 0xF) as u8;
    (state_id, local_x, local_z, local_y)
}

/// Two-or-more same-tick, same-section changes (e.g. the lever's own toggle plus the wire
/// chain it powers, all one hop apart) arrive bundled as one `SectionBlocksUpdate`, never as
/// separate `Block Update`s (`world.rs`'s own `broadcast_changed_positions`, M3.5-B06 Context
/// §3.1) -- this helper handles both wire shapes, mirroring `play_redstone_field_report.rs`'s
/// own identical `collect_block_updates_at`.
async fn collect_block_updates_at(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    wanted: &[BlockPos],
    window: Duration,
) -> std::collections::HashMap<BlockPos, i32> {
    let mut seen = std::collections::HashMap::new();
    let deadline = tokio::time::Instant::now() + window;
    while seen.len() < wanted.len() {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == BlockUpdate::ID => {
                let update = decode_one::<BlockUpdate>(body).unwrap();
                let pos = unpack_position(update.location);
                if wanted.contains(&pos) {
                    seen.insert(pos, update.block_state_id);
                }
            }
            Ok((id, body)) if id == SectionBlocksUpdate::ID => {
                let packet = decode_one::<SectionBlocksUpdate>(body).unwrap();
                let (chunk_x, section_y, chunk_z) = unpack_section_position(packet.section_pos);
                for entry in packet.states {
                    let (state_id, local_x, local_z, local_y) = unpack_block_in_section(entry);
                    let pos = BlockPos::new(
                        chunk_x * 16 + local_x as i32,
                        section_y * 16 + local_y as i32,
                        chunk_z * 16 + local_z as i32,
                    );
                    if wanted.contains(&pos) {
                        seen.insert(pos, state_id as i32);
                    }
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    seen
}

/// Drains every clientbound packet on `socket` for the FULL `window`, unconditionally (mirrors
/// `play_block_use_field_report.rs`'s own identical `drain_traffic_for` helper).
async fn drain_traffic_for(socket: &mut TcpStream, accumulator: &mut BytesMut, window: Duration) {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return;
        }
        if tokio::time::timeout(remaining, recv_clientbound(socket, accumulator))
            .await
            .is_err()
        {
            return;
        }
    }
}

fn lever_id(face: &str, facing: &str, powered: bool) -> i32 {
    state_id(
        block_id::LEVER,
        &[
            ("face", face),
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
    .expect("every (face,facing,powered) combination is a real lever state")
    .0 as i32
}

fn wire_power(raw: i32) -> u8 {
    properties(GenStateId(raw as u32))
        .iter()
        .find(|(name, _)| *name == "power")
        .map(|(_, v)| v.parse::<u8>().unwrap())
        .unwrap()
}

async fn place_lever(
    actor: &mut TcpStream,
    acc: &mut BytesMut,
    seq: &mut i32,
    location: BlockPos,
    direction: i32,
) -> i32 {
    place_and_read_id(actor, acc, seq, location, direction).await
}

#[tokio::test]
async fn floor_lever_placement_across_four_player_yaws_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Lever))
            .await;

        // (yaw, expected floor `facing`) -- this project's own `look_vector` convention
        // (`mining.rs`'s own doc comment): 0 -> South, 90 -> West, 180 -> North, 270 -> East.
        for (index, (yaw, expected_facing)) in [
            (0.0f32, "south"),
            (90.0, "west"),
            (180.0, "north"),
            (270.0, "east"),
        ]
        .into_iter()
        .enumerate()
        {
            set_yaw(&world, 1, &mut a, yaw).await;
            // `index + 1` -- never `index` alone -- keeps every target off the player's own
            // spawn column (`SPAWN_POSITION` = `(0, -60, 0)`, `connection.rs`'s own constant),
            // while staying well within creative reach (`BLOCK_INTERACTION_RANGE_CREATIVE` = 5,
            // plus the fixed 1-block verification buffer).
            let ground = BlockPos::new(index as i32 + 1, -61, 0);
            let target = BlockPos::new(index as i32 + 1, -60, 0);
            world.debug_set_block_state(ground, STONE.0).await;

            let placed = place_lever(&mut a, &mut a_acc, &mut seq, ground, 1).await;
            assert_eq!(
                placed,
                lever_id("floor", expected_facing, false),
                "yaw {yaw} must place a floor lever facing {expected_facing}"
            );
            let info = world.debug_query_block(target).await.unwrap();
            assert_eq!(info.raw_state as i32, placed);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn wall_lever_placement_on_each_face_of_a_placed_stone() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Lever))
            .await;

        // Clicking a stone's own side face places a wall lever whose own `facing` matches that
        // SAME clicked direction (it points away from the wall, in the direction the player
        // clicked toward) -- `resolve_orientation`'s own `Lever` arm doc comment. Four separate
        // stones, each offset from spawn (`(0, -60, 0)`) on its own distinct axis, keeps every
        // clicked location and target well within creative reach while never touching the
        // player's own spawn column or any other stone's own target.
        for (direction, expected_facing, stone_pos, offset) in [
            (2i32, "north", BlockPos::new(0, -60, 2), (0, 0, -1)),
            (3, "south", BlockPos::new(0, -60, -2), (0, 0, 1)),
            (4, "west", BlockPos::new(2, -60, -2), (-1, 0, 0)),
            (5, "east", BlockPos::new(-2, -60, -2), (1, 0, 0)),
        ] {
            world.debug_set_block_state(stone_pos, STONE.0).await;
            let target = BlockPos::new(
                stone_pos.x + offset.0,
                stone_pos.y + offset.1,
                stone_pos.z + offset.2,
            );

            let placed = place_lever(&mut a, &mut a_acc, &mut seq, stone_pos, direction).await;
            assert_eq!(
                placed,
                lever_id("wall", expected_facing, false),
                "clicking direction {direction} of a stone must place a wall lever facing {expected_facing}"
            );
            let info = world.debug_query_block(target).await.unwrap();
            assert_eq!(info.raw_state as i32, placed);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn ceiling_lever_placement_under_an_overhang() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        set_yaw(&world, 1, &mut a, 0.0).await; // South.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Lever))
            .await;

        let overhang = BlockPos::new(2, -59, 0);
        let target = BlockPos::new(2, -60, 0);
        world.debug_set_block_state(overhang, STONE.0).await;

        // Clicking the underside (Down, direction 0) of the overhang from below.
        let placed = place_lever(&mut a, &mut a_acc, &mut seq, overhang, 0).await;
        assert_eq!(placed, lever_id("ceiling", "south", false));
        let info = world.debug_query_block(target).await.unwrap();
        assert_eq!(info.raw_state as i32, placed);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn toggle_powers_a_wire_chain_sounds_the_bystander_and_toggling_off_depowers_it() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // Mount stone at (1,-60,0); lever on its east face -> lever at (2,-60,0), facing east,
        // mount = West = the stone. Every position here stays within creative reach of spawn
        // (`(0, -60, 0)`).
        let mount = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(mount, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Lever))
            .await;
        let lever_pos = BlockPos::new(2, -60, 0);
        let placed = place_lever(&mut a, &mut a_acc, &mut seq, mount, 5).await;
        assert_eq!(placed, lever_id("wall", "east", false));

        // A two-wire chain east of the lever, each on its own floor.
        let wire1_floor = BlockPos::new(3, -61, 0);
        let wire2_floor = BlockPos::new(4, -61, 0);
        world.debug_set_block_state(wire1_floor, STONE.0).await;
        world.debug_set_block_state(wire2_floor, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let wire1_pos = BlockPos::new(3, -60, 0);
        let wire2_pos = BlockPos::new(4, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, wire1_floor, 1).await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, wire2_floor, 1).await;

        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        // --- Toggle ON ---
        click(&mut a, &mut a_acc, &mut seq, lever_pos, 1).await;

        let seen = collect_block_updates_at(
            &mut b,
            &mut b_acc,
            &[lever_pos, wire1_pos, wire2_pos],
            Duration::from_millis(800),
        )
        .await;
        assert_eq!(
            seen.get(&lever_pos).copied(),
            Some(lever_id("wall", "east", true)),
            "the bystander must observe the lever flip to powered=true"
        );

        // Poll (bounded) for the wire chain's own async settle.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut wire1_final = None;
        let mut wire2_final = None;
        while tokio::time::Instant::now() < deadline {
            let w1 = world.debug_query_block(wire1_pos).await.unwrap().raw_state as i32;
            let w2 = world.debug_query_block(wire2_pos).await.unwrap().raw_state as i32;
            if wire_power(w1) == 15 && wire_power(w2) == 14 {
                wire1_final = Some(w1);
                wire2_final = Some(w2);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(
            wire1_final.map(wire_power),
            Some(15),
            "the wire directly beside the lever must read the lever's own weak 15"
        );
        assert_eq!(
            wire2_final.map(wire_power),
            Some(14),
            "the next wire in the chain must read one less, via ordinary wire decay"
        );

        // The actor never hears its own click; the bystander hears exactly one `Sound`,
        // pitch 0.6 (now powered), at the lever's own position.
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        let mut actor_sound = None;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, recv_clientbound(&mut a, &mut a_acc)).await {
                Ok((id, body)) if id == Sound::ID => {
                    actor_sound = Some(decode_one::<Sound>(body).unwrap());
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(
            actor_sound.is_none(),
            "the acting player must not hear its own click"
        );

        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        let mut bystander_sound = None;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, recv_clientbound(&mut b, &mut b_acc)).await {
                Ok((id, body)) if id == Sound::ID => {
                    bystander_sound = Some(decode_one::<Sound>(body).unwrap());
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let sound = bystander_sound.expect("the bystander must hear the click");
        assert_eq!(sound.pitch, 0.6);
        let expected_x = ((lever_pos.x as f64 + 0.5) * 8.0) as i32;
        let expected_y = ((lever_pos.y as f64 + 0.5) * 8.0) as i32;
        let expected_z = ((lever_pos.z as f64 + 0.5) * 8.0) as i32;
        assert_eq!(sound.x, expected_x);
        assert_eq!(sound.y, expected_y);
        assert_eq!(sound.z, expected_z);

        // --- Toggle OFF ---
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(200)).await;
        click(&mut a, &mut a_acc, &mut seq, lever_pos, 1).await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut both_zero = false;
        while tokio::time::Instant::now() < deadline {
            let w1 = world.debug_query_block(wire1_pos).await.unwrap().raw_state as i32;
            let w2 = world.debug_query_block(wire2_pos).await.unwrap().raw_state as i32;
            if wire_power(w1) == 0 && wire_power(w2) == 0 {
                both_zero = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            both_zero,
            "toggling the lever off must depower the whole chain"
        );
        let lever_after = world.debug_query_block(lever_pos).await.unwrap().raw_state as i32;
        assert_eq!(lever_after, lever_id("wall", "east", false));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn breaking_the_support_block_pops_the_lever_to_air() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let mount = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(mount, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Lever))
            .await;
        let lever_pos = BlockPos::new(2, -60, 0);
        let placed = place_lever(&mut a, &mut a_acc, &mut seq, mount, 5).await;
        assert_eq!(placed, lever_id("wall", "east", false));

        // A real break (not `debug_set_block_state`, which bypasses shape-update dispatch
        // entirely) -- looking straight at the mount stone and breaking it. Creative/instabuild
        // (this world's own default game-mode, `GameModeState`'s own doc comment) breaks any
        // block in a single tick, so `StartDestroy` alone suffices (mirrors
        // `play_block_place_break.rs`'s own identical single-packet break).
        seq += 1;
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(mount),
                direction: 5,
                sequence: seq,
            },
        )
        .await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            seq
        );

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut lever_is_air = false;
        while tokio::time::Instant::now() < deadline {
            let raw = world.debug_query_block(lever_pos).await.unwrap().raw_state;
            if raw == rc_registries::generated_v776::block_states::default_state::AIR.0 {
                lever_is_air = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            lever_is_air,
            "breaking the mount must pop the lever to air (MECH-D84 support-loss)"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn lever_on_a_piston_base_pops_when_the_piston_extends_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // A north-facing piston (pushes north): when extended, its own NORTH face (the
        // recessed end) loses ALL sturdiness, its own SOUTH face (`facing.opposite()`) stays
        // Full-sturdy forever (MECH-D84). A piston faces AWAY from the player's own look
        // direction (`resolve_orientation`'s own Piston arm: `nearest_direction6(..).
        // opposite()`) -- yaw 0 looks South, so the piston faces North.
        set_yaw(&world, 1, &mut a, 0.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        // Kept close to spawn (`(0, -60, 0)`) throughout -- creative reach is only 5 blocks
        // (plus a fixed 1-block verification buffer), and the player never moves in this test.
        let piston_pos = BlockPos::new(2, -60, 0);
        let piston_placed =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_props = properties(GenStateId(piston_placed as u32));
        assert_eq!(
            piston_props.iter().find(|(n, _)| *n == "facing").unwrap().1,
            "north",
            "yaw 0 must place a north-facing piston"
        );

        // Test lever on the EAST face (perpendicular to facing -- Center-only once extended,
        // so it pops) and the power-source lever on the SOUTH face (`facing.opposite()` --
        // stays Full-sturdy forever, so it survives to keep the piston powered).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Lever))
            .await;
        let test_lever_pos = BlockPos::new(3, -60, 0);
        let test_lever_placed =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, piston_pos, 5).await;
        assert_eq!(test_lever_placed, lever_id("wall", "east", false));

        let power_lever_pos = BlockPos::new(2, -60, 1);
        let power_lever_placed =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, piston_pos, 3).await;
        assert_eq!(power_lever_placed, lever_id("wall", "south", false));

        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;
        click(&mut a, &mut a_acc, &mut seq, power_lever_pos, 1).await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let mut test_lever_popped = false;
        while tokio::time::Instant::now() < deadline {
            let raw = world
                .debug_query_block(test_lever_pos)
                .await
                .unwrap()
                .raw_state;
            if raw == rc_registries::generated_v776::block_states::default_state::AIR.0 {
                test_lever_popped = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            test_lever_popped,
            "the lever on the piston base's own east face must pop once the piston extends \
             (that face is only Center-sturdy, never Full, on an extended horizontal-facing base)"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn sneaking_with_stone_in_hand_places_stone_instead_of_toggling_the_lever_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let mount = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(mount, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Lever))
            .await;
        let lever_pos = BlockPos::new(2, -60, 0);
        let placed = place_lever(&mut a, &mut a_acc, &mut seq, mount, 5).await;
        assert_eq!(placed, lever_id("wall", "east", false));

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        set_sneaking(&mut a, true).await;
        let above_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, lever_pos, 1).await;
        assert_eq!(
            above_id, STONE.0 as i32,
            "sneaking must place stone above the lever, not toggle it"
        );

        let after = world.debug_query_block(lever_pos).await.unwrap().raw_state;
        assert_eq!(
            after as i32, placed,
            "the lever's own state must be untouched by a sneak-click"
        );
    })
    .await
    .unwrap();
}
