//! ARCH-D6's "largest internal cell-connectivity cut" split algorithm and the
//! `LifecycleOutcome` type reporting what a merge/split just did.

use std::collections::BTreeSet;

use rc_messaging::RegionId;

use crate::grid::GridCell;

/// What `RegionManager::after_tick`/`force_split`/`force_merge` did, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleOutcome {
    None,
    /// `old`'s id is permanently retired; `new_a` is always the size->= fragment
    /// (`largest_connectivity_cut`'s own canonical return order), `new_b` the other.
    Split {
        old: RegionId,
        new_a: RegionId,
        new_b: RegionId,
    },
    /// `old_a`'s and `old_b`'s ids are both permanently retired.
    Merged {
        old_a: RegionId,
        old_b: RegionId,
        new: RegionId,
    },
}

/// ARCH-D6's "largest internal cell-connectivity cut" (Context has the full algorithm
/// and the exact tie-break order). Returns `(bigger_or_equal, smaller_or_equal)`.
/// Panics if `cells.len() < 2` or no valid 2-way connectivity cut exists (the latter is
/// unreachable for any cell set that is itself internally connected, which every
/// `ManagedRegion`'s own cell set always is by construction).
pub fn largest_connectivity_cut(
    cells: &BTreeSet<GridCell>,
) -> (BTreeSet<GridCell>, BTreeSet<GridCell>) {
    todo!()
}
