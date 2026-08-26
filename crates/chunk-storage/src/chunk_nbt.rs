//! Vanilla chunk NBT (de)serialization at the pinned DataVersion (WORLD-D11/D16), built
//! entirely on M2-B01's already-committed chunk components and M2-B02's already-committed
//! `rc-nbt` surface. Hand-written, never derived (WORLD-D11) -- see this blueprint's own
//! Context for the full schema, the on-disk paletted-container encoding, the registry-id
//! resolver seam, and the fixed-default/opaque-extra unknown-tag policy.

use crate::{
    BiomeColumn, BiomeId, BlockEntityIndex, BlockStateColumn, BlockStateId, ChunkKeyTag,
    ChunkPersistenceState, ChunkStatus, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_core::{ChunkKey, DimensionId};
use rc_nbt::{Mutf8Str, Mutf8String, borrow, owned};

/// The pinned target's DataVersion (WORLD-D16). Every document this crate writes
/// stamps this value; a loaded document whose `DataVersion` differs is refused.
pub const DATA_VERSION: i32 = 4903;

/// The vanilla `yPos` value every document this crate writes or accepts must carry --
/// `WORLD_MIN_Y / 16` (Context).
pub const MIN_SECTION_Y: i32 = crate::WORLD_MIN_Y / 16;

/// Caller-supplied bridge from this crate's registry-agnostic `BlockStateId` to the
/// vanilla `{Name, Properties}` palette-entry shape (Context's Resolved discrepancy).
/// No implementation ships in this crate.
pub trait BlockStateNames {
    /// The block's namespaced id and its state's property key/value pairs, in **any**
    /// order -- this crate re-sorts them before writing (next subsection). `None` means
    /// "this crate's registry has no entry for `id`" (an incomplete/corrupt resolver,
    /// or a raw id from a newer registry this build does not know about).
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)>;
    /// The inverse: a name + property set (in whatever order the NBT document stored
    /// them) resolved back to a concrete id. `None` if no registered state matches.
    fn resolve(
        &self,
        name: &Mutf8Str,
        properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId>;
}

/// As `BlockStateNames`, for biomes -- plain-string palette entries, no properties.
pub trait BiomeNames {
    fn name(&self, id: BiomeId) -> Option<Mutf8String>;
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChunkNbtError {
    #[error("unsupported DataVersion: expected {expected}, found {found}")]
    UnsupportedDataVersion { expected: i32, found: i32 },
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("field `{0}` has the wrong NBT tag type")]
    WrongFieldType(&'static str),
    #[error("yPos {found} does not match this engine's fixed world bounds (expected {expected})")]
    UnexpectedYPos { expected: i32, found: i32 },
    #[error("section Y {0} is out of the supported light/block range")]
    SectionYOutOfRange(i32),
    #[error("missing required block section for Y {0}")]
    MissingSection(i32),
    #[error("malformed palette in field `{0}`: {1}")]
    MalformedPalette(&'static str, String),
    #[error(
        "block_entities must be empty at M2 scope (no BlockEntityCodec exists yet, WORLD-D6) — found {0} entries"
    )]
    UnsupportedBlockEntities(usize),
    #[error("unknown block state name `{0}` — the supplied BlockStateNames resolver has no match")]
    UnknownBlockStateName(String),
    #[error("unknown biome name `{0}` — the supplied BiomeNames resolver has no match")]
    UnknownBiomeName(String),
    #[error(transparent)]
    Nbt(#[from] rc_nbt::NbtError),
}

/// Every component `chunk_from_nbt` reconstructs, plus the two fields this crate does
/// not store anywhere else (Context: `isLightOn` is a plain passthrough; `extra` is the
/// opaque unknown-tag bag).
pub struct ChunkNbtDocument {
    pub chunk_key: ChunkKeyTag,
    pub blocks: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entities: BlockEntityIndex,
    pub status: ChunkStatus,
    pub persistence: ChunkPersistenceState,
    pub is_light_on: bool,
    pub extra: Vec<(Mutf8String, owned::NbtTag)>,
}

/// Bundles the two registry resolvers and the two `PaletteThresholds` a caller must
/// supply (Context -- this crate never bakes in a registry's own size). One `to_nbt`/
/// `from_nbt` call pair per chunk; cheap to construct, holds only borrows and `Copy`
/// values.
pub struct ChunkNbtCodec<'a, N: BlockStateNames, B: BiomeNames> {
    pub block_names: &'a N,
    pub biome_names: &'a B,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

impl<'a, N: BlockStateNames, B: BiomeNames> ChunkNbtCodec<'a, N, B> {
    /// Builds the full vanilla chunk NBT compound (Context: schema, ordering, and the
    /// fixed-default/opaque-extra policy). `extra` is re-emitted verbatim, appended
    /// after every known and fixed-default field, in its given order -- pass `&[]` for
    /// a chunk with no captured unknown tags (e.g. one this engine created itself).
    /// Errors only on a non-empty `block_entities` or an `id` the resolvers cannot
    /// name.
    #[allow(clippy::too_many_arguments)]
    pub fn to_nbt(
        &self,
        chunk_key: ChunkKey,
        blocks: &BlockStateColumn,
        biomes: &BiomeColumn,
        light: &LightColumn,
        heightmaps: &HeightmapSet,
        block_entities: &BlockEntityIndex,
        status: ChunkStatus,
        persistence: ChunkPersistenceState,
        is_light_on: bool,
        extra: &[(Mutf8String, owned::NbtTag)],
    ) -> Result<owned::NbtCompound, ChunkNbtError> {
        let _ = (
            chunk_key,
            blocks,
            biomes,
            light,
            heightmaps,
            block_entities,
            status,
            persistence,
            is_light_on,
            extra,
        );
        todo!()
    }

    /// The inverse. `dimension` is supplied by the caller (the region file the
    /// document was read from names it -- vanilla chunk NBT itself carries no
    /// dimension field, only `xPos`/`zPos`) and combined with the loaded `xPos`/`zPos`
    /// into the returned `ChunkKeyTag`.
    pub fn from_nbt(
        &self,
        tag: &borrow::NbtCompound<'_, '_>,
        dimension: DimensionId,
    ) -> Result<ChunkNbtDocument, ChunkNbtError> {
        let _ = (tag, dimension);
        todo!()
    }
}
