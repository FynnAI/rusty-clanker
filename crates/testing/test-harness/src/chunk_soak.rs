//! M2-B08's deterministic chunk write/read soak primitive (Acceptance Criterion 2,
//! `11-roadmap-milestones.md`: "10,000 synthetic chunk write/read round trips with zero
//! checksum mismatches"). See the owning blueprint's Context, "Deterministic chunk-
//! content generation for the 10,000-chunk soak".
//!
//! Forced deviation from the blueprint's own Deliverables sketch: `generate_chunk_payload`
//! does **not** go through `rc_chunk_storage::ChunkNbtCodec`/`BlockStateNames` (the real
//! vanilla chunk-NBT schema, M2-B04) — that codec requires a caller-supplied name
//! resolver capable of naming *every* `BlockStateId` a generated payload might use, and
//! this soak deliberately drives the palette up to 200 (`Indirect`) and 257+ (`Direct`)
//! distinct synthetic ids per section to exercise every `Palette<T>` shape (Context) —
//! far more ids than the one real resolver committed anywhere in this workspace
//! (`crates/server/src/play/registry_resolvers.rs`, private to that crate and closed over
//! exactly the five block states M2's own real content ever needs) can name. This module
//! is a storage-layer soak, not a chunk-schema soak (a real `AnvilDiskBackend::read_chunk`
//! only ever validates NBT *well-formedness*, `rc_nbt::read_borrowed_strict` — it never
//! decodes chunk semantics, Context's own primitive doc comment: "this primitive does not
//! decode NBT at all"), so this module builds its own small, self-contained, valid NBT
//! envelope directly from `rc_chunk_storage`'s already-public `PalettedContainer`/
//! `BlockStateColumn` types — real palette-shape data, real bit-packing, never resolved
//! to or from a block name anywhere in this file.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rc_chunk_storage::{
    BlockStateColumn, BlockStateId, ChunkStorageBackend, Palette, PaletteThresholds,
    RegionFileKind, SECTION_COUNT,
};
use rc_nbt::{Mutf8String, owned};

/// One soak trial's outcome.
#[derive(Debug, Clone)]
pub struct SoakCaseOutcome {
    pub index: u32,
    pub palette_shape: PaletteShape,
    pub round_trip_identical: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaletteShape {
    SingleValue,
    Indirect,
    Direct,
}

#[derive(Debug, Clone)]
pub struct SoakReport {
    pub seed: u64,
    pub total: u32,
    pub mismatches: Vec<SoakCaseOutcome>,
}

impl SoakReport {
    pub fn zero_mismatches(&self) -> bool {
        self.mismatches.is_empty()
    }
}

/// `index % 3` (Context: `0` -> every section `SingleValue`, `1` -> every section
/// `Indirect`, `2` -> at least one section forced `Direct`) — exposed independently of
/// `generate_chunk_payload` so a caller/report can attribute a mismatch to a specific
/// palette shape rather than only an opaque index.
pub fn palette_shape_for(index: u32) -> PaletteShape {
    match index % 3 {
        0 => PaletteShape::SingleValue,
        1 => PaletteShape::Indirect,
        _ => PaletteShape::Direct,
    }
}

/// The pinned target's own block-state registry size (`registry_id.rs`'s own doc
/// comment: "for the pinned DataVersion 4903 target... 32366 block states, i.e.
/// `direct_bits = 15`") — this soak's own synthetic ids stay well inside that range
/// regardless of shape, matching a real registry's own numeric bounds even though no
/// name is ever resolved for them.
const BLOCK_THRESHOLDS: PaletteThresholds = PaletteThresholds::blocks(15);

/// A small, dependency-free `[min, max]` inclusive range draw off a raw `RngCore` —
/// avoids pulling in the `rand` crate's own `Rng::gen_range` purely for this (Constraint
/// (c): `rand_chacha` is this blueprint's one permitted new external dependency).
fn next_range(rng: &mut ChaCha8Rng, min: u32, max_inclusive: u32) -> u32 {
    debug_assert!(max_inclusive >= min);
    let span = max_inclusive - min + 1;
    min + (rng.next_u32() % span)
}

/// Builds one deterministic `BlockStateColumn` per `(seed, index)` and `shape`
/// (Context's exact palette-cycling rule): `SingleValue` -> every section a single,
/// `index`-derived id (constructed directly, never via `.set()`, so the container's own
/// `Palette` stays genuinely `SingleValue` rather than upgrading on first write);
/// `Indirect` -> every section 2..=200 distinct ids scattered across its 4096 entries;
/// `Direct` -> every section >=257 distinct ids, forcing promotion past
/// `PaletteThresholds::blocks(15)`'s 256-entry (`max_indirect_bits = 8`) `Indirect`
/// ceiling.
fn build_column(seed: u64, index: u32, shape: PaletteShape) -> BlockStateColumn {
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ (index as u64));
    let mut column = BlockStateColumn::new(BlockStateId(0), BLOCK_THRESHOLDS);

    for section_index in 0..SECTION_COUNT {
        match shape {
            PaletteShape::SingleValue => {
                let value = BlockStateId(1000 + index.wrapping_mul(7) + section_index as u32);
                *column.section_mut(section_index) =
                    rc_chunk_storage::PalettedContainer::new_single(
                        value,
                        rc_chunk_storage::SECTION_BLOCKS,
                        BLOCK_THRESHOLDS,
                    );
            }
            // Indirect/Direct: the container already starts `SingleValue(air)`
            // (`BlockStateColumn::new`) -- that is itself the palette's first entry,
            // so introducing `total - 1` further *distinct*, never-before-touched
            // cells (each `.set()` call landing on a fresh index, `0..total-1`, well
            // under `SECTION_BLOCKS`) lands the container on exactly `total` palette
            // entries. Deliberately sparse (only `total - 1` of the section's 4096
            // cells are ever touched, not all 4096): `PalettedContainer::set`'s own
            // bit-width-growth algorithm unpacks+repacks its *entire* data array on
            // every power-of-two crossing, so touching every cell (this module's
            // first-drafted approach) drove the real 10,000-chunk soak leg to ~400s,
            // more than double its own 180s Tier-1 budget -- a forced, load-bearing
            // performance fix, not a content-fidelity concern (Context's own
            // palette-shape rule only needs the *final distinct-entry count* right,
            // never that every one of a section's 4096 cells hold a "real" value).
            PaletteShape::Indirect => {
                let total = next_range(&mut rng, 2, 200);
                let container = column.section_mut(section_index);
                for i in 0..(total - 1) {
                    let value = BlockStateId(2000 + section_index as u32 * 1000 + i);
                    container.set(i as usize, value);
                }
            }
            // `Direct` only needs "at least one section forced Direct" (Context's own
            // literal wording, unlike Indirect's "every section") -- section `0` is
            // that one section; every other section here uses the same cheap,
            // unwritten-`SingleValue(air)` shape `Indirect` still exercises for its
            // own baseline. This matters for more than tidiness: `Direct`'s own
            // `bits_per_entry` is always the registry's *full* `direct_bits` (`15`,
            // `PaletteThresholds::blocks(15)`) regardless of how few distinct values
            // are actually present, so one `Direct` section alone already carries
            // `4096 * 15 / 64 ≈ 960` packed `u64` words (~7.5KB) -- forcing all 24
            // sections `Direct` (this module's first-drafted approach) meant ~180KB
            // of packed data *per chunk*, ×~3,333 `Direct`-shaped chunks: the real
            // dominant cost behind the 10,000-chunk soak's first two measured runs
            // (408s, then 312s after the `Indirect`-side sparsity fix above -- neither
            // within the 180s Tier-1 budget). One forced section already fully
            // exercises the `Direct` code path end-to-end; the other 23 do not need
            // to pay for it too.
            PaletteShape::Direct if section_index == 0 => {
                let total = next_range(&mut rng, 257, 400);
                let container = column.section_mut(section_index);
                for i in 0..(total - 1) {
                    let value = BlockStateId(9000 + i);
                    container.set(i as usize, value);
                }
            }
            PaletteShape::Direct => {}
        }
    }

    column
}

fn section_to_nbt(
    container: &rc_chunk_storage::PalettedContainer<BlockStateId>,
) -> owned::NbtCompound {
    let mut fields: Vec<(Mutf8String, owned::NbtTag)> = Vec::new();
    match container.palette() {
        Palette::SingleValue(value) => {
            fields.push((
                "Palette".into(),
                owned::NbtTag::IntArray(vec![value.0 as i32]),
            ));
        }
        Palette::Indirect { entries, .. } => {
            let ids: Vec<i32> = entries.iter().map(|id| id.0 as i32).collect();
            fields.push(("Palette".into(), owned::NbtTag::IntArray(ids)));
            let words: Vec<i64> = container.raw_words().iter().map(|&w| w as i64).collect();
            fields.push(("Data".into(), owned::NbtTag::LongArray(words)));
        }
        Palette::Direct { .. } => {
            let words: Vec<i64> = container.raw_words().iter().map(|&w| w as i64).collect();
            fields.push(("Data".into(), owned::NbtTag::LongArray(words)));
        }
    }
    owned::NbtCompound::from_values(fields)
}

/// Deterministically generates one already-NBT-encoded chunk payload from `(seed,
/// index)` (Context's exact palette-cycling rule, restated in this module's own doc
/// comment above). Pure, no I/O. **Not** compressed — `ChunkStorageBackend::write_chunk`
/// applies compression itself (the real, committed `AnvilDiskBackend` shape, WORLD-D13 —
/// a forced, documented deviation from the blueprint's own "already-compressed" framing,
/// which predates that backend's actual committed signature).
pub fn generate_chunk_payload(seed: u64, index: u32) -> Vec<u8> {
    let shape = palette_shape_for(index);
    let column = build_column(seed, index, shape);

    let sections: Vec<owned::NbtCompound> = column.sections().iter().map(section_to_nbt).collect();

    let root = owned::NbtCompound::from_values(vec![
        ("Seed".into(), owned::NbtTag::Long(seed as i64)),
        ("Index".into(), owned::NbtTag::Int(index as i32)),
        (
            "Sections".into(),
            owned::NbtTag::List(owned::NbtList::Compound(sections)),
        ),
    ]);

    rc_nbt::write_owned(&owned::BaseNbt::new("", root))
}

/// Runs `count` soak trials against `backend`: generate payload via
/// `generate_chunk_payload(seed, i)`, write it via `ChunkStorageBackend::write_chunk`,
/// immediately read it back via `read_chunk`, and record a `SoakCaseOutcome` only when
/// the read-back bytes are not byte-identical to what was written, or the round trip
/// itself errored (Context's "round_trip_write_read" primitive, applied inline — no
/// separate `rc_chunk_storage` primitive of that exact name is committed, Context's own
/// sanctioned "mechanical substitution" fallback). `dim` is always
/// `rc_core::DimensionId::OVERWORLD`; `kind` is always `RegionFileKind::Terrain`
/// (Context). Chunk coordinates are `(i as i32, 0)`, spreading the corpus across
/// multiple region files (WORLD-D12's 32x32 layout), matching Context exactly.
pub fn run_soak(backend: &dyn ChunkStorageBackend, seed: u64, count: u32) -> SoakReport {
    let mut mismatches = Vec::new();
    let dim = rc_core::DimensionId::OVERWORLD;

    for index in 0..count {
        let payload = generate_chunk_payload(seed, index);
        let palette_shape = palette_shape_for(index);
        let x = index as i32;
        let z = 0;

        let outcome = (|| -> Result<bool, String> {
            backend
                .write_chunk(dim, RegionFileKind::Terrain, x, z, &payload, None)
                .map_err(|e| e.to_string())?;
            let read_back = backend
                .read_chunk(dim, RegionFileKind::Terrain, x, z, None)
                .map_err(|e| e.to_string())?;
            Ok(read_back.as_deref() == Some(payload.as_slice()))
        })();

        match outcome {
            Ok(true) => {}
            Ok(false) => mismatches.push(SoakCaseOutcome {
                index,
                palette_shape,
                round_trip_identical: false,
                error: None,
            }),
            Err(message) => mismatches.push(SoakCaseOutcome {
                index,
                palette_shape,
                round_trip_identical: false,
                error: Some(message),
            }),
        }
    }

    SoakReport {
        seed,
        total: count,
        mismatches,
    }
}
