//! Subprocess entry point `xtask::m1_report` spawns for its own AC1b/AC1c cases.
//! Forced deviation from this blueprint's own Deliverables sketch (`m1_report.rs`
//! calling `rc_paritybot::idle_stability::run_idle_stability_scenario` directly,
//! in-process, inside a `tokio::runtime::Runtime`): `xtask`'s own binary must never
//! link `azalea` (see `Cargo.toml`'s own doc comment on this bin target for why) —
//! this binary is that isolation boundary, invoked as a real OS subprocess.
//!
//! Usage: `idle_stability_runner <host> <port> <username> <login_timeout_secs>
//! <idle_duration_secs>`. Prints a small line-based result to stdout — not JSON;
//! `rc-paritybot` deliberately carries no `serde_json` dependency (Constraint (c),
//! "no new external dependencies beyond the pinned set" restated for this crate):
//!
//! ```text
//! RESULT=OK
//! REACHED_LOGIN=<bool>
//! REACHED_SPAWN=<bool>
//! ```
//! or
//! ```text
//! RESULT=ERROR
//! MESSAGE=<single-line error description>
//! ```
//! Exit code 0 iff `RESULT=OK`.

use std::time::Duration;

use rc_paritybot::idle_stability::{ScenarioConfig, run_idle_stability_scenario};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [host, port, username, login_timeout_secs, idle_duration_secs] =
        match <[String; 5]>::try_from(args) {
            Ok(a) => a,
            Err(_) => {
                eprintln!(
                    "usage: idle_stability_runner <host> <port> <username> <login_timeout_secs> <idle_duration_secs>"
                );
                return std::process::ExitCode::FAILURE;
            }
        };

    let port: u16 = match port.parse() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("idle_stability_runner: invalid port {port:?}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let login_timeout = match login_timeout_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            eprintln!("idle_stability_runner: invalid login_timeout_secs: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let idle_duration = match idle_duration_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            eprintln!("idle_stability_runner: invalid idle_duration_secs: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut config = ScenarioConfig::new(host, port, username, idle_duration);
    config.login_timeout = login_timeout;

    match run_idle_stability_scenario(config).await {
        Ok(outcome) => {
            println!("RESULT=OK");
            println!("REACHED_LOGIN={}", outcome.reached_login);
            println!("REACHED_SPAWN={}", outcome.reached_spawn);
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            let message = err.to_string().replace('\n', " ");
            println!("RESULT=ERROR");
            println!("MESSAGE={message}");
            std::process::ExitCode::FAILURE
        }
    }
}
