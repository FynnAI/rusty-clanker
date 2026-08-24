use crate::Address;
use crate::RegionId;

/// The cross-partition message envelope (ARCH-D25). Exact field shape pinned there.
/// Generic over the payload so the type is reusable if a future revision ever needs a
/// second payload enum (none does today — every current use is `Message<RegionMessage>`).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message<T> {
    pub from: RegionId,
    pub to: Address,
    /// The *sending* region's own tick counter at emission time (CLUSTER-D25: stays
    /// region-local even in cluster mode).
    pub tick_stamp: u64,
    /// Monotonic per distinct `to: Address` value, starting at 0, persisting across
    /// ticks — this blueprint's concrete resolution of ARCH-D25's otherwise-unpinned
    /// `seq` semantics (see Context). Assigned by `RegionMessageState::drain_outbox`.
    pub seq: u32,
    pub payload: T,
}
