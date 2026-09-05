//! test-matrix: boundaries=waived(the two Y positions checked (-60 surface air, -62 dirt) are this changeset's own acceptance-criterion positions, not a Y=-64/319 world-limit sweep -- see light_chunk_border.rs/light_propagation_golden_grids.rs for the mechanics-level boundary suite) orientations=waived(a single default yaw/pitch spawn look, straight down, is used throughout -- Face::Up only, mirroring play_block_place_break.rs's own identical placement setup) self=waived(the actor's own reach/obstruction gate is exercised only incidentally, never this suite's own subject) composition=waived(single torch, no ≥3-component chain) nondefault-state=yes
//! M4-B07 field-report test-authoring: end-to-end proof that the composition root actually
//! wires the M4-B07 Stage-8 light engine into `HardcodedWorld` -- before this changeset,
//! `RcExecutorBuilder::with_lighting_driver` was never called (`world.rs`'s own executor-
//! build block had every other Stage registration but this one), no `LightPropertiesRegistry`
//! resource existed in `bootstrap_region`, and no real chunk entity ever carried
//! `LightPropagatorState`/`SkyLightSourceColumn` (the production chunk-spawn site,
//! `rc_chunk_storage::lifecycle::ChunkLifecycleManager::pre_tick`, cannot construct either
//! type itself -- WS-D3, `rc-chunk-storage` sits below `rc-mechanics`) -- so every light read
//! was unconditionally `0` regardless of what the block-level engine itself computed
//! correctly. This suite joins a real loopback player, reads sky/block light back through
//! `HardcodedWorld::debug_query_light` (mirrors `debug_query_block`'s own established
//! contract), places a redstone torch through the real network placement pipeline (`UseItemOn`
//! -- `PlaceableBlockKind` has no plain `Torch` variant at this milestone's own scope, only
//! `RedstoneTorch`; a freshly placed, unpowered redstone torch resolves `lit=true` by
//! `RedstoneTorchBehavior`'s own "never observed" default, `redstone/torch.rs`'s own doc
//! comment, so it lights at its own real emission value, `7`, per `production_registry`'s
//! own `REDSTONE_TORCH` entry -- `litBlockEmission(7)`, `Blocks.java` line ~1923), and breaks
//! it again, confirming light tracks the change at every step.
//!
//! Superflat filler geometry this suite depends on (`crates/server/src/play/world.rs`'s own
//! `superflat_filler`/`SuperflatFiller`, M2-B05's restatement of M1-B05's own byte-verified
//! layer table): bedrock at Y=-64, dirt at Y=-63/-62, grass at Y=-61, air from Y=-60 upward --
//! every column in this world shares this identical vertical structure, so the exact (x, z)
//! chosen below carries no significance beyond "loaded and reachable."

use rc_core::BlockPos;
use rc_registries::block_state_properties::state_id;
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, KeepAliveClientbound,
    KeepAliveServerbound, PlayerAction, SetPlayerPositionAndRotation, UseItemOn, pack_position,
};
use rusty_clanker_server::play::{
    HardcodedWorld, HeldItemStub, PlaceableBlockKind, PlayerProfile, enter_play,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;

use bytes::{Bytes, BytesMut};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};

// --- Shared harness (mirrors `play_block_place_break.rs`'s own identical helpers) ---

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
    let world = world.clone();
    let profile = PlayerProfile {
        uuid,
        username: username.to_string(),
    };
    tokio::spawn(async move {
        enter_play(handle, inbound, profile, &world).await;
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

/// Polls `world.debug_query_light(pos)` (bounded only by the surrounding test's own outer
/// `tokio::time::timeout` -- mirrors `play_block_place_break.rs`'s own `wait_until`'s
/// established "no second, independent deadline" rationale) until it equals `expected`, then
/// returns.
async fn wait_for_light(world: &HardcodedWorld, pos: BlockPos, expected: (u8, u8)) {
    loop {
        if world.debug_query_light(pos).await == Some(expected) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// As `play_block_place_break.rs`'s own `wait_until` -- polls a synchronous predicate.
async fn wait_until_sync(mut check: impl FnMut() -> bool) {
    loop {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn light_engine_wiring_end_to_end() {
    tokio::time::timeout(std::time::Duration::from_secs(300), async {
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        // --- Sky light already converged by the time the join's own chunk grid finished
        // loading (Stage 8 runs every tick, `ChunkLifecycleManager::pre_tick` spawns a fresh
        // chunk entity with `LightColumn::new_uninitialized()`, and this changeset's own
        // `ensure_light_components` backfill + Stage 8's own "freshly uninitialized" full-
        // recompute trigger, M4-B07 Context §8 step 2, together seed it on the very first
        // tick that entity exists) -- full sun (15) at the surface air cell, zero inside the
        // dirt two blocks below it, zero block light throughout (no emitter placed yet). ---
        wait_for_light(&world, BlockPos::new(3, -60, 2), (15, 0)).await;
        assert_eq!(
            world.debug_query_light(BlockPos::new(3, -62, 2)).await,
            Some((0, 0)),
            "inside the dirt, two blocks below the surface, should read (sky=0, block=0)"
        );

        // --- Place a redstone torch on top of the grass block at (3, -61, 2) -- A moves to
        // (2, -60, 2) first (mirrors `play_block_place_break.rs`'s own identical "click the
        // neighbouring column, not the one A is standing in" placement setup, avoiding this
        // same obstruction gate on the actor's own feet). ---
        send_packet(
            &mut a,
            &SetPlayerPositionAndRotation {
                x: 2.0,
                y: -60.0,
                z: 2.0,
                yaw: 0.0,
                pitch: 90.0,
                on_ground: true,
            },
        )
        .await;
        let sessions = world.player_sessions();
        wait_until_sync(|| {
            sessions.with_record_mut(uuid::Uuid::from_u128(1), |r| r.data.pos)
                == Some([2.0, -60.0, 2.0])
        })
        .await;

        world
            .debug_set_held_item(1, HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch))
            .await;

        send_packet(
            &mut a,
            &UseItemOn {
                hand: 0,
                location: pack_position(BlockPos::new(3, -61, 2)),
                direction: 1, // Face::Up
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside_block: false,
                hits_world_border: false,
                sequence: 1,
            },
        )
        .await;

        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            1
        );
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(3, -60, 2)));
        // A freshly placed, unpowered redstone torch is `lit=true` (`redstone/torch.rs`'s own
        // "never observed" default, module doc comment above) -- a non-default block state
        // (`REDSTONE_TORCH`'s own generated default state also happens to be `lit=true`, but
        // this asserts the placed id explicitly rather than assuming it).
        let lit_torch_id = state_id(block_id::REDSTONE_TORCH, &[("lit", "true")])
            .expect("lit=true is a legal minecraft:redstone_torch value");
        assert_eq!(update.block_state_id, lit_torch_id.0 as i32);

        // --- Block light now tracks the torch: 7 at the torch's own cell (its own real
        // emission, `production_registry`'s own `REDSTONE_TORCH` entry), 6 one block away
        // (opacity floors at 1 per hop through open air, `LightProperties::get_opacity`'s own
        // `MIN_OPACITY` floor) -- within a few ticks, since Stage 8's own bounded 16-round
        // BSP loop (M4-B07 Context §8) converges well before that cap for a single source
        // this close to its own origin. ---
        wait_for_light(&world, BlockPos::new(3, -60, 2), (15, 7)).await;
        wait_for_light(&world, BlockPos::new(4, -60, 2), (15, 6)).await;

        // --- Break it again -- creative/instabuild players break instantly on `StartDestroy`
        // alone (`play_block_place_break.rs`'s own identical single-packet break). ---
        send_packet(
            &mut a,
            &PlayerAction {
                status: 0,
                location: pack_position(BlockPos::new(3, -60, 2)),
                direction: 1,
                sequence: 2,
            },
        )
        .await;
        let body = recv_packet_of_type(&mut a, &mut a_acc, AcknowledgeBlockChange::ID).await;
        assert_eq!(
            decode_one::<AcknowledgeBlockChange>(body).unwrap().sequence,
            2
        );
        let body = recv_packet_of_type(&mut a, &mut a_acc, BlockUpdate::ID).await;
        let update = decode_one::<BlockUpdate>(body).unwrap();
        assert_eq!(update.location, pack_position(BlockPos::new(3, -60, 2)));
        assert_eq!(update.block_state_id, blocks::AIR.0 as i32);

        wait_for_light(&world, BlockPos::new(3, -60, 2), (15, 0)).await;
        wait_for_light(&world, BlockPos::new(4, -60, 2), (15, 0)).await;
    })
    .await
    .unwrap();
}
