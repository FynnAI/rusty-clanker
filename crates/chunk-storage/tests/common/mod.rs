//! Shared test-only fixtures for `M2-B04`'s acceptance-test suite (Rust's own
//! `tests/common/mod.rs` convention -- not a standalone test binary).
//!
//! Not every helper here is used by every test binary that declares `mod common;` --
//! each integration-test file gets its own compiled copy of this module, so a helper
//! used by one binary and not another is expected, not dead code.
#![allow(dead_code)]

use rc_chunk_storage::{
    BiomeColumn, BiomeId, BiomeNames, BlockEntityRecord, BlockStateColumn, BlockStateId,
    BlockStateNames, ChunkGenStatus, ChunkKeyTag, ChunkNbtCodec, ChunkPersistenceState,
    ChunkStatus, HeightmapSet, LightColumn, PaletteThresholds, WORLD_MIN_Y,
};
use rc_core::{ChunkKey, DimensionId};
use rc_nbt::{Mutf8Str, Mutf8String, owned};

// Synthetic, hand-authored test-only names -- never real Mojang data (Context).
pub struct MockBlockNames;

impl BlockStateNames for MockBlockNames {
    // id 0 = "test:air" (no properties); id 1 = "test:bedrock" (no properties);
    // id 2 = "test:dirt" (no properties); id 3 = "test:grass_block" (no properties) --
    // mirroring M1-B05's own four-block superflat set, renamed into this crate's own
    // test namespace since it must not depend on rc-protocol (Context).
    // id 4 = "test:door", properties returned in a DELIBERATELY non-alphabetical
    // order -- {"open": "false", "facing": "north", "half": "lower"} -- to prove
    // to_nbt's own sort-before-write rule (Context's "Property-compound ordering").
    // ids 100..500 (400 values) = "test:distinct_<id>" (no properties) -- the palette
    // this blueprint's >256-distinct-values-in-one-section test uses.
    // every other id -> None.
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)> {
        match id.0 {
            0 => Some((Mutf8String::from("test:air"), vec![])),
            1 => Some((Mutf8String::from("test:bedrock"), vec![])),
            2 => Some((Mutf8String::from("test:dirt"), vec![])),
            3 => Some((Mutf8String::from("test:grass_block"), vec![])),
            4 => Some((
                Mutf8String::from("test:door"),
                vec![
                    (Mutf8String::from("open"), Mutf8String::from("false")),
                    (Mutf8String::from("facing"), Mutf8String::from("north")),
                    (Mutf8String::from("half"), Mutf8String::from("lower")),
                ],
            )),
            n @ 100..=499 => Some((Mutf8String::from(format!("test:distinct_{n}")), vec![])),
            _ => None,
        }
    }

    fn resolve(
        &self,
        name: &Mutf8Str,
        _properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId> {
        let name = name.to_str();
        match name.as_ref() {
            "test:air" => Some(BlockStateId(0)),
            "test:bedrock" => Some(BlockStateId(1)),
            "test:dirt" => Some(BlockStateId(2)),
            "test:grass_block" => Some(BlockStateId(3)),
            "test:door" => Some(BlockStateId(4)),
            other => other
                .strip_prefix("test:distinct_")
                .and_then(|n| n.parse::<u32>().ok())
                .filter(|&n| (100..500).contains(&n))
                .map(BlockStateId),
        }
    }
}

// 0 = "test:plains", 1 = "test:desert" (a second synthetic biome for the
// `biome_palette_entries_are_plain_strings_not_compounds` edge case), else `None`.
pub struct MockBiomeNames;

impl BiomeNames for MockBiomeNames {
    fn name(&self, id: BiomeId) -> Option<Mutf8String> {
        match id.0 {
            0 => Some(Mutf8String::from("test:plains")),
            1 => Some(Mutf8String::from("test:desert")),
            _ => None,
        }
    }

    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId> {
        match name.to_str().as_ref() {
            "test:plains" => Some(BiomeId(0)),
            "test:desert" => Some(BiomeId(1)),
            _ => None,
        }
    }
}

pub fn thresholds() -> (PaletteThresholds, PaletteThresholds) {
    (PaletteThresholds::blocks(15), PaletteThresholds::biomes(4))
}

pub const BLOCK_NAMES: MockBlockNames = MockBlockNames;
pub const BIOME_NAMES: MockBiomeNames = MockBiomeNames;

/// A fresh `ChunkNbtCodec` over the shared mock resolvers and the shared "real-shaped"
/// `thresholds()`, one per call (cheap -- only borrows and `Copy` values, per the
/// Deliverables' own doc comment).
pub fn codec() -> ChunkNbtCodec<'static, MockBlockNames, MockBiomeNames> {
    let (block_thresholds, biome_thresholds) = thresholds();
    ChunkNbtCodec {
        block_names: &BLOCK_NAMES,
        biome_names: &BIOME_NAMES,
        block_thresholds,
        biome_thresholds,
    }
}

/// Wraps a freshly built `to_nbt` compound as an unnamed root, writes it, and reads it
/// back via `rc_nbt::read_owned` -- the round-trip most of this blueprint's schema
/// assertions drive off of.
pub fn write_then_read_owned(compound: owned::NbtCompound) -> owned::Nbt {
    let bytes = encode_bytes(compound);
    rc_nbt::read_owned(&bytes).expect("a freshly encoded chunk compound always decodes")
}

/// Wraps a freshly built `to_nbt` compound as an unnamed root and writes it -- the raw
/// bytes `ChunkNbtCodec::from_nbt` (which needs a zero-copy `borrow::NbtCompound`) reads
/// back from.
pub fn encode_bytes(compound: owned::NbtCompound) -> Vec<u8> {
    let base = owned::BaseNbt::from(compound);
    rc_nbt::write_owned(&base)
}

/// Unwraps a `borrow::Nbt`'s root compound, panicking on `Nbt::None` (every document
/// this blueprint's own encoder produces is always `Some`).
pub fn borrow_compound<'a, 'tape>(
    nbt: &'a rc_nbt::borrow::Nbt<'a>,
) -> rc_nbt::borrow::NbtCompound<'a, 'tape> {
    match nbt {
        rc_nbt::borrow::Nbt::Some(base) => base.as_compound(),
        rc_nbt::borrow::Nbt::None => panic!("expected a decoded document, found Nbt::None"),
    }
}

/// A fully populated, self-consistent set of the seven M2-B01 data components plus a
/// `ChunkKeyTag`, matching M1-B05's own superflat layer content (bedrock/dirt/grass/air
/// in section 0, air everywhere else, single "test:plains" biome), status `Full`,
/// `dirty: false`, `last_saved_tick: 0`. Returned as a struct the round-trip tests
/// destructure and compare field-by-field against a decoded `ChunkNbtDocument`.
pub struct Fixture {
    pub chunk_key: ChunkKeyTag,
    pub blocks: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entity_records: Vec<BlockEntityRecord>,
    pub status: ChunkStatus,
    pub persistence: ChunkPersistenceState,
}

/// M1-B05's own layer table (Context's `common::superflat_fixture` doc comment):
/// bedrock at `y == -64`, dirt at `y in -63..=-61`, grass at `y == -60`, air elsewhere.
pub fn superflat_fixture() -> Fixture {
    superflat_fixture_at(ChunkKey::new(DimensionId::OVERWORLD, 0, 0))
}

pub fn superflat_fixture_at(key: ChunkKey) -> Fixture {
    let (block_thresholds, biome_thresholds) = thresholds();
    let mut blocks = BlockStateColumn::new(BlockStateId(0), block_thresholds);
    for local_y in 0..16i32 {
        let world_y = WORLD_MIN_Y + local_y;
        let block = match world_y {
            -64 => BlockStateId(1),
            -63..=-61 => BlockStateId(2),
            -60 => BlockStateId(3),
            _ => BlockStateId(0),
        };
        if block.0 != 0 {
            for z in 0u8..16 {
                for x in 0u8..16 {
                    blocks.set(x, world_y, z, block);
                }
            }
        }
    }
    let biomes = BiomeColumn::new(BiomeId(0), biome_thresholds);
    let light = LightColumn::new_uninitialized();
    // First air Y is one above the topmost real block (the grass layer at y == -60):
    // every one of the six heightmap types stays in lockstep (M2-B01's own
    // `note_block_change` invariant), so `new_uniform` is exact, not an approximation.
    let heightmaps = HeightmapSet::new_uniform(-59);
    Fixture {
        chunk_key: ChunkKeyTag(key),
        blocks,
        biomes,
        light,
        heightmaps,
        block_entity_records: Vec::new(),
        status: ChunkStatus(ChunkGenStatus::Full),
        persistence: ChunkPersistenceState {
            dirty: false,
            last_saved_tick: 0,
        },
    }
}

/// Every section `SingleValue` (air / plains, never `set`), all light sections
/// `None`/`None`. Used by both `chunk_nbt_schema.rs`'s and `chunk_nbt_roundtrip.rs`'s
/// own all-air cases (factored here so neither duplicates the construction).
pub fn all_air_fixture() -> Fixture {
    all_air_fixture_at(ChunkKey::new(DimensionId::OVERWORLD, 0, 0))
}

pub fn all_air_fixture_at(key: ChunkKey) -> Fixture {
    let (block_thresholds, biome_thresholds) = thresholds();
    let blocks = BlockStateColumn::new(BlockStateId(0), block_thresholds);
    let biomes = BiomeColumn::new(BiomeId(0), biome_thresholds);
    let light = LightColumn::new_uninitialized();
    let heightmaps = HeightmapSet::new_uniform(WORLD_MIN_Y);
    Fixture {
        chunk_key: ChunkKeyTag(key),
        blocks,
        biomes,
        light,
        heightmaps,
        block_entity_records: Vec::new(),
        status: ChunkStatus(ChunkGenStatus::Full),
        persistence: ChunkPersistenceState {
            dirty: false,
            last_saved_tick: 0,
        },
    }
}
