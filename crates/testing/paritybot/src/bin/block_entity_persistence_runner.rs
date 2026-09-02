//! Subprocess entry point `xtask::m3_5_be_report` spawns for its two live-bot legs.
//! Forced deviation from an in-process call (identical reasoning to
//! `restart_persistence_runner.rs`'s own doc comment): `xtask`'s own binary must
//! never link `azalea`.
//!
//! Usage: `block_entity_persistence_runner <apply|observe> <host> <port>
//! <login_timeout_secs>`. Prints a small line-based result to stdout (mirrors
//! `restart_persistence_runner`'s own identical convention, no `serde_json`
//! dependency in this crate):
//!
//! `apply` mode:
//! ```text
//! RESULT=OK
//! ```
//! or
//! ```text
//! RESULT=ERROR
//! MESSAGE=<single-line error description>
//! ```
//!
//! `observe` mode additionally prints, on success, one
//! `POS=<x>,<y>,<z>,<state_id_or_blank>,<has_block_entity: 0|1>` line per observed
//! test position, before the trailing `RESULT=OK`.
//!
//! Exit code 0 iff `RESULT=OK`.

use std::time::Duration;

use rc_paritybot::block_entity_persistence::{apply_placements, observe_presence};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [mode, host, port, login_timeout_secs] = match <[String; 4]>::try_from(args) {
        Ok(a) => a,
        Err(_) => {
            eprintln!(
                "usage: block_entity_persistence_runner <apply|observe> <host> <port> <login_timeout_secs>"
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let port: u16 = match port.parse() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("block_entity_persistence_runner: invalid port {port:?}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let login_timeout = match login_timeout_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            eprintln!("block_entity_persistence_runner: invalid login_timeout_secs: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match mode.as_str() {
        "apply" => match apply_placements(&host, port, login_timeout).await {
            Ok(()) => {
                println!("RESULT=OK");
                std::process::ExitCode::SUCCESS
            }
            Err(err) => {
                println!("RESULT=ERROR");
                println!("MESSAGE={}", err.to_string().replace('\n', " "));
                std::process::ExitCode::FAILURE
            }
        },
        "observe" => match observe_presence(&host, port, login_timeout).await {
            Ok(observations) => {
                for observed in &observations {
                    let state = observed
                        .state_id
                        .map(|id| id.to_string())
                        .unwrap_or_default();
                    println!(
                        "POS={},{},{},{state},{}",
                        observed.pos.0,
                        observed.pos.1,
                        observed.pos.2,
                        observed.has_block_entity as u8
                    );
                }
                println!("RESULT=OK");
                std::process::ExitCode::SUCCESS
            }
            Err(err) => {
                println!("RESULT=ERROR");
                println!("MESSAGE={}", err.to_string().replace('\n', " "));
                std::process::ExitCode::FAILURE
            }
        },
        other => {
            eprintln!(
                "block_entity_persistence_runner: unrecognized mode {other:?} (expected apply|observe)"
            );
            std::process::ExitCode::FAILURE
        }
    }
}
