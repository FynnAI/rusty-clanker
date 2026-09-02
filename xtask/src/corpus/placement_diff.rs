//! `xtask placement-diff` (governance changeset, "M3 field-report harness: a placement
//! differential harness"): the test that would have caught the M3 owner findings the
//! redstone parity corpus structurally cannot — that corpus declares oracle-pre-
//! resolved block-state ids and drives the Stage-4 engine directly (`parity_check.rs`'s
//! own module doc comment); it never exercises the real client -> server placement/
//! break path at all. Follows the exact architectural split `fetch-corpus`/
//! `parity-check redstone` already established (`fetch_corpus.rs`'s own module doc
//! comment): every real bot action lives one process-hop away, in `rc-paritybot`'s
//! `placement_diff_runner` bin target (this crate must never link `azalea`), while
//! this file owns resolution (jar/server binary), the EULA gate, the zombie-oracle
//! check, subprocess orchestration, the oracle-side capture cache, the diff itself
//! (`rc_gametest::placement_trace::diff_captures`), human-readable decoding
//! (`blocks_report::BlocksIndex`), and the final `TierResult` report. Never runs in
//! Tier 1 (same "CI tier placement" rule as `fetch-corpus`/`parity-check` — a real
//! oracle process, network or a locally-cached jar, and Java, plus now also our own
//! freshly built release binary).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rc_gametest::placement_spec::{InteractionScenario, enumerate_scenarios};
use rc_gametest::placement_trace::{PlacementCaptureFile, diff_captures, read_capture};

use crate::corpus::blocks_report::BlocksIndex;
use crate::tier_result::{Status, TierResult};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    Oracle,
    Ours,
    Both,
}

impl Side {
    pub fn parse(value: &str) -> Result<Side, String> {
        match value {
            "oracle" => Ok(Side::Oracle),
            "ours" => Ok(Side::Ours),
            "both" => Ok(Side::Both),
            other => Err(format!(
                "unknown --side {other:?} — expected \"oracle\", \"ours\", or \"both\""
            )),
        }
    }

    // `pub(crate)` (M3.5-B03): `xtask::corpus::protocol_diff::run` reuses this same
    // `Side` type unmodified (never redefined) rather than duplicating its own
    // oracle/ours dispatch logic — these two were private only because no second
    // caller existed until now.
    pub(crate) fn wants_oracle(self) -> bool {
        matches!(self, Side::Oracle | Side::Both)
    }
    pub(crate) fn wants_ours(self) -> bool {
        matches!(self, Side::Ours | Side::Both)
    }
}

pub struct PlacementDiffArgs {
    pub version: String,
    pub server_jar: Option<PathBuf>,
    pub server_bin: PathBuf,
    pub only: Option<String>,
    pub side: Side,
    pub accept_eula: bool,
}

/// Distinct from `fetch-corpus`'s own `25566` — `placement_diff_runner`'s own module
/// doc comment has the identical citation for why this never needs to actually agree
/// with that port, only to be internally consistent with the runner it launches.
const ORACLE_PORT: u16 = 25567;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

/// Governance fix: `run_placement_diff_runner_subprocess` always launches
/// `placement_diff_runner` with `current_dir` set to `crates/testing/paritybot`
/// (mirroring `fetch_corpus.rs`'s own identical subprocess-hop pattern) — a
/// `--server-bin`/`--server-jar` value the caller gave as a path relative to *this*
/// process's own cwd (the repo root, when `xtask` is invoked the normal way) would
/// silently resolve against that *different* directory instead once it crosses the
/// subprocess boundary, producing an opaque "path not found" failure discovered live
/// against a real run rather than a clean, absolute path failing the same way
/// regardless of which process interprets it. Every path this file ever hands to the
/// runner subprocess is passed through this first — relative to `repo_root` for a
/// relative input (matching every other verb's own convention of resolving a bare
/// relative CLI path against the repo root), passed through unchanged for an
/// already-absolute one.
fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, uniquely named temp directory, removed best-effort on `Drop` — mirrors
/// `m2_report.rs`'s own identical `TempWorldDir` convention, restated locally here for
/// the identical reason that module's own doc comment gives (`xtask` has no test-only
/// dependency section this could otherwise live in).
struct TempWorldDir {
    path: PathBuf,
}
impl TempWorldDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-placement-diff-{label}-{}-{}",
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

/// Non-negotiable (Context, "Zombie-oracle check before any capture ... documented
/// failure mode"): a prior crashed/interrupted run's own oracle `java` process can be
/// left bound to `ORACLE_PORT`, which `launch_oracle_server`'s own bare TCP-connect
/// readiness poll (`crates/testing/gametest/src/capture.rs`) would then happily accept
/// as "the new instance is up" — actually still talking to the stale zombie the
/// whole capture run through, silently invalidating every result. Checked and cleared
/// here, before any jar is even resolved, using PowerShell (`Get-NetTCPConnection`/
/// `Stop-Process`, this project's own Windows-only development machine, `CLAUDE.md`'s
/// own env) rather than a second hand-rolled port-ownership primitive. Best-effort:
/// a failure to *query* is silently treated as "nothing to clean up" (this machine's
/// own PowerShell is not itself part of this harness's own correctness contract), but
/// a port that is STILL bound after a genuine kill attempt is a real, loud failure —
/// proceeding to launch a second oracle against an already-occupied port would either
/// fail outright or, worse, silently talk to whatever still holds it.
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
        "placement-diff: port {port} is already bound — attempting to clear a zombie oracle process before proceeding"
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
    eprintln!("placement-diff: port {port} is clear");
    Ok(())
}

/// Best-effort, run unconditionally on the way out of `run` (Context, "kill leftover
/// `rusty-clanker-server` processes on exit"): belt-and-braces alongside `ManagedServer
/// `'s own guaranteed-teardown `Drop` (which already kills the specific child this
/// process itself spawned) — this additionally clears any `rusty-clanker-server.exe`
/// that a killed-mid-flight `placement_diff_runner` subprocess left orphaned (a hard
/// `Command::kill` on that subprocess, e.g. from this function's own caller's timeout
/// path, ends *that* process without ever running *its own* `Drop` glue).
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

enum RunnerOutcome {
    Ok,
    Failure(String),
}

/// Runs `placement_diff_runner` as a subprocess with `args`, draining stdout/stderr
/// concurrently with the poll loop via the shared `crate::process::spawn_drained`
/// (M3.5-B06 — that module's own doc comment has the full pipe-buffer-deadlock diagnosis
/// this file's own hand-rolled drain threads used to duplicate) — never read only after the
/// child is observed to have exited, which is exactly the deadlock that diagnosis documents.
fn run_placement_diff_runner_subprocess(
    repo_root: &Path,
    args: &[String],
    deadline: Duration,
) -> RunnerOutcome {
    let paritybot_dir = repo_root.join("crates/testing/paritybot");

    let mut command = Command::new("cargo");
    command
        .current_dir(&paritybot_dir)
        .env("RUSTC_BOOTSTRAP", "1")
        .env_remove("RUSTUP_TOOLCHAIN")
        .arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("placement_diff_runner")
        .arg("--")
        .args(args);

    let (stdout, stderr) = match crate::process::spawn_drained(&mut command, deadline) {
        Ok(output) => (output.stdout, output.stderr),
        Err(crate::process::SpawnDrainedError::SpawnFailed(err)) => {
            return RunnerOutcome::Failure(format!("failed to spawn placement_diff_runner: {err}"));
        }
        Err(crate::process::SpawnDrainedError::PollFailed(err)) => {
            return RunnerOutcome::Failure(format!("failed to poll placement_diff_runner: {err}"));
        }
        Err(crate::process::SpawnDrainedError::TimedOut) => {
            return RunnerOutcome::Failure(format!(
                "placement_diff_runner did not exit within {deadline:?} of its own start"
            ));
        }
    };

    if stdout.lines().any(|line| line == "RESULT=OK") {
        return RunnerOutcome::Ok;
    }
    let message = stdout
        .lines()
        .find_map(|line| line.strip_prefix("MESSAGE="))
        .map(str::to_string)
        .unwrap_or_else(|| {
            let stderr_tail: String = stderr.lines().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
            format!(
                "placement_diff_runner produced no RESULT=OK line; stdout: {stdout:?}; stderr (last 20 lines): {stderr_tail}"
            )
        });
    RunnerOutcome::Failure(message)
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

/// The git-ignored oracle capture cache (Context, "Oracle results are cached like
/// corpus traces... hash-manifested if you can reuse the existing manifest machinery
/// cheaply; otherwise plain JSON cache with the oracle jar sha1 recorded") — a plain
/// postcard capture file plus a sidecar `.sha1` text file recording which jar produced
/// it, never `fetch_corpus.rs`'s own per-contraption `read_trace_if_current` machinery
/// (that machinery keys on a `RedstoneTrace`'s own `source_jar_sha1` *field*, one trace
/// per contraption; this cache is coarser-grained by design — one capture file for the
/// whole scenario set — since `--only` narrows a run cheaply enough that per-scenario
/// cache entries would add real complexity for no measured benefit here).
fn oracle_cache_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("corpus/placement-diff")
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
) -> Option<PlacementCaptureFile> {
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

/// I/O wrapper (`xtask placement-diff [--version 26.2] [--server-jar <path>]
/// --server-bin <path> [--only <scenario-id>] [--side oracle|ours|both] [--accept-eula]`).
pub fn run(args: &PlacementDiffArgs) -> std::process::ExitCode {
    let repo_root = repo_root();
    let mut result = TierResult::new("placement-diff");

    if args.side.wants_oracle()
        && let Err(message) = crate::corpus::fetch_corpus::eula_gate(&repo_root, args.accept_eula)
    {
        eprintln!("placement-diff: {message}");
        result.push("consent", Status::Fail, Some(message));
        return finish(result);
    }
    if args.side.wants_oracle() {
        result.push("consent", Status::Pass, None);
        if let Err(message) = zombie_oracle_check(ORACLE_PORT) {
            eprintln!("placement-diff: {message}");
            result.push("zombie-oracle-check", Status::Fail, Some(message));
            return finish(result);
        }
        result.push("zombie-oracle-check", Status::Pass, None);
    }

    let scenario_count = enumerate_scenarios().len() + InteractionScenario::ALL.len();
    eprintln!(
        "placement-diff: {scenario_count} scenario(s) enumerated{}",
        args.only
            .as_deref()
            .map(|only| format!(" (narrowed to --only {only:?})"))
            .unwrap_or_default()
    );

    let mut oracle_capture: Option<PlacementCaptureFile> = None;
    let mut ours_capture: Option<PlacementCaptureFile> = None;

    if args.side.wants_oracle() {
        let jar = if let Some(server_jar) = &args.server_jar {
            let server_jar = absolutize(&repo_root, server_jar);
            match std::fs::read(&server_jar) {
                Ok(bytes) => (server_jar.clone(), sha1_hex(&bytes)),
                Err(err) => {
                    let message = format!("failed to read {}: {err}", server_jar.display());
                    eprintln!("placement-diff: {message}");
                    result.push("resolve-jar", Status::Fail, Some(message));
                    return finish(result);
                }
            }
        } else {
            match crate::fetch_data::fetch_server_jar(&args.version, &repo_root) {
                Ok(jar) => (jar.jar_path, jar.sha1),
                Err(err) => {
                    eprintln!("placement-diff: {err}");
                    result.push("resolve-jar", Status::Fail, Some(err.to_string()));
                    return finish(result);
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
                "placement-diff: reusing cached oracle capture (sha1 {source_jar_sha1} matches)"
            );
            result.push(
                "capture-oracle",
                Status::Pass,
                Some("cache hit".to_string()),
            );
            oracle_capture = Some(cached);
        } else {
            let work_dir = repo_root
                .join("target/placement-diff-oracle")
                .join(&args.version);
            let out_path = repo_root.join("target/verify/placement-diff-oracle.postcard");
            let mut runner_args = vec![
                "oracle".to_string(),
                jar_path.display().to_string(),
                work_dir.display().to_string(),
                out_path.display().to_string(),
                source_jar_sha1.clone(),
            ];
            if let Some(only) = &args.only {
                runner_args.push(only.clone());
            }
            let deadline =
                Duration::from_secs(300) + Duration::from_secs(20) * scenario_count as u32;
            match run_placement_diff_runner_subprocess(&repo_root, &runner_args, deadline) {
                RunnerOutcome::Ok => match read_capture(&out_path) {
                    Ok(capture) => {
                        result.push("capture-oracle", Status::Pass, None);
                        if args.only.is_none()
                            && let Err(err) =
                                write_oracle_cache(&repo_root, &source_jar_sha1, &out_path)
                        {
                            eprintln!("placement-diff: failed to persist oracle cache: {err}");
                        }
                        oracle_capture = Some(capture);
                    }
                    Err(err) => {
                        let message = format!("failed to read {}: {err}", out_path.display());
                        result.push("capture-oracle", Status::Fail, Some(message));
                    }
                },
                RunnerOutcome::Failure(message) => {
                    eprintln!("placement-diff: oracle capture: {message}");
                    result.push("capture-oracle", Status::Fail, Some(message));
                }
            }
        }
    }

    if args.side.wants_ours() {
        let server_bin = absolutize(&repo_root, &args.server_bin);
        let world = TempWorldDir::new("ours");
        let out_path = repo_root.join("target/verify/placement-diff-ours.postcard");
        let mut runner_args = vec![
            "ours".to_string(),
            server_bin.display().to_string(),
            world.path.display().to_string(),
            out_path.display().to_string(),
        ];
        if let Some(only) = &args.only {
            runner_args.push(only.clone());
        }
        let deadline = Duration::from_secs(180) + Duration::from_secs(20) * scenario_count as u32;
        match run_placement_diff_runner_subprocess(&repo_root, &runner_args, deadline) {
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
                eprintln!("placement-diff: our capture: {message}");
                result.push("capture-ours", Status::Fail, Some(message));
            }
        }
    }

    if let (Some(oracle), Some(ours)) = (&oracle_capture, &ours_capture) {
        let blocks_index =
            BlocksIndex::load(&crate::corpus::blocks_report::default_reference_path());
        push_diff_cases(&mut result, oracle, ours, &blocks_index);
    }

    kill_leftover_rusty_clanker_server();
    finish(result)
}

fn finish(result: TierResult) -> std::process::ExitCode {
    let result = result.finalize();
    print_case_table(&result);
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("placement-diff: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}

/// One `TierResult` case per scenario id present in *either* capture — every
/// `Status::Fail` case's own `detail` decodes both sides' state ids (and, for the one
/// scenario that carries it, block-entity presence) via `blocks_index` for a human
/// reader, alongside the raw ids `diff_captures` itself compared. Missing-on-one-side
/// scenarios (`diff_captures`'s own doc comment — never silently dropped) get their
/// own dedicated failing case too.
fn push_diff_cases(
    result: &mut TierResult,
    oracle: &PlacementCaptureFile,
    ours: &PlacementCaptureFile,
    blocks_index: &BlocksIndex,
) {
    let report = diff_captures(oracle, ours);

    let mut mismatches_by_scenario: std::collections::BTreeMap<
        &str,
        Vec<&rc_gametest::CellMismatch>,
    > = std::collections::BTreeMap::new();
    for mismatch in &report.mismatches {
        mismatches_by_scenario
            .entry(mismatch.scenario_id.as_str())
            .or_default()
            .push(mismatch);
    }

    let all_ids: std::collections::BTreeSet<&str> = oracle
        .scenarios
        .iter()
        .map(|s| s.scenario_id.as_str())
        .chain(ours.scenarios.iter().map(|s| s.scenario_id.as_str()))
        .collect();

    for id in all_ids {
        if report.missing_in_oracle.iter().any(|m| m == id) {
            result.push(
                id,
                Status::Fail,
                Some("present in our capture but never captured against the oracle".to_string()),
            );
            continue;
        }
        if report.missing_in_ours.iter().any(|m| m == id) {
            result.push(
                id,
                Status::Fail,
                Some(
                    "present in the oracle capture but never captured against our own server"
                        .to_string(),
                ),
            );
            continue;
        }
        match mismatches_by_scenario.get(id) {
            None => {
                result.push(id, Status::Pass, None);
            }
            Some(mismatches) => {
                let detail = mismatches
                    .iter()
                    .map(|m| {
                        format!(
                            "cell {:?}: oracle={} (block-entity {}) vs ours={} (block-entity {})",
                            m.pos,
                            blocks_index.describe(m.oracle_state_id),
                            m.oracle_has_block_entity,
                            blocks_index.describe(m.ours_state_id),
                            m.ours_has_block_entity,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                result.push(id, Status::Fail, Some(detail));
            }
        }
    }
}

fn print_case_table(result: &TierResult) {
    println!(
        "placement-diff — {} case(s), overall {:?}",
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
