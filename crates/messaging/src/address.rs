use rc_core::{ChunkKey, RcEntityId};

/// A region's identity (ARCH-D24's `RegionId -> ...` directory key). Never reused
/// within one server process's lifetime, even after the region it named merges away
/// (this blueprint's Context section explains why). This crate does not allocate
/// `RegionId` values — that is `rc-scheduler`'s ARCH-D6 region-lifecycle job; this
/// type only fixes the identifier's shape.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RegionId(pub u64);

/// Where a `RegionMessage` is headed. Exact shape pinned by ARCH-D25. Resolution of
/// `Entity`/`Chunk` to a concrete owning `RegionId` happens inside whichever concrete
/// `Transport` implementation calls `Transport::send` (ARCH-D25/ARCH-D27) — this crate
/// never performs that resolution itself and never re-resolves a received `Address`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Address {
    Region(RegionId),
    Entity(RcEntityId),
    Chunk(ChunkKey),
}
