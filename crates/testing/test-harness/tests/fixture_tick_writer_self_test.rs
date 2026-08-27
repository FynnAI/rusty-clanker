//! `fixture_tick_writer` real-subprocess self-tests (Acceptance tests, Tier 1): a
//! genuine, separate, wall-clock-paced OS process proving the harness's own
//! end-to-end log-read-then-analyze pipeline — not merely `analyze_tps` in
//! isolation — correctly classifies both a lagged and an on-time tick producer,
//! before this same pipeline is ever trusted against a real `rusty-clanker-server`.

use std::process::Command;

use rc_test_harness::tick_cadence::{analyze_tps, parse_tick_log};

/// Cargo sets `CARGO_BIN_EXE_<name>` at test-binary build time for every `[[bin]]`
/// target declared in this same crate's own `Cargo.toml` — the portable, OS-agnostic
/// way to locate a sibling binary (handles the `.exe` suffix on Windows
/// automatically), used here instead of hand-deriving a path off `current_exe()`.
fn fixture_tick_writer_path() -> &'static str {
    env!("CARGO_BIN_EXE_fixture_tick_writer")
}

fn temp_log_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rc-fixture-tick-writer-self-test-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn run_fixture(out_path: &std::path::Path, tick_count: u64, tick_period_ms: u64) {
    let status = Command::new(fixture_tick_writer_path())
        .arg("--out")
        .arg(out_path)
        .arg("--tick-count")
        .arg(tick_count.to_string())
        .arg("--tick-period-ms")
        .arg(tick_period_ms.to_string())
        .status()
        .expect("fixture_tick_writer should spawn");
    assert!(
        status.success(),
        "fixture_tick_writer exited with {status:?}"
    );
}

#[test]
fn lagged_fixture_process_fails_the_tps_leg() {
    let out_path = temp_log_path("lagged");
    run_fixture(&out_path, 20, 60);

    let entries = parse_tick_log(&out_path).expect("parse_tick_log should succeed");
    let report = analyze_tps(&entries, 20.0, 0.01);
    assert!(!report.within_tolerance, "report: {report:?}");

    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn on_time_fixture_process_passes_the_tps_leg() {
    let out_path = temp_log_path("on-time");
    run_fixture(&out_path, 20, 50);

    let entries = parse_tick_log(&out_path).expect("parse_tick_log should succeed");
    let report = analyze_tps(&entries, 20.0, 0.01);
    assert!(report.within_tolerance, "report: {report:?}");

    let _ = std::fs::remove_file(&out_path);
}
