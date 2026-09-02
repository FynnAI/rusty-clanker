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
        Self {
            ranges: Vec::new(),
            default: Arc::new(NoSignalSource),
        }
    }

    /// Panics on overlap with an already-registered range (identical contract to B01's
    /// `BlockBehaviorRegistry::register_range`).
    pub fn register_range(
        &mut self,
        start: BlockStateId,
        end_exclusive: BlockStateId,
        source: Arc<dyn RedstoneSignalSource>,
    ) {
        let overlaps = self
            .ranges
            .iter()
            .any(|(s, e, _)| start < *e && *s < end_exclusive);
        assert!(
            !overlaps,
            "SignalSourceRegistry::register_range: [{start:?}, {end_exclusive:?}) overlaps an already-registered range"
        );
        self.ranges.push((start, end_exclusive, source));
        self.ranges.sort_by_key(|(start, _, _)| *start);
    }

    /// Returns the matching range's source, or the shared `NoSignalSource` default.
    pub fn resolve(&self, state: BlockStateId) -> &Arc<dyn RedstoneSignalSource> {
        for (start, end_exclusive, source) in &self.ranges {
            if state >= *start && state < *end_exclusive {
                return source;
            }
        }
        &self.default
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
    let Some(state) = world.get_block(pos) else {
        return false;
    };
    let props = rc_physics::tier1_shape_table().lookup(state.to_raw());
    let boxes = props.shape.boxes();
    boxes.len() == 1
        && boxes[0].min == Vec3::new(0.0, 0.0, 0.0)
        && boxes[0].max == Vec3::new(1.0, 1.0, 1.0)
}

/// `emitted_toward` (Context §A) — the one shared quasi-connectivity primitive.
pub fn emitted_toward(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    towards: Direction,
) -> u8 {
    let Some(state) = world.get_block(pos) else {
        return 0;
    };
    let weak = registry
        .resolve(state)
        .weak_signal_toward(world, pos, towards);
    if is_conductor(world, pos) {
        weak.max(direct_signal_to(world, registry, pos))
    } else {
        weak
    }
}

/// `direct_signal_to` (Context §A) — all 6 faces of the conductor at `pos`.
pub fn direct_signal_to(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
) -> u8 {
    let mut max_signal = 0u8;
    for d in NEIGHBOR_CHANGED_ORDER {
        let npos = d.apply(pos);
        let Some(nstate) = world.get_block(npos) else {
            continue;
        };
        let v = registry
            .resolve(nstate)
            .direct_signal_toward(world, npos, d.opposite());
        max_signal = max_signal.max(v);
    }
    max_signal
}

/// `signal_into` (Context §A) — what `pos` receives from its neighbor in `from`.
pub fn signal_into(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    from: Direction,
) -> u8 {
    emitted_toward(world, registry, from.apply(pos), from.opposite())
}

/// `best_neighbor_signal` (Context §A) — max over all 6 sides.
pub fn best_neighbor_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
) -> u8 {
    NEIGHBOR_CHANGED_ORDER
        .into_iter()
        .map(|d| signal_into(world, registry, pos, d))
        .max()
        .unwrap_or(0)
}

/// `has_signal(pos, from) = signal_into(pos, from) > 0` — a thin boolean convenience, used
/// directly by torch's own input check and available to M3-B05 (piston) for its own QC input.
pub fn has_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    from: Direction,
) -> bool {
    signal_into(world, registry, pos, from) > 0
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
    let front = facing.apply(pos);
    let plain = signal_into(world, registry, pos, facing);
    let raw_wire = world
        .get_block(front)
        .and_then(|state| registry.resolve(state).raw_wire_power(world, front));
    match raw_wire {
        Some(raw) if raw > plain => raw,
        _ => plain,
    }
}

/// One direction's worth of `notify_neighbor_changed_only`'s own per-neighbor dispatch,
/// factored out so the QC relay hop below can reuse the identical local-vs-cross-region logic
/// with a different `origin`/`dir` pair.
fn notify_one(
    ctx: &mut UpdateContext,
    dimension: rc_core::DimensionId,
    origin: BlockPos,
    dir: Direction,
) {
    let npos = dir.apply(origin);
    let chunk = npos.chunk_key(dimension);
    let owner = (ctx.ownership.resolve)(chunk);
    if owner == ctx.ownership.local {
        ctx.engine.emit_single(PendingUpdate::NeighborChanged {
            pos: npos,
            from: dir.opposite(),
        });
    } else {
        let new_state = ctx
            .world
            .get_block(origin)
            .expect("`origin` is always locally-loaded when this fires");
        ctx.outbound.push((
            Address::Chunk(chunk),
            RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                chunk,
                pos: origin,
                kind: BorderUpdateKind::BlockChanged {
                    new_state: new_state.to_raw(),
                },
            }),
        ));
    }
}

/// Cross-region-aware neighbor-changed-ONLY notify (Context §I) — every tier-1 component's
/// own state-change propagation goes through this, never a bare `ctx.engine.emit_*` call and
/// never `UpdateContext::set_block` (which would also fire an unwanted shape-update pass).
///
/// M3 field-report fix (Task 1): also relays through a **conductor** neighbor to that
/// conductor's own further neighbors, one hop -- `SignalGetter.getSignal`'s conductor rule
/// (research doc §3.1/Notes: "any block that is a conductor and reads `level.getSignal`... on
/// itself or a neighbor inherits [quasi-connectivity] automatically") means a conductor's own
/// aggregate signal can change purely because *one of its six faces* changed, even though the
/// conductor's own stored `BlockState` never does -- so a position reading *through* that
/// conductor (a wire resting on it, a torch mounted on it, anything on its far side) needs its
/// own recompute retriggered too, exactly as if the conductor itself had changed. Confirmed
/// against a real oracle diff: a repeater's own output face touches a plain conductor, which a
/// wire tile rests against on a *different* face two hops from the repeater
/// (`redstone/qc/wire_strong_vs_weak_power_door`'s own `(1,1,1)`, `docs/findings-for-planning.md`)
/// -- without this relay, that wire's own power silently never updates once the repeater's own
/// single-hop notify only reaches the conductor itself (`NoOpBehavior`, nothing to recompute).
/// Bounded to exactly one relay hop (never chained through a second conductor) -- vanilla's own
/// conductor rule is itself one hop (`getDirectSignalTo` scans the *immediate* conductor's six
/// faces only, `direct_signal_to`'s own doc comment), so a chain of multiple conductors needs no
/// further relay: each position along such a chain still gets its own real trigger the normal
/// way, since a plain conductor is never itself a redstone consumer that could silently swallow
/// an intermediate signal change the way a `NoOpBehavior` neighbor otherwise would.
pub fn notify_neighbor_changed_only(ctx: &mut UpdateContext, at: BlockPos) {
    let dimension = ctx.world.dimension();
    for dir in NEIGHBOR_CHANGED_ORDER {
        notify_one(ctx, dimension, at, dir);
        let npos = dir.apply(at);
        if is_conductor(ctx.world, npos) {
            for relay_dir in NEIGHBOR_CHANGED_ORDER {
                notify_one(ctx, dimension, npos, relay_dir);
            }
        }
    }
}

/// Cross-region-aware, single-target `ShapeUpdate` dispatch (Context, M3 field-report fix:
/// wire's own `updateIndirectNeighbourShapes` diagonal relay -- `wire.rs`'s own
/// `diagonal_shape_update_cascade` doc comment has the full citation and the "why this exact
/// target/`from` pair" geometry). Unlike `notify_neighbor_changed_only`'s uniform 6-neighbor
/// fan-out, this dispatches to exactly one already-computed `target` position with a fresh
/// full `SHAPE_DEPTH` budget -- a targeted relay, not a continuation of the triggering
/// cascade's own remaining depth: `BlockBehavior::on_shape_update`/`on_placed`'s own trait
/// signatures carry no depth-remaining parameter for a caller to thread through here, and
/// `NeighborUpdateEngine::SHAPE_DEPTH`'s own 512-hop bound is already far beyond anything a
/// legal contraption could ever need, so restarting the budget here is a bounded,
/// practically-unobservable simplification rather than a real parity gap.
///
/// Local-target only. `target`'s own ownership is resolved fresh here since the calling wire
/// behavior only knows the *local* geometry it walked to find `target`, never its ownership --
/// a genuinely-remote `target` is silently skipped rather than misrouted: sending it would need
/// a dedicated `BorderUpdateEvent` payload this project's own message substrate does not yet
/// carry (unlike `notify_one`'s existing `BlockChanged` payload, which communicates *the
/// locally-owned origin's* new state to a remote neighbor -- this call's own `target` is the
/// *remote* side, with nothing local to attach to that payload). A documented, bounded-latency
/// gap (`docs/findings-for-planning.md`) rather than a blocking dependency (binding principle:
/// "no cross-partition blocking... fire-and-forget with bounded-latency delivery") -- every
/// caller in this corpus is single-region (`replay_contraption`'s own `RegionOwnership::
/// always_local`), so `owner == ctx.ownership.local` always holds in practice today.
pub fn notify_shape_update_at(ctx: &mut UpdateContext, target: BlockPos, from: Direction) {
    let dimension = ctx.world.dimension();
    let chunk = target.chunk_key(dimension);
    let owner = (ctx.ownership.resolve)(chunk);
    if owner == ctx.ownership.local {
        ctx.engine.emit_single(PendingUpdate::ShapeUpdate {
            pos: target,
            from,
            remaining_depth: crate::neighbor_update::NeighborUpdateEngine::SHAPE_DEPTH,
        });
    }
}

/// A diode's own `facing` property value, as `minecraft:repeater`/`minecraft:comparator`'s own
/// generated per-block-state-property registry entries spell it -- shared between
/// `repeater.rs`'s and `comparator.rs`'s own own-state id arithmetic (M3.5-B02, WS-D15: the
/// generated per-property registry's `state_id`/`with_property`/`properties` API replaced the
/// former hand-derived `diode_facing_index` stride arithmetic this pair of functions used to
/// be), since both components' `facing` property shares this identical string spelling. Ordinary
/// property-value string mapping, not itself an id table (`blueprints/M3.5/
/// M3.5-B02-retire-hand-authored-id-tables.md` §3.2).
pub(crate) fn diode_facing_str(facing: Direction) -> &'static str {
    match facing {
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
        Direction::Up | Direction::Down => {
            panic!("diode_facing_str: a diode's own facing is always horizontal, got {facing:?}")
        }
    }
}

/// Inverse of `diode_facing_str` (M3 field-report fix, Task 2) — recovers a diode's own
/// `facing` from its own stored `BlockStateId`'s decoded `facing` property, the read-side
/// companion `RepeaterBehavior::on_placed`/`ComparatorBehavior::on_placed` need to reseed their
/// own facing side-table directly off a freshly-(re)placed position's real id, without requiring
/// an explicit `place()` call from the caller.
pub(crate) fn diode_facing_from_str(value: &str) -> Direction {
    match value {
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        other => panic!("diode_facing_from_str: unrecognized diode facing value {other:?}"),
    }
}

/// The two horizontal directions perpendicular to `facing`'s own axis, in a fixed (but
/// otherwise arbitrary — every caller takes the `max` of both) order (Context §F: repeater's
/// `alternate_signal`; Context §G: comparator's side-input reading). Shared between
/// `repeater.rs` and `comparator.rs`.
pub(crate) fn perpendicular_pair(facing: Direction) -> (Direction, Direction) {
    match facing {
        Direction::West | Direction::East => (Direction::North, Direction::South),
        Direction::North | Direction::South => (Direction::West, Direction::East),
        Direction::Up | Direction::Down => {
            panic!("perpendicular_pair: a diode's own facing is always horizontal, got {facing:?}")
        }
    }
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
    let behind = facing.opposite().apply(pos);
    let Some(state) = world.get_block(behind) else {
        return false;
    };
    let source = registry.resolve(state);
    if !source.is_diode() {
        return false;
    }
    // `behind` is the neighbor `pos`'s own output flows into (Context §F, ASSET-D18(f)
    // research verdict). Its own `FACING` "points back" at `pos` -- i.e. its own output flows
    // straight back into `pos`, two diodes facing directly at each other head-to-head -- exactly
    // when `behind_facing == facing.opposite()`; that is the sole non-prioritized case. A
    // same-facing behind-diode (the ordinary same-direction daisy chain, where `behind`'s own
    // input reads straight from `pos`'s output) is instead prioritized, matching every other
    // behind-diode orientation including the perpendicular-chain case.
    match source.diode_facing(behind) {
        Some(behind_facing) => behind_facing != facing.opposite(),
        None => false,
    }
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
    if should_prioritize_diode(world, registry, pos, facing) {
        TickPriority::ExtremelyHigh
    } else if currently_powered {
        TickPriority::VeryHigh
    } else {
        TickPriority::High
    }
}

/// `getControlInputSignal(pos, direction, onlyDiodes)` (Context §F/§G) — the single shared
/// side-input reading both `RepeaterBehavior::alternate_signal` (`only_diodes = true`, its own
/// `sideInputDiodesOnly`) and `ComparatorBehavior::side_input_signal` (`only_diodes = false`)
/// dispatch through, verified against the decompiled 26.2 reference (M3 field-report fix, Rule
/// 1): this is deliberately narrower than the general `emitted_toward`/`signal_into` quasi-
/// connectivity primitive above -- side input never follows `emitted_toward`'s own "conductor
/// relays its neighbors' direct signal" branch at all (it queries `side`'s own immediate
/// neighbor block directly, never a further hop through it even when that neighbor happens to be
/// a conductor), and for a plain (non-diode, non-wire) block it reads that neighbor's own DIRECT
/// signal, never its weak one -- the distinction the former, buggy `comparator.rs::
/// side_input_signal` (routed through `signal::signal_into`) collapsed, letting a torch's own
/// unconditional-except-toward-its-input-direction weak `15` leak into a comparator's side
/// reading. `only_diodes = true` (repeater): `0` unless the neighbor is itself a diode
/// (repeater/comparator), in which case its own direct signal counts; plain wire and torches
/// never lock a repeater. `only_diodes = false` (comparator): `redstone_block` -> `15` and
/// `redstone_wire` -> its own raw stored `POWER` value both fall out of the same two general
/// cases below (`raw_wire_power`'s own bypass hook; `redstone_block`'s own `direct_signal_toward`
/// is an unconditional `15` regardless of query direction) without a literal per-block special
/// case; any other signal source contributes its own direct signal in that direction (a lit
/// floor torch's own direct signal is `15` only straight `Up`, `TorchBehavior::direct_signal_
/// toward`'s own doc comment -- so a torch standing beside a comparator, queried from a
/// horizontal direction, now correctly contributes `0`); anything else (ordinary terrain, even a
/// powered conductor with no `is_signal_source` block of its own) contributes `0`.
pub(crate) fn control_input_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
    side: Direction,
    only_diodes: bool,
) -> u8 {
    let neighbor_pos = side.apply(pos);
    let towards = side.opposite();
    let Some(state) = world.get_block(neighbor_pos) else {
        return 0;
    };
    let source = registry.resolve(state);
    if only_diodes {
        return if source.is_diode() {
            source.direct_signal_toward(world, neighbor_pos, towards)
        } else {
            0
        };
    }
    if let Some(power) = source.raw_wire_power(world, neighbor_pos) {
        return power;
    }
    if source.is_signal_source() {
        source.direct_signal_toward(world, neighbor_pos, towards)
    } else {
        0
    }
}
