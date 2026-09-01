//! `placement-diff`'s own CLI-parsing self-tests (governance changeset, "M3
//! field-report harness") — mirrors `cli_parsing.rs`'s own established
//! `Cli::try_parse_from` idiom for `fetch-corpus`/`parity-check`, plus
//! `corpus::placement_diff::Side::parse`'s own small pure parser.

use clap::Parser;
use xtask::corpus::placement_diff::Side;
use xtask::{Cli, Command};

#[test]
fn parses_with_required_server_bin_only() {
    let cli = Cli::try_parse_from([
        "xtask",
        "placement-diff",
        "--server-bin",
        "target/release/rusty-clanker-server",
    ])
    .unwrap();
    assert_eq!(
        cli.command,
        Command::PlacementDiff {
            version: "26.2".to_string(),
            server_jar: None,
            server_bin: "target/release/rusty-clanker-server".into(),
            only: None,
            side: "both".to_string(),
            accept_eula: false,
        }
    );
}

#[test]
fn missing_server_bin_is_a_parse_error() {
    let result = Cli::try_parse_from(["xtask", "placement-diff"]);
    assert!(
        result.is_err(),
        "--server-bin is required but parsing succeeded"
    );
}

#[test]
fn parses_every_flag() {
    let cli = Cli::try_parse_from([
        "xtask",
        "placement-diff",
        "--version",
        "26.2",
        "--server-jar",
        "C:/oracle/server.jar",
        "--server-bin",
        "target/release/rusty-clanker-server.exe",
        "--only",
        "hopper/dir_north/face_top_of_floor/pitch_level",
        "--side",
        "oracle",
        "--accept-eula",
    ])
    .unwrap();
    assert_eq!(
        cli.command,
        Command::PlacementDiff {
            version: "26.2".to_string(),
            server_jar: Some("C:/oracle/server.jar".into()),
            server_bin: "target/release/rusty-clanker-server.exe".into(),
            only: Some("hopper/dir_north/face_top_of_floor/pitch_level".to_string()),
            side: "oracle".to_string(),
            accept_eula: true,
        }
    );
}

#[test]
fn side_parse_accepts_the_three_known_values() {
    assert_eq!(Side::parse("oracle"), Ok(Side::Oracle));
    assert_eq!(Side::parse("ours"), Ok(Side::Ours));
    assert_eq!(Side::parse("both"), Ok(Side::Both));
}

#[test]
fn side_parse_rejects_anything_else() {
    assert!(Side::parse("bogus").is_err());
    assert!(Side::parse("").is_err());
    assert!(
        Side::parse("Both").is_err(),
        "case-sensitive, matching every other xtask enum-like flag"
    );
}
