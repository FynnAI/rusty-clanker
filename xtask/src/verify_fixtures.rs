//! TEST-D47: fixture integrity manifest — wired vacuous at M0. This is the
//! `crates/testing/rc-golden-data/fixtures/manifest.json` manifest (protected path #4,
//! `xtask/src/path_guard.rs`) — a different manifest from `xtask/src/fixture_manifest.rs`
//! (M0-B07's own `crates/registries/generated/v776/MANIFEST.json`, "verify-generated").

pub const MANIFEST_PATH: &str = "crates/testing/rc-golden-data/fixtures/manifest.json";

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub generator: String,
    pub source_jar_sha1: String,
}

/// Pure: for each `entries` row, recomputes the SHA-256 of `<repo_root>/<entry.path>`
/// and compares to `entry.sha256`. Returns `(path, expected, actual)` for every
/// mismatch; a missing file reports `actual = "<file missing>"`.
pub fn check_manifest(
    repo_root: &std::path::Path,
    entries: &[ManifestEntry],
) -> Vec<(String, String, String)> {
    todo!()
}

/// CLI entry point (`xtask verify-fixtures`): if `MANIFEST_PATH` does not exist,
/// writes a `target/verify/verify-fixtures.json` reporting `0` cases and returns
/// `ExitCode::SUCCESS` immediately. Otherwise parses it and runs `check_manifest`.
pub fn run() -> std::process::ExitCode {
    todo!()
}
