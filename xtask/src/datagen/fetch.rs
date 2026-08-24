//! CLI-facing I/O wrapper for the `fetch-data` verb. Reuses M0-B08's shared
//! `crate::fetch_data::{fetch_server_jar, run_data_reports}` for every piece of
//! piston-meta resolution, jar download, and `--reports` invocation (Context) — this
//! file's own job is exactly the two CLI-level cases that shared primitive does not
//! cover (`--server-jar`, `--offline`) plus this verb's own Java-version check and
//! cross-process SHA-1 sidecar persistence.

use std::path::PathBuf;

use super::java_check;
use crate::fetch_data::{self, FetchedJar};

pub struct FetchArgs {
    pub version: String,
    /// Use this already-downloaded jar instead of letting `fetch_server_jar`
    /// download one. When not `offline`, its bytes are copied into
    /// `fetch_data::ORACLE_JAR_DIR`'s expected cache path first, so
    /// `fetch_server_jar`'s own already-exists-and-hash-matches fast path still
    /// verifies it against Mojang's declared SHA-1 (Context).
    pub server_jar: Option<std::path::PathBuf>,
    /// Skip all network access. Requires `server_jar`. SHA-1 verification against
    /// Mojang's declared value and the live per-version Java-requirement lookup are
    /// both skipped (a warning is printed for each); `java_check::FALLBACK_MIN_JAVA_MAJOR`
    /// is used as the Java-version floor instead.
    pub offline: bool,
}

pub struct FetchOutcome {
    /// `datagen-output/<version>/generated/reports/` — `fetch_data::run_data_reports`'s
    /// own return path, exactly what `codegen`'s `reports_dir` argument should point
    /// at.
    pub reports_dir: std::path::PathBuf,
    /// SHA-1 (lowercase hex) of the jar actually used — feeds `codegen`'s
    /// `source_jar_sha1` provenance field regardless of whether it was downloaded,
    /// supplied via `--server-jar`, or hashed locally under `--offline`. `run` also
    /// persists this value to an `oracle/<version>/server.jar.sha1` sidecar text file
    /// (no trailing newline), since a later, separate `xtask codegen` process
    /// invocation cannot read this in-memory struct across the process boundary and
    /// must not re-hash the (possibly large) jar just to recover it.
    pub jar_sha1: String,
}

/// Mirrors `crate::fetch_data`'s own identical, independent resolution of the
/// workspace root — both modules compute this the same way rather than one calling
/// into the other's private internals.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
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

/// Copies `source`'s bytes to `dest` (creating parent directories as needed) unless
/// `source` already *is* `dest`. Used by both the online `--server-jar` case and the
/// `--offline` case to place a locally-supplied jar at `fetch_data::ORACLE_JAR_DIR`'s
/// expected cache path.
fn copy_into_cache(source: &std::path::Path, dest: &std::path::Path) -> Result<Vec<u8>, String> {
    let bytes =
        std::fs::read(source).map_err(|e| format!("failed to read {}: {e}", source.display()))?;
    if source != dest {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        std::fs::write(dest, &bytes)
            .map_err(|e| format!("failed to write {}: {e}", dest.display()))?;
    }
    Ok(bytes)
}

/// Orchestrates the full verb by calling `crate::fetch_data::fetch_server_jar` /
/// `crate::fetch_data::run_data_reports` for every ordinary (non-offline) case
/// (Context) — never re-resolving piston-meta or re-invoking `--reports` itself.
/// Concrete error cases and their exact actionable messages (Implementation steps
/// below give the precise wording for each): `offline` given without `server_jar`;
/// `server_jar` given but the path does not exist; any `fetch_data::FetchDataError`
/// the shared primitive itself produces (version not found, network failure, hash
/// mismatch, java not found, `--reports` failure), surfaced via its own `Display`
/// impl; local Java below the required major version (`check_java`, this
/// blueprint's own module, run against whichever `min_java_major` the online or
/// offline path produced).
pub fn run(args: &FetchArgs) -> Result<FetchOutcome, String> {
    if args.offline && args.server_jar.is_none() {
        return Err(
            "--offline requires --server-jar <path> (nothing to run --reports against \
             without either a download or a supplied jar)"
                .to_string(),
        );
    }
    if let Some(path) = &args.server_jar
        && !path.exists()
    {
        return Err(format!(
            "--server-jar path '{}' does not exist. Either omit --server-jar to let \
             fetch-data download it automatically, or supply the correct path to a \
             legally-obtained server.jar for Minecraft {} (protocol 776 at time of writing).",
            path.display(),
            args.version
        ));
    }

    let root = repo_root();
    let cache_jar_path = root
        .join(fetch_data::ORACLE_JAR_DIR)
        .join(&args.version)
        .join("server.jar");

    let fetched = if args.offline {
        // Unwrap is safe: checked above that --offline implies server_jar.is_some().
        let server_jar = args.server_jar.as_ref().expect("checked above");
        let jar_bytes = copy_into_cache(server_jar, &cache_jar_path)?;
        eprintln!(
            "fetch-data: --offline given — skipping SHA-1 verification against Mojang's \
             manifest and skipping the live per-version Java-requirement lookup; using the \
             supplied jar's own locally-computed hash for provenance and \
             java_check::FALLBACK_MIN_JAVA_MAJOR ({}) as the Java-version floor.",
            java_check::FALLBACK_MIN_JAVA_MAJOR
        );
        FetchedJar {
            jar_path: cache_jar_path.clone(),
            version_id: args.version.clone(),
            sha1: sha1_hex(&jar_bytes),
            min_java_major: java_check::FALLBACK_MIN_JAVA_MAJOR,
        }
    } else {
        if let Some(server_jar) = &args.server_jar {
            copy_into_cache(server_jar, &cache_jar_path)?;
        }
        fetch_data::fetch_server_jar(&args.version, &root).map_err(|e| e.to_string())?
    };

    java_check::check_java(fetched.min_java_major)?;

    let reports_dir = fetch_data::run_data_reports(&fetched, &root).map_err(|e| e.to_string())?;

    let sha1_sidecar_path = root
        .join(fetch_data::ORACLE_JAR_DIR)
        .join(&args.version)
        .join("server.jar.sha1");
    std::fs::write(&sha1_sidecar_path, &fetched.sha1)
        .map_err(|e| format!("failed to write {}: {e}", sha1_sidecar_path.display()))?;

    Ok(FetchOutcome {
        reports_dir,
        jar_sha1: fetched.sha1,
    })
}
