//! Entity identity: `EntityUuid` (vanilla's own externally-visible, randomly-assigned
//! entity UUID) and `NetworkEntityIdAllocator` (the wire-protocol `Entity ID` counter,
//! formalized from `HardcodedWorld::alloc_network_entity_id`'s own already-merged
//! per-region pattern, M1-B05). Neither is `rc_core::RcEntityId` (ARCH-D24's internal,
//! monotonic, transfer-stable identity) — see this module's own doc comments for why.

use std::sync::atomic::{AtomicI32, Ordering};

/// A process-unique, `Copy` entity UUID (the base bundle's `UUID` field, MECH-D30).
/// Not `rc_core::RcEntityId` (internal, monotonic, ARCH-D24) — this is vanilla's own
/// externally-visible, randomly-assigned identity. See this blueprint's Context,
/// "Entity identity," for why `uuid::Uuid::new_v4()`-backed randomness introduces no
/// MECH-D5 parity concern: vanilla itself draws an entity's UUID from a per-entity,
/// non-world-seeded `RandomSource` stream, never from the world-seeded `RcRandom`
/// sequence MECH-D5 governs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EntityUuid(pub u128);

impl EntityUuid {
    /// Mints a fresh, cryptographically-random UUID (vanilla's own `UUID.randomUUID()`
    /// equivalent). Never call this to reconstruct a previously-assigned value — use
    /// the `From<u128>`/tuple-field-access path for that (deserialization, tests).
    pub fn new_random() -> Self {
        Self(uuid::Uuid::new_v4().as_u128())
    }
}

/// A `NetworkEntityIdAllocator`-shared, per-region, lock-free, thread-safe monotonic
/// i32 counter (the wire-protocol `Entity ID` every spawn/movement/removal packet
/// carries) — distinct from `RcEntityId` (internal, 64-bit, ARCH-D24-stable across
/// transfers) exactly as M1-B05's own `HardcodedWorld::alloc_network_entity_id`
/// already establishes for players; this type formalizes the identical allocator so
/// every entity kind, not only players, draws from one shared numeric space per
/// region. First `alloc()` on a fresh instance returns `1`. Thread-safe; never blocks.
pub struct NetworkEntityIdAllocator(AtomicI32);

impl NetworkEntityIdAllocator {
    pub const fn new() -> Self {
        Self(AtomicI32::new(1))
    }

    pub fn alloc(&self) -> i32 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for NetworkEntityIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}
