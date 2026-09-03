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
//!
//! TEST-D58 (`docs/planning/09-testing-quality.md`): the first real scheduled run
//! still spent 2h11m running both captures sequentially into their own deadlines
//! on `windows-2025` and produced no diff and no per-step evidence at all, while the
//! Linux leg hung on an orphaned pipe — the two captures are embarrassingly parallel
//! and the diff itself is milliseconds of pure computation, so CI now runs them as
//! three separate jobs per OS (`.github/workflows/ci.yml`):
//! `protocol-capture-oracle`/`protocol-capture-ours` each drive one side of this same
//! `run` (`--side oracle`/`--side ours`) and upload their own `.postcard` capture
//! plus the timings/report JSON as artifacts; a third, cheap `protocol-diff` job
//! downloads both artifacts and calls this verb again with `--diff-only
//! --oracle-capture <path> --ours-capture <path>` — no subprocess, no Java, no EULA
//! gate, just `read_capture` on both files followed by exactly the same
//! `diff_into_result` fold the in-process `--side both` path already uses (single
//! source of truth for "what counts as a diff", never duplicated). A missing or
//! unreadable capture file becomes one `Fail` case naming the path
//! (`capture-oracle`/`capture-ours`) rather than a panic — the two capture jobs run
//! independently and either one can fail on its own budget before the diff job ever
//! runs, and this job is `if: always()` specifically so it still reports that.
//!
//! Governance fix (M3.5-B03, TEST-D48/TEST-D50): a real run measured `RESULT=OK`
//! plus a readable `.postcard` file on a side that had actually captured 0 of 51
//! redstone-corpus contraptions (`docs/findings-for-planning.md`'s own "stance-
//! walk timeout" finding) — `run`'s own `capture-oracle`/`capture-ours` cases now
//! `Pass` only when `evaluate_completeness` (fed `expected_totals`'s own resolved
//! counts, primarily `protocol_diff_runner`'s own `begin` line) reports every
//! expected session step and every expected contraption `done` and the
//! `finished` line's own `failed=<n>` was `0`; otherwise `Fail`, with
//! `capture_fail_detail`'s own `"captured K of M contraptions and S of T session
//! steps (F failed): <last failure line>"` detail. The oracle cache
//! (`write_oracle_cache`) is seeded only on that same `Pass`, so a cached capture
//! is never itself an incomplete one. `target/verify/protocol-diff-timings.json`
//! carries the same `expected_steps`/`expected_contraptions`/`failed` numbers per
//! side.
//!
//! TEST-D59 (known-divergence register, M3.5-B03 governance changeset): the diff
//! step (`diff_into_result`, both the `run` and `run_diff_only` paths) now resolves
//! every `protocol_capture::diff_captures` mismatch against the committed register
//! at `crates/testing/gametest/corpus/protocol-diff/known-divergences.ron`
//! (`load_and_verify_register`: verifies the register's own TEST-D47 manifest first,
//! then `rc_gametest::known_divergences::load_register`, then the TEST-D59 expiry
//! check against `blueprints/<milestone>/*-COMPLETION-REPORT.md`, each its own
//! `TierResult` case). A step whose every mismatch/missing-packet-type is covered by
//! a register entry now `Pass`es with a `known (...)`-annotated detail
//! (`rc_gametest::known_divergences::resolve_step`); anything left over still fails,
//! with a compact detail (`compact_mismatch_detail`/`compact_missing_detail`: packet
//! type names and counts, and — for a body mismatch — one example body per side
//! truncated to its first 32 bytes in hex plus the full body length,
//! `body_preview`). The first real full diff produced a 135 MB `protocol-diff.json`
//! because raw packet bodies used to be dumped straight into case details —
//! `write_bodies_dump` now writes the complete, untruncated oracle/ours body lists to
//! a separate `target/verify/protocol-diff-bodies.json` instead (not part of
//! `.github/workflows/ci.yml`'s own artifact upload list — a local-debugging-only
//! file), keeping the summary report itself well under 1 MB for the full corpus.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rc_gametest::known_divergences::{self, DivergenceClass, KnownDivergence};
use rc_gametest::protocol_capture::{
    PacketTypeDiff, ProtocolCaptureFile, diff_captures, read_capture,
};

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
    /// Required whenever `side` wants `ours` (`Side::Ours`/`Side::Both`) and
    /// `diff_only` is `false` — `run` itself rejects a genuinely missing value at
    /// that point with its own `resolve-server-bin` `Fail` case, no panic. `None` for
    /// every other invocation shape (`--side oracle`, `--diff-only`), which is
    /// exactly why this is optional at the clap level (TEST-D58 Deliverable 1) rather
    /// than always-required.
    pub server_bin: Option<PathBuf>,
    pub only: Option<String>,
    pub side: Side,
    pub accept_eula: bool,
    pub debug_hooks: bool,
    /// `--capture-deadline-secs <n>`: overrides both sides' own `protocol_diff_runner`
    /// subprocess deadline. `None` keeps today's per-side defaults (module doc
    /// comment).
    pub capture_deadline_secs: Option<u64>,
    /// `--diff-only` (TEST-D58 Deliverable 1): skip capture entirely and diff two
    /// already-captured files straight off disk via `run_diff_only`. Every field
    /// above except `capture_deadline_secs`/`debug_hooks` is ignored when this is
    /// `true` — clap's own `conflicts_with_all`/`required_if_eq` on the CLI struct
    /// (`xtask::lib::Command::ProtocolDiff`) already guarantees `oracle_capture`/
    /// `ours_capture` are both `Some` whenever this is `true`, and that none of
    /// `side`/`server_bin`/`server_jar`/`accept_eula`/`only` were ever passed
    /// alongside it.
    pub diff_only: bool,
    pub oracle_capture: Option<PathBuf>,
    pub ours_capture: Option<PathBuf>,
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
    /// From the `finished` line's own `failed=<n>` field (M3.5-B03 governance
    /// fix, `protocol_diff_runner`'s own module doc comment has the full
    /// "missing contraptions still reported pass" citation) — how many
    /// redstone-corpus contraptions `redstone_wire_capture::run_redstone_wire_
    /// capture` itself reported as failed. `None` exactly when `total_ms` is
    /// `None` too (the side never reached its own `finished` line at all).
    pub failed: Option<u64>,
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
                    } else if let Some(f) = tok.strip_prefix("failed=").and_then(|v| v.parse().ok())
                    {
                        summary.failed = Some(f);
                    }
                }
            }
            _ => {}
        }
    }
    summary
}

/// This file's own restatement of `rc_paritybot::protocol_session::SESSION_STEPS.
/// len()` — this crate never links `rc_paritybot`/`azalea`
/// (`protocol_diff_runner`'s own module doc comment has the full "must never
/// link azalea" citation), so this is a plain, hand-kept integer instead of an
/// import. Every scripted-session step's own real action always runs and always
/// prints its own `done` line regardless of `--only` (`protocol_session::push_
/// step`'s own doc comment: `only` filters only what ends up in the *saved*
/// capture, never whether the step runs), so this fallback is correct regardless
/// of `--only`'s own value. A future change to `SESSION_STEPS` silently drifting
/// from this constant is a known risk (recorded in `docs/findings-for-planning.md`,
/// M3.5-B03) that would only ever weaken the rare "the child's own `begin` line
/// was never captured at all" fallback below, never the common case (which
/// always reads the real count straight from that `begin` line).
const EXPECTED_SESSION_STEPS_FALLBACK: u64 = 32;

/// Real fallback source of truth for "how many contraptions this run should have
/// attempted" when the child's own `begin` line was never captured at all (a
/// crash before any progress line printed) — a live count of the same committed
/// `.ron` corpus directory `protocol_diff_runner::corpus_dir` resolves, counted
/// directly (`std::fs::read_dir`) rather than imported, for the same "never link
/// azalea" reason `EXPECTED_SESSION_STEPS_FALLBACK` cites.
fn corpus_contraption_count(repo_root: &Path) -> u64 {
    let dir = repo_root.join("crates/testing/gametest/corpus/redstone");
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "ron"))
                .count() as u64
        })
        .unwrap_or(0)
}

/// Resolves "how much was this run actually supposed to capture" — primarily the
/// `begin` line's own `steps=`/`contraptions=` counts (already correct for
/// `--only`: `protocol_diff_runner::planned_contraption_count`'s own doc comment).
/// Falls back to `EXPECTED_SESSION_STEPS_FALLBACK`/`corpus_contraption_count`
/// only when the child crashed before ever printing its own `begin` line —
/// `only` then makes the contraption fallback `1` (assuming `--only` named a
/// contraption id; the rare case where it instead names a session step id makes
/// this fallback's own contraption count too high by one, never too low, so it
/// can only ever make an incomplete run *harder* to mistake for complete, never
/// the reverse) or the full corpus size when `--only` was not given at all.
pub fn expected_totals(summary: &ProgressSummary, only: bool, repo_root: &Path) -> (u64, u64) {
    let steps = summary
        .steps_total
        .unwrap_or(EXPECTED_SESSION_STEPS_FALLBACK);
    let contraptions = summary.contraptions_total.unwrap_or_else(|| {
        if only {
            1
        } else {
            corpus_contraption_count(repo_root)
        }
    });
    (steps, contraptions)
}

/// One side's own capture-completeness verdict (M3.5-B03 governance fix,
/// TEST-D48/TEST-D50: "never report success while having verified only part of
/// what it claims") — `steps_done`/`contraptions_done` are counted straight out
/// of `ProgressSummary::completed` by `evaluate_completeness`, split on the `id`
/// prefix every session-step id carries (`"session/"`, `protocol_session::
/// SESSION_STEPS`) versus every contraption id (`"redstone/<category>/<slug>"`,
/// `rc_gametest::spec::ContraptionSpec::id`'s own doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureCompleteness {
    pub steps_done: u64,
    pub steps_expected: u64,
    pub contraptions_done: u64,
    pub contraptions_expected: u64,
    pub failed: u64,
}

impl CaptureCompleteness {
    /// `Pass` only when every expected session step and every expected
    /// contraption printed its own `done` line (`>=`, never `==`: an extra,
    /// duplicated `done` line must never turn an otherwise-complete run into a
    /// reported failure) and not a single contraption failed.
    pub fn is_complete(&self) -> bool {
        self.steps_done >= self.steps_expected
            && self.contraptions_done >= self.contraptions_expected
            && self.failed == 0
    }

    /// The base `"captured K of M contraptions and S of T session steps (F
    /// failed)"` wording (task's own exact format) — `capture_fail_detail` below
    /// appends the trailing `": <last failure line>"` clause.
    pub fn detail(&self) -> String {
        format!(
            "captured {} of {} contraptions and {} of {} session steps ({} failed)",
            self.contraptions_done,
            self.contraptions_expected,
            self.steps_done,
            self.steps_expected,
            self.failed
        )
    }
}

/// Pure fold of one side's own `ProgressSummary` plus its own expected totals
/// (`expected_totals`) into a `CaptureCompleteness` verdict — never touches
/// `stderr` directly (`capture_fail_detail` below is the one place that does).
pub fn evaluate_completeness(
    summary: &ProgressSummary,
    steps_expected: u64,
    contraptions_expected: u64,
) -> CaptureCompleteness {
    let steps_done = summary
        .completed
        .iter()
        .filter(|entry| entry.id.starts_with("session/"))
        .count() as u64;
    let contraptions_done = summary.completed.len() as u64 - steps_done;
    CaptureCompleteness {
        steps_done,
        steps_expected,
        contraptions_done,
        contraptions_expected,
        failed: summary.failed.unwrap_or(0),
    }
}

/// `completeness.detail()` plus `": <line>"` naming the *last* stderr line
/// containing `"failed"` (whichever real failure — a session-step warning, a
/// contraption failure, a reconnect failure — happened most recently), omitted
/// entirely when no such line exists (mirrors `timeout_detail`'s own "last:"
/// clause, simply omitted rather than fabricated, when nothing is there to name).
pub fn capture_fail_detail(completeness: &CaptureCompleteness, stderr: &str) -> String {
    let mut message = completeness.detail();
    if let Some(last) = stderr
        .lines()
        .map(str::trim)
        .rfind(|line| line.contains("failed"))
    {
        message.push_str(": ");
        message.push_str(last);
    }
    message
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
    /// M3.5-B03 governance fix (`CaptureCompleteness`'s own doc comment) —
    /// `expected_totals`'s own resolved `(steps, contraptions)`, `None` only when
    /// `timings_side` was never even called for this side (`Timings::default`'s
    /// own "this side never ran" reading, `TimingsSide`'s own doc comment above).
    expected_steps: Option<u64>,
    expected_contraptions: Option<u64>,
    /// `ProgressSummary::failed`, restated here so the timings JSON carries the
    /// same number `capture-<side>`'s own completeness gate used, without a
    /// reader needing to re-parse stderr itself.
    failed: Option<u64>,
}

/// `target/verify/protocol-diff-timings.json`'s own top-level shape (Deliverable 3).
#[derive(Debug, Clone, Default, serde::Serialize)]
struct Timings {
    oracle: TimingsSide,
    ours: TimingsSide,
}

fn timings_side(
    run: &RunnerRun,
    deadline: Duration,
    summary: &ProgressSummary,
    steps_expected: u64,
    contraptions_expected: u64,
) -> TimingsSide {
    TimingsSide {
        completed: summary.completed.clone(),
        total_ms: summary.total_ms,
        timed_out: run.timed_out,
        deadline_secs: deadline.as_secs(),
        expected_steps: Some(steps_expected),
        expected_contraptions: Some(contraptions_expected),
        failed: Some(summary.failed.unwrap_or(0)),
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
/// [--server-bin <path>] [--only <step>] [--side oracle|ours|both] [--accept-eula]
/// [--debug-hooks] [--capture-deadline-secs <n>]`, or the disjoint `xtask
/// protocol-diff --diff-only --oracle-capture <path> --ours-capture <path>` shape —
/// dispatched to `run_diff_only` before any of the capture-only setup below ever
/// runs). Structurally identical to `placement_diff::run`: EULA gate, zombie-oracle
/// check, jar resolution + sha1-keyed cache, subprocess launch with concurrent pipe
/// drain, `diff_into_result` (this file's own `protocol_capture::diff_captures`
/// wrapper), `TierResult` written to `target/verify/protocol-diff.json`, plus (module
/// doc comment) per-side timings written to
/// `target/verify/protocol-diff-timings.json`. `--capture-deadline-secs` overrides
/// both sides' own subprocess deadline; absent, each side keeps its own existing
/// default (3300s oracle / 3000s ours).
pub fn run(args: &ProtocolDiffArgs) -> std::process::ExitCode {
    if args.diff_only {
        return run_diff_only(args);
    }

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
            let summary = parse_progress_lines(&run.stderr);
            let (steps_expected, contraptions_expected) =
                expected_totals(&summary, args.only.is_some(), &repo_root);
            timings.oracle = timings_side(
                &run,
                deadline,
                &summary,
                steps_expected,
                contraptions_expected,
            );
            match run.outcome {
                RunnerOutcome::Ok => match read_capture(&out_path) {
                    Ok(capture) => {
                        // M3.5-B03 governance fix (TEST-D48/TEST-D50): `RESULT=OK`
                        // plus a readable postcard file is not by itself proof
                        // every expected step/contraption was actually captured —
                        // `capture-oracle` (and the oracle cache this run may seed
                        // for every later `--only`-less run) must never `Pass` on
                        // a partial capture.
                        let completeness =
                            evaluate_completeness(&summary, steps_expected, contraptions_expected);
                        if completeness.is_complete() {
                            result.push("capture-oracle", Status::Pass, None);
                            if args.only.is_none()
                                && let Err(err) =
                                    write_oracle_cache(&repo_root, &source_jar_sha1, &out_path)
                            {
                                eprintln!("protocol-diff: failed to persist oracle cache: {err}");
                            }
                            oracle_capture = Some(capture);
                        } else {
                            let message = capture_fail_detail(&completeness, &run.stderr);
                            eprintln!("protocol-diff: oracle capture incomplete: {message}");
                            result.push("capture-oracle", Status::Fail, Some(message));
                        }
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
        let server_bin = match &args.server_bin {
            Some(server_bin) => absolutize(&repo_root, server_bin),
            None => {
                let message = "--server-bin is required when --side includes \"ours\" \
                    (\"ours\" or \"both\")"
                    .to_string();
                eprintln!("protocol-diff: {message}");
                result.push("resolve-server-bin", Status::Fail, Some(message));
                return finish(&repo_root, result, &timings);
            }
        };
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
        let summary = parse_progress_lines(&run.stderr);
        let (steps_expected, contraptions_expected) =
            expected_totals(&summary, args.only.is_some(), &repo_root);
        timings.ours = timings_side(
            &run,
            deadline,
            &summary,
            steps_expected,
            contraptions_expected,
        );
        match run.outcome {
            RunnerOutcome::Ok => match read_capture(&out_path) {
                Ok(capture) => {
                    // M3.5-B03 governance fix — same completeness gate as the
                    // oracle side above (TEST-D48/TEST-D50).
                    let completeness =
                        evaluate_completeness(&summary, steps_expected, contraptions_expected);
                    if completeness.is_complete() {
                        result.push("capture-ours", Status::Pass, None);
                        ours_capture = Some(capture);
                    } else {
                        let message = capture_fail_detail(&completeness, &run.stderr);
                        eprintln!("protocol-diff: ours capture incomplete: {message}");
                        result.push("capture-ours", Status::Fail, Some(message));
                    }
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
        let register = load_and_verify_register(&repo_root, &mut result);
        diff_into_result(&mut result, oracle, ours, &register, &repo_root);
    }

    kill_leftover_rusty_clanker_server();
    finish(&repo_root, result, &timings)
}

/// TEST-D58 Deliverable 1: runs exactly the diff the `both` path above runs (§3.10's
/// own "the diff is milliseconds" claim depends on this being nothing but that same
/// pure fold) and pushes it into `result` — the "diff two already-read captures into
/// a `TierResult`" step, factored out as its own pure function precisely so it is
/// testable directly against small, hand-built `ProtocolCaptureFile`s with no files
/// on disk at all (`xtask/tests/`'s own `diff_into_result` tests). Shared by `run`'s
/// in-process `--side both` path and `run_diff_only`'s own artifact-diff path below —
/// one source of truth for "what counts as a diff", so the two paths can never
/// silently drift into producing different case names or statuses for the same pair
/// of captures.
pub fn diff_into_result(
    result: &mut TierResult,
    oracle: &ProtocolCaptureFile,
    ours: &ProtocolCaptureFile,
    register: &[KnownDivergence],
    repo_root: &Path,
) {
    let report_by_step = diff_captures(oracle, ours);
    push_diff_cases(result, &report_by_step, register);
    write_bodies_dump(repo_root, &report_by_step);
}

/// TEST-D59: verifies the register's own TEST-D47 manifest (mirrors
/// `parity_check.rs`'s identical "manifest before content" discipline for the
/// redstone corpus — same `fixture_manifest::verify_manifest` machinery, this
/// verb's own `crates/testing/gametest/corpus/protocol-diff/` directory), loads and
/// validates the register (`rc_gametest::known_divergences::load_register`), and
/// checks every `Missing`/`Body` entry's own `expires` milestone against
/// `blueprints/<milestone>/*-COMPLETION-REPORT.md` on disk — pushing its own case(s)
/// into `result` for each concern. Returns the loaded register (empty on any
/// load/validation failure — `resolve_step` then correctly treats every observed
/// divergence as unregistered rather than silently trusting a broken/tampered file,
/// while `register-load`'s own `Fail` case names why).
fn load_and_verify_register(repo_root: &Path, result: &mut TierResult) -> Vec<KnownDivergence> {
    let dir = register_dir(repo_root);
    let manifest_path = dir.join("manifest.json");
    let violations = crate::fixture_manifest::verify_manifest(&manifest_path, &dir);
    if violations.is_empty() {
        result.push("register-manifest", Status::Pass, None);
    } else {
        for violation in &violations {
            result.push(
                format!("register-manifest::{}", violation.path),
                Status::Fail,
                Some(format!("[{}] {}", violation.kind, violation.message)),
            );
        }
    }

    let register_path = dir.join("known-divergences.ron");
    let register = match known_divergences::load_register(&register_path) {
        Ok(entries) => {
            result.push(
                "register-load",
                Status::Pass,
                Some(format!("{} entries", entries.len())),
            );
            entries
        }
        Err(message) => {
            eprintln!("protocol-diff: {message}");
            result.push("register-load", Status::Fail, Some(message));
            Vec::new()
        }
    };

    for entry in known_divergences::expired_entries(&register, |milestone| {
        milestone_has_completion_report(repo_root, milestone)
    }) {
        let class_name = match entry.class {
            DivergenceClass::Missing => "Missing",
            DivergenceClass::Body => "Body",
            DivergenceClass::Timer => "Timer",
        };
        let case_name = format!("register::expired::{}::{}", entry.steps, entry.packet);
        let message = format!(
            "{class_name} entry for (steps={:?}, packet={:?}) closes with {:?} but its own \
             expires milestone {:?} already has a completion report — this known divergence \
             is a regression in disguise and must be removed from the register",
            entry.steps, entry.packet, entry.closes_with, entry.expires
        );
        eprintln!("protocol-diff: {message}");
        result.push(case_name, Status::Fail, Some(message));
    }

    register
}

fn register_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("crates/testing/gametest/corpus/protocol-diff")
}

/// TEST-D59's own expiry predicate, injected into `known_divergences::
/// expired_entries`: `true` iff `blueprints/<milestone>/` contains at least one
/// `*-COMPLETION-REPORT.md` file (the exact naming `blueprints/M3/M3-COMPLETION-
/// REPORT.md` already established) — a milestone directory that doesn't exist at all
/// (not yet started) reports `false`, never an error.
fn milestone_has_completion_report(repo_root: &Path, milestone: &str) -> bool {
    let dir = repo_root.join("blueprints").join(milestone);
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .ends_with("-COMPLETION-REPORT.md")
            })
        })
        .unwrap_or(false)
}

/// TEST-D59 Deliverable 3: the first real full diff produced a 135 MB
/// `protocol-diff.json` because every mismatch's own raw packet bodies were dumped
/// straight into its case detail — this writes the complete, untruncated body lists
/// (every `mismatches` entry of every step, known-covered or not) to a separate
/// `target/verify/protocol-diff-bodies.json` instead, best-effort (a write failure
/// here must never turn an otherwise-successful `protocol-diff` run into a failure —
/// logged and swallowed, exactly like `write_timings`). Never referenced by
/// `.github/workflows/ci.yml`'s own artifact upload list — a local-debugging-only
/// file. Writes nothing (not even an empty file) when there is nothing to dump, so a
/// clean run never leaves a stale dump from an earlier failing one lying around
/// looking current.
fn write_bodies_dump(
    repo_root: &Path,
    report_by_step: &std::collections::BTreeMap<
        String,
        rc_gametest::protocol_capture::ProtocolDiffReport,
    >,
) {
    #[derive(serde::Serialize)]
    struct BodiesDumpEntry<'a> {
        step_id: &'a str,
        packet_id: i32,
        packet_name: Option<&'a str>,
        oracle_only_bodies: &'a [(Vec<u8>, usize)],
        ours_only_bodies: &'a [(Vec<u8>, usize)],
    }

    let mut entries = Vec::new();
    for (step_id, report) in report_by_step {
        for diff in &report.mismatches {
            entries.push(BodiesDumpEntry {
                step_id,
                packet_id: diff.packet_id,
                packet_name: diff.packet_name.as_deref(),
                oracle_only_bodies: &diff.oracle_only_bodies,
                ours_only_bodies: &diff.ours_only_bodies,
            });
        }
    }
    if entries.is_empty() {
        return;
    }

    let dir = repo_root.join("target/verify");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        eprintln!("protocol-diff: failed to create {}: {err}", dir.display());
        return;
    }
    let json = match serde_json::to_string_pretty(&entries) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("protocol-diff: failed to serialize bodies dump: {err}");
            return;
        }
    };
    let path = dir.join("protocol-diff-bodies.json");
    if let Err(err) = std::fs::write(&path, json) {
        eprintln!("protocol-diff: failed to write {}: {err}", path.display());
    }
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

/// `--diff-only` (TEST-D58 Deliverable 1): no subprocesses, no Java, no server, no
/// EULA gate — reads both already-captured `.postcard` files straight off disk
/// (`diff_only_side`, once per side) and folds them through `diff_into_result`, the
/// exact same diff the capture-driving `run` path above uses. Never calls
/// `write_timings` — the timings file is a capture-only artifact, already written by
/// whichever `--side oracle`/`--side ours` run produced the two files this reads, and
/// this path never re-derives it from anything (there is no subprocess stderr to
/// parse here at all).
fn run_diff_only(args: &ProtocolDiffArgs) -> std::process::ExitCode {
    let mut result = TierResult::new("protocol-diff");

    let oracle = diff_only_side(
        &mut result,
        "capture-oracle",
        args.oracle_capture.as_deref(),
    );
    let ours = diff_only_side(&mut result, "capture-ours", args.ours_capture.as_deref());
    if let (Some(oracle), Some(ours)) = (&oracle, &ours) {
        let repo_root = repo_root();
        let register = load_and_verify_register(&repo_root, &mut result);
        diff_into_result(&mut result, oracle, ours, &register, &repo_root);
    }

    let result = result.finalize();
    print_case_table(&result);
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("protocol-diff: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}

/// Reads one `--diff-only` side's own capture file and pushes exactly one
/// `case_name` (`"capture-oracle"`/`"capture-ours"`) case into `result`: `Pass` with
/// detail `"from artifact <path>"` on success, `Fail` naming the path on any read
/// error — `rc_gametest::protocol_capture::CaptureReadError`'s own `Display` already
/// covers both "file not found" and "not a valid capture" without this function ever
/// needing to distinguish them. `path: None` (clap's own `required_if_eq` on
/// `--diff-only` should make this unreachable from the CLI, but this function never
/// trusts that from the inside — no `.expect()`/`.unwrap()` anywhere in this path)
/// produces the same kind of `Fail` case rather than panicking.
fn diff_only_side(
    result: &mut TierResult,
    case_name: &'static str,
    path: Option<&Path>,
) -> Option<ProtocolCaptureFile> {
    let Some(path) = path else {
        let side = case_name.trim_start_matches("capture-");
        let message = format!("--diff-only requires --{side}-capture <path>");
        eprintln!("protocol-diff: {message}");
        result.push(case_name, Status::Fail, Some(message));
        return None;
    };
    match read_capture(path) {
        Ok(capture) => {
            result.push(
                case_name,
                Status::Pass,
                Some(format!("from artifact {}", path.display())),
            );
            Some(capture)
        }
        Err(err) => {
            let message = format!("failed to read {}: {err}", path.display());
            eprintln!("protocol-diff: {message}");
            result.push(case_name, Status::Fail, Some(message));
            None
        }
    }
}

/// One `TierResult` case per step id (§3.9: "one `TierResult` case per step id" —
/// this blueprint's own restatement of `placement_diff.rs::push_diff_cases`'
/// convention for a per-step, not per-scenario, unit) — a pure fold from
/// `protocol_capture::diff_captures`'s own output, kept separate from `run`'s own
/// I/O shell so it can be exercised directly against a small, hand-built fixture
/// (this module's own `tests::tier_result_shape`).
///
/// TEST-D59: every step is first resolved against `register`
/// (`known_divergences::resolve_step`) — a step with nothing left unregistered
/// `Pass`es (detail `None` when the register never mattered at all, matching today's
/// clean-step behavior exactly; a `registered=N; known (...): ...` detail whenever
/// it did); a step with anything left over `Fail`s with a compact detail — packet
/// type names and counts, never a raw byte dump (Deliverable 3, `compact_mismatch_
/// detail`/`compact_missing_detail`) — capped well under the 1 MB summary-report
/// budget for the full corpus.
fn push_diff_cases(
    result: &mut TierResult,
    report_by_step: &std::collections::BTreeMap<
        String,
        rc_gametest::protocol_capture::ProtocolDiffReport,
    >,
    register: &[KnownDivergence],
) {
    for (step_id, report) in report_by_step {
        let verdict = known_divergences::resolve_step(step_id, report, register);

        let known_parts: Vec<String> = verdict.known.iter().map(known_detail).collect();

        if verdict.passes() {
            let detail = if known_parts.is_empty() {
                None
            } else {
                Some(format!(
                    "registered={}; known: {}",
                    known_parts.len(),
                    known_parts.join("; ")
                ))
            };
            result.push(step_id, Status::Pass, detail);
            continue;
        }

        let mut detail_parts = Vec::new();
        if !known_parts.is_empty() {
            detail_parts.push(format!("known: {}", known_parts.join("; ")));
        }
        if !verdict.unregistered_missing_in_oracle.is_empty() {
            detail_parts.push(compact_missing_detail(
                "packet id(s) present only in ours, never observed from the oracle",
                &verdict.unregistered_missing_in_oracle,
                &report.packet_names,
            ));
        }
        if !verdict.unregistered_missing_in_ours.is_empty() {
            detail_parts.push(compact_missing_detail(
                "packet id(s) present only in the oracle capture, never observed from ours",
                &verdict.unregistered_missing_in_ours,
                &report.packet_names,
            ));
        }
        for diff in &verdict.unregistered_mismatches {
            detail_parts.push(compact_mismatch_detail(diff));
        }

        let unregistered_count = verdict.unregistered_missing_in_oracle.len()
            + verdict.unregistered_missing_in_ours.len()
            + verdict.unregistered_mismatches.len();
        result.push(
            step_id,
            Status::Fail,
            Some(format!(
                "registered={} unregistered={}; {}",
                known_parts.len(),
                unregistered_count,
                detail_parts.join("; ")
            )),
        );
    }
}

/// TEST-D59 Deliverable 3: at most the first 32 bytes of `body`, hex-encoded, plus
/// the body's own full length — never the whole body (that goes only into
/// `write_bodies_dump`'s own separate, uncapped file).
fn body_preview(body: &[u8]) -> String {
    const MAX_PREVIEW_BYTES: usize = 32;
    let take = body.len().min(MAX_PREVIEW_BYTES);
    let hex: String = body[..take].iter().map(|b| format!("{b:02x}")).collect();
    format!("len={} first{take}={hex}", body.len())
}

/// One `unregistered_mismatches` entry's own compact detail: the packet type's own
/// name/id and how many distinct normalized bodies differ per side, plus one example
/// body per side (`body_preview`) — never the full `oracle_only_bodies`/
/// `ours_only_bodies` lists themselves.
fn compact_mismatch_detail(diff: &PacketTypeDiff) -> String {
    let name = diff.packet_name.as_deref().unwrap_or("<unresolved>");
    let mut detail = format!(
        "packet id {} ({name}): {} distinct oracle-only bod(ies), {} distinct ours-only bod(ies)",
        diff.packet_id,
        diff.oracle_only_bodies.len(),
        diff.ours_only_bodies.len()
    );
    if let Some((body, excess)) = diff.oracle_only_bodies.first() {
        detail.push_str(&format!(
            "; example oracle-only body (excess count {excess}): {}",
            body_preview(body)
        ));
    }
    if let Some((body, excess)) = diff.ours_only_bodies.first() {
        detail.push_str(&format!(
            "; example ours-only body (excess count {excess}): {}",
            body_preview(body)
        ));
    }
    detail
}

/// One `unregistered_missing_in_oracle`/`unregistered_missing_in_ours` list's own
/// compact detail — packet type names (resolved from `names`, `<unresolved>` when
/// the id never carried one) with their raw ids, never anything about the body (a
/// presence-set entry has no body to compare in the first place).
fn compact_missing_detail(
    label: &str,
    ids: &[i32],
    names: &std::collections::BTreeMap<i32, String>,
) -> String {
    let parts: Vec<String> = ids
        .iter()
        .map(|id| match names.get(id) {
            Some(name) => format!("{name} (id {id})"),
            None => format!("<unresolved> (id {id})"),
        })
        .collect();
    format!("{label}: {}", parts.join(", "))
}

/// One `StepVerdict::known` entry's own compact detail — TEST-D59's own exact wording:
/// `known (<closes_with>, expires <milestone>)` for `Missing`/`Body`, `known
/// (timer-driven)` for `Timer`.
fn known_detail(m: &known_divergences::KnownEntryMatch<'_>) -> String {
    let name = m.packet_name.as_deref().unwrap_or("<unresolved>");
    match m.matched.class {
        DivergenceClass::Timer => format!("{name}: known (timer-driven)"),
        DivergenceClass::Missing | DivergenceClass::Body => format!(
            "{name}: known ({}, expires {})",
            m.matched.closes_with.as_deref().unwrap_or("?"),
            m.matched.expires.as_deref().unwrap_or("?")
        ),
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
                ..Default::default()
            },
        );
        report_by_step.insert(
            "session/move".to_string(),
            ProtocolDiffReport {
                mismatches: vec![],
                missing_in_oracle: vec![],
                missing_in_ours: vec![],
                ..Default::default()
            },
        );

        let mut result = TierResult::new("protocol-diff");
        push_diff_cases(&mut result, &report_by_step, &[]);
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

    fn temp_repo_root(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rc-protocol-diff-register-test-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_register_dir(repo_root: &Path, ron_body: &str) {
        let dir = register_dir(repo_root);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("known-divergences.ron"), ron_body).unwrap();
        let sha256 = crate::fixture_manifest::compute_sha256_hex(ron_body.as_bytes());
        let manifest = format!(
            r#"{{"protocol_version":776,"mc_version":"26.2","entries":[{{"path":"known-divergences.ron","sha256":"{sha256}","generator_tool_version":"test","source_jar_sha1":"n/a"}}]}}"#
        );
        std::fs::write(dir.join("manifest.json"), manifest).unwrap();
    }

    #[test]
    fn milestone_has_completion_report_true_only_when_a_report_file_exists() {
        let repo_root = temp_repo_root("milestone-report");
        assert!(!milestone_has_completion_report(&repo_root, "M4"));

        let dir = repo_root.join("blueprints/M4");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(
            !milestone_has_completion_report(&repo_root, "M4"),
            "an empty milestone directory must not count as complete"
        );

        std::fs::write(dir.join("M4-COMPLETION-REPORT.md"), "done").unwrap();
        assert!(milestone_has_completion_report(&repo_root, "M4"));
    }

    #[test]
    fn load_and_verify_register_pushes_manifest_and_load_cases_for_a_clean_register() {
        let repo_root = temp_repo_root("clean-register");
        write_register_dir(
            &repo_root,
            r#"[
                (steps: "session/*", packet: "minecraft:keep_alive", class: Timer, closes_with: None, expires: None),
            ]"#,
        );

        let mut result = TierResult::new("protocol-diff");
        let register = load_and_verify_register(&repo_root, &mut result);

        assert_eq!(register.len(), 1);
        let manifest_case = result
            .cases
            .iter()
            .find(|c| c.name == "register-manifest")
            .expect("register-manifest case present");
        assert_eq!(manifest_case.status, Status::Pass);
        let load_case = result
            .cases
            .iter()
            .find(|c| c.name == "register-load")
            .expect("register-load case present");
        assert_eq!(load_case.status, Status::Pass);
    }

    #[test]
    fn load_and_verify_register_fails_the_manifest_case_on_a_tampered_file() {
        let repo_root = temp_repo_root("tampered-register");
        write_register_dir(
            &repo_root,
            r#"[
                (steps: "session/*", packet: "minecraft:keep_alive", class: Timer, closes_with: None, expires: None),
            ]"#,
        );
        // Tamper with the register file after the manifest was computed from its
        // original content — the manifest hash no longer matches the file on disk.
        std::fs::write(
            register_dir(&repo_root).join("known-divergences.ron"),
            r#"[
                (steps: "session/*", packet: "minecraft:set_time", class: Timer, closes_with: None, expires: None),
            ]"#,
        )
        .unwrap();

        let mut result = TierResult::new("protocol-diff");
        let _ = load_and_verify_register(&repo_root, &mut result);

        assert!(
            result
                .cases
                .iter()
                .any(|c| c.name.starts_with("register-manifest::") && c.status == Status::Fail),
            "expected a failing register-manifest::<path> case, got: {:?}",
            result.cases.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn load_and_verify_register_fails_on_an_expired_entry() {
        let repo_root = temp_repo_root("expired-register");
        write_register_dir(
            &repo_root,
            r#"[
                (steps: "session/spawn", packet: "minecraft:commands", class: Missing, closes_with: Some("NET hardening"), expires: Some("M4")),
            ]"#,
        );
        let blueprint_dir = repo_root.join("blueprints/M4");
        std::fs::create_dir_all(&blueprint_dir).unwrap();
        std::fs::write(blueprint_dir.join("M4-COMPLETION-REPORT.md"), "done").unwrap();

        let mut result = TierResult::new("protocol-diff");
        let _ = load_and_verify_register(&repo_root, &mut result);

        let expired_case = result
            .cases
            .iter()
            .find(|c| c.name.starts_with("register::expired::"))
            .expect("an expired-entry case must be pushed");
        assert_eq!(expired_case.status, Status::Fail);
        let result = result.finalize();
        assert_eq!(result.status, Status::Fail);
    }

    #[test]
    fn body_preview_caps_at_32_bytes_but_names_the_real_length() {
        let body: Vec<u8> = (0..200u16).map(|n| (n % 256) as u8).collect();
        let preview = body_preview(&body);
        assert!(preview.contains("len=200"));
        // "first32=" plus exactly 32 bytes of hex (64 chars) is the whole tail.
        let hex_part = preview.split('=').next_back().unwrap();
        assert_eq!(hex_part.len(), 64);
    }
}
