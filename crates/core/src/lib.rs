//! `rc-core` — foundational shared types with zero I/O: coordinate math, entity-id
//! types, and the workspace-wide error/result convention every other crate follows
//! (see the crate-level docs on `rc_messaging::TransportError` for the first concrete
//! instantiation of that convention: a `thiserror`-derived, crate-local error enum,
//! never `Box<dyn Error>` or `anyhow`).
//!
//! `rc-core` itself has no fallible public constructors — every type here accepts any
//! value of its underlying representation without validation (e.g. `BlockPos` performs
//! no world-height range check; that belongs to whichever crate owns world bounds).

mod coords;
mod entity_id;

pub use coords::{BlockPos, ChunkKey, DimensionId};
pub use entity_id::{RcEntityId, RcEntityIdAllocator};
