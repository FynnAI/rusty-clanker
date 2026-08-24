use xtask::forbidden_patterns::{
    PatternViolation, check_empty_test_body, check_tautological_assertion,
    check_undocumented_tier_cfg, check_unlinked_ignore, check_weakened_tests,
};
use xtask::path_guard::ChangesetType;

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
    let head =
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn a() {\n        assert_eq!(1, 1);\n    }\n}\n";
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
