//! `save_cadence`'s own self-tests (Acceptance tests).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rc_test_harness::save_cadence::{SaveEvent, analyze_cadence, parse_save_event_log};

fn event(tick: u64, region_id: &str) -> SaveEvent {
    SaveEvent {
        tick,
        region_id: region_id.to_string(),
        elapsed_ms: tick * 50,
    }
}

#[test]
fn on_time_events_report_no_violations() {
    let events = vec![
        event(0, "r0.0"),
        event(1200, "r0.0"),
        event(2400, "r0.0"),
        event(3600, "r0.0"),
        event(4801, "r0.0"),
    ];
    let report = analyze_cadence(&events, 1200);
    assert!(
        report.within_tolerance(),
        "unexpected violations: {:?}",
        report.violations
    );
}

#[test]
fn late_save_timer_is_caught_by_the_cadence_leg() {
    let events = vec![event(0, "r0.0"), event(1200, "r0.0"), event(2402, "r0.0")];
    let report = analyze_cadence(&events, 1200);
    assert!(!report.within_tolerance());
    assert_eq!(report.violations.len(), 1);
    let violation = &report.violations[0];
    assert_eq!(violation.at_index, 2);
    assert_eq!(violation.expected_interval_ticks, 1200);
    assert_eq!(violation.actual_interval_ticks, 1202);
}

#[test]
fn early_save_is_also_a_violation() {
    let events = vec![event(0, "r0.0"), event(1200, "r0.0"), event(2397, "r0.0")];
    let report = analyze_cadence(&events, 1200);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].at_index, 2);
    assert_eq!(report.violations[0].actual_interval_ticks, 1197);
}

/// The gap between a chunk's immediate first save and its first interval-elapsed save is
/// decided by when the harness's dirty driver first touched the chunk (bot login, recenter,
/// aim settle), never by the save timer -- `analyze_cadence` treats it as warm-up. Observed
/// for real on the `ubuntu-24.04` runner: first save at the join burst, first churn toggle
/// three ticks after the interval had elapsed, every later gap exactly on cadence.
#[test]
fn first_gap_after_a_chunks_immediate_first_save_is_warm_up_not_cadence() {
    let events = vec![
        event(340, "1:0,0"),
        event(363, "1:0,0"),
        event(383, "1:0,0"),
        event(403, "1:0,0"),
    ];
    let report = analyze_cadence(&events, 20);
    assert!(
        report.within_tolerance(),
        "unexpected violations: {:?}",
        report.violations
    );
}

#[test]
fn a_late_second_gap_is_still_caught_after_the_warm_up_gap() {
    let events = vec![
        event(340, "1:0,0"),
        event(363, "1:0,0"),
        event(386, "1:0,0"),
    ];
    let report = analyze_cadence(&events, 20);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].at_index, 2);
    assert_eq!(report.violations[0].actual_interval_ticks, 23);
}

#[test]
fn multiple_regions_are_analyzed_independently() {
    let events = vec![
        event(0, "r0.0"),
        event(5, "r1.0"),
        event(1200, "r0.0"),
        event(1206, "r1.0"),
    ];
    let report = analyze_cadence(&events, 1200);
    assert!(
        report.within_tolerance(),
        "unexpected violations: {:?}",
        report.violations
    );
}

#[test]
fn single_event_per_region_produces_no_violation() {
    let events = vec![event(0, "r0.0")];
    let report = analyze_cadence(&events, 1200);
    assert!(report.within_tolerance());
    assert!(report.violations.is_empty());
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_log_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "rc-test-harness-save-cadence-{name}-{}-{}.jsonl",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write_temp_log(path: &Path, content: &str) {
    std::fs::write(path, content).expect("write temp save-event log");
}

#[test]
fn parse_save_event_log_skips_malformed_lines() {
    let path = temp_log_path("parse_save_event_log_skips_malformed_lines");
    let content = concat!(
        "{\"tick\": 0, \"region_id\": \"r0.0\", \"elapsed_ms\": 10}\n",
        "not json\n",
        "{\"tick\": 1200, \"region_id\": \"r0.0\", \"elapsed_ms\": 70}\n",
    );
    write_temp_log(&path, content);

    let events = parse_save_event_log(&path).expect("parse should succeed despite the bad line");
    let _ = std::fs::remove_file(&path);

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].tick, 0);
    assert_eq!(events[1].tick, 1200);
}
