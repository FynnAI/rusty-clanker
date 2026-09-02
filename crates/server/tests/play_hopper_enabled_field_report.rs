//! M3.5-B06 test-authoring (Context §3.2, Acceptance tests §5.3): the hopper `ENABLED`
//! neighbor-changed re-evaluation, driven over a real loopback connection (mirrors
//! `play_redstone_field_report.rs`'s own established harness — no shared `tests/` support
//! module exists in this crate today, so every helper below is duplicated per this crate's own
//! established per-file-duplication convention).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state::HOPPER;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, UseItemOn, pack_position, unpack_position,
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

/// Scans clientbound traffic on `socket` for up to `window`, collecting every `Block Update`
/// whose own `location` matches one of `wanted` (mirrors `play_redstone_field_report.rs`'s own
/// identical helper).
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
            Ok(_) => {}
            Err(_) => break,
        }
    }
    seen
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

#[tokio::test]
async fn torch_next_to_a_running_hopper_disables_it_and_removal_re_enables_it() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // B is a passive bystander -- takes no action of its own, only observes the cascade
        // reach a real client, not merely the server's own internal world state.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // The hopper, placed with no neighbor signal at all -> `HOPPER.0` (enabled=true,
        // facing=down). Click the floor tile directly below with `Face::Up` (direction 1),
        // mirroring `play_block_entity_chunk_list.rs`'s own already-proven
        // `hopper_placed_with_no_neighbor_signal_stays_enabled` geometry. `x=5` -- well clear
        // of `SPAWN_POSITION = (0, -59, 0)`, which the (never-moving) actor's own body
        // occupies for this whole test; a placement landing exactly on the actor's own spawn
        // cell would be rejected as self-obstructed (M3 field-report Defect 1 fix), unrelated
        // to this test's own point.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Hopper))
            .await;
        let hopper_pos = BlockPos::new(5, -59, 0);
        let hopper_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(5, -60, 0), 1).await;
        assert_eq!(
            hopper_id, HOPPER.0 as i32,
            "hopper with no neighbor signal -> enabled=true, facing=down"
        );
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // A floor torch, placed AFTER the hopper (exercising the neighbor-changed cascade,
        // not the placement-time check `mining.rs`'s own `apply_placement_with_redstone`
        // already covers), directly West of the hopper -- click the floor tile there with
        // `Face::Up` (direction 1), landing the torch horizontally adjacent to the hopper.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(4, -59, 0);
        let torch_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(4, -60, 0), 1).await;
        assert_ne!(
            torch_id, 0,
            "the floor torch placement must actually succeed"
        );

        // No further player action after the torch placement -- the hopper's own cascaded
        // disable must reach B's real client as a `Block Update`, not only update the
        // server's internal world state.
        let seen =
            collect_block_updates_at(&mut b, &mut b_acc, &[hopper_pos], Duration::from_secs(2))
                .await;
        assert_eq!(
            seen.get(&hopper_pos).copied(),
            Some(HOPPER.0 as i32 + 5),
            "a lit torch now adjacent to the hopper must disable it (enabled=false, same \
             facing, +5 offset) and that must reach a real bystander client -- got {seen:?}"
        );

        // Break the torch directly (Creative -> instant finalize, mirrors
        // `play_redstone_field_report.rs`'s own established "instant break" pattern) -- the
        // hopper's own neighbor signal drops back to zero, re-enabling it.
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
        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            seq
        );
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let torch_update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(torch_update.location, pack_position(torch_pos));
        assert_eq!(torch_update.block_state_id, 0, "torch -> air");

        let seen =
            collect_block_updates_at(&mut b, &mut b_acc, &[hopper_pos], Duration::from_secs(2))
                .await;
        assert_eq!(
            seen.get(&hopper_pos).copied(),
            Some(HOPPER.0 as i32),
            "removing the torch must re-enable the hopper (enabled=true) and that must reach \
             a real bystander client -- got {seen:?}"
        );
    })
    .await
    .unwrap();
}
