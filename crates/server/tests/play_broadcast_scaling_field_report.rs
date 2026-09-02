//! M3.5-B06 test-authoring (Context §3.1, Acceptance tests §5.1): `SectionBlocksUpdate` +
//! per-viewer view-distance filtering, driven over real loopback connections (mirrors
//! `play_redstone_field_report.rs`'s own established harness — no shared `tests/` support
//! module exists in this crate today, so every helper below is duplicated per this crate's own
//! established per-file-duplication convention).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, VarLong, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SectionBlocksUpdate, SetPlayerPositionAndRotation,
    UseItemOn, pack_position, unpack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (mirrors `play_redstone_field_report.rs`'s own identical helpers) ---

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

/// Places `held` at `(location, direction)` and returns the direct `Block Update`'s own
/// `block_state_id` (mirrors `play_redstone_field_report.rs`'s own identical helper).
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

/// This file's own local inverse of `packets::pack_section_position` (no shared
/// `unpack_section_position` exists in production, mirroring `unpack_position`'s own already-
/// established shape for the sibling packing function).
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

/// This file's own local inverse of `packets::pack_block_in_section`.
fn unpack_block_in_section(entry: VarLong) -> (u32, u8, u8, u8) {
    let packed = entry.get() as u64;
    let state_id = (packed >> 12) as u32;
    let local_x = ((packed >> 8) & 0xF) as u8;
    let local_z = ((packed >> 4) & 0xF) as u8;
    let local_y = (packed & 0xF) as u8;
    (state_id, local_x, local_z, local_y)
}

/// Scans `socket`'s own clientbound traffic for up to `window`, collecting every
/// `Block Update`/`Section Blocks Update` entry whose own absolute position matches `wanted`
/// -- the per-viewer filtering test's own "must receive absolutely nothing referencing this
/// position, in either wire shape" assertion.
async fn collect_block_or_section_updates_at(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    wanted: BlockPos,
    window: Duration,
) -> Vec<i32> {
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == BlockUpdate::ID => {
                let update = decode_one::<BlockUpdate>(body).unwrap();
                if unpack_position(update.location) == wanted {
                    seen.push(update.block_state_id);
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
                    if pos == wanted {
                        seen.push(state_id as i32);
                    }
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    seen
}

/// Scans `socket`'s own clientbound traffic for up to `window`, collecting every
/// `Section Blocks Update` packet seen in full (undecoded) -- the coalescing test's own
/// "exactly one such packet, not two separate `Block Update`s" assertion.
async fn collect_section_blocks_updates(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    window: Duration,
) -> Vec<SectionBlocksUpdate> {
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == SectionBlocksUpdate::ID => {
                seen.push(decode_one::<SectionBlocksUpdate>(body).unwrap());
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    seen
}

#[tokio::test]
async fn bystander_outside_view_distance_receives_nothing() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        // B never moves -- stays at `SPAWN_POSITION = (0, -59, 0)`, so its own `sent_chunks`
        // never grows past the joining grid's own `PLACEHOLDER_RADIUS_CHUNKS`-chunk radius
        // around spawn's chunk `(0, 0)`.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let a_uuid_raw: u128 = 1;
        let a_uuid = uuid::Uuid::from_u128(a_uuid_raw);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", a_uuid_raw).await;
        let sessions = world.player_sessions();

        // A walks from `(0, -59, 0)` to `(112, -59, 0)` in 14 legal 8-block steps (each step's
        // own squared length is 64, comfortably under `evaluate_movement`'s own speed-check
        // budget) -- `112 >> 4 = 7` chunks out, well past the 5-chunk joining radius around
        // B's own spawn chunk `(0, 0)`.
        const STEPS: i32 = 14;
        const STEP_LEN: f64 = 8.0;
        for step in 1..=STEPS {
            let x = STEP_LEN * step as f64;
            send_packet(
                &mut a,
                &SetPlayerPositionAndRotation {
                    x,
                    y: -59.0,
                    z: 0.0,
                    yaw: 0.0,
                    pitch: 0.0,
                    on_ground: true,
                },
            )
            .await;
            wait_until(|| {
                sessions.with_record_mut(a_uuid, |r| r.data.pos) == Some([x, -59.0, 0.0])
            })
            .await;
        }
        drain_traffic_for(&mut a, &mut a_acc, Duration::from_millis(200)).await;
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(200)).await;

        // A places one stone block two cells directly below their own feet (`UseItemOn` on
        // the floor block immediately underfoot, `Face::Down` = direction 0, well within
        // reach) -- clicking `Face::Up` on that same block would instead target A's own feet
        // cell, which `is_placement_obstructed` already correctly rejects (the acting
        // player's own body is never excluded, M3 field-report Defect 1 fix) -- irrelevant to
        // this test's own point (view-distance filtering, not placement legality), so this
        // test avoids that self-obstruction entirely instead of exercising it.
        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let below_feet = BlockPos::new(112, -60, 0);
        let placed_pos = BlockPos::new(112, -61, 0);
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, below_feet, 0).await;
        assert_ne!(id, 0, "the stone placement must actually succeed");

        // B, still sitting at spawn, must receive absolutely nothing referencing `placed_pos`
        // -- neither a `Block Update` nor a `Section Blocks Update` -- across a full 2-second
        // window.
        let seen = collect_block_or_section_updates_at(
            &mut b,
            &mut b_acc,
            placed_pos,
            Duration::from_secs(2),
        )
        .await;
        assert!(
            seen.is_empty(),
            "a bystander outside view distance must receive nothing for a position it was \
             never sent the chunk for -- got {seen:?}"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn two_same_tick_same_section_changes_coalesce_into_one_section_blocks_update() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        // Both A and B join at spawn -- trivially in view of each other, isolating the
        // coalescing behavior under test from the filtering behavior the sibling test covers.
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // The support block, at `(2, -59, 2)` -- click the floor tile directly below with
        // `Face::Up` (direction 1).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let support_pos = BlockPos::new(2, -59, 2);
        let support_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -60, 2), 1).await;
        assert_ne!(support_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // Two wall torches, both attached to the support's own horizontal faces -- North
        // (direction 2, lands at `(2, -59, 1)`) and East (direction 5, lands at `(3, -59,
        // 2)`) -- both within section `(chunk_x=0, chunk_z=0, section_y=(-59)>>4=-4)`.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_north_pos = BlockPos::new(2, -59, 1);
        let torch_north_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, support_pos, 2).await;
        assert_ne!(torch_north_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        let torch_east_pos = BlockPos::new(3, -59, 2);
        let torch_east_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, support_pos, 5).await;
        assert_ne!(torch_east_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // Break the support (Creative -> instant finalize, mirrors `play_redstone_field_
        // report.rs`'s own established "instant break" pattern): both wall torches lose
        // their own support in the SAME synchronous cascade, same tick, same section.
        seq += 1;
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(support_pos),
                direction: 1,
                sequence: seq,
            },
        )
        .await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            seq
        );
        // A's own direct response for the position it acted on: a plain `Block Update` for
        // the support itself -- out of this test's own scope (a single-change direct
        // response, not the cascaded pair).
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let support_update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(support_update.location, pack_position(support_pos));
        assert_eq!(support_update.block_state_id, 0, "support block -> air");

        // B's own traffic: exactly one `Section Blocks Update`, not two `Block Update`s, for
        // the two cascaded torch pops.
        let section_updates =
            collect_section_blocks_updates(&mut b, &mut b_acc, Duration::from_secs(2)).await;
        assert_eq!(
            section_updates.len(),
            1,
            "exactly one Section Blocks Update must coalesce the two same-tick, same-section \
             torch pops -- got {section_updates:?}"
        );
        let packet = &section_updates[0];
        let (chunk_x, section_y, chunk_z) = unpack_section_position(packet.section_pos);
        assert_eq!(
            (chunk_x, section_y, chunk_z),
            (0, -4, 0),
            "section_pos must decode to (chunk_x=0, section_y=-4, chunk_z=0)"
        );
        assert_eq!(packet.states.len(), 2, "exactly two coalesced changes");
        let mut decoded: Vec<(BlockPos, u32)> = packet
            .states
            .iter()
            .map(|&entry| {
                let (state_id, local_x, local_z, local_y) = unpack_block_in_section(entry);
                (
                    BlockPos::new(
                        chunk_x * 16 + local_x as i32,
                        section_y * 16 + local_y as i32,
                        chunk_z * 16 + local_z as i32,
                    ),
                    state_id,
                )
            })
            .collect();
        decoded.sort_by_key(|(pos, _)| (pos.x, pos.z));
        assert_eq!(
            decoded,
            vec![(torch_north_pos, 0), (torch_east_pos, 0)],
            "both torch positions decode to AIR (state id 0) at their own local coordinates"
        );
    })
    .await
    .unwrap();
}
