//! `xtask protocol-diff --diff-only`'s own CLI-parsing self-tests (M3.5-B03
//! governance changeset, TEST-D58 Deliverable 1) — mirrors
//! `protocol_diff_capture_deadline_cli.rs`'s own established `Cli::try_parse_from`
//! idiom. TEST-D45: written before the implementation changeset that adds the
//! `diff_only`/`oracle_capture`/`ours_capture` fields (and `server_bin`'s own
//! `PathBuf` -> `Option<PathBuf>` widening) themselves.

use clap::Parser;
use xtask::{Cli, Command};

#[test]
fn diff_only_parses_with_both_capture_paths_and_no_server_bin() {
    let cli = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "captures/oracle/protocol-diff-oracle.postcard",
        "--ours-capture",
        "captures/ours/protocol-diff-ours.postcard",
    ])
    .unwrap();
    match cli.command {
        Command::ProtocolDiff {
            diff_only,
            oracle_capture,
            ours_capture,
            server_bin,
            ..
        } => {
            assert!(diff_only);
            assert_eq!(
                oracle_capture,
                Some("captures/oracle/protocol-diff-oracle.postcard".into())
            );
            assert_eq!(
                ours_capture,
                Some("captures/ours/protocol-diff-ours.postcard".into())
            );
            assert_eq!(server_bin, None, "--diff-only never needs --server-bin");
        }
        other => panic!("expected Command::ProtocolDiff, got {other:?}"),
    }
}

#[test]
fn diff_only_missing_oracle_capture_is_a_parse_error() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--ours-capture",
        "b.postcard",
    ]);
    assert!(
        result.is_err(),
        "--diff-only without --oracle-capture parsed successfully"
    );
}

#[test]
fn diff_only_missing_ours_capture_is_a_parse_error() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "a.postcard",
    ]);
    assert!(
        result.is_err(),
        "--diff-only without --ours-capture parsed successfully"
    );
}

#[test]
fn diff_only_with_neither_capture_path_is_a_parse_error() {
    let result = Cli::try_parse_from(["xtask", "protocol-diff", "--diff-only"]);
    assert!(
        result.is_err(),
        "--diff-only with no capture paths at all parsed successfully"
    );
}

#[test]
fn diff_only_conflicts_with_side() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "a.postcard",
        "--ours-capture",
        "b.postcard",
        "--side",
        "oracle",
    ]);
    assert!(
        result.is_err(),
        "--diff-only combined with --side parsed successfully"
    );
}

#[test]
fn diff_only_conflicts_with_server_bin() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "a.postcard",
        "--ours-capture",
        "b.postcard",
        "--server-bin",
        "target/release/rusty-clanker-server",
    ]);
    assert!(
        result.is_err(),
        "--diff-only combined with --server-bin parsed successfully"
    );
}

#[test]
fn diff_only_conflicts_with_server_jar() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "a.postcard",
        "--ours-capture",
        "b.postcard",
        "--server-jar",
        "C:/oracle/server.jar",
    ]);
    assert!(
        result.is_err(),
        "--diff-only combined with --server-jar parsed successfully"
    );
}

#[test]
fn diff_only_conflicts_with_accept_eula() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "a.postcard",
        "--ours-capture",
        "b.postcard",
        "--accept-eula",
    ]);
    assert!(
        result.is_err(),
        "--diff-only combined with --accept-eula parsed successfully"
    );
}

#[test]
fn diff_only_conflicts_with_only() {
    let result = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "a.postcard",
        "--ours-capture",
        "b.postcard",
        "--only",
        "session/spawn",
    ]);
    assert!(
        result.is_err(),
        "--diff-only combined with --only parsed successfully"
    );
}

#[test]
fn diff_only_still_allows_capture_deadline_secs_and_debug_hooks() {
    // Neither flag is capture-driving on its own — `--capture-deadline-secs` and
    // `--debug-hooks` are simply irrelevant in `--diff-only` mode, not conflicting.
    let cli = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--diff-only",
        "--oracle-capture",
        "a.postcard",
        "--ours-capture",
        "b.postcard",
        "--capture-deadline-secs",
        "5400",
    ]);
    assert!(
        cli.is_ok(),
        "--diff-only combined with --capture-deadline-secs should parse"
    );
}

#[test]
fn without_diff_only_the_new_fields_default_to_none_and_false() {
    let cli = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--server-bin",
        "target/release/rusty-clanker-server",
    ])
    .unwrap();
    match cli.command {
        Command::ProtocolDiff {
            diff_only,
            oracle_capture,
            ours_capture,
            ..
        } => {
            assert!(!diff_only);
            assert_eq!(oracle_capture, None);
            assert_eq!(ours_capture, None);
        }
        other => panic!("expected Command::ProtocolDiff, got {other:?}"),
    }
}

#[test]
fn side_oracle_no_longer_needs_server_bin() {
    // TEST-D58 §3.10's own `protocol-capture-oracle` job invocation never passes
    // `--server-bin` at all — `server_bin` widened from `PathBuf` to
    // `Option<PathBuf>` specifically to allow this.
    let cli = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--side",
        "oracle",
        "--accept-eula",
        "--capture-deadline-secs",
        "5400",
    ]);
    assert!(
        cli.is_ok(),
        "--side oracle without --server-bin should parse"
    );
}

#[test]
fn side_both_still_parses_with_server_bin_given() {
    let cli = Cli::try_parse_from([
        "xtask",
        "protocol-diff",
        "--server-bin",
        "target/release/rusty-clanker-server",
        "--accept-eula",
    ])
    .unwrap();
    match cli.command {
        Command::ProtocolDiff { server_bin, .. } => {
            assert_eq!(
                server_bin,
                Some("target/release/rusty-clanker-server".into())
            );
        }
        other => panic!("expected Command::ProtocolDiff, got {other:?}"),
    }
}
