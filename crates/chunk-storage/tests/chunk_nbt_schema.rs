//! Field-by-field NBT schema assertions (M2-B04 Deliverables/Context).

mod common;

use common::{all_air_fixture, codec, superflat_fixture};
use rc_chunk_storage::MIN_SECTION_Y;
use rc_core::{ChunkKey, DimensionId};

#[test]
fn all_air_chunk_has_24_uniform_sections_and_no_data_arrays() {
    let fixture = all_air_fixture();
    let compound = codec()
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
        .expect("all-air fixture always encodes");
    let nbt = common::write_then_read_owned(compound);

    assert_eq!(nbt.int("DataVersion"), Some(4903));
    assert_eq!(nbt.int("xPos"), Some(fixture.chunk_key.0.x));
    assert_eq!(nbt.int("zPos"), Some(fixture.chunk_key.0.z));
    assert_eq!(nbt.int("yPos"), Some(MIN_SECTION_Y));
    assert_eq!(
        nbt.string("Status").unwrap().to_str().as_ref(),
        "minecraft:full"
    );
    assert!(nbt.get("isLightOn").is_none());

    let sections = nbt.list("sections").unwrap().compounds().unwrap();
    assert_eq!(sections.len(), 24);
    for section in sections {
        let y = section.byte("Y").unwrap() as i32;
        assert!((-4..=19).contains(&y));

        let block_states = section.compound("block_states").unwrap();
        let block_palette = block_states.list("palette").unwrap().compounds().unwrap();
        assert_eq!(block_palette.len(), 1);
        assert_eq!(
            block_palette[0].string("Name").unwrap().to_str().as_ref(),
            "test:air"
        );
        assert!(block_palette[0].get("Properties").is_none());
        assert!(block_states.get("data").is_none());

        let biomes = section.compound("biomes").unwrap();
        let biome_palette = biomes.list("palette").unwrap().strings().unwrap();
        assert_eq!(biome_palette.len(), 1);
        assert_eq!(biome_palette[0].to_str().as_ref(), "test:plains");
        assert!(biomes.get("data").is_none());
    }

    let heightmaps = nbt.compound("Heightmaps").unwrap();
    assert_eq!(heightmaps.len(), 4);
    for key in [
        "WORLD_SURFACE",
        "OCEAN_FLOOR",
        "MOTION_BLOCKING",
        "MOTION_BLOCKING_NO_LEAVES",
    ] {
        assert_eq!(heightmaps.long_array(key).unwrap().len(), 37);
    }
    assert!(heightmaps.get("WORLD_SURFACE_WG").is_none());
    assert!(heightmaps.get("OCEAN_FLOOR_WG").is_none());

    assert!(
        nbt.list("block_entities")
            .unwrap()
            .compounds()
            .unwrap()
            .is_empty()
    );
    assert_eq!(nbt.long("LastUpdate"), Some(0));
    assert_eq!(nbt.long("InhabitedTime"), Some(0));

    assert!(nbt.get("structures").is_some());
    assert!(nbt.get("block_ticks").is_some());
    assert!(nbt.get("fluid_ticks").is_some());
    assert!(nbt.get("PostProcessing").is_some());
}

#[test]
fn superflat_section_zero_matches_a_hand_computed_indirect_palette() {
    let fixture = superflat_fixture();
    let compound = codec()
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
        .expect("superflat fixture always encodes");
    let nbt = common::write_then_read_owned(compound);

    let sections = nbt.list("sections").unwrap().compounds().unwrap();
    let section_zero = sections
        .iter()
        .find(|s| s.byte("Y").unwrap() == -4)
        .expect("section Y == -4 must be present");

    let block_states = section_zero.compound("block_states").unwrap();
    let palette = block_states.list("palette").unwrap().compounds().unwrap();
    let names: Vec<String> = palette
        .iter()
        .map(|entry| entry.string("Name").unwrap().to_str().into_owned())
        .collect();
    assert_eq!(
        names,
        vec!["test:bedrock", "test:dirt", "test:grass_block", "test:air"]
    );
    assert_eq!(palette.len(), 4);

    let data = block_states.long_array("data").unwrap();
    // `ceil_log2(4) == 2`, floor-bumped to `max(4, 2) == 4` (Context's on-disk floor).
    let bits_per_entry = 4u32;
    let entries_per_long = (64 / bits_per_entry) as usize;
    assert_eq!(data.len(), 4096usize.div_ceil(entries_per_long));

    let words: Vec<u64> = data.iter().map(|&v| v as u64).collect();
    let locals = rc_chunk_storage::unpack_bits(&words, bits_per_entry, 4096);
    for local_y in 0..16i32 {
        let world_y = rc_chunk_storage::WORLD_MIN_Y + local_y;
        let expected_name = match world_y {
            -64 => "test:bedrock",
            -63..=-61 => "test:dirt",
            -60 => "test:grass_block",
            _ => "test:air",
        };
        let expected_index = names.iter().position(|n| n == expected_name).unwrap();
        for z in 0..16usize {
            for x in 0..16usize {
                let index = (local_y as usize) * 256 + z * 16 + x;
                assert_eq!(
                    locals[index] as usize, expected_index,
                    "cell ({x}, {local_y}, {z}) mismatch"
                );
            }
        }
    }
}

#[test]
fn dimension_and_ypos_round_trip_through_from_nbt() {
    let key = ChunkKey::new(DimensionId::THE_NETHER, 7, -3);
    let fixture = common::superflat_fixture_at(key);
    let compound = codec()
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
        .expect("superflat fixture always encodes");
    let bytes = common::encode_bytes(compound);
    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let tag = common::borrow_compound(&nbt);

    let document = codec()
        .from_nbt(&tag, DimensionId::THE_NETHER)
        .expect("a document this crate wrote always decodes");
    assert_eq!(
        document.chunk_key.0,
        ChunkKey::new(DimensionId::THE_NETHER, 7, -3)
    );
}

#[test]
fn wrong_dimension_argument_produces_a_different_key_not_an_error() {
    let key = ChunkKey::new(DimensionId::THE_NETHER, 7, -3);
    let fixture = common::superflat_fixture_at(key);
    let compound = codec()
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
        .expect("superflat fixture always encodes");
    let bytes = common::encode_bytes(compound);
    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let tag = common::borrow_compound(&nbt);

    let document = codec()
        .from_nbt(&tag, DimensionId::OVERWORLD)
        .expect("dimension is never validated by this crate");
    assert_eq!(document.chunk_key.0.dimension, DimensionId::OVERWORLD);
    assert_eq!(document.chunk_key.0.x, 7);
    assert_eq!(document.chunk_key.0.z, -3);
}
