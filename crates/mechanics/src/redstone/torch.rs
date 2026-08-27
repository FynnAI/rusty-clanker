//! Redstone torch — inverter, quasi-connectivity input, burnout (Context §E).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TorchAttachment {
    Floor,
    Wall(Direction),
}

impl TorchAttachment {
    /// The direction this torch reads its input from (Context §E).
    pub fn input_direction(self) -> Direction {
        match self {
            TorchAttachment::Floor => Direction::Down,
            TorchAttachment::Wall(facing) => facing.opposite(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct TorchState {
    lit: bool,
    burnt_out: bool,
}

/// Redstone torch (Context §E). One instance per region (Context §I).
pub struct TorchBehavior {
    attachment: TorchAttachment,
    state: Mutex<HashMap<BlockPos, TorchState>>,
    recent_toggles: Mutex<HashMap<BlockPos, VecDeque<u64>>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl TorchBehavior {
    pub const RECENT_TOGGLE_TIMER: u64 = 60;
    pub const MAX_RECENT_TOGGLES: usize = 8;
    pub const RESTART_DELAY: u64 = 160;
    pub const REEVAL_DELAY: u64 = 2;

    pub fn new(attachment: TorchAttachment) -> Self {
        Self {
            attachment,
            state: Mutex::new(HashMap::new()),
            recent_toggles: Mutex::new(HashMap::new()),
            registry: OnceLock::new(),
        }
    }

    /// `true` if never observed (matches vanilla's own freshly-placed-lit default, Context §E).
    pub fn lit(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.lit)
            .unwrap_or(true)
    }

    /// Pure query, no mutation (Context §E's "out of scope, flagged" support-loss note) —
    /// `true` iff this floor torch's support block is currently not a conductor.
    pub fn should_pop(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        let support = self.attachment.input_direction().apply(pos);
        !signal::is_conductor(world, support)
    }

    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>) {
        self.registry
            .set(registry)
            .unwrap_or_else(|_| panic!("TorchBehavior::bind_registry called more than once"));
    }

    fn registry(&self) -> &Arc<SignalSourceRegistry> {
        self.registry
            .get()
            .expect("TorchBehavior: bind_registry must run before dispatch")
    }

    fn has_neighbor_signal(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        signal::has_signal(
            world,
            self.registry(),
            pos,
            self.attachment.input_direction(),
        )
    }

    fn is_burnt_out(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.burnt_out)
            .unwrap_or(false)
    }

    fn set_burnt_out(&self, pos: BlockPos, value: bool) {
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_insert(TorchState {
                lit: true,
                burnt_out: false,
            })
            .burnt_out = value;
    }

    fn set_lit(&self, pos: BlockPos, value: bool) {
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_insert(TorchState {
                lit: true,
                burnt_out: false,
            })
            .lit = value;
    }

    /// Prunes entries older than `RECENT_TOGGLE_TIMER`, pushes `current_tick`, returns the
    /// resulting count (Context §E burnout paragraph).
    fn record_and_prune_toggle(&self, pos: BlockPos, current_tick: u64) -> usize {
        let mut toggles = self.recent_toggles.lock().unwrap();
        let entries = toggles.entry(pos).or_default();
        entries.retain(|&t| current_tick.saturating_sub(t) <= Self::RECENT_TOGGLE_TIMER);
        entries.push_back(current_tick);
        entries.len()
    }

    /// The reeval logic shared by both the ordinary 2-tick re-check and the burnout-restart
    /// tick (Context §E: "that tick itself still respects the ordinary ... flip logic").
    fn reeval_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let target_lit = !self.has_neighbor_signal(ctx.world, pos);
        let current_lit = self.lit(pos);
        if current_lit == target_lit {
            return;
        }
        self.set_lit(pos, target_lit);
        if !target_lit {
            let count = self.record_and_prune_toggle(pos, ctx.current_tick);
            if count >= Self::MAX_RECENT_TOGGLES {
                self.set_burnt_out(pos, true);
                ctx.schedule_block_tick(pos, Self::RESTART_DELAY, TickPriority::Normal);
            }
        }
        signal::notify_neighbor_changed_only(ctx, pos);
    }
}

impl RedstoneSignalSource for TorchBehavior {
    fn weak_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        if !self.lit(pos) {
            return 0;
        }
        if towards == self.attachment.input_direction() {
            0
        } else {
            15
        }
    }
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        if self.lit(pos) && towards == self.attachment.input_direction().opposite() {
            15
        } else {
            0
        }
    }
    fn is_signal_source(&self) -> bool {
        true
    }
}

impl BlockBehavior for TorchBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        if self.is_burnt_out(pos) {
            return;
        }
        let target_lit = !self.has_neighbor_signal(ctx.world, pos);
        let current_lit = self.lit(pos);
        if current_lit != target_lit && !ctx.scheduled.is_block_tick_pending(pos) {
            ctx.schedule_block_tick(pos, Self::REEVAL_DELAY, TickPriority::Normal);
        }
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        // A pending restart tick (burnout) always clears the flag before the ordinary reeval
        // logic runs -- a no-op when not currently burnt out (Context §E: "that tick itself
        // still respects the ordinary ... flip logic").
        self.set_burnt_out(pos, false);
        self.reeval_tick(ctx, pos);
    }
    fn on_shape_update(
        &self,
        _ctx: &mut UpdateContext,
        _pos: BlockPos,
        _from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        None // Context §E: detection only, via `should_pop`; no mutation here.
    }
}
