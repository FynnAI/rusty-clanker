//! The versioned postcard `ChunkSnapshot` (M2-B04 Deliverables/Context, WORLD-D20).

use proptest::prelude::*;
use rc_chunk_storage::{
    ChunkSnapshot, RC_CHUNK_SNAPSHOT_VERSION, SnapshotError, SnapshotLightSection, decode_snapshot,
    encode_snapshot, peek_snapshot_version,
};
use rc_core::{ChunkKey, DimensionId};

/// Generates a length-`len` `Vec<T>` from a small (32-entry) random pattern tiled with a
/// random offset -- keeps proptest's per-case value-tree cost bounded even though
/// `ChunkSnapshot`'s real field lengths (98304/1536/256) are large.
fn tiled_vec<T: Copy + std::fmt::Debug + 'static>(
    len: usize,
    element: impl Strategy<Value = T>,
) -> impl Strategy<Value = Vec<T>> {
    (proptest::collection::vec(element, 32), any::<usize>()).prop_map(move |(pattern, offset)| {
        (0..len)
            .map(|i| pattern[(i + offset) % pattern.len()])
            .collect()
    })
}

fn light_section_strategy() -> impl Strategy<Value = SnapshotLightSection> {
    (
        proptest::option::of(tiled_vec(2048, any::<u8>())),
        proptest::option::of(tiled_vec(2048, any::<u8>())),
    )
        .prop_map(|(sky, block)| SnapshotLightSection { sky, block })
}

fn chunk_snapshot_strategy() -> impl Strategy<Value = ChunkSnapshot> {
    (
        (any::<i32>(), any::<i32>()),
        tiled_vec(98304, any::<u32>()),
        tiled_vec(1536, any::<u32>()),
        proptest::collection::vec(light_section_strategy(), 26),
        (
            tiled_vec(256, any::<u16>()),
            tiled_vec(256, any::<u16>()),
            tiled_vec(256, any::<u16>()),
            tiled_vec(256, any::<u16>()),
            tiled_vec(256, any::<u16>()),
            tiled_vec(256, any::<u16>()),
        ),
        prop_oneof![Just(0u8), Just(1u8)],
        any::<bool>(),
        any::<u64>(),
    )
        .prop_map(
            |(
                (x, z),
                block_ids,
                biome_ids,
                light_sections,
                (h0, h1, h2, h3, h4, h5),
                gen_status,
                dirty,
                last_saved_tick,
            )| ChunkSnapshot {
                chunk_key: ChunkKey::new(DimensionId::OVERWORLD, x, z),
                block_ids,
                biome_ids,
                light_sections,
                heightmaps: [h0, h1, h2, h3, h4, h5],
                gen_status,
                dirty,
                last_saved_tick,
            },
        )
}

fn sample_snapshot() -> ChunkSnapshot {
    ChunkSnapshot {
        chunk_key: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        block_ids: vec![0u32; 98304],
        biome_ids: vec![0u32; 1536],
        light_sections: vec![SnapshotLightSection::default(); 26],
        heightmaps: [
            vec![0u16; 256],
            vec![0u16; 256],
            vec![0u16; 256],
            vec![0u16; 256],
            vec![0u16; 256],
            vec![0u16; 256],
        ],
        gen_status: 1,
        dirty: false,
        last_saved_tick: 0,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    #[test]
    fn chunk_snapshot_round_trips_through_encode_decode(snapshot in chunk_snapshot_strategy()) {
        let bytes = encode_snapshot(&snapshot);
        let decoded = decode_snapshot(&bytes).expect("a freshly encoded snapshot always decodes");
        prop_assert_eq!(decoded, snapshot);
    }
}

#[test]
fn peek_snapshot_version_reads_without_decoding_the_body() {
    let mut bytes = encode_snapshot(&sample_snapshot());
    for byte in bytes.iter_mut().skip(2) {
        *byte = 0xFF;
    }
    assert_eq!(
        peek_snapshot_version(&bytes).expect("2-byte prefix is always present"),
        RC_CHUNK_SNAPSHOT_VERSION
    );
}

#[test]
fn mismatched_version_is_rejected_without_attempting_a_body_decode() {
    let mut bytes = encode_snapshot(&sample_snapshot());
    bytes[0] = 0x00;
    bytes[1] = 0x63; // 99, an unsupported version -- body bytes untouched.
    let err = decode_snapshot(&bytes).unwrap_err();
    assert!(matches!(
        err,
        SnapshotError::UnsupportedVersion {
            expected: 1,
            found: 99
        }
    ));
}

#[test]
fn truncated_prefix_is_rejected() {
    assert!(matches!(
        peek_snapshot_version(&[0x00]),
        Err(SnapshotError::Truncated)
    ));
    assert!(matches!(
        decode_snapshot(&[]),
        Err(SnapshotError::Truncated)
    ));
}

#[test]
fn dimension_and_chunk_coordinates_round_trip() {
    let mut snapshot = sample_snapshot();
    snapshot.chunk_key = ChunkKey::new(DimensionId::THE_END, -12, 8);
    let bytes = encode_snapshot(&snapshot);
    let decoded = decode_snapshot(&bytes).unwrap();
    assert_eq!(
        decoded.chunk_key,
        ChunkKey::new(DimensionId::THE_END, -12, 8)
    );
}
