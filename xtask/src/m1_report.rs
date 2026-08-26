//! M1-B06: drives the M1 acceptance harness against a real, freshly-spawned
//! `rusty-clanker-server` and writes `target/verify/m1-acceptance.json`.
//!
//! Forced deviation from this blueprint's own Deliverables sketch (Context,
//! "azalea's own upstream nightly-toolchain requirement" — restated here since this
//! module is where the consequence actually lands): this verb never links `azalea`
//! or `rc-paritybot` into `xtask.exe` itself (every other Tier-1 gate depends on
//! building that binary under this project's pinned *stable* toolchain, WS-D4).
//! `rc-test-harness`'s probe/process-orchestration pieces are still called in-process
//! (they are `tokio`/`azalea`-free); the azalea-dependent idle-stability scenario is
//! instead driven by spawning `rc-paritybot`'s own `idle_stability_runner` binary as
//! a real OS subprocess, built via a nested `cargo run` invoked with
//! `crates/testing/paritybot/` as its working directory (picking up that crate's own
//! nested `rust-toolchain.toml` override, `channel = "nightly"`) and
//! `RUSTC_BOOTSTRAP=1` set for good measure. This verb therefore stays fully
//! synchronous — no `tokio::runtime::Runtime`/`block_on` anywhere in `xtask`.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rc_test_harness::probe::{ProbeConfig, probe_status};
use rc_test_harness::process::{ManagedServerConfig, spawn_server};

use crate::tier_result::{CaseResult, Status, TierResult};

#[derive(serde::Serialize)]
pub struct ManualStep {
    pub id: &'static str,
    pub description: &'static str,
    pub procedure_doc: &'static str,
}

/// Wraps `TierResult` (unmodified — see Constraints, "no edit to tier_result.rs")
/// with the one field TEST-D40's schema has no slot for: a manual, non-automatable
/// step (AC3), which is never a `CaseResult` and never affects `automated.status`.
#[derive(serde::Serialize)]
pub struct M1ReportResult {
    #[serde(flatten)]
    pub automated: TierResult, // tier = "m1-acceptance"; cases named "AC1a_status_pong",
    // "AC1b_login_config_play_spawn", "AC1c_idle_stability",
    // "AC2_status_json_fields" — every case Pass/Fail per
    // tier_result::Status, aggregated the same way tier1::run
    // already aggregates (Status::Fail if any case failed)
    pub manual_steps: Vec<ManualStep>, // always exactly one entry, AC3
    pub mode: String,                  // "smoke" | "full"
    pub target: String,                // "<ip>:<port>" actually used
}

pub const OUT_PATH: &str = "target/verify/m1-acceptance.json";

const AC3_MANUAL_STEP: ManualStep = ManualStep {
    id: "AC3",
    description: "Online-mode session validation against a real Microsoft/Mojang account — cannot be automated (09-testing-quality.md's zero-human-test-loop principle governs routine verification only; a real account login is a genuine one-time human action).",
    procedure_doc: "docs/MANUAL-VERIFICATION-M1.md",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    Smoke,
    Full,
}

impl Mode {
    /// `Smoke` -> `Duration::from_secs(90)`, `Full` -> `Duration::from_secs(1800)`.
    pub fn idle_duration(self) -> Duration {
        match self {
            Mode::Smoke => Duration::from_secs(90),
            Mode::Full => Duration::from_secs(1800),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Mode::Smoke => "smoke",
            Mode::Full => "full",
        }
    }
}

/// Mirrors `xtask/src/main.rs`'s own `repo_root` — each call site computes this the
/// same way rather than one calling into another's internals (that file's own
/// precedent).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

/// CLI entry point (`xtask m1-report --server-bin <path> --mode {smoke|full}`).
pub fn run(server_bin: PathBuf, mode: Mode) -> std::process::ExitCode {
    let idle_duration = mode.idle_duration();
    let mut result = TierResult::new("m1-acceptance");
    let mut target = String::new();

    let managed = match spawn_server(ManagedServerConfig {
        binary_path: server_bin,
        offline: true,
        startup_timeout: Duration::from_secs(30),
        extra_args: Vec::new(),
        // M2-B08: three new, purely additive `ManagedServerConfig` fields
        // (Deliverables) -- `None` on every one reproduces this pre-existing call
        // site's own exact prior behavior.
        ..Default::default()
    }) {
        Ok(managed) => managed,
        Err(err) => {
            let detail = format!("failed to start rusty-clanker-server: {err}");
            for case in [
                "AC1a_status_pong",
                "AC1b_login_config_play_spawn",
                "AC1c_idle_stability",
                "AC2_status_json_fields",
            ] {
                result.push(case, Status::Fail, Some(detail.clone()));
            }
            return finish(result, mode, target);
        }
    };
    target = managed.addr.to_string();

    // AC1a/AC2: the raw-TCP status probe covers both connectivity+pong (AC1a) and
    // the Status Response JSON field validation (AC2) in one round trip.
    match probe_status(&ProbeConfig::new("127.0.0.1", managed.addr.port()), 776) {
        Ok(_) => {
            result.push("AC1a_status_pong", Status::Pass, None);
            result.push("AC2_status_json_fields", Status::Pass, None);
        }
        Err(err) => {
            let detail = err.to_string();
            result.push("AC1a_status_pong", Status::Fail, Some(detail.clone()));
            result.push("AC2_status_json_fields", Status::Fail, Some(detail));
        }
    }

    // AC1b/AC1c: the azalea-driven idle-stability scenario, run as its own
    // subprocess (module doc comment).
    let outcome = run_idle_stability_subprocess(
        "127.0.0.1",
        managed.addr.port(),
        Duration::from_secs(30),
        idle_duration,
    );
    let (ac1b, ac1c) = classify_idle_stability_outcome(&outcome);
    result.cases.push(ac1b);
    result.cases.push(ac1c);

    drop(managed); // explicit: tear the server down before writing the report

    finish(result, mode, target)
}

fn finish(mut result: TierResult, mode: Mode, target: String) -> std::process::ExitCode {
    result = result.finalize();
    let report = M1ReportResult {
        automated: result,
        manual_steps: vec![AC3_MANUAL_STEP],
        mode: mode.as_str().to_string(),
        target,
    };
    let status = report.automated.status;
    if let Err(err) = write_report(&report) {
        eprintln!("m1-report: failed to write {OUT_PATH}: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(status)
}

fn write_report(report: &M1ReportResult) -> std::io::Result<()> {
    let path = std::path::Path::new(OUT_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

enum IdleStabilityOutcome {
    Ok {
        #[allow(dead_code)]
        reached_login: bool,
        #[allow(dead_code)]
        reached_spawn: bool,
    },
    Error(String),
    ProcessFailure(String),
}

/// Builds and runs `rc-paritybot`'s `idle_stability_runner` as a subprocess (module
/// doc comment), bounded by `login_timeout + idle_duration` plus a generous fixed
/// grace period for the nested `cargo run`'s own (possibly cold) build time.
fn run_idle_stability_subprocess(
    host: &str,
    port: u16,
    login_timeout: Duration,
    idle_duration: Duration,
) -> IdleStabilityOutcome {
    let paritybot_dir = repo_root().join("crates/testing/paritybot");
    // Deliberately a `dev`, not `--release`, build: verified live that this
    // workspace's own `[profile.release]` (`lto = "fat"`, `codegen-units = 1`)
    // combined with the pinned nightly compiler this crate's own override selects
    // triggers a genuine `rustc` internal-compiler-error in `tokio`'s own codegen —
    // an upstream nightly/LTO interaction bug, not anything in this project's code.
    // A `dev` build sidesteps it entirely and is the right speed/robustness
    // trade-off for a verification tool anyway (never shipped).
    let mut command = Command::new("cargo");
    command
        .current_dir(&paritybot_dir)
        .env("RUSTC_BOOTSTRAP", "1")
        // Integration fix (discovered running the real M1 acceptance leg): when this
        // xtask process is itself launched via `cargo run -p xtask` from the repo
        // root, rustup's own proxy resolves the root's pinned stable toolchain
        // (`rust-toolchain.toml`, WS-D4) *before* exec'ing xtask.exe and stamps that
        // resolution into `RUSTUP_TOOLCHAIN` in xtask's own environment as a caching
        // optimization. `std::process::Command` inherits the parent environment by
        // default, so that stamped stable toolchain silently leaked into this nested
        // `cargo` invocation and overrode `paritybot_dir`'s own `rust-toolchain.toml`
        // (`channel = "nightly-2026-07-25"`) entirely -- reproduced live: with
        // `RUSTUP_TOOLCHAIN` set, azalea's `build.rs` panics with "Azalea currently
        // requires nightly Rust" even though the correct nightly toolchain is
        // installed and resolves correctly for this exact command run standalone
        // (no enclosing `cargo run`). Removing the inherited override here lets
        // rustup re-resolve the toolchain from `paritybot_dir`'s own file, as
        // originally intended by this module's own doc comment above.
        .env_remove("RUSTUP_TOOLCHAIN")
        .args([
            "run",
            "--quiet",
            "--bin",
            "idle_stability_runner",
            "--",
            host,
            &port.to_string(),
            "rc_m1_report_bot",
            &login_timeout.as_secs().to_string(),
            &idle_duration.as_secs().to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return IdleStabilityOutcome::ProcessFailure(format!(
                "failed to spawn idle_stability_runner: {err}"
            ));
        }
    };

    // A generous, fixed allowance for the nested `cargo run`'s own (possibly cold,
    // first-ever) build of the azalea-dependent binary — never part of the timed
    // scenario itself, purely a bound against a hung/never-returning subprocess.
    let build_grace = Duration::from_secs(300);
    let deadline = Instant::now() + login_timeout + idle_duration + build_grace;

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return IdleStabilityOutcome::ProcessFailure(format!(
                        "idle_stability_runner did not exit within {deadline:?} of its own start"
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                return IdleStabilityOutcome::ProcessFailure(format!(
                    "failed to poll idle_stability_runner: {err}"
                ));
            }
        }
    }

    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }

    parse_idle_stability_output(&stdout, &stderr)
}

fn parse_idle_stability_output(stdout: &str, stderr: &str) -> IdleStabilityOutcome {
    let mut result_line: Option<&str> = None;
    let mut reached_login = false;
    let mut reached_spawn = false;
    let mut message = String::new();

    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("RESULT=") {
            result_line = Some(value);
        } else if let Some(value) = line.strip_prefix("REACHED_LOGIN=") {
            reached_login = value == "true";
        } else if let Some(value) = line.strip_prefix("REACHED_SPAWN=") {
            reached_spawn = value == "true";
        } else if let Some(value) = line.strip_prefix("MESSAGE=") {
            message = value.to_string();
        }
    }

    match result_line {
        Some("OK") => IdleStabilityOutcome::Ok {
            reached_login,
            reached_spawn,
        },
        Some("ERROR") => IdleStabilityOutcome::Error(message),
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
            IdleStabilityOutcome::ProcessFailure(format!(
                "idle_stability_runner produced no parseable RESULT= line; stdout: {stdout:?}; stderr (last 20 lines): {stderr_tail}"
            ))
        }
    }
}

/// Maps one `IdleStabilityOutcome` onto AC1b's/AC1c's own `CaseResult`s. A
/// `DisconnectedDuringIdle` error means the connection genuinely reached Play/spawn
/// (AC1b passed) before failing to hold through the full idle window (AC1c fails);
/// every other outcome (a login/spawn failure, or the subprocess itself never
/// producing a result) fails both, since AC1c's prerequisite was never met.
fn classify_idle_stability_outcome(outcome: &IdleStabilityOutcome) -> (CaseResult, CaseResult) {
    match outcome {
        IdleStabilityOutcome::Ok { .. } => (
            CaseResult {
                name: "AC1b_login_config_play_spawn".to_string(),
                status: Status::Pass,
                detail: None,
            },
            CaseResult {
                name: "AC1c_idle_stability".to_string(),
                status: Status::Pass,
                detail: None,
            },
        ),
        IdleStabilityOutcome::Error(message)
            if message.contains("disconnected during the idle window") =>
        {
            (
                CaseResult {
                    name: "AC1b_login_config_play_spawn".to_string(),
                    status: Status::Pass,
                    detail: None,
                },
                CaseResult {
                    name: "AC1c_idle_stability".to_string(),
                    status: Status::Fail,
                    detail: Some(message.clone()),
                },
            )
        }
        IdleStabilityOutcome::Error(message) => (
            CaseResult {
                name: "AC1b_login_config_play_spawn".to_string(),
                status: Status::Fail,
                detail: Some(message.clone()),
            },
            CaseResult {
                name: "AC1c_idle_stability".to_string(),
                status: Status::Fail,
                detail: Some("prerequisite (AC1b) not met".to_string()),
            },
        ),
        IdleStabilityOutcome::ProcessFailure(message) => (
            CaseResult {
                name: "AC1b_login_config_play_spawn".to_string(),
                status: Status::Fail,
                detail: Some(message.clone()),
            },
            CaseResult {
                name: "AC1c_idle_stability".to_string(),
                status: Status::Fail,
                detail: Some("prerequisite (AC1b) not met".to_string()),
            },
        ),
    }
}
