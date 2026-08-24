use std::process::ExitCode;

use clap::Parser;
use xtask::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::FmtCheck => xtask::fmt_check::run(),
        Command::Lint => xtask::lint::run(),
        Command::LintDeps => xtask::lint_deps::run(),
        Command::Test => xtask::test::run(),
    }
}
