//! Implements M3-B04's `ContainerSignalSource` for the tier-1 block-entity set (Context:
//! "Wiring into M3-B04's `ContainerSignalSource`" — closing that blueprint's own comparator
//! seam). One instance per region, constructed once by the composition root and shared — via
//! two independent `Arc` clones — with both `ComparatorBehavior::new` (M3-B04, Stage 4, read
//! side) and Stage 7's own driver (this blueprint, write side). The `Mutex` is never actually
//! contended: Stage 4 and Stage 7 run strictly sequentially within one region's own tick
//! (M0-B05's pipeline-stage ordering), the same "required only to satisfy a trait bound"
//! rationale M3-B04's own Context §I/§I½ already gives for its comparable per-region
//! `Mutex`/`OnceLock` fields.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;

use crate::redstone::ContainerSignalSource;

pub struct Tier1ContainerSignalSource {
    signals: Mutex<HashMap<BlockPos, u8>>,
}

impl Tier1ContainerSignalSource {
    pub fn new() -> Self {
        Self {
            signals: Mutex::new(HashMap::new()),
        }
    }

    /// Called once per tier-1 block entity, every Stage-7 pass (Deliverables,
    /// `run_block_entity_tick`), overwriting `pos`'s cached signal with the value that
    /// entity's own `comparator_signal()` method returns this tick. A position with no tier-1
    /// container present is never written (Stage 7 only ever visits real block entities) —
    /// combined with `container_signal`'s own `None`-for-absent contract below, a position
    /// stays unread by any comparator until the first Stage-7 pass after it is created (a
    /// documented, bounded, at-most-one-tick latency — Context).
    pub fn record(&self, pos: BlockPos, signal: u8) {
        self.signals.lock().unwrap().insert(pos, signal);
    }

    /// Removes a position's cached entry. Not called by anything in this blueprint (no
    /// block-entity removal exists yet, M3-B01's own already-established placement/removal
    /// gap) — provided for a future removal-pipeline blueprint to call alongside its own
    /// block-entity despawn, so a stale signal never outlives the container it described.
    pub fn forget(&self, pos: BlockPos) {
        self.signals.lock().unwrap().remove(&pos);
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
