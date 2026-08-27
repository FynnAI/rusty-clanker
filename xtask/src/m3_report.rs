//! M3-B08: drives the M3 acceptance harness (AC1: the redstone parity corpus, via
//! M3-B07's already-built `xtask::corpus` verbs, reused unmodified; AC2: a 20-bot
//! single-region load-test leg) against a real, freshly-spawned
//! `rusty-clanker-server` and writes `target/verify/m3-acceptance.json`.
//!
//! Test-authoring changeset (TEST-D45/D46): struct/enum shapes final, function
//! bodies `todo!()` — every acceptance test in `xtask/tests/m3_report_cli.rs` that
//! exercises a stubbed body fails until the following governance commit fills these
//! in. See that governance commit's own module doc comment for the forced deviations
//! this module carries (never links `rc-paritybot`/`azalea`; `build_report`'s `bots`
//! parameter is this module's own azalea-free local mirror of
//! `rc_paritybot::load_scenario::LoadScenarioReport`, not that type itself).

use std::path::PathBuf;
use std::time::Duration;

use rc_test_harness::tick_cadence::TpsReport;

use crate::tier_result::TierResult;

pub const OUT_PATH: &str = "target/verify/m3-acceptance.json";

#[derive(serde::Serialize)]
pub struct M3ReportResult {
    #[serde(flatten)]
    pub automated: TierResult, // tier = "m3-acceptance"; six cases, Context's table
    pub mode: String,          // "smoke" | "full"
    pub target: String,        // "<ip>:<port>" the load-test leg actually used
    pub load_test_duration_secs: u64,
    pub redstone_corpus_contraption_count: usize,
    pub bot_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    Smoke,
    Full,
}

impl Mode {
    /// `Smoke` -> `Duration::from_secs(60)`, `Full` -> `Duration::from_secs(600)`
    /// (AC2's own literal 10-real-minute threshold) — Context's "only duration
    /// compresses" rule; every other parameter (bot count, arena, interaction rate,
    /// corpus filter) is identical between modes.
    pub fn load_test_duration(self) -> Duration {
        todo!()
    }
}

/// An azalea-free local mirror of `rc_paritybot::load_scenario::BotOutcome` (module
/// doc comment).
#[derive(Debug, Clone, Default)]
pub struct LoadBotOutcome {
    pub reached_spawn: bool,
    pub waypoint_visits: u64,
    pub interaction_cycles: u64,
    pub disconnected_at: Option<Duration>,
    pub disconnect_reason: Option<String>,
}

/// An azalea-free local mirror of `rc_paritybot::load_scenario::LoadScenarioReport`.
#[derive(Debug, Clone, Default)]
pub struct LoadScenarioReport {
    pub bot_count: u32,
    pub per_bot: Vec<(String, Result<LoadBotOutcome, String>)>,
}

impl LoadScenarioReport {
    /// `true` iff every entry is `Ok(outcome)` with `outcome.disconnected_at.is_none()`
    /// — every bot reached Spawn, ran the entire scenario, and disconnected only via
    /// the scenario's own clean shutdown at the end.
    pub fn all_completed_cleanly(&self) -> bool {
        todo!()
    }

    pub fn disconnected_or_failed_count(&self) -> u32 {
        todo!()
    }
}

/// Pure: scans `stdout` for a line exactly matching `RC_REGION_COUNT=<digits>` and
/// returns the parsed value, `None` if no such line is present or it fails to parse.
pub fn parse_region_count_line(_stdout: &[String]) -> Option<u32> {
    todo!()
}

/// Pure aggregation (Acceptance tests exercise this directly against synthetic
/// inputs). Builds the six cases from Context's table and `finalize`s the wrapped
/// `TierResult`.
pub fn build_report(
    _mode: Mode,
    _target: String,
    _fetch_corpus_result: &TierResult,
    _parity_check_result: &TierResult,
    _tps: TpsReport,
    _bots: &LoadScenarioReport,
    _region_count_observed: Option<u32>,
) -> M3ReportResult {
    todo!()
}

/// CLI entry point (`xtask m3-report --server-bin <path> --mode {smoke|full}`).
pub fn run(_server_bin: PathBuf, _mode: Mode) -> std::process::ExitCode {
    todo!()
}
