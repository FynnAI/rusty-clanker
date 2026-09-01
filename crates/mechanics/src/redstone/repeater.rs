//! Repeater — delay, boolean lock, priority selection (Context §F).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
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
    /// `true` once `seed_powered_from_world` has run for this position at least once (M3
    /// field-report fix, own-state writeback's own read-side companion). `place()`'s own
    /// "placement is out of this blueprint's scope" gap (Context §F) means `powered` always
    /// starts seeded `false` here, regardless of a fixture's real initial `POWERED` value --
    /// left uncorrected, that stale default would feed `is_locked`/`base_input_positive`
    /// (`weak_signal_toward`) for a locking *neighbor* repeater placed already-`powered=true`,
    /// producing a spurious few-tick-late "just turned on" transition purely from this gap
    /// rather than any real signal change. `seed_powered_from_world` reconciles it against the
    /// position's own live raw id the first time this behavior ever dispatches for `pos` --
    /// exactly once, before any signal computation reads it -- and never again afterward, so
    /// every later `powered` value stays this behavior's own genuine computed transition.
    seeded: bool,
}

/// Own-state id arithmetic for `minecraft:repeater` (M3 field-report fix: own-state writeback;
/// WS-D15's generated per-property registry is future work, so this constant is read directly
/// off `datagen-output/26.2/generated/reports/blocks.json`'s own `minecraft:repeater` entry,
/// protocol 776, states 7034..=7097). `delay` (`[1,2,3,4]`, blocks.json order) is the
/// slowest-varying property, stride 16; then `facing` (`signal::diode_facing_index`), stride 4;
/// then `locked` (`[true,false]`), stride 2; then `powered` (`[true,false]`), stride 1:
/// `id = 7034 + (delay-1)*16 + facing_idx*4 + locked_idx*2 + powered_idx` (`locked_idx`/
/// `powered_idx`: `true` -> `0`, `false` -> `1`, blocks.json's own listed value order).
const REPEATER_BASE: u32 = 7034;
/// `air`'s own raw id (M3 field-report fix, Task 3) -- stable by protocol convention
/// (`wire.rs`'s/`piston.rs`'s own identical documented `AIR_ID` convention).
const AIR_ID: BlockStateId = BlockStateId(0);

fn repeater_state_id(
    delay_setting: u8,
    facing: Direction,
    locked: bool,
    powered: bool,
) -> BlockStateId {
    BlockStateId(
        REPEATER_BASE
            + (delay_setting as u32 - 1) * 16
            + signal::diode_facing_index(facing) * 4
            + u32::from(!locked) * 2
            + u32::from(!powered),
    )
}

/// Repeater (Context §F). One instance per region (Context §I).
pub struct RepeaterBehavior {
    /// M3 field-report fix (Task 2): interior-mutable (`&self`-callable `place`, matching
    /// `state`'s own pre-existing `Mutex` shape) — a real composition root has no way to update
    /// a repeater's own facing after this behavior has ever been registered into a shared
    /// `BlockBehaviorRegistry`/`SignalSourceRegistry` (both wrap it in an `Arc` immediately
    /// after construction), so `place` must be re-callable through a shared reference for a
    /// player (or a replay's own scripted action) breaking and re-placing a repeater to ever
    /// observe a new facing (`docs/findings-for-planning.md`'s own "diode re-placement" entry).
    facing: Mutex<HashMap<BlockPos, Direction>>,
    state: Mutex<HashMap<BlockPos, RepeaterState>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body and by `is_locked`
    /// (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl RepeaterBehavior {
    pub fn new() -> Self {
        Self {
            facing: Mutex::new(HashMap::new()),
            state: Mutex::new(HashMap::new()),
            registry: OnceLock::new(),
        }
    }

    /// Establishes (or, called again for an already-registered position, *replaces*) a
    /// repeater's facing and delay setting — a full replace-on-replace, resetting every other
    /// per-position field to its own fresh-placement default (`powered: false`, `seeded:
    /// false`, mirroring a genuinely new block replacing whatever previously occupied this
    /// position), not a partial update (M3 field-report fix, Task 2: placement-state seeding is
    /// now re-entrant — callable any number of times for the same position, including after this
    /// behavior has been wrapped in an `Arc` and registered).
    pub fn place(&self, pos: BlockPos, facing: Direction, delay_setting: u8) {
        self.facing.lock().unwrap().insert(pos, facing);
        self.state.lock().unwrap().insert(
            pos,
            RepeaterState {
                powered: false,
                delay_setting,
                seeded: false,
            },
        );
    }

    pub fn facing(&self, pos: BlockPos) -> Direction {
        *self
            .facing
            .lock()
            .unwrap()
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
            seeded: false,
        });
        entry.powered = value;
    }

    /// Own-state writeback's read-side companion (M3 field-report fix) -- `RepeaterState::
    /// seeded`'s own doc comment. Reads `pos`'s own currently-stored raw id (if any) and
    /// decodes its `powered` bit into the side-table, exactly once. Called at the top of every
    /// `BlockBehavior` entry point, before any signal computation reads `powered` (`is_locked`'s
    /// own `alternate_signal` -> `control_input_signal` -> `weak_signal_toward` chain, when
    /// `pos` is itself a *neighbor* another repeater's lock check reads).
    fn seed_powered_from_world(&self, world: &dyn BlockWorldAccess, pos: BlockPos) {
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(pos).or_insert(RepeaterState {
            powered: false,
            delay_setting: 1,
            seeded: false,
        });
        if entry.seeded {
            return;
        }
        entry.seeded = true;
        if let Some(current) = world.get_block(pos) {
            let bits = current.0.wrapping_sub(REPEATER_BASE) % 4;
            entry.powered = bits % 2 == 0;
        }
    }

    /// Reads the registry via `self.registry()` (Context §I½) — no longer takes a `registry`
    /// parameter; a test calling this directly must call `bind_registry` first.
    pub fn is_locked(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        self.alternate_signal(world, pos) > 0
    }

    /// `alternate_signal` (Context §F): `max` of the two perpendicular side control-input
    /// readings, each gated by `sideInputDiodesOnly` (repeater always sets this `true` —
    /// `signal::control_input_signal`'s own `only_diodes` parameter, shared with comparator's
    /// own `false`-gated side reading, M3 field-report fix Rule 1).
    fn alternate_signal(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> u8 {
        let facing = self.facing(pos);
        let (a, b) = signal::perpendicular_pair(facing);
        let registry = self.registry();
        signal::control_input_signal(world, registry, pos, a, true)
            .max(signal::control_input_signal(world, registry, pos, b, true))
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

    /// Own-state writeback (M3 field-report fix): writes `pos`'s own new `BlockStateId` via the
    /// raw world accessor, replacing only `locked_new`/`powered_new` (`None` = "leave this one
    /// bit exactly as `pos`'s own currently-stored raw id already has it") -- vanilla's own
    /// `RepeaterBlock`/`DiodeBlock` writes each touch exactly one property at a time
    /// (`state.setValue(LOCKED, ..)` from `neighborChanged`, `state.setValue(POWERED, ..)` from
    /// `tick`, never both together), both via `level.setBlock(pos, .., 2)` (flag 2 =
    /// clients-only, no cascading neighbor/shape update of their own -- this never goes through
    /// `ctx.set_block`, the real neighbor notify stays exactly whatever the caller already does
    /// via `signal::notify_neighbor_changed_only`). Reading the untouched bit back off the
    /// current raw id, rather than off this behavior's own `RepeaterState.powered` side-table,
    /// matters specifically for `powered`: `place()`'s own "placement is out of this
    /// blueprint's scope" gap (Context §F) means that side-table field is seeded `false`
    /// unconditionally, never from a block's real placed `POWERED` value -- blindly writing it
    /// back would silently clobber a fixture's genuinely-already-`powered=true` initial
    /// placement the moment this component's own settle-time `on_neighbor_changed` first fires.
    /// A position with no currently-stored raw id (not yet placed) is left untouched.
    fn write_state_id(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        locked_new: Option<bool>,
        powered_new: Option<bool>,
    ) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        let bits = (current.0.wrapping_sub(REPEATER_BASE)) % 4;
        let current_locked = bits / 2 == 0;
        let current_powered = bits % 2 == 0;
        let facing = self.facing(pos);
        let delay = self.delay_setting(pos);
        let id = repeater_state_id(
            delay,
            facing,
            locked_new.unwrap_or(current_locked),
            powered_new.unwrap_or(current_powered),
        );
        ctx.world.set_block(pos, id);
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
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        // Own-state writeback's read-side companion (M3 field-report fix, `RepeaterState::
        // seeded`'s own doc comment): a *neighbor* repeater's own `is_locked` check reaches
        // this repeater's signal through here, possibly before this repeater's own `on_
        // neighbor_changed`/`on_scheduled_tick` has ever dispatched -- seed unconditionally,
        // right here, so every reader always sees this position's real placed `POWERED` value
        // rather than `place()`'s own unconditional `false` default.
        self.seed_powered_from_world(world, pos);
        // `FACING` points toward this repeater's own INPUT side (Context §F, ASSET-D18(f)
        // research verdict); output flows out the opposite side ("a repeater fires away from
        // you" -- placement sets `FACING = playerLookDirection.opposite()`).
        if self.powered(pos) && towards == self.facing(pos).opposite() {
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
    /// `checkTickOnNeighbor` (Context §F), plus `RepeaterBlock::neighborChanged`'s own
    /// additional immediate `LOCKED` writeback (M3 field-report fix, Context §F/`is_locked`'s
    /// own doc comment) -- vanilla's `RepeaterBlock` overrides `neighborChanged` beyond
    /// `DiodeBlock`'s base specifically to recompute+write `LOCKED` on every call, independent
    /// of (and not gated by) whether a `POWERED`-toggle tick also gets scheduled below.
    ///
    /// M3 field-report fix (regression correction): support-loss destruction lives here now,
    /// direction-agnostically, relocated from `on_placed` (WRONG HOOK -- vanilla never
    /// self-validates a command-placed block; a `/setblock`'d repeater with no floor support
    /// survives until some neighbor changes, confirmed against a real oracle diff:
    /// `comparator_2tick_fixed_delay`'s own isolated floor-less comparator stays alive in the
    /// oracle trace while the old `on_placed` check destroyed it at tick 0). Every one of the
    /// six `on_neighbor_changed` trigger directions re-checks support, which always looks
    /// straight down regardless of `_from` -- mirrors `WireBehavior::should_pop`'s identical
    /// check, but reachable from any neighbor-changed trigger rather than only a `Down`-
    /// direction shape update.
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        if !signal::is_conductor(ctx.world, Direction::Down.apply(pos)) {
            ctx.set_block(pos, AIR_ID);
            return;
        }
        self.seed_powered_from_world(ctx.world, pos);
        let locked = self.is_locked(ctx.world, pos);
        if !locked {
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
        self.write_state_id(ctx, pos, Some(locked), None);
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
                self.write_state_id(ctx, pos, None, Some(false));
                signal::notify_neighbor_changed_only(ctx, pos);
            }
        } else {
            self.set_powered(pos, true);
            self.write_state_id(ctx, pos, None, Some(true));
            signal::notify_neighbor_changed_only(ctx, pos);
            if !self.base_input_positive(ctx.world, pos) {
                ctx.schedule_block_tick(pos, self.get_delay(pos), TickPriority::VeryHigh);
            }
        }
    }

    /// M3 field-report fix (Task 2): reseeds `facing`/`delay_setting` (a full replace-on-replace
    /// via `place`, `place`'s own doc comment) directly off `pos`'s own current raw id — the
    /// exact inverse of `repeater_state_id`'s own arithmetic, mirroring `seed_powered_from_
    /// world`'s established decode-from-raw-id pattern. A caller with its own freshly-written
    /// `BlockStateId` at `pos` (a real `/setblock`-shaped placement, never this behavior's own
    /// internal writeback, which goes through the raw world accessor and never reaches this
    /// hook) calls `on_placed` instead of a losing, ordering-sensitive direct `place()` call of
    /// its own.
    ///
    /// M3 field-report fix (regression correction): no longer checks floor support -- vanilla
    /// never self-validates a command-placed block (`on_neighbor_changed`'s own doc comment has
    /// the full citation); a repeater `/setblock`'d with no floor support simply survives until
    /// some neighbor changes.
    fn on_placed(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        let rel = current.0.wrapping_sub(REPEATER_BASE);
        if rel >= 64 {
            return; // not a real repeater id -- defensive only, dispatch never reaches here otherwise
        }
        let delay_setting = (rel / 16) as u8 + 1;
        let facing = signal::diode_facing_from_index((rel % 16) / 4);
        self.place(pos, facing, delay_setting);
    }
}
