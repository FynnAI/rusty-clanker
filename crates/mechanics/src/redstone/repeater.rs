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
        Self {
            facing: HashMap::new(),
            state: Mutex::new(HashMap::new()),
            registry: OnceLock::new(),
        }
    }

    /// Test/composition-root-only: establishes a repeater's fixed facing and delay setting
    /// (placement is out of this blueprint's scope, Context §F).
    pub fn place(&mut self, pos: BlockPos, facing: Direction, delay_setting: u8) {
        self.facing.insert(pos, facing);
        self.state.get_mut().unwrap().insert(
            pos,
            RepeaterState {
                powered: false,
                delay_setting,
            },
        );
    }

    pub fn facing(&self, pos: BlockPos) -> Direction {
        *self
            .facing
            .get(&pos)
            .expect("RepeaterBehavior::facing: position was never placed")
    }

    pub fn delay_setting(&self, pos: BlockPos) -> u8 {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.delay_setting)
            .unwrap_or(1)
    }

    pub fn get_delay(&self, pos: BlockPos) -> u64 {
        self.delay_setting(pos) as u64 * 2
    }

    pub fn powered(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.powered)
            .unwrap_or(false)
    }

    fn set_powered(&self, pos: BlockPos, value: bool) {
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(pos).or_insert(RepeaterState {
            powered: false,
            delay_setting: 1,
        });
        entry.powered = value;
    }

    /// Reads the registry via `self.registry()` (Context §I½) — no longer takes a `registry`
    /// parameter; a test calling this directly must call `bind_registry` first.
    pub fn is_locked(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        self.alternate_signal(world, pos) > 0
    }

    /// `alternate_signal` (Context §F): `max` of the two perpendicular side control-input
    /// readings, each gated by `sideInputDiodesOnly` (repeater always sets this `true`).
    fn alternate_signal(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> u8 {
        let facing = self.facing(pos);
        let (a, b) = signal::perpendicular_pair(facing);
        let registry = self.registry();
        control_input_signal(world, registry, pos, a)
            .max(control_input_signal(world, registry, pos, b))
    }

    fn should_prioritize(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        signal::should_prioritize_diode(world, self.registry(), pos, self.facing(pos))
    }

    fn base_input_positive(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        signal::base_diode_input_signal(world, self.registry(), pos, self.facing(pos)) > 0
    }

    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>) {
        self.registry
            .set(registry)
            .unwrap_or_else(|_| panic!("RepeaterBehavior::bind_registry called more than once"));
    }

    fn registry(&self) -> &Arc<SignalSourceRegistry> {
        self.registry
            .get()
            .expect("RepeaterBehavior: bind_registry must run before dispatch")
    }
}

/// `control_input_signal` (Context §F): `sideInputDiodesOnly` -- `0` unless the neighbor at
/// `side.apply(pos)` is itself a diode, in which case its full `emitted_toward` reading back
/// toward `pos` counts.
fn control_input_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    side: Direction,
) -> u8 {
    let neighbor_pos = side.apply(pos);
    match world.get_block(neighbor_pos) {
        Some(state) if registry.resolve(state).is_diode() => {
            signal::emitted_toward(world, registry, neighbor_pos, side.opposite())
        }
        _ => 0,
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
        if self.powered(pos) && towards == self.facing(pos) {
            15
        } else {
            0
        }
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
    /// `checkTickOnNeighbor` (Context §F).
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        if self.is_locked(ctx.world, pos) {
            return;
        }
        let should = self.base_input_positive(ctx.world, pos);
        let powered = self.powered(pos);
        if powered != should && !ctx.scheduled.is_block_tick_pending(pos) {
            let priority = if self.should_prioritize(ctx.world, pos) {
                TickPriority::ExtremelyHigh
            } else if powered {
                TickPriority::VeryHigh
            } else {
                TickPriority::High
            };
            ctx.schedule_block_tick(pos, self.get_delay(pos), priority);
        }
    }

    /// `tick` (Context §F), restated as an explicit two-phase state machine: turning off is
    /// gated on the live `should` value; turning on is unconditional once reached (a scheduled
    /// tick only ever fires because *some* earlier call found a mismatch, Context §F's own
    /// "turn-on is immediate too" framing) -- then immediately re-checked so a since-ended short
    /// input pulse still self-schedules a matching turn-off at this repeater's own fixed delay
    /// width, rather than being silently swallowed.
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        if self.is_locked(ctx.world, pos) {
            return;
        }
        if self.powered(pos) {
            if !self.base_input_positive(ctx.world, pos) {
                self.set_powered(pos, false);
                signal::notify_neighbor_changed_only(ctx, pos);
            }
        } else {
            self.set_powered(pos, true);
            signal::notify_neighbor_changed_only(ctx, pos);
            if !self.base_input_positive(ctx.world, pos) {
                ctx.schedule_block_tick(pos, self.get_delay(pos), TickPriority::VeryHigh);
            }
        }
    }
}
