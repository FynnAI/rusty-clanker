//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=waived(a single default yaw/pitch spawn rotation is used throughout -- North-facing components only, see redstone_repeater.rs/redstone_comparator.rs for the pure per-facing sweep) self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(single component per test, no ≥3-component chain) nondefault-state=yes
//! M3 field-report test-authoring (MECH-D82, wave 3 Stream B, task B2/B3): block-use
//! dispatch, end-to-end over real loopback connections -- mirrors `play_hopper_enabled_field_
//! report.rs`'s/`play_placement_candidate_field_report.rs`'s own established harness. The
//! comparator's own side-input scenario uses a `minecraft:redstone_block` (always-constant-15,
//! already wired into production dispatch) as the "side input," never a real chest -- setting
//! up a container with real items over the wire needs full inventory-click plumbing this
//! changeset does not build; the mechanics-level `redstone_comparator_use.rs` already proves
//! `on_use`'s side-input re-evaluation with a synthetic signal source directly.

use bytes::{Bytes, BytesMut};
use rc_core::BlockPos;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::block_state_properties::{properties, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::block_states::default_state::{REDSTONE_BLOCK, STONE};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PLAYER_INPUT_SHIFT, PlayerInput, Sound, UseItemOn, pack_position,
    unpack_position,
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

/// A plain block-use click (no placement expected) -- sends `UseItemOn` and consumes the
/// unconditional `Acknowledge Block Change`, without assuming any particular clientbound
/// packet follows on the actor's own connection (MECH-D78's own dual-cell resend is asserted
/// separately, in `play_use_resend_field_report.rs`).
async fn click(
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

async fn set_sneaking(actor: &mut TcpStream, sneaking: bool) {
    send_packet(
        actor,
        &PlayerInput {
            flags: if sneaking { PLAYER_INPUT_SHIFT } else { 0 },
        },
    )
    .await;
}

/// Scans clientbound traffic on `socket` for up to `window`, collecting every `Block Update`
/// whose own `location` matches one of `wanted` (mirrors `play_hopper_enabled_field_report.rs`'s
/// own identical helper).
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

/// Drains every clientbound packet on `socket` for the FULL `window`, unconditionally --
/// unlike `collect_block_updates_at`, never returns early on a first match (mirrors
/// `play_hopper_enabled_field_report.rs`'s own identical `drain_traffic_for` helper).
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

fn repeater_id(delay: u8) -> u32 {
    state_id(
        block_id::REPEATER,
        &[
            ("facing", "north"),
            ("delay", &delay.to_string()),
            ("locked", "false"),
            ("powered", "false"),
        ],
    )
    .unwrap()
    .0
}

fn comparator_powered(raw: u32) -> bool {
    properties(GenStateId(raw))
        .iter()
        .find(|(name, _)| *name == "powered")
        .unwrap()
        .1
        == "true"
}

fn comparator_mode(raw: u32) -> &'static str {
    match properties(GenStateId(raw))
        .iter()
        .find(|(name, _)| *name == "mode")
        .unwrap()
        .1
    {
        "compare" => "compare",
        "subtract" => "subtract",
        other => panic!("unrecognized mode {other:?}"),
    }
}

#[tokio::test]
async fn four_clicks_on_a_repeater_cycle_delay_with_one_update_per_click() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Repeater))
            .await;
        let repeater_pos = BlockPos::new(2, -60, 0);
        let placed_id =
            place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        assert_eq!(placed_id, repeater_id(1) as i32, "placed at delay=1");

        // Drains the placement's own already-broadcast `Block Update` from `b`'s own
        // connection first -- otherwise the click loop's own first `collect_block_updates_at`
        // call would pick up that stale, pre-click packet instead of a fresh one.
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_millis(300)).await;

        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;
        for expected_delay in [2u8, 3, 4, 1] {
            click(&mut a, &mut a_acc, &mut seq, repeater_pos, 1).await;
            let seen = collect_block_updates_at(
                &mut b,
                &mut b_acc,
                &[repeater_pos],
                Duration::from_millis(500),
            )
            .await;
            assert_eq!(
                seen.get(&repeater_pos).copied(),
                Some(repeater_id(expected_delay) as i32),
                "delay must cycle to {expected_delay} and reach the bystander -- got {seen:?}"
            );
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn comparator_mode_cycle_with_a_side_input_flips_powered_and_sounds() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let (mut b, mut b_acc) = spawn_actor(&world, "b", 2).await;
        let mut seq = 0;

        // FACING = North (default spawn rotation) -> input from North, side = East/West,
        // output/front = South. Front input 15, side input 15: Compare mode ties on ->
        // powered=true; Subtract mode 15-15=0 -> powered=false. Both signal sources are
        // placed BEFORE the comparator itself, so placement's own self-resolution already
        // seeds powered=true, mode=compare.
        world
            .debug_set_block_state(BlockPos::new(2, -60, -1), REDSTONE_BLOCK.0)
            .await; // front (North neighbor)
        world
            .debug_set_block_state(BlockPos::new(3, -60, 0), REDSTONE_BLOCK.0)
            .await; // side (East neighbor)

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Comparator))
            .await;
        let comparator_pos = BlockPos::new(2, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;

        // `refresh_output`'s own `POWERED` re-evaluation is a SCHEDULED tick (`on_neighbor_
        // changed`'s own placement-time self-resolution only ever *schedules* it, matching
        // vanilla's own asynchronous diode settle -- never a synchronous placement-time
        // write), so polls (bounded) rather than asserting immediately.
        let mut before_powered = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            let info = world.debug_query_block(comparator_pos).await.unwrap();
            assert_eq!(comparator_mode(info.raw_state), "compare");
            if comparator_powered(info.raw_state) {
                before_powered = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            before_powered,
            "input=15/side=15/compare must settle to powered=true"
        );
        // Drains the settle tick's own already-broadcast `Block Update` (if any reached `b`
        // before this point) so the click loop's own collection below starts clean -- a
        // generous, unconditional (never-early-return) window since the settle tick's own
        // broadcast can lag `debug_query_block`'s own direct (non-network) read by more than
        // one tick.
        drain_traffic_for(&mut b, &mut b_acc, Duration::from_secs(1)).await;

        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;
        click(&mut a, &mut a_acc, &mut seq, comparator_pos, 1).await;

        let seen = collect_block_updates_at(
            &mut b,
            &mut b_acc,
            &[comparator_pos],
            Duration::from_millis(500),
        )
        .await;
        let after = *seen
            .get(&comparator_pos)
            .expect("the mode/powered flip must reach the bystander");
        assert_eq!(comparator_mode(after as u32), "subtract");
        assert!(
            !comparator_powered(after as u32),
            "15-15 must not turn it on"
        );

        // B3: the actor receives no sound at all; the bystander receives exactly one, at
        // the comparator's own position, pitch 0.55 (subtract).
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        let mut actor_sound = None;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, recv_clientbound(&mut a, &mut a_acc)).await {
                Ok((id, body)) if id == Sound::ID => {
                    actor_sound = Some(decode_one::<Sound>(body).unwrap());
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(
            actor_sound.is_none(),
            "the acting player must not hear its own click"
        );

        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        let mut bystander_sound = None;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, recv_clientbound(&mut b, &mut b_acc)).await {
                Ok((id, body)) if id == Sound::ID => {
                    bystander_sound = Some(decode_one::<Sound>(body).unwrap());
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let sound = bystander_sound.expect("the bystander must hear the click");
        assert_eq!(sound.pitch, 0.55);
        let expected_x = ((comparator_pos.x as f64 + 0.5) * 8.0) as i32;
        let expected_y = ((comparator_pos.y as f64 + 0.5) * 8.0) as i32;
        let expected_z = ((comparator_pos.z as f64 + 0.5) * 8.0) as i32;
        assert_eq!(sound.x, expected_x);
        assert_eq!(sound.y, expected_y);
        assert_eq!(sound.z, expected_z);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn sneaking_with_stone_in_hand_places_stone_instead_and_leaves_repeater_unchanged_nondefault_case()
 {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Repeater))
            .await;
        let repeater_pos = BlockPos::new(2, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let before = world
            .debug_query_block(repeater_pos)
            .await
            .unwrap()
            .raw_state;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        set_sneaking(&mut a, true).await;
        let above_id = place_and_read_id(&mut a, &mut a_acc, &mut seq, repeater_pos, 1).await;
        assert_eq!(
            above_id, STONE.0 as i32,
            "sneaking must place stone, not cycle delay"
        );

        let after = world
            .debug_query_block(repeater_pos)
            .await
            .unwrap()
            .raw_state;
        assert_eq!(before, after, "the repeater's own state must be untouched");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn empty_hand_click_on_plain_stone_still_produces_no_state_change() {
    tokio::time::timeout(Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;
        let mut seq = 0;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::Stone))
            .await;
        let stone_pos = BlockPos::new(2, -60, 0);
        place_and_read_id(&mut a, &mut a_acc, &mut seq, BlockPos::new(2, -61, 0), 1).await;
        let before = world.debug_query_block(stone_pos).await.unwrap().raw_state;

        world.debug_set_held_item(1, HeldItemStub::EmptyHand).await;
        click(&mut a, &mut a_acc, &mut seq, stone_pos, 1).await;

        let after = world.debug_query_block(stone_pos).await.unwrap().raw_state;
        assert_eq!(before, after, "plain stone has no on_use handler at all");
    })
    .await
    .unwrap();
}
