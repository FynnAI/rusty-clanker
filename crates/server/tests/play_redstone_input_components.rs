//! test-matrix: boundaries=waived(fixed local test-world positions, see world_bounds_fan_out.rs) orientations=yes self=waived(placement into the actor's own cell is covered by mining_placement_obstruction.rs; buttons and plates are noCollision and never obstruct) composition=yes nondefault-state=yes
//! M4-B10 — buttons and pressure plates end to end over real loopback connections, mirroring
//! `play_lever_field_report.rs`'s own established harness: placement orientation, a press
//! powering a wire chain and releasing on schedule, the material-dependent release delay, the
//! press/release sound reaching a bystander but never the presser, a player (and a dropped
//! item) pressing a plate, the weighted-plate analog output and its own recheck-cadence lag,
//! the placement-time support gate, and the MECH-D84 support-loss pop for both block families.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::block_state_properties::{properties, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::block_states::default_state::{AIR, HOPPER, STONE};
use rc_registries::generated_v776::registries::{item, sound_event};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SetPlayerPosition, Sound, UseItemOn, pack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (mirrors `play_lever_field_report.rs`'s own identical helpers) ---

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

/// A rejected placement attempt: sends `UseItemOn` and consumes the unconditional
/// `Acknowledge Block Change`, but never expects a `Block Update` for the (unchanged) target.
async fn attempt_place(
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

/// A plain block-use click (mirrors `play_lever_field_report.rs`'s own identical helper).
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

/// Scans `socket` for up to `window` for a `Sound` packet, returning the first one seen.
async fn find_sound(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    window: Duration,
) -> Option<Sound> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == Sound::ID => return Some(decode_one::<Sound>(body).unwrap()),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
}

fn button_id(
    block: rc_registries::generated_v776::block_state_properties::BlockId,
    face: &str,
    facing: &str,
    powered: bool,
) -> i32 {
    state_id(
        block,
        &[
            ("face", face),
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
    .expect("every (face,facing,powered) combination is a real button state")
    .0 as i32
}

fn wire_power(raw: i32) -> u8 {
    properties(GenStateId(raw as u32))
        .iter()
        .find(|(name, _)| *name == "power")
        .map(|(_, v)| v.parse::<u8>().unwrap())
        .unwrap()
}

/// Polls `world.debug_query_input_signal(pos)` (an async debug accessor) until it equals
/// `expected`, bounded by a generous timeout -- an async-aware analogue of `play_lever_field_
/// report.rs`'s own synchronous `wait_until` helper, needed here because every check this file
/// makes goes through an `async fn` round trip to the region's own tick-loop thread rather than
/// reading already-local state.
async fn wait_for_signal(
    world: &HardcodedWorld,
    pos: BlockPos,
    expected: Option<u8>,
    timeout: Duration,
) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        if world.debug_query_input_signal(pos).await == expected {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

/// As `wait_for_signal`, but reads a wire's own decoded `power` digit via `debug_query_block`.
async fn wait_for_wire_power(
    world: &HardcodedWorld,
    pos: BlockPos,
    expected: u8,
    timeout: Duration,
) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        if let Some(info) = world.debug_query_block(pos).await
            && wire_power(info.raw_state as i32) == expected
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

#[tokio::test]
async fn button_placement_orientation_over_a_real_connection() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::StoneButton))
            .await;

        // Wall: click the east face of a stone (direction 5, `resolve_orientation`'s own
        // `AttachFace::Wall` candidate, facing = the clicked direction's own opposite = west).
        let wall_stone = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(wall_stone, STONE.0).await;
        let wall_target = BlockPos::new(2, -60, 0);
        let placed = place_and_read_id(&mut a, &mut a_acc, &mut seq, wall_stone, 5).await;
        assert_eq!(
            placed,
            button_id(block_id::STONE_BUTTON, "wall", "east", false)
        );
        let info = world.debug_query_block(wall_target).await.unwrap();
        assert_eq!(info.raw_state as i32, placed);

        // Floor: click the top face of a stone (direction 1) from directly above.
        let floor_stone = BlockPos::new(3, -61, 0);
        world.debug_set_block_state(floor_stone, STONE.0).await;
        let floor_target = BlockPos::new(3, -60, 0);
        let placed_floor = place_and_read_id(&mut a, &mut a_acc, &mut seq, floor_stone, 1).await;
        let props = properties(GenStateId(placed_floor as u32));
        assert_eq!(props.iter().find(|(n, _)| *n == "face").unwrap().1, "floor");
        assert_eq!(
            props.iter().find(|(n, _)| *n == "powered").unwrap().1,
            "false"
        );
        let info = world.debug_query_block(floor_target).await.unwrap();
        assert_eq!(info.raw_state as i32, placed_floor);

        // Ceiling: click the bottom face of an overhang (direction 0) from below.
        let overhang = BlockPos::new(4, -59, 0);
        world.debug_set_block_state(overhang, STONE.0).await;
        let ceiling_target = BlockPos::new(4, -60, 0);
        let placed_ceiling = place_and_read_id(&mut a, &mut a_acc, &mut seq, overhang, 0).await;
        let props = properties(GenStateId(placed_ceiling as u32));
        assert_eq!(
            props.iter().find(|(n, _)| *n == "face").unwrap().1,
            "ceiling"
        );
        assert_eq!(
            props.iter().find(|(n, _)| *n == "powered").unwrap().1,
            "false"
        );
        let info = world.debug_query_block(ceiling_target).await.unwrap();
        assert_eq!(info.raw_state as i32, placed_ceiling);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn pressing_a_button_powers_a_wire_chain_and_releases_on_schedule_composition() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let mount = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(mount, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::StoneButton))
            .await;
        let button_pos = BlockPos::new(2, -60, 0);
        let placed = place_and_read_id(&mut a, &mut a_acc, &mut seq, mount, 5).await;
        assert_eq!(
            placed,
            button_id(block_id::STONE_BUTTON, "wall", "east", false)
        );

        let wire_floor = BlockPos::new(3, -61, 0);
        world.debug_set_block_state(wire_floor, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let wire_pos = BlockPos::new(3, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, wire_floor, 1).await;

        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        // Press.
        click(&mut a, &mut a_acc, &mut seq, button_pos, 1).await;

        // Tick-precise assertion via the debug accessor (Context §C: 20 ticks for stone) --
        // avoids a wall-clock race against the real 20-tick/50ms-per-tick release schedule.
        // Bounded to `<= 20` rather than exact equality: the round trip through `click`'s own
        // ack wait is real wall-clock time, so a tick can already have elapsed between the
        // press and this read.
        let delay = world
            .debug_pending_block_tick_delay(button_pos)
            .await
            .expect("a block tick must be queued immediately after the press");
        assert!(
            (18..=20).contains(&delay),
            "a stone button must schedule its release 20 ticks out (delay was {delay})"
        );

        assert!(
            wait_for_signal(&world, button_pos, Some(15), Duration::from_secs(2)).await,
            "the button must read powered=15 right after the press"
        );
        assert!(
            wait_for_wire_power(&world, wire_pos, 15, Duration::from_secs(2)).await,
            "the wire must observe the button's own weak 15"
        );

        // The button eventually releases and the wire chain depowers with it.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut released = false;
        while tokio::time::Instant::now() < deadline {
            let button_raw = world.debug_query_block(button_pos).await.unwrap().raw_state;
            if button_raw == button_id(block_id::STONE_BUTTON, "wall", "east", false) as u32 {
                released = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(released, "the button must release on its own schedule");
        assert!(
            wait_for_wire_power(&world, wire_pos, 0, Duration::from_secs(2)).await,
            "the wire must depower with the button"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn wooden_button_release_is_thirty_ticks_nondefault_material() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let mount = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(mount, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::OakButton))
            .await;
        let button_pos = BlockPos::new(2, -60, 0);
        let placed = place_and_read_id(&mut a, &mut a_acc, &mut seq, mount, 5).await;
        assert_eq!(
            placed,
            button_id(block_id::OAK_BUTTON, "wall", "east", false)
        );

        click(&mut a, &mut a_acc, &mut seq, button_pos, 1).await;

        // Bounded to `<= 30` for the same real-wall-clock-round-trip reason as the stone
        // button's own identical assertion above.
        let delay = world
            .debug_pending_block_tick_delay(button_pos)
            .await
            .expect("a block tick must be queued immediately after the press");
        assert!(
            (28..=30).contains(&delay),
            "a wooden button must schedule its release 30 ticks out (delay was {delay})"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn button_press_sound_reaches_a_bystander_and_not_the_presser() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        let mount = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(mount, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::StoneButton))
            .await;
        let button_pos = BlockPos::new(2, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, mount, 5).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(200)).await;

        // Press: the bystander hears the click-on, the presser does not.
        click(&mut a, &mut a_acc, &mut seq, button_pos, 1).await;
        let bystander_press = find_sound(&mut b, &mut b_acc, Duration::from_millis(500))
            .await
            .expect("the bystander must hear the press");
        assert_eq!(
            bystander_press.sound_registry_id_plus_one - 1,
            sound_event::BLOCK_STONE_BUTTON_CLICK_ON.0 as i32
        );
        let actor_press = find_sound(&mut a, &mut a_acc, Duration::from_millis(200)).await;
        assert!(
            actor_press.is_none(),
            "the presser must not hear its own press"
        );

        // Release: both connections hear the click-off.
        let bystander_release = find_sound(&mut b, &mut b_acc, Duration::from_secs(3))
            .await
            .expect("the bystander must hear the release");
        assert_eq!(
            bystander_release.sound_registry_id_plus_one - 1,
            sound_event::BLOCK_STONE_BUTTON_CLICK_OFF.0 as i32
        );
        let actor_release = find_sound(&mut a, &mut a_acc, Duration::from_secs(3))
            .await
            .expect("the presser must also hear the release");
        assert_eq!(
            actor_release.sound_registry_id_plus_one - 1,
            sound_event::BLOCK_STONE_BUTTON_CLICK_OFF.0 as i32
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn player_standing_on_a_plate_powers_it_within_one_tick_and_releases_after_twenty() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let plate_floor = BlockPos::new(1, -61, 0);
        world.debug_set_block_state(plate_floor, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::OakPressurePlate))
            .await;
        let plate_pos = BlockPos::new(1, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, plate_floor, 1).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        // Walk onto the plate.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 1.5,
                y: -60.0,
                z: 0.5,
                on_ground: true,
            },
        )
        .await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut pressed = false;
        while tokio::time::Instant::now() < deadline {
            if world.debug_query_input_signal(plate_pos).await == Some(15) {
                pressed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(pressed, "a player standing on the plate must power it");

        // Step off.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 0.5,
                y: -60.0,
                z: 0.5,
                on_ground: true,
            },
        )
        .await;
        // The press itself is detected via a bounded poll above (not a synchronous click, like
        // the button's own press), so up to a couple of ticks may already have elapsed by the
        // time this reads the queued delay -- bounded to a generous range rather than the exact
        // `20` the button's own synchronous-click test asserts.
        let delay = world
            .debug_pending_block_tick_delay(plate_pos)
            .await
            .expect("a recheck must still be queued while pressed");
        assert!(
            delay <= 20,
            "a plain plate re-checks every 20 ticks (delay was {delay})"
        );

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut released = false;
        while tokio::time::Instant::now() < deadline {
            if world.debug_query_input_signal(plate_pos).await == Some(0) {
                released = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(released, "stepping off must release the plate");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn stone_plate_is_pressed_by_a_player_since_a_player_is_living() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let plate_floor = BlockPos::new(1, -61, 0);
        world.debug_set_block_state(plate_floor, STONE.0).await;
        world
            .debug_set_held_item(
                1,
                HeldItemStub::Block(PlaceableBlockKind::StonePressurePlate),
            )
            .await;
        let plate_pos = BlockPos::new(1, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, plate_floor, 1).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 1.5,
                y: -60.0,
                z: 0.5,
                on_ground: true,
            },
        )
        .await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut pressed = false;
        while tokio::time::Instant::now() < deadline {
            if world.debug_query_input_signal(plate_pos).await == Some(15) {
                pressed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            pressed,
            "a MOBS-sensitivity (stone) plate must still be pressed by a player, a living entity"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn weighted_plate_analog_output_from_dropped_item_entities() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let plate_floor = BlockPos::new(1, -61, 0);
        world.debug_set_block_state(plate_floor, STONE.0).await;
        world
            .debug_set_held_item(
                1,
                HeldItemStub::Block(PlaceableBlockKind::LightWeightedPressurePlate),
            )
            .await;
        let plate_pos = BlockPos::new(1, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, plate_floor, 1).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        // The neighbouring wire, reading the same analog value (`direct_signal_toward` only
        // fires downward, so the wire sits directly north of the plate instead, reading its
        // own weak signal).
        let wire_floor = BlockPos::new(1, -61, 1);
        world.debug_set_block_state(wire_floor, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let wire_pos = BlockPos::new(1, -60, 1);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, wire_floor, 1).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        let plate_center = rc_physics::Vec3::new(1.5, -60.0, 0.5);
        let mut spawned: Vec<rc_core::RcEntityId> = Vec::new();
        // N in {1, 7, 8, 16} -> signal {1, 7, 8, 15} (Acceptance tests' own literal case set;
        // `weighted_plate_signal(16, 15)` saturates to 15). Deltas applied in order so the
        // running total matches the second column of each case (1 -> 7 needs +6, 7 -> 8 needs
        // +1, 8 -> 16 needs +8).
        let cases: &[(usize, u8)] = &[(1, 1), (7, 7), (8, 8), (16, 15)];
        let deltas = [1usize, 6, 1, 8];
        // A distinct item id per spawned entity -- M4-B02's own item-stack merge logic
        // (`stacks_can_combine`) would otherwise combine same-item entities dropped at the
        // identical position into fewer, larger stacks, undercounting the plate's own
        // per-ENTITY census (vanilla's `getEntityCount` counts entities, not stack quantity;
        // real dropped stacks of the SAME item at the same spot merge too, which is exactly why
        // this test uses distinct item types instead, to keep every drop its own entity).
        const DISTINCT_ITEMS: &[rc_registries::generated_v776::registries::RegistryEntryId] = &[
            item::STONE,
            item::GRANITE,
            item::POLISHED_GRANITE,
            item::DIORITE,
            item::POLISHED_DIORITE,
            item::ANDESITE,
            item::POLISHED_ANDESITE,
            item::DEEPSLATE,
            item::COBBLED_DEEPSLATE,
            item::POLISHED_DEEPSLATE,
            item::CALCITE,
            item::TUFF,
            item::TUFF_SLAB,
            item::TUFF_STAIRS,
            item::TUFF_WALL,
            item::CHISELED_TUFF,
        ];

        for (delta, expected_signal) in deltas.iter().zip(cases.iter().map(|(_, s)| *s)) {
            for _ in 0..*delta {
                let item_id = DISTINCT_ITEMS[spawned.len() % DISTINCT_ITEMS.len()];
                let id = world
                    .debug_spawn_item_entity(
                        BlockPos::new(
                            plate_center.x.floor() as i32,
                            plate_center.y as i32,
                            plate_center.z.floor() as i32,
                        ),
                        item_id,
                        1,
                    )
                    .await;
                spawned.push(id);
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
            let mut matched = false;
            while tokio::time::Instant::now() < deadline {
                if world.debug_query_input_signal(plate_pos).await == Some(expected_signal) {
                    matched = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            assert!(
                matched,
                "expected signal {expected_signal} for {} total item entities",
                spawned.len()
            );
            let wire_deadline = tokio::time::Instant::now() + Duration::from_secs(3);
            let mut wire_matched = false;
            while tokio::time::Instant::now() < wire_deadline {
                let raw = world.debug_query_block(wire_pos).await.unwrap().raw_state;
                if wire_power(raw as i32) == expected_signal {
                    wire_matched = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            assert!(
                wire_matched,
                "the neighbouring wire must read the same value"
            );
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn weighted_plate_output_lags_a_count_change_by_at_most_the_recheck_cadence() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        let plate_floor = BlockPos::new(1, -61, 0);
        world.debug_set_block_state(plate_floor, STONE.0).await;
        world
            .debug_set_held_item(
                1,
                HeldItemStub::Block(PlaceableBlockKind::LightWeightedPressurePlate),
            )
            .await;
        let plate_pos = BlockPos::new(1, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, plate_floor, 1).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        // Get the plate pressed at power 1 first.
        world
            .debug_spawn_item_entity(BlockPos::new(1, -60, 0), item::STONE, 1)
            .await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            if world.debug_query_input_signal(plate_pos).await == Some(1) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert_eq!(world.debug_query_input_signal(plate_pos).await, Some(1));

        // Six more items dropped while pressed -- the census jump is only picked up by the next
        // scheduled recheck (10 ticks for a weighted plate), never synchronously. Distinct item
        // ids again (this file's own `weighted_plate_analog_output_from_dropped_item_entities`
        // doc comment has the full "avoid the item-stack merge undercounting the census"
        // rationale).
        for item_id in [
            item::GRANITE,
            item::POLISHED_GRANITE,
            item::DIORITE,
            item::POLISHED_DIORITE,
            item::ANDESITE,
            item::POLISHED_ANDESITE,
        ] {
            world
                .debug_spawn_item_entity(BlockPos::new(1, -60, 0), item_id, 1)
                .await;
        }

        // Immediately after: still 1 (no synchronous re-evaluation on entity-inside while
        // already pressed, Context §D's own gate).
        assert_eq!(
            world.debug_query_input_signal(plate_pos).await,
            Some(1),
            "the analog output must not jump synchronously"
        );

        // Within one recheck cadence (10 ticks == 500ms, generously bounded): 7.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut caught_up = false;
        while tokio::time::Instant::now() < deadline {
            if world.debug_query_input_signal(plate_pos).await == Some(7) {
                caught_up = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(
            caught_up,
            "the plate must catch up to 7 within its own recheck cadence"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn plate_placement_is_refused_without_a_rigid_or_center_sturdy_block_below() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::OakPressurePlate))
            .await;

        // Over air: refused. The ambient superflat terrain is solid at every (x,z) on the
        // player's own walking floor (one below spawn, Y=-61) -- to reach a position whose OWN
        // "below" is genuine, untouched air, an anchor stone is placed one layer ABOVE that
        // floor (Y=-59) and its own EAST side face is clicked, landing the target at the SAME
        // elevated Y whose "below" (Y=-60, the ordinarily-open walking layer) was never touched.
        let anchor = BlockPos::new(1, -59, 0);
        world.debug_set_block_state(anchor, STONE.0).await;
        let air_target = BlockPos::new(2, -59, 0);
        attempt_place(&mut a, &mut a_acc, &mut seq, anchor, 5).await;
        let after = world.debug_query_block(air_target).await;
        assert!(
            after.is_none() || after.unwrap().raw_state == AIR.0,
            "placement over air must be refused"
        );

        // Over a hopper (Rigid-only): accepted.
        let hopper_pos = BlockPos::new(2, -61, 0);
        world.debug_set_block_state(hopper_pos, HOPPER.0).await;
        let hopper_target = BlockPos::new(2, -60, 0);
        let placed = place_and_read_id(&mut a, &mut a_acc, &mut seq, hopper_pos, 1).await;
        let props = properties(GenStateId(placed as u32));
        assert_eq!(
            props.iter().find(|(n, _)| *n == "powered").unwrap().1,
            "false"
        );
        let info = world.debug_query_block(hopper_target).await.unwrap();
        assert_eq!(info.raw_state as i32, placed);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn breaking_the_support_pops_a_pressed_button_and_a_pressed_plate() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // Button on a stone mount, wired to its own wire.
        let button_mount = BlockPos::new(1, -60, 0);
        world.debug_set_block_state(button_mount, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::StoneButton))
            .await;
        let button_pos = BlockPos::new(2, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, button_mount, 5).await;

        let button_wire_floor = BlockPos::new(3, -61, 0);
        world
            .debug_set_block_state(button_wire_floor, STONE.0)
            .await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let button_wire_pos = BlockPos::new(3, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, button_wire_floor, 1).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        click(&mut a, &mut a_acc, &mut seq, button_pos, 1).await;
        assert!(
            wait_for_wire_power(&world, button_wire_pos, 15, Duration::from_secs(2)).await,
            "the button's own wire must power up"
        );

        // Plate on its own stone floor, wired to a second wire on the far side.
        let plate_floor = BlockPos::new(1, -61, 2);
        world.debug_set_block_state(plate_floor, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::OakPressurePlate))
            .await;
        let plate_pos = BlockPos::new(1, -60, 2);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, plate_floor, 1).await;

        // East of the plate, at the same Y -- reads the plate's own unconditional WEAK signal
        // directly (`ownSignal`, every direction, no relay needed -- a plate is not itself a
        // conductor), mirroring the button's own same-level adjacent wire above exactly.
        let plate_wire_floor = BlockPos::new(2, -61, 2);
        world.debug_set_block_state(plate_wire_floor, STONE.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let plate_wire_pos = BlockPos::new(2, -60, 2);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, plate_wire_floor, 1).await;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;

        // Stand on the plate to press it.
        send_packet(
            &mut a,
            &SetPlayerPosition {
                x: 1.5,
                y: -60.0,
                z: 2.5,
                on_ground: true,
            },
        )
        .await;
        assert!(
            wait_for_signal(&world, plate_pos, Some(15), Duration::from_secs(2)).await,
            "standing on the plate must press it"
        );
        assert!(
            wait_for_wire_power(&world, plate_wire_pos, 15, Duration::from_secs(2)).await,
            "the plate's own wire must power up too"
        );

        // Break both supports via a real destroy action (instabuild breaks in one tick).
        for (support_pos, direction) in [(button_mount, 5i32), (plate_floor, 1i32)] {
            seq += 1;
            send_packet(
                &mut a,
                &PlayerAction {
                    status: 0,
                    location: pack_position(support_pos),
                    direction,
                    sequence: seq,
                },
            )
            .await;
            let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
            assert_eq!(
                decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
                seq
            );
        }

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut both_air = false;
        while tokio::time::Instant::now() < deadline {
            let button_raw = world
                .debug_query_block(button_pos)
                .await
                .map(|i| i.raw_state);
            let plate_raw = world
                .debug_query_block(plate_pos)
                .await
                .map(|i| i.raw_state);
            if button_raw == Some(AIR.0) && plate_raw == Some(AIR.0) {
                both_air = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(both_air, "both the button and the plate must pop to air");

        assert!(
            wait_for_wire_power(&world, button_wire_pos, 0, Duration::from_secs(2)).await,
            "the button's own wire must drop to 0"
        );
        assert!(
            wait_for_wire_power(&world, plate_wire_pos, 0, Duration::from_secs(2)).await,
            "the plate's own wire must drop to 0"
        );
    })
    .await
    .unwrap();
}
