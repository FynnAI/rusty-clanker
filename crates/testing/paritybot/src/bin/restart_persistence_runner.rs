//! Subprocess entry point `xtask::m2_report` spawns for its own AC1 cases. Forced
//! deviation from this blueprint's own Deliverables sketch (`m2_report.rs` calling
//! `rc_paritybot::restart_persistence::apply_actions`/`observe_state` directly,
//! in-process, inside a `tokio::runtime::Runtime`) — identical reasoning and identical
//! resolution to `idle_stability_runner.rs`'s own doc comment: `xtask`'s own binary must
//! never link `azalea`.
//!
//! Usage: `restart_persistence_runner <apply|observe> <host> <port> <username>
//! <login_timeout_secs>`. Prints a small line-based result to stdout (no `serde_json`
//! dependency in this crate, Constraint (c), mirroring `idle_stability_runner`'s own
//! identical convention):
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
//! `observe` mode additionally prints, on success, one `BLOCK=<x>,<y>,<z>,<raw_id>` line
//! per observed test position (Context's fixed 5-position table) followed by one
//! `HEALTH=<f32>` line, all before the trailing `RESULT=OK`.
//!
//! Exit code 0 iff `RESULT=OK`.
//!
//! M2 field-report AC3 fix: a third mode, `churn`, added for `xtask::m2_report`'s own cadence
//! leg (`finish_after_cadence`) — `rc_paritybot::restart_persistence::churn`'s own doc comment
//! has the full "why" (keeping one chunk continuously dirty across the whole cadence
//! observation window, which the previous "re-run `apply` periodically" driver never actually
//! did). Usage: `restart_persistence_runner churn <host> <port> <username>
//! <login_timeout_secs> <duration_secs> <period_ms>`. On success, prints:
//! ```text
//! RESULT=OK
//! TOGGLE_COUNT=<u64>
//! DURATION_MS=<u128>
//! ```
//! or, identically to `apply`/`observe`, `RESULT=ERROR`/`MESSAGE=<single-line error
//! description>` on failure. Exit code 0 iff `RESULT=OK`, exactly like the other two modes.

use std::time::Duration;

use rc_paritybot::restart_persistence::{apply_actions, churn, observe_state};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // `churn`'s own arg count (7, including the mode itself) differs from `apply`/`observe`'s
    // shared 5 — intercepted here, before the shared 5-arg destructuring below, so that
    // destructuring (and everything that follows it) stays byte-identical to its pre-`churn`
    // behavior for both of the modes it already handled.
    if args.first().map(String::as_str) == Some("churn") {
        return run_churn(&args).await;
    }

    let [mode, host, port, username, login_timeout_secs] = match <[String; 5]>::try_from(args) {
        Ok(a) => a,
        Err(_) => {
            eprintln!(
                "usage: restart_persistence_runner <apply|observe> <host> <port> <username> <login_timeout_secs>"
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let port: u16 = match port.parse() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("restart_persistence_runner: invalid port {port:?}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let login_timeout = match login_timeout_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            eprintln!("restart_persistence_runner: invalid login_timeout_secs: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match mode.as_str() {
        "apply" => match apply_actions(&host, port, &username, login_timeout).await {
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
        "observe" => match observe_state(&host, port, &username, login_timeout).await {
            Ok(state) => {
                for (pos, raw_id) in &state.blocks {
                    println!("BLOCK={},{},{},{raw_id}", pos.x, pos.y, pos.z);
                }
                println!("HEALTH={}", state.health);
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
                "restart_persistence_runner: unrecognized mode {other:?} (expected apply|observe)"
            );
            std::process::ExitCode::FAILURE
        }
    }
}

/// `churn` mode's own entry point (module doc comment above has the full usage/output
/// contract) — kept separate from, and checked for ahead of, `main`'s own shared `apply`/
/// `observe` 5-arg destructuring so that destructuring's behavior for those two modes stays
/// untouched.
async fn run_churn(args: &[String]) -> std::process::ExitCode {
    let [
        _mode,
        host,
        port,
        username,
        login_timeout_secs,
        duration_secs,
        period_ms,
    ] = match <[String; 7]>::try_from(args.to_vec()) {
        Ok(a) => a,
        Err(_) => {
            eprintln!(
                "usage: restart_persistence_runner churn <host> <port> <username> <login_timeout_secs> <duration_secs> <period_ms>"
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let port: u16 = match port.parse() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("restart_persistence_runner: invalid port {port:?}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let login_timeout = match login_timeout_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            eprintln!("restart_persistence_runner: invalid login_timeout_secs: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let duration = match duration_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            eprintln!("restart_persistence_runner: invalid duration_secs: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let period = match period_ms.parse::<u64>() {
        Ok(ms) => Duration::from_millis(ms),
        Err(err) => {
            eprintln!("restart_persistence_runner: invalid period_ms: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match churn(&host, port, &username, login_timeout, duration, period).await {
        Ok(summary) => {
            println!("RESULT=OK");
            println!("TOGGLE_COUNT={}", summary.toggle_count);
            println!("DURATION_MS={}", summary.duration.as_millis());
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            println!("RESULT=ERROR");
            println!("MESSAGE={}", err.to_string().replace('\n', " "));
            std::process::ExitCode::FAILURE
        }
    }
}
