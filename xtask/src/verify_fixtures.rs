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
    let mut mismatches = Vec::new();
    for entry in entries {
        match std::fs::read(repo_root.join(&entry.path)) {
            Ok(bytes) => {
                let actual = crate::fixture_manifest::compute_sha256_hex(&bytes);
                if actual != entry.sha256 {
                    mismatches.push((entry.path.clone(), entry.sha256.clone(), actual));
                }
            }
            Err(_) => {
                mismatches.push((
                    entry.path.clone(),
                    entry.sha256.clone(),
                    "<file missing>".to_string(),
                ));
            }
        }
    }
    mismatches
}

/// CLI entry point (`xtask verify-fixtures`): if `MANIFEST_PATH` does not exist,
/// writes a `target/verify/verify-fixtures.json` reporting `0` cases and returns
/// `ExitCode::SUCCESS` immediately. Otherwise parses it and runs `check_manifest`.
pub fn run() -> std::process::ExitCode {
    let mut result = crate::tier_result::TierResult::new("verify-fixtures");

    let manifest_path = std::path::Path::new(MANIFEST_PATH);
    if !manifest_path.exists() {
        println!("verify-fixtures: {MANIFEST_PATH} does not exist yet — 0 entries, vacuous pass");
        result.push(
            "manifest-exists",
            crate::tier_result::Status::Pass,
            Some(format!("{MANIFEST_PATH} does not exist — 0 entries")),
        );
        let result = result.finalize();
        if let Err(err) = crate::tier_result::write(&result) {
            eprintln!("verify-fixtures: failed to write result JSON: {err}");
            return std::process::ExitCode::FAILURE;
        }
        return crate::tier_result::exit_code_for(result.status);
    }

    let repo_root = repo_root();
    let manifest_text = match std::fs::read_to_string(manifest_path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("verify-fixtures: failed to read {MANIFEST_PATH}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let entries: Vec<ManifestEntry> = match serde_json::from_str(&manifest_text) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("verify-fixtures: failed to parse {MANIFEST_PATH}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mismatches = check_manifest(&repo_root, &entries);
    if mismatches.is_empty() {
        result.push(
            "manifest-hashes",
            crate::tier_result::Status::Pass,
            Some(format!("{} entries, 0 mismatches", entries.len())),
        );
    } else {
        for (path, expected, actual) in &mismatches {
            eprintln!("verify-fixtures: {path}: manifest says {expected}, disk has {actual}");
            result.push(
                format!("manifest-hashes::{path}"),
                crate::tier_result::Status::Fail,
                Some(format!("manifest says {expected}, disk has {actual}")),
            );
        }
    }

    let result = result.finalize();
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("verify-fixtures: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}
