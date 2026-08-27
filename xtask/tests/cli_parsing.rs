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

/// DEFECT 5's own CLI-surface regression test: `fetch-corpus` must expose the same
/// `--accept-eula` flag shape `setup-oracle` already has (TEST-D41).
#[test]
fn parses_fetch_corpus_with_accept_eula() {
    let cli = Cli::try_parse_from(["xtask", "fetch-corpus", "--accept-eula"]).unwrap();
    assert_eq!(
        cli.command,
        Command::FetchCorpus {
            version: "26.2".to_string(),
            server_jar: None,
            only: None,
            accept_eula: true,
        }
    );
}

#[test]
fn fetch_corpus_defaults_accept_eula_to_false() {
    let cli = Cli::try_parse_from(["xtask", "fetch-corpus"]).unwrap();
    assert_eq!(
        cli.command,
        Command::FetchCorpus {
            version: "26.2".to_string(),
            server_jar: None,
            only: None,
            accept_eula: false,
        }
    );
}
