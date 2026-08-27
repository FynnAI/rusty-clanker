//! Comparator — compare/subtract modes, container-fullness analog input (Context §G).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ComparatorMode {
    Compare,
    Subtract,
}

/// The interface boundary M3-B06 implements (Context §G). This blueprint's own tests supply a
/// `HashMap`-backed fake — see Acceptance tests.
pub trait ContainerSignalSource: Send + Sync {
    /// The vanilla analog signal `0..=15` a comparator reading `pos` should see, per the
    /// container-fullness formula (Context §G), or `None` if `pos` holds no tier-1 container
    /// (comparator falls back to `base_diode_input_signal`) — distinct from `Some(0)`, which
    /// means "an empty container is present."
    fn container_signal(&self, pos: BlockPos) -> Option<u8>;
}

/// The trivial default: no position is ever a container (used when no block-entity blueprint
/// has landed yet — the composition root's own safe fallback, not a test-only type).
pub struct NoContainers;
impl ContainerSignalSource for NoContainers {
    fn container_signal(&self, _pos: BlockPos) -> Option<u8> {
        None
    }
}

#[derive(Copy, Clone, Debug)]
struct ComparatorState {
    powered: bool,
    output: u8,
    mode: ComparatorMode,
}

/// Comparator (Context §G). One instance per region (Context §I).
pub struct ComparatorBehavior {
    facing: HashMap<BlockPos, Direction>,
    state: Mutex<HashMap<BlockPos, ComparatorState>>,
    containers: Arc<dyn ContainerSignalSource>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl ComparatorBehavior {
    pub fn new(containers: Arc<dyn ContainerSignalSource>) -> Self {
        todo!()
    }

    pub fn place(&mut self, pos: BlockPos, facing: Direction, mode: ComparatorMode) {
        todo!()
    }

    /// Test/composition-root-only mode toggle (Context §G — use-item mode cycling is out of
    /// scope, no item-use handling exists at M3).
    pub fn set_mode(&self, pos: BlockPos, mode: ComparatorMode) {
        todo!()
    }

    pub fn facing(&self, pos: BlockPos) -> Direction {
        todo!()
    }

    pub fn mode(&self, pos: BlockPos) -> ComparatorMode {
        todo!()
    }

    pub fn output(&self, pos: BlockPos) -> u8 {
        todo!()
    }

    pub fn powered(&self, pos: BlockPos) -> bool {
        todo!()
    }

    fn set_output(&self, pos: BlockPos, output: u8) {
        todo!()
    }

    fn set_powered(&self, pos: BlockPos, powered: bool) {
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
            .expect("ComparatorBehavior: bind_registry must run before dispatch")
    }

    fn get_input_signal(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        todo!()
    }

    fn side_input_signal(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        todo!()
    }

    /// `calculate_output_signal` (Context §G) — a pure function, exposed directly for the
    /// acceptance tests' own hand-derived table (see Acceptance tests) without needing a full
    /// `UpdateContext` to exercise it.
    pub fn calculate_output_signal(input: u8, side: u8, mode: ComparatorMode) -> u8 {
        todo!()
    }

    pub fn should_turn_on(input: u8, side: u8, mode: ComparatorMode) -> bool {
        todo!()
    }
}

impl RedstoneSignalSource for ComparatorBehavior {
    fn weak_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        todo!()
    }
    fn direct_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        self.weak_signal_toward(world, pos, towards)
    }
    fn is_signal_source(&self) -> bool {
        true
    }
    fn is_diode(&self) -> bool {
        true
    }
    fn connects_from(&self, _world: &dyn BlockWorldAccess, pos: BlockPos, from: Direction) -> bool {
        from == self.facing(pos) || from == self.facing(pos).opposite()
    }
    fn diode_facing(&self, pos: BlockPos) -> Option<Direction> {
        Some(self.facing(pos))
    }
}

impl BlockBehavior for ComparatorBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        todo!()
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        todo!()
    }
}
