//! Implements M3-B04's `ContainerSignalSource` for the tier-1 block-entity set (Context:
//! "Wiring into M3-B04's `ContainerSignalSource`" — closing that blueprint's own comparator
//! seam). One instance per region, constructed once by the composition root and shared — via
//! two independent `Arc` clones — with both `ComparatorBehavior::new` (M3-B04, Stage 4, read
//! side) and Stage 7's own driver (this blueprint, write side). The `Mutex` is never actually
//! contended: Stage 4 and Stage 7 run strictly sequentially within one region's own tick
//! (M0-B05's pipeline-stage ordering), the same "required only to satisfy a trait bound"
//! rationale M3-B04's own Context §I/§I½ already gives for its comparable per-region
//! `Mutex`/`OnceLock` fields.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;

use crate::redstone::ContainerSignalSource;

pub struct Tier1ContainerSignalSource {
    signals: Mutex<HashMap<BlockPos, u8>>,
    /// Section C (M3 field-report fix): positions whose `record`ed signal actually changed
    /// (including a position's first-ever `record`) since the last `take_changed` call -- the
    /// minimal parity-faithful stand-in for vanilla's `BlockEntity.setChanged ->
    /// updateNeighbourForOutputSignal` push (docs/findings-for-planning.md's own "Stage7->
    /// Stage4 container notify" entry). A plain `HashSet`, not a queue -- `record` is called at
    /// most once per position per Stage-7 pass, so a position needs at most one re-evaluation
    /// per pass regardless.
    changed: Mutex<HashSet<BlockPos>>,
}

impl Tier1ContainerSignalSource {
    pub fn new() -> Self {
        Self {
            signals: Mutex::new(HashMap::new()),
            changed: Mutex::new(HashSet::new()),
        }
    }

    /// Called once per tier-1 block entity, every Stage-7 pass (Deliverables,
    /// `run_block_entity_tick`), overwriting `pos`'s cached signal with the value that
    /// entity's own `comparator_signal()` method returns this tick. A position with no tier-1
    /// container present is never written (Stage 7 only ever visits real block entities) —
    /// combined with `container_signal`'s own `None`-for-absent contract below, a position
    /// stays unread by any comparator until the first Stage-7 pass after it is created (a
    /// documented, bounded, at-most-one-tick latency — Context).
    ///
    /// Section C (M3 field-report fix): also records `pos` into `changed` whenever the new
    /// `signal` actually differs from whatever was previously stored there (or nothing was
    /// stored yet at all) — `take_changed`'s own doc comment has the read side.
    pub fn record(&self, pos: BlockPos, signal: u8) {
        let previous = self.signals.lock().unwrap().insert(pos, signal);
        if previous != Some(signal) {
            self.changed.lock().unwrap().insert(pos);
        }
    }

    /// Removes a position's cached entry. Not called by anything in this blueprint (no
    /// block-entity removal exists yet, M3-B01's own already-established placement/removal
    /// gap) — provided for a future removal-pipeline blueprint to call alongside its own
    /// block-entity despawn, so a stale signal never outlives the container it described.
    pub fn forget(&self, pos: BlockPos) {
        self.signals.lock().unwrap().remove(&pos);
    }

    /// Section C (M3 field-report fix): drains and returns every position `record` has marked
    /// changed since the last call to this method — the minimal parity-faithful stand-in for
    /// vanilla's `BlockEntity.setChanged -> updateNeighbourForOutputSignal` push (docs/findings-
    /// for-planning.md's own "Stage7->Stage4 container notify" entry). The caller (the replay
    /// driver's own per-tick loop, `crates/testing/gametest/src/replay.rs`) calls this once,
    /// right after `run_block_entity_tick`, and fires `signal::notify_neighbor_changed_only` at
    /// each returned position — that helper's own one-hop conductor relay already covers "a
    /// comparator reads straight off the container" and "a comparator reads through a conductor
    /// the container also touches" identically, so no separate QC-relay logic is needed here.
    /// Order is unspecified (a plain `HashSet` drain) — Stage-4 dispatch settles every notified
    /// position to a fixed point regardless of the order notifications arrive in.
    pub fn take_changed(&self) -> Vec<BlockPos> {
        self.changed.lock().unwrap().drain().collect()
    }
}

impl Default for Tier1ContainerSignalSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerSignalSource for Tier1ContainerSignalSource {
    fn container_signal(&self, pos: BlockPos) -> Option<u8> {
        self.signals.lock().unwrap().get(&pos).copied()
    }
}

/// The `bevy_ecs::Resource`-carrying wrapper around the region's own
/// `Tier1ContainerSignalSource` (Context: "Composition-root sequencing this fix requires" —
/// inserted by the composition root, like `WorldSeed`, since no uniform default exists;
/// `bootstrap_default_stage7_resources` does *not* insert this one). `register_stage7`'s
/// system reads it via `Res<ContainerSignalsResource>`.
#[derive(Resource, Clone)]
pub struct ContainerSignalsResource(pub Arc<Tier1ContainerSignalSource>);
