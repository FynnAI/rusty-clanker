//! `xtask fetch-corpus` (blueprint Deliverables): resolves the pinned jar (reusing
//! `xtask::fetch_data::fetch_server_jar` unmodified, M0-B08), then drives the whole
//! corpus capture as a real OS subprocess — `rc-paritybot`'s own new
//! `fetch_corpus_runner` bin target, mirroring `m1_report`/`m2_report`'s already-
//! established `idle_stability_runner`/`restart_persistence_runner` subprocess
//! pattern exactly (`xtask.exe` itself never links `azalea`, `rc-gametest`'s own
//! `Cargo.toml` doc comment has the full citation for why `rc-gametest` alone —
//! which *is* linked directly, see `parity_check.rs` — cannot supply this leg).
//! Never runs in Tier 1 (Context, "CI tier placement") — needs a real oracle
//! process, network or a locally-cached jar, and Java.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use rc_gametest::spec::load_spec;

use crate::corpus::parity_check;
use crate::tier_result::{Status, TierResult};

pub struct FetchCorpusArgs {
    pub version: String,
    pub server_jar: Option<PathBuf>,
    pub only: Option<String>,
    /// TEST-D41 legal consent, same flag shape as `xtask setup-oracle --accept-eula`.
    pub accept_eula: bool,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

enum RunnerOutcome {
    Cases(Vec<(String, Status, Option<String>)>),
    ProcessFailure(String),
}

/// Builds and runs `rc-paritybot`'s `fetch_corpus_runner` as a subprocess
/// (`m1_report::run_idle_stability_subprocess`'s own identical mechanism, restated
/// here): `current_dir` set to `crates/testing/paritybot` so rustup resolves that
/// crate's own nested nightly `rust-toolchain.toml`, `RUSTC_BOOTSTRAP=1` set, and
/// `RUSTUP_TOOLCHAIN` removed from the child's environment (that same module's own
/// doc comment explains why the removal is required).
fn run_fetch_corpus_runner_subprocess(
    repo_root: &std::path::Path,
    jar_path: &std::path::Path,
    work_dir: &std::path::Path,
    corpus_ron_dir: &std::path::Path,
    corpus_out_dir: &std::path::Path,
    source_jar_sha1: &str,
    only: Option<&str>,
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
        .arg("fetch_corpus_runner")
        .arg("--")
        .arg(jar_path)
        .arg(work_dir)
        .arg(corpus_ron_dir)
        .arg(corpus_out_dir)
        .arg(source_jar_sha1);
    if let Some(only) = only {
        command.arg(only);
    }

    // A generous, fixed allowance for the nested `cargo run`'s own (possibly cold,
    // first-ever) build of the azalea-dependent binary, plus the full corpus
    // capture budget (Context, "Rates and limits": ≤10 minutes end to end).
    // Concurrent pipe drains via `crate::process::spawn_drained` (M3.5-B06) — that
    // module's own doc comment has the full pipe-buffer-deadlock diagnosis.
    let build_grace = Duration::from_secs(300);
    let capture_budget = Duration::from_secs(600);
    let deadline = build_grace + capture_budget;

    match crate::process::spawn_drained(&mut command, deadline) {
        Ok(output) => parse_runner_output(&output.stdout, &output.stderr),
        Err(crate::process::SpawnDrainedError::SpawnFailed(err)) => {
            RunnerOutcome::ProcessFailure(format!("failed to spawn fetch_corpus_runner: {err}"))
        }
        Err(crate::process::SpawnDrainedError::PollFailed(err)) => {
            RunnerOutcome::ProcessFailure(format!("failed to poll fetch_corpus_runner: {err}"))
        }
        Err(crate::process::SpawnDrainedError::TimedOut) => RunnerOutcome::ProcessFailure(format!(
            "fetch_corpus_runner did not exit within {deadline:?} of its own start"
        )),
    }
}

fn parse_runner_output(stdout: &str, stderr: &str) -> RunnerOutcome {
    let mut cases = Vec::new();
    let mut result_line: Option<&str> = None;
    let mut error_message = String::new();

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("CASE=") {
            let (id, rest) = rest.split_once(" STATUS=").unwrap_or((rest, ""));
            let (status_word, detail) = rest.split_once(" DETAIL=").unwrap_or((rest, ""));
            let status = if status_word == "pass" {
                Status::Pass
            } else {
                Status::Fail
            };
            let detail = if detail.is_empty() {
                None
            } else {
                Some(detail.to_string())
            };
            cases.push((id.to_string(), status, detail));
        } else if let Some(value) = line.strip_prefix("RESULT=") {
            result_line = Some(value);
        } else if let Some(value) = line.strip_prefix("MESSAGE=") {
            error_message = value.to_string();
        }
    }

    match result_line {
        Some("OK") => RunnerOutcome::Cases(cases),
        Some("ERROR") => RunnerOutcome::ProcessFailure(error_message),
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
                "fetch_corpus_runner produced no parseable RESULT= line; stdout: {stdout:?}; stderr (last 20 lines): {stderr_tail}"
            ))
        }
    }
}

/// DEFECT 5 fix (TEST-D41): this verb ultimately launches the same real, legally-gated
/// vanilla oracle jar `xtask setup-oracle` does
/// (`crates/testing/gametest/src/capture.rs::launch_oracle_server`, which unconditionally
/// writes `eula=true`) but never itself checked the project's own consent gate — an
/// implementation slip against `blueprints/M3/M3-B07-redstone-corpus.md`'s own explicit claim
/// that this path is already covered by it. `rc-paritybot` (which actually spawns the jar,
/// several process-hops downstream) cannot depend on `xtask` (WS-D4's own crate-graph
/// direction), so the gate lives here, at the one call site that can enforce it before any
/// subprocess is spawned — reusing `setup_oracle::consent_already_given`/`record_consent`
/// verbatim, never a second, independent consent primitive. Factored out as its own function,
/// parameterized on `repo_root` (rather than inlined in `run` using this file's own
/// hardcoded `repo_root()`), so this crate's own integration tests can exercise every branch
/// against a disposable temp directory instead of the real project's own `oracle/.eula-
/// accepted` marker.
pub fn eula_gate(repo_root: &std::path::Path, accept_eula: bool) -> Result<(), String> {
    if crate::setup_oracle::consent_already_given(repo_root) {
        return Ok(());
    }
    if !accept_eula {
        return Err(
            "legal consent required — re-run with --accept-eula, or set \
                     RC_ORACLE_EULA_ACCEPTED=1 (same gate as `xtask setup-oracle`, TEST-D41) \
                     — this verb launches the same real vanilla oracle jar"
                .to_string(),
        );
    }
    crate::setup_oracle::record_consent(repo_root)
        .map_err(|err| format!("failed to record consent: {err}"))
}

/// I/O wrapper (`xtask fetch-corpus [--version 26.2] [--server-jar <path>] [--only
/// <id>] [--accept-eula]`).
pub fn run(args: &FetchCorpusArgs) -> std::process::ExitCode {
    let repo_root = repo_root();
    let mut result = TierResult::new("fetch-corpus");

    if let Err(message) = eula_gate(&repo_root, args.accept_eula) {
        eprintln!("fetch-corpus: {message}");
        result.push("consent", Status::Fail, Some(message));
        let result = result.finalize();
        let _ = crate::tier_result::write(&result);
        return crate::tier_result::exit_code_for(result.status);
    }
    result.push("consent", Status::Pass, None);

    // DEFECT 4(b) fix: an `--only` value matching no committed contraption spec used to leave
    // the spec list `fetch_corpus_runner` builds empty while `run_full_corpus_capture` still
    // launched the real oracle and connected a bot — real wall-clock cost — captured nothing,
    // and printed `RESULT=OK` with zero `CASE=` lines, which this function's own aggregation
    // (below) then read as a vacuous `Pass`. Resolved here, before the jar is even resolved,
    // let alone the oracle launched — reusing `parity_check.rs`'s own identical `--only` check
    // (`only_filter_matches_nothing`) against the same committed corpus directory, not a
    // second, independently-drifting copy of the same logic.
    let corpus_ron_dir = repo_root.join("crates/testing/gametest/corpus/redstone");
    let ron_paths = match parity_check::sorted_ron_paths(&corpus_ron_dir) {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!(
                "fetch-corpus: failed to read {}: {err}",
                corpus_ron_dir.display()
            );
            result.push("only-filter", Status::Fail, Some(err.to_string()));
            let result = result.finalize();
            let _ = crate::tier_result::write(&result);
            return crate::tier_result::exit_code_for(result.status);
        }
    };
    if let Some(only) = &args.only
        && parity_check::only_filter_matches_nothing(&ron_paths, only)
    {
        let known_ids: Vec<String> = ron_paths
            .iter()
            .filter_map(|path| load_spec(path).ok())
            .map(|spec| spec.id)
            .collect();
        let message = format!(
            "--only {only:?} matches no committed contraption spec under {} (known ids: \
             {known_ids:?}) — refusing to launch the oracle to verify nothing",
            corpus_ron_dir.display()
        );
        eprintln!("fetch-corpus: {message}");
        result.push("only-filter", Status::Fail, Some(message));
        let result = result.finalize();
        let _ = crate::tier_result::write(&result);
        return crate::tier_result::exit_code_for(result.status);
    }

    let jar = if let Some(server_jar) = &args.server_jar {
        crate::fetch_data::FetchedJar {
            jar_path: server_jar.clone(),
            version_id: args.version.clone(),
            sha1: match std::fs::read(server_jar) {
                Ok(bytes) => sha1_hex(&bytes),
                Err(err) => {
                    eprintln!(
                        "fetch-corpus: failed to read {}: {err}",
                        server_jar.display()
                    );
                    result.push("resolve-jar", Status::Fail, Some(err.to_string()));
                    let result = result.finalize();
                    let _ = crate::tier_result::write(&result);
                    return crate::tier_result::exit_code_for(result.status);
                }
            },
            min_java_major: 0,
        }
    } else {
        match crate::fetch_data::fetch_server_jar(&args.version, &repo_root) {
            Ok(jar) => jar,
            Err(err) => {
                eprintln!("fetch-corpus: {err}");
                result.push("resolve-jar", Status::Fail, Some(err.to_string()));
                let result = result.finalize();
                let _ = crate::tier_result::write(&result);
                return crate::tier_result::exit_code_for(result.status);
            }
        }
    };
    result.push(
        "resolve-jar",
        Status::Pass,
        Some(format!("sha1 {}", jar.sha1)),
    );

    let corpus_out_dir = repo_root.join("corpus/redstone");
    let work_dir = repo_root
        .join("target/fetch-corpus-oracle")
        .join(&args.version);

    let outcome = run_fetch_corpus_runner_subprocess(
        &repo_root,
        &jar.jar_path,
        &work_dir,
        &corpus_ron_dir,
        &corpus_out_dir,
        &jar.sha1,
        args.only.as_deref(),
    );

    match outcome {
        RunnerOutcome::Cases(cases) => {
            for (id, status, detail) in cases {
                result.push(id, status, detail);
            }
        }
        RunnerOutcome::ProcessFailure(message) => {
            eprintln!("fetch-corpus: {message}");
            result.push("fetch_corpus_runner", Status::Fail, Some(message));
        }
    }

    let result = result.finalize();
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("fetch-corpus: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
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
