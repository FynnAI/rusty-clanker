//! Shared piston-meta resolution / `server.jar` fetch / `--reports` invocation
//! primitive (NET-D9, ASSET-D29). Specified and owned by blueprint M0-B08
//! (`M0-B08-verification-wiring.md`'s "`xtask/src/fetch_data.rs` (new — shared with
//! M0-B07)" deliverable) — this file exists here, ahead of M0-B08's own changeset,
//! only because M0-B07 (this blueprint) hard-depends on it at compile time and
//! M0-B08 has not yet landed in this repository. Its shape (constants, struct/enum
//! fields, function signatures and doc comments) is copied verbatim from M0-B08's own
//! binding spec, not invented — so M0-B08's eventual implementer finds this module
//! already exactly matching what their own blueprint asks them to create, and can
//! proceed directly to their remaining deliverables (`setup_oracle.rs`,
//! `tier_result.rs`, `path_guard.rs`, etc.) without redoing this one. See M0-B07's
//! final report for the full citation of this forced deviation.

use std::path::{Path, PathBuf};

pub const ORACLE_JAR_DIR: &str = "oracle";
pub const DATAGEN_OUTPUT_DIR: &str = "datagen-output";
const PISTON_META_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

pub struct FetchedJar {
    pub jar_path: PathBuf,
    pub version_id: String,
    /// SHA-1 recorded by piston-meta for this version's server.jar, already verified
    /// against the actually-downloaded (or cache-hit) bytes.
    pub sha1: String,
    /// The per-version manifest's own declared `javaVersion.majorVersion` (read once
    /// here, during piston-meta resolution) — exposed so a caller needing it (e.g.
    /// M0-B07's `fetch-data` verb, which must check a local Java runtime meets this
    /// floor) never re-fetches or re-parses the manifest a second time just for this
    /// one field. Not consulted by `run_data_reports` itself, which only checks that
    /// some `java` binary is runnable at all — see its own doc comment.
    pub min_java_major: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum FetchDataError {
    #[error("network error contacting {0}: {1}")]
    Network(String, String),
    #[error("version {0} not found in the piston-meta manifest")]
    VersionNotFound(String),
    #[error("downloaded server.jar SHA-1 mismatch: manifest says {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("`java` was not found on PATH — a JRE 21+ is required to run --reports")]
    JavaNotFound,
    #[error("`--reports` exited with a non-zero status")]
    ReportsFailed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(serde::Deserialize)]
struct VersionManifestV2 {
    versions: Vec<VersionManifestEntry>,
}

#[derive(serde::Deserialize)]
struct VersionManifestEntry {
    id: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct PerVersionManifest {
    downloads: Downloads,
    #[serde(rename = "javaVersion")]
    java_version: JavaVersion,
}

#[derive(serde::Deserialize)]
struct Downloads {
    server: ServerDownload,
}

#[derive(serde::Deserialize)]
struct ServerDownload {
    sha1: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct JavaVersion {
    #[serde(rename = "majorVersion")]
    major_version: u32,
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

/// Resolves `version_id` (e.g. `"26.2"`) against `PISTON_META_MANIFEST_URL`,
/// downloads the matching `server.jar` to `<repo_root>/<ORACLE_JAR_DIR>/<version_id>/server.jar`
/// (skipping the download if that path already exists AND its SHA-1 already matches
/// the manifest's recorded value — the TEST-D44 fast-path), and returns it.
pub fn fetch_server_jar(version_id: &str, repo_root: &Path) -> Result<FetchedJar, FetchDataError> {
    let manifest_text = reqwest::blocking::get(PISTON_META_MANIFEST_URL)
        .and_then(|resp| resp.text())
        .map_err(|e| {
            FetchDataError::Network(PISTON_META_MANIFEST_URL.to_string(), e.to_string())
        })?;
    let manifest: VersionManifestV2 = serde_json::from_str(&manifest_text).map_err(|e| {
        FetchDataError::Network(PISTON_META_MANIFEST_URL.to_string(), e.to_string())
    })?;

    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| FetchDataError::VersionNotFound(version_id.to_string()))?;

    let per_version_text = reqwest::blocking::get(&entry.url)
        .and_then(|resp| resp.text())
        .map_err(|e| FetchDataError::Network(entry.url.clone(), e.to_string()))?;
    let per_version: PerVersionManifest = serde_json::from_str(&per_version_text)
        .map_err(|e| FetchDataError::Network(entry.url.clone(), e.to_string()))?;

    let expected_sha1 = per_version.downloads.server.sha1;
    let min_java_major = per_version.java_version.major_version;

    let jar_dir = repo_root.join(ORACLE_JAR_DIR).join(version_id);
    std::fs::create_dir_all(&jar_dir)?;
    let jar_path = jar_dir.join("server.jar");

    let needs_download = match std::fs::read(&jar_path) {
        Ok(existing) => sha1_hex(&existing) != expected_sha1,
        Err(_) => true,
    };

    if needs_download {
        let server_url = per_version.downloads.server.url;
        let bytes = reqwest::blocking::get(&server_url)
            .and_then(|resp| resp.bytes())
            .map_err(|e| FetchDataError::Network(server_url.clone(), e.to_string()))?;
        let actual_sha1 = sha1_hex(&bytes);
        if actual_sha1 != expected_sha1 {
            return Err(FetchDataError::HashMismatch {
                expected: expected_sha1,
                actual: actual_sha1,
            });
        }
        std::fs::write(&jar_path, &bytes)?;
    }

    Ok(FetchedJar {
        jar_path,
        version_id: version_id.to_string(),
        sha1: expected_sha1,
        min_java_major,
    })
}

/// Runs `java -DbundlerMainClass=net.minecraft.data.Main -jar <jar.jar_path> --reports`
/// — copied verbatim from NET-D9's own pinned invocation text, with **no** `--output`
/// flag added (M0-B07's own Context carries the identical constraint: "no `--output`
/// flag is passed, matching NET-D9's exact invocation string with nothing added") —
/// with the subprocess's working directory set to
/// `<repo_root>/<DATAGEN_OUTPUT_DIR>/<jar.version_id>/` (created first if absent), so
/// the generator's own default output lands at `<cwd>/generated/reports/*.json`.
/// Skips the run if that reports directory already exists and is non-empty (TEST-D44
/// fast-path — no content hash to check here, since `--reports` output is
/// deterministic per jar). Requires `java` on `PATH`. Returns
/// `<repo_root>/<DATAGEN_OUTPUT_DIR>/<jar.version_id>/generated/reports/`.
pub fn run_data_reports(jar: &FetchedJar, repo_root: &Path) -> Result<PathBuf, FetchDataError> {
    let output_dir = repo_root.join(DATAGEN_OUTPUT_DIR).join(&jar.version_id);
    let reports_dir = output_dir.join("generated").join("reports");

    let already_populated = std::fs::read_dir(&reports_dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    if already_populated {
        return Ok(reports_dir);
    }

    std::fs::create_dir_all(&output_dir)?;

    let status = std::process::Command::new("java")
        .arg("-DbundlerMainClass=net.minecraft.data.Main")
        .arg("-jar")
        .arg(&jar.jar_path)
        .arg("--reports")
        .current_dir(&output_dir)
        .status()
        .map_err(|_| FetchDataError::JavaNotFound)?;

    if !status.success() {
        return Err(FetchDataError::ReportsFailed);
    }

    Ok(reports_dir)
}
