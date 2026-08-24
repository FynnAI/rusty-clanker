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
        Self {
            reads: reads.into_iter().collect(),
            writes: writes.into_iter().collect(),
            reads_all: false,
            writes_all: false,
        }
    }

    /// `reads`/`writes` empty, `reads_all`/`writes_all` as given — for systems using
    /// an unrestricted dynamic query (ARCH-D4's `FilteredEntityRef`/`FilteredEntityMut`).
    pub fn wildcard(reads_all: bool, writes_all: bool) -> Self {
        Self {
            reads: HashSet::new(),
            writes: HashSet::new(),
            reads_all,
            writes_all,
        }
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
        // `try_reads_and_writes`/`try_writes` fail (`Err`) exactly when the
        // underlying access is unbounded (an "all except" dynamic query, e.g. a
        // `FilteredEntityRef`/`FilteredEntityMut` with exclusions). Treated here as
        // a conservative full wildcard: always sound for `is_compatible` below (it
        // only ever serializes *more* than strictly necessary, never less) -- no
        // native system needs finer handling at M0.
        match (access.try_reads_and_writes(), access.try_writes()) {
            (Ok(reads_and_writes), Ok(writes)) => Self {
                reads: reads_and_writes.iter().collect(),
                writes: writes.iter().collect(),
                reads_all: false,
                writes_all: false,
            },
            (_, Err(_)) => Self::wildcard(true, true),
            (Err(_), Ok(_)) => Self::wildcard(true, false),
        }
    }

    /// True iff `self` and `other` may run concurrently (Context's compatibility rule).
    pub fn is_compatible(&self, other: &Self) -> bool {
        if self.writes_all || other.writes_all {
            return false;
        }
        if self.reads_all && (other.writes_all || !other.writes.is_empty()) {
            return false;
        }
        if other.reads_all && (self.writes_all || !self.writes.is_empty()) {
            return false;
        }
        if !self.writes.is_disjoint(&other.writes) {
            return false;
        }
        if !self.writes.is_disjoint(&other.reads) {
            return false;
        }
        if !other.writes.is_disjoint(&self.reads) {
            return false;
        }
        true
    }
}
