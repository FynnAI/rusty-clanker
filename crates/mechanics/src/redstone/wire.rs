//! Redstone wire — classic (default) power evaluator (MECH-D11/D12, Context §D).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

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
    fn compute_power(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        let block_signal = signal::best_neighbor_signal(world, registry, pos);
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
        // The unconditional 7-cell-plus notify (Context §D): `pos` itself first, then its own
        // 6 neighbors in `NEIGHBOR_CHANGED_ORDER` -- no shape update is fired.
        signal::notify_neighbor_changed_only(ctx, pos);
        for dir in crate::direction::NEIGHBOR_CHANGED_ORDER {
            signal::notify_neighbor_changed_only(ctx, dir.apply(pos));
        }
    }
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        _from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        let registry = Arc::clone(self.registry());
        let connections = self.compute_connections(ctx.world, &registry, pos);
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_default()
            .connections = connections;
        None
    }
}
