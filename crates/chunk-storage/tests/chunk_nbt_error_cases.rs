//! Precisely-triggerable schema violations (M2-B04 Acceptance tests) -- every case
//! asserts the exact `ChunkNbtError` variant via `matches!`, not just `.is_err()`.

mod common;

use bevy_ecs::prelude::Entity;
use common::{codec, thresholds};
use rc_chunk_storage::{
    BiomeColumn, BiomeId, BlockEntityIndex, BlockStateColumn, BlockStateId, ChunkNbtError,
};
use rc_core::DimensionId;
use rc_nbt::owned::{NbtCompound, NbtList, NbtTag};

fn base_compound() -> NbtCompound {
    let fixture = common::all_air_fixture();
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
        .expect("all-air fixture always encodes")
}

fn decode(compound: NbtCompound) -> Result<rc_chunk_storage::ChunkNbtDocument, ChunkNbtError> {
    let bytes = common::encode_bytes(compound);
    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let tag = common::borrow_compound(&nbt);
    codec().from_nbt(&tag, DimensionId::OVERWORLD)
}

/// `ChunkNbtDocument` (deliberately, per the Deliverables) derives no `Debug`, so
/// `Result::unwrap_err` is not available on `decode`'s own return type -- this is the
/// substitute every error-case test below uses instead.
fn expect_err(result: Result<rc_chunk_storage::ChunkNbtDocument, ChunkNbtError>) -> ChunkNbtError {
    match result {
        Ok(_) => panic!("expected from_nbt to fail, but it succeeded"),
        Err(err) => err,
    }
}

fn section_zero_mut(compound: &mut NbtCompound) -> &mut NbtCompound {
    let NbtList::Compound(list) = compound.list_mut("sections").unwrap() else {
        panic!("sections must be a compound list");
    };
    list.iter_mut()
        .find(|section| section.byte("Y") == Some(-4))
        .expect("section Y == -4 must be present")
}

#[test]
fn wrong_data_version_is_rejected() {
    let mut compound = base_compound();
    *compound.int_mut("DataVersion").unwrap() = 4902;
    let err = expect_err(decode(compound));
    assert!(matches!(
        err,
        ChunkNbtError::UnsupportedDataVersion {
            expected: 4903,
            found: 4902
        }
    ));
}

#[test]
fn wrong_ypos_is_rejected() {
    let mut compound = base_compound();
    *compound.int_mut("yPos").unwrap() = -5;
    let err = expect_err(decode(compound));
    assert!(matches!(
        err,
        ChunkNbtError::UnexpectedYPos {
            expected: -4,
            found: -5
        }
    ));
}

#[test]
fn missing_block_section_is_rejected() {
    let mut compound = base_compound();
    let NbtList::Compound(list) = compound.list_mut("sections").unwrap() else {
        panic!("sections must be a compound list");
    };
    list.retain(|section| section.byte("Y") != Some(3));
    let err = expect_err(decode(compound));
    assert!(matches!(err, ChunkNbtError::MissingSection(3)));
}

#[test]
fn non_empty_block_entities_on_save_is_rejected() {
    let fixture = common::all_air_fixture();
    let (_, _) = thresholds();
    let mut block_entities = BlockEntityIndex::new();
    block_entities.push(Entity::from_raw_u32(0).unwrap());

    let err = codec()
        .to_nbt(
            fixture.chunk_key.0,
            &fixture.blocks,
            &fixture.biomes,
            &fixture.light,
            &fixture.heightmaps,
            &block_entities,
            fixture.status,
            fixture.persistence,
            false,
            &[],
        )
        .unwrap_err();
    assert!(matches!(err, ChunkNbtError::UnsupportedBlockEntities(1)));
}

#[test]
fn non_empty_block_entities_on_load_is_rejected() {
    let mut compound = base_compound();
    let dummy = NbtCompound::from_values(vec![("id".into(), NbtTag::String("test:dummy".into()))]);
    *compound.get_mut("block_entities").unwrap() = NbtTag::List(NbtList::Compound(vec![dummy]));
    let err = expect_err(decode(compound));
    assert!(matches!(err, ChunkNbtError::UnsupportedBlockEntities(1)));
}

#[test]
fn out_of_range_local_palette_index_is_rejected() {
    let mut compound = base_compound();
    let palette = NbtList::Compound(vec![
        NbtCompound::from_values(vec![("Name".into(), NbtTag::String("test:air".into()))]),
        NbtCompound::from_values(vec![("Name".into(), NbtTag::String("test:bedrock".into()))]),
    ]);
    let mut locals = vec![0u32; 4096];
    locals[0] = 2; // out of range for a 2-entry palette
    let words = rc_chunk_storage::pack_bits(&locals, 4);
    let data: Vec<i64> = words.iter().map(|&w| w as i64).collect();
    let block_states = NbtCompound::from_values(vec![
        ("palette".into(), NbtTag::List(palette)),
        ("data".into(), NbtTag::LongArray(data)),
    ]);
    *section_zero_mut(&mut compound)
        .get_mut("block_states")
        .unwrap() = NbtTag::Compound(block_states);

    let err = expect_err(decode(compound));
    assert!(matches!(
        err,
        ChunkNbtError::MalformedPalette("block_states", _)
    ));
}

#[test]
fn unresolvable_block_state_name_on_save_is_rejected() {
    let (block_thresholds, biome_thresholds) = thresholds();
    let mut blocks = BlockStateColumn::new(BlockStateId(0), block_thresholds);
    blocks.section_mut(0).set(0, BlockStateId(9999));
    let biomes = BiomeColumn::new(BiomeId(0), biome_thresholds);
    let fixture = common::all_air_fixture();

    let err = codec()
        .to_nbt(
            fixture.chunk_key.0,
            &blocks,
            &biomes,
            &fixture.light,
            &fixture.heightmaps,
            &fixture.block_entities,
            fixture.status,
            fixture.persistence,
            false,
            &[],
        )
        .unwrap_err();
    assert!(matches!(err, ChunkNbtError::UnknownBlockStateName(_)));
}

#[test]
fn unresolvable_block_state_name_on_load_is_rejected() {
    let mut compound = base_compound();
    let palette = NbtList::Compound(vec![NbtCompound::from_values(vec![(
        "Name".into(),
        NbtTag::String("test:does_not_exist".into()),
    )])]);
    let block_states = NbtCompound::from_values(vec![("palette".into(), NbtTag::List(palette))]);
    *section_zero_mut(&mut compound)
        .get_mut("block_states")
        .unwrap() = NbtTag::Compound(block_states);

    let err = expect_err(decode(compound));
    assert!(matches!(err, ChunkNbtError::UnknownBlockStateName(_)));
}
