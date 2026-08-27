//! M3-B07's own packet-observation module, additive to `idle_stability` (module doc
//! comment): wraps the same azalea bot connection `idle_stability` already
//! establishes the pattern for, exposing a plain, azalea-free surface to callers
//! (mirroring `idle_stability::ScenarioOutcome`'s own "wrap azalea behind clean
//! project types" discipline). `corpus_capture` (this crate's own new module) and
//! `rc-gametest`'s `capture` module are the only consumers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum PacketCaptureError {
    #[error("connect/login timed out")]
    LoginTimeout,
    #[error("azalea error: {0}")]
    Azalea(String),
}

type WorldPos = (i32, i32, i32);
type StateMap = Arc<Mutex<HashMap<WorldPos, u32>>>;
type AnalogMap = Arc<Mutex<HashMap<WorldPos, u8>>>;

/// A live, continuously-updated view over one bot session's observed block state
/// (module doc comment, "Packet observation"). Cheap to clone (`Arc`-backed); every
/// clone observes the same underlying map.
#[derive(Clone, Default)]
pub struct BlockSnapshotView {
    states: StateMap,
    analogs: AnalogMap,
}

impl BlockSnapshotView {
    /// A freshly-constructed view with no packets recorded yet — this crate's own
    /// test-only constructor (`packet_capture_types.rs`); `connect_and_observe`
    /// below constructs one internally and never exposes this constructor to a real
    /// caller's own choice of empty-vs-populated state.
    pub fn new() -> Self {
        todo!()
    }

    pub fn state_id_at(&self, pos: (i32, i32, i32)) -> Option<u32> {
        todo!()
    }

    pub fn analog_at(&self, pos: (i32, i32, i32)) -> Option<u8> {
        todo!()
    }
}

/// Disconnects the bot cleanly on `Drop` (mirrors `idle_stability`'s own
/// clean-disconnect discipline).
pub struct ObserverHandle {
    // private
}

/// Connects one offline-account bot (module doc comment, "Bot connection") and
/// returns a live `BlockSnapshotView` updated from every clientbound
/// block-state-affecting packet this session receives.
pub async fn connect_and_observe(
    host: &str,
    port: u16,
    account_name: &str,
    login_timeout: Duration,
) -> Result<(BlockSnapshotView, ObserverHandle), PacketCaptureError> {
    todo!()
}
