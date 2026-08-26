//! `play::persistence` — the live, currently-connected-players' player-record working
//! set (M2-B06). Fully self-contained: takes any `Arc<dyn PlayerDataStore>` (real or
//! fake), never `HardcodedWorld` or any other `rusty-clanker-server`-internal type —
//! independently constructible and testable. See `blueprints/M2/M2-B06-player-
//! persistence.md` for the full design, including the composition-root integration
//! recipe (not part of this blueprint's own Tier-1 gate).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{
    LoadedPlayerRecord, PlayerDataStore, PlayerPersistenceError, load_player, save_player,
};

/// Default per-player save interval: `6000` ticks / 5 minutes (WORLD-D23's own
/// already-established default) — used only when no `WorldConfig::
/// save_interval_ticks()` (M2-B05) is already present to reuse instead.
pub const DEFAULT_SAVE_INTERVAL_TICKS: u64 = 6000;

#[derive(Clone, Copy, Debug)]
pub struct PlayerPersistenceConfig {
    pub save_interval_ticks: u64,
}

impl Default for PlayerPersistenceConfig {
    fn default() -> Self {
        todo!()
    }
}

/// The live, currently-connected-players' working set. `Clone`, cheap (`Arc`-backed).
#[derive(Clone)]
pub struct PlayerSessionStore {
    store: Arc<dyn PlayerDataStore>,
    sessions: Arc<Mutex<HashMap<uuid::Uuid, LoadedPlayerRecord>>>,
}

impl PlayerSessionStore {
    pub fn new(store: Arc<dyn PlayerDataStore>) -> Self {
        todo!()
    }

    /// Loads (or freshly defaults, via `LoadedPlayerRecord::fresh_default(dimension,
    /// default_pos)`) `uuid`'s record, inserts it into the live set, and returns a
    /// clone of its current `pos`/`rotation`.
    pub fn load_or_create(
        &self,
        uuid: uuid::Uuid,
        dimension: rc_core::DimensionId,
        default_pos: [f64; 3],
    ) -> Result<([f64; 3], [f32; 2]), PlayerPersistenceError> {
        todo!()
    }

    /// Synchronously saves `uuid`'s current record and removes it from the live set.
    /// A no-op (`Ok(())`) if `uuid` is not currently present.
    pub fn save_and_remove(&self, uuid: uuid::Uuid) -> Result<(), PlayerPersistenceError> {
        todo!()
    }

    /// Clones every currently-connected player's `(Uuid, LoadedPlayerRecord)` pair (a
    /// short-held lock) without removing anything from the live set.
    pub fn snapshot_all(&self) -> Vec<(uuid::Uuid, LoadedPlayerRecord)> {
        todo!()
    }

    /// Saves every entry `snapshot_all` would return, logging (`tracing::warn!`,
    /// never panicking) on any individual failure.
    pub fn save_all(&self) {
        todo!()
    }

    /// Direct mutable access to one live record — this blueprint's own stand-in for
    /// "the player's own action (block break, item pickup) changed their state".
    /// `None` if `uuid` is not currently connected.
    pub fn with_record_mut<R>(
        &self,
        uuid: uuid::Uuid,
        f: impl FnOnce(&mut LoadedPlayerRecord) -> R,
    ) -> Option<R> {
        todo!()
    }
}
