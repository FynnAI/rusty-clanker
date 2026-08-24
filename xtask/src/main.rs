use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use xtask::datagen::{codegen, fetch};
use xtask::{Cli, Command, fixture_manifest};

/// Mirrors `crate::fetch_data`'s and `datagen::fetch::run`'s own identical,
/// independent resolution of the workspace root — each of the three call sites
/// computes this the same way rather than one calling into another's internals.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::FmtCheck => xtask::fmt_check::run(),
        Command::Lint => xtask::lint::run(),
        Command::LintDeps => xtask::lint_deps::run(),
        Command::Test => xtask::test::run(),
        Command::FetchData {
            version,
            server_jar,
            offline,
        } => {
            let args = fetch::FetchArgs {
                version,
                server_jar,
                offline,
            };
            match fetch::run(&args) {
                Ok(outcome) => {
                    println!(
                        "fetch-data: reports at {} (jar sha1 {})",
                        outcome.reports_dir.display(),
                        outcome.jar_sha1
                    );
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("fetch-data: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Codegen {
            version,
            protocol_version,
        } => {
            let root = repo_root();
            let reports_dir = root
                .join(xtask::fetch_data::DATAGEN_OUTPUT_DIR)
                .join(&version)
                .join("generated")
                .join("reports");
            let out_dir = root
                .join("crates/registries/generated")
                .join(format!("v{protocol_version}"));
            let sha1_path = root
                .join(xtask::fetch_data::ORACLE_JAR_DIR)
                .join(&version)
                .join("server.jar.sha1");
            let source_jar_sha1 = match std::fs::read_to_string(&sha1_path) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!(
                        "codegen: missing {} — run `cargo xtask fetch-data {version}` first",
                        sha1_path.display()
                    );
                    return ExitCode::FAILURE;
                }
            };
            let args = codegen::CodegenArgs {
                reports_dir,
                out_dir,
                source_jar_sha1,
                protocol_version,
                mc_version: version,
            };
            match codegen::run(&args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("codegen: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::VerifyGenerated => {
            let root = repo_root();
            let out_dir = root.join("crates/registries/generated/v776");
            let manifest_path = out_dir.join("MANIFEST.json");
            let violations = fixture_manifest::verify_manifest(&manifest_path, &out_dir);
            if violations.is_empty() {
                println!("verify-generated: OK");
                ExitCode::SUCCESS
            } else {
                for violation in &violations {
                    eprintln!(
                        "verify-generated: {} [{}]: {}",
                        violation.path, violation.kind, violation.message
                    );
                }
                ExitCode::FAILURE
            }
        }
    }
}
