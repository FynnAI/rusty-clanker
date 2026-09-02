//! test-matrix: boundaries=waived(codec round-trip suite, no world interaction) orientations=waived(codec round-trip suite, no placement) self=waived(codec round-trip suite, no actor) composition=waived(codec round-trip suite, single-record fixtures) nondefault-state=waived(codec round-trip suite, block-entity records carry no block state)
//! M3.5-B05 acceptance tests (Section 4): `ChunkNbtCodec::to_nbt`/`from_nbt` now carry
//! real `BlockEntityRecord`s through the chunk's own `block_entities` list, in on-disk
//! load order, and the Anvil container itself (unchanged by this blueprint) carries the
//! new-shaped payload correctly.

mod common;
mod support;

use common::{codec, superflat_fixture};
use rc_chunk_storage::{
    AnvilDiskBackend, BlockEntityRecord, ChunkStorageBackend, CompressionScheme, RegionFileKind,
};
use rc_core::{BlockPos, DimensionId};
use rc_nbt::owned;
use support::TempWorldDir;

fn record_with_int_field(pos: BlockPos, id: &str, field_value: i32) -> BlockEntityRecord {
    let data = owned::NbtCompound::from_values(vec![
        ("id".into(), owned::NbtTag::String(id.into())),
        ("x".into(), owned::NbtTag::Int(pos.x)),
        ("y".into(), owned::NbtTag::Int(pos.y)),
        ("z".into(), owned::NbtTag::Int(pos.z)),
        ("SomeIntField".into(), owned::NbtTag::Int(field_value)),
    ]);
    BlockEntityRecord {
        pos,
        id: id.to_string(),
        data,
    }
}

fn record_with_string_field(pos: BlockPos, id: &str, field_value: &str) -> BlockEntityRecord {
    let data = owned::NbtCompound::from_values(vec![
        ("id".into(), owned::NbtTag::String(id.into())),
        ("x".into(), owned::NbtTag::Int(pos.x)),
        ("y".into(), owned::NbtTag::Int(pos.y)),
        ("z".into(), owned::NbtTag::Int(pos.z)),
        (
            "SomeStringField".into(),
            owned::NbtTag::String(field_value.into()),
        ),
    ]);
    BlockEntityRecord {
        pos,
        id: id.to_string(),
        data,
    }
}

#[test]
fn chunk_nbt_round_trip_preserves_two_block_entities_in_order() {
    let fixture = superflat_fixture();
    let records = vec![
        record_with_int_field(BlockPos::new(1, -60, 2), "test:one", 42),
        record_with_string_field(BlockPos::new(3, -60, 4), "test:two", "hello"),
    ];

    let compound = codec()
        .to_nbt(
            fixture.chunk_key.0,
            &fixture.blocks,
            &fixture.biomes,
            &fixture.light,
            &fixture.heightmaps,
            &records,
            fixture.status,
            fixture.persistence,
            false,
            &[],
        )
        .expect("a well-formed fixture with two block entities always encodes");
    let bytes = common::encode_bytes(compound);
    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let tag = common::borrow_compound(&nbt);
    let document = codec()
        .from_nbt(&tag, fixture.chunk_key.0.dimension)
        .expect("a document this crate wrote always decodes");

    assert_eq!(document.block_entity_records, records);
}

#[test]
fn chunk_nbt_round_trip_with_zero_block_entities_still_round_trips() {
    let fixture = superflat_fixture();

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
            &[],
        )
        .expect("a well-formed fixture always encodes");
    let bytes = common::encode_bytes(compound);
    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let tag = common::borrow_compound(&nbt);
    let document = codec()
        .from_nbt(&tag, fixture.chunk_key.0.dimension)
        .expect("a document this crate wrote always decodes");

    assert!(document.block_entity_records.is_empty());
}

#[test]
fn anvil_disk_round_trip_preserves_block_entity_records() {
    let dir = TempWorldDir::new("anvil_disk_round_trip_preserves_block_entity_records");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let fixture = superflat_fixture();
    let records = vec![
        record_with_int_field(BlockPos::new(5, -60, 6), "test:one", 7),
        record_with_string_field(BlockPos::new(8, -60, 9), "test:two", "world"),
    ];

    let compound = codec()
        .to_nbt(
            fixture.chunk_key.0,
            &fixture.blocks,
            &fixture.biomes,
            &fixture.light,
            &fixture.heightmaps,
            &records,
            fixture.status,
            fixture.persistence,
            false,
            &[],
        )
        .expect("a well-formed fixture with two block entities always encodes");
    let bytes = common::encode_bytes(compound);

    backend
        .write_chunk(
            DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            0,
            0,
            &bytes,
            None,
        )
        .unwrap();

    let read_back = backend
        .read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0, None)
        .unwrap()
        .expect("just-written chunk must read back");

    let nbt = rc_nbt::read_borrowed_strict(&read_back).unwrap();
    let tag = common::borrow_compound(&nbt);
    let document = codec()
        .from_nbt(&tag, DimensionId::OVERWORLD)
        .expect("a document this crate wrote always decodes");

    assert_eq!(document.block_entity_records, records);
}
