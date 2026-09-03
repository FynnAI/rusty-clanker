//! The byte-identity guarantee (M2-B04 Deliverables/Context): `chunk_from_nbt(chunk_to_nbt(
//! components, extra: &[]))` reproduces every M2-B01 component exactly.

mod common;

use common::{codec, superflat_fixture};
use proptest::prelude::*;
use rc_chunk_storage::{
    BiomeColumn, BlockStateColumn, ChunkGenStatus, ChunkNbtDocument, ChunkPersistenceState,
    ChunkStatus, HeightmapKind, HeightmapSet, LightColumn, LightNibbles, SECTION_COUNT,
    WORLD_HEIGHT, WORLD_MIN_Y,
};
use rc_nbt::{Mutf8String, owned::NbtTag};

fn assert_blocks_equal(a: &BlockStateColumn, b: &BlockStateColumn) {
    for world_y in WORLD_MIN_Y..WORLD_MIN_Y + WORLD_HEIGHT {
        for z in 0u8..16 {
            for x in 0u8..16 {
                assert_eq!(
                    a.get(x, world_y, z),
                    b.get(x, world_y, z),
                    "block mismatch at ({x}, {world_y}, {z})"
                );
            }
        }
    }
}

fn assert_biomes_equal(a: &BiomeColumn, b: &BiomeColumn) {
    for section in 0..SECTION_COUNT {
        for local_qy in 0i32..4 {
            let world_y = WORLD_MIN_Y + (section as i32) * 16 + local_qy * 4;
            for qz in 0u8..4 {
                for qx in 0u8..4 {
                    assert_eq!(
                        a.get(qx, world_y, qz),
                        b.get(qx, world_y, qz),
                        "biome mismatch at ({qx}, {world_y}, {qz})"
                    );
                }
            }
        }
    }
}

fn assert_light_equal(a: &LightColumn, b: &LightColumn) {
    for i in 0..rc_chunk_storage::LIGHT_SECTION_COUNT {
        assert_eq!(
            a.section(i).sky,
            b.section(i).sky,
            "sky light mismatch at light index {i}"
        );
        assert_eq!(
            a.section(i).block,
            b.section(i).block,
            "block light mismatch at light index {i}"
        );
    }
}

fn assert_heightmaps_equal(a: &HeightmapSet, b: &HeightmapSet) {
    for kind in HeightmapKind::ALL {
        for z in 0u8..16 {
            for x in 0u8..16 {
                assert_eq!(
                    a.world_y(kind, x, z),
                    b.world_y(kind, x, z),
                    "heightmap {kind:?} mismatch at ({x}, {z})"
                );
            }
        }
    }
}

fn round_trip(fixture: &common::Fixture, extra: &[(Mutf8String, NbtTag)]) -> ChunkNbtDocument {
    let compound = codec()
        .to_nbt(
            fixture.chunk_key.0,
            &fixture.blocks,
            &fixture.biomes,
            &fixture.light,
            &fixture.heightmaps,
            &fixture.block_entity_records,
            fixture.status,
            fixture.persistence,
            false,
            extra,
        )
        .expect("a well-formed fixture always encodes");
    let bytes = common::encode_bytes(compound);
    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let tag = common::borrow_compound(&nbt);
    codec()
        .from_nbt(&tag, fixture.chunk_key.0.dimension)
        .expect("a document this crate wrote always decodes")
}

fn assert_document_matches_fixture(document: &ChunkNbtDocument, fixture: &common::Fixture) {
    assert_blocks_equal(&fixture.blocks, &document.blocks);
    assert_biomes_equal(&fixture.biomes, &document.biomes);
    assert_light_equal(&fixture.light, &document.light);
    assert_heightmaps_equal(&fixture.heightmaps, &document.heightmaps);
    assert!(document.block_entity_records.is_empty());
    assert_eq!(document.status, ChunkStatus(ChunkGenStatus::Full));
    assert_eq!(
        document.persistence,
        ChunkPersistenceState {
            dirty: false,
            last_saved_tick: 0,
        }
    );
}

#[test]
fn load_of_save_reproduces_every_component_for_the_superflat_fixture() {
    let fixture = superflat_fixture();
    let document = round_trip(&fixture, &[]);
    assert_document_matches_fixture(&document, &fixture);
}

#[test]
fn load_of_save_round_trips_for_an_all_air_uninhabited_chunk() {
    let fixture = common::all_air_fixture();
    let document = round_trip(&fixture, &[]);
    assert_document_matches_fixture(&document, &fixture);
}

/// `None` maps to `LightNibbles::Uninitialized` (WORLD-D8's own "not yet initialized"
/// shortcut, the un-amended type's exact `None` case); `Some(bytes)` maps to
/// `LightNibbles::Data(bytes)` (a fully materialized array, the un-amended type's
/// exact `Some` case). `Filled(v)` is deliberately not exercised by this proptest --
/// nothing in this crate's own on-disk NBT writer produces it yet (Context, M2
/// field-report ledger entry); it is `chunk_nbt.rs`'s own `materialized_nibbles`
/// helper that decides how a `Filled` value would round-trip, exercised directly by
/// that module's own unit coverage instead.
fn light_section_strategy() -> impl Strategy<Value = (Option<Vec<u8>>, Option<Vec<u8>>)> {
    (
        proptest::option::of(proptest::collection::vec(any::<u8>(), 2048)),
        proptest::option::of(proptest::collection::vec(any::<u8>(), 2048)),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    #[test]
    fn load_of_save_round_trips_with_partial_light_data(
        sections in proptest::collection::vec(light_section_strategy(), rc_chunk_storage::LIGHT_SECTION_COUNT),
    ) {
        let mut fixture = superflat_fixture();
        for (index, (sky, block)) in sections.into_iter().enumerate() {
            let section = fixture.light.section_mut(index);
            section.sky = sky
                .map(|bytes| LightNibbles::Data(Box::new(<[u8; 2048]>::try_from(bytes).unwrap())))
                .unwrap_or(LightNibbles::Uninitialized);
            section.block = block
                .map(|bytes| LightNibbles::Data(Box::new(<[u8; 2048]>::try_from(bytes).unwrap())))
                .unwrap_or(LightNibbles::Uninitialized);
        }
        let document = round_trip(&fixture, &[]);
        assert_light_equal(&fixture.light, &document.light);
    }
}

#[test]
fn extra_fields_round_trip_when_present_on_reload() {
    let fixture = superflat_fixture();
    let extra = vec![(Mutf8String::from("custom_test_tag"), NbtTag::Int(42))];

    let document = round_trip(&fixture, &extra);
    assert_eq!(document.extra, extra);

    // Idempotent: re-supplying the loaded `extra` reproduces it a second time.
    let document_again = round_trip(&fixture, &document.extra);
    assert_eq!(document_again.extra, extra);
}
