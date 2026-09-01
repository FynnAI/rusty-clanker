//! Redstone wire — classic (default) power evaluator (MECH-D11/D12, Context §D).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

/// Own-state id arithmetic for `minecraft:redstone_wire` (M3 field-report fix: own-state
/// writeback; WS-D15's generated per-property registry is future work, so these constants are
/// read directly off `datagen-output/26.2/generated/reports/blocks.json`'s own
/// `minecraft:redstone_wire` entry, protocol 776). Five properties, blocks.json's own listed
/// per-property value order (`east`/`north`/`south`/`west`: `[up, side, none]`; `power`:
/// `0..=15`), enumerated alphabetically-by-property-name with the *last* property varying
/// fastest (`west`), matching the real generated state list exactly (verified directly against
/// every id this fixture corpus places, `redstone_wire.rs`'s own test module):
/// `id = WIRE_BASE + east_idx*432 + north_idx*144 + power*9 + south_idx*3 + west_idx`.
const WIRE_BASE: u32 = 4011;
const WIRE_MAX: u32 = 5306; // inclusive -- east=none,north=none,power=15,south=none,west=none
const WIRE_STRIDE_WEST: u32 = 1;
const WIRE_STRIDE_SOUTH: u32 = 3;
const WIRE_STRIDE_POWER: u32 = 9;
const WIRE_STRIDE_NORTH: u32 = 144;
const WIRE_STRIDE_EAST: u32 = 432;

/// `true` iff `raw` is a real `minecraft:redstone_wire` id (`WIRE_BASE..=WIRE_MAX`) —
/// `new_power_state_id`/`new_connections_state_id` may only ever be called with a `raw` this
/// returns `true` for, since `wire_decode`'s own unchecked subtraction underflows otherwise.
/// This project's own established acceptance-test convention registers `WireBehavior` at small
/// arbitrary placeholder ids unrelated to blocks.json's real range (e.g. `redstone_repeater.rs`'s
/// own `WIRE_ID = BlockStateId(4)`, standing in for "some wire neighbor" without needing a real
/// id) — dispatch through a real `SignalSourceRegistry`/`BlockBehaviorRegistry` range is always
/// in-range by construction, but this guard keeps every such test double safe too, leaving its
/// placeholder id untouched rather than attempting arithmetic that assumes a real one.
fn is_wire_range(raw: u32) -> bool {
    (WIRE_BASE..=WIRE_MAX).contains(&raw)
}

/// `[min, max]` inclusive -- `dispatch_ranges::derive_tier1_state_ids`'s own read side (M3
/// field-report fix, "production never wires redstone"): exposes this module's own already
/// oracle-verified `WIRE_BASE`/`WIRE_MAX` constants above, rather than duplicating them a second
/// time the way `crates/testing/gametest/src/replay.rs`'s own `WIRE_RANGE` constant currently
/// does.
pub(crate) fn state_range() -> (u32, u32) {
    (WIRE_BASE, WIRE_MAX)
}

/// `air`'s own raw id (M3 field-report fix, Task 1) — stable by protocol convention
/// (`rc_physics::shapes`'s identical documented assumption, `piston.rs`'s own identical
/// `AIR_ID` convention), hardcoded directly since this crate has no `rc-registries` dependency
/// (WS-D3 rule 1).
const AIR_ID: BlockStateId = BlockStateId(0);

/// `RedStoneWireBlock::canSurvive` (Context/research doc §3.1/Notes): a wire tile requires a
/// conductor directly beneath it. Mirrors `TorchBehavior::should_pop`'s identical role/shape.
fn should_pop(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
    !signal::is_conductor(world, Direction::Down.apply(pos))
}

/// The real 3-way visual connection shape blocks.json's own `east`/`north`/`south`/`west`
/// properties each carry (M3 field-report fix, Task 4 -- closes this module's own former
/// "connected sides are always encoded `side`, never `up`" approximation,
/// `docs/findings-for-planning.md`'s own "wire up/side" entry). Mirrors vanilla's own
/// `RedstoneSide` enum (`08-redstone-ticking.md` §3.1).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WireSideShape {
    None,
    Side,
    Up,
}

/// blocks.json's own per-direction `[up, side, none]` index (`up`=0, `side`=1, `none`=2).
/// `on_neighbor_changed`'s own power-only writeback below preserves a pre-existing `up` bit a
/// position's id already held, by construction, since it decodes and re-encodes every
/// connection digit unchanged and only ever replaces the power digit.
fn side_index(shape: WireSideShape) -> u32 {
    match shape {
        WireSideShape::Up => 0,
        WireSideShape::Side => 1,
        WireSideShape::None => 2,
    }
}

fn wire_state_id(east: u32, north: u32, power: u8, south: u32, west: u32) -> BlockStateId {
    BlockStateId(
        WIRE_BASE
            + east * WIRE_STRIDE_EAST
            + north * WIRE_STRIDE_NORTH
            + u32::from(power) * WIRE_STRIDE_POWER
            + south * WIRE_STRIDE_SOUTH
            + west * WIRE_STRIDE_WEST,
    )
}

/// Inverse of `wire_state_id` — `raw` must already be known to lie in wire's own real range
/// (dispatch only ever reaches this behavior through that registered range), so every
/// intermediate index below is guaranteed in-bounds without needing to check.
fn wire_decode(raw: u32) -> (u32, u32, u8, u32, u32) {
    let rel = raw - WIRE_BASE;
    let east = rel / WIRE_STRIDE_EAST;
    let rel = rel % WIRE_STRIDE_EAST;
    let north = rel / WIRE_STRIDE_NORTH;
    let rel = rel % WIRE_STRIDE_NORTH;
    let power = (rel / WIRE_STRIDE_POWER) as u8;
    let rel = rel % WIRE_STRIDE_POWER;
    let south = rel / WIRE_STRIDE_SOUTH;
    let west = rel % WIRE_STRIDE_SOUTH;
    (east, north, power, south, west)
}

/// `on_neighbor_changed`'s own writeback: replaces only the `power` digit, decoding and
/// re-encoding every connection digit unchanged (preserves a pre-existing `up` bit exactly,
/// `side_index`'s own doc comment).
fn new_power_state_id(current_raw: u32, power: u8) -> BlockStateId {
    let (east, north, _old_power, south, west) = wire_decode(current_raw);
    wire_state_id(east, north, power, south, west)
}

/// `on_shape_update`'s own writeback: replaces every connection digit with the freshly
/// recomputed 3-way shape (`side_index`'s own `up`/`side`/`none` encoding), preserving whatever
/// `power` digit the position's own current raw id already holds.
fn new_connections_state_id(
    current_raw: u32,
    east: WireSideShape,
    north: WireSideShape,
    south: WireSideShape,
    west: WireSideShape,
) -> BlockStateId {
    let (_east, _north, power, _south, _west) = wire_decode(current_raw);
    wire_state_id(
        side_index(east),
        side_index(north),
        power,
        side_index(south),
        side_index(west),
    )
}

/// "Does this side connect at all," per direction — `weak_signal_toward`'s own output-gating
/// boolean (Context §D). The *visual* `NONE`/`SIDE`/`UP` three-way shape (M3 field-report fix,
/// Task 4: `WireSideShape`) is a strict refinement of this same union (`shape != None`), computed
/// once per side by `connection_shape_on_side` and never independently — this struct stays
/// boolean-only since nothing that reads it (`weak_signal_toward`'s own connectivity gate) cares
/// which of `Side`/`Up` a connected side actually is.
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
    /// `RedStoneWireBlock`'s own `shouldSignal` field (research doc §3.1) restated directly:
    /// one mutable flag shared by every wire tile this single per-region instance handles
    /// (mirrors vanilla's own block-singleton-per-type instance, `getBlockSignal`'s own doc
    /// comment below). `true` except for the brief window `compute_power` holds it `false`
    /// while it queries every *other* signal source's contribution toward the one position
    /// currently being recomputed -- safe without finer-grained (per-position) scoping because
    /// Stage-4 dispatch is strictly sequential, never reentrant, within one region (binding
    /// principle: "redstone... always fully sequential and single-worker per region").
    should_signal: AtomicBool,
}

impl WireBehavior {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
            registry: OnceLock::new(),
            should_signal: AtomicBool::new(true),
        }
    }

    /// Current stored power (`0` for a never-yet-computed position — matches vanilla's own
    /// freshly-placed-wire default of `0`).
    pub fn power(&self, pos: BlockPos) -> u8 {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.power)
            .unwrap_or(0)
    }

    pub fn connections(&self, pos: BlockPos) -> WireConnections {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.connections)
            .unwrap_or_default()
    }

    /// Test/composition-root-only: directly sets this position's own stored connectivity,
    /// bypassing `on_shape_update`'s own connectivity recomputation — this blueprint's own
    /// `wire_output_is_gated_by_connections_horizontally_only` acceptance test needs to force a
    /// specific connectivity value without depending on a particular neighbor arrangement (a
    /// minimal, necessary addition beyond Context §I½'s own literal deliverable listing, which
    /// names no such setter — documented as a deviation in the completion report).
    pub fn set_connections(&self, pos: BlockPos, connections: WireConnections) {
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_default()
            .connections = connections;
    }

    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>) {
        self.registry
            .set(registry)
            .unwrap_or_else(|_| panic!("WireBehavior::bind_registry called more than once"));
    }

    fn registry(&self) -> &Arc<SignalSourceRegistry> {
        self.registry
            .get()
            .expect("WireBehavior: bind_registry must run before dispatch")
    }

    /// `getIncomingWireSignal` (Context §D): four horizontal neighbors, plus -- for each
    /// neighbor that is a redstone conductor with a non-conductor ceiling above `pos` -- the
    /// wire one block above it, and -- for each non-conductor neighbor -- the wire one block
    /// below it. `max(candidates) - 1`, floored at 0.
    fn incoming_wire_signal(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        let mut candidates: Vec<u8> = Vec::new();
        for dir in [
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
        ] {
            let same_height = dir.apply(pos);
            if let Some(p) = wire_power_at(world, registry, same_height) {
                candidates.push(p);
            }
            if signal::is_conductor(world, same_height)
                && !signal::is_conductor(world, Direction::Up.apply(pos))
                && let Some(p) = wire_power_at(world, registry, Direction::Up.apply(same_height))
            {
                candidates.push(p);
            }
            if !signal::is_conductor(world, same_height)
                && let Some(p) = wire_power_at(world, registry, Direction::Down.apply(same_height))
            {
                candidates.push(p);
            }
        }
        candidates
            .into_iter()
            .max()
            .map(|m| m.saturating_sub(1))
            .unwrap_or(0)
    }

    /// `updatePowerStrength` (Context §D): `block_signal.max(incoming_wire_signal)`, with the
    /// `block_signal == 15` short-circuit (pure perf optimization, no observable difference).
    /// `block_signal` is `getBlockSignal`/`level.getBestNeighborSignal` (research doc §3.1)
    /// computed via the shared `signal::best_neighbor_signal` every other component calls, with
    /// `should_signal` held `false` for the duration of the call (`getBlockSignal`'s own doc
    /// comment below) -- the single global exclusion this position's own recompute needs, not a
    /// per-neighbor special case.
    fn compute_power(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        let block_signal = self.block_signal(world, registry, pos);
        if block_signal == 15 {
            return 15;
        }
        block_signal.max(self.incoming_wire_signal(world, registry, pos))
    }

    /// `getBlockSignal` (Context §D, research doc §3.1: "wire's own `isSignalSource` temporarily
    /// disabled via the `shouldSignal` flag to avoid self-counting"). Vanilla disables *every*
    /// wire's signal contribution — not merely an immediately-adjacent one — for the duration of
    /// this one call, which is what actually prevents a wire from reading its own power back
    /// through a quasi-connectivity bounce off its own supporting conductor (that conductor's
    /// own `direct_signal_to` scans *all six* of its faces, one of which is the very wire tile
    /// currently mid-recompute — a shallow "is my immediate neighbor a wire" check, this
    /// project's own former approach, cannot see that indirect path at all) while still keeping
    /// wire-to-wire power transfer flowing exclusively through `incoming_wire_signal`'s own
    /// dedicated, decay-aware (`-1` per hop) walk (`raw_wire_power` is deliberately never gated
    /// on `should_signal` — that hook is what `incoming_wire_signal` reads instead). Restoring
    /// the flag before returning (rather than a scope guard) is safe here: this function makes
    /// no further calls once querying is done, and Stage-4 dispatch never re-enters this
    /// instance mid-call (struct doc comment).
    fn block_signal(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        self.should_signal.store(false, Ordering::Relaxed);
        let result = signal::best_neighbor_signal(world, registry, pos);
        self.should_signal.store(true, Ordering::Relaxed);
        result
    }

    /// `shouldConnectTo`/`getConnectionState`/`getConnectingSide` (Context §D): the real 3-way
    /// `NONE`/`SIDE`/`UP` visual connection shape for one horizontal `dir` (M3 field-report fix,
    /// Task 4 -- closes this method's own former "restated as a single boolean" scope
    /// narrowing, `docs/findings-for-planning.md`'s own "wire up/side" entry). Three geometric
    /// cases, `08-redstone-ticking.md` §3.1's own citation ("a same-height check first, then a
    /// check one block up... and one block down..., preferring UP over SIDE when the neighbor's
    /// top face is sturdy"):
    /// - **Up** (checked first, matching the documented priority): my own ceiling is open (not
    ///   a conductor -- `08-redstone-ticking.md`'s own "conductor-occlusion rule," already an
    ///   established gate elsewhere in this module, e.g. `incoming_wire_signal`) and a wire
    ///   climbs one block up on the far side (`dir.apply(pos)`'s own `Up` neighbor) -- the
    ///   classic "wire climbs a step" case; geometrically this and the `Side` case below are
    ///   mutually exclusive in every legitimate build (a same-height wire/source occupies the
    ///   space a solid climbable step would need), so checking `Up` unconditionally first never
    ///   actually overrides a real `Side` connection in practice, only formalizes the documented
    ///   priority.
    /// - **Side**: the same-height neighbor itself connects back (a wire, or any other signal
    ///   source facing this position), or -- the open-ledge case -- the same-height neighbor is
    ///   itself non-conductor and a wire sits one block *below* it (a wire descending off a
    ///   ledge renders `Side` from this ascending tile's own perspective; the *other* wire's own
    ///   `Up` property, read from its own position looking back, is what renders the climb).
    /// - **None**: neither of the above.
    ///
    /// The boolean "does this side connect at all" union (`compute_connections`'s own
    /// `WireConnections`, still used by `weak_signal_toward`'s own output gating) is exactly
    /// `shape != None` -- this method's own three cases are additive refinements of that same
    /// former boolean, never a behavior change to which sides connect.
    fn connection_shape_on_side(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
        dir: Direction,
    ) -> WireSideShape {
        let same_height = dir.apply(pos);
        if !signal::is_conductor(world, Direction::Up.apply(pos)) {
            let up = Direction::Up.apply(same_height);
            if wire_power_at(world, registry, up).is_some() {
                return WireSideShape::Up;
            }
        }
        if let Some(state) = world.get_block(same_height)
            && registry
                .resolve(state)
                .connects_from(world, same_height, dir.opposite())
        {
            return WireSideShape::Side;
        }
        if !signal::is_conductor(world, same_height) {
            let down = Direction::Down.apply(same_height);
            if wire_power_at(world, registry, down).is_some() {
                return WireSideShape::Side;
            }
        }
        WireSideShape::None
    }

    /// The 3-way shape for all four horizontal sides, in `east/north/south/west` order (matching
    /// `wire_state_id`'s own parameter order) -- the read side `on_shape_update` needs for its
    /// own state-id writeback (M3 field-report fix, Task 4). Finishes with vanilla's own
    /// `RedStoneWireBlock.getConnectionState` post-processing pass (M3 field-report fix, Task 1
    /// -- closes the "isolated wire auto-extends to a straight line" gap
    /// `docs/findings-for-planning.md` names for this exact fixture): per axis, a side that is
    /// still `None` after `connection_shape_on_side` gets auto-set to `Side` when the *other*
    /// axis is fully disconnected (`None` on both of its own sides) -- a lone connection on one
    /// axis, with nothing at all on the perpendicular axis, renders the whole tile as a straight
    /// line through it rather than a dead end. Computed from the four freshly-read raw shapes
    /// only (never a shape this same pass just mutated): each of the four checks below reads
    /// only the *opposite* axis's pair, which this pass never rewrites, so evaluation order
    /// across the four checks cannot matter.
    fn compute_connection_shapes(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> (WireSideShape, WireSideShape, WireSideShape, WireSideShape) {
        let east = self.connection_shape_on_side(world, registry, pos, Direction::East);
        let north = self.connection_shape_on_side(world, registry, pos, Direction::North);
        let south = self.connection_shape_on_side(world, registry, pos, Direction::South);
        let west = self.connection_shape_on_side(world, registry, pos, Direction::West);

        let no_east = east == WireSideShape::None;
        let no_west = west == WireSideShape::None;
        let no_north = north == WireSideShape::None;
        let no_south = south == WireSideShape::None;
        let no_east_west = no_east && no_west;
        let no_north_south = no_north && no_south;

        let north = if no_north && no_east_west {
            WireSideShape::Side
        } else {
            north
        };
        let south = if no_south && no_east_west {
            WireSideShape::Side
        } else {
            south
        };
        let east = if no_east && no_north_south {
            WireSideShape::Side
        } else {
            east
        };
        let west = if no_west && no_north_south {
            WireSideShape::Side
        } else {
            west
        };

        (east, north, south, west)
    }
}

/// Whether the position holds a redstone wire (any `WireBehavior`, generalized via the
/// `raw_wire_power` hook, Context §F/§C) and, if so, its current stored power.
fn wire_power_at(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
) -> Option<u8> {
    let state = world.get_block(pos)?;
    registry.resolve(state).raw_wire_power(world, pos)
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
        if !self.should_signal.load(Ordering::Relaxed) {
            return 0;
        }
        let power = self.power(pos);
        let connections = self.connections(pos);
        let connected = match towards {
            Direction::West => connections.west,
            Direction::East => connections.east,
            Direction::North => connections.north,
            Direction::South => connections.south,
            Direction::Up | Direction::Down => false,
        };
        if connected { power } else { 0 }
    }
    /// `Down` only, unconditional on power (Context §A's worked QC example) -- except while
    /// `should_signal` is `false` (`block_signal`'s own doc comment): this is exactly the path
    /// that let a wire's own supporting conductor bounce its power back to it via quasi-
    /// connectivity (`direct_signal_to` scans all six of that conductor's faces, `Up` among
    /// them), so it must be gated identically to `weak_signal_toward` above.
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        if !self.should_signal.load(Ordering::Relaxed) {
            return 0;
        }
        if towards == Direction::Down {
            self.power(pos)
        } else {
            0
        }
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
        let registry = Arc::clone(self.registry());
        let new_power = self.compute_power(ctx.world, &registry, pos);
        let changed = {
            let mut state = self.state.lock().unwrap();
            let entry = state.entry(pos).or_default();
            if entry.power == new_power {
                false
            } else {
                entry.power = new_power;
                true
            }
        };
        if !changed {
            return;
        }
        // M3 field-report fix: own-state writeback -- vanilla's `DefaultRedstoneWireEvaluator`
        // writes the recomputed power straight back into the `BlockState` (update flag `2` =
        // clients-only, no cascading shape update of its own -- the actual neighbor notify is
        // `notify_neighbor_changed_only` below, unchanged), so this write goes through the raw
        // world accessor, never `ctx.set_block`. `new_power_state_id` preserves every
        // connection digit already stored (`side_index`'s own doc comment).
        if let Some(current) = ctx.get_block(pos)
            && is_wire_range(current.0)
        {
            let new_id = new_power_state_id(current.0, new_power);
            ctx.world.set_block(pos, new_id);
        }
        // The unconditional 7-cell-plus notify (Context §D): `pos` itself first, then its own
        // 6 neighbors in `NEIGHBOR_CHANGED_ORDER` -- no shape update is fired.
        signal::notify_neighbor_changed_only(ctx, pos);
        for dir in crate::direction::NEIGHBOR_CHANGED_ORDER {
            signal::notify_neighbor_changed_only(ctx, dir.apply(pos));
        }
    }
    /// M3 field-report fix (Task 1): support-loss destruction — a `Down`-direction shape update
    /// destroys the wire (returns air) if its own floor support (`should_pop`, this module's own
    /// top-of-file helper) is gone, mirroring `TorchBehavior::on_shape_update`'s identical fix;
    /// every other trigger direction is unaffected and falls straight through to the existing
    /// connections recompute below. Also clears this position's own side-table state, so a
    /// future re-placement here starts fresh rather than inheriting the destroyed wire's last
    /// power/connections.
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        if from == Direction::Down && should_pop(ctx.world, pos) {
            self.state.lock().unwrap().remove(&pos);
            return Some(AIR_ID);
        }
        let registry = Arc::clone(self.registry());
        // M3 field-report fix (Task 4): the 3-way shape is now computed directly (`connection_
        // shape_on_side`'s own doc comment) -- the boolean `WireConnections` `weak_signal_
        // toward`'s own output gating still reads is exactly this shape's own `!= None`, derived
        // here rather than recomputed independently, so there is only one geometric computation
        // to keep correct.
        let (east_shape, north_shape, south_shape, west_shape) =
            self.compute_connection_shapes(ctx.world, &registry, pos);
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_default()
            .connections = WireConnections {
            east: east_shape != WireSideShape::None,
            north: north_shape != WireSideShape::None,
            south: south_shape != WireSideShape::None,
            west: west_shape != WireSideShape::None,
        };
        // M3 field-report fix: own-state writeback -- vanilla's `RedStoneWireBlock::updateShape`
        // always recomputes and returns its own connection-encoded state for a horizontal
        // trigger direction; this blueprint's own real caller contract (`dispatch_one`/
        // `stage4.rs`'s private equivalent) writes the returned id and, if non-`None`, continues
        // the shape-update cascade one hop further -- so returning unconditionally here would
        // bounce that cascade back and forth along an already-settled wire run indefinitely (no
        // visited-set anywhere in that mechanism, the previous wave's own documented hazard,
        // `docs/findings-for-planning.md`). Gating on "did the recomputed id actually change"
        // instead mirrors vanilla's own real fixed-point termination: an `updateShape` call that
        // would return the state it already holds is, observably, a no-op.
        match ctx.get_block(pos) {
            Some(current) if is_wire_range(current.0) => {
                let new_id = new_connections_state_id(
                    current.0,
                    east_shape,
                    north_shape,
                    south_shape,
                    west_shape,
                );
                if new_id == current {
                    None
                } else {
                    Some(new_id)
                }
            }
            _ => None,
        }
    }
}
