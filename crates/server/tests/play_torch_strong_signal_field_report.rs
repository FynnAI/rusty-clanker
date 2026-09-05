//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=yes self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(single instance per test, no ≥3-component chain; the pure-domain wall-vs-floor-torch axis comparison is covered directly in redstone_torch_strong_signal.rs) nondefault-state=yes
//! M3 field-report test-authoring (finding 4, real-connection half): a wall torch's own
//! direct/strong signal must fire straight up (the vanilla-hard-coded axis, `redstone_torch_
//! strong_signal.rs`'s own doc comment has the full citation), not sideways toward its own
//! facing — proven end-to-end over a real loopback connection, mirroring `play_redstone_
//! field_report.rs`'s own established helper shape (every helper below is copied from that
//! file; integration tests cannot share code across files in this crate today). Flat world:
//! grass at y=-61, player spawns at y=-60.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, UseItemOn, pack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (copied from `play_redstone_field_report.rs`'s own identical helpers) ---

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
/// `block_state_id` (the response `respond_place` sends for the position the player directly
/// acted on).
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

/// `WIRE_BASE`/stride arithmetic (`play_redstone_field_report.rs`'s own top-of-file doc
/// comment has the full citation): `east*432 + north*144 + power*9 + south*3 + west*1`, base
/// 4011. An isolated wire with a single vertical power source below it (no same-layer
/// neighbors) auto-promotes every side to `Side` (`side=1` on all four) — matches
/// `play_redstone_field_report.rs`'s own `isolated_redstone_wire_gets_the_connected_plus_
/// shape_not_a_bare_dot` test's `power=0` id (4591) exactly, offset here by `power*9`.
fn isolated_wire_id_for_power(power: i32) -> i32 {
    4011 + 1 * 432 + 1 * 144 + power * 9 + 1 * 3 + 1 * 1
}

#[tokio::test]
async fn a_wall_torch_powers_a_conductor_directly_above_it_to_full_strength_orientation_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // S: the wall torch's own mount block.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s_pos = BlockPos::new(1, -60, 0);
        let s_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -61, 0), 1).await;
        assert_ne!(s_id, 0);

        // A wall torch on S's own EAST face (direction=5) -> lands at (2,-60,0), facing=east,
        // always lit at placement.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(2, -60, 0);
        let torch_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s_pos, 5).await;
        assert_eq!(torch_id, 6893, "wall torch facing=east, lit=true");

        // S2: a Stone directly ABOVE the torch -- a conductor, so it relays whatever the torch
        // emits straight up into it onward to anything touching S2's own other faces,
        // including straight up (finding 4's own real-client symptom: "torch under a solid
        // block does not power dust on top").
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s2_pos = BlockPos::new(2, -59, 0);
        let s2_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, torch_pos, 1).await;
        assert_ne!(s2_id, 0);

        // A redstone wire directly ON TOP of S2 -- reads S2's own aggregated direct signal,
        // which (post-fix) must be the torch's own strong signal, `15`, straight up through
        // the conductor.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let wire_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s2_pos, 1).await;
        assert_eq!(
            wire_id,
            isolated_wire_id_for_power(15),
            "a wall torch's own strong signal must reach straight up through the conductor \
             directly above it, exactly like a floor torch's -- finding 4, the wall-torch \
             strong-signal-axis defect"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_floor_torch_powers_a_conductor_directly_above_it_to_full_strength_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // S: the floor torch's own support block.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s_pos = BlockPos::new(1, -60, 0);
        let s_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -61, 0), 1).await;
        assert_ne!(s_id, 0);

        // A floor torch on top of S (direction=1, up face).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(1, -59, 0);
        let torch_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s_pos, 1).await;
        assert_eq!(torch_id, 6885, "floor torch -> lit=true");

        // S2: a Stone directly above the torch.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s2_pos = BlockPos::new(1, -58, 0);
        let s2_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, torch_pos, 1).await;
        assert_ne!(s2_id, 0);

        // A redstone wire on top of S2 -- already correct before this fix (a floor torch's
        // own attachment-derived axis happened to already be `Up`), kept green as a
        // regression lock.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let wire_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s2_pos, 1).await;
        assert_eq!(
            wire_id,
            isolated_wire_id_for_power(15),
            "a floor torch's own strong signal reaches straight up through the conductor \
             directly above it"
        );
    })
    .await
    .unwrap();
}
