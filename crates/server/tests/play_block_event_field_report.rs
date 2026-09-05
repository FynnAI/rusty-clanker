//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=waived(a single default yaw/pitch spawn rotation is used throughout -- North-facing piston only, see redstone_repeater.rs for the pure per-facing sweep) self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(single piston instance per test, no ≥3-component chain) nondefault-state=yes
//! M3 field-report test-authoring (MECH-D83, wave 3 Stream B, task B1): the `block_event`
//! packet, end-to-end over real loopback connections -- mirrors `play_redstone_field_report.
//! rs`'s/`play_hopper_enabled_field_report.rs`'s own established harness. A default (yaw=0,
//! pitch=0) spawn rotation resolves `nearest_direction6(0,0).opposite()` to a North-facing
//! piston for every scenario here (`mining.rs`'s own `look_vector`: yaw=0/pitch=0 ->
//! `(0,0,1)` -> South is the look direction, `.opposite()` -> North is the piston's own
//! `FACING`) -- every trigger below uses the piston's own East neighbor (a real, verified-
//! against-`play_hopper_enabled_field_report.rs` floor-torch-reaches-a-horizontal-neighbor
//! signal path, never the push direction itself).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::block_state_properties::properties;
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::block_states::default_state::OBSIDIAN;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockEvent, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, UseItemOn, pack_position, unpack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (mirrors `play_hopper_enabled_field_report.rs`'s own identical helpers) ---

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

/// Collects every `block_event` packet seen on `socket` for up to `window`.
async fn collect_block_events(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    window: Duration,
) -> Vec<BlockEvent> {
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return seen;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == BlockEvent::ID => {
                seen.push(decode_one::<BlockEvent>(body).unwrap());
            }
            Ok(_) => {}
            Err(_) => return seen,
        }
    }
}

fn facing_of(raw_state_id: i32) -> Direction {
    let props = properties(GenStateId(raw_state_id as u32));
    let facing_str = props.iter().find(|(name, _)| *name == "facing").unwrap().1;
    match facing_str {
        "down" => Direction::Down,
        "up" => Direction::Up,
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        other => panic!("unrecognized facing value {other:?}"),
    }
}

#[tokio::test]
async fn extending_piston_broadcasts_a_block_event_with_facing_and_wire_block_id() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        let piston_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_pos = BlockPos::new(2, -60, 0);
        let facing = facing_of(piston_id);
        assert_eq!(facing, Direction::North, "default spawn rotation math");

        // A floor torch at the piston's own East neighbor -- a horizontal, non-push-direction
        // signal source (`play_hopper_enabled_field_report.rs`'s own identical proven
        // floor-torch-reaches-a-horizontal-neighbor geometry).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -61, 0), 1).await;

        let events = collect_block_events(&mut b, &mut b_acc, Duration::from_secs(2)).await;
        assert_eq!(
            events.len(),
            1,
            "exactly one block_event for the piston's own extend trigger -- got {events:?}"
        );
        let event = events[0];
        assert_eq!(unpack_position(event.location), piston_pos);
        assert_eq!(event.action_id, 0, "TRIGGER_EXTEND");
        assert_eq!(
            event.action_param,
            facing.vanilla_ordinal() as u8,
            "parameter is the vanilla facing ordinal"
        );
        assert_eq!(
            event.block_id, 138,
            "minecraft:piston's own real wire registry id"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn breaking_the_torch_triggers_a_contract_block_event() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_pos = BlockPos::new(2, -60, 0);
        let torch_pos = BlockPos::new(3, -60, 0);

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -61, 0), 1).await;

        // Drain the extend event first -- this test is about the SUBSEQUENT contract.
        collect_block_events(&mut b, &mut b_acc, Duration::from_secs(2)).await;

        // Creative -> instant finalize (mirrors this crate's own established "instant break"
        // pattern).
        seq += 1;
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(torch_pos),
                direction: 1,
                sequence: seq,
            },
        )
        .await;
        recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;

        let events = collect_block_events(&mut b, &mut b_acc, Duration::from_secs(2)).await;
        assert_eq!(events.len(), 1, "got {events:?}");
        assert_eq!(unpack_position(events[0].location), piston_pos);
        assert_eq!(events[0].action_id, 1, "TRIGGER_CONTRACT");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn sticky_piston_with_nothing_in_front_drops_on_retract_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::StickyPiston))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_pos = BlockPos::new(2, -60, 0);
        let torch_pos = BlockPos::new(3, -60, 0);

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -61, 0), 1).await;
        collect_block_events(&mut b, &mut b_acc, Duration::from_secs(2)).await; // drain extend

        seq += 1;
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(torch_pos),
                direction: 1,
                sequence: seq,
            },
        )
        .await;
        recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;

        let events = collect_block_events(&mut b, &mut b_acc, Duration::from_secs(2)).await;
        assert_eq!(events.len(), 1, "got {events:?}");
        assert_eq!(unpack_position(events[0].location), piston_pos);
        assert_eq!(
            events[0].action_id, 2,
            "TRIGGER_DROP -- sticky, nothing in front to pull"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn piston_blocked_by_obsidian_sends_no_block_event_at_all() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_pos = BlockPos::new(2, -60, 0);
        // The push direction is North (`facing_of`'s own doc-comment math) -- obsidian
        // directly ahead of the piston blocks the extend outright.
        world
            .debug_set_block_state(Direction::North.apply(piston_pos), OBSIDIAN.0)
            .await;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -61, 0), 1).await;

        let events = collect_block_events(&mut b, &mut b_acc, Duration::from_secs(2)).await;
        assert!(
            events.is_empty(),
            "a blocked extend must never confirm/broadcast a block_event -- got {events:?}"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_bystander_at_60_blocks_receives_the_event_one_at_70_does_not() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut near, mut near_acc) = spawn_actor(&world, "near", 2).await;
        let (mut far, mut far_acc) = spawn_actor(&world, "far", 3).await;

        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_pos = BlockPos::new(2, -60, 0);

        // 60 blocks away (< 64) and 70 blocks away (> 64), both along the same axis, from
        // the actor's own default spawn (0, -60, 0) -- teleporting `near`/`far` themselves
        // (not the actor) keeps the piston's own placement geometry unaffected.
        world
            .debug_teleport_player(
                2,
                [
                    piston_pos.x as f64 + 60.0,
                    piston_pos.y as f64,
                    piston_pos.z as f64,
                ],
            )
            .await;
        world
            .debug_teleport_player(
                3,
                [
                    piston_pos.x as f64 + 70.0,
                    piston_pos.y as f64,
                    piston_pos.z as f64,
                ],
            )
            .await;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -61, 0), 1).await;

        let near_events =
            collect_block_events(&mut near, &mut near_acc, Duration::from_secs(2)).await;
        let far_events = collect_block_events(&mut far, &mut far_acc, Duration::from_secs(2)).await;
        assert_eq!(near_events.len(), 1, "60 blocks < 64 -- must receive it");
        assert!(
            far_events.is_empty(),
            "70 blocks > 64 -- must not receive it"
        );
    })
    .await
    .unwrap();
}
