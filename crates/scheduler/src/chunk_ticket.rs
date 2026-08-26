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
        match level {
            Some(l) if l <= PLAYER_TICKET_SOURCE_LEVEL => ChunkLoadState::EntityTicking,
            Some(TICKING_LEVEL) => ChunkLoadState::Ticking,
            Some(BORDER_LEVEL) => ChunkLoadState::Border,
            _ => ChunkLoadState::Inaccessible,
        }
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
    /// The full, fresh-every-step level snapshot (Context's `contribution` formula,
    /// unfiltered -- every level `31..=44` any ticket's flood-fill reaches this step).
    /// Recomputed wholesale each `step()` call; `level()`/`load_state()` read directly
    /// from this map. No history is needed here: a key's level is a pure function of the
    /// current ticket set, so this map carries no hysteresis state of its own.
    levels: HashMap<ChunkKey, u8>,
    /// The *residency-eligible* bookkeeping set this module's own hysteresis (WORLD-D25)
    /// and `needs_load`/`needs_unload` churn are computed from -- unlike `levels` above,
    /// this map only ever contains a key once it has been observed at `level <=
    /// BORDER_LEVEL` (i.e., it was, or still is, actually load-eligible/resident), and it
    /// keeps tracking that key across the hysteresis window even after it recedes past
    /// `BORDER_LEVEL`, removing it only in the exact `step()` call that places it into
    /// `ChunkChurn::needs_unload`.
    ///
    /// M2-B05 implementation note (a forced, necessary deviation from the blueprint's own
    /// literal `step()` pseudocode, recorded here and in the implementation changeset's
    /// commit body): a single unfiltered map cannot serve both roles. `contribution`'s own
    /// flood-fill reaches every level up to `44` out to Chebyshev distance `radius + 13`
    /// regardless of how small `radius` is (Context: "flood-fills outward +1 per chunk
    /// step, capped at 44") -- so even a single `radius: 0` ticket produces hundreds of
    /// `level > BORDER_LEVEL` "ring" keys that were *never* load-eligible. The blueprint's
    /// own literal pseudocode folds those ring keys into the very same hysteresis
    /// bookkeeping as genuinely-resident keys, which (a) cannot satisfy WORLD-D25's own
    /// two-consecutive-over-threshold rule for a key whose covering ticket vanishes
    /// entirely (the "self.levels = new_levels" line loses the key on the very step that
    /// needs to remember it was over-threshold last step too, exactly the case this
    /// blueprint's own tests 5-7 exercise), and (b) if patched only by *not* dropping
    /// receded keys, instead perpetually re-flags every one of those never-resident ring
    /// keys as `needs_unload` on every subsequent step for as long as the ticket exists,
    /// since their own `level > BORDER_LEVEL` status is stable, not transient. Splitting
    /// "the full level snapshot" (`levels`, above) from "the load-eligible hysteresis set"
    /// (this field) resolves both: `level()`/`load_state()` still see every reachable
    /// level exactly as `contribution` computes it (this blueprint's own test 1's exact
    /// per-distance level values, including the ring itself, are unaffected), while
    /// `needs_load`/`needs_unload` churn is driven only by keys that were genuinely
    /// load-eligible at some point -- matching every one of this blueprint's own
    /// acceptance-test scenarios (5, 6, 7) exactly.
    tracked: HashMap<ChunkKey, u8>,
    over_threshold_last_step: HashSet<ChunkKey>,
    memory_pressure: bool,
}

/// `contribution`'s own level formula (WORLD-D24's ticket/level system, resolved to a
/// closed-form formula, Context): Chebyshev distance from the ticket's own center,
/// source level `31`, `+1` per chunk step past `radius`, capped at `44`.
fn contribution(ticket: &PlayerTicket, key: ChunkKey) -> Option<u8> {
    if key.dimension != ticket.center.dimension {
        return None;
    }
    let dx = key.x.abs_diff(ticket.center.x);
    let dz = key.z.abs_diff(ticket.center.z);
    let d = dx.max(dz);
    let radius = ticket.radius as u32;
    let level: u32 = if d <= radius {
        PLAYER_TICKET_SOURCE_LEVEL as u32
    } else {
        PLAYER_TICKET_SOURCE_LEVEL as u32 + (d - radius)
    };
    if level <= MAX_TICKET_LEVEL as u32 {
        Some(level as u8)
    } else {
        None
    }
}

impl TicketManager {
    pub fn new() -> Self {
        Self {
            tickets: HashMap::new(),
            levels: HashMap::new(),
            tracked: HashMap::new(),
            over_threshold_last_step: HashSet::new(),
            memory_pressure: false,
        }
    }

    /// Registers (or replaces) `player`'s ticket, centered at `center` with the given
    /// `radius` (chunks; WORLD-D24's vanilla default is `10`, operator-configurable).
    pub fn register_player(&mut self, player: PlayerTicketId, center: ChunkKey, radius: u8) {
        self.tickets.insert(player, PlayerTicket { center, radius });
    }

    /// WORLD-D24's "re-centered on chunk crossing" -- no production call site exists at
    /// M2 (no movement mechanics before `M3`/`M4`, Context); exposed for a future
    /// mechanics blueprint and this blueprint's own synthetic-movement churn tests.
    pub fn move_player(&mut self, player: PlayerTicketId, new_center: ChunkKey) {
        if let Some(ticket) = self.tickets.get_mut(&player) {
            ticket.center = new_center;
        }
    }

    pub fn unregister_player(&mut self, player: PlayerTicketId) {
        self.tickets.remove(&player);
    }

    /// WORLD-D26's memory-budget flag (Context) -- set by whoever tracks the actual byte
    /// budget (out of this blueprint's own scope to implement the byte counter itself).
    pub fn set_memory_pressure(&mut self, over_budget: bool) {
        self.memory_pressure = over_budget;
    }

    /// The most recently computed level for `key` (as of the last `step()` call), if any
    /// ticket reaches it, or if `key` is still tracked pending a second consecutive
    /// over-threshold `step()` before unload (WORLD-D25's hysteresis window).
    pub fn level(&self, key: ChunkKey) -> Option<u8> {
        self.levels.get(&key).copied()
    }

    pub fn load_state(&self, key: ChunkKey) -> ChunkLoadState {
        ChunkLoadState::from_level(self.level(key))
    }

    /// Recomputes every reachable chunk's level from the current ticket set (Context's
    /// exact `contribution` formula) and returns this step's load/unload churn (driven by
    /// `tracked`, not `levels` -- see that field's own doc comment). Call exactly once
    /// per tick.
    pub fn step(&mut self) -> ChunkChurn {
        let mut new_levels: HashMap<ChunkKey, u8> = HashMap::new();
        for ticket in self.tickets.values() {
            let span = ticket.radius as i64
                + (MAX_TICKET_LEVEL as i64 - PLAYER_TICKET_SOURCE_LEVEL as i64);
            for dx in -span..=span {
                for dz in -span..=span {
                    let key = ChunkKey {
                        dimension: ticket.center.dimension,
                        x: ticket.center.x.wrapping_add(dx as i32),
                        z: ticket.center.z.wrapping_add(dz as i32),
                    };
                    if let Some(level) = contribution(ticket, key) {
                        new_levels
                            .entry(key)
                            .and_modify(|existing| *existing = (*existing).min(level))
                            .or_insert(level);
                    }
                }
            }
        }

        let mut churn = ChunkChurn::default();
        for (&key, &level) in &new_levels {
            if level <= BORDER_LEVEL && !self.tracked.contains_key(&key) {
                churn.needs_load.push(key);
            }
        }

        // Only keys that are, or were, load-eligible (`level <= BORDER_LEVEL` at some
        // point) ever enter the hysteresis bookkeeping -- see `tracked`'s own doc comment
        // for why folding the unfiltered flood-fill "ring" into this same set is wrong.
        let mut all_keys: HashSet<ChunkKey> = self.tracked.keys().copied().collect();
        for (&key, &level) in &new_levels {
            if level <= BORDER_LEVEL {
                all_keys.insert(key);
            }
        }

        let mut over_this_step: HashSet<ChunkKey> = HashSet::new();
        let mut next_tracked: HashMap<ChunkKey, u8> = HashMap::new();

        for key in all_keys {
            match new_levels.get(&key) {
                Some(&level) if level <= BORDER_LEVEL => {
                    next_tracked.insert(key, level);
                }
                Some(&level) => {
                    // Reachable, but at a level past the resident/border boundary.
                    over_this_step.insert(key);
                    if self.over_threshold_last_step.contains(&key) || self.memory_pressure {
                        churn.needs_unload.push(key);
                    } else {
                        next_tracked.insert(key, level);
                    }
                }
                None => {
                    // Fell out of every ticket's reach entirely this step.
                    over_this_step.insert(key);
                    if self.over_threshold_last_step.contains(&key) || self.memory_pressure {
                        churn.needs_unload.push(key);
                    } else if let Some(&last) = self.tracked.get(&key) {
                        next_tracked.insert(key, last);
                    }
                }
            }
        }

        self.tracked = next_tracked;
        self.over_threshold_last_step = over_this_step;
        self.levels = new_levels;
        churn
    }
}

impl Default for TicketManager {
    fn default() -> Self {
        Self::new()
    }
}
