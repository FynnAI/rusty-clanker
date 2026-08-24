//! `xtask` — dev-only tooling binary for the Rusty Clanker workspace.
//!
//! This crate has a library target purely so `xtask/tests/*.rs` can exercise
//! the pure logic (`metadata`, `lint_deps::check_rules`) and CLI parsing
//! (`Cli`, `Command`) directly, without shelling out to the compiled binary.
//! `main.rs` is a thin wrapper dispatching on `Command` to each verb's `run()`.

pub mod fmt_check;
pub mod lint;
pub mod lint_deps;
pub mod metadata;
pub mod test;

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
}
