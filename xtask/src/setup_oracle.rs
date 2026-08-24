//! `xtask setup-oracle` (TEST-D41/D43/D44): the one-command, one-human-step
//! differential-testing oracle bootstrap. Never invoked by `tier0`/`tier1`/CI —
//! see this blueprint's Constraints (e).

use std::path::{Path, PathBuf};

pub const PINNED_VERSION: &str = "26.2"; // NET-D1
const CONSENT_MARKER_FILE: &str = "oracle/.eula-accepted";
const CONSENT_ENV_VAR: &str = "RC_ORACLE_EULA_ACCEPTED";

#[derive(thiserror::Error, Debug)]
pub enum SetupOracleError {
    #[error(transparent)]
    Fetch(#[from] crate::fetch_data::FetchDataError),
    #[error("legal consent required — re-run with --accept-eula, or set {CONSENT_ENV_VAR}=1")]
    ConsentRequired,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// True iff `<repo_root>/oracle/.eula-accepted` already exists, or `RC_ORACLE_EULA_ACCEPTED`
/// is set to exactly `"1"` in the process environment — the two unattended-after-first-run
/// paths TEST-D41/D43 require. Never itself prompts.
pub fn consent_already_given(repo_root: &Path) -> bool {
    if repo_root.join(CONSENT_MARKER_FILE).exists() {
        return true;
    }
    std::env::var(CONSENT_ENV_VAR).ok().as_deref() == Some("1")
}

/// Writes the marker file (creating `oracle/` if needed) so future calls to
/// `consent_already_given` return `true` without re-checking the env var.
pub fn record_consent(repo_root: &Path) -> std::io::Result<()> {
    let marker = repo_root.join(CONSENT_MARKER_FILE);
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(marker, b"")
}

/// The three harness scaffold directories `setup_oracle::run` creates, relative to
/// `repo_root` — a pure function so its shape is unit-testable without touching disk.
pub fn harness_dirs(repo_root: &Path, version_id: &str) -> [PathBuf; 3] {
    let harness_root = repo_root
        .join(crate::fetch_data::ORACLE_JAR_DIR)
        .join(version_id)
        .join("harness");
    [
        harness_root.join("scenarios"),
        harness_root.join("seeds"),
        harness_root.join("working"),
    ]
}

/// Full bootstrap (TEST-D41): consent gate, then `fetch_data::fetch_server_jar` +
/// `fetch_data::run_data_reports` for `PINNED_VERSION`, then creates every path from
/// `harness_dirs` (empty — populated by a later blueprint). Writes
/// `target/verify/setup-oracle.json` (TEST-D40) and returns the matching `ExitCode`.
pub fn run(cli_accept_flag: bool) -> std::process::ExitCode {
    let repo_root = repo_root();
    let mut result = crate::tier_result::TierResult::new("setup-oracle");

    if !consent_already_given(&repo_root) && !cli_accept_flag {
        let err = SetupOracleError::ConsentRequired;
        eprintln!("setup-oracle: {err}");
        result.push(
            "consent",
            crate::tier_result::Status::Fail,
            Some(err.to_string()),
        );
        let result = result.finalize();
        let _ = crate::tier_result::write(&result);
        return crate::tier_result::exit_code_for(result.status);
    }
    if cli_accept_flag
        && !consent_already_given(&repo_root)
        && let Err(err) = record_consent(&repo_root)
    {
        eprintln!("setup-oracle: failed to record consent: {err}");
        result.push(
            "consent",
            crate::tier_result::Status::Fail,
            Some(err.to_string()),
        );
        let result = result.finalize();
        let _ = crate::tier_result::write(&result);
        return crate::tier_result::exit_code_for(result.status);
    }
    result.push("consent", crate::tier_result::Status::Pass, None);

    let jar = match crate::fetch_data::fetch_server_jar(PINNED_VERSION, &repo_root) {
        Ok(jar) => jar,
        Err(err) => {
            eprintln!("setup-oracle: {err}");
            result.push(
                "fetch-server-jar",
                crate::tier_result::Status::Fail,
                Some(err.to_string()),
            );
            let result = result.finalize();
            let _ = crate::tier_result::write(&result);
            return crate::tier_result::exit_code_for(result.status);
        }
    };
    result.push(
        "fetch-server-jar",
        crate::tier_result::Status::Pass,
        Some(format!("sha1 {}", jar.sha1)),
    );

    let reports_dir = match crate::fetch_data::run_data_reports(&jar, &repo_root) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("setup-oracle: {err}");
            result.push(
                "run-data-reports",
                crate::tier_result::Status::Fail,
                Some(err.to_string()),
            );
            let result = result.finalize();
            let _ = crate::tier_result::write(&result);
            return crate::tier_result::exit_code_for(result.status);
        }
    };
    result.push(
        "run-data-reports",
        crate::tier_result::Status::Pass,
        Some(reports_dir.display().to_string()),
    );

    let mut dirs_ok = true;
    for dir in harness_dirs(&repo_root, PINNED_VERSION) {
        if let Err(err) = std::fs::create_dir_all(&dir) {
            eprintln!("setup-oracle: failed to create {}: {err}", dir.display());
            dirs_ok = false;
        }
    }
    result.push(
        "harness-dirs",
        if dirs_ok {
            crate::tier_result::Status::Pass
        } else {
            crate::tier_result::Status::Fail
        },
        None,
    );

    let result = result.finalize();
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("setup-oracle: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "setup-oracle: OK — jar sha1 {}, reports at {}",
        jar.sha1,
        reports_dir.display()
    );
    crate::tier_result::exit_code_for(result.status)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}
