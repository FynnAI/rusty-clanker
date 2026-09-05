//! Repeater — delay, boolean lock, priority selection (Context §F).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_registries::block_state_properties::{properties, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::{BlockStateId as GenStateId, default_state};

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

/// `air`'s own raw id (M3 field-report fix, Task 3) -- M3.5-B02: read off `rc-registries`' own
/// generated `default_state::AIR` constant (value unchanged, `0`) now that this crate already
/// depends on `rc-registries` normally.
const AIR_ID: BlockStateId = BlockStateId(default_state::AIR.0);

/// `true` iff `raw` falls inside `minecraft:repeater`'s own real generated id range
/// (M3.5-B02, WS-D15) -- mirrors `wire.rs`'s documented `is_wire_range` convention: keeps the
/// decode-from-raw-id read paths below (`seed_powered_from_world`/`write_state_id`) safe
/// against this project's own established acceptance-test convention of registering
/// `RepeaterBehavior` at small arbitrary placeholder ids (e.g. `redstone_repeater.rs`'s own
/// `REPEATER_ID` constant, a small `BlockStateId`), standing in for "some repeater" without
/// needing a real id -- a cheap range-containment check never panics, unlike a full
/// `properties()` decode of an id with no `locked`/`powered` property at all.
fn is_repeater_range(raw: u32) -> bool {
    let range = rc_registries::block_state_properties::range_of(block_id::REPEATER);
    (range.first.0..=range.last.0).contains(&raw)
}

fn locked_str(locked: bool) -> &'static str {
    if locked { "true" } else { "false" }
}

fn powered_str(powered: bool) -> &'static str {
    if powered { "true" } else { "false" }
}

fn repeater_property<'a>(props: &'a [(&str, &str)], name: &str, raw: u32) -> &'a str {
    props
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| panic!("repeater: raw id {raw} has no {name} property"))
}

fn repeater_state_id(
    delay_setting: u8,
    facing: Direction,
    locked: bool,
    powered: bool,
) -> BlockStateId {
    let id = state_id(
        block_id::REPEATER,
        &[
            ("delay", &delay_setting.to_string()),
            ("facing", signal::diode_facing_str(facing)),
            ("locked", locked_str(locked)),
            ("powered", powered_str(powered)),
        ],
    )
    .expect("repeater_state_id: every (delay,facing,locked,powered) combination is legal");
    BlockStateId(id.0)
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
        if let Some(current) = world.get_block(pos)
            && is_repeater_range(current.0)
        {
            let props = properties(GenStateId(current.0));
            entry.powered = repeater_property(props, "powered", current.0) == "true";
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
        let (current_locked, current_powered) = if is_repeater_range(current.0) {
            let props = properties(GenStateId(current.0));
            (
                repeater_property(props, "locked", current.0) == "true",
                repeater_property(props, "powered", current.0) == "true",
            )
        } else {
            (false, false)
        };
        let facing = self.facing(pos);
        let delay = self.delay_setting(pos);
        let id = repeater_state_id(
            delay,
            facing,
            locked_new.unwrap_or(current_locked),
            powered_new.unwrap_or(current_powered),
        );
        ctx.write_block_state(pos, id);
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
        // M3 field-report fix (MECH-D84): a repeater's own floor support check is `Rigid`-
        // sturdiness on the block below's top face, never the redstone-conductor test (a chest
        // is not a conductor but is `Rigid`-sturdy, and a repeater does survive on one).
        if !signal::is_face_sturdy(
            ctx.world,
            Direction::Down.apply(pos),
            Direction::Up,
            rc_physics::SupportKind::Rigid,
        ) {
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
        if !is_repeater_range(current.0) {
            return; // not a real repeater id -- defensive only, dispatch never reaches here otherwise
        }
        let props = properties(GenStateId(current.0));
        let delay_setting: u8 = repeater_property(props, "delay", current.0)
            .parse()
            .unwrap_or_else(|_| {
                panic!(
                    "repeater: raw id {} has a malformed delay property",
                    current.0
                )
            });
        let facing = signal::diode_facing_from_str(repeater_property(props, "facing", current.0));
        self.place(pos, facing, delay_setting);
    }
}
