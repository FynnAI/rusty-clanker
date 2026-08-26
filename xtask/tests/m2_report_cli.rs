//! `m2_report`'s own CLI/schema/path-guard-correction self-tests (Acceptance tests).

use std::time::Duration;

use xtask::m2_report::{M2ReportResult, Mode};
use xtask::path_guard::{ChangesetType, check_paths};
use xtask::tier_result::{Status, TierResult};

#[test]
fn mode_cadence_params_smoke() {
    assert_eq!(Mode::Smoke.cadence_params(), (20, Duration::from_secs(30)));
}

#[test]
fn mode_cadence_params_full() {
    assert_eq!(
        Mode::Full.cadence_params(),
        (1200, Duration::from_secs(1800))
    );
}

#[test]
fn m2_report_result_serializes_with_flattened_tier_fields_and_new_fields() {
    let mut tier = TierResult::new("m2-acceptance");
    tier.push("AC1a_block_state_disk_identical", Status::Pass, None);
    let tier = tier.finalize();

    let report = M2ReportResult {
        automated: tier,
        mode: "smoke".to_string(),
        target: "127.0.0.1:12345".to_string(),
        save_interval_ticks_used: 20,
    };

    let value = serde_json::to_value(&report).expect("M2ReportResult should serialize");
    let object = value.as_object().expect("top level should be an object");

    assert!(object.contains_key("tier"));
    assert!(object.contains_key("status"));
    assert!(object.contains_key("cases"));

    assert!(object.contains_key("mode"));
    assert!(object.contains_key("target"));
    assert!(object.contains_key("save_interval_ticks_used"));
}

#[test]
fn path_guard_already_covers_m2_b08s_own_new_paths() {
    let violations = check_paths(
        ChangesetType::Implementation,
        &[
            "crates/testing/test-harness/src/chunk_soak.rs".to_string(),
            "crates/testing/paritybot/src/restart_persistence.rs".to_string(),
            "xtask/src/m2_report.rs".to_string(),
        ],
    );
    assert_eq!(violations.len(), 3, "got {violations:?}");
}
