//! Repeater — delay, boolean lock, priority selection (Context §F).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug)]
struct RepeaterState {
    powered: bool,
    delay_setting: u8, // 1..=4
}

/// Repeater (Context §F). One instance per region (Context §I).
pub struct RepeaterBehavior {
    facing: HashMap<BlockPos, Direction>, // set once, at placement time — this blueprint's tests seed it directly
    state: Mutex<HashMap<BlockPos, RepeaterState>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body and by `is_locked`
    /// (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl RepeaterBehavior {
    pub fn new() -> Self {
        todo!()
    }

    /// Test/composition-root-only: establishes a repeater's fixed facing and delay setting
    /// (placement is out of this blueprint's scope, Context §F).
    pub fn place(&mut self, pos: BlockPos, facing: Direction, delay_setting: u8) {
        todo!()
    }

    pub fn facing(&self, pos: BlockPos) -> Direction {
        todo!()
    }

    pub fn delay_setting(&self, pos: BlockPos) -> u8 {
        todo!()
    }

    pub fn get_delay(&self, pos: BlockPos) -> u64 {
        self.delay_setting(pos) as u64 * 2
    }

    pub fn powered(&self, pos: BlockPos) -> bool {
        todo!()
    }

    fn set_powered(&self, pos: BlockPos, value: bool) {
        todo!()
    }

    /// Reads the registry via `self.registry()` (Context §I½) — no longer takes a `registry`
    /// parameter; a test calling this directly must call `bind_registry` first.
    pub fn is_locked(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        todo!()
    }

    fn alternate_signal(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> u8 {
        todo!()
    }

    fn should_prioritize(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        todo!()
    }

    fn base_input_positive(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
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
            .expect("RepeaterBehavior: bind_registry must run before dispatch")
    }
}

impl Default for RepeaterBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl RedstoneSignalSource for RepeaterBehavior {
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

impl BlockBehavior for RepeaterBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        todo!()
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        todo!()
    }
}
