use clap::Parser;
use xtask::{Cli, Command};

#[test]
fn parses_fmt_check() {
    let cli = Cli::try_parse_from(["xtask", "fmt-check"]).unwrap();
    assert_eq!(cli.command, Command::FmtCheck);
}

#[test]
fn parses_lint() {
    let cli = Cli::try_parse_from(["xtask", "lint"]).unwrap();
    assert_eq!(cli.command, Command::Lint);
}

#[test]
fn parses_lint_deps() {
    let cli = Cli::try_parse_from(["xtask", "lint-deps"]).unwrap();
    assert_eq!(cli.command, Command::LintDeps);
}

#[test]
fn parses_test() {
    let cli = Cli::try_parse_from(["xtask", "test"]).unwrap();
    assert_eq!(cli.command, Command::Test);
}

#[test]
fn rejects_unknown_verb() {
    assert!(Cli::try_parse_from(["xtask", "not-a-real-verb"]).is_err());
}
