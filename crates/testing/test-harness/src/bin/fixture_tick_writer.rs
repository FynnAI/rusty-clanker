//! A tiny, real, standalone subprocess used only by this blueprint's own Tier-1 self
//! tests to prove `tick_cadence`'s analysis pipeline against an actual foreign
//! process before it is ever trusted against a real `rusty-clanker-server` — mirrors
//! M1-B06's own `process_self_tests.rs` precedent of using "a trivial... test
//! fixture binary... implementer's choice of a portable fixture" as a stand-in
//! target process, made concrete and reusable here rather than ad hoc.
//!
//! Usage: `fixture_tick_writer --out <path> --tick-count <n> --tick-period-ms <n>`.
//! Writes exactly `tick_count` lines to `out`, each one
//! `serde_json::to_string(&rc_test_harness::tick_cadence::TickLogEntry { tick,
//! elapsed_ms })` (`{"tick":1,"elapsed_ms":<period>}`,
//! `{"tick":2,"elapsed_ms":<period*2>}`, ...) — the identical shape a real
//! `rusty-clanker-server` would write, so this fixture and the real server exercise
//! the exact same downstream parser — sleeping `tick_period_ms` real milliseconds
//! between each write (a genuine, real-time-paced process, not an instantaneous batch
//! write). Exits 0 on success.

use std::io::Write;
use std::time::Duration;

use rc_test_harness::tick_cadence::TickLogEntry;

fn parse_args(args: Vec<String>) -> Result<(std::path::PathBuf, u64, u64), String> {
    let mut out: Option<std::path::PathBuf> = None;
    let mut tick_count: Option<u64> = None;
    let mut tick_period_ms: Option<u64> = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--out" => {
                let value = iter.next().ok_or("--out requires a value")?;
                out = Some(std::path::PathBuf::from(value));
            }
            "--tick-count" => {
                let value = iter.next().ok_or("--tick-count requires a value")?;
                tick_count = Some(
                    value
                        .parse()
                        .map_err(|_| format!("--tick-count value {value:?} is not a valid u64"))?,
                );
            }
            "--tick-period-ms" => {
                let value = iter.next().ok_or("--tick-period-ms requires a value")?;
                tick_period_ms =
                    Some(value.parse().map_err(|_| {
                        format!("--tick-period-ms value {value:?} is not a valid u64")
                    })?);
            }
            other => return Err(format!("unrecognized argument {other:?}")),
        }
    }

    let out = out.ok_or("--out is required")?;
    let tick_count = tick_count.ok_or("--tick-count is required")?;
    let tick_period_ms = tick_period_ms.ok_or("--tick-period-ms is required")?;
    Ok((out, tick_count, tick_period_ms))
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out_path, tick_count, tick_period_ms) = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!(
                "fixture_tick_writer: {message}\nusage: fixture_tick_writer --out <path> --tick-count <n> --tick-period-ms <n>"
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut file = match std::fs::File::create(&out_path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!(
                "fixture_tick_writer: failed to create {}: {err}",
                out_path.display()
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    for tick in 1..=tick_count {
        std::thread::sleep(Duration::from_millis(tick_period_ms));
        let entry = TickLogEntry {
            tick,
            elapsed_ms: tick * tick_period_ms,
        };
        let line = match serde_json::to_string(&entry) {
            Ok(line) => line,
            Err(err) => {
                eprintln!("fixture_tick_writer: failed to serialize tick {tick}: {err}");
                return std::process::ExitCode::FAILURE;
            }
        };
        if let Err(err) = writeln!(file, "{line}") {
            eprintln!("fixture_tick_writer: failed to write tick {tick}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    }
    if let Err(err) = file.flush() {
        eprintln!(
            "fixture_tick_writer: failed to flush {}: {err}",
            out_path.display()
        );
        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}
