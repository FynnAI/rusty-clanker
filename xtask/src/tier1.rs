//! TEST-D37 Tier 1: every gate above plus `path-guard`, `lint-tests`, `verify-fixtures`.

/// Pure: combines already-computed sub-results into one aggregate `TierResult` named
/// `tier`; overall status is `Fail` if any sub-result's status is `Fail`; each
/// sub-result's cases are copied through with `<sub-result.tier>::` prefixed onto
/// each case name.
pub fn aggregate(
    tier: &str,
    sub_results: &[crate::tier_result::TierResult],
) -> crate::tier_result::TierResult {
    todo!()
}

/// I/O (`xtask tier1 [--base <ref>]`): runs, in order, `fmt_check::run`,
/// `lint::run`, `lint_deps::run`, `test::run`, `path_guard::run(base)`,
/// `forbidden_patterns::run(base)`, `verify_fixtures::run` — collecting each verb's
/// own already-written `target/verify/<verb>.json` (re-reading it, not re-running
/// the verb twice) into `aggregate`, writing the result to `target/verify/tier1.json`.
/// Does not short-circuit on the first failure — every sub-verb still runs, so one
/// `tier1` invocation always reports the complete picture.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    todo!()
}
