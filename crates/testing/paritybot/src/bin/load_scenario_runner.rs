//! Subprocess entry point `xtask::m3_report` spawns for its own 20-bot single-region
//! load-test leg. Forced deviation from this blueprint's own Deliverables sketch
//! (`m3_report.rs` calling `rc_paritybot::load_scenario::run_load_scenario` directly,
//! in-process, inside a `tokio::runtime::Runtime`): `xtask`'s own binary must never
//! link `azalea` (see `Cargo.toml`'s own doc comment on this bin target for why) —
//! this binary is that isolation boundary, invoked as a real OS subprocess, identical
//! in shape to `idle_stability_runner`/`restart_persistence_runner`/
//! `fetch_corpus_runner`.
//!
//! Usage: `load_scenario_runner <host> <port> <login_timeout_secs>
//! <run_duration_secs>`. Bot count/arena layout/interaction cadence are never CLI
//! parameters — Context's own "compressed... never means... a shrunk bot count... a
//! different interaction rate... a smaller arena" rule means every run always uses
//! `load_scenario`'s own fixed constants (`COLS`/`ROWS`/`ARENA_MIN`/`ARENA_MAX`/
//! `BASE_Y`) unconditionally. Prints a small line-based result to stdout — not JSON;
//! `rc-paritybot` deliberately carries no `serde_json` dependency (Constraint (c),
//! restated from `idle_stability_runner`'s own identical convention):
//!
//! ```text
//! BOT_USERNAME=<username>
//! BOT_RESULT=OK
//! BOT_REACHED_SPAWN=<bool>
//! BOT_WAYPOINTS=<u64>
//! BOT_INTERACTIONS=<u64>
//! BOT_DISCONNECTED_AT_MS=<u64|none>
//! BOT_DETAIL=<single-line text, possibly empty>
//! ... (repeated per bot, in plan order)
//! RESULT=OK
//! ```
//! or, for a login-phase failure on one particular bot:
//! ```text
//! BOT_USERNAME=<username>
//! BOT_RESULT=ERROR
//! BOT_DETAIL=<single-line error description>
//! ```
//! (no further `BOT_*` lines for that bot), or, for a failure before the scenario
//! could even start:
//! ```text
//! RESULT=ERROR
//! MESSAGE=<single-line error description>
//! ```
//! Exit code 0 iff `RESULT=OK` was reached (i.e. the scenario itself ran to
//! completion) — individual bot outcomes are data for the caller to aggregate
//! (`xtask::m3_report::build_report`'s own AC2 cases), never this binary's own
//! success/failure verdict, mirroring `fetch_corpus_runner`'s identical "RESULT=OK
//! even when some per-item outcome failed" convention.

use std::time::Duration;

use rc_paritybot::load_scenario::{
    ARENA_MAX, ARENA_MIN, BASE_Y, COLS, LoadScenarioConfig, ROWS, run_load_scenario,
};

fn single_line(text: impl std::fmt::Display) -> String {
    text.to_string().replace('\n', " ")
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [host, port, login_timeout_secs, run_duration_secs] = match <[String; 4]>::try_from(args) {
        Ok(a) => a,
        Err(_) => {
            eprintln!(
                "usage: load_scenario_runner <host> <port> <login_timeout_secs> <run_duration_secs>"
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let port: u16 = match port.parse() {
        Ok(p) => p,
        Err(err) => {
            println!("RESULT=ERROR");
            println!(
                "MESSAGE={}",
                single_line(format!("invalid port {port:?}: {err}"))
            );
            return std::process::ExitCode::FAILURE;
        }
    };
    let login_timeout = match login_timeout_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            println!("RESULT=ERROR");
            println!(
                "MESSAGE={}",
                single_line(format!("invalid login_timeout_secs: {err}"))
            );
            return std::process::ExitCode::FAILURE;
        }
    };
    let run_duration = match run_duration_secs.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            println!("RESULT=ERROR");
            println!(
                "MESSAGE={}",
                single_line(format!("invalid run_duration_secs: {err}"))
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let config = LoadScenarioConfig {
        host,
        port,
        cols: COLS,
        rows: ROWS,
        arena_min: ARENA_MIN,
        arena_max: ARENA_MAX,
        base_y: BASE_Y,
        login_timeout,
        run_duration,
    };

    let report = run_load_scenario(config).await;

    for (username, result) in report.per_bot {
        println!("BOT_USERNAME={username}");
        match result {
            Ok(outcome) => {
                println!("BOT_RESULT=OK");
                println!("BOT_REACHED_SPAWN={}", outcome.reached_spawn);
                println!("BOT_WAYPOINTS={}", outcome.waypoint_visits);
                println!("BOT_INTERACTIONS={}", outcome.interaction_cycles);
                match outcome.disconnected_at {
                    Some(at) => println!("BOT_DISCONNECTED_AT_MS={}", at.as_millis()),
                    None => println!("BOT_DISCONNECTED_AT_MS=none"),
                }
                println!(
                    "BOT_DETAIL={}",
                    single_line(outcome.disconnect_reason.unwrap_or_default())
                );
            }
            Err(message) => {
                println!("BOT_RESULT=ERROR");
                println!("BOT_DETAIL={}", single_line(message));
            }
        }
    }

    println!("RESULT=OK");
    std::process::ExitCode::SUCCESS
}
