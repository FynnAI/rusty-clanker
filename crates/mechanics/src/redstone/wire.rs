//! Redstone wire — classic (default) power evaluator (MECH-D11/D12, Context §D).

use std::collections::HashMap;
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

/// blocks.json's own per-direction `[up, side, none]` index (`up`=0, `side`=1, `none`=2) —
/// this project's own `WireConnections` only ever tracks "does this side connect at all"
/// (Context §D's documented scope narrowing, restated on that struct's own doc comment below),
/// never the *visual* up-vs-side distinction real vanilla's own `RedstoneSide` enum carries, so
/// a connected side is always encoded `side` (never `up`) here — a documented, bounded
/// approximation (`docs/findings-for-planning.md` records the still-open follow-up). Only
/// `on_neighbor_changed`'s own power-only writeback below ever preserves a pre-existing `up`
/// bit a position's id already held, by construction, since it decodes and re-encodes every
/// connection digit unchanged and only ever replaces the power digit.
fn side_index(connected: bool) -> u32 {
    if connected { 1 } else { 2 }
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
/// recomputed `WireConnections` (`side_index`'s own `side`/`none`-only encoding), preserving
/// whatever `power` digit the position's own current raw id already holds.
fn new_connections_state_id(current_raw: u32, connections: WireConnections) -> BlockStateId {
    let (_east, _north, power, _south, _west) = wire_decode(current_raw);
    wire_state_id(
        side_index(connections.east),
        side_index(connections.north),
        power,
        side_index(connections.south),
        side_index(connections.west),
    )
}

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
        Self {
            state: Mutex::new(HashMap::new()),
            registry: OnceLock::new(),
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
    /// `block_signal` uses `block_signal_excluding_wire`, not the shared `signal::best_neighbor_
    /// signal` every other component calls (M3 field-report fix: own-state writeback follow-up
    /// -- own doc comment below).
    fn compute_power(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        let block_signal = block_signal_excluding_wire(world, registry, pos);
        if block_signal == 15 {
            return 15;
        }
        block_signal.max(self.incoming_wire_signal(world, registry, pos))
    }

    /// `shouldConnectTo`/`getConnectionState` (Context §D), restated as a single "does this
    /// side connect at all" boolean (this blueprint's own documented scope narrowing -- the
    /// visual `NONE`/`SIDE`/`UP` three-way property is not modeled).
    fn connects_on_side(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
        dir: Direction,
    ) -> bool {
        let same_height = dir.apply(pos);
        if let Some(state) = world.get_block(same_height)
            && registry
                .resolve(state)
                .connects_from(world, same_height, dir.opposite())
        {
            return true;
        }
        if !signal::is_conductor(world, Direction::Up.apply(pos)) {
            let up = Direction::Up.apply(same_height);
            if wire_power_at(world, registry, up).is_some() {
                return true;
            }
        }
        if !signal::is_conductor(world, same_height) {
            let down = Direction::Down.apply(same_height);
            if wire_power_at(world, registry, down).is_some() {
                return true;
            }
        }
        false
    }

    fn compute_connections(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> WireConnections {
        WireConnections {
            west: self.connects_on_side(world, registry, pos, Direction::West),
            east: self.connects_on_side(world, registry, pos, Direction::East),
            north: self.connects_on_side(world, registry, pos, Direction::North),
            south: self.connects_on_side(world, registry, pos, Direction::South),
        }
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

/// `getBlockSignal`/`level.getBestNeighborSignal` (Context §D), computed with wire's own
/// `shouldSignal` flag effectively disabled (M3 field-report fix: own-state writeback
/// follow-up, research doc §3.1: "wire's own `isSignalSource` temporarily disabled via the
/// `shouldSignal` flag to avoid self-counting"). Mirrors `signal::best_neighbor_signal` exactly
/// except: any neighbor that is itself a wire tile (`wire_power_at` returns `Some`, the same
/// `raw_wire_power` hook `incoming_wire_signal` already uses to identify "is this a wire")
/// contributes `0` here, regardless of its own real power or connectivity. Real vanilla applies
/// this exclusion globally while *any* wire computes its own target strength — not only to
/// avoid a wire reading back its own power through a QC bounce off its own supporting block,
/// but, just as importantly, to keep wire-to-wire power transfer flowing *exclusively* through
/// `incoming_wire_signal`'s own dedicated, decay-aware (`-1` per hop) walk. Without this
/// exclusion, once `on_shape_update` establishes real connectivity between two adjacent wire
/// tiles, a source-adjacent tile's own undecayed `weak_signal_toward` output would reach this
/// function's own `== 15` short-circuit directly, propagating power=15 with zero decay down an
/// entire wire run (confirmed empirically: `redstone/pulse/wire_signal_decay_15_chain`'s own
/// parity-check regression once own-state writeback made real connections observable,
/// `redstone_wire.rs`'s own `wire_chain_decays_correctly_once_neighbors_are_shape_connected`
/// regression test). Every *non*-wire signal source (torch, redstone_block, diode) still
/// contributes normally, exactly as `signal::best_neighbor_signal` already would.
fn block_signal_excluding_wire(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    pos: BlockPos,
) -> u8 {
    crate::direction::NEIGHBOR_CHANGED_ORDER
        .into_iter()
        .map(|dir| {
            let npos = dir.apply(pos);
            if wire_power_at(world, registry, npos).is_some() {
                0
            } else {
                signal::signal_into(world, registry, pos, dir)
            }
        })
        .max()
        .unwrap_or(0)
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
    /// `Down` only, unconditional on power (Context §A's worked QC example).
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
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
        let connections = self.compute_connections(ctx.world, &registry, pos);
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_default()
            .connections = connections;
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
                let new_id = new_connections_state_id(current.0, connections);
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
