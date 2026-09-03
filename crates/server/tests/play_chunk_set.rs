//! M1-B05 acceptance test: `enter_play` sends a well-formed Play-entry sequence and the
//! full superflat placeholder chunk batch over a real loopback socket, no
//! M1-B02/B03/B04 dependency (Deliverables, "write these FIRST").
//!
//! M1 integration fix, round 4: the chunk count grew from 9 (radius 1) to 25 (radius 2)
//! once a real, graphical vanilla client's own render-mesh neighbor requirement was
//! diagnosed (`rusty_clanker_server::play::chunk::PLACEHOLDER_RADIUS_CHUNKS`'s own doc
//! comment has the full writeup), and a new assertion checked the actual regression that
//! fix exists for: every chunk of the render-safe "visible" area has its own full 3x3
//! neighborhood present in the sent set.
//!
//! M1 integration fix, round 5 prep, test-authoring commit: every count/coordinate below
//! that used to hard-code the radius-2 grid's own shape now derives from
//! `EXPECTED_RADIUS_CHUNKS` instead -- a single local mirror of `play::chunk::
//! PLACEHOLDER_RADIUS_CHUNKS`'s own current value, duplicated (not imported) because
//! `chunk` stays a crate-internal module (`play::mod`'s own doc comment: "every
//! acceptance test that needs chunk-byte assertions writes its own test-local decode
//! mirror instead"). A future radius change still needs this one constant bumped here to
//! match, but nothing else in this file.
//!
//! M1 integration fix, round 5, test-authoring commit: `EXPECTED_RADIUS_CHUNKS` bumped
//! `2 -> 5` to match the companion implementation commit's own `PLACEHOLDER_RADIUS_CHUNKS`
//! raise (round-5 real-client result: the unmeshed edge at radius 2 sat too close to
//! spawn to be out of immediate view) -- the one line this file ever needs touching for a
//! radius change, exactly as designed above.

use bytes::{Buf, Bytes};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, ChunkBatchStart, ConfirmTeleportation, GameEvent, LevelChunkWithLight,
    LoginPlay, SetChunkCacheCenter, SetDefaultSpawnPosition, SetHealth, SynchronizePlayerPosition,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Mirrors `play::chunk::PLACEHOLDER_RADIUS_CHUNKS`'s own current value (module doc
/// comment above has the full "why a mirror, not an import" writeup). The one line to
/// change whenever that constant changes.
const EXPECTED_RADIUS_CHUNKS: i32 = 5;

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

/// Reads exactly one framed, uncompressed payload off `socket`, splits its leading
/// packet-id `VarInt` from the body, and returns both.
async fn recv_packet(socket: &mut TcpStream, accumulator: &mut bytes::BytesMut) -> (i32, Bytes) {
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
    let mut framed = bytes::BytesMut::new();
    rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
    socket.write_all(&framed).await.unwrap();
}

// --- Test-local chunk-byte decode mirrors (Acceptance tests' own instruction: "not a
// production deliverable") ---

struct DecodedContainer {
    bits_per_entry: u8,
    palette: Vec<u32>,
    longs: Vec<i64>,
}

/// M1 integration fix, test-authoring commit: protocol 776 carries no explicit "data
/// array length" `VarInt` on the wire (`chunk.rs`'s own `encode_paletted_container` doc
/// comment) -- a real client computes it deterministically from `bits_per_entry` and the
/// container's own fixed entry count, so this mirror now takes `entry_count` and computes
/// the same length itself instead of reading a field that no longer exists.
fn decode_paletted_container(buf: &mut Bytes, entry_count: usize) -> DecodedContainer {
    let bits_per_entry = buf.get_u8();
    match bits_per_entry {
        0 => {
            let value = VarInt::decode(buf).unwrap().get() as u32;
            DecodedContainer {
                bits_per_entry,
                palette: vec![value],
                longs: Vec::new(),
            }
        }
        _ => {
            let palette_len = VarInt::decode(buf).unwrap().get() as usize;
            let mut palette = Vec::with_capacity(palette_len);
            for _ in 0..palette_len {
                palette.push(VarInt::decode(buf).unwrap().get() as u32);
            }
            let entries_per_long = 64 / bits_per_entry as usize;
            let data_len = entry_count.div_ceil(entries_per_long);
            let mut longs = Vec::with_capacity(data_len);
            for _ in 0..data_len {
                longs.push(buf.get_i64());
            }
            DecodedContainer {
                bits_per_entry,
                palette,
                longs,
            }
        }
    }
}

fn index_at(container: &DecodedContainer, index: usize) -> u32 {
    if container.bits_per_entry == 0 {
        return container.palette[0];
    }
    let bits = container.bits_per_entry as u32;
    let entries_per_long = 64 / bits as usize;
    let long_index = index / entries_per_long;
    let slot = index % entries_per_long;
    let mask = (1u64 << bits) - 1;
    let palette_index =
        (((container.longs[long_index] as u64) >> (slot as u32 * bits)) & mask) as usize;
    container.palette[palette_index]
}

#[tokio::test]
async fn enter_play_sends_a_well_formed_login_and_chunk_batch() {
    // M2 integration test-authoring fix: raised from `10` -- `enter_play` now awaits a
    // real, ticket-driven `RC-IoPool` load of the full 121-chunk grid before sending it
    // (`connection.rs`'s own `request_chunk_grid` call), a genuinely asynchronous round
    // trip absent when this budget was first tuned against the old, instantly-
    // synthesized placeholder blob -- observed at ~6.3s under a real full-suite `cargo
    // nextest run`'s own contention, comfortably inside the old `10`s but with little
    // margin; `30` gives real headroom.
    tokio::time::timeout(std::time::Duration::from_secs(120), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        let task = tokio::spawn(async move {
            let world = HardcodedWorld::new();
            enter_play(
                handle,
                inbound,
                PlayerProfile {
                    uuid: 1,
                    username: "tester".to_string(),
                },
                &world,
            )
            .await;
        });

        let mut accumulator = bytes::BytesMut::new();

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, LoginPlay::ID);
        let login_play = decode_one::<LoginPlay>(body).unwrap();
        assert!(login_play.is_flat);
        assert_eq!(login_play.game_mode, 1);
        assert_eq!(
            login_play.dimension_names,
            vec!["minecraft:overworld".to_string()]
        );

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, SetDefaultSpawnPosition::ID);
        decode_one::<SetDefaultSpawnPosition>(body).unwrap();

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, SynchronizePlayerPosition::ID);
        let sync_position = decode_one::<SynchronizePlayerPosition>(body).unwrap();
        assert_eq!(sync_position.teleport_id, 1);

        // Prove `enter_play` does not block waiting for this ack before continuing.
        send_packet(&mut client, &ConfirmTeleportation { teleport_id: 1 }).await;

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, GameEvent::ID);
        let game_event = decode_one::<GameEvent>(body).unwrap();
        assert_eq!(game_event.event, 13);
        assert_eq!(game_event.value, 0.0);

        // M2 integration test-authoring fix: `SetHealth` is a new packet in this exact
        // Play-entry position (`connection.rs`'s own `enter_play` doc comment on this
        // call site has the full "why here, not after `ChunkBatchFinished`" writeup) --
        // this test's own strict, packet-by-packet sequence assertion needed a matching
        // decode step. A fresh player (this test's own `HardcodedWorld::new()`, never
        // persisted before) gets `LoadedPlayerRecord::fresh_default`'s own values
        // (`rc-chunk-storage`'s `player.rs`).
        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, SetHealth::ID);
        let set_health = decode_one::<SetHealth>(body).unwrap();
        assert_eq!(set_health.health, 20.0);
        assert_eq!(set_health.food, 20);
        assert_eq!(set_health.saturation, 5.0);

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, SetChunkCacheCenter::ID);
        let chunk_cache_center = decode_one::<SetChunkCacheCenter>(body).unwrap();
        assert_eq!(chunk_cache_center.chunk_x, 0);
        assert_eq!(chunk_cache_center.chunk_z, 0);

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, ChunkBatchStart::ID);
        decode_one::<ChunkBatchStart>(body).unwrap();

        let expected_coords: Vec<(i32, i32)> = (-EXPECTED_RADIUS_CHUNKS..=EXPECTED_RADIUS_CHUNKS)
            .flat_map(|cx| {
                (-EXPECTED_RADIUS_CHUNKS..=EXPECTED_RADIUS_CHUNKS).map(move |cz| (cx, cz))
            })
            .collect();
        let expected_chunk_count = expected_coords.len();

        let mut chunks = Vec::with_capacity(expected_chunk_count);
        for _ in 0..expected_chunk_count {
            let (id, body) = recv_packet(&mut client, &mut accumulator).await;
            assert_eq!(id, LevelChunkWithLight::ID);
            chunks.push(decode_one::<LevelChunkWithLight>(body).unwrap());
        }

        let actual_coords: Vec<(i32, i32)> =
            chunks.iter().map(|c| (c.chunk_x, c.chunk_z)).collect();
        assert_eq!(actual_coords, expected_coords);

        // M1 integration fix, round 4: the actual regression guard for the "only the
        // spawn chunk renders" root cause -- a real client will not build a render mesh
        // for a chunk unless every one of its own neighboring columns was also sent.
        // Every chunk of the render-safe "visible" area (radius `EXPECTED_RADIUS_CHUNKS -
        // 1`, one ring inside the actual send radius) must have its own full 3x3
        // neighborhood present in `actual_coords`.
        let sent: std::collections::HashSet<(i32, i32)> = actual_coords.iter().copied().collect();
        let render_safe_radius = EXPECTED_RADIUS_CHUNKS - 1;
        for cx in -render_safe_radius..=render_safe_radius {
            for cz in -render_safe_radius..=render_safe_radius {
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        assert!(
                            sent.contains(&(cx + dx, cz + dz)),
                            "chunk ({cx}, {cz})'s own neighbor ({}, {}) is missing from the \
                             sent set -- a real client will not render ({cx}, {cz})",
                            cx + dx,
                            cz + dz
                        );
                    }
                }
            }
        }

        let first = &chunks[0];
        for other in &chunks[1..] {
            assert_eq!(other.data, first.data);
            assert_eq!(other.heightmaps, first.heightmaps);
            assert_eq!(other.sky_light_mask, first.sky_light_mask);
            assert_eq!(other.empty_block_light_mask, first.empty_block_light_mask);
        }

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, ChunkBatchFinished::ID);
        let finished = decode_one::<ChunkBatchFinished>(body).unwrap();
        assert_eq!(finished.batch_size, expected_chunk_count as i32);

        // Decode section 0 and section 1 of the first chunk's `data` blob.
        // M1 integration fix, test-authoring commit: a real section carries a second
        // `i16` field, `fluid_count`, between `block_count` and the block paletted
        // container (`chunk.rs`'s own `encode_section` doc comment) -- read and asserted
        // here (always `0`, this world has no fluid content) so this decode mirror stays
        // byte-accurate against the real production format.
        let mut data = Bytes::from(first.data.clone());
        let block_count = data.get_i16();
        assert_eq!(
            block_count, 1024,
            "1 bedrock + 2 dirt + 1 grass, x256 columns"
        );
        let fluid_count = data.get_i16();
        assert_eq!(fluid_count, 0);

        let block_states = decode_paletted_container(&mut data, 4096);
        assert_eq!(block_states.bits_per_entry, 4);
        assert_eq!(block_states.palette.len(), 4);
        assert_eq!(index_at(&block_states, 0), blocks::BEDROCK.0);
        assert_eq!(index_at(&block_states, 15 * 256), blocks::AIR.0);

        let _biomes = decode_paletted_container(&mut data, 64);

        let section1_block_count = data.get_i16();
        assert_eq!(section1_block_count, 0);
        let section1_fluid_count = data.get_i16();
        assert_eq!(section1_fluid_count, 0);
        let section1_blocks = decode_paletted_container(&mut data, 4096);
        assert_eq!(section1_blocks.bits_per_entry, 0);
        assert_eq!(section1_blocks.palette, vec![blocks::AIR.0]);

        // M1 integration fix, test-authoring commit: `heightmaps` used to assert a
        // hand-rolled network-NBT compound (`WORLD_SURFACE`/`MOTION_BLOCKING`/
        // `MOTION_BLOCKING_NO_LEAVES`, each 37 packed longs) -- a shape a real client
        // cannot decode at all (`chunk.rs`'s own `build_placeholder_heightmaps` doc
        // comment: a real client's `ClientboundLevelChunkPacketData.heightmaps` is a
        // `VarInt`-prefixed tuple list, never NBT, and misreads our NBT byte count as a
        // bogus tuple count, desyncing immediately). Corrected to assert the now-empty
        // list this field actually carries -- legal, parity-neutral vanilla wire
        // behavior: a real client recomputes the identical heightmap values itself from
        // the chunk's own block data whenever none are supplied.
        assert!(
            first.heightmaps.is_empty(),
            "heightmaps must be sent empty -- a real client cannot decode a populated \
             field in this wire shape (see chunk.rs's own build_placeholder_heightmaps)"
        );

        task.abort();
    })
    .await
    .unwrap();
}
