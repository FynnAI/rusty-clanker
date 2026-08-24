//! `RegionManager`: owns a set of `ManagedRegion`s plus their cell-ownership directory
//! and `RegionId` allocator, and drives ARCH-D6's merge/split evaluation.

use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

use rc_core::DimensionId;
use rc_messaging::{Address, Message, RegionId, RegionMessage, Transport};

use crate::directory::{RegionDirectory, RegionIdAllocator};
use crate::grid::GridCell;
use crate::lifecycle::{LifecycleOutcome, largest_connectivity_cut};
use crate::managed_region::ManagedRegion;
use crate::pool::RcWorkerPool; // M0-B04, `pub mod pool` at this crate's root
use crate::synthetic_load::SyntheticLoadProfile;
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
        Self {
            executor,
            regions: HashMap::new(),
            directory: RegionDirectory::new(),
            id_alloc: RegionIdAllocator::new(),
            tick_budget_ms,
        }
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
        let cells: BTreeSet<GridCell> = cells.into_iter().collect();
        assert!(
            !cells.is_empty(),
            "RegionManager::spawn_region requires a non-empty cell set"
        );
        for &cell in &cells {
            assert_eq!(
                cell.dimension, dimension,
                "RegionManager::spawn_region: every cell must share the given dimension"
            );
            assert!(
                self.directory.owner_of(cell).is_none(),
                "RegionManager::spawn_region: cell {cell:?} is already owned by another live region"
            );
        }

        let id = self.id_alloc.alloc();
        let state = self.executor.spawn_region(id);
        let managed = ManagedRegion::new(state, dimension, cells.clone(), self.tick_budget_ms);

        for &cell in &cells {
            self.directory.assign(cell, id);
        }
        self.regions.insert(id, managed);
        id
    }

    pub fn region(&self, id: RegionId) -> Option<&ManagedRegion> {
        self.regions.get(&id)
    }
    pub fn region_mut(&mut self, id: RegionId) -> Option<&mut ManagedRegion> {
        self.regions.get_mut(&id)
    }
    /// Every currently-live region id, ascending.
    pub fn region_ids(&self) -> Vec<RegionId> {
        let mut ids: Vec<RegionId> = self.regions.keys().copied().collect();
        ids.sort();
        ids
    }
    pub fn neighbors_of(&self, id: RegionId) -> Vec<RegionId> {
        match self.regions.get(&id) {
            Some(region) => self
                .directory
                .adjacent_regions(id, region.cells())
                .into_iter()
                .collect(),
            None => Vec::new(),
        }
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
        let executor = self.executor; // `&'e RcExecutor` is `Copy`
        let start = Instant::now();
        let report = {
            let region = self
                .regions
                .get_mut(&id)
                .expect("RegionManager::tick_region: unknown region id");
            executor.tick_region(&mut region.state, pool, transport)
        };
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let outcome = self.record_synthetic_tick(id, elapsed_ms, transport);
        (report, outcome)
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
        let just_hit_split = {
            let region = self
                .regions
                .get_mut(&id)
                .expect("RegionManager::record_synthetic_tick: unknown region id");
            region.record_tick_duration(sample_ms)
        };

        if just_hit_split {
            let cell_count = self
                .regions
                .get(&id)
                .expect("region just borrowed above")
                .cells()
                .len();
            if cell_count >= 2 {
                return self.execute_split(id, transport);
            }
            // A single-cell region cannot split -- silently skipped (Context /
            // Acceptance test `single_cell_region_cannot_split_and_is_silently_skipped`).
        }

        // Merge evaluation: only the smaller-`RegionId` side of each adjacent pair
        // ever tracks/evaluates that pair's merge counter (Context: "Who evaluates a
        // merge"), in ascending neighbor order for determinism.
        let mut neighbors = self.neighbors_of(id);
        neighbors.sort();
        for neighbor in neighbors {
            if id >= neighbor {
                continue;
            }
            let self_ewma = self
                .regions
                .get(&id)
                .and_then(|region| region.tick_duration_ewma_ms());
            let neighbor_ewma = self
                .regions
                .get(&neighbor)
                .and_then(|region| region.tick_duration_ewma_ms());
            let (Some(self_ewma), Some(neighbor_ewma)) = (self_ewma, neighbor_ewma) else {
                continue; // one side has no sample yet -- nothing to combine
            };
            let combined = self_ewma + neighbor_ewma;

            let just_hit_merge = self
                .regions
                .get_mut(&id)
                .expect("region just borrowed above")
                .update_merge_candidate(neighbor, combined);
            if just_hit_merge {
                return self.execute_merge(id, neighbor, transport);
            }
        }

        LifecycleOutcome::None
    }

    /// Bypasses hysteresis entirely and executes a split immediately. Panics if `id` is
    /// unknown or owns fewer than 2 cells.
    pub fn force_split(&mut self, id: RegionId, transport: &dyn Transport) -> LifecycleOutcome {
        let cell_count = self
            .regions
            .get(&id)
            .expect("RegionManager::force_split: unknown region id")
            .cells()
            .len();
        assert!(
            cell_count >= 2,
            "RegionManager::force_split: region {id:?} owns fewer than 2 cells"
        );
        self.execute_split(id, transport)
    }

    /// Bypasses hysteresis entirely and executes a merge immediately. Panics if `a`/`b`
    /// are unknown or not currently adjacent.
    pub fn force_merge(
        &mut self,
        a: RegionId,
        b: RegionId,
        transport: &dyn Transport,
    ) -> LifecycleOutcome {
        assert!(
            self.regions.contains_key(&b),
            "RegionManager::force_merge: unknown region id {b:?}"
        );
        let neighbors = {
            let region_a = self
                .regions
                .get(&a)
                .unwrap_or_else(|| panic!("RegionManager::force_merge: unknown region id {a:?}"));
            self.directory.adjacent_regions(a, region_a.cells())
        };
        assert!(
            neighbors.contains(&b),
            "RegionManager::force_merge: regions {a:?} and {b:?} are not adjacent"
        );
        self.execute_merge(a, b, transport)
    }

    /// The Region Lifecycle Sync Operation's merge protocol (Context), resolving 01's
    /// Open Question on an in-flight message at the exact tick a merge reassigns
    /// ownership: drain both queues completely, redirect every `Address::Region(a|b)`
    /// message to the fresh `new_id` (any other `Address` value is left untouched),
    /// re-send through `transport` (never a direct inbox mutation), then build the
    /// merged region with load-conserved `SyntheticLoadProfile`.
    fn execute_merge(
        &mut self,
        a: RegionId,
        b: RegionId,
        transport: &dyn Transport,
    ) -> LifecycleOutcome {
        let new_id = self.id_alloc.alloc();

        let mut drained = drain_all(transport, a);
        drained.extend(drain_all(transport, b));
        for msg in drained {
            let to = match msg.to {
                Address::Region(r) if r == a || r == b => Address::Region(new_id),
                other => other,
            };
            let _ = transport.send(Message { to, ..msg });
        }

        let region_a = self
            .regions
            .remove(&a)
            .expect("execute_merge: region a must exist");
        let region_b = self
            .regions
            .remove(&b)
            .expect("execute_merge: region b must exist");

        let dimension = region_a.dimension();
        let mut cells: BTreeSet<GridCell> = region_a.cells().clone();
        cells.extend(region_b.cells().iter().copied());

        let load_of = |region: &ManagedRegion| {
            region
                .state
                .world
                .get_resource::<SyntheticLoadProfile>()
                .map(|profile| profile.busy_work_micros)
                .unwrap_or(0)
        };
        let combined_load = load_of(&region_a) + load_of(&region_b);

        let new_state = self.executor.spawn_region(new_id);
        let mut new_region =
            ManagedRegion::new(new_state, dimension, cells.clone(), self.tick_budget_ms);
        new_region
            .state
            .world
            .insert_resource(SyntheticLoadProfile {
                busy_work_micros: combined_load,
            });

        for &cell in &cells {
            self.directory.unassign(cell);
            self.directory.assign(cell, new_id);
        }
        self.regions.insert(new_id, new_region);

        LifecycleOutcome::Merged {
            old_a: a,
            old_b: b,
            new: new_id,
        }
    }

    /// The Region Lifecycle Sync Operation's split protocol (Context): pick the
    /// largest-balanced connectivity cut, drain `old`'s queue, redirect every
    /// `Address::Region(old)` message to `new_a` (the size->= fragment -- the
    /// documented fallback for a bare region-addressed message, M0 never emits an
    /// `Address::Chunk` payload to resolve more precisely), re-send through
    /// `transport`, then build both fragment regions with cell-fraction-conserved
    /// `SyntheticLoadProfile`.
    fn execute_split(&mut self, old: RegionId, transport: &dyn Transport) -> LifecycleOutcome {
        let (bigger, smaller, dimension, old_len, old_micros) = {
            let region_old = self
                .regions
                .get(&old)
                .expect("execute_split: region must exist");
            let (bigger, smaller) = largest_connectivity_cut(region_old.cells());
            let old_micros = region_old
                .state
                .world
                .get_resource::<SyntheticLoadProfile>()
                .map(|profile| profile.busy_work_micros)
                .unwrap_or(0);
            (
                bigger,
                smaller,
                region_old.dimension(),
                region_old.cells().len(),
                old_micros,
            )
        };

        let new_a = self.id_alloc.alloc();
        let new_b = self.id_alloc.alloc();

        let drained = drain_all(transport, old);
        for msg in drained {
            let to = match msg.to {
                Address::Region(r) if r == old => Address::Region(new_a),
                other => other,
            };
            let _ = transport.send(Message { to, ..msg });
        }

        self.regions.remove(&old);

        let new_a_micros =
            ((old_micros as f64) * (bigger.len() as f64) / (old_len as f64)).round() as u64;
        let new_b_micros = old_micros - new_a_micros;

        let state_a = self.executor.spawn_region(new_a);
        let mut managed_a =
            ManagedRegion::new(state_a, dimension, bigger.clone(), self.tick_budget_ms);
        managed_a.state.world.insert_resource(SyntheticLoadProfile {
            busy_work_micros: new_a_micros,
        });

        let state_b = self.executor.spawn_region(new_b);
        let mut managed_b =
            ManagedRegion::new(state_b, dimension, smaller.clone(), self.tick_budget_ms);
        managed_b.state.world.insert_resource(SyntheticLoadProfile {
            busy_work_micros: new_b_micros,
        });

        for &cell in &bigger {
            self.directory.unassign(cell);
            self.directory.assign(cell, new_a);
        }
        for &cell in &smaller {
            self.directory.unassign(cell);
            self.directory.assign(cell, new_b);
        }

        self.regions.insert(new_a, managed_a);
        self.regions.insert(new_b, managed_b);

        LifecycleOutcome::Split { old, new_a, new_b }
    }
}

/// Drains `into`'s entire inbox queue via `Transport::try_recv`, in receive order.
fn drain_all(transport: &dyn Transport, into: RegionId) -> Vec<Message<RegionMessage>> {
    let mut out = Vec::new();
    while let Some(msg) = transport.try_recv(into) {
        out.push(msg);
    }
    out
}
