use clap::Parser;
use xtask::{Cli, Command};

#[test]
fn parses_fetch_data_with_version_only() {
    let cli = Cli::try_parse_from(["xtask", "fetch-data", "26.2"]).unwrap();
    match cli.command {
        Command::FetchData {
            version,
            server_jar,
            offline,
        } => {
            assert_eq!(version, "26.2");
            assert_eq!(server_jar, None);
            assert!(!offline);
        }
        other => panic!("expected Command::FetchData, got {other:?}"),
    }
}

#[test]
fn parses_fetch_data_with_server_jar_flag() {
    let cli = Cli::try_parse_from([
        "xtask",
        "fetch-data",
        "26.2",
        "--server-jar",
        "C:/tmp/server.jar",
    ])
    .unwrap();
    match cli.command {
        Command::FetchData { server_jar, .. } => {
            assert_eq!(
                server_jar,
                Some(std::path::PathBuf::from("C:/tmp/server.jar"))
            );
        }
        other => panic!("expected Command::FetchData, got {other:?}"),
    }
}

#[test]
fn fetch_data_requires_version_argument() {
    assert!(Cli::try_parse_from(["xtask", "fetch-data"]).is_err());
}

#[test]
fn parses_codegen_with_defaults() {
    let cli = Cli::try_parse_from(["xtask", "codegen"]).unwrap();
    match cli.command {
        Command::Codegen {
            version,
            protocol_version,
        } => {
            assert_eq!(version, "26.2");
            assert_eq!(protocol_version, 776);
        }
        other => panic!("expected Command::Codegen, got {other:?}"),
    }
}

#[test]
fn parses_codegen_with_explicit_protocol_version() {
    let cli = Cli::try_parse_from(["xtask", "codegen", "--protocol-version", "777"]).unwrap();
    match cli.command {
        Command::Codegen {
            protocol_version, ..
        } => {
            assert_eq!(protocol_version, 777);
        }
        other => panic!("expected Command::Codegen, got {other:?}"),
    }
}

#[test]
fn parses_verify_generated() {
    let cli = Cli::try_parse_from(["xtask", "verify-generated"]).unwrap();
    assert_eq!(cli.command, Command::VerifyGenerated);
}
