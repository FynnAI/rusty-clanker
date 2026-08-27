//! Redstone wire — classic (default) power evaluator (MECH-D11/D12, Context §D).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct WireConnections {
    pub west: bool,
    pub east: bool,
    pub north: bool,
    pub south: bool,
}

/// Per-position wire state (Context §I): `0..=15` power plus horizontal connectivity.
#[derive(Copy, Clone, Debug, Default)]
struct WireState {
    power: u8,
    connections: WireConnections,
}

/// Redstone wire (Context §D). One instance per region (Context §I).
pub struct WireBehavior {
    state: Mutex<HashMap<BlockPos, WireState>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl WireBehavior {
    pub fn new() -> Self {
        todo!()
    }

    /// Current stored power (`0` for a never-yet-computed position — matches vanilla's own
    /// freshly-placed-wire default of `0`).
    pub fn power(&self, pos: BlockPos) -> u8 {
        todo!()
    }

    pub fn connections(&self, pos: BlockPos) -> WireConnections {
        todo!()
    }

    /// Test/composition-root-only: directly sets this position's own stored connectivity,
    /// bypassing `on_shape_update`'s own connectivity recomputation — this blueprint's own
    /// `wire_output_is_gated_by_connections_horizontally_only` acceptance test needs to force a
    /// specific connectivity value without depending on a particular neighbor arrangement (a
    /// minimal, necessary addition beyond Context §I½'s own literal deliverable listing, which
    /// names no such setter — documented as a deviation in the completion report).
    pub fn set_connections(&self, pos: BlockPos, connections: WireConnections) {
        todo!()
    }

    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>) {
        todo!()
    }

    fn registry(&self) -> &Arc<SignalSourceRegistry> {
        self.registry
            .get()
            .expect("WireBehavior: bind_registry must run before dispatch")
    }
}

impl Default for WireBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl RedstoneSignalSource for WireBehavior {
    /// Context §D output geometry: gated on `connections`, horizontal only.
    fn weak_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        todo!()
    }
    /// `Down` only, unconditional on power (Context §A's worked QC example).
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        todo!()
    }
    fn is_signal_source(&self) -> bool {
        true
    }
    fn raw_wire_power(&self, _world: &dyn BlockWorldAccess, pos: BlockPos) -> Option<u8> {
        Some(self.power(pos))
    }
}

impl BlockBehavior for WireBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        todo!()
    }
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        _from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        todo!()
    }
}
