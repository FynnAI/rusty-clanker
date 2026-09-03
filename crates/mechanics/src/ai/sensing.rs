//! Sensing: nearest-player targeting, a coarse line-of-sight test, and a per-tick
//! seen/unseen cache (M4-B03 blueprint Context §H).

use std::collections::HashSet;

use rc_core::BlockPos;
use rc_core::RcEntityId;

use crate::world_access::BlockWorldAccess;

#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Component))]
#[derive(Clone, Debug, Default)]
pub struct Sensing {
    seen: HashSet<RcEntityId>,
    unseen: HashSet<RcEntityId>,
}

impl Sensing {
    /// Clears both sets — called once per Stage-6a tick per entity, before any
    /// `has_line_of_sight` call that tick.
    pub fn clear(&mut self) {
        todo!()
    }

    /// Checks `seen`/`unseen` first; on a cache miss, calls `raycast_line_of_sight` and
    /// caches the result under `target`.
    pub fn has_line_of_sight(
        &mut self,
        from_eye: [f64; 3],
        target: RcEntityId,
        target_eye: [f64; 3],
        world: &dyn BlockWorldAccess,
    ) -> bool {
        todo!()
    }
}

/// A hand-typed opacity table mirroring `PathTypeTable`'s own bounded-scope precedent
/// (Context §H).
pub struct OpacityTable;

impl OpacityTable {
    pub fn classify(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        todo!()
    }
}

pub fn tier1_opacity_table() -> &'static OpacityTable {
    todo!()
}

/// A coarse DDA voxel-step raycast from `from` to `to` (Context §H). Bounded, documented
/// deviation from vanilla's own exact partial-shape occlusion test: full-cube opacity
/// only.
pub fn raycast_line_of_sight(from: [f64; 3], to: [f64; 3], world: &dyn BlockWorldAccess) -> bool {
    todo!()
}

/// Squared-distance nearest-of candidates within `max_range_blocks`, `None` if empty or
/// all out of range (`TargetingConditions`-style range gate).
pub fn nearest_within_range(
    origin: [f64; 3],
    candidates: impl IntoIterator<Item = (RcEntityId, [f64; 3])>,
    max_range_blocks: f64,
) -> Option<RcEntityId> {
    todo!()
}
