//! TEST-D49: forbidden-pattern lints — five checks against a changeset's diff.

use crate::path_guard::ChangesetType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternViolation {
    UnlinkedIgnore { file: String, line: String },
    TautologicalAssertion { file: String, line: String },
    EmptyTestBody { file: String, fn_name: String },
    UndocumentedTierCfg { file: String, line: String },
    DeletedTest { file: String, fn_name: String },
    AssertionCountRegression { file: String, before: usize, after: usize },
}

// Each is pure — takes already-extracted text, no I/O — and independently unit
// testable (see Acceptance tests). `added_lines` are diff `+` lines with the leading
// `+` already stripped.
pub fn check_unlinked_ignore(file: &str, added_lines: &[String]) -> Vec<PatternViolation> {
    todo!()
}

pub fn check_tautological_assertion(file: &str, added_lines: &[String]) -> Vec<PatternViolation> {
    todo!()
}

pub fn check_empty_test_body(file: &str, head_content: &str) -> Vec<PatternViolation> {
    todo!()
}

pub fn check_undocumented_tier_cfg(file: &str, added_lines: &[String]) -> Vec<PatternViolation> {
    todo!()
}

pub fn check_weakened_tests(
    file: &str,
    base_content: &str,
    head_content: &str,
    changeset_type: ChangesetType,
) -> Vec<PatternViolation> {
    todo!()
}

/// CLI entry point (`xtask lint-tests [--base <ref>]`): same base-resolution rule as
/// `path_guard::run`; for every changed file, shells out to `git diff`/`git show
/// <ref>:<path>` to gather the inputs each `check_*` function above needs and unions
/// their results. Writes `target/verify/lint-tests.json`, returns the matching
/// `ExitCode`.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    todo!()
}
