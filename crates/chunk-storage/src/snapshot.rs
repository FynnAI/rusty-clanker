//! The versioned `postcard` `ChunkSnapshot` (WORLD-D20) -- a separate, parallel
//! representation from `chunk_nbt`'s vanilla schema, used only for fast in-memory
//! hand-off (cluster migration/takeover staging), never for durable Anvil storage. See
//! this blueprint's own Context for why this is a flat, self-contained `struct` tree
//! rather than a `#[derive(Serialize)]` wrapper over M2-B01's own component types.

/// This engine's own internal compatibility counter for `ChunkSnapshot`'s wire shape --
/// independent of Mojang's `DataVersion` (WORLD-D20).
pub const RC_CHUNK_SNAPSHOT_VERSION: u16 = 1;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub struct SnapshotLightSection {
    /// Always exactly 2048 bytes when `Some` -- mirrors `LightSection`'s own nibble-
    /// packed array shape (M2-B01), stored as a `Vec` only because `serde`'s derive
    /// does not implement `[u8; 2048]` directly without an extra crate this blueprint
    /// does not add.
    pub sky: Option<Vec<u8>>,
    pub block: Option<Vec<u8>>,
}

/// A flat, self-contained hand-off snapshot of one chunk column (WORLD-D20) -- built
/// only from raw scalar data reachable through M2-B01's public accessors, never a
/// `#[derive(Serialize)]` wrapping M2-B01's own component types directly (Context: those
/// types derive no `serde` impls and this blueprint does not retroactively add any).
/// Block entities are **not** captured (Context -- WORLD-D6's codec does not exist
/// yet).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ChunkSnapshot {
    pub chunk_key: rc_core::ChunkKey,
    /// Section-major flat array of raw block-state ids, length always
    /// `SECTION_COUNT * SECTION_BLOCKS` (98304); entry `section * 4096 + block_index`
    /// (`crate::column::block_index`'s own within-section convention).
    pub block_ids: Vec<u32>,
    /// Section-major flat array of raw biome ids, length always
    /// `SECTION_COUNT * SECTION_BIOME_CELLS` (1536).
    pub biome_ids: Vec<u32>,
    /// One entry per `LIGHT_SECTION_COUNT` (26) light section, ascending index order.
    pub light_sections: Vec<SnapshotLightSection>,
    /// Six flat 256-entry raw-value arrays (`HeightmapSet::raw`'s own convention),
    /// indexed in `HeightmapKind::ALL`'s declared order.
    pub heightmaps: [Vec<u16>; 6],
    /// `0` = `ChunkGenStatus::Generating`, `1` = `ChunkGenStatus::Full` -- the same
    /// mapping `chunk_nbt`'s `Status` field uses.
    pub gen_status: u8,
    pub dirty: bool,
    pub last_saved_tick: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot bytes truncated before the 2-byte format_version prefix")]
    Truncated,
    #[error("unsupported ChunkSnapshot format_version: expected {expected}, found {found}")]
    UnsupportedVersion { expected: u16, found: u16 },
    #[error("postcard decode error: {0}")]
    Decode(#[from] postcard::Error),
}

/// Encodes `snapshot` as `[format_version: 2 bytes, big-endian][postcard-encoded body]`
/// -- the version prefix is raw, fixed-width bytes, never itself postcard-encoded
/// (Context: this is what makes `peek_snapshot_version` decodable without knowing any
/// later version's body shape).
pub fn encode_snapshot(snapshot: &ChunkSnapshot) -> Vec<u8> {
    let _ = snapshot;
    todo!()
}

/// Reads only the 2-byte version prefix, without attempting to decode the body.
pub fn peek_snapshot_version(bytes: &[u8]) -> Result<u16, SnapshotError> {
    let _ = bytes;
    todo!()
}

/// Full decode. `SnapshotError::UnsupportedVersion` if the prefix does not equal
/// `RC_CHUNK_SNAPSHOT_VERSION` (Context: exact-match policy, no migration, mirroring
/// WORLD-D16 on this second, independent versioning axis).
pub fn decode_snapshot(bytes: &[u8]) -> Result<ChunkSnapshot, SnapshotError> {
    let _ = bytes;
    todo!()
}
