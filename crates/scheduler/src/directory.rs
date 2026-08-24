//! `RegionIdAllocator` and the ARCH-D6-scoped `GridCell -> RegionId` directory.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::AtomicU64;

use rc_messaging::RegionId;

use crate::grid::GridCell;

/// A thread-safe, lock-free monotonic `RegionId` allocator (`rc-messaging` explicitly
/// declines to own this — M0-B02's own Context: "This crate does not allocate `RegionId`
/// values — that is `rc-scheduler`'s ARCH-D6 region-lifecycle job"). Mirrors
/// `rc_core::RcEntityIdAllocator`'s exact shape and guarantees.
pub struct RegionIdAllocator(AtomicU64);

impl RegionIdAllocator {
    /// First `alloc()` on a fresh instance returns `RegionId(1)`; `0` is reserved as a
    /// never-valid sentinel.
    pub const fn new() -> Self {
        todo!()
    }
    /// Thread-safe; never blocks; every returned value is unique for this instance's
    /// lifetime and strictly greater than every previously-returned value.
    pub fn alloc(&self) -> RegionId {
        todo!()
    }
}
impl Default for RegionIdAllocator {
    fn default() -> Self {
        todo!()
    }
}

/// This blueprint's own cell-ownership bookkeeping (the ARCH-D6-scoped, `GridCell`-keyed
/// analog of ARCH-D24's full `ChunkKey -> RegionId` directory — see Context's "Directory
/// scope" note for exactly what this narrower type does and does not claim to be
/// authoritative for; in particular it is *not* consulted by any `Transport`
/// implementation, since `rc-transport-inproc` has no Cargo dependency on `rc-scheduler`).
#[derive(Debug, Default)]
pub struct RegionDirectory {
    owner: HashMap<GridCell, RegionId>,
}

impl RegionDirectory {
    pub fn new() -> Self {
        todo!()
    }
    pub fn owner_of(&self, cell: GridCell) -> Option<RegionId> {
        todo!()
    }
    pub(crate) fn assign(&mut self, cell: GridCell, region: RegionId) {
        todo!()
    }
    pub(crate) fn unassign(&mut self, cell: GridCell) {
        todo!()
    }
    /// Every currently-live region id adjacent to `region`'s own `cells` (ARCH-D6's
    /// "neighboring region"): distinct owning-region ids of every 4-directional
    /// neighbor of every cell in `cells`, excluding `region` itself.
    pub fn adjacent_regions(
        &self,
        region: RegionId,
        cells: &BTreeSet<GridCell>,
    ) -> BTreeSet<RegionId> {
        todo!()
    }
}
