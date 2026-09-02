//! M3-B08: drives the M3 acceptance harness (AC1: the redstone parity corpus, via
//! M3-B07's already-built `xtask::corpus` verbs, reused unmodified; AC2: a 20-bot
//! single-region load-test leg) against a real, freshly-spawned
//! `rusty-clanker-server` and writes `target/verify/m3-acceptance.json`.
//!
//! Forced deviation from this blueprint's own Deliverables sketch (Context: "mirrors
//! `m1_report::run`'s/`m2_report::run`'s identical isolation pattern... one isolated
//! `tokio::runtime::Runtime::new()?.block_on(...)` call"). That description does not
//! match what `m1_report.rs`/`m2_report.rs` actually do (both of those modules'
//! *own* doc comments state the real, load-bearing reason: `xtask.exe` must never
//! link `azalea`/`rc-paritybot` at all — WS-D4's pinned-*stable*-toolchain rule for
//! every Tier-1-gated binary, azalea's own `rust-toolchain.toml` pins `channel =
//! "nightly"` — so neither module ever calls into `rc-paritybot` in-process, with or
//! without a `tokio::runtime::Runtime`; each drives its own azalea-dependent leg by
//! spawning one of `rc-paritybot`'s own `*_runner` bin targets as a real OS
//! subprocess). This module follows the *actual*, established pattern instead: the
//! load-test leg is driven by spawning `rc-paritybot`'s new `load_scenario_runner`
//! bin target as a subprocess, identical in shape to `run_idle_stability_subprocess`/
//! `run_restart_persistence_subprocess`/`fetch_corpus.rs`'s own `run_fetch_corpus_
//! runner_subprocess`. This verb therefore stays fully synchronous — no `tokio::
//! runtime::Runtime`/`block_on` anywhere in `xtask`, exactly like every prior report
//! verb.
//!
//! A second, consequent deviation: `build_report`'s own `bots` parameter cannot be
//! typed `&rc_paritybot::load_scenario::LoadScenarioReport` — importing that type
//! would add the same forbidden Cargo dependency edge (`rc-paritybot`'s own
//! `Cargo.toml` already carries `azalea` unconditionally, so even importing one
//! azalea-free type from that crate pulls the whole crate, and therefore `azalea`,
//! into `xtask.exe`'s own build graph). This module instead defines its own
//! azalea-free `LoadScenarioReport`/`LoadBotOutcome` (below), field-for-field
//! identical to `rc_paritybot::load_scenario`'s own shape, that `run_load_scenario_
//! subprocess`'s own output parser fills in from the subprocess's line-based
//! protocol — the identical "local, necessary duplicate" precedent `m2_report.rs`'s
//! own `EXPECTED_BLOCKS`/`ReportRegistryResolvers` already establishes for the
//! identical underlying reason (a type/constant that legitimately exists in a
//! forbidden-to-depend-on crate).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rc_test_harness::process::{ManagedServerConfig, spawn_server};
use rc_test_harness::tick_cadence::{self, TpsReport};

use crate::corpus::{fetch_corpus, parity_check};
use crate::tier_result::{Status, TierResult};

pub const OUT_PATH: &str = "target/verify/m3-acceptance.json";

const TARGET_TPS: f64 = 20.0;
const TPS_TOLERANCE: f64 = 0.01;
const SERVER_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const BOT_LOGIN_TIMEOUT: Duration = Duration::from_secs(30);
/// A generous, fixed allowance for the nested `cargo run`'s own (possibly cold,
/// first-ever) build of the azalea-dependent `load_scenario_runner` binary — never
/// part of the timed load-test leg itself, purely a bound against a hung/never-
/// returning subprocess (mirrors every prior `*_runner` subprocess wrapper's own
/// identical `build_grace`).
const BUILD_GRACE: Duration = Duration::from_secs(300);

#[derive(serde::Serialize)]
pub struct M3ReportResult {
    #[serde(flatten)]
    pub automated: TierResult, // tier = "m3-acceptance"; six cases, Context's table
    pub mode: String,   // "smoke" | "full"
    pub target: String, // "<ip>:<port>" the load-test leg actually used
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
        match self {
            Mode::Smoke => Duration::from_secs(60),
            Mode::Full => Duration::from_secs(600),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Mode::Smoke => "smoke",
            Mode::Full => "full",
        }
    }
}

/// Module doc comment's own "Forced deviation": an azalea-free local mirror of
/// `rc_paritybot::load_scenario::BotOutcome`.
#[derive(Debug, Clone, Default)]
pub struct LoadBotOutcome {
    pub reached_spawn: bool,
    pub waypoint_visits: u64,
    pub interaction_cycles: u64,
    pub disconnected_at: Option<Duration>,
    pub disconnect_reason: Option<String>,
}

/// Module doc comment's own "Forced deviation": an azalea-free local mirror of
/// `rc_paritybot::load_scenario::LoadScenarioReport`.
#[derive(Debug, Clone, Default)]
pub struct LoadScenarioReport {
    pub bot_count: u32,
    pub per_bot: Vec<(String, Result<LoadBotOutcome, String>)>,
}

impl LoadScenarioReport {
    /// `true` iff every entry is `Ok(outcome)` with `outcome.disconnected_at.is_none()`
    /// — every bot reached Spawn, ran the entire scenario, and disconnected only via
    /// the scenario's own clean shutdown at the end. `false` (never vacuously `true`)
    /// when there is no bot data at all — e.g. the load-scenario subprocess itself
    /// never produced a parseable result.
    pub fn all_completed_cleanly(&self) -> bool {
        !self.per_bot.is_empty()
            && self.per_bot.iter().all(
                |(_, result)| matches!(result, Ok(outcome) if outcome.disconnected_at.is_none()),
            )
    }

    pub fn disconnected_or_failed_count(&self) -> u32 {
        self.per_bot
            .iter()
            .filter(
                |(_, result)| !matches!(result, Ok(outcome) if outcome.disconnected_at.is_none()),
            )
            .count() as u32
    }
}

/// Pure: scans `stdout` for a line exactly matching `RC_REGION_COUNT=<digits>` and
/// returns the parsed value, `None` if no such line is present or it fails to parse.
pub fn parse_region_count_line(stdout: &[String]) -> Option<u32> {
    for line in stdout {
        if let Some(rest) = line.strip_prefix("RC_REGION_COUNT=") {
            let rest = rest.trim();
            if !rest.is_empty()
                && rest.bytes().all(|b| b.is_ascii_digit())
                && let Ok(n) = rest.parse::<u32>()
            {
                return Some(n);
            }
        }
    }
    None
}

fn status_of(pass: bool) -> Status {
    if pass { Status::Pass } else { Status::Fail }
}

/// Pure aggregation (Acceptance tests exercise this directly against synthetic
/// inputs — the "perturbed redstone replay must fail the parity leg" and "lagged
/// engine must fail the TPS leg" self-tests both ultimately assert on this
/// function's own output, not merely on the lower-layer functions M3-B07/this
/// blueprint's own `tick_cadence` already separately prove correct). Builds the six
/// cases from Context's table and `finalize`s the wrapped `TierResult`.
pub fn build_report(
    mode: Mode,
    target: String,
    fetch_corpus_result: &TierResult,
    parity_check_result: &TierResult,
    tps: TpsReport,
    bots: &LoadScenarioReport,
    region_count_observed: Option<u32>,
) -> M3ReportResult {
    let mut result = TierResult::new("m3-acceptance");

    result.push(
        "AC1_fetch_corpus_capture_succeeded",
        status_of(fetch_corpus_result.status == Status::Pass),
        Some(format!(
            "fetch-corpus status: {:?}",
            fetch_corpus_result.status
        )),
    );
    result.push(
        "AC1_redstone_corpus_size_at_least_50",
        status_of(parity_check_result.cases.len() >= 50),
        Some(format!(
            "{} contraption(s) in the corpus",
            parity_check_result.cases.len()
        )),
    );
    result.push(
        "AC1_redstone_corpus_parity",
        status_of(parity_check_result.status == Status::Pass),
        Some(format!(
            "parity-check-redstone status: {:?}",
            parity_check_result.status
        )),
    );
    result.push(
        "AC2a_tps_within_one_percent_over_full_duration",
        status_of(tps.within_tolerance),
        Some(format!(
            "measured_tps={:.4} drift_ratio={:.4}",
            tps.measured_tps, tps.drift_ratio
        )),
    );
    result.push(
        "AC2b_all_bots_completed_without_unexpected_disconnect",
        status_of(bots.all_completed_cleanly()),
        Some(format!(
            "{} bot(s) total, {} disconnected/failed",
            bots.bot_count,
            bots.disconnected_or_failed_count()
        )),
    );
    result.push(
        "AC2c_single_region_topology_pinned",
        status_of(region_count_observed == Some(1)),
        Some(format!(
            "observed RC_REGION_COUNT={region_count_observed:?}"
        )),
    );

    let automated = result.finalize();
    M3ReportResult {
        automated,
        mode: mode.as_str().to_string(),
        target,
        load_test_duration_secs: mode.load_test_duration().as_secs(),
        redstone_corpus_contraption_count: parity_check_result.cases.len(),
        bot_count: bots.bot_count,
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

fn read_tier_result(path: &Path, tier: &str) -> TierResult {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<TierResult>(&content) {
            Ok(result) => result,
            Err(err) => {
                failed_tier_result(tier, format!("failed to parse {}: {err}", path.display()))
            }
        },
        Err(err) => failed_tier_result(tier, format!("failed to read {}: {err}", path.display())),
    }
}

fn failed_tier_result(tier: &str, detail: String) -> TierResult {
    let mut result = TierResult::new(tier);
    result.push("read-result", Status::Fail, Some(detail));
    result.finalize()
}

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, uniquely named temp directory, removed best-effort on `Drop` — no
/// `tempfile` dependency added (mirrors `m2_report.rs`'s own identical `TempWorldDir`
/// convention, restated locally here since `xtask` has no shared test-only helper
/// module this could otherwise live in).
struct TempWorldDir {
    path: PathBuf,
}

impl TempWorldDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-m3-report-{label}-{}-{}",
            std::process::id(),
            TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("create temp world dir");
        Self { path }
    }
}

impl Drop for TempWorldDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

enum RunnerOutcome {
    Report(LoadScenarioReport),
    ProcessFailure(String),
}

/// Builds and runs `rc-paritybot`'s `load_scenario_runner` as a subprocess (module
/// doc comment): `current_dir` set to `crates/testing/paritybot` so rustup resolves
/// that crate's own nested nightly `rust-toolchain.toml`, `RUSTC_BOOTSTRAP=1` set,
/// and `RUSTUP_TOOLCHAIN` removed from the child's environment — identical mechanism
/// to `m1_report::run_idle_stability_subprocess`/`fetch_corpus::run_fetch_corpus_
/// runner_subprocess`.
fn run_load_scenario_subprocess(
    repo_root: &Path,
    host: &str,
    port: u16,
    login_timeout: Duration,
    run_duration: Duration,
) -> RunnerOutcome {
    let paritybot_dir = repo_root.join("crates/testing/paritybot");

    let mut command = Command::new("cargo");
    command
        .current_dir(&paritybot_dir)
        .env("RUSTC_BOOTSTRAP", "1")
        .env_remove("RUSTUP_TOOLCHAIN")
        .args([
            "run",
            "--quiet",
            "--bin",
            "load_scenario_runner",
            "--",
            host,
            &port.to_string(),
            &login_timeout.as_secs().to_string(),
            &run_duration.as_secs().to_string(),
        ]);

    // M3 field-report fix (pipe-buffer deadlock), now centralized in `crate::process::
    // spawn_drained` (M3.5-B06): a real `load_scenario_runner` run's own output volume (20
    // concurrently-pathfinding azalea clients, confirmed live at 700KB+ of stderr in one
    // ordinary 60-second run) reliably fills the OS pipe buffer once nobody drains it until
    // after exit is observed -- `spawn_drained`'s own doc comment has the full original
    // diagnosis.
    let deadline = BUILD_GRACE + login_timeout + run_duration + Duration::from_secs(60);
    match crate::process::spawn_drained(&mut command, deadline) {
        Ok(output) => parse_load_scenario_runner_output(&output.stdout, &output.stderr),
        Err(crate::process::SpawnDrainedError::SpawnFailed(err)) => {
            RunnerOutcome::ProcessFailure(format!("failed to spawn load_scenario_runner: {err}"))
        }
        Err(crate::process::SpawnDrainedError::PollFailed(err)) => {
            RunnerOutcome::ProcessFailure(format!("failed to poll load_scenario_runner: {err}"))
        }
        Err(crate::process::SpawnDrainedError::TimedOut) => RunnerOutcome::ProcessFailure(format!(
            "load_scenario_runner did not exit within {deadline:?} of its own start"
        )),
    }
}

struct BotAccumulator {
    username: String,
    ok: bool,
    reached_spawn: bool,
    waypoints: u64,
    interactions: u64,
    disconnected_at_ms: Option<u64>,
    detail: String,
}

fn flush_bot(
    current: Option<BotAccumulator>,
    per_bot: &mut Vec<(String, Result<LoadBotOutcome, String>)>,
) {
    let Some(acc) = current else { return };
    let result = if acc.ok {
        Ok(LoadBotOutcome {
            reached_spawn: acc.reached_spawn,
            waypoint_visits: acc.waypoints,
            interaction_cycles: acc.interactions,
            disconnected_at: acc.disconnected_at_ms.map(Duration::from_millis),
            disconnect_reason: (!acc.detail.is_empty()).then_some(acc.detail),
        })
    } else {
        Err(acc.detail)
    };
    per_bot.push((acc.username, result));
}

fn parse_load_scenario_runner_output(stdout: &str, stderr: &str) -> RunnerOutcome {
    let mut current: Option<BotAccumulator> = None;
    let mut per_bot = Vec::new();
    let mut overall_result: Option<&str> = None;
    let mut overall_message = String::new();

    for line in stdout.lines() {
        if let Some(username) = line.strip_prefix("BOT_USERNAME=") {
            flush_bot(current.take(), &mut per_bot);
            current = Some(BotAccumulator {
                username: username.to_string(),
                ok: true,
                reached_spawn: false,
                waypoints: 0,
                interactions: 0,
                disconnected_at_ms: None,
                detail: String::new(),
            });
        } else if let Some(value) = line.strip_prefix("BOT_RESULT=") {
            if let Some(acc) = current.as_mut() {
                acc.ok = value == "OK";
            }
        } else if let Some(value) = line.strip_prefix("BOT_REACHED_SPAWN=") {
            if let Some(acc) = current.as_mut() {
                acc.reached_spawn = value == "true";
            }
        } else if let Some(value) = line.strip_prefix("BOT_WAYPOINTS=") {
            if let Some(acc) = current.as_mut() {
                acc.waypoints = value.parse().unwrap_or(0);
            }
        } else if let Some(value) = line.strip_prefix("BOT_INTERACTIONS=") {
            if let Some(acc) = current.as_mut() {
                acc.interactions = value.parse().unwrap_or(0);
            }
        } else if let Some(value) = line.strip_prefix("BOT_DISCONNECTED_AT_MS=") {
            if let Some(acc) = current.as_mut() {
                acc.disconnected_at_ms = value.parse().ok();
            }
        } else if let Some(value) = line.strip_prefix("BOT_DETAIL=") {
            if let Some(acc) = current.as_mut() {
                acc.detail = value.to_string();
            }
        } else if let Some(value) = line.strip_prefix("RESULT=") {
            overall_result = Some(value);
        } else if let Some(value) = line.strip_prefix("MESSAGE=") {
            overall_message = value.to_string();
        }
    }
    flush_bot(current.take(), &mut per_bot);

    match overall_result {
        Some("OK") => RunnerOutcome::Report(LoadScenarioReport {
            bot_count: per_bot.len() as u32,
            per_bot,
        }),
        Some("ERROR") => RunnerOutcome::ProcessFailure(overall_message),
        _ => {
            let stderr_tail: String = stderr
                .lines()
                .rev()
                .take(20)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            RunnerOutcome::ProcessFailure(format!(
                "load_scenario_runner produced no parseable RESULT= line; stdout: {stdout:?}; stderr (last 20 lines): {stderr_tail}"
            ))
        }
    }
}

/// CLI entry point (`xtask m3-report --server-bin <path> --mode {smoke|full}`).
pub fn run(server_bin: PathBuf, mode: Mode) -> std::process::ExitCode {
    let repo_root = repo_root();
    let run_duration = mode.load_test_duration();

    // Step 1: the redstone corpus legs -- M3-B07's own verbs, reused unmodified
    // (module doc comment). Calls each verb's own `run`, then re-reads its own
    // already-written `target/verify/*.json` output, never re-implementing the
    // comparison (Context).
    let _ = fetch_corpus::run(&fetch_corpus::FetchCorpusArgs {
        version: "26.2".to_string(),
        server_jar: None,
        only: None,
        // Never `true`: TEST-D41 makes EULA acceptance the one step no automated
        // agent may take on a human's behalf, and an aggregating report verb is
        // exactly such an agent. `false` means "rely on consent already given"
        // (`RC_ORACLE_EULA_ACCEPTED` or the `oracle/.eula-accepted` marker), so
        // `fetch_corpus::eula_gate` fails loudly here when it is absent rather
        // than silently writing `eula=true` and launching the oracle.
        accept_eula: false,
    });
    let fetch_corpus_result = read_tier_result(
        &repo_root.join("target/verify/fetch-corpus.json"),
        "fetch-corpus",
    );

    let _ = parity_check::run(&parity_check::ParityCheckRedstoneArgs { only: None });
    let parity_check_result = read_tier_result(
        &repo_root.join("target/verify/parity-check-redstone.json"),
        "parity-check-redstone",
    );

    // Step 2/3/4: the real server + load-test leg.
    let world_dir = TempWorldDir::new("load");
    let tick_log_path = world_dir.path.join("tick-log.ndjson");

    let mut target = String::new();
    let mut region_count_observed: Option<u32> = None;
    let mut tps = TpsReport {
        sample_count: 0,
        duration_secs: 0.0,
        measured_tps: 0.0,
        drift_ratio: -1.0,
        within_tolerance: false,
    };
    let mut bots = LoadScenarioReport::default();

    match spawn_server(ManagedServerConfig {
        binary_path: server_bin,
        offline: true,
        startup_timeout: SERVER_STARTUP_TIMEOUT,
        world_dir: Some(world_dir.path.clone()),
        tick_log: Some(tick_log_path.clone()),
        region_lifecycle: Some("pinned-single".to_string()),
        capture_stdout: true,
        ..Default::default()
    }) {
        Ok(mut managed) => {
            target = managed.addr.to_string();
            region_count_observed = parse_region_count_line(&managed.stdout_snapshot());

            match run_load_scenario_subprocess(
                &repo_root,
                "127.0.0.1",
                managed.addr.port(),
                BOT_LOGIN_TIMEOUT,
                run_duration,
            ) {
                RunnerOutcome::Report(report) => bots = report,
                RunnerOutcome::ProcessFailure(message) => {
                    eprintln!("m3-report: load_scenario_runner: {message}");
                }
            }

            managed.graceful_shutdown(Duration::from_secs(10));
            drop(managed); // guaranteed teardown either way (Drop's own hard-kill fallback)

            match tick_cadence::parse_tick_log(&tick_log_path) {
                Ok(entries) if entries.len() >= 2 => {
                    tps = tick_cadence::analyze_tps(&entries, TARGET_TPS, TPS_TOLERANCE);
                }
                Ok(entries) => {
                    eprintln!(
                        "m3-report: tick log at {} only had {} sample(s), need at least 2",
                        tick_log_path.display(),
                        entries.len()
                    );
                }
                Err(err) => {
                    eprintln!(
                        "m3-report: failed to read tick log at {}: {err}",
                        tick_log_path.display()
                    );
                }
            }
        }
        Err(err) => {
            eprintln!("m3-report: failed to start rusty-clanker-server: {err}");
        }
    }

    let report = build_report(
        mode,
        target,
        &fetch_corpus_result,
        &parity_check_result,
        tps,
        &bots,
        region_count_observed,
    );
    let status = report.automated.status;

    if let Err(err) = write_report(&report) {
        eprintln!("m3-report: failed to write {OUT_PATH}: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(status)
}

fn write_report(report: &M3ReportResult) -> std::io::Result<()> {
    let path = Path::new(OUT_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}
