//! M3-B08's 20-bot single-region load-test scenario (Acceptance Criterion 2,
//! `11-roadmap-milestones.md`: "20 simulated bots... concentrated within a single
//! region... sustained tick rate measured... within ±1% of the target 20 TPS").
//!
//! Test-authoring changeset (TEST-D45/D46): struct/enum shapes final, function
//! bodies `todo!()` — every acceptance test in `tests/load_scenario_layout.rs` fails
//! until the following governance commit fills these in. `run_one_load_bot`/
//! `run_load_scenario` are azalea-driven and exercised only by a real `m3-report`
//! run (Tier 2/manual), never by this blueprint's own Tier-1 test changeset.

use std::time::Duration;

use rc_core::BlockPos;

/// ARCH-D6's own grid-cell size, restated locally — `rc-paritybot` has no dependency
/// on `rc-scheduler` (a production crate; WS-D3's dependency-graph rule keeps test
/// crates from depending on it), Context.
pub const GRID_CELL_BLOCKS: i32 = 256;

pub const ARENA_MIN: (i32, i32) = (32, 32);
pub const ARENA_MAX: (i32, i32) = (224, 224);
pub const BASE_Y: i32 = -59;
pub const COLS: u32 = 5;
pub const ROWS: u32 = 4;
pub const PATROL_HALF_EXTENT: i32 = 3;
pub const INTERACTION_POST_OFFSET_SOUTH: i32 = 2;
pub const INTERACTION_PERIOD_TICKS: u32 = 40;
pub const START_STAGGER_TICKS_PER_BOT: u32 = 2;
/// M3-B03's own `BLOCK_INTERACTION_RANGE_CREATIVE`, restated (Context: this
/// scenario's own layout margin is sized so this bound is trivially satisfied every
/// time, never approached).
pub const CREATIVE_REACH: f64 = 5.0;

/// ARCH-D6's floor-division grid-cell convention, restated locally (module doc
/// comment).
pub fn block_grid_cell(_x: i32, _z: i32) -> (i32, i32) {
    todo!()
}

#[derive(Debug, Clone)]
pub struct BotPlan {
    pub username: String,
    pub waypoints: [BlockPos; 4],
    pub interaction_post: BlockPos,
    pub start_offset_ticks: u32,
}

/// Pure, deterministic (Context's exact per-cell centering formula). Returns
/// `cols * rows` entries, row-major (`index = row * cols + col`), usernames
/// `format!("rc-load-bot-{index:02}")`.
pub fn plan_bot_layout(
    _cols: u32,
    _rows: u32,
    _arena_min: (i32, i32),
    _arena_max: (i32, i32),
    _base_y: i32,
) -> Vec<BotPlan> {
    todo!()
}

#[derive(Debug, Clone, Default)]
pub struct BotOutcome {
    pub reached_spawn: bool,
    pub waypoint_visits: u64,
    pub interaction_cycles: u64,
    /// `Some(d)` iff the bot disconnected before `run_duration` elapsed, `d` measured
    /// from the bot's own connection start. `None` means it ran the full duration and
    /// this function itself performed the clean shutdown.
    pub disconnected_at: Option<Duration>,
    pub disconnect_reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadBotError {
    #[error("no Event::Login observed within {0:?}")]
    LoginTimeout(Duration),
}

/// Runs one bot's full behavior loop (Context) against `plan`. Only a login timeout
/// is `Err` — any later disconnect is captured in the returned `BotOutcome` (`Ok`),
/// so the caller can keep the other 19 bots running.
pub async fn run_one_load_bot(
    _host: &str,
    _port: u16,
    _plan: &BotPlan,
    _login_timeout: Duration,
    _run_duration: Duration,
) -> Result<BotOutcome, LoadBotError> {
    todo!()
}

pub struct LoadScenarioConfig {
    pub host: String,
    pub port: u16,
    pub cols: u32,
    pub rows: u32,
    pub arena_min: (i32, i32),
    pub arena_max: (i32, i32),
    pub base_y: i32,
    pub login_timeout: Duration,
    pub run_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct LoadScenarioReport {
    pub bot_count: u32,
    /// `(username, outcome-or-login-error-message)`, one entry per planned bot, in
    /// `plan_bot_layout`'s own order.
    pub per_bot: Vec<(String, Result<BotOutcome, String>)>,
}

impl LoadScenarioReport {
    /// `true` iff every entry is `Ok(outcome)` with `outcome.disconnected_at.is_none()`
    /// — every bot reached Spawn, ran the entire scenario, and disconnected only via
    /// this scenario's own clean shutdown at the end.
    pub fn all_completed_cleanly(&self) -> bool {
        todo!()
    }

    pub fn disconnected_or_failed_count(&self) -> u32 {
        todo!()
    }
}

/// Orchestrates the whole load test: plans every bot, runs them all concurrently,
/// joins them, and assembles the report. Never panics on an individual bot's own
/// `Err`/disconnect — those are data, not a reason to abort the other 19.
pub async fn run_load_scenario(_config: LoadScenarioConfig) -> LoadScenarioReport {
    todo!()
}
