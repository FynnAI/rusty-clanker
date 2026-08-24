use xtask::tier_result::{Status, TierResult};
use xtask::tier1::aggregate;

fn passing_result(tier: &str, case: &str) -> TierResult {
    let mut result = TierResult::new(tier);
    result.push(case, Status::Pass, None);
    result.finalize()
}

fn failing_result(tier: &str, case: &str) -> TierResult {
    let mut result = TierResult::new(tier);
    result.push(case, Status::Fail, Some("boom".to_string()));
    result.finalize()
}

#[test]
fn aggregate_fails_if_any_sub_result_failed() {
    let sub_results = vec![
        passing_result("fmt-check", "ok"),
        failing_result("lint", "boom"),
    ];
    let aggregated = aggregate("tier1", &sub_results);
    assert_eq!(aggregated.status, Status::Fail);
}

#[test]
fn aggregate_passes_if_all_sub_results_passed() {
    let sub_results = vec![
        passing_result("fmt-check", "ok"),
        passing_result("lint", "ok"),
    ];
    let aggregated = aggregate("tier1", &sub_results);
    assert_eq!(aggregated.status, Status::Pass);
}

#[test]
fn aggregate_prefixes_case_names_with_sub_tier() {
    let sub_results = vec![passing_result("lint-deps", "rules")];
    let aggregated = aggregate("tier1", &sub_results);
    assert!(
        aggregated
            .cases
            .iter()
            .any(|c| c.name == "lint-deps::rules")
    );
}
