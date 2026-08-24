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
    todo!()
}

/// Writes the marker file (creating `oracle/` if needed) so future calls to
/// `consent_already_given` return `true` without re-checking the env var.
pub fn record_consent(repo_root: &Path) -> std::io::Result<()> {
    todo!()
}

/// The three harness scaffold directories `setup_oracle::run` creates, relative to
/// `repo_root` — a pure function so its shape is unit-testable without touching disk.
pub fn harness_dirs(repo_root: &Path, version_id: &str) -> [PathBuf; 3] {
    todo!()
}

/// Full bootstrap (TEST-D41): consent gate, then `fetch_data::fetch_server_jar` +
/// `fetch_data::run_data_reports` for `PINNED_VERSION`, then creates every path from
/// `harness_dirs` (empty — populated by a later blueprint). Writes
/// `target/verify/setup-oracle.json` (TEST-D40) and returns the matching `ExitCode`.
pub fn run(cli_accept_flag: bool) -> std::process::ExitCode {
    todo!()
}
