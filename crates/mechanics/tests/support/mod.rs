//! Shared test doubles for M3-B04's acceptance suite (Acceptance tests' own opening
//! paragraph): `FakeWorld` (`BlockWorldAccess`, mirroring M3-B01's own `stage4_ordering.rs`
//! -local `FakeWorld` pattern) and `TestSignalSource` (`RedstoneSignalSource`, standing in for
//! "a lever/button would provide this input," Context §H), reused across every `redstone_*.rs`
//! file in this directory via `mod support;`.
//!
//! Each `tests/*.rs` file that does `mod support;` is compiled as its own, separate crate
//! (Cargo's own integration-test model), so `dead_code` analysis runs independently per
//! consuming file — by this module's own design (shared across multiple, differently-scoped
//! acceptance test files), no single consumer uses every item declared here.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Mutex;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::LightDirtyQueue;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::RedstoneSignalSource;
use rc_mechanics::stage4::run_scheduled_phase;
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo, NeighborUpdateEngine,
    ScheduledTickQueue,
};
use rc_messaging::{Address, RegionId};

/// A `HashMap<BlockPos, BlockStateId>`-backed `BlockWorldAccess`, with a fixed single-region
/// `local` identity (`owner_of` is unused by this blueprint's own tests, which route ownership
/// entirely through `RegionOwnership::always_local`/an explicit custom `resolve` closure, never
/// through `BlockWorldAccess::owner_of` itself — mirrored here only to satisfy the trait).
pub struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    pub local: Address,
}

impl FakeWorld {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            local: Address::Region(RegionId(0)),
        }
    }

    pub fn with_local(local: Address) -> Self {
        Self {
            blocks: HashMap::new(),
            local,
        }
    }
}

impl Default for FakeWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockWorldAccess for FakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        self.local
    }
    fn local_identity(&self) -> Address {
        self.local
    }
}

/// `M4-B06` — a `HashMap<BlockPos, BlockStateId>`-backed `BlockWorldAccess` where every
/// unset position resolves to a caller-supplied default (`fluid_spread_golden.rs`'s own
/// "all terrain defaulting to solid stone unless explicitly set to air/fluid" convention),
/// rather than `FakeWorld`'s own "unset = no block" (`None`) — the fluid algorithm's own
/// occlusion/solidity checks (`occlusion::is_solid`/`is_full_cube`) need every position to
/// resolve to *some* shape, and "no block loaded" would otherwise silently read as "no
/// occlusion at all" rather than "ordinary solid terrain," which is not what any of the
/// fluid acceptance tests intend to exercise.
pub struct FluidFakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    pub default_state: BlockStateId,
    pub local: Address,
}

impl FluidFakeWorld {
    pub fn new(default_state: BlockStateId) -> Self {
        Self {
            blocks: HashMap::new(),
            default_state,
            local: Address::Region(RegionId(0)),
        }
    }

    pub fn set(&mut self, pos: BlockPos, state: BlockStateId) {
        self.blocks.insert(pos, state);
    }
}

impl BlockWorldAccess for FluidFakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        Some(self.blocks.get(&pos).copied().unwrap_or(self.default_state))
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        self.local
    }
    fn local_identity(&self) -> Address {
        self.local
    }
}

/// `M4-B06` — runs `stage4::run_scheduled_phase` once per simulated tick, `ticks` times, over a
/// single local region (`ownership`, no inbound border events). Fluid settling is not
/// necessarily "queue eventually empties" (a stable fluid network keeps re-arming its own
/// neighbors forever via `set_block`'s own "a no-op write still fans out" rule, exactly mirroring
/// vanilla's own perpetual-but-observably-stable re-tick behavior) — every golden/settling test
/// instead runs a fixed, generously-sized tick budget and asserts on the resulting world state.
#[allow(clippy::too_many_arguments)]
pub fn settle_fluids(
    world: &mut FluidFakeWorld,
    scheduled: &mut ScheduledTickQueue,
    registry: &BlockBehaviorRegistry,
    ownership: &RegionOwnership,
    ticks: u64,
) {
    let mut engine = NeighborUpdateEngine::new();
    let mut events = BlockEventQueue::new();
    let mut halo = BorderHalo::new();
    for current_tick in 0..ticks {
        let mut outbound = Vec::new();
        let mut changed = Vec::new();
        let mut light_dirty = LightDirtyQueue::new();
        run_scheduled_phase(
            world,
            &[],
            &mut halo,
            ownership,
            &mut engine,
            scheduled,
            &mut events,
            registry,
            &mut outbound,
            &mut changed,
            &mut light_dirty,
            current_tick,
        );
    }
}

/// Externally-settable `RedstoneSignalSource` standing in for "a lever/button would provide
/// this input" (Context §H) — no real lever/button block type exists in this codebase yet.
pub struct TestSignalSource {
    power: Mutex<u8>,
    is_diode: bool,
    connects: Mutex<HashMap<Direction, bool>>, // per-direction override; missing = default true
}

impl TestSignalSource {
    /// A plain, non-diode signal source fixed at `power` — every direction's weak/direct
    /// output is `power`, connects from every direction by default.
    pub fn fixed(power: u8) -> Self {
        Self {
            power: Mutex::new(power),
            is_diode: false,
            connects: Mutex::new(HashMap::new()),
        }
    }

    /// As `fixed`, but `is_diode() == true` — `repeater_lock_is_boolean_not_magnitude`'s own
    /// side-input stand-in (Acceptance tests' own test note: `TestSignalSource::
    /// with_diode_flag(power)`).
    pub fn with_diode_flag(power: u8) -> Self {
        Self {
            power: Mutex::new(power),
            is_diode: true,
            connects: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_power(&self, power: u8) {
        *self.power.lock().unwrap() = power;
    }

    /// Overrides `connects_from`'s own per-direction answer (default `true` for every
    /// direction not explicitly set) — `weak_signal_gated_by_connects_from`'s own fixture.
    pub fn set_connects_from(&self, from: Direction, value: bool) {
        self.connects.lock().unwrap().insert(from, value);
    }
}

impl RedstoneSignalSource for TestSignalSource {
    fn weak_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        *self.power.lock().unwrap()
    }
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        *self.power.lock().unwrap()
    }
    fn is_signal_source(&self) -> bool {
        true
    }
    fn is_diode(&self) -> bool {
        self.is_diode
    }
    fn connects_from(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        from: Direction,
    ) -> bool {
        self.connects
            .lock()
            .unwrap()
            .get(&from)
            .copied()
            .unwrap_or(true)
    }
}
