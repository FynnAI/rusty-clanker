//! `xtask` — dev-only tooling binary for the Rusty Clanker workspace.
//!
//! This crate has a library target purely so `xtask/tests/*.rs` can exercise
//! the pure logic (`metadata`, `lint_deps::check_rules`) and CLI parsing
//! (`Cli`, `Command`) directly, without shelling out to the compiled binary.
//! `main.rs` is a thin wrapper dispatching on `Command` to each verb's `run()`.

pub mod case_matrix;
pub mod claims_gate;
pub mod corpus;
pub mod datagen;
pub mod fetch_data;
pub mod fixture_manifest;
pub mod fmt_check;
pub mod forbidden_patterns;
pub mod lint;
pub mod lint_deps;
pub mod m1_report;
pub mod m2_report;
pub mod m3_report;
pub mod metadata;
pub mod path_guard;
pub mod quarantine;
pub mod setup_oracle;
pub mod spec_citation;
pub mod test;
pub mod tier0;
pub mod tier1;
pub mod tier_result;
pub mod verifier_report;
pub mod verify_claims;
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
    /// M1 registry-sync fix follow-up (docs/research/mc-26.2/26-registry-sync-configuration.md
    /// §5.3/§6): reads a local data-generator "generated" tree's own `data/minecraft/tags/**`
    /// JSON (never committed — ASSET-D18(f)'s carve-out only covers the derived output) plus a
    /// prior FetchData run's cached `registries.json`, and emits
    /// `crates/registries/generated/v<protocol_version>/tags.rs` plus a merged MANIFEST.json
    /// entry.
    CodegenTags {
        /// Directory containing `data/minecraft/tags/**` — typically a full (`--all`) local
        /// data-generator run's own `generated/` output.
        #[arg(long)]
        tags_dir: std::path::PathBuf,
        /// Minecraft version whose cached `registries.json` to read.
        #[arg(long, default_value = "26.2")]
        version: String,
        #[arg(long, default_value_t = 776)]
        protocol_version: u32,
    },
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
    /// M1-B06: drives the M1 acceptance harness against a real, freshly-spawned
    /// `rusty-clanker-server` and writes `target/verify/m1-acceptance.json`.
    M1Report {
        #[arg(long)]
        server_bin: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = m1_report::Mode::Smoke)]
        mode: m1_report::Mode,
    },
    /// M2-B08: drives the M2 acceptance harness (restart round-trip + save-cadence
    /// legs) against a real, freshly-spawned `rusty-clanker-server` and writes
    /// `target/verify/m2-acceptance.json`.
    M2Report {
        #[arg(long)]
        server_bin: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = m2_report::Mode::Smoke)]
        mode: m2_report::Mode,
    },
    /// M3-B07: capture the redstone parity corpus from a real, legally obtained
    /// vanilla oracle server into the git-ignored `corpus/redstone/` trace cache
    /// (WS-D10). Never a Tier-1 gate (Context, "CI tier placement").
    FetchCorpus {
        #[arg(long, default_value = "26.2")]
        version: String,
        #[arg(long)]
        server_jar: Option<std::path::PathBuf>,
        /// Restrict to one contraption id, for local iteration.
        #[arg(long)]
        only: Option<String>,
        /// TEST-D41 legal consent, same flag shape as `setup-oracle --accept-eula` — this
        /// verb launches the same real vanilla oracle jar and is bound by the identical
        /// consent gate (`setup_oracle::consent_already_given`).
        #[arg(long)]
        accept_eula: bool,
    },
    /// M3-B07: `xtask parity-check <corpus>` — this blueprint wires exactly the
    /// `"redstone"` corpus (WS-D9 already reserves the verb shape for a future
    /// `"worldgen"` corpus too, added by whichever M5 blueprint needs it, not this
    /// one).
    ParityCheck {
        corpus: String,
        #[arg(long)]
        only: Option<String>,
    },
    /// M3-B08: drives the M3 acceptance harness (redstone corpus parity + 20-bot
    /// single-region load leg) against a real, freshly-spawned `rusty-clanker-server`
    /// and a real oracle, and writes `target/verify/m3-acceptance.json`.
    M3Report {
        #[arg(long)]
        server_bin: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = m3_report::Mode::Smoke)]
        mode: m3_report::Mode,
    },
    /// M3 field-report harness (governance changeset): drives every tier-1 placeable
    /// block kind through the real client -> server `UseItemOn`/creative-hotbar
    /// packet path (never the redstone corpus's own oracle-pre-resolved,
    /// Stage-4-direct replay) against both a real vanilla oracle and our own real
    /// `rusty-clanker-server`, diffs the resulting block states, and writes
    /// `target/verify/placement-diff.json`.
    PlacementDiff {
        #[arg(long, default_value = "26.2")]
        version: String,
        #[arg(long)]
        server_jar: Option<std::path::PathBuf>,
        #[arg(long)]
        server_bin: std::path::PathBuf,
        /// Restrict to one scenario id, for local iteration.
        #[arg(long)]
        only: Option<String>,
        /// `oracle` (capture/cache only, no diff), `ours` (capture only, no diff), or
        /// `both` (capture both sides and diff) — default `both`.
        #[arg(long, default_value = "both")]
        side: String,
        /// TEST-D41 legal consent, same flag shape as `xtask setup-oracle
        /// --accept-eula`/`xtask fetch-corpus --accept-eula` — this verb launches the
        /// same real vanilla oracle jar and is bound by the identical consent gate
        /// whenever `--side` includes the oracle.
        #[arg(long)]
        accept_eula: bool,
    },
    /// TEST-D57: exact-count CLAIMS.md audit for one milestone's blueprints.
    VerifyClaims { milestone: String },
}
