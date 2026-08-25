//! M1-B05 acceptance test: `enter_play` sends a well-formed Play-entry sequence and the
//! full 9-chunk superflat placeholder batch over a real loopback socket, no M1-B02/B03/B04
//! dependency (Deliverables, "write these FIRST").

use std::collections::HashMap;

use bytes::{Buf, Bytes};
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rc_registries::generated_v776::block_states::default_state as blocks;
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, ChunkBatchStart, ConfirmTeleportation, GameEvent, LevelChunkWithLight,
    LoginPlay, SetChunkCacheCenter, SetDefaultSpawnPosition, SynchronizePlayerPosition,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

fn decode_paletted_container(buf: &mut Bytes) -> DecodedContainer {
    let bits_per_entry = buf.get_u8();
    match bits_per_entry {
        0 => {
            let value = VarInt::decode(buf).unwrap().get() as u32;
            let _data_array_length = VarInt::decode(buf).unwrap().get();
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
            let data_len = VarInt::decode(buf).unwrap().get() as usize;
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

fn decode_heightmaps(bytes: &[u8]) -> HashMap<String, Vec<i64>> {
    assert_eq!(
        bytes[0], 0x0A,
        "root TAG_Compound must be unnamed type id 0x0A"
    );
    let mut pos = 1usize;
    let mut result = HashMap::new();
    loop {
        let type_id = bytes[pos];
        pos += 1;
        if type_id == 0x00 {
            break;
        }
        assert_eq!(type_id, 0x0C, "expected TAG_Long_Array");
        let name_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        pos += 2;
        let name = String::from_utf8(bytes[pos..pos + name_len].to_vec()).unwrap();
        pos += name_len;
        let count = i32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let mut longs = Vec::with_capacity(count);
        for _ in 0..count {
            longs.push(i64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap()));
            pos += 8;
        }
        result.insert(name, longs);
    }
    result
}

fn unpack_bits(longs: &[i64], bits_per_entry: u32, count: usize) -> Vec<u32> {
    let entries_per_long = 64 / bits_per_entry as usize;
    let mask = (1u64 << bits_per_entry) - 1;
    let mut out = Vec::with_capacity(count);
    'outer: for long in longs {
        for i in 0..entries_per_long {
            if out.len() == count {
                break 'outer;
            }
            out.push((((*long as u64) >> (i as u32 * bits_per_entry)) & mask) as u32);
        }
    }
    out
}

#[tokio::test]
async fn enter_play_sends_a_well_formed_login_and_chunk_batch() {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
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

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, SetChunkCacheCenter::ID);
        let chunk_cache_center = decode_one::<SetChunkCacheCenter>(body).unwrap();
        assert_eq!(chunk_cache_center.chunk_x, 0);
        assert_eq!(chunk_cache_center.chunk_z, 0);

        let (id, body) = recv_packet(&mut client, &mut accumulator).await;
        assert_eq!(id, ChunkBatchStart::ID);
        decode_one::<ChunkBatchStart>(body).unwrap();

        let expected_coords: Vec<(i32, i32)> = (-1..=1)
            .flat_map(|cx| (-1..=1).map(move |cz| (cx, cz)))
            .collect();
        assert_eq!(expected_coords.len(), 9);

        let mut chunks = Vec::with_capacity(9);
        for _ in 0..9 {
            let (id, body) = recv_packet(&mut client, &mut accumulator).await;
            assert_eq!(id, LevelChunkWithLight::ID);
            chunks.push(decode_one::<LevelChunkWithLight>(body).unwrap());
        }

        let actual_coords: Vec<(i32, i32)> =
            chunks.iter().map(|c| (c.chunk_x, c.chunk_z)).collect();
        assert_eq!(actual_coords, expected_coords);

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
        assert_eq!(finished.batch_size, 9);

        // Decode section 0 and section 1 of the first chunk's `data` blob.
        let mut data = Bytes::from(first.data.clone());
        let block_count = data.get_i16();
        assert_eq!(
            block_count, 1280,
            "1 bedrock + 3 dirt + 1 grass, x256 columns"
        );

        let block_states = decode_paletted_container(&mut data);
        assert_eq!(block_states.bits_per_entry, 4);
        assert_eq!(block_states.palette.len(), 4);
        assert_eq!(index_at(&block_states, 0), blocks::BEDROCK.0);
        assert_eq!(index_at(&block_states, 15 * 256), blocks::AIR.0);

        let _biomes = decode_paletted_container(&mut data);

        let section1_block_count = data.get_i16();
        assert_eq!(section1_block_count, 0);
        let section1_blocks = decode_paletted_container(&mut data);
        assert_eq!(section1_blocks.bits_per_entry, 0);
        assert_eq!(section1_blocks.palette, vec![blocks::AIR.0]);

        // Heightmaps.
        let heightmaps = decode_heightmaps(&first.heightmaps);
        assert_eq!(heightmaps.len(), 3);
        for name in [
            "WORLD_SURFACE",
            "MOTION_BLOCKING",
            "MOTION_BLOCKING_NO_LEAVES",
        ] {
            let longs = heightmaps
                .get(name)
                .unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(longs.len(), 37);
            let values = unpack_bits(longs, 9, 256);
            assert!(values.iter().all(|&v| v == 5), "{name} must be all 5s");
        }

        task.abort();
    })
    .await
    .unwrap();
}
