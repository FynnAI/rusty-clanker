//! `xtask::corpus::protocol_diff::diff_into_result`'s own self-tests (M3.5-B03
//! governance changeset, TEST-D58 "protocol-differential harness execution shape",
//! Deliverable 1) — TEST-D45, written before the implementation changeset that adds
//! `diff_into_result`/`run_diff_only`/the `--diff-only` clap wiring themselves. Proves
//! the pure "diff two already-read captures into a `TierResult`" step this
//! changeset factors out runs exactly the same per-packet-type diff the existing
//! `--side both` path already runs (same normalization, same case-per-step-id
//! convention `protocol_diff.rs`'s own inline `tests::tier_result_shape` pins one
//! layer down against a hand-built `ProtocolDiffReport` map rather than real
//! `ProtocolCaptureFile`s), against small, hand-built in-memory captures — no files
//! on disk anywhere in this test file.

use rc_gametest::protocol_capture::{
    CapturedPacket, PROTOCOL_CAPTURE_FORMAT_VERSION, ProtocolCaptureFile, StepCapture,
};
use xtask::corpus::protocol_diff::diff_into_result;
use xtask::tier_result::{Status, TierResult};

/// `"block_update"` deliberately never appears in `protocol_capture::
/// NORMALIZATION_RULES` (checked against the module's own table) — its raw bytes are
/// compared byte-for-byte with no masking at all, which is exactly what these tests
/// need to observe a real, unmasked byte difference through `diff_into_result`.
fn packet(index: u32, packet_id: i32, body: Vec<u8>) -> CapturedPacket {
    CapturedPacket {
        index,
        packet_id,
        body,
        packet_name: Some("block_update".to_string()),
    }
}

fn capture(source_label: &str, steps: Vec<StepCapture>) -> ProtocolCaptureFile {
    ProtocolCaptureFile {
        format_version: PROTOCOL_CAPTURE_FORMAT_VERSION,
        source_label: source_label.to_string(),
        steps,
    }
}

#[test]
fn two_identical_captures_diff_to_an_all_pass_result() {
    let steps = vec![
        StepCapture {
            step_id: "session/spawn".to_string(),
            packets: vec![packet(0, 9, vec![1, 2, 3])],
        },
        StepCapture {
            step_id: "session/move".to_string(),
            packets: vec![packet(0, 11, vec![7, 7, 7])],
        },
    ];
    let oracle = capture("oracle:deadbeef", steps.clone());
    let ours = capture("ours", steps);

    let mut result = TierResult::new("protocol-diff");
    diff_into_result(&mut result, &oracle, &ours);
    let result = result.finalize();

    assert_eq!(result.status, Status::Pass);
    assert_eq!(result.cases.len(), 2);
    for case in &result.cases {
        assert_eq!(
            case.status,
            Status::Pass,
            "case {} unexpectedly failed",
            case.name
        );
        assert!(
            case.detail.is_none(),
            "case {} carried an unexpected detail: {:?}",
            case.name,
            case.detail
        );
    }
    let names: Vec<&str> = result.cases.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"session/spawn"));
    assert!(names.contains(&"session/move"));
}

#[test]
fn a_differing_packet_body_produces_exactly_that_steps_own_fail_case() {
    let oracle_steps = vec![
        StepCapture {
            step_id: "session/spawn".to_string(),
            packets: vec![packet(0, 9, vec![1, 2, 3])],
        },
        StepCapture {
            step_id: "session/move".to_string(),
            packets: vec![packet(0, 11, vec![7, 7, 7])],
        },
    ];
    let ours_steps = vec![
        StepCapture {
            step_id: "session/spawn".to_string(),
            // One byte differs from the oracle side — "block_update" is unmasked,
            // so this must surface as a real diff, not be normalized away.
            packets: vec![packet(0, 9, vec![1, 2, 4])],
        },
        StepCapture {
            step_id: "session/move".to_string(),
            packets: vec![packet(0, 11, vec![7, 7, 7])],
        },
    ];
    let oracle = capture("oracle:deadbeef", oracle_steps);
    let ours = capture("ours", ours_steps);

    let mut result = TierResult::new("protocol-diff");
    diff_into_result(&mut result, &oracle, &ours);
    let result = result.finalize();

    assert_eq!(result.status, Status::Fail);
    assert_eq!(result.cases.len(), 2);

    let spawn_case = result
        .cases
        .iter()
        .find(|c| c.name == "session/spawn")
        .expect("session/spawn case present");
    assert_eq!(spawn_case.status, Status::Fail);
    assert!(spawn_case.detail.is_some());

    let move_case = result
        .cases
        .iter()
        .find(|c| c.name == "session/move")
        .expect("session/move case present");
    assert_eq!(move_case.status, Status::Pass);
    assert!(move_case.detail.is_none());
}

#[test]
fn diff_into_result_only_ever_adds_diff_cases_never_replacing_existing_ones() {
    // `run_diff_only` pushes its own "capture-oracle"/"capture-ours" cases before
    // calling `diff_into_result` — this proves the pure function only ever appends,
    // never clears or otherwise disturbs whatever the caller already pushed.
    let steps = vec![StepCapture {
        step_id: "session/spawn".to_string(),
        packets: vec![packet(0, 9, vec![1, 2, 3])],
    }];
    let oracle = capture("oracle:deadbeef", steps.clone());
    let ours = capture("ours", steps);

    let mut result = TierResult::new("protocol-diff");
    result.push(
        "capture-oracle",
        Status::Pass,
        Some("from artifact captures/oracle/protocol-diff-oracle.postcard".to_string()),
    );
    result.push(
        "capture-ours",
        Status::Pass,
        Some("from artifact captures/ours/protocol-diff-ours.postcard".to_string()),
    );
    diff_into_result(&mut result, &oracle, &ours);
    let result = result.finalize();

    assert_eq!(result.status, Status::Pass);
    assert_eq!(result.cases.len(), 3);
    assert_eq!(result.cases[0].name, "capture-oracle");
    assert_eq!(result.cases[1].name, "capture-ours");
    assert_eq!(result.cases[2].name, "session/spawn");
}
