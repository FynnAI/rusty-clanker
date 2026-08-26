//! Anvil `.mca` region-file container (WORLD-D12/D13/D17), the `ChunkStorageBackend`
//! trait and its `AnvilDiskBackend` implementation over WORLD-D14's save-folder layout.
//! `ObjectStoreBackend`, `IoUringAnvilDiskBackend`, Stage-9 scheduling wiring, and real
//! `ChunkColumn` NBT schemas are explicitly out of scope — see this crate's own
//! module-level docs in the owning blueprint (M2-B03) for the full boundary.

mod backend;
mod checksum;
mod compression;
mod error;
mod region_file;

pub use backend::{AnvilDiskBackend, ChunkStorageBackend, RegionFileKind};
pub use checksum::content_checksum;
pub use compression::CompressionScheme;
pub use error::StorageError;
pub use region_file::RegionFile;
