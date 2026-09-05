//! M3 field-report test-authoring (MECH-D78, wave 3 Stream B, task B4): `respond_place`'s own
//! unconditional dual-cell (clicked + placement-direction) resend to the actor, on every
//! outcome alike -- vanilla's own `ServerGamePacketListenerImpl.handleUseItemOn` tail
//! (`this.send(new ClientboundBlockUpdatePacket(level, pos)); this.send(new
//! ClientboundBlockUpdatePacket(level, pos.relative(direction)));`, sent unconditionally,
//! ASSET-D18(f) reference, decompiled-source-verified). No case-matrix header:
//! `play_use_resend_field_report` does not match any of
//! `xtask::case_matrix::MECHANIC_TEST_PREFIXES`.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state::STONE;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, UseItemOn, pack_position, unpack_position,
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

/// Collects every `Block Update` seen on the actor's own connection for up to `window`,
/// keyed by position (a real click can touch the same position more than once across the
/// broadcast + resend paths -- callers assert on the FULL ordered list when duplicates
/// matter, on this map otherwise).
async fn collect_actor_block_updates(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    window: Duration,
) -> Vec<(BlockPos, i32)> {
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return seen;
        }
        match tokio::time::timeout(remaining, recv_clientbound(socket, accumulator)).await {
            Ok((id, body)) if id == BlockUpdate::ID => {
                let update = decode_one::<BlockUpdate>(body).unwrap();
                seen.push((unpack_position(update.location), update.block_state_id));
            }
            Ok(_) => {}
            Err(_) => return seen,
        }
    }
}

async fn use_item_on(
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

#[tokio::test]
async fn a_successful_placement_sends_the_actor_two_block_updates_besides_the_broadcast() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let clicked = BlockPos::new(2, -61, 0);
        let target = BlockPos::new(2, -60, 0);
        use_item_on(&mut a, &mut a_acc, &mut seq, clicked, 1).await;

        let updates =
            collect_actor_block_updates(&mut a, &mut a_acc, Duration::from_millis(500)).await;
        // MECH-D78: the broadcast (`target`, once) PLUS the unconditional dual-cell resend
        // (`clicked`, `target` again) -- three `Block Update`s total on the actor's own
        // connection for one successful placement.
        assert_eq!(
            updates,
            vec![
                (target, STONE.0 as i32),
                (
                    clicked,
                    world.debug_query_block(clicked).await.unwrap().raw_state as i32
                ),
                (target, STONE.0 as i32),
            ],
            "broadcast + dual-cell resend, in that order"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_rejected_placement_sends_both_cells_to_the_actor() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // Place stone at `target` FIRST, then try to place again at the exact same target
        // -> `RejectReason::TargetNotAir`.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let clicked = BlockPos::new(2, -61, 0);
        let target = BlockPos::new(2, -60, 0);
        use_item_on(&mut a, &mut a_acc, &mut seq, clicked, 1).await;
        collect_actor_block_updates(&mut a, &mut a_acc, Duration::from_millis(300)).await; // drain the first placement's own traffic

        // `clicked` is the pre-existing FLOOR tile (whatever this world's own default
        // terrain is there) -- never itself replaced by a placement, unlike `target`.
        let clicked_state = world.debug_query_block(clicked).await.unwrap().raw_state as i32;
        use_item_on(&mut a, &mut a_acc, &mut seq, clicked, 1).await;
        let updates =
            collect_actor_block_updates(&mut a, &mut a_acc, Duration::from_millis(500)).await;
        assert_eq!(
            updates,
            vec![(clicked, clicked_state), (target, STONE.0 as i32)],
            "no broadcast (nothing changed) -- exactly the dual-cell resend"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn an_empty_hand_click_on_stone_sends_both_cells_to_the_actor() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let clicked = BlockPos::new(2, -61, 0);
        let target = BlockPos::new(2, -60, 0);
        use_item_on(&mut a, &mut a_acc, &mut seq, clicked, 1).await;
        collect_actor_block_updates(&mut a, &mut a_acc, Duration::from_millis(300)).await; // drain

        let clicked_state = world.debug_query_block(clicked).await.unwrap().raw_state as i32;
        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;
        use_item_on(&mut a, &mut a_acc, &mut seq, clicked, 1).await;

        let updates =
            collect_actor_block_updates(&mut a, &mut a_acc, Duration::from_millis(500)).await;
        // `NothingToPlace` (empty hand): today's code sends nothing at all here -- red.
        assert_eq!(
            updates,
            vec![(clicked, clicked_state), (target, STONE.0 as i32)],
            "MECH-D78's own unconditional resend must fire even for NothingToPlace"
        );
    })
    .await
    .unwrap();
}
