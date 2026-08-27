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

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::tier_result::{Status, TierResult};

pub struct FetchCorpusArgs {
    pub version: String,
    pub server_jar: Option<PathBuf>,
    pub only: Option<String>,
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
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return RunnerOutcome::ProcessFailure(format!(
                "failed to spawn fetch_corpus_runner: {err}"
            ));
        }
    };

    // A generous, fixed allowance for the nested `cargo run`'s own (possibly cold,
    // first-ever) build of the azalea-dependent binary, plus the full corpus
    // capture budget (Context, "Rates and limits": ≤10 minutes end to end).
    let build_grace = Duration::from_secs(300);
    let capture_budget = Duration::from_secs(600);
    let deadline = Instant::now() + build_grace + capture_budget;

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return RunnerOutcome::ProcessFailure(format!(
                        "fetch_corpus_runner did not exit within {deadline:?} of its own start"
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                return RunnerOutcome::ProcessFailure(format!(
                    "failed to poll fetch_corpus_runner: {err}"
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

    parse_runner_output(&stdout, &stderr)
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

/// I/O wrapper (`xtask fetch-corpus [--version 26.2] [--server-jar <path>] [--only
/// <id>]`).
pub fn run(args: &FetchCorpusArgs) -> std::process::ExitCode {
    let repo_root = repo_root();
    let mut result = TierResult::new("fetch-corpus");

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

    let corpus_ron_dir = repo_root.join("crates/testing/gametest/corpus/redstone");
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
