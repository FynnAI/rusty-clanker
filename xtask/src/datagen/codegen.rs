//! Pure codegen (`generate`) plus the CLI-facing I/O wrapper (`run`) for the `codegen`
//! verb. See Context's "Determinism" subsection for the four rules `generate` must
//! follow — restated as doc comments on the function itself below.

use super::reports::{BlocksReport, RegistriesReport};

/// `xtask`'s own crate version, tagged as this codegen format's identity — written
/// into every `MANIFEST.json` entry's `generator_tool_version` field.
pub const CODEGEN_TOOL_VERSION: &str = concat!("xtask-codegen/", env!("CARGO_PKG_VERSION"));

pub struct GeneratedFiles {
    /// `(relative filename under crates/registries/generated/v<protocol_version>/, content)`,
    /// in write order: `("registries.rs", ...)`, `("block_states.rs", ...)`.
    pub files: Vec<(String, String)>,
}

/// Pure transform: `--reports` data in, generated Rust source out. No filesystem
/// access. Deterministic per Context's four rules: parses/iterates only via `BTreeMap`
/// (never `HashMap`), sorts registry entries by `protocol_id` explicitly, embeds no
/// timestamp anywhere, and sanitizes identifiers as a pure function of the input
/// string alone. Two logically-identical `RegistriesReport`/`BlocksReport` values
/// (even if built via `.insert()` calls in different orders) MUST produce byte-
/// identical `GeneratedFiles::files` content — this is the property
/// `output_is_independent_of_input_insertion_order` (Acceptance tests) checks.
pub fn generate(registries: &RegistriesReport, blocks: &BlocksReport) -> GeneratedFiles {
    let _ = (registries, blocks);
    todo!()
}

pub struct CodegenArgs {
    /// Directory containing `registries.json`/`blocks.json` (a prior `fetch-data`
    /// run's `datagen-output/<version>/generated/reports/` — M0-B08's shared
    /// `fetch_data::run_data_reports`'s own return path, reused as-is).
    pub reports_dir: std::path::PathBuf,
    /// `crates/registries/generated/v<protocol_version>/` — created if absent.
    pub out_dir: std::path::PathBuf,
    pub source_jar_sha1: String,
    pub protocol_version: u32,
    pub mc_version: String,
}

/// I/O wrapper: reads `registries.json`+`blocks.json` from `args.reports_dir` (`Err`
/// naming the exact missing file and suggesting `cargo xtask fetch-data <version>` if
/// either is absent), calls `generate`, writes both files plus `MANIFEST.json` under
/// `args.out_dir`, then immediately calls `fixture_manifest::verify_manifest` against
/// what it just wrote as a self-check (defense against a write-time bug producing a
/// manifest that does not actually match the bytes on disk).
pub fn run(args: &CodegenArgs) -> Result<(), String> {
    let _ = args;
    todo!()
}
