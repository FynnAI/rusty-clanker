//! `m3_report`'s own CLI/schema/aggregation/path-guard-correction self-tests
//! (Acceptance tests). `build_report`'s own `bots` parameter is `xtask::m3_report::
//! LoadScenarioReport` — this module's own azalea-free local mirror of
//! `rc_paritybot::load_scenario::LoadScenarioReport` (`m3_report.rs`'s own module doc
//! comment, "Forced deviation" — `xtask` never depends on `rc-paritybot`).

use std::time::Duration;

use rc_test_harness::tick_cadence::TpsReport;
use xtask::m3_report::{
    LoadBotOutcome, LoadScenarioReport, M3ReportResult, Mode, build_report, parse_region_count_line,
};
use xtask::path_guard::{ChangesetType, check_paths};
use xtask::tier_result::{Status, TierResult};

fn passing_bots() -> LoadScenarioReport {
    LoadScenarioReport {
        bot_count: 20,
        per_bot: (0..20)
            .map(|i| {
                (
                    format!("rc-load-bot-{i:02}"),
                    Ok(LoadBotOutcome {
                        reached_spawn: true,
                        waypoint_visits: 10,
                        interaction_cycles: 3,
                        disconnected_at: None,
                        disconnect_reason: None,
                    }),
                )
            })
            .collect(),
    }
}

fn in_tolerance_tps() -> TpsReport {
    TpsReport {
        sample_count: 12000,
        duration_secs: 600.0,
        measured_tps: 20.0,
        drift_ratio: 0.0,
        within_tolerance: true,
    }
}

fn passing_tier(tier: &str, case_count: usize) -> TierResult {
    let mut result = TierResult::new(tier);
    for i in 0..case_count {
        result.push(format!("case-{i}"), Status::Pass, None);
    }
    result.finalize()
}

fn case_status(report: &M3ReportResult, name: &str) -> Status {
    report
        .automated
        .cases
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("missing case {name}"))
        .status
}

#[test]
fn mode_load_test_duration_smoke_is_60s() {
    assert_eq!(Mode::Smoke.load_test_duration(), Duration::from_secs(60));
}

#[test]
fn mode_load_test_duration_full_is_600s() {
    assert_eq!(Mode::Full.load_test_duration(), Duration::from_secs(600));
}

#[test]
fn parse_region_count_line_finds_the_value() {
    assert_eq!(
        parse_region_count_line(&[
            "some other line".to_string(),
            "RC_REGION_COUNT=1".to_string()
        ]),
        Some(1)
    );
}

#[test]
fn parse_region_count_line_returns_none_when_absent() {
    assert_eq!(parse_region_count_line(&["nothing here".to_string()]), None);
}

#[test]
fn m3_report_result_serializes_with_flattened_tier_fields() {
    let mut tier = TierResult::new("m3-acceptance");
    tier.push("AC1_redstone_corpus_parity", Status::Pass, None);
    let tier = tier.finalize();

    let report = M3ReportResult {
        automated: tier,
        mode: "smoke".to_string(),
        target: "127.0.0.1:25566".to_string(),
        load_test_duration_secs: 60,
        redstone_corpus_contraption_count: 50,
        bot_count: 20,
    };

    let value = serde_json::to_value(&report).expect("M3ReportResult should serialize");
    let object = value.as_object().expect("top level should be an object");

    assert!(object.contains_key("tier"));
    assert!(object.contains_key("status"));
    assert!(object.contains_key("cases"));
    assert!(object.contains_key("mode"));
    assert!(object.contains_key("target"));
    assert!(object.contains_key("load_test_duration_secs"));
    assert!(object.contains_key("redstone_corpus_contraption_count"));
    assert!(object.contains_key("bot_count"));
}

#[test]
fn perturbed_redstone_replay_is_caught_by_the_parity_leg() {
    let fetch_corpus_result = passing_tier("fetch-corpus", 1);

    // A synthetic stand-in for "some contraption's replay diverged from its captured
    // trace" (M3-B07's own `diff_traces.rs` already proves the underlying comparison
    // catches this; this test proves this blueprint's own aggregation propagates it)
    // -- 50 total cases (>= the AC1 size gate) so only the parity sub-check, not the
    // size gate, is exercised here.
    let mut parity_check_result = TierResult::new("parity-check-redstone");
    parity_check_result.push(
        "some-contraption",
        Status::Fail,
        Some("mismatch".to_string()),
    );
    for i in 0..49 {
        parity_check_result.push(format!("contraption-{i}"), Status::Pass, None);
    }
    let parity_check_result = parity_check_result.finalize();

    let bots = passing_bots();
    let tps = in_tolerance_tps();

    let report = build_report(
        Mode::Smoke,
        "127.0.0.1:25566".to_string(),
        &fetch_corpus_result,
        &parity_check_result,
        tps,
        &bots,
        Some(1),
    );

    assert_eq!(report.automated.status, Status::Fail);
    assert_eq!(
        case_status(&report, "AC1_redstone_corpus_parity"),
        Status::Fail
    );
    for name in [
        "AC1_fetch_corpus_capture_succeeded",
        "AC1_redstone_corpus_size_at_least_50",
        "AC2a_tps_within_one_percent_over_full_duration",
        "AC2b_all_bots_completed_without_unexpected_disconnect",
        "AC2c_single_region_topology_pinned",
    ] {
        assert_eq!(
            case_status(&report, name),
            Status::Pass,
            "{name} should still pass"
        );
    }
}

#[test]
fn corpus_below_50_fails_the_size_gate_independently_of_parity() {
    let fetch_corpus_result = passing_tier("fetch-corpus", 1);
    let parity_check_result = passing_tier("parity-check-redstone", 3);
    let bots = passing_bots();
    let tps = in_tolerance_tps();

    let report = build_report(
        Mode::Smoke,
        "127.0.0.1:25566".to_string(),
        &fetch_corpus_result,
        &parity_check_result,
        tps,
        &bots,
        Some(1),
    );

    assert_eq!(
        case_status(&report, "AC1_redstone_corpus_size_at_least_50"),
        Status::Fail
    );
    assert_eq!(
        case_status(&report, "AC1_redstone_corpus_parity"),
        Status::Pass
    );
}

#[test]
fn region_count_mismatch_fails_only_ac2c() {
    let fetch_corpus_result = passing_tier("fetch-corpus", 1);
    let parity_check_result = passing_tier("parity-check-redstone", 50);
    let bots = passing_bots();
    let tps = in_tolerance_tps();

    let report = build_report(
        Mode::Smoke,
        "127.0.0.1:25566".to_string(),
        &fetch_corpus_result,
        &parity_check_result,
        tps,
        &bots,
        Some(2),
    );

    assert_eq!(report.automated.status, Status::Fail);
    assert_eq!(
        case_status(&report, "AC2c_single_region_topology_pinned"),
        Status::Fail
    );
    for name in [
        "AC1_fetch_corpus_capture_succeeded",
        "AC1_redstone_corpus_size_at_least_50",
        "AC1_redstone_corpus_parity",
        "AC2a_tps_within_one_percent_over_full_duration",
        "AC2b_all_bots_completed_without_unexpected_disconnect",
    ] {
        assert_eq!(
            case_status(&report, name),
            Status::Pass,
            "{name} should still pass"
        );
    }
}

#[test]
fn disconnected_bot_fails_only_ac2b() {
    let fetch_corpus_result = passing_tier("fetch-corpus", 1);
    let parity_check_result = passing_tier("parity-check-redstone", 50);
    let mut bots = passing_bots();
    bots.per_bot[0].1 = Ok(LoadBotOutcome {
        reached_spawn: true,
        waypoint_visits: 4,
        interaction_cycles: 1,
        disconnected_at: Some(Duration::from_secs(10)),
        disconnect_reason: Some("connection reset".to_string()),
    });
    let tps = in_tolerance_tps();

    let report = build_report(
        Mode::Smoke,
        "127.0.0.1:25566".to_string(),
        &fetch_corpus_result,
        &parity_check_result,
        tps,
        &bots,
        Some(1),
    );

    assert_eq!(report.automated.status, Status::Fail);
    assert_eq!(
        case_status(
            &report,
            "AC2b_all_bots_completed_without_unexpected_disconnect"
        ),
        Status::Fail
    );
    for name in [
        "AC1_fetch_corpus_capture_succeeded",
        "AC1_redstone_corpus_size_at_least_50",
        "AC1_redstone_corpus_parity",
        "AC2a_tps_within_one_percent_over_full_duration",
        "AC2c_single_region_topology_pinned",
    ] {
        assert_eq!(
            case_status(&report, name),
            Status::Pass,
            "{name} should still pass"
        );
    }
}

#[test]
fn path_guard_already_covers_m3_b08s_own_new_paths() {
    let violations = check_paths(
        ChangesetType::Implementation,
        &[
            "crates/testing/test-harness/src/tick_cadence.rs".to_string(),
            "crates/testing/paritybot/src/load_scenario.rs".to_string(),
            "xtask/src/m3_report.rs".to_string(),
        ],
    );
    assert_eq!(violations.len(), 3, "got {violations:?}");
}
