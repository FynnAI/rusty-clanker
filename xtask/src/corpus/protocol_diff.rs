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

enum RunnerOutcome {
    Ok,
    Failure(String),
}

/// Runs `protocol_diff_runner` as a subprocess with `args`, draining stdout/stderr
/// concurrently with the poll loop via the shared `crate::process::spawn_drained`
/// (M3.5-B06 — that module's own doc comment has the full pipe-buffer-deadlock
/// diagnosis this file's own hand-rolled drain threads used to duplicate) — never
/// read only after the child is observed to have exited, which is exactly the
/// deadlock that diagnosis documents.
fn run_protocol_diff_runner_subprocess(
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
        .arg("protocol_diff_runner")
        .arg("--")
        .args(args);

    let (stdout, stderr) = match crate::process::spawn_drained(&mut command, deadline) {
        Ok(output) => (output.stdout, output.stderr),
        Err(crate::process::SpawnDrainedError::SpawnFailed(err)) => {
            return RunnerOutcome::Failure(format!("failed to spawn protocol_diff_runner: {err}"));
        }
        Err(crate::process::SpawnDrainedError::PollFailed(err)) => {
            return RunnerOutcome::Failure(format!("failed to poll protocol_diff_runner: {err}"));
        }
        Err(crate::process::SpawnDrainedError::TimedOut) => {
            return RunnerOutcome::Failure(format!(
                "protocol_diff_runner did not exit within {deadline:?} of its own start"
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

/// I/O wrapper (`xtask protocol-diff [--version 26.2] [--server-jar <path>]
/// --server-bin <path> [--only <step>] [--side oracle|ours|both] [--accept-eula]
/// [--debug-hooks]`). Structurally identical to `placement_diff::run`: EULA gate,
/// zombie-oracle check, jar resolution + sha1-keyed cache, subprocess launch with
/// concurrent pipe drain, `protocol_capture::diff_captures`, `TierResult` written to
/// `target/verify/protocol-diff.json`.
pub fn run(args: &ProtocolDiffArgs) -> std::process::ExitCode {
    let repo_root = repo_root();
    let mut result = TierResult::new("protocol-diff");

    if args.side.wants_oracle()
        && let Err(message) = crate::corpus::fetch_corpus::eula_gate(&repo_root, args.accept_eula)
    {
        eprintln!("protocol-diff: {message}");
        result.push("consent", Status::Fail, Some(message));
        return finish(result);
    }
    if args.side.wants_oracle() {
        result.push("consent", Status::Pass, None);
        if let Err(message) = zombie_oracle_check(ORACLE_PORT) {
            eprintln!("protocol-diff: {message}");
            result.push("zombie-oracle-check", Status::Fail, Some(message));
            return finish(result);
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
                    return finish(result);
                }
            }
        } else {
            match crate::fetch_data::fetch_server_jar(&args.version, &repo_root) {
                Ok(jar) => (jar.jar_path, jar.sha1),
                Err(err) => {
                    eprintln!("protocol-diff: {err}");
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
            // (`SURVIVAL_DIG_HOLD`, ~9s) plus 51 real redstone-corpus placements each
            // dominate this budget far more than `placement-diff`'s own scenario
            // count ever did — generous enough for a real, contended-machine run.
            let deadline = Duration::from_secs(600) + Duration::from_secs(15) * 80;
            match run_protocol_diff_runner_subprocess(&repo_root, &runner_args, deadline) {
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
        let deadline = Duration::from_secs(300) + Duration::from_secs(15) * 80;
        match run_protocol_diff_runner_subprocess(&repo_root, &runner_args, deadline) {
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
    finish(result)
}

fn finish(result: TierResult) -> std::process::ExitCode {
    let result = result.finalize();
    print_case_table(&result);
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
    report_by_step: &std::collections::BTreeMap<String, rc_gametest::protocol_capture::ProtocolDiffReport>,
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
