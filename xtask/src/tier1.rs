//! TEST-D37 Tier 1: every gate above plus `path-guard`, `lint-tests`, `verify-fixtures`.

/// Pure: combines already-computed sub-results into one aggregate `TierResult` named
/// `tier`; overall status is `Fail` if any sub-result's status is `Fail`; each
/// sub-result's cases are copied through with `<sub-result.tier>::` prefixed onto
/// each case name.
pub fn aggregate(
    tier: &str,
    sub_results: &[crate::tier_result::TierResult],
) -> crate::tier_result::TierResult {
    let mut result = crate::tier_result::TierResult::new(tier);
    for sub in sub_results {
        for case in &sub.cases {
            result.push(
                format!("{}::{}", sub.tier, case.name),
                case.status,
                case.detail.clone(),
            );
        }
    }
    result.finalize()
}

/// The verb-name -> `target/verify/<name>.json` tiers `tier1::run` collects, in
/// execution order.
const SUB_VERBS: &[&str] = &[
    "fmt-check",
    "lint",
    "lint-deps",
    "test",
    "path-guard",
    "lint-tests",
    "verify-fixtures",
];

/// I/O (`xtask tier1 [--base <ref>]`): runs, in order, `fmt_check::run`,
/// `lint::run`, `lint_deps::run`, `test::run`, `path_guard::run(base)`,
/// `forbidden_patterns::run(base)`, `verify_fixtures::run` — collecting each verb's
/// own already-written `target/verify/<verb>.json` (re-reading it, not re-running
/// the verb twice) into `aggregate`, writing the result to `target/verify/tier1.json`.
/// Does not short-circuit on the first failure — every sub-verb still runs, so one
/// `tier1` invocation always reports the complete picture.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    let _ = crate::fmt_check::run();
    let _ = crate::lint::run();
    let _ = crate::lint_deps::run();
    let _ = crate::test::run();
    let _ = crate::path_guard::run(base);
    let _ = crate::forbidden_patterns::run(base);
    let _ = crate::verify_fixtures::run();

    let mut sub_results = Vec::with_capacity(SUB_VERBS.len());
    for &tier in SUB_VERBS {
        match read_sub_result(tier) {
            Ok(sub) => sub_results.push(sub),
            Err(err) => {
                eprintln!("tier1: {err}");
                let mut fallback = crate::tier_result::TierResult::new(tier);
                fallback.push("read-result", crate::tier_result::Status::Fail, Some(err));
                sub_results.push(fallback.finalize());
            }
        }
    }

    let aggregated = aggregate("tier1", &sub_results);
    if let Err(err) = crate::tier_result::write(&aggregated) {
        eprintln!("tier1: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(aggregated.status)
}

/// Re-reads `target/verify/<tier>.json` (already written by that verb's own `run`)
/// back into a `TierResult` — never re-running the verb a second time.
fn read_sub_result(tier: &str) -> Result<crate::tier_result::TierResult, String> {
    let path =
        std::path::Path::new(crate::tier_result::VERIFY_OUT_DIR).join(format!("{tier}.json"));
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}
