//! `m1_report`'s own CLI/schema/path-guard-correction self-tests (Acceptance tests).

use std::time::Duration;

use xtask::m1_report::{M1ReportResult, ManualStep, Mode};
use xtask::path_guard::{ChangesetType, check_paths};
use xtask::tier_result::{Status, TierResult};

#[test]
fn mode_idle_duration_smoke_is_90s() {
    assert_eq!(Mode::Smoke.idle_duration(), Duration::from_secs(90));
}

#[test]
fn mode_idle_duration_full_is_1800s() {
    assert_eq!(Mode::Full.idle_duration(), Duration::from_secs(1800));
}

#[test]
fn m1_report_result_serializes_with_flattened_tier_fields() {
    let mut tier = TierResult::new("m1-acceptance");
    tier.push("AC1a_status_pong", Status::Pass, None);
    let tier = tier.finalize();

    let report = M1ReportResult {
        automated: tier,
        manual_steps: vec![ManualStep {
            id: "AC3",
            description: "manual step",
            procedure_doc: "docs/MANUAL-VERIFICATION-M1.md",
        }],
        mode: "smoke".to_string(),
        target: "127.0.0.1:12345".to_string(),
    };

    let value = serde_json::to_value(&report).expect("M1ReportResult should serialize");
    let object = value.as_object().expect("top level should be an object");

    // Flattened from `TierResult`.
    assert!(object.contains_key("tier"));
    assert!(object.contains_key("status"));
    assert!(object.contains_key("cases"));

    // Sibling fields this blueprint's own schema adds.
    assert!(object.contains_key("manual_steps"));
    assert!(object.contains_key("mode"));
    assert!(object.contains_key("target"));
}

#[test]
fn path_guard_protects_the_corrected_testing_crate_paths() {
    let violations = check_paths(
        ChangesetType::Implementation,
        &[
            "crates/testing/test-harness/src/probe.rs".to_string(),
            "crates/testing/paritybot/tests/idle_stability_self_tests.rs".to_string(),
        ],
    );
    assert_eq!(violations.len(), 2, "got {violations:?}");
}
