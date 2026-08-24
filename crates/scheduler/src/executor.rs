//! RC-Executor: the built, immutable conflict graph plus the per-region tick driver.

use std::collections::HashSet;

use bevy_ecs::component::ComponentId;

use crate::access::ComponentAccessSummary;
use crate::registry::SystemFactory;

struct CompiledSystem {
    factory: SystemFactory,
    access: ComponentAccessSummary,
    structural_writes: HashSet<ComponentId>,
}

struct CompiledGroup {
    systems: Vec<CompiledSystem>, // index == order_tag
    waves: Vec<Vec<usize>>,       // from compute_waves; ignored by Stage 4's dispatch
}

/// The built, immutable RC-Executor (ARCH-D8: conflict graph computed once,
/// "reused for every tick of every region"). `Send + Sync` — safe to share
/// (`&RcExecutor`) across multiple regions' ticks running concurrently on
/// different threads, a later blueprint's use case, not exercised here.
pub struct RcExecutor {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [CompiledGroup; 5],
}

/// Minimal per-tick result. Extended by later blueprints as needed (e.g. per-stage
/// timing for ARCH-D19's hotness EWMA) — not this blueprint's scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickReport {
    pub tick_counter: u64,
}

impl RcExecutor {
    /// Creates a fresh region: a new `World` (bootstrapped identically to the
    /// prototype `World` used at build time), one freshly-`.initialize`d instance
    /// of every registered system, zeroed tick counter, empty `RegionMessageState`.
    pub fn spawn_region(&self, id: rc_messaging::RegionId) -> crate::region::RegionState {
        let _ = id;
        let _ = self.bootstrap;
        let _ = &self.groups;
        todo!()
    }

    /// Advances `region` through the fixed 11-stage pipeline exactly once
    /// (ARCH-D12), dispatching each domain group's waves onto `pool`, applying the
    /// two ARCH-D9 sync points with Stage 4's inline exception, and fulfilling
    /// M0-B02's exact Stage-1/Stage-10 driver contract against `transport`.
    /// Synchronous — this is the "synchronous test-mode tick driver" shape
    /// `09-testing-quality.md`'s TEST-D14 describes, bypassing real-time EDF
    /// admission entirely; a later blueprint wraps this in the wall-clock-paced,
    /// multi-region 20 TPS loop (out of scope here).
    pub fn tick_region(
        &self,
        region: &mut crate::region::RegionState,
        pool: &crate::pool::RcWorkerPool,
        transport: &dyn rc_messaging::Transport,
    ) -> TickReport {
        let _ = (region, pool, transport);
        todo!()
    }
}
