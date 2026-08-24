//! `xtask` — dev-only tooling binary for the Rusty Clanker workspace.
//!
//! This crate has a library target purely so `xtask/tests/*.rs` can exercise
//! the pure logic (`metadata`, `lint_deps::check_rules`) and CLI parsing
//! (`Cli`, `Command`) directly, without shelling out to the compiled binary.
//! `main.rs` is a thin wrapper dispatching on `Command` to each verb's `run()`.

pub mod datagen;
pub mod fetch_data;
pub mod fixture_manifest;
pub mod fmt_check;
pub mod forbidden_patterns;
pub mod lint;
pub mod lint_deps;
pub mod metadata;
pub mod path_guard;
pub mod quarantine;
pub mod setup_oracle;
pub mod test;
pub mod tier0;
pub mod tier1;
pub mod tier_result;
pub mod verifier_report;
pub mod verify_fixtures;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Command {
    /// cargo fmt --all -- --check
    FmtCheck,
    /// cargo clippy --workspace --all-targets -- -D warnings
    Lint,
    /// WS-D3 dependency-graph rule checker
    LintDeps,
    /// nextest (default features) + rusty-clanker-server monolithic + doctests
    Test,
    /// NET-D9: download (or accept via --server-jar) the pinned version's server.jar
    /// and run its --reports data generator locally, via M0-B08's shared
    /// `fetch_data` primitive (Context).
    FetchData {
        /// Minecraft version id, e.g. "26.2".
        version: String,
        #[arg(long)]
        server_jar: Option<std::path::PathBuf>,
        #[arg(long)]
        offline: bool,
    },
    /// NET-D9: read a prior FetchData run's cached reports (under `datagen-output/`,
    /// M0-B08's shared path) and emit generated Rust registry/block-state tables plus
    /// a TEST-D47 fixture manifest.
    Codegen {
        /// Minecraft version whose cached reports to read.
        #[arg(long, default_value = "26.2")]
        version: String,
        /// Output directory name suffix — protocol_version is an opaque, hand-bumped
        /// integer never derivable from --reports (docs/research/mc-26.2/00-source-
        /// overview.md's own note: "not derived from anything else... bumped by hand
        /// per release"), so it is a flag with NET-D1's current pin as its default,
        /// not something parsed out of the fetched data.
        #[arg(long, default_value_t = 776)]
        protocol_version: u32,
    },
    /// TEST-D47: recompute crates/registries/generated/v776/MANIFEST.json's hashes
    /// against the files on disk and fail on any mismatch.
    VerifyGenerated,
    /// TEST-D37 Tier 0: fmt-check + lint only, local convenience, never a CI gate
    Tier0,
    /// TEST-D37 Tier 1: every gate above plus path-guard, lint-tests, verify-fixtures
    Tier1 {
        #[arg(long)]
        base: Option<String>,
    },
    /// TEST-D46 CI path-guard
    PathGuard {
        #[arg(long)]
        base: Option<String>,
    },
    /// TEST-D49 forbidden-pattern lints
    LintTests {
        #[arg(long)]
        base: Option<String>,
    },
    /// TEST-D47 fixture-manifest integrity check (crates/testing/rc-golden-data)
    VerifyFixtures,
    /// TEST-D41 one-command oracle bootstrap
    SetupOracle {
        #[arg(long)]
        accept_eula: bool,
    },
    /// TEST-D51 quarantine a flaky test (auto-files a tracked issue)
    Quarantine {
        #[arg(long)]
        test: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        reason: String,
    },
    /// TEST-D51 list every currently-quarantined test
    ListQuarantine,
    /// TEST-D52 verifier-agent re-run entry point
    VerifierReport {
        #[arg(long)]
        base: Option<String>,
    },
}
