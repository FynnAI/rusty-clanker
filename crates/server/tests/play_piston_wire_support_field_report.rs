//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=yes self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(single instance per test, no ≥3-component chain; the pure-domain support-predicate matrix is covered directly in redstone_wire_piston_support.rs) nondefault-state=yes
//! M3 field-report test-authoring (finding 6, MECH-D84): a horizontal piston's own base is no
//! longer `Full`-sturdy on its top face once it extends (the missing 4/16 slab sits on the
//! facing axis) — redstone wire resting on top of it must pop. Proven end-to-end over a real
//! loopback connection, mirroring `play_redstone_field_report.rs`'s own established helper
//! shape (every helper below is copied from that file; integration tests cannot share code
//! across files in this crate today). Flat world: grass at y=-61, player spawns at y=-60.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, VarLong, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, SectionBlocksUpdate, SetPlayerRotation, UseItemOn, pack_position,
    unpack_position,
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

fn sign_extend(value: i64, bits: u32) -> i64 {
    let sign_bit = 1i64 << (bits - 1);
    if value >= sign_bit {
        value - (1i64 << bits)
    } else {
        value
    }
}

fn unpack_block_in_section(entry: VarLong) -> (u32, u8, u8, u8) {
    let packed = entry.get() as u64;
    let state_id = (packed >> 12) as u32;
    let local_x = ((packed >> 8) & 0xF) as u8;
    let local_z = ((packed >> 4) & 0xF) as u8;
    let local_y = (packed & 0xF) as u8;
    (state_id, local_x, local_z, local_y)
}

/// M3 field-report test-authoring (PLAN-D10, moving_piston placeholder): `collect_block_updates_
/// at`'s own early-exit ("return as soon as every wanted position has SOME value") is now
/// unsafe for a piston's own head cell -- vanilla writes the `moving_piston` placeholder there
/// immediately, at accept time, and only replaces it with the real settled content two ticks
/// later (`crates/mechanics/src/redstone/piston.rs`'s own `write_extend_placeholders` doc
/// comment). The early exit would return the moment the (still-transient) placeholder arrives,
/// never waiting for the real final content that follows it. This variant instead always
/// listens for the ENTIRE `window`, keeping only the LAST value seen per position (a `HashMap`
/// insert already overwrites) -- generous enough for every caller here (2500ms against a
/// handful-of-ticks settle time), just slower than the early-exit original.
async fn collect_settled_block_updates_at(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    wanted: &[BlockPos],
    window: Duration,
) -> std::collections::HashMap<BlockPos, i32> {
    let mut seen = std::collections::HashMap::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return seen;
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
            Err(_) => return seen,
        }
    }
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

async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

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

async fn staged<T>(
    stage: &'static str,
    limit: Duration,
    fut: impl std::future::Future<Output = T>,
) -> T {
    match tokio::time::timeout(limit, fut).await {
        Ok(value) => value,
        Err(_) => panic!("stage {stage:?} timed out after {limit:?}"),
    }
}

/// `nearest_direction6(90.0, 0.0)` resolves to `West` (`look_vector(90, 0) = (1, 0, 0)`,
/// dominant `+x`), so `.opposite()` -- `resolve_orientation`'s own piston rule -- gives
/// `facing = East` (`play_redstone_field_report.rs`'s own identical derivation, cross-checked
/// there against `mining.rs`'s/`piston.rs`'s shared `[north, east, south, west, up, down]`
/// facing-index order).
const YAW_FACING_EAST_PISTON: f32 = 90.0;

#[tokio::test]
async fn a_piston_extending_pops_wire_resting_on_its_own_base_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // B is a bystander watching for the wire's own pop -- proving it reaches a real,
        // entirely passive client, never merely the acting connection's own traffic.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // S: the piston's own placement anchor -- FACING points away from it (East).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s_pos = BlockPos::new(2, -60, 0);
        let s_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        assert_ne!(s_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The piston, FACING=East, placed by clicking S's own EAST face (direction=5).
        rotate(&mut a, &world, uuid_a, YAW_FACING_EAST_PISTON, 0.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        let piston_pos = BlockPos::new(3, -60, 0);
        let head_pos = BlockPos::new(4, -60, 0);
        let piston_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s_pos, 5).await;
        assert_eq!(
            piston_id, 2264,
            "piston facing=east, extended=false -- freshly placed, no signal yet (retracted, \
             registered as a full cube -- a wire may rest on top of it)"
        );
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // A redstone wire directly on top of the RETRACTED piston base -- survives, since a
        // retracted piston base is registered as a full cube.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let wire_pos = BlockPos::new(3, -59, 0);
        let wire_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, piston_pos, 1).await;
        assert_ne!(
            wire_id, 0,
            "the wire must survive placement on a retracted piston base"
        );
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The driver: a floor torch at the piston's own NORTH neighbor (not its push
        // direction, East), standing on its own separate floor support -- mirrors
        // `play_redstone_field_report.rs`'s own identical activation shape.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -61, -1), 1).await;
        assert_eq!(torch_id, 6885, "floor torch -> lit=true");

        // The torch's own placement-time fan-out reaches the piston synchronously; the
        // resulting extend is only QUEUED here, processed at the next real Stage-4 tick --
        // entirely without any further player action. Once the base's own EXTENDED flip
        // lands, its own shape-update fan-out reaches the wire directly above it, which must
        // pop to air within a few ticks (MECH-D84 -- an extended east-facing base's own top
        // face is no longer Full-sturdy).
        let seen = staged(
            "piston extends and the wire above it pops, no further player action",
            Duration::from_secs(5),
            collect_settled_block_updates_at(
                &mut b,
                &mut b_acc,
                &[piston_pos, head_pos, wire_pos],
                Duration::from_millis(2500),
            ),
        )
        .await;
        assert_eq!(
            seen.get(&piston_pos).copied(),
            Some(2258),
            "the piston's own base flips to extended=true (facing=east) -- got {seen:?}"
        );
        assert_eq!(
            seen.get(&head_pos).copied(),
            Some(2275),
            "a plain (non-sticky) piston_head, facing=east, settles two ticks later -- got \
             {seen:?}"
        );
        assert_eq!(
            seen.get(&wire_pos).copied(),
            Some(0),
            "the wire resting on the now-extended base must pop to air -- got {seen:?}"
        );

        let wire_state = world.debug_query_block(wire_pos).await.unwrap();
        assert_eq!(wire_state.raw_state, 0, "wire cell is air server-side too");
    })
    .await
    .unwrap();
}
