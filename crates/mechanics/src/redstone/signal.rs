//! The shared power-query substrate every tier-1 redstone component builds on (Context §A/§C):
//! the one quasi-connectivity primitive (`emitted_toward`/`direct_signal_to`/`signal_into`/
//! `best_neighbor_signal`), the `RedstoneSignalSource` trait plus its range-based registry, and
//! the cross-region-aware neighbor-changed-only notify (Context §I) every component's own
//! state-change propagation routes through instead of `UpdateContext::set_block`.

use std::sync::Arc;

use rc_chunk_storage::{BlockStateId, RegistryId};
use rc_core::BlockPos;
use rc_physics::Vec3;

use crate::behavior::UpdateContext;
use crate::direction::{Direction, NEIGHBOR_CHANGED_ORDER};
use crate::neighbor_update::PendingUpdate;
use crate::scheduled_tick::TickPriority;
use crate::world_access::BlockWorldAccess;
use rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, RegionMessage};

/// The power-query trait every tier-1 redstone `BlockBehavior` also implements (Context §C).
/// Every default is `0`/`false` — the shared `NoSignalSource` default for any block-state id
/// with no registered redstone behavior at all (ordinary terrain).
pub trait RedstoneSignalSource: Send + Sync {
    /// Weak output `pos` delivers toward `towards` — what a non-conductor neighbor reads.
    fn weak_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        0
    }
    /// Strong/direct output — what a *conductor* resting against `pos` reads via `direct_signal_to`.
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        0
    }
    /// `true` for every tier-1 component (wire/torch/repeater/comparator); `false` for
    /// `NoSignalSource` — used by wire's own default `connects_from`.
    fn is_signal_source(&self) -> bool {
        false
    }
    /// `true` only for `RepeaterBehavior`/`ComparatorBehavior` — the single "am I a diode"
    /// predicate `sideInputDiodesOnly`'s filter and `should_prioritize`'s behind-block check
    /// both share (Context §F), rather than two independently-invented names for the same
    /// concept.
    fn is_diode(&self) -> bool {
        false
    }
    /// Whether this component connects to a wire approaching it from `from` (Context §D/§C).
    /// Default: any signal source connects from any direction (correct for wire and torch);
    /// `RepeaterBehavior`/`ComparatorBehavior` override this to their own front/back axis only.
    fn connects_from(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        _from: Direction,
    ) -> bool {
        self.is_signal_source()
    }
    /// Only `WireBehavior` overrides this (Context §F/§C): a diode's `get_input_signal` reads
    /// a wire neighbor's raw stored power directly, bypassing `weak_signal_toward`, when it is
    /// higher than the plain signal read.
    fn raw_wire_power(&self, _world: &dyn BlockWorldAccess, _pos: BlockPos) -> Option<u8> {
        None
    }
    /// The diode's own facing direction — `None` for any non-directional source (wire, torch,
    /// `NoSignalSource`); `Some(facing)` for `RepeaterBehavior`/`ComparatorBehavior`. A second
    /// deliberately-special-cased hook alongside `raw_wire_power` (not itself part of Context
    /// §C's literal trait listing — added because `RepeaterBehavior::should_prioritize`'s own
    /// "is the behind-diode feeding straight through" check, Context §F, needs to compare a
    /// neighbor diode's *specific* facing value, which `connects_from`'s axis-only predicate
    /// cannot recover since it deliberately answers `true` for both a diode's front and back).
    fn diode_facing(&self, _pos: BlockPos) -> Option<Direction> {
        None
    }
}

/// The shared default for every unregistered block-state id (ordinary terrain) — mirrors
/// `rc_mechanics::behavior::NoOpBehavior`'s identical role for `BlockBehavior`.
pub struct NoSignalSource;
impl RedstoneSignalSource for NoSignalSource {}

/// Range-based registry (Context §C), mirroring B01's `BlockBehaviorRegistry`'s exact shape —
/// a distinct type since it stores a different trait object, not a generic wrapper over it.
pub struct SignalSourceRegistry {
    ranges: Vec<(BlockStateId, BlockStateId, Arc<dyn RedstoneSignalSource>)>,
    default: Arc<dyn RedstoneSignalSource>,
}

impl SignalSourceRegistry {
    pub fn new() -> Self {
        todo!()
    }

    /// Panics on overlap with an already-registered range (identical contract to B01's
    /// `BlockBehaviorRegistry::register_range`).
    pub fn register_range(
        &mut self,
        start: BlockStateId,
        end_exclusive: BlockStateId,
        source: Arc<dyn RedstoneSignalSource>,
    ) {
        todo!()
    }

    pub fn resolve(&self, state: BlockStateId) -> &Arc<dyn RedstoneSignalSource> {
        todo!()
    }
}

impl Default for SignalSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// `is_conductor` (Context §B): reuses `rc_physics::tier1_shape_table()` directly — `true` iff
/// the block at `pos` (or air/unloaded, which is never a conductor) has a shape equal to
/// exactly one box spanning `(0,0,0)..(1,1,1)`.
pub fn is_conductor(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
    todo!()
}

/// `emitted_toward` (Context §A) — the one shared quasi-connectivity primitive.
pub fn emitted_toward(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    towards: Direction,
) -> u8 {
    todo!()
}

/// `direct_signal_to` (Context §A) — all 6 faces of the conductor at `pos`.
pub fn direct_signal_to(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
) -> u8 {
    todo!()
}

/// `signal_into` (Context §A) — what `pos` receives from its neighbor in `from`.
pub fn signal_into(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    from: Direction,
) -> u8 {
    todo!()
}

/// `best_neighbor_signal` (Context §A) — max over all 6 sides.
pub fn best_neighbor_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
) -> u8 {
    todo!()
}

/// `has_signal(pos, from) = signal_into(pos, from) > 0` — a thin boolean convenience, used
/// directly by torch's own input check and available to M3-B05 (piston) for its own QC input.
pub fn has_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    from: Direction,
) -> bool {
    todo!()
}

/// The shared repeater/comparator "front-face signal, raised to a wire neighbor's raw power"
/// helper (Context §F, research §3.6's own `getInputSignal` base — `RedstoneSignalSource::
/// raw_wire_power` is the special-cased wire-bypass hook).
pub fn base_diode_input_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    facing: Direction,
) -> u8 {
    todo!()
}

/// Cross-region-aware neighbor-changed-ONLY notify (Context §I) — every tier-1 component's
/// own state-change propagation goes through this, never a bare `ctx.engine.emit_*` call and
/// never `UpdateContext::set_block` (which would also fire an unwanted shape-update pass).
pub fn notify_neighbor_changed_only(ctx: &mut UpdateContext, at: BlockPos) {
    todo!()
}

/// The two horizontal directions perpendicular to `facing`'s own axis, in a fixed (but
/// otherwise arbitrary — every caller takes the `max` of both) order (Context §F: repeater's
/// `alternate_signal`; Context §G: comparator's side-input reading). Shared between
/// `repeater.rs` and `comparator.rs`.
pub(crate) fn perpendicular_pair(facing: Direction) -> (Direction, Direction) {
    todo!()
}

/// `should_prioritize` (Context §F), generalized over any diode via `facing` rather than
/// requiring a concrete `RepeaterBehavior`/`ComparatorBehavior` — shared by both `checkTickOnNeighbor`
/// overrides (Context §F/§G: "the same shared `DiodeBlock`-base priority-selection logic").
pub(crate) fn should_prioritize_diode(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    facing: Direction,
) -> bool {
    todo!()
}

/// The full `checkTickOnNeighbor` priority selection (Context §F, 3-way): `ExtremelyHigh` if
/// `should_prioritize_diode`, else `VeryHigh` if `currently_powered`, else `High`.
pub(crate) fn diode_priority(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    facing: Direction,
    currently_powered: bool,
) -> TickPriority {
    todo!()
}
