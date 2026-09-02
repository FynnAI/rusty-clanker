//! TEST-D49: forbidden-pattern lints — five checks against a changeset's diff.

use crate::path_guard::ChangesetType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternViolation {
    UnlinkedIgnore {
        file: String,
        line: String,
    },
    TautologicalAssertion {
        file: String,
        line: String,
    },
    EmptyTestBody {
        file: String,
        fn_name: String,
    },
    UndocumentedTierCfg {
        file: String,
        line: String,
    },
    DeletedTest {
        file: String,
        fn_name: String,
    },
    AssertionCountRegression {
        file: String,
        before: usize,
        after: usize,
    },
    // TEST-D55 (case-matrix header, `crate::case_matrix`):
    MissingCaseMatrixHeader {
        file: String,
    },
    MalformedCaseMatrixHeader {
        file: String,
        error: String,
    },
    CaseMatrixCategoryUnbacked {
        file: String,
        category: String,
    },
    // TEST-D56 (spec citation, `crate::spec_citation`):
    MissingSpecCitation {
        file: String,
        fn_name: String,
        literal: String,
    },
    MalformedSpecCitation {
        file: String,
        fn_name: String,
        comment: String,
    },
}

// Each is pure — takes already-extracted text, no I/O — and independently unit
// testable (see Acceptance tests). `added_lines` are diff `+` lines with the leading
// `+` already stripped.

/// Check 1: an added line that is exactly `#[ignore]`, or an `#[ignore = "…"]` whose
/// quoted reason contains neither a `#<digits>` nor an `issues/<digits>` substring.
pub fn check_unlinked_ignore(file: &str, added_lines: &[String]) -> Vec<PatternViolation> {
    let mut violations = Vec::new();
    for line in added_lines {
        let trimmed = line.trim();
        if trimmed == "#[ignore]" {
            violations.push(PatternViolation::UnlinkedIgnore {
                file: file.to_string(),
                line: trimmed.to_string(),
            });
            continue;
        }
        if let Some(reason) = extract_ignore_reason(trimmed)
            && !contains_linked_reason(&reason)
        {
            violations.push(PatternViolation::UnlinkedIgnore {
                file: file.to_string(),
                line: trimmed.to_string(),
            });
        }
    }
    violations
}

/// Extracts the quoted reason string from `#[ignore = "…"]` / `#[ignore="…"]`.
/// `None` if `trimmed` is not that shape at all (including the bare `#[ignore]` case,
/// handled separately by the caller).
fn extract_ignore_reason(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("#[ignore")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// True iff `reason` contains a `#<digits>` substring or an `issues/<digits>` substring.
fn contains_linked_reason(reason: &str) -> bool {
    for (i, b) in reason.bytes().enumerate() {
        if b == b'#'
            && reason[i + 1..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        {
            return true;
        }
    }
    if let Some(pos) = reason.find("issues/") {
        let rest = &reason[pos + "issues/".len()..];
        if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    false
}

const TAUTOLOGICAL_SUBSTRINGS: &[&str] = &[
    "assert!(true)",
    "assert!(true,",
    "debug_assert!(true)",
    "debug_assert!(true,",
    "assert_eq!(true, true)",
    "assert_eq!(true,true)",
];

/// Removes the content of `"…"`-delimited string literals from `line` (naive — no
/// escape-sequence awareness; quote characters themselves and everything between a
/// pair are blanked to spaces). Keeps a fixture line like
/// `check_tautological_assertion("f.rs", &["assert!(true);".to_string()])` — this
/// very blueprint's own acceptance-test source — from being mistaken by this same
/// check for the real tautological-assertion code it merely names as string data;
/// genuine violation code is never itself written inside a string literal.
pub(crate) fn strip_string_literals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    for ch in line.chars() {
        if ch == '"' {
            in_string = !in_string;
            out.push(' ');
            continue;
        }
        out.push(if in_string { ' ' } else { ch });
    }
    out
}

/// Check 2: an added line, trimmed, containing one of the exact tautological-assertion
/// literal substrings (outside of any `"…"` string literal on that line).
pub fn check_tautological_assertion(file: &str, added_lines: &[String]) -> Vec<PatternViolation> {
    let mut violations = Vec::new();
    for line in added_lines {
        let trimmed = line.trim();
        let code_only = strip_string_literals(trimmed);
        if TAUTOLOGICAL_SUBSTRINGS
            .iter()
            .any(|s| code_only.contains(s))
        {
            violations.push(PatternViolation::TautologicalAssertion {
                file: file.to_string(),
                line: trimmed.to_string(),
            });
        }
    }
    violations
}

/// Byte offsets immediately after each *physical source line* whose trimmed content
/// is exactly `#[test]` — i.e. a genuine standalone attribute line, never a `#[test]`
/// substring that merely appears inside a string literal or a doc comment elsewhere
/// on some (necessarily longer) physical line. Deliberately line-based rather than a
/// raw substring search: a hand-rolled, non-tokenizing scanner that searched raw text
/// would also match the literal `"#[test]\nfn …"` string fixtures this very
/// blueprint's own acceptance tests embed as `&str` arguments, and doc-comment prose
/// that mentions `` `#[test]` `` — both false positives this crate's own source
/// would otherwise trip on itself.
pub(crate) fn test_attr_offsets(content: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut offset = 0usize;
    for line in content.split_inclusive('\n') {
        let line_end = offset + line.len();
        if line.trim_end_matches(['\n', '\r']).trim() == "#[test]" {
            offsets.push(line_end);
        }
        offset = line_end;
    }
    offsets
}

/// Locates the `fn … { … }` immediately following a genuine `#[test]` attribute (its
/// end-of-line byte offset `after_attr`) via brace counting, and returns an
/// `EmptyTestBody` violation iff that body is empty or todo!()/unimplemented!()-only.
fn extract_test_body_violation(
    file: &str,
    content: &str,
    after_attr: usize,
) -> Option<PatternViolation> {
    let fn_pos_rel = content[after_attr..].find("fn ")?;
    let fn_pos = after_attr + fn_pos_rel;
    let name_start = fn_pos + "fn ".len();
    let paren_rel = content[name_start..].find('(')?;
    let name_end = name_start + paren_rel;
    let fn_name = content[name_start..name_end].trim().to_string();

    let open_brace_rel = content[name_end..].find('{')?;
    let open_brace = name_end + open_brace_rel;

    let mut depth = 0i32;
    let mut close_brace = None;
    for (i, ch) in content[open_brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close_brace = Some(open_brace + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close_brace = close_brace?;

    let body = &content[open_brace + 1..close_brace];
    if is_empty_or_noop_body(body) {
        Some(PatternViolation::EmptyTestBody {
            file: file.to_string(),
            fn_name,
        })
    } else {
        None
    }
}

/// Check 3: for every `#[test]` attribute found anywhere in `head_content`, locates
/// the following `fn … { … }` via brace counting and flags an empty or todo!()/
/// unimplemented!()-only body.
pub fn check_empty_test_body(file: &str, head_content: &str) -> Vec<PatternViolation> {
    test_attr_offsets(head_content)
        .into_iter()
        .filter_map(|after_attr| extract_test_body_violation(file, head_content, after_attr))
        .collect()
}

const NOOP_BODY_TOKENS: &[&str] = &[
    "todo!();",
    "todo!()",
    "unimplemented!();",
    "unimplemented!()",
];

/// True iff `body` (a test fn's `{ … }` interior), with `//` line comments and
/// whitespace stripped, is empty or consists solely of one or more of
/// `todo!();`/`todo!()`/`unimplemented!();`/`unimplemented!()`.
fn is_empty_or_noop_body(body: &str) -> bool {
    let stripped: String = body
        .lines()
        .map(|line| line.find("//").map_or(line, |idx| &line[..idx]))
        .collect::<Vec<_>>()
        .join("\n");
    let compact: String = stripped.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return true;
    }
    let mut remaining = compact.as_str();
    while let Some(tok) = NOOP_BODY_TOKENS.iter().find(|t| remaining.starts_with(*t)) {
        remaining = &remaining[tok.len()..];
        if remaining.is_empty() {
            return true;
        }
    }
    false
}

/// Check 4: an added line, trimmed, starting with `#[cfg(` or `#[cfg_attr(`, where a
/// `#[test]` appears within the next 2 added lines, unless that line or the one
/// immediately before it, trimmed, contains `tier-change-reviewed:`.
pub fn check_undocumented_tier_cfg(file: &str, added_lines: &[String]) -> Vec<PatternViolation> {
    let mut violations = Vec::new();
    for i in 0..added_lines.len() {
        let trimmed = added_lines[i].trim();
        if !(trimmed.starts_with("#[cfg(") || trimmed.starts_with("#[cfg_attr(")) {
            continue;
        }
        let lookahead_end = (i + 3).min(added_lines.len());
        let test_within_next_two = added_lines[(i + 1)..lookahead_end]
            .iter()
            .any(|l| l.contains("#[test]"));
        if !test_within_next_two {
            continue;
        }
        let documented = trimmed.contains("tier-change-reviewed:")
            || (i > 0 && added_lines[i - 1].trim().contains("tier-change-reviewed:"));
        if !documented {
            violations.push(PatternViolation::UndocumentedTierCfg {
                file: file.to_string(),
                line: trimmed.to_string(),
            });
        }
    }
    violations
}

/// Extracts every `#[test]`-annotated function name found in `content`, in source
/// order (duplicates preserved — callers that need a set convert as needed).
pub(crate) fn extract_test_fn_names(content: &str) -> Vec<String> {
    test_attr_offsets(content)
        .into_iter()
        .filter_map(|after_attr| {
            let fn_pos_rel = content[after_attr..].find("fn ")?;
            let fn_pos = after_attr + fn_pos_rel;
            let name_start = fn_pos + "fn ".len();
            let paren_rel = content[name_start..].find('(')?;
            let name_end = name_start + paren_rel;
            Some(content[name_start..name_end].trim().to_string())
        })
        .collect()
}

const ASSERTION_SUBSTRINGS: &[&str] = &[
    "assert!(",
    "assert_eq!(",
    "assert_ne!(",
    "debug_assert!(",
    "debug_assert_eq!(",
    "debug_assert_ne!(",
    ".assert_",
];

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut start = 0usize;
    while let Some(pos) = haystack[start..].find(needle) {
        count += 1;
        start += pos + needle.len();
    }
    count
}

fn total_assertion_count(content: &str) -> usize {
    ASSERTION_SUBSTRINGS
        .iter()
        .map(|s| count_occurrences(content, s))
        .sum()
}

/// Check 5 (only when `changeset_type == Implementation`): (a) any `#[test]` fn
/// present at `base_content` and absent at `head_content` is a `DeletedTest`; (b) a
/// strict decrease in the total assertion-macro-substring count is an
/// `AssertionCountRegression`.
pub fn check_weakened_tests(
    file: &str,
    base_content: &str,
    head_content: &str,
    changeset_type: ChangesetType,
) -> Vec<PatternViolation> {
    if changeset_type != ChangesetType::Implementation {
        return Vec::new();
    }

    let mut violations = Vec::new();

    let base_names: std::collections::BTreeSet<String> =
        extract_test_fn_names(base_content).into_iter().collect();
    let head_names: std::collections::BTreeSet<String> =
        extract_test_fn_names(head_content).into_iter().collect();
    for fn_name in base_names.difference(&head_names) {
        violations.push(PatternViolation::DeletedTest {
            file: file.to_string(),
            fn_name: fn_name.clone(),
        });
    }

    let before = total_assertion_count(base_content);
    let after = total_assertion_count(head_content);
    if after < before {
        violations.push(PatternViolation::AssertionCountRegression {
            file: file.to_string(),
            before,
            after,
        });
    }

    violations
}

/// Per-commit gate mirroring `path_guard::evaluate_commit`'s decision shape (one
/// commit is one changeset, TEST-D45): `Skip` for an empty or docs-only-exempt
/// commit, `Lint(t)` to run every check under that commit's own declared type,
/// `Fail` for a missing or malformed trailer on a non-exempt commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintGate {
    Skip(String),
    Lint(ChangesetType),
    Fail(Vec<String>),
}

pub fn commit_lint_gate(commit_message: &str, changed_files: &[String]) -> LintGate {
    if changed_files.is_empty() {
        return LintGate::Skip("no changed files".to_string());
    }
    match crate::path_guard::parse_changeset_type(commit_message) {
        Ok(Some(t)) => LintGate::Lint(t),
        Ok(None) if crate::path_guard::docs_only_exemption(changed_files) => {
            LintGate::Skip(format!(
                "docs-only exemption: {} Markdown file(s), none protected — trailer not required",
                changed_files.len()
            ))
        }
        Ok(None) => LintGate::Fail(vec![
            "commit message is missing a required `Changeset-Type:` trailer".to_string(),
        ]),
        Err(msg) => LintGate::Fail(vec![msg]),
    }
}

/// The `+`-lines one single commit adds to `file` (first-parent diff).
fn added_lines_for(sh: &xshell::Shell, sha: &str, file: &str) -> Result<Vec<String>, String> {
    let parent = format!("{sha}^");
    let output = xshell::cmd!(sh, "git diff {parent} {sha} -- {file}")
        .read()
        .map_err(|err| format!("`git diff {parent} {sha} -- {file}` failed: {err}"))?;
    Ok(output
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .map(|l| l[1..].to_string())
        .collect())
}

/// `git show <rev>:<file>`, treating a failure (file absent at `rev` — newly added or
/// since deleted) as empty content rather than a hard error; git's own `fatal:` line
/// for that expected case is suppressed.
fn content_at(sh: &xshell::Shell, rev: &str, file: &str) -> String {
    let spec = format!("{rev}:{file}");
    xshell::cmd!(sh, "git show {spec}")
        .ignore_stderr()
        .read()
        .unwrap_or_default()
}

fn describe_violation(v: &PatternViolation) -> (&'static str, String) {
    match v {
        PatternViolation::UnlinkedIgnore { file, line } => (
            "unlinked-ignore",
            format!("{file}: unlinked #[ignore] — {line}"),
        ),
        PatternViolation::TautologicalAssertion { file, line } => (
            "tautological-assertion",
            format!("{file}: tautological assertion — {line}"),
        ),
        PatternViolation::EmptyTestBody { file, fn_name } => (
            "empty-test-body",
            format!("{file}: empty/no-op test body in fn {fn_name}"),
        ),
        PatternViolation::UndocumentedTierCfg { file, line } => (
            "undocumented-tier-cfg",
            format!("{file}: undocumented tier-removing cfg — {line}"),
        ),
        PatternViolation::DeletedTest { file, fn_name } => {
            ("deleted-test", format!("{file}: deleted test fn {fn_name}"))
        }
        PatternViolation::AssertionCountRegression {
            file,
            before,
            after,
        } => (
            "assertion-count-regression",
            format!("{file}: assertion count regressed from {before} to {after}"),
        ),
        PatternViolation::MissingCaseMatrixHeader { file } => (
            "missing-case-matrix-header",
            format!("{file}: no `//! test-matrix: …` header found (TEST-D55)"),
        ),
        PatternViolation::MalformedCaseMatrixHeader { file, error } => (
            "malformed-case-matrix-header",
            format!("{file}: malformed `//! test-matrix: …` header — {error}"),
        ),
        PatternViolation::CaseMatrixCategoryUnbacked { file, category } => (
            "case-matrix-category-unbacked",
            format!("{file}: test-matrix category `{category}=yes` has no backing test name"),
        ),
        PatternViolation::MissingSpecCitation {
            file,
            fn_name,
            literal,
        } => (
            "missing-spec-citation",
            format!(
                "{file}: fn {fn_name}: block-state-id literal {literal} has no `// source: …` \
                 citation or `// source-waived: …` waiver within the previous 3 lines (TEST-D56)"
            ),
        ),
        PatternViolation::MalformedSpecCitation {
            file,
            fn_name,
            comment,
        } => (
            "malformed-spec-citation",
            format!("{file}: fn {fn_name}: malformed citation comment — {comment}"),
        ),
    }
}

/// CLI entry point (`xtask lint-tests [--base <ref>]`): same base-resolution rule as
/// `path_guard::run`, and the same **per-commit** walk (`path_guard::rev_list`) — one
/// commit is one changeset, so each commit in `<base>..HEAD` is linted under its own
/// `Changeset-Type:` trailer against its own first-parent diff (`commit_lint_gate`;
/// docs-only all-Markdown commits are exempt from the trailer, exactly as in the
/// path-guard). For every file one commit changes, shells out to `git diff <sha>^
/// <sha>`/`git show <rev>:<path>` to gather the inputs each `check_*` function above
/// needs and unions their results. Writes `target/verify/lint-tests.json`, returns
/// the matching `ExitCode`.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("lint-tests: failed to create shell: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut result = crate::tier_result::TierResult::new("lint-tests");

    let Some(resolved_base) = crate::path_guard::resolve_base(&sh, base) else {
        println!(
            "lint-tests: no base ref resolvable (first commit in the repository) — skipping, vacuous pass"
        );
        result.push(
            "resolve-base",
            crate::tier_result::Status::Pass,
            Some("no base ref resolvable — vacuous pass".to_string()),
        );
        let result = result.finalize();
        if let Err(err) = crate::tier_result::write(&result) {
            eprintln!("lint-tests: failed to write result JSON: {err}");
            return std::process::ExitCode::FAILURE;
        }
        return crate::tier_result::exit_code_for(result.status);
    };

    let commits = match crate::path_guard::rev_list(&sh, &resolved_base) {
        Ok(commits) => commits,
        Err(err) => {
            eprintln!("lint-tests: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    if commits.is_empty() {
        result.push(
            "commits",
            crate::tier_result::Status::Pass,
            Some(format!("no commits in {resolved_base}..HEAD")),
        );
    }

    for sha in &commits {
        let short = &sha[..sha.len().min(9)];
        let commit_message = match xshell::cmd!(sh, "git log -1 --format=%B {sha}").read() {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("lint-tests: failed to read commit message of {sha}: {err}");
                return std::process::ExitCode::FAILURE;
            }
        };
        let parent = format!("{sha}^");
        let changed_files = match xshell::cmd!(sh, "git diff --name-only {parent} {sha}")
            .read()
            .map(|out| {
                out.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            }) {
            Ok(files) => files,
            Err(err) => {
                eprintln!("lint-tests: `git diff --name-only {parent} {sha}` failed: {err}");
                return std::process::ExitCode::FAILURE;
            }
        };

        let changeset_type = match commit_lint_gate(&commit_message, &changed_files) {
            LintGate::Skip(note) => {
                result.push(
                    format!("commit::{short}"),
                    crate::tier_result::Status::Pass,
                    Some(note),
                );
                continue;
            }
            LintGate::Fail(lines) => {
                for line in &lines {
                    eprintln!("lint-tests: commit {short}: {line}");
                }
                result.push(
                    format!("commit::{short}"),
                    crate::tier_result::Status::Fail,
                    Some(lines.join("; ")),
                );
                continue;
            }
            LintGate::Lint(t) => t,
        };

        let mut commit_violations: Vec<PatternViolation> = Vec::new();
        for file in &changed_files {
            let added_lines = match added_lines_for(&sh, sha, file) {
                Ok(lines) => lines,
                Err(err) => {
                    eprintln!("lint-tests: {err}");
                    return std::process::ExitCode::FAILURE;
                }
            };
            commit_violations.extend(check_unlinked_ignore(file, &added_lines));
            commit_violations.extend(check_tautological_assertion(file, &added_lines));
            commit_violations.extend(check_undocumented_tier_cfg(file, &added_lines));

            let head_content = content_at(&sh, sha, file);
            commit_violations.extend(check_empty_test_body(file, &head_content));
            commit_violations.extend(crate::case_matrix::check_case_matrix(file, &head_content));
            commit_violations.extend(crate::spec_citation::check_literal_citations(
                file,
                &head_content,
            ));

            let base_content = content_at(&sh, &parent, file);
            commit_violations.extend(check_weakened_tests(
                file,
                &base_content,
                &head_content,
                changeset_type,
            ));
        }

        if commit_violations.is_empty() {
            result.push(
                format!("commit::{short}"),
                crate::tier_result::Status::Pass,
                Some(format!(
                    "{} changed files, 0 violations",
                    changed_files.len()
                )),
            );
        } else {
            for (i, v) in commit_violations.iter().enumerate() {
                let (name, detail) = describe_violation(v);
                eprintln!("lint-tests: commit {short}: {detail}");
                result.push(
                    format!("commit::{short}::{i}::{name}"),
                    crate::tier_result::Status::Fail,
                    Some(detail),
                );
            }
        }
    }

    let result = result.finalize();
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("lint-tests: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}
