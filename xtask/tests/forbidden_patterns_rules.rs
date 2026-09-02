use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use xtask::forbidden_patterns::{
    LintGate, PatternViolation, check_empty_test_body, check_hardcoded_block_state_literal,
    check_raw_stdio_piped, check_raw_stdio_piped_whole_tree, check_tautological_assertion,
    check_undocumented_tier_cfg, check_unlinked_ignore, check_weakened_tests, commit_lint_gate,
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

// --- M3.5-B06's own cases: check 7, `check_raw_stdio_piped`. ---

#[test]
fn bare_stdio_piped_under_xtask_src_is_flagged() {
    let v = check_raw_stdio_piped(
        "xtask/src/anything_else.rs",
        &[".stdout(Stdio::piped())".to_string()],
    );
    assert_eq!(v.len(), 1);
}

#[test]
fn bare_stdio_piped_inside_process_rs_itself_is_not_flagged() {
    let v = check_raw_stdio_piped(
        "xtask/src/process.rs",
        &[".stdout(Stdio::piped())".to_string()],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn stdio_piped_outside_xtask_src_is_not_flagged() {
    let v = check_raw_stdio_piped(
        "crates/testing/paritybot/src/main.rs",
        &[".stdout(Stdio::piped())".to_string()],
    );
    assert_eq!(v.len(), 0);
}

#[test]
fn stdio_piped_mentioned_only_in_a_string_literal_is_not_flagged() {
    let v = check_raw_stdio_piped(
        "xtask/src/anything_else.rs",
        &["// never write \"Stdio::piped()\" outside process.rs".to_string()],
    );
    assert_eq!(v.len(), 0);
}

// --- `check_raw_stdio_piped_whole_tree`: the whole-tree companion (M3.5-B06
// field-report fix). `check_raw_stdio_piped` above only ever sees one commit's own
// diff-added lines, so a pre-existing occurrence added by some earlier, already-merged
// commit is invisible to it forever -- exactly the gap that let `m3_5_be_report.rs`'s
// own raw pipe survive B06's original centralization undetected. `check_raw_stdio_
// piped_whole_tree` instead reads every `xtask/src/**/*.rs` file's *current* on-disk
// content relative to the process's own cwd, so these tests use the same `TempCwd`
// chdir idiom `xtask/tests/verify_claims_cli.rs` already established for a
// CWD-relative xtask function, rather than `check_raw_stdio_piped`'s own in-memory
// `&[String]` fixture style above (there is no line list to hand it — chdir is the
// fixture).

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// See `xtask/tests/verify_claims_cli.rs`'s own `CWD_LOCK` doc comment: `cargo test`
/// (libtest) runs every test in this file's binary in one process with thread-based
/// parallelism by default, so a `std::env::set_current_dir` in one test can otherwise
/// race another's; `cargo nextest run` gives every `#[test]` its own OS process and
/// needs no such lock, but this serializes correctly under either runner.
static CWD_LOCK: Mutex<()> = Mutex::new(());

/// A fresh temp dir containing an empty `xtask/src/` (mirroring the real repo layout
/// `check_raw_stdio_piped_whole_tree` walks), chdir'd into for the duration of one test
/// (see `CWD_LOCK`). Restored and removed on `Drop`.
struct TempCwd {
    _lock: std::sync::MutexGuard<'static, ()>,
    original: PathBuf,
    dir: PathBuf,
}

impl TempCwd {
    fn new(label: &str) -> Self {
        let lock = CWD_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let original = std::env::current_dir().expect("current dir");
        let dir = std::env::temp_dir().join(format!(
            "rc-xtask-raw-stdio-piped-whole-tree-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("xtask/src")).expect("create fixture xtask/src");
        std::env::set_current_dir(&dir).expect("chdir into temp dir");
        Self {
            _lock: lock,
            original,
            dir,
        }
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).expect("create parent dirs");
        std::fs::write(&path, content).expect("write fixture file");
    }
}

impl Drop for TempCwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn whole_tree_check_flags_a_pre_existing_occurrence_no_commit_diff_would_see() {
    let cwd = TempCwd::new("flags-pre-existing");
    // Nothing in `added_lines` for any commit ever mentions this file -- it's simply
    // sitting on disk, exactly like a violation introduced before this lint existed.
    cwd.write(
        "xtask/src/probe.rs",
        "fn spawn() {\n    Command::new(\"x\").stdout(Stdio::piped());\n}\n",
    );

    let violations = check_raw_stdio_piped_whole_tree();
    assert_eq!(violations.len(), 1);
    match &violations[0] {
        PatternViolation::RawStdioPiped { file, .. } => {
            assert_eq!(file, "xtask/src/probe.rs");
        }
        other => panic!("expected RawStdioPiped, got {other:?}"),
    }
}

#[test]
fn whole_tree_check_still_excludes_process_rs() {
    let cwd = TempCwd::new("excludes-process-rs");
    cwd.write(
        "xtask/src/process.rs",
        "fn spawn() {\n    Command::new(\"x\").stdout(Stdio::piped());\n}\n",
    );

    let violations = check_raw_stdio_piped_whole_tree();
    assert_eq!(violations, Vec::new());
}

#[test]
fn whole_tree_check_passes_a_clean_tree() {
    let cwd = TempCwd::new("clean-tree");
    cwd.write(
        "xtask/src/nested/mod.rs",
        "fn spawn() {\n    Command::new(\"x\").stdout(Stdio::inherit());\n}\n",
    );

    let violations = check_raw_stdio_piped_whole_tree();
    assert_eq!(violations, Vec::new());
}
