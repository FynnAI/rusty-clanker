use std::path::PathBuf;

/// `ChunkStorageBackend`'s one error type (WORLD-D17), shared by every module in this
/// tree. Every variant that names a path carries it for diagnostics — no variant is
/// ever constructed with a placeholder/empty path (Constraints).
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// A region file's own structure is invalid independent of any one chunk's record
    /// (wrong overall length, non-sector-aligned size, an unreadable header).
    #[error("region file {path} structurally corrupt: {reason}")]
    Corrupt { path: PathBuf, reason: String },

    /// One chunk's location-table entry claims sectors outside the file's own current
    /// extent, or a record's declared `length` exceeds the sectors it was allocated.
    #[error(
        "chunk ({local_x},{local_z}) sector pointer out of bounds: offset {offset} count {count}, file has {file_sectors} sectors"
    )]
    SectorOutOfBounds {
        local_x: u8,
        local_z: u8,
        offset: u32,
        count: u8,
        file_sectors: u32,
    },

    #[error("unknown chunk compression scheme id {0}")]
    UnknownCompressionType(u8),

    #[error("decompression failed: {0}")]
    Decompress(String),

    /// This crate's own defense-in-depth corruption check (Context) — decompressed
    /// bytes that do not parse as a well-formed NBT document via
    /// `rc_nbt::read_borrowed_strict`.
    #[error("chunk payload failed NBT well-formedness validation: {0}")]
    InvalidNbtPayload(String),

    #[error("an external chunk record at {path} points to a `.mcc` file that does not exist")]
    MissingExternalFile { path: PathBuf },

    #[error("world at {path} is already open (held by another process via session.lock)")]
    WorldAlreadyOpen { path: PathBuf },

    #[error(
        "unsupported dimension {0:?} — only the built-in Overworld/Nether/End are mapped to a save folder at this milestone's scope"
    )]
    UnsupportedDimension(rc_core::DimensionId),

    /// WORLD-D14 (M3.5-B05): `world_root` still uses the pre-M3.5 legacy save layout
    /// (a top-level `region/` directory directly under the world root, with no
    /// `dimensions/` directory yet present). This engine no longer reads or writes
    /// that layout at all — refused fast, never silently misread or silently
    /// double-written under the new layout alongside the old one.
    #[error(
        "world at {path} uses the pre-M3.5 legacy save layout (region/ at the world root); this engine no longer reads or writes that layout — delete this directory (or move it aside) and let it regenerate, or migrate it by hand under dimensions/minecraft/overworld/"
    )]
    LegacyLayoutDetected { path: PathBuf },
}
