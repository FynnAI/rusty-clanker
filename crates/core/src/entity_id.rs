use std::sync::atomic::{AtomicU64, Ordering};

/// A globally unique, monotonically-allocated entity identifier (ARCH-D24): "monotonic,
/// allocated once at spawn, distinct from the ephemeral intra-`World` `bevy_ecs::Entity`
/// index+generation, and stable across ARCH-D10 transfers." This type does not itself
/// enforce uniqueness on construction — use `RcEntityIdAllocator::alloc` for that; a raw
/// constructor is exposed for deserialization and test-fixture use, where reconstructing
/// a specific previously-allocated value (not minting a new one) is exactly the point.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct RcEntityId(pub u64);

impl RcEntityId {
    /// Reconstruct a specific id value (deserialization, tests, or a future spawn-time
    /// integration handing this crate an id it decides not to hand out via
    /// `RcEntityIdAllocator`). Never call this to mint a *new* id in production code —
    /// that is `RcEntityIdAllocator::alloc`'s exclusive job.
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }
}

/// A thread-safe, lock-free monotonic `RcEntityId` allocator. Every value returned by
/// `alloc` is strictly greater than every previously-returned value from the same
/// instance, and no two calls (even concurrent, from different `RC-WorkerPool` threads,
/// ARCH-D18) ever return the same value. Intended to be shared as a single
/// server-lifetime instance (e.g. behind an `Arc` or a `static`) — `alloc` takes `&self`,
/// not `&mut self`, precisely so callers never need external synchronization.
pub struct RcEntityIdAllocator(AtomicU64);

impl RcEntityIdAllocator {
    /// The first `alloc()` call on a freshly-constructed instance returns `RcEntityId(1)`.
    pub const fn new() -> Self {
        Self(AtomicU64::new(1))
    }

    /// Allocate the next id. Thread-safe; never blocks.
    pub fn alloc(&self) -> RcEntityId {
        RcEntityId(self.0.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for RcEntityIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}
