//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=yes self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(single instance in this file, no ≥3-component chain; max two components chained over a real connection; ≥3 pure-domain chains covered in redstone_wire.rs/redstone_repeater.rs) nondefault-state=yes
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
    KeepAliveServerbound, PlayerAction, SetPlayerRotation, UseItemOn, pack_position,
    unpack_position,
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

/// Reads and discards clientbound traffic on `socket` until a `Block Update` matching exactly
/// `(pos, expected_id)` is seen, then returns -- leaving the accumulator positioned right after
/// it. Every test below that watches a bystander for a scheduled-tick-driven change first places
/// the triggering block itself, whose own direct-response `Block Update` `broadcast_to_all`
/// (`world.rs`) *also* delivers to the bystander (every currently-connected player, actor
/// included) -- a blind, fixed-duration drain (`drain_traffic_for`) risks either stopping before
/// that echo arrives (leaving it to be misread as the later genuine change by a subsequent
/// `collect_block_updates_at` scan) or, if long enough to be safe against that, running past the
/// LATER genuine change itself and silently discarding it too (both real failure modes this
/// file's own first draft of the two tests below hit directly -- confirmed by tracing the
/// server's own tick-indexed `TickChangedPositions` drain against the client's own observed
/// traffic side by side). Matching the echo's own exact, already-known value instead of racing a
/// clock closes both failure modes at once: nothing after the echo is ever consumed here, no
/// matter how soon it follows.
async fn drain_until_echo(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
    pos: BlockPos,
    expected_id: i32,
    window: Duration,
) {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(
            !remaining.is_zero(),
            "drain_until_echo: timed out waiting for the echo Block Update at {pos:?} = \
             {expected_id}"
        );
        let (id, body) = tokio::time::timeout(remaining, recv_clientbound(socket, accumulator))
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "drain_until_echo: timed out waiting for the echo Block Update at {pos:?} = \
                     {expected_id}"
                )
            });
        if id == BlockUpdate::ID {
            let update = decode_one::<BlockUpdate>(body).unwrap();
            if unpack_position(update.location) == pos && update.block_state_id == expected_id {
                return;
            }
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

async fn wait_until(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Sets `yaw`/`pitch` and waits for it to land server-side -- every yaw-driven placement below
/// (a repeater's own `FACING`) needs this first. Never touches position -- every actor in this
/// file stays at spawn, mirroring `play_block_state_orientation_real_client.rs`'s own identical
/// helper (this crate's own established per-file-duplication convention, no shared `tests/`
/// support module exists today).
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

// Yaw values producing each cardinal repeater `FACING`
// (`nearest_horizontal_direction4(yaw).opposite()` -- `mining_block_state_ids.rs`'s/
// `play_block_state_orientation_real_client.rs`'s own identical constants/derivation, restated
// here since integration tests cannot share code across files in this crate today).
const YAW_FACING_SOUTH: f32 = 180.0;

/// A per-stage hang guard (`play_block_break_place_full.rs`'s own established helper, restated
/// here): wraps `fut` in an outer `tokio::time::timeout`, panicking with `stage`'s own name on
/// expiry rather than letting a genuine regression hang this file's own outer 60s test timeout
/// silently until that fires with a far less specific message.
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

#[tokio::test]
async fn isolated_redstone_wire_gets_the_connected_plus_shape_not_a_bare_dot() {
    tokio::time::timeout(Duration::from_secs(300), async {
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
async fn two_adjacent_wires_connect_to_each_other_on_both_sides_orientation_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
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
    tokio::time::timeout(Duration::from_secs(300), async {
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
async fn a_floor_torch_pops_when_its_own_support_block_is_broken_nondefault_case() {
    tokio::time::timeout(Duration::from_secs(300), async {
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

/// M3 field-report test-authoring: closes the gap the previous wave's own field report
/// root-caused (`docs/findings-for-planning.md`'s own "block-state changes made outside a
/// direct player action never reach any client" entry) -- `executor.tick_region`'s own ordinary
/// per-tick Stage-4 dispatch (a scheduled tick with no concurrent direct player action) now
/// broadcasts through `UpdateContext::changed`/`rc_mechanics::stage4::ecs::TickChangedPositions`,
/// drained once per tick by `crates/server/src/play/world.rs`'s own tick loop -- the disciplined
/// replacement for the retired `snapshot_cascade_neighborhood`/`broadcast_cascaded_changes`
/// bounded-neighborhood stop-gap. Proves it for a torch's own delayed re-eval turning it OFF: a
/// four-block vertical stack (Stone S1 -> driver floor torch, always lit, giving direct upward
/// power into whatever sits on top of it, real vanilla's own "torch strongly powers the block
/// directly above it" mechanic -> Stone B, now powered -> the torch under test, placed on top of
/// B). The instant the test torch is placed, its OWN placement-time self-resolution
/// (`apply_placement`'s own Root Cause 2 fix) synchronously sees B already reading power=15 and
/// schedules its own 2-tick re-eval (`TorchBehavior::REEVAL_DELAY`) -- entirely without any
/// later player action -- so its own direct placement response still reads `lit=true`; only the
/// LATER, unsolicited `Block Update` (this test's real point) reads `lit=false`.
#[tokio::test]
async fn a_floor_torch_turns_off_via_a_scheduled_tick_with_no_further_player_action() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // B is a bystander watching for the torch's own scheduled-tick flip -- proving it
        // reaches a real, entirely passive client, never merely the acting connection's own
        // traffic.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // S1: a Stone support, purely so the driver torch below has a solid floor to sit on.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s1_pos = BlockPos::new(1, -59, 0);
        let s1_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(1, -60, 0), 1).await;
        assert_ne!(s1_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The driver: a floor torch on S1. Nothing ever powers ITS OWN support (S1 stays plain
        // Stone forever), so it stays lit permanently -- a stable power source for the rest of
        // this circuit.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let driver_pos = BlockPos::new(1, -58, 0);
        let driver_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s1_pos, 1).await;
        assert_eq!(driver_id, 6885, "driver floor torch -> lit=true");
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // B: a Stone directly above the driver -- a conductor, so it aggregates and relays the
        // driver's own direct upward signal onward (`direct_signal_to`) to whatever touches B on
        // any OTHER face, including straight up.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let b_pos = BlockPos::new(1, -57, 0);
        let b_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, driver_pos, 1).await;
        assert_ne!(b_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The torch under test: a floor torch on B.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(1, -56, 0);
        let torch_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, b_pos, 1).await;
        assert_eq!(
            torch_id, 6885,
            "the torch's own direct placement response -- still lit=true; the re-eval it \
             already scheduled synchronously, this same action, has not fired yet"
        );
        // Consumes ONLY B's own echo of this SAME placement's direct response (still lit=true,
        // 6885) -- never a fixed time window, which could just as easily race past the 2-tick
        // re-eval below and silently swallow it too (`drain_until_echo`'s own doc comment has
        // the full incident citation).
        drain_until_echo(&mut b, &mut b_acc, torch_pos, 6885, Duration::from_secs(2)).await;

        // WITHOUT any further player action: the scheduled re-eval fires 2 game ticks later
        // (100ms at 20 TPS) -- driven purely by `executor.tick_region`'s own ordinary per-tick
        // dispatch, never by another direct player action.
        let seen = staged(
            "torch scheduled re-eval reaches a real client",
            Duration::from_secs(5),
            collect_block_updates_at(
                &mut b,
                &mut b_acc,
                &[torch_pos],
                Duration::from_millis(2000),
            ),
        )
        .await;

        let torch_state = world.debug_query_block(torch_pos).await.unwrap();
        assert_eq!(
            seen.get(&torch_pos).copied(),
            Some(6886),
            "the torch's own scheduled re-eval (lit=true -> lit=false) must reach a real client \
             as its own unsolicited Block Update, with no further player action -- got {seen:?}"
        );

        assert_eq!(
            torch_state.raw_state, 6886,
            "torch reads lit=false server-side too"
        );
    })
    .await
    .unwrap();
}

/// Companion to the torch test above: a repeater's own scheduled `POWERED` flip, a SECOND,
/// independent tier-1 redstone component proving the same disciplined broadcast mechanism.
/// `Repeater` IS one of `apply_placement`'s own placement-time self-resolution kinds (Root
/// Cause 2, unlike Piston -- `docs/findings-for-planning.md`'s own matching entry on that gap),
/// so R2's OWN placement already self-checks against the driver torch (already lit before R2 is
/// placed) and schedules its own 2-tick powered-flip re-eval synchronously, entirely without a
/// later player action.
#[tokio::test]
async fn a_repeater_flips_powered_via_a_scheduled_tick_with_no_further_player_action() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // B is a bystander watching for the repeater's own scheduled-tick flip.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // S2: a Stone the driver torch attaches to.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s2_pos = BlockPos::new(3, -59, 0);
        let s2_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -60, 0), 1).await;
        assert_ne!(s2_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The driver: a wall torch on S2's own NORTH face (`direction: 2` = `Face::North`,
        // `block_action.rs`'s own `Face` enum order) -- always lit, emitting weak signal 15
        // toward every direction except its own input side (South, back into S2) -- North
        // included, the direction the repeater below reads it from.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let driver_pos = BlockPos::new(3, -59, -1);
        let driver_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s2_pos, 2).await;
        assert_eq!(driver_id, 6887, "driver wall torch, facing=north, lit=true");
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // R2: a repeater, FACING=South (its own input side), placed directly north of the
        // driver -- so its own front face points straight at it. `YAW_FACING_SOUTH` ->
        // `nearest_horizontal_direction4(180.0).opposite()` = South (this file's own top-of-file
        // doc comment has the shared id-arithmetic citation with `mining_block_state_ids.rs`).
        rotate(&mut a, &world, uuid_a, YAW_FACING_SOUTH, 0.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Repeater))
            .await;
        let r2_pos = BlockPos::new(3, -59, -2);
        let r2_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -60, -2), 1).await;
        assert_eq!(
            r2_id, 7041,
            "repeater facing=south, delay=1, locked=false, powered=false -- R2's OWN placement \
             already self-resolves against the driver's already-lit signal, scheduling its own \
             2-tick powered-flip re-eval without waiting for a later notify"
        );
        // Consumes ONLY B's own echo of this SAME placement's direct response (still
        // powered=false, 7041) -- never a fixed time window (`drain_until_echo`'s own doc
        // comment has the full incident citation: a blind window either races past the 2-tick
        // powered-flip below and swallows it too, or stops too early and lets a later scan
        // misread this echo as that flip).
        drain_until_echo(&mut b, &mut b_acc, r2_pos, 7041, Duration::from_secs(2)).await;

        // WITHOUT any further player action: the scheduled powered-flip fires 2 game ticks later
        // (`RepeaterBehavior::get_delay`, `delay_setting=1` -> 2 ticks).
        let seen = staged(
            "repeater scheduled powered-flip reaches a real client",
            Duration::from_secs(5),
            collect_block_updates_at(&mut b, &mut b_acc, &[r2_pos], Duration::from_millis(2000)),
        )
        .await;
        assert_eq!(
            seen.get(&r2_pos).copied(),
            Some(7040),
            "the repeater's own scheduled POWERED flip must reach a real client as its own \
             unsolicited Block Update, with no further player action -- got {seen:?}"
        );

        let r2_state = world.debug_query_block(r2_pos).await.unwrap();
        assert_eq!(
            r2_state.raw_state, 7040,
            "repeater reads powered=true server-side too"
        );
        let _ = driver_pos; // named for readability above; asserted via `driver_id` already.
    })
    .await
    .unwrap();
}

/// M3 field-report test-authoring: closes the piston-specific half of `docs/findings-for-
/// planning.md`'s own "a piston placed by an actual connected player is never wired into
/// `PistonBehavior`'s own internal per-position state at all" finding -- `PistonBehavior` now
/// implements `on_placed` (`crates/mechanics/src/redstone/piston.rs`), seeding its own
/// per-position `facing`/`sticky`/`extended` state straight off the placed id exactly like
/// `RepeaterBehavior`/`ComparatorBehavior`/`WireBehavior::on_placed` already do -- closing the
/// "every `BlockBehavior` method early-returns the instant `self.state.lock().unwrap().get(&pos)`
/// comes back `None`" gap the finding root-caused.
///
/// Proves the full round trip end-to-end over real loopback connections: a real player places a
/// retracted piston (no signal reaches it yet -- its own placement response still just reads the
/// plain retracted id), then places a redstone torch adjacent to one of its non-push-direction
/// sides. The torch's own placement-time fan-out (`border::fan_out_from_changed_block`, the same
/// mechanism every other tier-1 component's own placement already exercises) reaches the piston's
/// `on_neighbor_changed`, which now finds a properly seeded `PistonState` instead of silently
/// early-returning -- queuing a real extend. Then removes the power (breaking the torch) and
/// watches the piston retract -- entirely without further player action after each triggering
/// placement/break, both directions of `TickChangedPositions`'s own disciplined broadcast
/// (previous wave) reach a real, entirely passive bystander connection.
///
/// Facing/yaw arithmetic is hand-derived and cross-checked the same way this file's own top
/// doc-comment already establishes for wire: `nearest_direction6(90.0, 0.0)` resolves to `West`
/// (`look_vector(90, 0) = (1, 0, 0)`, dominant `+x`), so `.opposite()` -- `resolve_orientation`'s
/// own piston rule -- gives `facing = East`, matching `mining.rs`'s own `full6_piston_index`/
/// `piston.rs`'s own `piston_facing_index` shared `[north, east, south, west, up, down]` order
/// (`East` index `1`).
#[tokio::test]
async fn a_piston_placed_by_a_real_player_extends_and_retracts_via_an_adjacent_torch() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        // B is a bystander watching for the piston's own extend/retract -- proving both reach a
        // real, entirely passive client, never merely the acting connection's own traffic.
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // S: a Stone the piston is placed against, purely as a placement anchor -- the piston's
        // own FACING points AWAY from it (East), never into it.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s_pos = BlockPos::new(2, -59, 0);
        let s_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -60, 0), 1).await;
        assert_ne!(s_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The piston, FACING=East, placed by clicking S's own EAST face (direction=5) -- lands
        // directly east of S, pushing further east, away from S.
        rotate(&mut a, &world, uuid_a, 90.0, 0.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Piston))
            .await;
        let piston_pos = BlockPos::new(3, -59, 0);
        let head_pos = BlockPos::new(4, -59, 0);
        let piston_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s_pos, 5).await;
        // PISTON default = 2263 (extended=false, facing=north); `full6_piston_index(East) == 1`:
        // 2263 + 1 = 2264 -- a plain retracted piston, no signal anywhere near it yet.
        assert_eq!(
            piston_id, 2264,
            "piston facing=east, extended=false -- freshly placed, no signal yet"
        );
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The driver: a floor torch at the piston's own NORTH neighbor (`piston_neighbor_
        // signal`'s own candidate list -- North is not this piston's own push direction, East, so
        // it is a valid activation side), standing on its OWN separate floor support --
        // deliberately NOT attached to the piston's own face (a wall torch's own `weak_signal_
        // toward` returns 0 toward its own attachment/input side, `torch.rs`'s own doc comment --
        // attaching directly to the piston would emit no signal toward it at all).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(3, -59, -1);
        let torch_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(3, -60, -1), 1).await;
        assert_eq!(torch_id, 6885, "floor torch -> lit=true");

        // The torch's OWN placement-time fan-out reaches the piston synchronously, this same
        // action -- `PistonBehavior::on_placed`'s own earlier seeding (this changeset's own new
        // production wiring) is what lets `on_neighbor_changed` actually find real state here
        // instead of silently early-returning. The resulting `TRIGGER_EXTEND` block event is only
        // QUEUED here, not yet processed -- `run_block_event_subphase` picks it up at the next
        // real Stage-4 tick, entirely without any further player action.
        let seen = staged(
            "piston extends via an adjacent torch, no further player action",
            Duration::from_secs(5),
            collect_block_updates_at(
                &mut b,
                &mut b_acc,
                &[piston_pos, head_pos],
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
        let piston_state = world.debug_query_block(piston_pos).await.unwrap();
        assert_eq!(piston_state.raw_state, 2258, "extended server-side too");
        let head_state = world.debug_query_block(head_pos).await.unwrap();
        assert_eq!(head_state.raw_state, 2275, "head settled server-side too");

        // Now remove the power: breaking the torch (Creative -> instant finalize) synchronously
        // fans out to the piston's own North face, exactly mirroring `a_floor_torch_pops_when_
        // its_own_support_block_is_broken`'s own established break-cascade shape -- the resulting
        // `TRIGGER_CONTRACT` is again only queued here, processed at the next real tick.
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

        let seen = staged(
            "piston retracts once its power source is broken, no further player action",
            Duration::from_secs(5),
            collect_block_updates_at(
                &mut b,
                &mut b_acc,
                &[piston_pos, head_pos],
                Duration::from_millis(2500),
            ),
        )
        .await;
        assert_eq!(
            seen.get(&head_pos).copied(),
            Some(0),
            "a bare (non-sticky) retract's content clears to air, immediately at block-event time \
             -- got {seen:?}"
        );
        assert_eq!(
            seen.get(&piston_pos).copied(),
            Some(2264),
            "the base's own EXTENDED=false flip settles two ticks later -- got {seen:?}"
        );
        let piston_state = world.debug_query_block(piston_pos).await.unwrap();
        assert_eq!(piston_state.raw_state, 2264, "retracted server-side too");
        let head_state = world.debug_query_block(head_pos).await.unwrap();
        assert_eq!(head_state.raw_state, 0, "head cell is air server-side too");
    })
    .await
    .unwrap();
}

/// Companion to the extend/retract test above: a STICKY piston, proving the sticky-pull half of
/// the same real-player-placement wiring -- a stone directly in front of the settled head is
/// pulled BACK to the head's own old position when the piston retracts (`resolve_retract`'s own
/// one-block candidate walk), rather than merely clearing to air like a bare retract does.
///
/// `nearest_direction6(270.0, 0.0)` resolves to `East` (`mining.rs`'s own `look_vector`:
/// `yaw_sin = sin(270deg) = -1`, `look.x = -yaw_sin * cos(pitch) = 1`, dominant axis `+x` =
/// `East`), so `.opposite()` gives `facing = West` for this piston.
#[tokio::test]
async fn a_sticky_piston_placed_by_a_real_player_pulls_the_block_back_on_retract() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world = HardcodedWorld::new();
        let uuid_a = uuid::Uuid::from_u128(1);
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // S: the sticky piston's own placement anchor -- FACING points away from it (West).
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let s_pos = BlockPos::new(-2, -59, 0);
        let s_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(-2, -60, 0), 1).await;
        assert_ne!(s_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        rotate(&mut a, &world, uuid_a, 270.0, 0.0).await;
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::StickyPiston))
            .await;
        let piston_pos = BlockPos::new(-3, -59, 0);
        let head_pos = BlockPos::new(-4, -59, 0);
        let candidate_pos = BlockPos::new(-5, -59, 0);
        let piston_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, s_pos, 4).await;
        // STICKY_PISTON default = 2241 (extended=false, facing=north); `full6_piston_index(West)
        // == 3`: 2241 + 3 = 2244.
        assert_eq!(
            piston_id, 2244,
            "sticky piston facing=west, extended=false -- freshly placed, no signal yet"
        );
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // The driver, at the piston's own NORTH neighbor -- same non-attached, own-support shape
        // as the companion test above.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;
        let torch_pos = BlockPos::new(-3, -59, -1);
        let torch_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(-3, -60, -1), 1).await;
        assert_eq!(torch_id, 6885, "floor torch -> lit=true");

        let seen = staged(
            "sticky piston extends via an adjacent torch, no further player action",
            Duration::from_secs(5),
            collect_block_updates_at(
                &mut b,
                &mut b_acc,
                &[piston_pos, head_pos],
                Duration::from_millis(2500),
            ),
        )
        .await;
        assert_eq!(
            seen.get(&piston_pos).copied(),
            Some(2238),
            "sticky base flips to extended=true (facing=west) -- got {seen:?}"
        );
        assert_eq!(
            seen.get(&head_pos).copied(),
            Some(2284),
            "a STICKY piston_head, facing=west, settles two ticks later -- got {seen:?}"
        );
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // A Stone placed directly in front of the now-settled head -- `resolve_retract`'s own
        // one-block sticky-pull candidate, two cells out from the piston along its own push
        // direction. Clicked on the head's own WEST face (direction=4), which is now a real,
        // solid `piston_head` block server-side.
        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let stone_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, head_pos, 4).await;
        assert_ne!(stone_id, 0);
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        // Remove the power: breaking the torch synchronously fans out to the piston, queuing a
        // real sticky retract (`TRIGGER_CONTRACT`, `resolve_retract` finds the Stone as a
        // pushable/pullable Normal-class candidate).
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

        let seen = staged(
            "sticky piston pulls the stone back once its power source is broken, no further \
             player action",
            Duration::from_secs(5),
            collect_block_updates_at(
                &mut b,
                &mut b_acc,
                &[candidate_pos, head_pos, piston_pos],
                Duration::from_millis(2500),
            ),
        )
        .await;
        assert_eq!(
            seen.get(&candidate_pos).copied(),
            Some(0),
            "the pulled block's own source position clears to air immediately, at block-event \
             time -- got {seen:?}"
        );
        assert_eq!(
            seen.get(&head_pos).copied(),
            Some(stone_id),
            "the pulled Stone's own real content lands at the head's old position, deferred \
             alongside the base's own EXTENDED flip -- got {seen:?}"
        );
        assert_eq!(
            seen.get(&piston_pos).copied(),
            Some(2244),
            "the sticky base flips back to extended=false (facing=west) -- got {seen:?}"
        );

        let candidate_state = world.debug_query_block(candidate_pos).await.unwrap();
        assert_eq!(
            candidate_state.raw_state, 0,
            "candidate cell is air server-side too"
        );
        let head_state = world.debug_query_block(head_pos).await.unwrap();
        assert_eq!(
            head_state.raw_state, stone_id as u32,
            "the pulled Stone settled at the head's old position server-side too"
        );
        let piston_state = world.debug_query_block(piston_pos).await.unwrap();
        assert_eq!(piston_state.raw_state, 2244, "retracted server-side too");
    })
    .await
    .unwrap();
}
