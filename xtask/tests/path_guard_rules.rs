use xtask::path_guard::{
    ChangesetType, check_paths, docs_only_exemption, evaluate_commit, glob_match,
    parse_changeset_type,
};

// The mixed-push scenario the guard exists to judge correctly: a test-authoring commit
// touching tests/ and an implementation commit touching only src/ are BOTH fine — each
// commit is one changeset, judged by its own trailer against its own diff (never the
// blended range under HEAD's trailer).
#[test]
fn evaluate_commit_accepts_a_test_authoring_commit_touching_tests() {
    let verdict = evaluate_commit(
        "Subject\n\nChangeset-Type: test-authoring\n",
        &["crates/server/tests/play_movement_application.rs".to_string()],
    );
    assert!(verdict.is_ok());
}

#[test]
fn evaluate_commit_accepts_an_implementation_commit_touching_only_src() {
    let verdict = evaluate_commit(
        "Subject\n\nChangeset-Type: implementation\n",
        &["crates/server/src/play/movement.rs".to_string()],
    );
    assert!(verdict.is_ok());
}

#[test]
fn evaluate_commit_rejects_an_implementation_commit_touching_tests() {
    let verdict = evaluate_commit(
        "Subject\n\nChangeset-Type: implementation\n",
        &["crates/server/tests/play_movement_application.rs".to_string()],
    );
    let failures = verdict.unwrap_err();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("crates/*/tests/**"));
}

#[test]
fn evaluate_commit_accepts_a_trailerless_docs_only_commit() {
    let verdict = evaluate_commit(
        "Docs update, no trailer.\n",
        &["blueprints/M2/M2-COMPLETION-REPORT.md".to_string()],
    );
    assert!(verdict.is_ok());
}

#[test]
fn evaluate_commit_rejects_a_trailerless_code_commit() {
    let verdict = evaluate_commit(
        "Code change, no trailer.\n",
        &["crates/core/src/lib.rs".to_string()],
    );
    let failures = verdict.unwrap_err();
    assert!(failures[0].contains("Changeset-Type"));
}

#[test]
fn evaluate_commit_passes_an_empty_commit() {
    assert!(evaluate_commit("Subject only, empty diff.\n", &[]).is_ok());
}

#[test]
fn docs_only_exemption_accepts_unprotected_markdown() {
    assert!(docs_only_exemption(&[
        "CLAUDE.md".to_string(),
        "README.md".to_string(),
        "blueprints/M1/M1-COMPLETION-REPORT.md".to_string(),
        "docs/research/mc-26.2/26-registry-sync-configuration.md".to_string(),
    ]));
}

#[test]
fn docs_only_exemption_rejects_protected_markdown() {
    assert!(!docs_only_exemption(&[
        "CLAUDE.md".to_string(),
        "docs/planning/09-testing-quality.md".to_string(),
    ]));
}

#[test]
fn docs_only_exemption_rejects_any_non_markdown_file() {
    assert!(!docs_only_exemption(&[
        "README.md".to_string(),
        "crates/core/src/lib.rs".to_string(),
    ]));
}

#[test]
fn docs_only_exemption_rejects_empty_change_set() {
    assert!(!docs_only_exemption(&[]));
}

#[test]
fn implementation_touching_tests_dir_is_blocked() {
    let violations = check_paths(
        ChangesetType::Implementation,
        &["crates/core/tests/foo.rs".to_string()],
    );
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].pattern, "crates/*/tests/**");
}

#[test]
fn implementation_touching_xtask_is_blocked() {
    let violations = check_paths(
        ChangesetType::Implementation,
        &["xtask/src/lint.rs".to_string()],
    );
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].pattern, "xtask/**");
}

#[test]
fn implementation_touching_slo_doc_is_blocked() {
    let violations = check_paths(
        ChangesetType::Implementation,
        &["docs/planning/09-testing-quality.md".to_string()],
    );
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].pattern, "docs/planning/09-testing-quality.md");
}

#[test]
fn test_authoring_may_touch_tests_dir() {
    let violations = check_paths(
        ChangesetType::TestAuthoring,
        &["crates/core/tests/foo.rs".to_string()],
    );
    assert_eq!(violations.len(), 0);
}

#[test]
fn governance_may_touch_xtask() {
    let violations = check_paths(
        ChangesetType::Governance,
        &["xtask/src/main.rs".to_string()],
    );
    assert_eq!(violations.len(), 0);
}

#[test]
fn implementation_touching_unrelated_src_is_allowed() {
    let violations = check_paths(
        ChangesetType::Implementation,
        &["crates/core/src/lib.rs".to_string()],
    );
    assert_eq!(violations.len(), 0);
}

#[test]
fn glob_match_double_star_matches_nested_paths() {
    assert!(glob_match(
        "crates/testing/rc-gametest/corpus/**",
        "crates/testing/rc-gametest/corpus/redstone/bud.ron"
    ));
}

#[test]
fn glob_match_single_star_matches_exactly_one_segment() {
    assert!(glob_match(
        "crates/*/tests/**",
        "crates/core/tests/foo/bar.rs"
    ));
    assert!(!glob_match(
        "crates/*/tests/**",
        "crates/core/src/tests_helper.rs"
    ));
}

#[test]
fn parse_changeset_type_reads_trailer() {
    let msg = "Subject\n\nBody.\n\nChangeset-Type: implementation\n";
    assert_eq!(
        parse_changeset_type(msg),
        Ok(Some(ChangesetType::Implementation))
    );
}

#[test]
fn parse_changeset_type_missing_returns_none() {
    let msg = "Subject\n\nBody with no trailer at all.\n";
    assert_eq!(parse_changeset_type(msg), Ok(None));
}

#[test]
fn parse_changeset_type_conflicting_values_errors() {
    let msg = "Subject\n\nChangeset-Type: implementation\nChangeset-Type: governance\n";
    assert!(parse_changeset_type(msg).is_err());
}

#[test]
fn parse_changeset_type_unrecognized_value_errors() {
    let msg = "Subject\n\nChangeset-Type: bogus\n";
    assert!(parse_changeset_type(msg).is_err());
}

/// Governance fix (M3 field-report): every line of the commit message is checked
/// against the `"changeset-type:"` prefix length (15 bytes) regardless of content, so
/// a multi-byte UTF-8 character (e.g. an em dash) landing at that exact byte offset in
/// some *other*, non-trailer line used to panic (`str::split_at` requires a char
/// boundary) instead of simply not matching. `—` (U+2014) is 3 bytes, so it straddles
/// byte offset 15 whenever it starts at byte 13 or 14 of a line — tried at every ASCII
/// prefix length from 0 to 16 so this test doesn't depend on hand-counting the exact
/// straddle point, reproducing the panic this regression test guards against
/// regardless of which offset the previous unconditional `split_at` actually broke on.
#[test]
fn parse_changeset_type_tolerates_multibyte_utf8_at_every_prefix_boundary() {
    for pad in 0..=16 {
        let line = format!("{}—line\n", "x".repeat(pad));
        let msg = format!("Subject\n\n{line}\nChangeset-Type: governance\n");
        assert_eq!(
            parse_changeset_type(&msg),
            Ok(Some(ChangesetType::Governance)),
            "panicked or misparsed with a {pad}-byte ASCII pad before the em dash"
        );
    }
}
