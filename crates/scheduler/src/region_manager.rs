//! `RegionManager`: owns a set of `ManagedRegion`s plus their cell-ownership directory
//! and `RegionId` allocator, and drives ARCH-D6's merge/split evaluation.

use std::collections::HashMap;

use rc_core::DimensionId;
use rc_messaging::{RegionId, Transport};

use crate::directory::{RegionDirectory, RegionIdAllocator};
use crate::grid::GridCell;
use crate::lifecycle::LifecycleOutcome;
use crate::managed_region::ManagedRegion;
use crate::pool::RcWorkerPool; // M0-B04, `pub mod pool` at this crate's root
use crate::{RcExecutor, TickReport}; // M0-B05, re-exported at this crate's root

/// Owns a set of `ManagedRegion`s plus their cell-ownership directory and `RegionId`
/// allocator, and drives ARCH-D6's merge/split evaluation. Wraps one `&RcExecutor`
/// (M0-B05) — never constructs or ticks a `RegionState` except through it.
pub struct RegionManager<'e> {
    executor: &'e RcExecutor,
    regions: HashMap<RegionId, ManagedRegion>,
    directory: RegionDirectory,
    id_alloc: RegionIdAllocator,
    tick_budget_ms: f64,
}

impl<'e> RegionManager<'e> {
    pub fn new(executor: &'e RcExecutor, tick_budget_ms: f64) -> Self {
        todo!()
    }

    /// Allocates a fresh `RegionId` (never reused), constructs a `ManagedRegion` via
    /// `executor.spawn_region`, registers every cell in the directory. Panics if `cells`
    /// is empty, any cell's dimension differs, or any cell is already owned by another
    /// live region.
    pub fn spawn_region(
        &mut self,
        dimension: DimensionId,
        cells: impl IntoIterator<Item = GridCell>,
    ) -> RegionId {
        todo!()
    }

    pub fn region(&self, id: RegionId) -> Option<&ManagedRegion> {
        todo!()
    }
    pub fn region_mut(&mut self, id: RegionId) -> Option<&mut ManagedRegion> {
        todo!()
    }
    /// Every currently-live region id, ascending.
    pub fn region_ids(&self) -> Vec<RegionId> {
        todo!()
    }
    pub fn neighbors_of(&self, id: RegionId) -> Vec<RegionId> {
        todo!()
    }

    /// Ticks `id` via `self.executor.tick_region` (the real M0-B05 pipeline over
    /// `pool`/`transport`), measures the call's own wall-clock duration, and feeds that
    /// duration into `record_synthetic_tick`'s bookkeeping. Panics (propagating any
    /// panic from `RcExecutor::tick_region` unchanged) if `id` is unknown or a system
    /// panics — the caller's own test harness is this blueprint's "zero panics" gate.
    pub fn tick_region(
        &mut self,
        id: RegionId,
        pool: &RcWorkerPool,
        transport: &dyn Transport,
    ) -> (TickReport, LifecycleOutcome) {
        todo!()
    }

    /// Bookkeeping-only: feeds a caller-supplied `sample_ms` directly into `id`'s
    /// EWMA/hysteresis (Context's formulas) without calling `RcExecutor::tick_region` at
    /// all, then evaluates and, if triggered, executes a split or merge. This
    /// blueprint's own fast hysteresis/merge/split tests use this exclusively.
    pub fn record_synthetic_tick(
        &mut self,
        id: RegionId,
        sample_ms: f64,
        transport: &dyn Transport,
    ) -> LifecycleOutcome {
        todo!()
    }

    /// Bypasses hysteresis entirely and executes a split immediately. Panics if `id` is
    /// unknown or owns fewer than 2 cells.
    pub fn force_split(&mut self, id: RegionId, transport: &dyn Transport) -> LifecycleOutcome {
        todo!()
    }

    /// Bypasses hysteresis entirely and executes a merge immediately. Panics if `a`/`b`
    /// are unknown or not currently adjacent.
    pub fn force_merge(
        &mut self,
        a: RegionId,
        b: RegionId,
        transport: &dyn Transport,
    ) -> LifecycleOutcome {
        todo!()
    }
}
