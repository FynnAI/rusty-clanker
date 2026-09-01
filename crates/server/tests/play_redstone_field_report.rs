//! M3 field-report test-authoring: the owner's own tonight-manual-test redstone scenarios,
//! end-to-end over real loopback connections (mirrors `play_block_break_place_full.rs`'s own
//! established shape) -- "redstone wire never connects to neighbors", "torches never power
//! anything and don't pop when their support is broken." Every literal id below was hand-derived
//! from `crates/mechanics/src/redstone/wire.rs`'s own documented `WIRE_BASE`/stride arithmetic
//! (`east*432 + north*144 + power*9 + south*3 + west*1`, base 4011, `up`=0/`side`=1/`none`=2 per
//! side) and cross-checked by decoding each literal back into its own five digits before use.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
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

// --- Shared harness (mirrors `play_block_break_place_full.rs`'s own identical helpers) ---

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
/// whose own `location` matches one of `wanted` -- used to prove a CASCADED change (a position
/// other than the one the player directly acted on) actually reaches a real client, not merely
/// the server's own internal world state. Returns once every wanted position has been seen, or
/// once `window` elapses (whichever first) -- the caller's own `assert_eq!` on the returned
/// map's length is what actually fails a genuine miss.
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

/// Reads and discards every clientbound packet arriving on `socket` for `window`, auto-
/// answering keep-alives along the way -- used to fully settle a bystander's own traffic
/// between two placements. Not a "drain exactly one packet" step: `world.rs`'s own
/// `broadcast_cascaded_changes` (this file's own top-of-file doc comment) harmlessly
/// re-broadcasts the directly-acted-upon position a second time alongside `respond_place`'s own
/// direct broadcast (a real, documented, intentionally-accepted duplicate -- never incorrect
/// for a real client, which treats a repeated identical `Block Update` as a no-op), so a fixed
/// "read exactly one" step can leave a second, stale-looking copy sitting unread in the
/// accumulator to be misread as a later genuine cascade by a subsequent `collect_block_updates_
/// at` scan -- this file's own first draft hit exactly that.
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

#[tokio::test]
async fn isolated_redstone_wire_gets_the_connected_plus_shape_not_a_bare_dot() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let mut seq = 0;

        // A wire with no neighbors at all still resolves its own real placement-time
        // connection shape (`WireBehavior::on_shape_update`'s own post-processing pass): every
        // side starts `None`, and BOTH axes being fully unconnected auto-promotes every side to
        // `Side` -- vanilla's real "plus" rendering for an isolated wire, never left at the raw
        // all-`None` default this project's own placeholder table used to leave it at forever
        // (Root Cause 2, "wire placement must run the vanilla placement-time connection
        // resolution"). `east=side(1), north=side(1), power=0, south=side(1), west=side(1)`:
        // `4011 + 1*432 + 1*144 + 0*9 + 1*3 + 1*1 = 4591`.
        let id = place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -60, 0), 1).await;
        assert_eq!(
            id, 4591,
            "isolated wire -> all four sides Side (the vanilla \"plus\")"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn two_adjacent_wires_connect_to_each_other_on_both_sides() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // B is a bystander whose own traffic proves the SECOND placement's cascade -- wire A's
        // own reconnection toward the newly-placed wire B -- actually reaches a real client,
        // not merely the server's own internal world state (Root Cause 3, the cascade-broadcast
        // gap `world.rs`'s own `broadcast_cascaded_changes` closes).
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let mut seq = 0;

        let wire_a_pos = BlockPos::new(1, -59, 0);
        let wire_b_pos = BlockPos::new(2, -59, 0);

        // Wire A, placed alone -> the isolated "plus" shape (previous test's own 4591).
        let id_a_isolated =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -60, 0), 1).await;
        assert_eq!(id_a_isolated, 4591);
        // Drain B's own copy of that same broadcast so it does not interfere with this test's
        // own later scan.
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // Wire B, placed directly EAST of A -> both wires collapse onto a single straight
        // East-West line (`wire.rs`'s own doc comment: "a lone connection on one axis... renders
        // the whole tile as a straight line through it"). B's own connections: `west=side(1)`
        // (A, a signal source, connects from any direction), `east=side(1)` (auto-promoted: the
        // perpendicular north/south axis is fully disconnected), `north=none(2)`, `south=
        // none(2)`, `power=0` (A itself carries no power yet):
        // `4011 + 1*432 + 2*144 + 0*9 + 2*3 + 1*1 = 4738`.
        let id_b =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -60, 0), 1).await;
        assert_eq!(
            id_b, 4738,
            "wire B, placed beside A -> connects West toward A, auto-extends East"
        );

        // A's own cascaded reconnection (symmetric to B's: `east=side` toward B, `west=none`
        // auto-promoted to `side` since north/south are both empty) settles to the IDENTICAL id
        // (4738) -- and, this test's own real point, actually reaches B's client as a real
        // `Block Update` packet for A's own position, not only the server's internal state.
        let seen = collect_block_updates_at(
            &mut b,
            &mut b_acc,
            &[wire_a_pos],
            Duration::from_millis(2000),
        )
        .await;
        assert_eq!(
            seen.get(&wire_a_pos).copied(),
            Some(4738),
            "wire A's own cascaded reconnection toward B must reach a real client as a Block \
             Update, not only update the server's internal world state -- got {seen:?}"
        );

        // The server's own internal state agrees (`debug_query_block`, `world.rs`'s own
        // diagnostic-only introspection hook).
        let a_state = world.debug_query_block(wire_a_pos).await.unwrap();
        assert_eq!(a_state.raw_state, 4738);
        let _ = wire_b_pos; // named for readability above; asserted via `id_b` already.
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_lit_wall_torch_powers_an_adjacent_wire_to_full_strength() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        // A reference Stone at (1, -59, 0).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let stone_pos = BlockPos::new(1, -59, 0);
        let stone_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -60, 0), 1).await;
        assert_ne!(stone_id, 0);

        // A redstone torch on the stone's own EAST face -> wall torch at (2, -59, 0), FACING =
        // East (points away from the wall, into the room), always lit at placement.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(2, -59, 0);
        let torch_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, stone_pos, 5).await;
        assert_eq!(torch_id, 6893, "wall torch facing=east, lit=true");

        // A redstone wire directly SOUTH of the torch (its own solid floor support is ordinary
        // superflat terrain, no extra stone needed) -- adjacent to the torch, same height.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneWire))
            .await;
        let wire_pos = BlockPos::new(2, -59, 1);
        let wire_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -60, 1), 1).await;

        // The torch is a lit, non-conductor signal source: `weak_signal_toward` reads `15`
        // toward any direction other than its own input side (`West`, back into the wall) --
        // `South` (toward the wire) qualifies, so the wire's own `block_signal` short-circuits
        // straight to `15` (`WireBehavior::compute_power`'s own `if block_signal == 15` fast
        // path). Connections: `north=side(1)` (the torch), `south=side(1)` (auto-promoted, the
        // perpendicular east/west axis is fully empty), `east=none(2)`, `west=none(2)`,
        // `power=15`: `4011 + 2*432 + 1*144 + 15*9 + 1*3 + 2*1 = 5159`.
        assert_eq!(
            wire_id, 5159,
            "wire beside a lit torch -> connects toward it AND reads power=15 immediately, no \
             separate action needed to \"kick\" the recompute"
        );
        let _ = wire_pos;
        let _ = torch_pos;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_floor_torch_pops_when_its_own_support_block_is_broken() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // B is a bystander watching for the torch's own cascaded pop-to-air, exactly mirroring
        // the wire-reconnection test's own real-client proof above.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // A reference Stone at (1, -59, 0), used purely as the torch's own removable support --
        // NOT the world's own natural superflat floor, so breaking it is a clean, isolated
        // "support disappears" event this test fully controls.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let support_pos = BlockPos::new(1, -59, 0);
        let stone_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -60, 0), 1).await;
        assert_ne!(stone_id, 0);
        // Drain B's own copy of the stone's own placement broadcast.
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // A floor torch on top of the stone: `Face::Up` clicked on the stone itself.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(1, -58, 0);
        let torch_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, support_pos, 1).await;
        assert_eq!(torch_id, 6885, "floor torch -> lit=true");
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // Break the stone (Creative -> instant finalize, `mining::finalize_break`'s own
        // synchronous `settle_neighbor_updates` call already carries the torch's own
        // `TorchBehavior::on_shape_update` support-loss pop within the SAME synchronous
        // response -- no separate tick wait is needed, unlike the delayed-destroy survival
        // path).
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
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let support_update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(support_update.location, pack_position(support_pos));
        assert_eq!(support_update.block_state_id, 0, "support block -> air");

        // The torch's own cascaded pop -- a DIFFERENT position than the one the player directly
        // broke -- must reach B's real client as its own `Block Update`, not merely leave the
        // server's internal world state correct while every client keeps rendering a torch that
        // no longer has anything to stand on.
        let seen = collect_block_updates_at(
            &mut b,
            &mut b_acc,
            &[torch_pos],
            Duration::from_millis(2000),
        )
        .await;
        assert_eq!(
            seen.get(&torch_pos).copied(),
            Some(0),
            "the torch's own support-loss pop must reach a real client as a Block Update -- \
             got {seen:?}"
        );

        let torch_state = world.debug_query_block(torch_pos).await.unwrap();
        assert_eq!(
            torch_state.raw_state, 0,
            "torch cell is air server-side too"
        );
    })
    .await
    .unwrap();
}
