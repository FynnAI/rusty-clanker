//! `xtask::corpus::protocol_diff::parse_progress_lines`'s own self-tests (M3.5-B03
//! governance changeset, "protocol-diff-runner progress lines") — TEST-D45, written
//! before the implementation changeset that adds `parse_progress_lines`/
//! `ProgressSummary`/`CompletedEntry` themselves. Pins the exact stable, parseable
//! prefix + key=value/fixed-position-field shape `protocol_diff_runner`'s own
//! `begin`/`done`/`finished` stderr lines use, so a future change to that format
//! breaks a test here rather than only showing up as a silently empty
//! `target/verify/protocol-diff-timings.json`.

use xtask::corpus::protocol_diff::parse_progress_lines;

#[test]
fn empty_stderr_yields_an_empty_summary() {
    let summary = parse_progress_lines("");
    assert_eq!(summary.steps_total, None);
    assert_eq!(summary.contraptions_total, None);
    assert!(summary.completed.is_empty());
    assert_eq!(summary.total_ms, None);
}

#[test]
fn unrelated_stderr_lines_are_ignored() {
    let stderr = "warning: unused variable\nprotocol_session: session/move failed: timeout\n";
    let summary = parse_progress_lines(stderr);
    assert_eq!(summary.steps_total, None);
    assert!(summary.completed.is_empty());
}

#[test]
fn begin_line_populates_both_totals() {
    let stderr = "protocol-diff-runner: begin oracle steps=32 contraptions=51\n";
    let summary = parse_progress_lines(stderr);
    assert_eq!(summary.steps_total, Some(32));
    assert_eq!(summary.contraptions_total, Some(51));
}

#[test]
fn done_lines_accumulate_in_order() {
    let stderr = "\
protocol-diff-runner: begin ours steps=32 contraptions=1
protocol-diff-runner: done session/login in 12 ms
protocol-diff-runner: done session/configuration in 4 ms
protocol-diff-runner: done redstone/comparator/comparator_2tick_fixed_delay in 987 ms
";
    let summary = parse_progress_lines(stderr);
    assert_eq!(summary.completed.len(), 3);
    assert_eq!(summary.completed[0].id, "session/login");
    assert_eq!(summary.completed[0].ms, 12);
    assert_eq!(summary.completed[1].id, "session/configuration");
    assert_eq!(summary.completed[1].ms, 4);
    assert_eq!(
        summary.completed[2].id,
        "redstone/comparator/comparator_2tick_fixed_delay"
    );
    assert_eq!(summary.completed[2].ms, 987);
}

#[test]
fn finished_line_populates_total_ms() {
    let stderr = "protocol-diff-runner: finished ours total_ms=45231\n";
    let summary = parse_progress_lines(stderr);
    assert_eq!(summary.total_ms, Some(45231));
}

#[test]
fn a_full_realistic_transcript_parses_end_to_end() {
    let stderr = "\
protocol-diff-runner: begin oracle steps=32 contraptions=51
protocol-diff-runner: done session/login in 8 ms
protocol-diff-runner: done session/spawn in 1520 ms
protocol-diff-runner: done redstone/piston/piston_extend in 640 ms
protocol-diff-runner: finished oracle total_ms=198340
";
    let summary = parse_progress_lines(stderr);
    assert_eq!(summary.steps_total, Some(32));
    assert_eq!(summary.contraptions_total, Some(51));
    assert_eq!(summary.completed.len(), 3);
    assert_eq!(summary.total_ms, Some(198340));
}

#[test]
fn a_line_missing_the_expected_fields_is_skipped_rather_than_panicking() {
    let stderr = "protocol-diff-runner: done\nprotocol-diff-runner: done session/move nonsense\n";
    let summary = parse_progress_lines(stderr);
    assert!(summary.completed.is_empty());
}

#[test]
fn stray_leading_whitespace_on_a_progress_line_is_still_recognized() {
    let stderr = "   protocol-diff-runner: done session/sneak in 55 ms\n";
    let summary = parse_progress_lines(stderr);
    assert_eq!(summary.completed.len(), 1);
    assert_eq!(summary.completed[0].id, "session/sneak");
}

/// A truncated (mid-write, right where a timeout's own kill would land) `done` line
/// never panics — treated exactly like any other malformed line, skipped.
#[test]
fn a_truncated_trailing_line_is_skipped_rather_than_panicking() {
    let stderr =
        "protocol-diff-runner: done session/place/stone in 210 ms\nprotocol-diff-runner: done sess";
    let summary = parse_progress_lines(stderr);
    assert_eq!(summary.completed.len(), 1);
    assert_eq!(summary.completed[0].id, "session/place/stone");
}
