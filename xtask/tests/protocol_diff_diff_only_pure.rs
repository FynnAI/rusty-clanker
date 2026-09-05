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

use rc_gametest::known_divergences::{DivergenceClass, KnownDivergence};
use rc_gametest::protocol_capture::{
    CapturedPacket, PROTOCOL_CAPTURE_FORMAT_VERSION, ProtocolCaptureFile, StepCapture,
};
use xtask::corpus::protocol_diff::diff_into_result;
use xtask::tier_result::{Status, TierResult};

/// A fresh, uniquely named temp directory `diff_into_result`'s own `repo_root`
/// parameter can point at — its own `write_bodies_dump` needs somewhere to write
/// `target/verify/protocol-diff-bodies.json` under, and this crate's own tests must
/// never write into the real repository checkout.
fn temp_repo_root(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rc-protocol-diff-pure-test-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

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
            observe_from: 0,
            packets: vec![packet(0, 9, vec![1, 2, 3])],
        },
        StepCapture {
            step_id: "session/move".to_string(),
            observe_from: 0,
            packets: vec![packet(0, 11, vec![7, 7, 7])],
        },
    ];
    let oracle = capture("oracle:deadbeef", steps.clone());
    let ours = capture("ours", steps);

    let mut result = TierResult::new("protocol-diff");
    let repo_root = temp_repo_root("all-pass");
    diff_into_result(&mut result, &oracle, &ours, &[], &repo_root);
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
            observe_from: 0,
            packets: vec![packet(0, 9, vec![1, 2, 3])],
        },
        StepCapture {
            step_id: "session/move".to_string(),
            observe_from: 0,
            packets: vec![packet(0, 11, vec![7, 7, 7])],
        },
    ];
    let ours_steps = vec![
        StepCapture {
            step_id: "session/spawn".to_string(),
            observe_from: 0,
            // One byte differs from the oracle side — "block_update" is unmasked,
            // so this must surface as a real diff, not be normalized away.
            packets: vec![packet(0, 9, vec![1, 2, 4])],
        },
        StepCapture {
            step_id: "session/move".to_string(),
            observe_from: 0,
            packets: vec![packet(0, 11, vec![7, 7, 7])],
        },
    ];
    let oracle = capture("oracle:deadbeef", oracle_steps);
    let ours = capture("ours", ours_steps);

    let mut result = TierResult::new("protocol-diff");
    let repo_root = temp_repo_root("differing-body");
    diff_into_result(&mut result, &oracle, &ours, &[], &repo_root);
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
        observe_from: 0,
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
    let repo_root = temp_repo_root("append-only");
    diff_into_result(&mut result, &oracle, &ours, &[], &repo_root);
    let result = result.finalize();

    assert_eq!(result.status, Status::Pass);
    assert_eq!(result.cases.len(), 3);
    assert_eq!(result.cases[0].name, "capture-oracle");
    assert_eq!(result.cases[1].name, "capture-ours");
    assert_eq!(result.cases[2].name, "session/spawn");
}

#[test]
fn a_registered_divergence_passes_with_a_known_detail_and_an_unregistered_one_still_fails() {
    // Two step ids: `session/spawn` diverges by a packet type covered by a `Missing`
    // register entry (must pass, with a "known" detail); `session/move` diverges by
    // an *unregistered* packet type (must still fail) — proves resolution is applied
    // per packet type, not blanket per step.
    let oracle = capture(
        "oracle:deadbeef",
        vec![
            StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![CapturedPacket {
                    index: 0,
                    packet_id: 55,
                    body: vec![1, 2, 3],
                    packet_name: Some("commands".to_string()),
                }],
            },
            StepCapture {
                step_id: "session/move".to_string(),
                observe_from: 0,
                packets: vec![packet(0, 200, vec![9, 9, 9])],
            },
        ],
    );
    let ours = capture(
        "ours",
        vec![
            StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![],
            },
            StepCapture {
                step_id: "session/move".to_string(),
                observe_from: 0,
                packets: vec![packet(0, 200, vec![9, 9, 8])],
            },
        ],
    );

    let register = vec![KnownDivergence {
        steps: "session/spawn".to_string(),
        packet: "minecraft:commands".to_string(),
        class: DivergenceClass::Missing,
        closes_with: Some("NET hardening: join sequence".to_string()),
        expires: Some("M5".to_string()),
    }];

    let mut result = TierResult::new("protocol-diff");
    let repo_root = temp_repo_root("registered-vs-unregistered");
    diff_into_result(&mut result, &oracle, &ours, &register, &repo_root);
    let result = result.finalize();

    let spawn_case = result
        .cases
        .iter()
        .find(|c| c.name == "session/spawn")
        .expect("session/spawn case present");
    assert_eq!(spawn_case.status, Status::Pass);
    let detail = spawn_case.detail.as_deref().expect("known detail present");
    assert!(detail.contains("known"), "unexpected detail: {detail}");
    assert!(
        detail.contains("NET hardening: join sequence"),
        "unexpected detail: {detail}"
    );
    assert!(detail.contains("expires M5"), "unexpected detail: {detail}");

    let move_case = result
        .cases
        .iter()
        .find(|c| c.name == "session/move")
        .expect("session/move case present");
    assert_eq!(move_case.status, Status::Fail);

    assert_eq!(result.status, Status::Fail);
}

#[test]
fn a_body_mismatch_detail_never_carries_the_full_body_bytes() {
    // A 200-byte body differing on both sides — the compact case detail
    // (Deliverable 3) must never contain the full 200-byte hex dump, only a
    // 32-byte-max preview plus the real length.
    let big_oracle_body: Vec<u8> = (0..200u16).map(|n| (n % 256) as u8).collect();
    let big_ours_body: Vec<u8> = (0..200u16).map(|n| ((n + 1) % 256) as u8).collect();

    let oracle = capture(
        "oracle:deadbeef",
        vec![StepCapture {
            step_id: "session/spawn".to_string(),
            observe_from: 0,
            packets: vec![packet(0, 9, big_oracle_body.clone())],
        }],
    );
    let ours = capture(
        "ours",
        vec![StepCapture {
            step_id: "session/spawn".to_string(),
            observe_from: 0,
            packets: vec![packet(0, 9, big_ours_body.clone())],
        }],
    );

    let mut result = TierResult::new("protocol-diff");
    let repo_root = temp_repo_root("bounded-detail");
    diff_into_result(&mut result, &oracle, &ours, &[], &repo_root);
    let result = result.finalize();

    let spawn_case = result
        .cases
        .iter()
        .find(|c| c.name == "session/spawn")
        .expect("session/spawn case present");
    assert_eq!(spawn_case.status, Status::Fail);
    let detail = spawn_case.detail.as_deref().expect("detail present");

    // The detail must stay small (a handful of lines' worth of text), never
    // proportional to the 200-byte body — this is the exact defect the first real
    // full diff hit (a 135 MB report).
    assert!(
        detail.len() < 1024,
        "case detail is not compact: {} bytes: {detail}",
        detail.len()
    );
    // It must still name the body's own real, full length...
    assert!(detail.contains("len=200"), "detail: {detail}");
    // ...and it must never contain a hex run long enough to encode the full
    // 200-byte body (a 32-byte preview is at most 64 hex characters).
    let longest_hex_run = detail
        .split(|c: char| !c.is_ascii_hexdigit())
        .map(str::len)
        .max()
        .unwrap_or(0);
    assert!(
        longest_hex_run <= 64,
        "detail contains a hex run of {longest_hex_run} chars — looks like a full body \
         dump, not a bounded preview: {detail}"
    );
}
