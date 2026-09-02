use xtask::forbidden_patterns::{
    LintGate, PatternViolation, check_empty_test_body, check_hardcoded_block_state_literal,
    check_tautological_assertion, check_undocumented_tier_cfg, check_unlinked_ignore,
    check_weakened_tests, commit_lint_gate,
};
use xtask::path_guard::ChangesetType;

// The per-commit gate (mirrors path_guard::evaluate_commit): a trailer-less docs-only
// commit is skipped, not failed — the regression that turned CI red on a pure
// completion-report push (run 33022662264) even though the path-guard passed it.
#[test]
fn gate_skips_a_trailerless_docs_only_commit() {
    let gate = commit_lint_gate(
        "Docs update, no trailer.\n",
        &["blueprints/M2/M2-COMPLETION-REPORT.md".to_string()],
    );
    assert!(matches!(gate, LintGate::Skip(_)));
}

#[test]
fn gate_fails_a_trailerless_code_commit() {
    let gate = commit_lint_gate(
        "Code change, no trailer.\n",
        &["crates/core/src/lib.rs".to_string()],
    );
    assert!(matches!(gate, LintGate::Fail(_)));
}

#[test]
fn gate_lints_under_the_commits_own_type() {
    let gate = commit_lint_gate(
        "Subject\n\nChangeset-Type: test-authoring\n",
        &["crates/server/tests/foo.rs".to_string()],
    );
    assert_eq!(gate, LintGate::Lint(ChangesetType::TestAuthoring));
}

#[test]
fn gate_skips_an_empty_commit() {
    assert!(matches!(
        commit_lint_gate("Subject only.\n", &[]),
        LintGate::Skip(_)
    ));
}

#[test]
fn gate_fails_a_malformed_trailer() {
    let gate = commit_lint_gate(
        "Subject\n\nChangeset-Type: bogus\n",
        &["crates/core/src/lib.rs".to_string()],
    );
    assert!(matches!(gate, LintGate::Fail(_)));
}

#[test]
fn bare_ignore_is_flagged() {
    let v = check_unlinked_ignore("f.rs", &["#[ignore]".to_string()]);
    assert_eq!(v.len(), 1);
}

#[test]
fn ignore_with_issue_number_is_allowed() {
    let v = check_unlinked_ignore("f.rs", &[r#"#[ignore = "flaky, see #142"]"#.to_string()]);
    assert_eq!(v.len(), 0);
}

#[test]
fn ignore_with_issues_url_is_allowed() {
    let v = check_unlinked_ignore(
        "f.rs",
        &[r#"#[ignore = "https://github.com/org/repo/issues/142"]"#.to_string()],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn ignore_with_reason_but_no_link_is_flagged() {
    let v = check_unlinked_ignore("f.rs", &[r#"#[ignore = "flaky test"]"#.to_string()]);
    assert_eq!(v.len(), 1);
}

#[test]
fn assert_true_is_flagged() {
    let v = check_tautological_assertion("f.rs", &["assert!(true);".to_string()]);
    assert_eq!(v.len(), 1);
}

#[test]
fn assert_eq_true_true_is_flagged() {
    let v = check_tautological_assertion("f.rs", &["assert_eq!(true, true);".to_string()]);
    assert_eq!(v.len(), 1);
}

#[test]
fn normal_assert_is_not_flagged() {
    let v = check_tautological_assertion("f.rs", &["assert_eq!(result, 42);".to_string()]);
    assert_eq!(v.len(), 0);
}

#[test]
fn empty_test_body_is_flagged() {
    let v = check_empty_test_body("f.rs", "#[test]\nfn does_nothing() {\n}\n");
    assert_eq!(v.len(), 1);
    match &v[0] {
        PatternViolation::EmptyTestBody { fn_name, .. } => assert_eq!(fn_name, "does_nothing"),
        other => panic!("expected EmptyTestBody, got {other:?}"),
    }
}

#[test]
fn todo_only_body_is_flagged() {
    let v = check_empty_test_body("f.rs", "#[test]\nfn stub() {\n    todo!();\n}\n");
    assert_eq!(v.len(), 1);
}

#[test]
fn real_test_body_is_not_flagged() {
    let v = check_empty_test_body(
        "f.rs",
        "#[test]\nfn real() {\n    assert_eq!(1 + 1, 2);\n}\n",
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn undocumented_cfg_before_test_is_flagged() {
    let v = check_undocumented_tier_cfg(
        "f.rs",
        &[
            "#[cfg(not(feature = \"slow\"))]".to_string(),
            "#[test]".to_string(),
            "fn foo() {}".to_string(),
        ],
    );
    assert_eq!(v.len(), 1);
}

#[test]
fn documented_cfg_before_test_is_allowed() {
    let v = check_undocumented_tier_cfg(
        "f.rs",
        &[
            "// tier-change-reviewed: #201".to_string(),
            "#[cfg(not(feature = \"slow\"))]".to_string(),
            "#[test]".to_string(),
        ],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn deleted_test_in_implementation_changeset_is_flagged() {
    let v = check_weakened_tests(
        "f.rs",
        "#[test]\nfn keep_me() {}\n#[test]\nfn remove_me() {}\n",
        "#[test]\nfn keep_me() {}\n",
        ChangesetType::Implementation,
    );
    assert_eq!(v.len(), 1);
    match &v[0] {
        PatternViolation::DeletedTest { fn_name, .. } => assert_eq!(fn_name, "remove_me"),
        other => panic!("expected DeletedTest, got {other:?}"),
    }
}

#[test]
fn deleted_test_in_test_authoring_changeset_is_allowed() {
    let v = check_weakened_tests(
        "f.rs",
        "#[test]\nfn keep_me() {}\n#[test]\nfn remove_me() {}\n",
        "#[test]\nfn keep_me() {}\n",
        ChangesetType::TestAuthoring,
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn assertion_count_regression_in_impl_changeset_is_flagged() {
    let base = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn a() {\n        assert_eq!(1, 1);\n        assert_eq!(2, 2);\n        assert_eq!(3, 3);\n    }\n}\n";
    let head = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn a() {\n        assert_eq!(1, 1);\n    }\n}\n";
    let v = check_weakened_tests("f.rs", base, head, ChangesetType::Implementation);
    assert_eq!(v.len(), 1);
    match &v[0] {
        PatternViolation::AssertionCountRegression { before, after, .. } => {
            assert_eq!(*before, 3);
            assert_eq!(*after, 1);
        }
        other => panic!("expected AssertionCountRegression, got {other:?}"),
    }
}

#[test]
fn assertion_count_increase_is_allowed() {
    let base = "assert_eq!(1, 1);\n";
    let head = "assert_eq!(1, 1);\nassert_eq!(2, 2);\n";
    let v = check_weakened_tests("f.rs", base, head, ChangesetType::Implementation);
    assert_eq!(v.len(), 0);
}

// --- M3.5-B01's own cases: check 6, `check_hardcoded_block_state_literal`. ---

#[test]
fn block_state_id_constructor_literal_is_flagged() {
    let v = check_hardcoded_block_state_literal(
        "crates/mechanics/src/redstone/registration.rs",
        &["pub const REDSTONE_BLOCK_STATE_ID: BlockStateId = BlockStateId(11311);".to_string()],
    );
    assert_eq!(v.len(), 1);
}

#[test]
fn block_state_id_constructor_with_variable_arg_is_not_flagged() {
    let v = check_hardcoded_block_state_literal(
        "crates/mechanics/src/redstone/registration.rs",
        &["BlockStateId(default.0)".to_string()],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn bare_u32_const_with_block_state_comment_is_flagged() {
    let v = check_hardcoded_block_state_literal(
        "crates/mechanics/src/redstone/wire.rs",
        &[
            "// redstone_wire block-state ids, protocol 776".to_string(),
            "const WIRE_MAX: u32 = 5306;".to_string(),
        ],
    );
    assert_eq!(v.len(), 1);
}

#[test]
fn bare_u32_const_with_unrelated_comment_is_not_flagged() {
    let v = check_hardcoded_block_state_literal(
        "crates/server/src/net/limits.rs",
        &["const MAX_PLAYERS: u32 = 5306;".to_string()],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn literal_range_with_block_state_comment_is_flagged() {
    let v = check_hardcoded_block_state_literal(
        "crates/mechanics/src/redstone/repeater.rs",
        &[
            "// repeater block-state range".to_string(),
            "const REPEATER_RANGE: (u32, u32) = (7034, 7097);".to_string(),
        ],
    );
    assert_eq!(v.len(), 1);
}

#[test]
fn generated_path_is_never_flagged() {
    let v = check_hardcoded_block_state_literal(
        "crates/registries/generated/v776/block_states.rs",
        &["pub const REDSTONE_BLOCK_STATE_ID: BlockStateId = BlockStateId(11311);".to_string()],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn tests_path_is_never_flagged() {
    let v = check_hardcoded_block_state_literal(
        "crates/mechanics/tests/redstone_repeater.rs",
        &["pub const REDSTONE_BLOCK_STATE_ID: BlockStateId = BlockStateId(11311);".to_string()],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn waiver_comment_on_same_line_suppresses() {
    let v = check_hardcoded_block_state_literal(
        "crates/mechanics/src/redstone/wire.rs",
        &[
            "const WIRE_MAX: u32 = 5306; // block-state-id-lint-waiver: pending M3.5-B02 retirement"
                .to_string(),
        ],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn waiver_comment_on_preceding_line_suppresses() {
    let v = check_hardcoded_block_state_literal(
        "crates/mechanics/src/redstone/wire.rs",
        &[
            "// block-state-id-lint-waiver: pending M3.5-B02 retirement".to_string(),
            "const WIRE_MAX: u32 = 5306; // redstone_wire".to_string(),
        ],
    );
    assert_eq!(v.len(), 0);
}
