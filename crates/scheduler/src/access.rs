//! `ComponentAccessSummary` — a normalized, `bevy_ecs`-API-decoupled summary of one
//! system's declared component access (Context: "The conflict-graph algorithm").

use std::collections::HashSet;

use bevy_ecs::component::ComponentId;

/// A normalized summary of one system's declared component access (Context:
/// "The conflict-graph algorithm"). Decoupled from `bevy_ecs::query::Access`'s own
/// API surface so `compute_waves` never depends on its exact 0.19.1 shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentAccessSummary {
    pub reads: HashSet<ComponentId>,
    pub writes: HashSet<ComponentId>,
    pub reads_all: bool,
    pub writes_all: bool,
}

impl ComponentAccessSummary {
    /// Construct directly from a fixed read/write set — the primary constructor this
    /// blueprint's own tests use (synthetic `ComponentId` values, no real `World`
    /// needed). `reads_all`/`writes_all` default to `false`.
    pub fn new(
        reads: impl IntoIterator<Item = ComponentId>,
        writes: impl IntoIterator<Item = ComponentId>,
    ) -> Self {
        let _ = (reads, writes);
        todo!()
    }

    /// `reads`/`writes` empty, `reads_all`/`writes_all` as given — for systems using
    /// an unrestricted dynamic query (ARCH-D4's `FilteredEntityRef`/`FilteredEntityMut`).
    pub fn wildcard(reads_all: bool, writes_all: bool) -> Self {
        let _ = (reads_all, writes_all);
        todo!()
    }

    /// Extracted from a real, `.initialize`d `bevy_ecs::system::System`'s combined
    /// access (Context's "bevy_ecs 0.19.1 API points to verify", point 1/2). Not
    /// exercised by this blueprint's pure `compute_waves` tests, which construct
    /// `ComponentAccessSummary` directly via `new`/`wildcard`; exercised only by the
    /// integration tests that run real systems.
    ///
    /// Delta from this blueprint's own sketch: `bevy_ecs` 0.19.1's `Access` type is
    /// no longer generic over its index type (`bevy_ecs::query::Access`, not
    /// `Access<ComponentId>`).
    pub fn from_bevy_access(access: &bevy_ecs::query::Access) -> Self {
        let _ = access;
        todo!()
    }

    /// True iff `self` and `other` may run concurrently (Context's compatibility rule).
    pub fn is_compatible(&self, other: &Self) -> bool {
        let _ = other;
        todo!()
    }
}
