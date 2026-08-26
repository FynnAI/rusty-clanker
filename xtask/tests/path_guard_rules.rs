use xtask::path_guard::{
    ChangesetType, check_paths, docs_only_exemption, glob_match, parse_changeset_type,
};

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
