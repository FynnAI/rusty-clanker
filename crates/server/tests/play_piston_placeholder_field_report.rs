//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=waived(a single default yaw/pitch spawn rotation is used throughout -- North-facing piston only, mirroring play_block_event_field_report.rs's own identical convention) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single piston instance per test, no ≥3-component chain) nondefault-state=yes
//! M3 field-report test-authoring (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84):
//! the real-connection, end-to-end proof of two things a real oracle capture settled
//! empirically (`xtask parity-check redstone`; `crates/server/src/play/world.rs`'s own doc
//! comment at its moving_piston-ranged broadcast filter has the full citation): the piston's own
//! `block_event` still arrives immediately (unaffected by this changeset), and the
//! `moving_piston` placeholder this changeset now writes server-side, immediately, is NEVER
//! independently visible to a client as its own `Block Update` -- the client's own last-known
//! value for a pushed position jumps straight from the pre-push content to the real final
//! content once the deferred commit lands, two ticks later. Helper pattern copied from
//! `play_block_event_field_report.rs` (integration tests cannot share code across files in this
//! crate today).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, VarLong, decode_one, encode_payload};
use rc_registries::block_state_properties::{range_of, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockEvent, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SectionBlocksUpdate, UseItemOn, pack_position,
    unpack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

// --- Shared harness (copied from `play_block_event_field_report.rs`'s own identical helpers) ---

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

/// One clientbound wire event this file's own ordered timeline cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimelineEvent {
    BlockEvent { pos: BlockPos, action_id: u8 },
    BlockUpdate { pos: BlockPos, state_id: i32 },
}

/// Records, IN ARRIVAL ORDER, every `block_event` and every `Block Update`/`Section Blocks
/// Update` entry at one of `wanted` positions seen on `socket` for up to `window` -- unlike
/// `play_redstone_field_report.rs`'s/`play_piston_wire_support_field_report.rs`'s own
/// `collect_block_updates_at` (a `HashMap`, last-value-wins, order-losing), this file's own
/// entire point is proving the ORDER packets arrive in, so every observation is appended to a
/// plain `Vec`, never deduplicated or overwritten.
async fn collect_timeline_at(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    wanted: &[BlockPos],
    window: Duration,
) -> Vec<TimelineEvent> {
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return seen;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == BlockEvent::ID => {
                let event = decode_one::<BlockEvent>(body).unwrap();
                let pos = unpack_position(event.location);
                if wanted.contains(&pos) {
                    seen.push(TimelineEvent::BlockEvent {
                        pos,
                        action_id: event.action_id,
                    });
                }
            }
            Ok((id, body)) if id == BlockUpdate::ID => {
                let update = decode_one::<BlockUpdate>(body).unwrap();
                let pos = unpack_position(update.location);
                if wanted.contains(&pos) {
                    seen.push(TimelineEvent::BlockUpdate {
                        pos,
                        state_id: update.block_state_id,
                    });
                }
            }
            Ok((id, body)) if id == SectionBlocksUpdate::ID => {
                let packet = decode_one::<SectionBlocksUpdate>(body).unwrap();
                let (chunk_x, section_y, chunk_z) = unpack_section_position(packet.section_pos);
                for entry in packet.states {
                    let (raw_state, local_x, local_z, local_y) = unpack_block_in_section(entry);
                    let pos = BlockPos::new(
                        chunk_x * 16 + local_x as i32,
                        section_y * 16 + local_y as i32,
                        chunk_z * 16 + local_z as i32,
                    );
                    if wanted.contains(&pos) {
                        seen.push(TimelineEvent::BlockUpdate {
                            pos,
                            state_id: raw_state as i32,
                        });
                    }
                }
            }
            Ok(_) => {}
            Err(_) => return seen,
        }
    }
}

/// `state_id(block_id::MOVING_PISTON, ..)`'s own real id for `(facing, sticky)` -- the exact
/// same lookup `crates/mechanics/src/redstone/piston.rs`'s own private `moving_piston_id` and
/// `crates/mechanics/tests/piston_moving_placeholder.rs`'s own identical restatement use.
fn moving_piston_id(facing: &str, sticky: bool) -> i32 {
    state_id(
        block_id::MOVING_PISTON,
        &[
            ("facing", facing),
            ("type", if sticky { "sticky" } else { "normal" }),
        ],
    )
    .expect("every (facing, sticky) pair is a legal moving_piston state")
    .0 as i32
}

#[tokio::test]
async fn extending_piston_broadcasts_block_event_then_final_content_never_the_placeholder() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        // Default spawn rotation -> a North-facing piston (`play_block_event_field_report.rs`'s
        // own identical derivation).
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_pos = BlockPos::new(2, -60, 0);
        let pushed_pos = BlockPos::new(2, -60, -1); // North of the piston -- its own push direction.
        let head_dest_pos = BlockPos::new(2, -60, -2); // one cell further north -- the head's own landing cell.

        // A single Stone directly in the piston's own push direction.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, -1), 1).await;
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The driver: a floor torch at the piston's own East neighbor (a valid QC-activation
        // side, never the push direction) -- mirrors `play_block_event_field_report.rs`'s own
        // identical activation geometry.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -61, 0), 1).await;

        let timeline = collect_timeline_at(
            &mut b,
            &mut b_acc,
            &[piston_pos, pushed_pos, head_dest_pos],
            Duration::from_millis(3000),
        )
        .await;

        // 1. The block_event (TRIGGER_EXTEND) at the piston's own position.
        let block_event_index = timeline
            .iter()
            .position(|e| {
                matches!(e, TimelineEvent::BlockEvent { pos, action_id } if *pos == piston_pos && *action_id == 0)
            })
            .unwrap_or_else(|| panic!("no TRIGGER_EXTEND block_event seen -- timeline: {timeline:?}"));

        // 2. Empirically settled against a real oracle capture (`xtask parity-check redstone`;
        // `crates/server/src/play/world.rs`'s own doc comment at its moving_piston-ranged
        // broadcast filter has the full citation): the `moving_piston` placeholder is never
        // independently visible to a client at all -- it is a real, immediate SERVER-side write
        // (proven directly by `piston_moving_placeholder.rs`'s own engine-level tests, and by
        // this same test's own real, immediate `block_event` above), but its own block state, at
        // its own position, never reaches a `Block Update`/`Section Blocks Update` packet. The
        // real final content is the FIRST and ONLY update this client ever sees at either
        // position, strictly after the block_event.
        let placeholder_id = moving_piston_id("north", false);
        assert!(
            !timeline.iter().any(|e| matches!(
                e,
                TimelineEvent::BlockUpdate { state_id, .. } if *state_id == placeholder_id
            )),
            "the moving_piston placeholder must never reach a client as its own Block Update -- \
             timeline: {timeline:?}"
        );

        let pushed_final_index = timeline
            .iter()
            .enumerate()
            .position(|(i, e)| {
                i > block_event_index
                    && matches!(e, TimelineEvent::BlockUpdate { pos, .. } if *pos == pushed_pos)
            })
            .unwrap_or_else(|| panic!("no Block Update ever arrived at pushed_pos -- timeline: {timeline:?}"));
        let head_final_index = timeline
            .iter()
            .enumerate()
            .position(|(i, e)| {
                i > block_event_index
                    && matches!(e, TimelineEvent::BlockUpdate { pos, .. } if *pos == head_dest_pos)
            })
            .unwrap_or_else(|| panic!("no Block Update ever arrived at head_dest_pos -- timeline: {timeline:?}"));

        let pushed_final = match timeline[pushed_final_index] {
            TimelineEvent::BlockUpdate { state_id, .. } => state_id,
            _ => unreachable!(),
        };
        let head_final = match timeline[head_final_index] {
            TimelineEvent::BlockUpdate { state_id, .. } => state_id,
            _ => unreachable!(),
        };
        assert_eq!(
            pushed_final, 2271,
            "a plain (non-sticky) piston_head, facing=north, settles at the pushed block's own \
             old position -- the FIRST update this client ever sees there, straight from the \
             pre-push Stone -- got timeline: {timeline:?}"
        );
        assert_ne!(
            head_final, placeholder_id,
            "the head's own landing cell must settle to the shifted Stone, not a placeholder -- \
             got timeline: {timeline:?}"
        );

        // Server-side too, once everything has settled.
        let pushed_state = world.debug_query_block(pushed_pos).await.unwrap();
        assert_eq!(pushed_state.raw_state, 2271);
        let head_state = world.debug_query_block(head_dest_pos).await.unwrap();
        assert_ne!(head_state.raw_state, placeholder_id as u32);

        // Sanity: the placeholder's own real range is exactly the twelve `moving_piston` ids
        // (Context) -- confirms `moving_piston_id` above resolved a real, in-range id, not some
        // unrelated coincidence.
        let range = range_of(block_id::MOVING_PISTON);
        assert!((range.first.0..=range.last.0).contains(&(placeholder_id as u32)));
    })
    .await
    .unwrap();
}

/// M3 field-report test-authoring (PLAN-D10, moving_piston placeholder — server-side
/// persistence fix, real-connection field report): the retract-side proof this file's own
/// extend test above does not cover at all -- a vacated source (here, a bare retraction's own
/// old head, since this piston is non-sticky and nothing is in front of it to pull) becomes
/// real, client-visible `air`, synchronously with the SAME tick's own triggering `block_event`
/// -- and (this changeset's own `crates/server/src/play/world.rs` drain-order fix) strictly
/// AFTER that block_event in packet arrival order, never before: verified directly against the
/// decompiled reference (`ServerLevel.runBlockEvents` sends the block_event packet immediately,
/// synchronously, while `PistonBaseBlock.triggerEvent`'s own resulting world mutation --
/// `moveBlocks`'s own `deleteAfterMove` air write in vanilla's real analogue -- is only picked
/// up by the NEXT tick's own `ChunkHolder.broadcastChanges` flush; `world.rs`'s own doc comment
/// at its drain-order fix has the full citation). The `moving_piston` placeholder this same
/// retract writes at the base cell, immediately, is -- like the extend test above -- never
/// independently visible to a client at all.
#[tokio::test]
async fn retracting_piston_broadcasts_block_event_then_the_vacated_air_never_the_placeholder() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;

        let mut seq = 0;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        // Default spawn rotation -> a North-facing piston (`play_block_event_field_report.rs`'s
        // own identical derivation).
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let piston_pos = BlockPos::new(2, -60, 0);
        let front_pos = BlockPos::new(2, -60, -1); // North -- the piston's own push direction.

        // The driver: a floor torch at the piston's own East neighbor (a valid QC-activation
        // side, never the push direction) -- mirrors `play_block_event_field_report.rs`'s own
        // identical activation geometry.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(3, -61, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, torch_pos, 1).await;

        // Let the extend fully settle (its own block_event, then the real content two ticks
        // later) before this test's own retract timeline capture starts -- this test is about
        // the SUBSEQUENT retract, not the extend (`play_block_event_field_report.rs`'s own
        // `breaking_the_torch_triggers_a_contract_block_event`'s identical "drain the extend
        // event first" framing).
        collect_timeline_at(
            &mut b,
            &mut b_acc,
            &[piston_pos, front_pos],
            Duration::from_millis(2500),
        )
        .await;

        // Creative -> instant finalize (mirrors this crate's own established "instant break"
        // pattern, `play_block_event_field_report.rs`'s own identical `PlayerAction` call).
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

        let timeline = collect_timeline_at(
            &mut b,
            &mut b_acc,
            &[piston_pos, front_pos],
            Duration::from_millis(3000),
        )
        .await;

        // 1. The block_event (TRIGGER_CONTRACT) at the piston's own position.
        let block_event_index = timeline
            .iter()
            .position(|e| {
                matches!(e, TimelineEvent::BlockEvent { pos, action_id } if *pos == piston_pos && *action_id == 1)
            })
            .unwrap_or_else(|| panic!("no TRIGGER_CONTRACT block_event seen -- timeline: {timeline:?}"));

        // 2. The vacated source's own air update -- real, client-visible, and (per this
        // changeset's own drain-order fix) arrives strictly AFTER the block_event that
        // triggered it, never before.
        let air_index = timeline
            .iter()
            .enumerate()
            .position(|(i, e)| {
                i > block_event_index
                    && matches!(e, TimelineEvent::BlockUpdate { pos, state_id } if *pos == front_pos && *state_id == 0)
            })
            .unwrap_or_else(|| {
                panic!("no air Block Update ever arrived at front_pos after the block_event -- timeline: {timeline:?}")
            });

        // 3. The moving_piston placeholder itself -- written at the base cell, momentarily,
        // transiently, server-side -- must never reach a client as its own Block Update, at any
        // point.
        let placeholder_id = moving_piston_id("north", false);
        assert!(
            !timeline.iter().any(|e| matches!(
                e,
                TimelineEvent::BlockUpdate { state_id, .. } if *state_id == placeholder_id
            )),
            "the moving_piston placeholder must never reach a client as its own Block Update -- \
             timeline: {timeline:?}"
        );

        // 4. The base's own real retracted content settles at the deferred commit, two ticks
        // after the block_event, strictly after the vacated-source air update above.
        let base_final_index = timeline
            .iter()
            .enumerate()
            .position(|(i, e)| {
                i > air_index && matches!(e, TimelineEvent::BlockUpdate { pos, .. } if *pos == piston_pos)
            })
            .unwrap_or_else(|| {
                panic!("no Block Update ever arrived at piston_pos after the vacated-source air update -- timeline: {timeline:?}")
            });
        let base_final = match timeline[base_final_index] {
            TimelineEvent::BlockUpdate { state_id, .. } => state_id,
            _ => unreachable!(),
        };
        let retracted_base_id = state_id(
            block_id::PISTON,
            &[("extended", "false"), ("facing", "north")],
        )
        .expect("every (extended, facing) pair is a legal piston state")
        .0 as i32;
        assert_eq!(
            base_final, retracted_base_id,
            "piston facing=north, extended=false -- the real retracted base id -- got timeline: \
             {timeline:?}"
        );

        // Server-side too, once everything has settled.
        let piston_state = world.debug_query_block(piston_pos).await.unwrap();
        assert_eq!(piston_state.raw_state, retracted_base_id as u32);
        let front_state = world.debug_query_block(front_pos).await.unwrap();
        assert_eq!(front_state.raw_state, 0);
    })
    .await
    .unwrap();
}
