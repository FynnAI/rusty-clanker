//! `xtask protocol-diff` (M3.5-B03, governance changeset "protocol-differential
//! harness against the vanilla oracle"): TEST-D54's own scripted-bot-session-plus-
//! redstone-corpus-over-the-wire byte-level differential harness. Follows the exact
//! architectural split `xtask placement-diff` already established
//! (`placement_diff.rs`'s own module doc comment): every real bot action lives one
//! process-hop away, in `rc-paritybot`'s `protocol_diff_runner` bin target (this
//! crate must never link `azalea`), while this file owns resolution (jar/server
//! binary), the EULA gate, the zombie-oracle check, subprocess orchestration, the
//! oracle-side capture cache, the diff itself
//! (`rc_gametest::protocol_capture::diff_captures`), and the final `TierResult`
//! report. Never runs in Tier 1 — a real oracle process (network or a locally-cached
//! jar), Java, and our own freshly built release binary, exactly like
//! `fetch-corpus`/`parity-check`/`placement-diff`.
//!
//! Governance fix (`docs/findings-for-planning.md`): the first real scheduled CI run
//! (`windows-2025`) hit both sides' own fixed subprocess deadline and produced
//! nothing usable, because `protocol_diff_runner` printed no per-step progress and a
//! timed-out `spawn_drained` call used to discard whatever it had captured so far.
//! Two things now fix that: `protocol_diff_runner`'s own `begin`/`done`/`finished`
//! stderr progress lines (its own module doc comment has the exact format), parsed
//! here by `parse_progress_lines` from each side's captured stderr (success or
//! timeout, `crate::process::SpawnDrainedError::TimedOut` now carries whatever it
//! captured) into `target/verify/protocol-diff-timings.json` and into a timed-out
//! side's own `TierResult` case detail; and a `--capture-deadline-secs <n>` flag
//! overriding both sides' own subprocess deadline (default, when absent: unchanged —
//! 3300s oracle / 3000s ours), which CI now passes as `5400` (the job may run up to
//! ~3.5h; GitHub's own job limit is 6h).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rc_gametest::protocol_capture::{ProtocolCaptureFile, diff_captures, read_capture};

use crate::corpus::placement_diff::Side;
use crate::tier_result::{Status, TierResult};

/// Distinct from `fetch-corpus`'s own `25566`/`placement-diff`'s own `25567`
/// (`protocol_diff_runner`'s own module doc comment has the identical citation for
/// why this never needs to actually agree with either, only to be internally
/// consistent with the runner it launches).
const ORACLE_PORT: u16 = 25568;

pub struct ProtocolDiffArgs {
    pub version: String,
    pub server_jar: Option<PathBuf>,
    pub server_bin: PathBuf,
    pub only: Option<String>,
    pub side: Side,
    pub accept_eula: bool,
    pub debug_hooks: bool,
    /// `--capture-deadline-secs <n>`: overrides both sides' own `protocol_diff_runner`
    /// subprocess deadline. `None` keeps today's per-side defaults (module doc
    /// comment).
    pub capture_deadline_secs: Option<u64>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

/// As `placement_diff.rs::absolutize` — every path this file ever hands to the
/// runner subprocess is passed through this first, since `protocol_diff_runner` is
/// launched with `current_dir` set to `crates/testing/paritybot` (below), and a
/// relative path would otherwise resolve against that different directory once it
/// crosses the subprocess boundary.
fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, uniquely named temp directory, removed best-effort on `Drop` — mirrors
/// `placement_diff.rs::TempWorldDir`'s own identical convention.
struct TempWorldDir {
    path: PathBuf,
}
impl TempWorldDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-protocol-diff-{label}-{}-{}",
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

/// As `placement_diff.rs::zombie_oracle_check`, restated for this verb's own
/// distinct `ORACLE_PORT`.
fn zombie_oracle_check(port: u16) -> Result<(), String> {
    if std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(300),
    )
    .is_err()
    {
        return Ok(());
    }

    eprintln!(
        "protocol-diff: port {port} is already bound — attempting to clear a zombie oracle process before proceeding"
    );
    let script = format!(
        "Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction SilentlyContinue | \
         Select-Object -ExpandProperty OwningProcess -Unique | \
         ForEach-Object {{ Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }}"
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    std::thread::sleep(Duration::from_millis(500));

    if std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(300),
    )
    .is_ok()
    {
        return Err(format!(
            "port {port} is still bound after attempting to kill its owning process — a zombie \
             oracle needs manual cleanup (Get-Process java; Stop-Process) before this run can \
             proceed safely"
        ));
    }
    eprintln!("protocol-diff: port {port} is clear");
    Ok(())
}

/// As `placement_diff.rs::kill_leftover_rusty_clanker_server`, restated (best-effort,
/// run unconditionally on the way out of `run`).
fn kill_leftover_rusty_clanker_server() {
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Process rusty-clanker-server -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// One completed step or contraption, parsed from one of `protocol_diff_runner`'s own
/// `protocol-diff-runner: done <id> in <ms> ms` stderr lines — shared between
/// `ProgressSummary` (the pure parse) and `TimingsSide` (the JSON this file writes),
/// since both need exactly the same `{id, ms}` shape.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CompletedEntry {
    pub id: String,
    pub ms: u64,
}

/// `parse_progress_lines`'s own return value: everything this file can recover from
/// one side's own captured stderr about how far `protocol_diff_runner` got.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgressSummary {
    /// From the `begin` line's own `steps=<n>` field, when present.
    pub steps_total: Option<u64>,
    /// From the `begin` line's own `contraptions=<m>` field, when present.
    pub contraptions_total: Option<u64>,
    /// Every `done` line, in the order it was printed.
    pub completed: Vec<CompletedEntry>,
    /// From the `finished` line's own `total_ms=<ms>` field, when present (i.e. the
    /// side actually completed — never set on a timed-out or hard-failed run).
    pub total_ms: Option<u64>,
}

/// The stable, parseable prefix `protocol_diff_runner`'s own three progress-line
/// shapes (its own module doc comment has the exact format) all share.
const PROGRESS_LINE_PREFIX: &str = "protocol-diff-runner: ";

/// Pure parse of `protocol_diff_runner`'s own `begin`/`done`/`finished` stderr lines
/// (Deliverable 1's own exact format) out of `stderr` — every other line (build
/// output, `protocol_session`/`redstone_wire_capture`'s own diagnostic `eprintln!`s,
/// cargo warnings, ...) is silently skipped, and a progress line that does not match
/// the expected field shape is skipped rather than panicking (a future format change
/// degrades this to "less timing detail", never a crash). Deliberately never sees a
/// live process — `xtask::corpus::protocol_diff::run` only ever calls this against
/// stderr `crate::process::spawn_drained` already fully captured (success or
/// timeout), exactly like every other post-hoc parse in this file.
pub fn parse_progress_lines(stderr: &str) -> ProgressSummary {
    let mut summary = ProgressSummary::default();
    for line in stderr.lines() {
        let Some(rest) = line.trim().strip_prefix(PROGRESS_LINE_PREFIX) else {
            continue;
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        match tokens.first().copied() {
            Some("begin") => {
                for tok in tokens.iter().skip(2) {
                    if let Some(n) = tok.strip_prefix("steps=").and_then(|v| v.parse().ok()) {
                        summary.steps_total = Some(n);
                    } else if let Some(m) = tok
                        .strip_prefix("contraptions=")
                        .and_then(|v| v.parse().ok())
                    {
                        summary.contraptions_total = Some(m);
                    }
                }
            }
            Some("done") => {
                if tokens.len() >= 5
                    && tokens[2] == "in"
                    && tokens[4] == "ms"
                    && let Ok(ms) = tokens[3].parse::<u64>()
                {
                    summary.completed.push(CompletedEntry {
                        id: tokens[1].to_string(),
                        ms,
                    });
                }
            }
            Some("finished") => {
                for tok in tokens.iter().skip(2) {
                    if let Some(ms) = tok.strip_prefix("total_ms=").and_then(|v| v.parse().ok()) {
                        summary.total_ms = Some(ms);
                    }
                }
            }
            _ => {}
        }
    }
    summary
}

/// Builds the timeout case-detail wording every side's own subprocess timeout uses
/// (module doc comment's own governance-fix citation): the existing fixed wording
/// (`"<runner> did not exit within Ns of its own start"`) plus how far the runner
/// actually got, parsed straight from its own captured stderr (`parse_progress_lines`)
/// — the child is already dead by the time this runs, so this is the only source of
/// truth left. `M`/`K` fall back to `0` when the `begin` line itself was never even
/// captured (the child died before printing anything) — the "last:" clause is simply
/// omitted in that case rather than fabricating an id.
fn timeout_detail(runner_name: &str, deadline: Duration, stderr: &str) -> String {
    let summary = parse_progress_lines(stderr);
    let total = summary.steps_total.unwrap_or(0) + summary.contraptions_total.unwrap_or(0);
    let mut message = format!(
        "{runner_name} did not exit within {}s of its own start — completed {} of {total} steps/contraptions",
        deadline.as_secs(),
        summary.completed.len()
    );
    if let Some(last) = summary.completed.last() {
        message.push_str(&format!(", last: {} ({} ms)", last.id, last.ms));
    }
    message
}

enum RunnerOutcome {
    Ok,
    Failure(String),
}

/// One `run_protocol_diff_runner_subprocess` call's own full result: the subprocess'
/// own captured stderr (needed afterward for both the timings JSON and, on a
/// timeout, the case-detail message), whether it timed out, and the resolved
/// `RunnerOutcome`.
struct RunnerRun {
    stderr: String,
    timed_out: bool,
    outcome: RunnerOutcome,
}

/// Runs `protocol_diff_runner` as a subprocess with `args`, draining stdout/stderr
/// concurrently with the poll loop via the shared `crate::process::spawn_drained`
/// (M3.5-B06 — that module's own doc comment has the full pipe-buffer-deadlock
/// diagnosis this file's own hand-rolled drain threads used to duplicate) — never
/// read only after the child is observed to have exited, which is exactly the
/// deadlock that diagnosis documents. A timeout no longer discards whatever the
/// child had already printed (module doc comment's own governance-fix citation) —
/// `RunnerRun::stderr` carries it either way.
fn run_protocol_diff_runner_subprocess(
    repo_root: &Path,
    args: &[String],
    deadline: Duration,
) -> RunnerRun {
    let paritybot_dir = repo_root.join("crates/testing/paritybot");

    let mut command = Command::new("cargo");
    command
        .current_dir(&paritybot_dir)
        .env("RUSTC_BOOTSTRAP", "1")
        .env_remove("RUSTUP_TOOLCHAIN")
        .arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("protocol_diff_runner")
        .arg("--")
        .args(args);

    let (stdout, stderr) = match crate::process::spawn_drained(&mut command, deadline) {
        Ok(output) => (output.stdout, output.stderr),
        Err(crate::process::SpawnDrainedError::SpawnFailed(err)) => {
            return RunnerRun {
                stderr: String::new(),
                timed_out: false,
                outcome: RunnerOutcome::Failure(format!(
                    "failed to spawn protocol_diff_runner: {err}"
                )),
            };
        }
        Err(crate::process::SpawnDrainedError::PollFailed(err)) => {
            return RunnerRun {
                stderr: String::new(),
                timed_out: false,
                outcome: RunnerOutcome::Failure(format!(
                    "failed to poll protocol_diff_runner: {err}"
                )),
            };
        }
        Err(crate::process::SpawnDrainedError::TimedOut(captured)) => {
            let detail = timeout_detail("protocol_diff_runner", deadline, &captured.stderr);
            return RunnerRun {
                stderr: captured.stderr,
                timed_out: true,
                outcome: RunnerOutcome::Failure(detail),
            };
        }
    };

    if stdout.lines().any(|line| line == "RESULT=OK") {
        return RunnerRun {
            stderr,
            timed_out: false,
            outcome: RunnerOutcome::Ok,
        };
    }
    let message = stdout
        .lines()
        .find_map(|line| line.strip_prefix("MESSAGE="))
        .map(str::to_string)
        .unwrap_or_else(|| {
            let stderr_tail: String = stderr
                .lines()
                .rev()
                .take(20)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "protocol_diff_runner produced no RESULT=OK line; stdout: {stdout:?}; stderr (last 20 lines): {stderr_tail}"
            )
        });
    RunnerRun {
        stderr,
        timed_out: false,
        outcome: RunnerOutcome::Failure(message),
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The git-ignored oracle capture cache, mirroring `placement_diff.rs::
/// oracle_cache_*` exactly, in this verb's own directory.
fn oracle_cache_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("corpus/protocol-diff")
}
fn oracle_cache_capture_path(repo_root: &Path) -> PathBuf {
    oracle_cache_dir(repo_root).join("oracle.postcard")
}
fn oracle_cache_sha1_path(repo_root: &Path) -> PathBuf {
    oracle_cache_dir(repo_root).join("oracle.sha1")
}

fn read_oracle_cache_if_current(
    repo_root: &Path,
    source_jar_sha1: &str,
) -> Option<ProtocolCaptureFile> {
    let recorded_sha1 = std::fs::read_to_string(oracle_cache_sha1_path(repo_root)).ok()?;
    if recorded_sha1.trim() != source_jar_sha1 {
        return None;
    }
    read_capture(&oracle_cache_capture_path(repo_root)).ok()
}

fn write_oracle_cache(
    repo_root: &Path,
    source_jar_sha1: &str,
    capture_path: &Path,
) -> std::io::Result<()> {
    let dir = oracle_cache_dir(repo_root);
    std::fs::create_dir_all(&dir)?;
    std::fs::copy(capture_path, oracle_cache_capture_path(repo_root))?;
    std::fs::write(oracle_cache_sha1_path(repo_root), source_jar_sha1)
}

/// One side's own row in `target/verify/protocol-diff-timings.json` — `completed`
/// straight from `ProgressSummary::completed`, `total_ms` from its own `finished`
/// line (`None` when the side never completed), `timed_out` from whether
/// `crate::process::spawn_drained` itself returned `TimedOut` for this side's own
/// subprocess call, and `deadline_secs` the deadline that call actually used.
/// Defaults to "this side never ran" (`--side oracle`/`--side ours` leaves the other
/// side's own row at these defaults, and so does an early exit before either side's
/// subprocess is ever launched — consent/zombie-check/jar-resolution failures).
#[derive(Debug, Clone, Default, serde::Serialize)]
struct TimingsSide {
    completed: Vec<CompletedEntry>,
    total_ms: Option<u64>,
    timed_out: bool,
    deadline_secs: u64,
}

/// `target/verify/protocol-diff-timings.json`'s own top-level shape (Deliverable 3).
#[derive(Debug, Clone, Default, serde::Serialize)]
struct Timings {
    oracle: TimingsSide,
    ours: TimingsSide,
}

fn timings_side(run: &RunnerRun, deadline: Duration) -> TimingsSide {
    let summary = parse_progress_lines(&run.stderr);
    TimingsSide {
        completed: summary.completed,
        total_ms: summary.total_ms,
        timed_out: run.timed_out,
        deadline_secs: deadline.as_secs(),
    }
}

/// Writes `timings` as pretty JSON to `target/verify/protocol-diff-timings.json`,
/// best-effort (a write failure here must never turn an otherwise-successful
/// `protocol-diff` run into a failure — logged and swallowed, exactly like this
/// file's own `write_oracle_cache` failure handling above).
fn write_timings(repo_root: &Path, timings: &Timings) {
    let dir = repo_root.join("target/verify");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        eprintln!("protocol-diff: failed to create {}: {err}", dir.display());
        return;
    }
    let json = match serde_json::to_string_pretty(timings) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("protocol-diff: failed to serialize timings: {err}");
            return;
        }
    };
    let path = dir.join("protocol-diff-timings.json");
    if let Err(err) = std::fs::write(&path, json) {
        eprintln!("protocol-diff: failed to write {}: {err}", path.display());
    }
}

/// I/O wrapper (`xtask protocol-diff [--version 26.2] [--server-jar <path>]
/// --server-bin <path> [--only <step>] [--side oracle|ours|both] [--accept-eula]
/// [--debug-hooks] [--capture-deadline-secs <n>]`). Structurally identical to
/// `placement_diff::run`: EULA gate, zombie-oracle check, jar resolution +
/// sha1-keyed cache, subprocess launch with concurrent pipe drain, `protocol_capture::
/// diff_captures`, `TierResult` written to `target/verify/protocol-diff.json`, plus
/// (module doc comment) per-side timings written to
/// `target/verify/protocol-diff-timings.json`. `--capture-deadline-secs` overrides
/// both sides' own subprocess deadline; absent, each side keeps its own existing
/// default (3300s oracle / 3000s ours).
pub fn run(args: &ProtocolDiffArgs) -> std::process::ExitCode {
    let repo_root = repo_root();
    let mut result = TierResult::new("protocol-diff");
    let mut timings = Timings::default();
    let capture_deadline_override = args.capture_deadline_secs.map(Duration::from_secs);

    if args.side.wants_oracle()
        && let Err(message) = crate::corpus::fetch_corpus::eula_gate(&repo_root, args.accept_eula)
    {
        eprintln!("protocol-diff: {message}");
        result.push("consent", Status::Fail, Some(message));
        return finish(&repo_root, result, &timings);
    }
    if args.side.wants_oracle() {
        result.push("consent", Status::Pass, None);
        if let Err(message) = zombie_oracle_check(ORACLE_PORT) {
            eprintln!("protocol-diff: {message}");
            result.push("zombie-oracle-check", Status::Fail, Some(message));
            return finish(&repo_root, result, &timings);
        }
        result.push("zombie-oracle-check", Status::Pass, None);
    }

    let mut oracle_capture: Option<ProtocolCaptureFile> = None;
    let mut ours_capture: Option<ProtocolCaptureFile> = None;

    if args.side.wants_oracle() {
        let jar = if let Some(server_jar) = &args.server_jar {
            let server_jar = absolutize(&repo_root, server_jar);
            match std::fs::read(&server_jar) {
                Ok(bytes) => (server_jar.clone(), sha1_hex(&bytes)),
                Err(err) => {
                    let message = format!("failed to read {}: {err}", server_jar.display());
                    eprintln!("protocol-diff: {message}");
                    result.push("resolve-jar", Status::Fail, Some(message));
                    return finish(&repo_root, result, &timings);
                }
            }
        } else {
            match crate::fetch_data::fetch_server_jar(&args.version, &repo_root) {
                Ok(jar) => (jar.jar_path, jar.sha1),
                Err(err) => {
                    eprintln!("protocol-diff: {err}");
                    result.push("resolve-jar", Status::Fail, Some(err.to_string()));
                    return finish(&repo_root, result, &timings);
                }
            }
        };
        let (jar_path, source_jar_sha1) = jar;
        result.push(
            "resolve-jar",
            Status::Pass,
            Some(format!("sha1 {source_jar_sha1}")),
        );

        let cached = args
            .only
            .is_none()
            .then(|| read_oracle_cache_if_current(&repo_root, &source_jar_sha1))
            .flatten();

        if let Some(cached) = cached {
            eprintln!(
                "protocol-diff: reusing cached oracle capture (sha1 {source_jar_sha1} matches)"
            );
            result.push(
                "capture-oracle",
                Status::Pass,
                Some("cache hit".to_string()),
            );
            oracle_capture = Some(cached);
        } else {
            let work_dir = repo_root
                .join("target/protocol-diff-oracle")
                .join(&args.version);
            let out_path = repo_root.join("target/verify/protocol-diff-oracle.postcard");
            let mut runner_args = vec![
                "oracle".to_string(),
                jar_path.display().to_string(),
                work_dir.display().to_string(),
                out_path.display().to_string(),
                source_jar_sha1.clone(),
            ];
            if args.debug_hooks {
                runner_args.push("--debug-hooks".to_string());
            }
            if let Some(only) = &args.only {
                runner_args.push(only.clone());
            }
            // The scripted session's own genuinely survival-timed dig step
            // (`SURVIVAL_DIG_HOLD`, ~9s) plus 51 real redstone-corpus placements
            // (each waiting up to its own `spec.max_ticks * 50ms`, MAX_TICKS capped
            // at 200 -> up to 10s) dominate this budget far more than `placement-
            // diff`'s own scenario count ever did. Real-run finding (`docs/findings-
            // for-planning.md`): a genuinely first, uncached local run also pays this
            // subprocess's own `cargo run` compile time out of the same deadline
            // (a real, contended-machine, first-run compile measured well past 10
            // minutes) — generous enough to absorb that too, not just the scripted
            // session/corpus wall-clock itself. `--capture-deadline-secs` overrides
            // this default outright (module doc comment's own governance-fix
            // citation).
            let deadline = capture_deadline_override
                .unwrap_or(Duration::from_secs(900) + Duration::from_secs(30) * 80);
            let run = run_protocol_diff_runner_subprocess(&repo_root, &runner_args, deadline);
            timings.oracle = timings_side(&run, deadline);
            match run.outcome {
                RunnerOutcome::Ok => match read_capture(&out_path) {
                    Ok(capture) => {
                        result.push("capture-oracle", Status::Pass, None);
                        if args.only.is_none()
                            && let Err(err) =
                                write_oracle_cache(&repo_root, &source_jar_sha1, &out_path)
                        {
                            eprintln!("protocol-diff: failed to persist oracle cache: {err}");
                        }
                        oracle_capture = Some(capture);
                    }
                    Err(err) => {
                        let message = format!("failed to read {}: {err}", out_path.display());
                        result.push("capture-oracle", Status::Fail, Some(message));
                    }
                },
                RunnerOutcome::Failure(message) => {
                    eprintln!("protocol-diff: oracle capture: {message}");
                    result.push("capture-oracle", Status::Fail, Some(message));
                }
            }
        }
    }

    if args.side.wants_ours() {
        let server_bin = absolutize(&repo_root, &args.server_bin);
        let world = TempWorldDir::new("ours");
        let out_path = repo_root.join("target/verify/protocol-diff-ours.postcard");
        let mut runner_args = vec![
            "ours".to_string(),
            server_bin.display().to_string(),
            world.path.display().to_string(),
            out_path.display().to_string(),
        ];
        if args.debug_hooks {
            runner_args.push("--debug-hooks".to_string());
        }
        if let Some(only) = &args.only {
            runner_args.push(only.clone());
        }
        // As the oracle side's own identical deadline above — real-run finding,
        // `docs/findings-for-planning.md`. `--capture-deadline-secs` overrides this
        // default outright, same as the oracle side.
        let deadline = capture_deadline_override
            .unwrap_or(Duration::from_secs(600) + Duration::from_secs(30) * 80);
        let run = run_protocol_diff_runner_subprocess(&repo_root, &runner_args, deadline);
        timings.ours = timings_side(&run, deadline);
        match run.outcome {
            RunnerOutcome::Ok => match read_capture(&out_path) {
                Ok(capture) => {
                    result.push("capture-ours", Status::Pass, None);
                    ours_capture = Some(capture);
                }
                Err(err) => {
                    let message = format!("failed to read {}: {err}", out_path.display());
                    result.push("capture-ours", Status::Fail, Some(message));
                }
            },
            RunnerOutcome::Failure(message) => {
                eprintln!("protocol-diff: our capture: {message}");
                result.push("capture-ours", Status::Fail, Some(message));
            }
        }
    }

    if let (Some(oracle), Some(ours)) = (&oracle_capture, &ours_capture) {
        let report_by_step = diff_captures(oracle, ours);
        push_diff_cases(&mut result, &report_by_step);
    }

    kill_leftover_rusty_clanker_server();
    finish(&repo_root, result, &timings)
}

fn finish(repo_root: &Path, result: TierResult, timings: &Timings) -> std::process::ExitCode {
    let result = result.finalize();
    print_case_table(&result);
    write_timings(repo_root, timings);
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("protocol-diff: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}

/// One `TierResult` case per step id (§3.9: "one `TierResult` case per step id" —
/// this blueprint's own restatement of `placement_diff.rs::push_diff_cases`'
/// convention for a per-step, not per-scenario, unit) — a pure fold from
/// `protocol_capture::diff_captures`'s own output, kept separate from `run`'s own
/// I/O shell so it can be exercised directly against a small, hand-built fixture
/// (this module's own `tests::tier_result_shape`).
fn push_diff_cases(
    result: &mut TierResult,
    report_by_step: &std::collections::BTreeMap<
        String,
        rc_gametest::protocol_capture::ProtocolDiffReport,
    >,
) {
    for (step_id, report) in report_by_step {
        if report.mismatches.is_empty()
            && report.missing_in_oracle.is_empty()
            && report.missing_in_ours.is_empty()
        {
            result.push(step_id, Status::Pass, None);
            continue;
        }

        let mut detail_parts = Vec::new();
        if !report.missing_in_oracle.is_empty() {
            detail_parts.push(format!(
                "packet id(s) present only in ours, never observed from the oracle: {:?}",
                report.missing_in_oracle
            ));
        }
        if !report.missing_in_ours.is_empty() {
            detail_parts.push(format!(
                "packet id(s) present only in the oracle capture, never observed from ours: {:?}",
                report.missing_in_ours
            ));
        }
        for diff in &report.mismatches {
            let name = diff.packet_name.as_deref().unwrap_or("<unresolved>");
            detail_parts.push(format!(
                "packet id {} ({name}): oracle-only bodies {:?}; ours-only bodies {:?}",
                diff.packet_id, diff.oracle_only_bodies, diff.ours_only_bodies
            ));
        }
        result.push(step_id, Status::Fail, Some(detail_parts.join("; ")));
    }
}

fn print_case_table(result: &TierResult) {
    println!(
        "protocol-diff — {} case(s), overall {:?}",
        result.cases.len(),
        result.status
    );
    for case in &result.cases {
        let mark = match case.status {
            Status::Pass => "PASS",
            Status::Fail => "FAIL",
        };
        match &case.detail {
            Some(detail) => println!("  [{mark}] {} — {detail}", case.name),
            None => println!("  [{mark}] {}", case.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_gametest::protocol_capture::{PacketTypeDiff, ProtocolDiffReport};

    #[test]
    fn tier_result_shape() {
        let mut report_by_step = std::collections::BTreeMap::new();
        report_by_step.insert(
            "session/spawn".to_string(),
            ProtocolDiffReport {
                mismatches: vec![PacketTypeDiff {
                    packet_id: 9,
                    packet_name: Some("block_update".to_string()),
                    oracle_only_bodies: vec![(vec![1, 2, 3], 1)],
                    ours_only_bodies: vec![(vec![1, 2, 4], 1)],
                }],
                missing_in_oracle: vec![77],
                missing_in_ours: vec![],
            },
        );
        report_by_step.insert(
            "session/move".to_string(),
            ProtocolDiffReport {
                mismatches: vec![],
                missing_in_oracle: vec![],
                missing_in_ours: vec![],
            },
        );

        let mut result = TierResult::new("protocol-diff");
        push_diff_cases(&mut result, &report_by_step);
        let result = result.finalize();

        assert_eq!(result.cases.len(), 2);
        let spawn_case = result
            .cases
            .iter()
            .find(|c| c.name == "session/spawn")
            .expect("session/spawn case present");
        assert_eq!(spawn_case.status, Status::Fail);
        assert!(spawn_case.detail.is_some());
        let move_case = result
            .cases
            .iter()
            .find(|c| c.name == "session/move")
            .expect("session/move case present");
        assert_eq!(move_case.status, Status::Pass);
        assert!(move_case.detail.is_none());
        assert_eq!(result.status, Status::Fail);
    }
}
