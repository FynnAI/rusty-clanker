//! Palette-shape edge cases (M2-B04 Deliverables/Context: "on-disk paletted-container
//! encoding" and "Property-compound ordering").

mod common;

use common::{codec, thresholds};
use rc_chunk_storage::{
    BiomeColumn, BiomeId, BlockEntityIndex, BlockStateColumn, BlockStateId, ChunkGenStatus,
    ChunkKeyTag, ChunkPersistenceState, ChunkStatus, HeightmapSet, LightColumn, WORLD_MIN_Y,
};
use rc_core::{ChunkKey, DimensionId};

fn fixture_with_block_section(setup: impl FnOnce(&mut BlockStateColumn)) -> common::Fixture {
    let (block_thresholds, biome_thresholds) = thresholds();
    let mut blocks = BlockStateColumn::new(BlockStateId(0), block_thresholds);
    setup(&mut blocks);
    common::Fixture {
        chunk_key: ChunkKeyTag(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)),
        blocks,
        biomes: BiomeColumn::new(BiomeId(0), biome_thresholds),
        light: LightColumn::new_uninitialized(),
        heightmaps: HeightmapSet::new_uniform(WORLD_MIN_Y),
        block_entities: BlockEntityIndex::new(),
        status: ChunkStatus(ChunkGenStatus::Full),
        persistence: ChunkPersistenceState {
            dirty: false,
            last_saved_tick: 0,
        },
    }
}

fn fixture_with_biome_section(setup: impl FnOnce(&mut BiomeColumn)) -> common::Fixture {
    let (block_thresholds, biome_thresholds) = thresholds();
    let mut biomes = BiomeColumn::new(BiomeId(0), biome_thresholds);
    setup(&mut biomes);
    common::Fixture {
        chunk_key: ChunkKeyTag(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)),
        blocks: BlockStateColumn::new(BlockStateId(0), block_thresholds),
        biomes,
        light: LightColumn::new_uninitialized(),
        heightmaps: HeightmapSet::new_uniform(WORLD_MIN_Y),
        block_entities: BlockEntityIndex::new(),
        status: ChunkStatus(ChunkGenStatus::Full),
        persistence: ChunkPersistenceState {
            dirty: false,
            last_saved_tick: 0,
        },
    }
}

fn encode(fixture: &common::Fixture) -> rc_nbt::owned::NbtCompound {
    codec()
        .to_nbt(
            fixture.chunk_key.0,
            &fixture.blocks,
            &fixture.biomes,
            &fixture.light,
            &fixture.heightmaps,
            &fixture.block_entities,
            fixture.status,
            fixture.persistence,
            false,
            &[],
        )
        .expect("fixture always encodes")
}

#[test]
fn property_bearing_block_writes_properties_sorted_alphabetically() {
    let fixture = fixture_with_block_section(|blocks| {
        blocks.section_mut(0).set(0, BlockStateId(4)); // test:door
    });
    let nbt = common::write_then_read_owned(encode(&fixture));

    let sections = nbt.list("sections").unwrap().compounds().unwrap();
    let section_zero = sections
        .iter()
        .find(|s| s.byte("Y").unwrap() == -4)
        .unwrap();
    let palette = section_zero
        .compound("block_states")
        .unwrap()
        .list("palette")
        .unwrap()
        .compounds()
        .unwrap();
    let door_entry = palette
        .iter()
        .find(|entry| entry.string("Name").unwrap().to_str().as_ref() == "test:door")
        .expect("test:door must be in the palette");
    let properties = door_entry.compound("Properties").unwrap();
    let keys: Vec<String> = properties.keys().map(|k| k.to_str().into_owned()).collect();
    assert_eq!(keys, vec!["facing", "half", "open"]);
}

#[test]
fn zero_property_block_omits_properties_tag_entirely() {
    let fixture = common::all_air_fixture();
    let nbt = common::write_then_read_owned(encode(&fixture));

    let sections = nbt.list("sections").unwrap().compounds().unwrap();
    let section_zero = sections
        .iter()
        .find(|s| s.byte("Y").unwrap() == -4)
        .unwrap();
    let palette = section_zero
        .compound("block_states")
        .unwrap()
        .list("palette")
        .unwrap()
        .compounds()
        .unwrap();
    assert_eq!(palette.len(), 1);
    assert!(palette[0].compound("Properties").is_none());
}

#[test]
fn single_value_section_omits_data_array_for_both_blocks_and_biomes() {
    let fixture = common::all_air_fixture();
    let nbt = common::write_then_read_owned(encode(&fixture));

    let sections = nbt.list("sections").unwrap().compounds().unwrap();
    let section_zero = sections
        .iter()
        .find(|s| s.byte("Y").unwrap() == -4)
        .unwrap();
    assert!(
        section_zero
            .compound("block_states")
            .unwrap()
            .get("data")
            .is_none()
    );
    assert!(
        section_zero
            .compound("biomes")
            .unwrap()
            .get("data")
            .is_none()
    );
}

#[test]
fn biome_palette_entries_are_plain_strings_not_compounds() {
    let fixture = fixture_with_biome_section(|biomes| {
        biomes.section_mut(0).set(0, BiomeId(1)); // test:desert
    });
    let nbt = common::write_then_read_owned(encode(&fixture));

    let sections = nbt.list("sections").unwrap().compounds().unwrap();
    let section_zero = sections
        .iter()
        .find(|s| s.byte("Y").unwrap() == -4)
        .unwrap();
    let palette_list = section_zero
        .compound("biomes")
        .unwrap()
        .list("palette")
        .unwrap();
    let strings = palette_list
        .strings()
        .expect("biome palette is a string list");
    assert_eq!(strings.len(), 2);
    assert!(palette_list.compounds().is_none());
}

#[test]
fn over_256_distinct_block_states_in_one_section_forces_wider_on_disk_packing_than_the_in_memory_container_uses()
 {
    let fixture = fixture_with_block_section(|blocks| {
        let section = blocks.section_mut(0);
        for i in 0..300u32 {
            section.set(i as usize, BlockStateId(100 + i));
        }
    });
    assert_eq!(
        fixture.blocks.section(0).bits_per_entry(),
        15,
        "sanity check: the in-memory container must be in wire-Direct mode at 15 bits"
    );

    let nbt = common::write_then_read_owned(encode(&fixture));
    let sections = nbt.list("sections").unwrap().compounds().unwrap();
    let section_zero = sections
        .iter()
        .find(|s| s.byte("Y").unwrap() == -4)
        .unwrap();
    let block_states = section_zero.compound("block_states").unwrap();
    let palette = block_states.list("palette").unwrap().compounds().unwrap();
    assert_eq!(palette.len(), 301);

    let data = block_states.long_array("data").unwrap();
    let recovered_bits = (1u32..=15)
        .find(|&bits| {
            let entries_per_long = (64 / bits) as usize;
            4096usize.div_ceil(entries_per_long) == data.len()
        })
        .expect("some bit width must reproduce the observed word count");
    assert_eq!(
        recovered_bits, 9,
        "ceil_log2(301) == 9, not the in-memory container's own 15"
    );

    let words: Vec<u64> = data.iter().map(|&v| v as u64).collect();
    let locals = rc_chunk_storage::unpack_bits(&words, recovered_bits, 4096);
    let names: Vec<String> = palette
        .iter()
        .map(|entry| entry.string("Name").unwrap().to_str().into_owned())
        .collect();
    for i in 0..300usize {
        let expected = format!("test:distinct_{}", 100 + i);
        assert_eq!(names[locals[i] as usize], expected, "cell {i} mismatch");
    }
    for i in 300..4096usize {
        assert_eq!(
            names[locals[i] as usize], "test:air",
            "cell {i} should still be air"
        );
    }
}

#[test]
fn same_section_encoded_twice_produces_byte_identical_output() {
    let fixture = fixture_with_block_section(|blocks| {
        let section = blocks.section_mut(0);
        for i in 0..300u32 {
            section.set(i as usize, BlockStateId(100 + i));
        }
    });
    let bytes_a = common::encode_bytes(encode(&fixture));
    let bytes_b = common::encode_bytes(encode(&fixture));
    assert_eq!(bytes_a, bytes_b);
}
