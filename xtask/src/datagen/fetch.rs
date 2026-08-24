//! CLI-facing I/O wrapper for the `fetch-data` verb. Reuses M0-B08's shared
//! `crate::fetch_data::{fetch_server_jar, run_data_reports}` for every piece of
//! piston-meta resolution, jar download, and `--reports` invocation (Context) — this
//! file's own job is exactly the two CLI-level cases that shared primitive does not
//! cover (`--server-jar`, `--offline`) plus this verb's own Java-version check and
//! cross-process SHA-1 sidecar persistence.

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
    let _ = args;
    todo!()
}
