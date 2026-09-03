//! `xtask::corpus::protocol_diff`'s own capture-completeness self-tests (M3.5-B03
//! governance changeset, "a capture with missing contraptions is not a pass") —
//! written before the implementation changeset that adds `evaluate_completeness`/
//! `CaptureCompleteness`/`capture_fail_detail` themselves (TEST-D45).
//! TEST-D48/TEST-D50: a verification that reports success while having verified
//! only part of what it claims is exactly the failure mode these tests exist to
//! prevent — every case below is pure arithmetic over a hand-built
//! `ProgressSummary`, no subprocess, no live server, mirroring
//! `protocol_diff_progress_parsing.rs`'s own established conventions.

use xtask::corpus::protocol_diff::{
    CompletedEntry, ProgressSummary, capture_fail_detail, evaluate_completeness,
};

fn summary_with(completed_ids: &[&str], failed: Option<u64>) -> ProgressSummary {
    ProgressSummary {
        steps_total: None,
        contraptions_total: None,
        completed: completed_ids
            .iter()
            .map(|id| CompletedEntry {
                id: id.to_string(),
                ms: 1,
            })
            .collect(),
        total_ms: Some(1),
        failed,
    }
}

#[test]
fn every_expected_step_and_contraption_done_with_zero_failed_is_complete() {
    let summary = summary_with(
        &[
            "session/login",
            "session/move",
            "redstone/clock/torch_clock_classic",
        ],
        Some(0),
    );
    let completeness = evaluate_completeness(&summary, 2, 1);
    assert!(completeness.is_complete());
}

#[test]
fn one_missing_contraption_is_incomplete_with_a_k_of_m_detail() {
    // 2 of 2 expected session steps done, but only 1 of 2 expected contraptions.
    let summary = summary_with(
        &[
            "session/login",
            "session/move",
            "redstone/clock/torch_clock_classic",
        ],
        Some(0),
    );
    let completeness = evaluate_completeness(&summary, 2, 2);
    assert!(!completeness.is_complete());
    assert_eq!(
        completeness.detail(),
        "captured 1 of 2 contraptions and 2 of 2 session steps (0 failed)"
    );
}

#[test]
fn any_nonzero_failed_count_is_incomplete_even_with_every_done_line_present() {
    let summary = summary_with(
        &["session/login", "redstone/clock/torch_clock_classic"],
        Some(1),
    );
    let completeness = evaluate_completeness(&summary, 1, 1);
    assert!(!completeness.is_complete());
    assert_eq!(
        completeness.detail(),
        "captured 1 of 1 contraptions and 1 of 1 session steps (1 failed)"
    );
}

#[test]
fn a_missing_failed_field_defaults_to_zero() {
    let summary = summary_with(&["session/login"], None);
    let completeness = evaluate_completeness(&summary, 1, 0);
    assert!(completeness.is_complete());
    assert_eq!(completeness.failed, 0);
}

#[test]
fn capture_fail_detail_appends_the_last_failure_line_from_stderr() {
    let summary = summary_with(&["session/login"], Some(1));
    let completeness = evaluate_completeness(&summary, 2, 0);
    let stderr = "\
protocol_session: session/move failed: timed out walking to (3, 1, 3)
redstone_wire_capture: redstone/clock/torch_clock_classic failed: timed out walking to (20, 65, 7)
";
    let detail = capture_fail_detail(&completeness, stderr);
    assert_eq!(
        detail,
        "captured 0 of 0 contraptions and 1 of 2 session steps (1 failed): \
         redstone_wire_capture: redstone/clock/torch_clock_classic failed: timed out walking to (20, 65, 7)"
    );
}

#[test]
fn capture_fail_detail_omits_the_trailing_clause_when_stderr_has_no_failure_line() {
    let summary = summary_with(&["session/login"], Some(0));
    let completeness = evaluate_completeness(&summary, 2, 0);
    let detail = capture_fail_detail(
        &completeness,
        "protocol-diff-runner: begin oracle steps=2 contraptions=0\n",
    );
    assert_eq!(
        detail,
        "captured 0 of 0 contraptions and 1 of 2 session steps (0 failed)"
    );
}

#[test]
fn contraption_done_lines_are_distinguished_from_session_step_done_lines_by_prefix() {
    // Two "redstone/..." ids and one "session/..." id — done counts must split
    // 2 contraptions / 1 step, never lumped together.
    let summary = summary_with(
        &[
            "session/spawn",
            "redstone/piston/piston_max_push_depth_12",
            "redstone/wire/wire_signal_decay_15_chain",
        ],
        Some(0),
    );
    let completeness = evaluate_completeness(&summary, 1, 2);
    assert_eq!(completeness.steps_done, 1);
    assert_eq!(completeness.contraptions_done, 2);
    assert!(completeness.is_complete());
}
