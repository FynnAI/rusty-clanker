//! WORLD-D24's ticket/level system, scoped to `Player` tickets only (M2-B05 blueprint
//! Context). Region-agnostic: no `bevy_ecs` dependency, no I/O, no knowledge of chunk
//! *contents* -- pure `ChunkKey` coordinate/level bookkeeping. One instance per region
//! (M2: exactly one, owned by `rusty-clanker-server`'s tick-loop thread).

use std::collections::{HashMap, HashSet};

use rc_core::ChunkKey;

pub const PLAYER_TICKET_SOURCE_LEVEL: u8 = 31;
pub const TICKING_LEVEL: u8 = 32;
pub const BORDER_LEVEL: u8 = 33;
pub const MAX_TICKET_LEVEL: u8 = 44;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerTicketId(pub i32); // wraps M1-B05's PlayerMarker::network_entity_id

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChunkLoadState {
    EntityTicking,
    Ticking,
    Border,
    Inaccessible,
}

impl ChunkLoadState {
    /// WORLD-D24's exact table (Context). `None` (untracked) maps to `Inaccessible`.
    pub const fn from_level(level: Option<u8>) -> Self {
        todo!()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChunkChurn {
    /// Chunks whose level just became `<= BORDER_LEVEL` and were not tracked at the
    /// previous `step()` call.
    pub needs_load: Vec<ChunkKey>,
    /// Chunks over `BORDER_LEVEL` at both this call and the immediately preceding one
    /// (WORLD-D25's hysteresis, Context), or currently over `BORDER_LEVEL` while
    /// `memory_pressure` is set (WORLD-D26's acceleration, Context).
    pub needs_unload: Vec<ChunkKey>,
}

#[derive(Clone, Debug)]
struct PlayerTicket {
    center: ChunkKey,
    radius: u8,
}

/// WORLD-D24's ticket/level system, scoped to `Player` tickets only (Context).
/// Region-agnostic: no `bevy_ecs` dependency, no I/O, no knowledge of chunk *contents* --
/// pure `ChunkKey` coordinate/level bookkeeping. One instance per region (M2: exactly
/// one, owned by `rusty-clanker-server`'s tick-loop thread).
pub struct TicketManager {
    tickets: HashMap<PlayerTicketId, PlayerTicket>,
    levels: HashMap<ChunkKey, u8>,
    tracked: HashMap<ChunkKey, u8>,
    over_threshold_last_step: HashSet<ChunkKey>,
    memory_pressure: bool,
}

impl TicketManager {
    pub fn new() -> Self {
        todo!()
    }

    /// Registers (or replaces) `player`'s ticket, centered at `center` with the given
    /// `radius` (chunks; WORLD-D24's vanilla default is `10`, operator-configurable).
    pub fn register_player(&mut self, player: PlayerTicketId, center: ChunkKey, radius: u8) {
        todo!()
    }

    /// WORLD-D24's "re-centered on chunk crossing" -- no production call site exists at
    /// M2 (no movement mechanics before `M3`/`M4`, Context); exposed for a future
    /// mechanics blueprint and this blueprint's own synthetic-movement churn tests.
    pub fn move_player(&mut self, player: PlayerTicketId, new_center: ChunkKey) {
        todo!()
    }

    pub fn unregister_player(&mut self, player: PlayerTicketId) {
        todo!()
    }

    /// WORLD-D26's memory-budget flag (Context) -- set by whoever tracks the actual byte
    /// budget (out of this blueprint's own scope to implement the byte counter itself).
    pub fn set_memory_pressure(&mut self, over_budget: bool) {
        todo!()
    }

    /// The most recently computed level for `key` (as of the last `step()` call), if any
    /// ticket reaches it, or if `key` is still tracked pending a second consecutive
    /// over-threshold `step()` before unload (WORLD-D25's hysteresis window).
    pub fn level(&self, key: ChunkKey) -> Option<u8> {
        todo!()
    }

    pub fn load_state(&self, key: ChunkKey) -> ChunkLoadState {
        todo!()
    }

    /// Recomputes every reachable chunk's level from the current ticket set (Context's
    /// exact `contribution` formula) and returns this step's load/unload churn (driven by
    /// `tracked`, not `levels` -- see that field's own doc comment). Call exactly once
    /// per tick.
    pub fn step(&mut self) -> ChunkChurn {
        todo!()
    }
}

impl Default for TicketManager {
    fn default() -> Self {
        Self::new()
    }
}
