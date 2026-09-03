//! `xtask protocol-diff --capture-deadline-secs`'s own CLI-parsing self-test (M3.5-B03
//! governance changeset, "protocol-diff-runner progress lines" Deliverable 4) —
//! mirrors `placement_diff_cli_parsing.rs`'s own established `Cli::try_parse_from`
//! idiom. TEST-D45: written before the implementation changeset that adds the
//! `capture_deadline_secs` field itself.

use clap::Parser;
use xtask::{Cli, Command};

#[test]
fn capture_deadline_secs_defaults_to_none() {
    let cli = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--server-bin",
        "target/release/rusty-clanker-server",
    ])
    .unwrap();
    match cli.command {
        Command::ProtocolDiff {
            capture_deadline_secs,
            ..
        } => assert_eq!(capture_deadline_secs, None),
        other => panic!("expected Command::ProtocolDiff, got {other:?}"),
    }
}

#[test]
fn capture_deadline_secs_parses_when_given() {
    let cli = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--server-bin",
        "target/release/rusty-clanker-server",
        "--capture-deadline-secs",
        "5400",
    ])
    .unwrap();
    match cli.command {
        Command::ProtocolDiff {
            capture_deadline_secs,
            ..
        } => assert_eq!(capture_deadline_secs, Some(5400)),
        other => panic!("expected Command::ProtocolDiff, got {other:?}"),
    }
}

#[test]
fn a_non_numeric_capture_deadline_secs_is_a_parse_error() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--server-bin",
        "target/release/rusty-clanker-server",
        "--capture-deadline-secs",
        "soon",
    ]);
    assert!(
        result.is_err(),
        "non-numeric --capture-deadline-secs parsed successfully"
    );
}
