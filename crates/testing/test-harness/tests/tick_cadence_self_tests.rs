//! `tick_cadence::{parse_tick_log, analyze_tps}` self-tests (Acceptance tests).

use std::io::Write;

use rc_test_harness::tick_cadence::{TickLogEntry, analyze_tps, parse_tick_log};

#[test]
fn on_time_log_reports_within_tolerance() {
    let entries: Vec<TickLogEntry> = (1..=20u64)
        .map(|tick| TickLogEntry {
            tick,
            elapsed_ms: tick * 50,
        })
        .collect();

    let report = analyze_tps(&entries, 20.0, 0.01);

    assert!(
        (report.measured_tps - 20.0).abs() < 1e-9,
        "measured_tps {} should be within 1e-9 of 20.0",
        report.measured_tps
    );
    assert!(report.within_tolerance);
}

#[test]
fn lagged_log_is_caught_by_the_tps_leg() {
    let entries: Vec<TickLogEntry> = (1..=20u64)
        .map(|tick| TickLogEntry {
            tick,
            elapsed_ms: tick * 60,
        })
        .collect();

    let report = analyze_tps(&entries, 20.0, 0.01);

    assert!(
        (report.measured_tps - 16.667).abs() < 1e-3,
        "measured_tps {} should be approximately 16.667",
        report.measured_tps
    );
    assert!(
        (report.drift_ratio - (-0.1667)).abs() < 1e-3,
        "drift_ratio {} should be approximately -0.1667",
        report.drift_ratio
    );
    assert!(!report.within_tolerance);
}

/// A bit-exact 1.00% drift ratio is not constructible from integer-millisecond
/// `elapsed_ms` inputs (20.2/20.0 is not exactly representable in binary floating
/// point — verified live: `101.0 / 5.0 / 20.0 - 1.0` evaluates to
/// `0.010000000000000009`, not `0.01`, regardless of which integer sample-count/
/// duration pair produces the `20.2` measured-TPS numerator). This test instead
/// directly exercises "`<=`, not `<`" the way the comparison itself is written:
/// measure the actual drift a real-looking, ~49.5ms/tick log produces, then re-check
/// tolerance set to that *exact* observed value. A `<` comparison would reject its
/// own boundary (`drift < drift` is always `false`); the literal formula
/// (Deliverables' own `drift_ratio.abs() <= tolerance`) accepts it.
#[test]
fn slightly_fast_log_at_the_edge_of_tolerance_passes() {
    let entries: Vec<TickLogEntry> = (0..=19u64)
        .map(|tick| TickLogEntry {
            tick,
            elapsed_ms: (tick as f64 * 49.5).round() as u64,
        })
        .collect();

    let probe = analyze_tps(&entries, 20.0, 1.0); // generous tolerance, only to read drift_ratio
    let boundary_tolerance = probe.drift_ratio.abs();

    let report = analyze_tps(&entries, 20.0, boundary_tolerance);
    assert!(
        report.within_tolerance,
        "drift_ratio {} should pass at its own exact boundary tolerance {boundary_tolerance}",
        report.drift_ratio
    );
}

#[test]
fn parse_tick_log_skips_malformed_lines() {
    let dir = std::env::temp_dir().join(format!(
        "rc-tick-cadence-self-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("tick-log.ndjson");

    let mut file = std::fs::File::create(&path).expect("create tick log");
    writeln!(file, r#"{{"tick":1,"elapsed_ms":50}}"#).unwrap();
    writeln!(file, "not json").unwrap();
    writeln!(file, r#"{{"tick":2,"elapsed_ms":100}}"#).unwrap();
    drop(file);

    let entries = parse_tick_log(&path).expect("parse_tick_log should succeed");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].tick, 1);
    assert_eq!(entries[0].elapsed_ms, 50);
    assert_eq!(entries[1].tick, 2);
    assert_eq!(entries[1].elapsed_ms, 100);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[should_panic]
fn analyze_tps_panics_on_fewer_than_two_entries() {
    let entries = vec![TickLogEntry {
        tick: 1,
        elapsed_ms: 50,
    }];
    let _ = analyze_tps(&entries, 20.0, 0.01);
}
