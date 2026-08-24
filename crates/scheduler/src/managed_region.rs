//! `ManagedRegion`: one `RegionState` (M0-B05) wrapped with the ARCH-D5/D6/D7/D19
//! cell/dimension/hysteresis bookkeeping M0-B05 does not itself own.

use std::collections::{BTreeSet, HashMap};

use rc_core::DimensionId;
use rc_messaging::RegionId;

use crate::RegionState; // M0-B05, re-exported at this crate's root
use crate::grid::GridCell;

/// One region's full state for this blueprint's purposes: M0-B05's `RegionState`
/// (`World`, tick counter, message state — untouched, wrapped by value, field `state`
/// is `pub` so callers reach `state.world`/`state.message_state` directly) plus the
/// ARCH-D5/D6/D7/D19 bookkeeping M0-B05 explicitly does not own.
pub struct ManagedRegion {
    pub state: RegionState,
    dimension: DimensionId,
    cells: BTreeSet<GridCell>,
    tick_budget_ms: f64,
    ewma_ms: Option<f64>,
    ticks_over_split_threshold: u32,
    /// ARCH-D6 merge hysteresis, one counter per adjacent neighbor this region is the
    /// *responsible* (smaller-`RegionId`) side for (Context: "Who evaluates a merge").
    merge_candidates: HashMap<RegionId, u32>,
}

impl ManagedRegion {
    /// Panics if `cells` is empty or any cell's `.dimension` differs from `dimension`.
    pub(crate) fn new(
        state: RegionState,
        dimension: DimensionId,
        cells: BTreeSet<GridCell>,
        tick_budget_ms: f64,
    ) -> Self {
        todo!()
    }

    pub fn id(&self) -> RegionId {
        todo!()
    }
    pub fn dimension(&self) -> DimensionId {
        todo!()
    }
    pub fn cells(&self) -> &BTreeSet<GridCell> {
        todo!()
    }
    pub fn tick_budget_ms(&self) -> f64 {
        todo!()
    }
    /// `0.9 * tick_budget_ms` (ARCH-D6).
    pub fn split_threshold_ms(&self) -> f64 {
        todo!()
    }
    /// `0.1 * tick_budget_ms` (this blueprint's concrete merge-threshold pin, Context).
    pub fn merge_threshold_ms(&self) -> f64 {
        todo!()
    }
    /// `None` until the first `record_tick_duration` call.
    pub fn tick_duration_ewma_ms(&self) -> Option<f64> {
        todo!()
    }
    pub fn ticks_over_split_threshold(&self) -> u32 {
        todo!()
    }
    /// `0` if `neighbor` has never been tracked (including: not currently adjacent, or
    /// this region is not the responsible side of that pair).
    pub fn merge_candidate_ticks(&self, neighbor: RegionId) -> u32 {
        todo!()
    }

    /// ARCH-D19's EWMA update (Context has the exact formula) plus the split-hysteresis
    /// counter update. Returns `true` iff this call just made
    /// `ticks_over_split_threshold` reach exactly 40.
    pub(crate) fn record_tick_duration(&mut self, sample_ms: f64) -> bool {
        todo!()
    }

    /// Updates the `(self, neighbor)` merge-hysteresis counter against a caller-supplied
    /// `combined_ewma_ms` (the sum of both regions' current EWMAs). Returns `true` iff
    /// this call just made that counter reach exactly 100.
    pub(crate) fn update_merge_candidate(
        &mut self,
        neighbor: RegionId,
        combined_ewma_ms: f64,
    ) -> bool {
        todo!()
    }
}
