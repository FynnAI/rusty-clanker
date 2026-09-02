//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit) orientations=waived(single canonical value/facing asserted, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=waived(every state asserted is that block kind's own default — no non-default property variant exercised)
//! M3 field-report test-authoring (owner's own live manual test against a real vanilla
//! client: "everything I place becomes stone"): the two serverbound packets a real client's
//! own creative-inventory hotbar interaction sends -- `SetCreativeModeSlot` (dropping an item
//! into a hotbar slot) and `SetCarriedItem` (selecting a hotbar slot) -- were decoded nowhere
//! in `connection.rs`'s inbound dispatch, so the join-time `HeldItem(HeldItemStub::Block(
//! PlaceableBlockKind::Stone))` default never changed for a real client no matter what it
//! actually held. Drives both real packets over a real loopback connection, exactly like a
//! real client would (drop redstone-wire dust into hotbar slot 0, select it), then places --
//! asserting the resulting `BlockUpdate` actually carries a redstone-wire state id, not
//! stone. A second case switches back to a hotbar slot holding stone and re-places, proving
//! the tracking is live (not "first real item wins forever").
//!
//! Harness mirrors `play_block_break_place_full.rs`'s own shape
//! (`placement_selects_the_held_items_own_block_and_orientation`'s own placement geometry:
//! `UseItemOn` clicking the `Up` face of `(1, -60, 0)` -- the grass column directly beside
//! spawn -- targets `(1, -59, 0)`, matching this file's own assertions below).

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rc_registries::generated_v776::registries::item;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, CreativeSlotItem,
    KeepAliveClientbound, KeepAliveServerbound, SetCarriedItem, SetCreativeModeSlot, UseItemOn,
    pack_position,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

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

/// No direct hook to poll "the held-item update has actually landed in `region.world`" from a
/// test (`world.rs`'s own `debug_held_item_tx` drain has no query counterpart, unlike e.g.
/// `player_sessions()` for rotation) -- a short fixed grace covers the tick this needs to
/// land, the same established idiom `play_sneak_reach.rs`'s own `PlayerInput`/shift-state wait
/// already uses for an identical "no poll hook" gap ("a short fixed grace covers the tick this
/// needs to land, mirroring the paritybot bot's own `AIM_SETTLE_TICKS` precedent").
async fn settle() {
    tokio::time::sleep(Duration::from_millis(150)).await;
}

async fn place_up_from(a: &mut TcpStream, a_acc: &mut BytesMut, base: BlockPos, sequence: i32) {
    send_packet(
        a,
        &UseItemOn {
            hand: 0,
            location: pack_position(base),
            direction: 1, // Face::Up
            cursor_x: 0.5,
            cursor_y: 1.0,
            cursor_z: 0.5,
            inside_block: false,
            hits_world_border: false,
            sequence,
        },
    )
    .await;
    let body = recv_packet_of_type(a, a_acc, AcknowledgeBlockChange::ID).await;
    assert_eq!(
        decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
        sequence
    );
}

#[tokio::test]
async fn selecting_a_redstone_wire_hotbar_slot_places_wire_not_stone() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // A real client's own "drag redstone dust into hotbar slot 0, then select it" wire
        // sequence: `SetCreativeModeSlot` addresses the full inventory container (slot 36 ==
        // hotbar index 0, `InventoryMenu.USE_ROW_SLOT_START`), `SetCarriedItem` addresses the
        // bare hotbar index (0..9) directly.
        send_packet(
            &mut a,
            &SetCreativeModeSlot {
                slot: 36,
                item: CreativeSlotItem {
                    item_id: Some(item::REDSTONE.0 as i32),
                },
            },
        )
        .await;
        send_packet(&mut a, &SetCarriedItem { slot: 0 }).await;
        settle().await;

        // Click the Up face of the grass column at (1, -60, 0) -- targets (1, -59, 0), never
        // A's own body at spawn (0, -59, 0), matching `play_block_break_place_full.rs`'s own
        // established placement geometry for this exact hardcoded world.
        place_up_from(&mut a, &mut a_acc, BlockPos::new(1, -60, 0), 2).await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(1, -59, 0)));
        // M3 field-report test-authoring fix (Root Cause 2, wire connection resolution): an
        // isolated placed wire no longer stays at `blocks::REDSTONE_WIRE.0`'s own raw
        // all-disconnected default (`5171`) -- `apply_placement` now resolves its real
        // placement-time connection shape immediately, and a wire with no neighbors at all
        // settles to the vanilla "plus" (every side auto-promoted to `Side`, `wire.rs`'s own
        // post-processing pass): `east=side(1), north=side(1), power=0, south=side(1),
        // west=side(1)` -> `4011 + 1*432 + 1*144 + 0*9 + 1*3 + 1*1 = 4591`
        // (`mining_block_state_ids.rs`'s/`play_redstone_field_report.rs`'s own identical
        // literal, cross-verified there).
        assert_eq!(update.block_state_id, 4591);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn switching_back_to_a_stone_slot_places_stone_again() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // Hotbar slot 0: redstone wire; hotbar slot 1: stone.
        send_packet(
            &mut a,
            &SetCreativeModeSlot {
                slot: 36,
                item: CreativeSlotItem {
                    item_id: Some(item::REDSTONE.0 as i32),
                },
            },
        )
        .await;
        send_packet(
            &mut a,
            &SetCreativeModeSlot {
                slot: 37,
                item: CreativeSlotItem {
                    item_id: Some(item::STONE.0 as i32),
                },
            },
        )
        .await;

        // Select redstone wire first and place it (proves the tracking is live, not "first
        // real item wins forever") at (1, -59, 0)...
        send_packet(&mut a, &SetCarriedItem { slot: 0 }).await;
        settle().await;
        place_up_from(&mut a, &mut a_acc, BlockPos::new(1, -60, 0), 2).await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        // M3 field-report test-authoring fix: this wire is also isolated (nothing else placed
        // yet) -- same `4591` "plus" resolution as the previous test's own identical citation.
        assert_eq!(
            decode_one::<BlockUpdate>(body).unwrap().block_state_id,
            4591
        );

        // ...then switch to the stone slot and place again at a fresh column -- must be
        // stone, not a stale carried-over redstone-wire selection.
        send_packet(&mut a, &SetCarriedItem { slot: 1 }).await;
        settle().await;
        place_up_from(&mut a, &mut a_acc, BlockPos::new(2, -60, 0), 3).await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(2, -59, 0)));
        assert_eq!(update.block_state_id, blocks::STONE.0 as i32);
    })
    .await
    .unwrap();
}
