//! `xtask::corpus::{fetch_corpus, parity_check}`'s own self-tests for two M3 field-report
//! fixes:
//!
//! - DEFECT 4: `--only <id>` matching no committed contraption spec used to leave both
//!   `parity_check::run`'s replay loop and `fetch_corpus`'s spec list silently empty, which
//!   each verb's own aggregation then read as a vacuous `Pass` — `only_filter_matches_nothing`
//!   (shared by both verbs, `fetch_corpus.rs` reuses `parity_check.rs`'s own definition) is
//!   the pure check both `run` functions now gate on before any replay/oracle work starts.
//! - DEFECT 5: `fetch-corpus` never checked the project's own TEST-D41 EULA consent gate
//!   before launching a real vanilla oracle jar — `eula_gate` is that check, factored out of
//!   `fetch_corpus::run` and parameterized on `repo_root` so it is testable against a
//!   disposable temp directory instead of this real checkout's own `oracle/.eula-accepted`
//!   marker.

use std::path::PathBuf;

use xtask::corpus::fetch_corpus::eula_gate;
use xtask::corpus::parity_check::{only_filter_matches_nothing, sorted_ron_paths};

fn temp_repo_root(label: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "rc-xtask-fetch-corpus-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("must create temp repo root");
    root
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

fn real_corpus_dir() -> PathBuf {
    repo_root().join("crates/testing/gametest/corpus/redstone")
}

/// DEFECT 4's own control case: `--only` matching a real committed id must never be reported
/// as "matches nothing" — the failure mode this fix must not introduce in the other direction.
#[test]
fn only_filter_matches_a_real_committed_id() {
    let ron_paths = sorted_ron_paths(&real_corpus_dir()).expect("corpus dir must be readable");
    assert!(
        !ron_paths.is_empty(),
        "expected at least one committed .ron fixture under {}",
        real_corpus_dir().display()
    );
    assert!(!only_filter_matches_nothing(
        &ron_paths,
        "redstone/pulse/torch_inverter_basic"
    ));
}

/// DEFECT 4's own actual regression test: a value that matches no committed contraption spec
/// must be reported as matching nothing — this is the condition both `parity_check::run` and
/// `fetch_corpus::run` gate their own early, loud failure on.
#[test]
fn only_filter_reports_a_nonexistent_id_as_matching_nothing() {
    let ron_paths = sorted_ron_paths(&real_corpus_dir()).expect("corpus dir must be readable");
    assert!(only_filter_matches_nothing(
        &ron_paths,
        "redstone/does-not-exist"
    ));
}

/// DEFECT 5's own actual regression test: no marker, no env var, no `--accept-eula` — the
/// gate must refuse, before `fetch_corpus::run` ever reaches jar resolution or the oracle
/// launch. Explicitly clears `RC_ORACLE_EULA_ACCEPTED` first (mirrors `setup_oracle_consent.
/// rs`'s own identical defensive pattern) so this assertion can never be defeated by that
/// variable already being set in the ambient environment this test happens to run in.
#[test]
fn eula_gate_rejects_without_consent_or_flag() {
    unsafe {
        std::env::remove_var("RC_ORACLE_EULA_ACCEPTED");
    }
    let root = temp_repo_root("no-consent");
    let err = eula_gate(&root, false).expect_err("must reject without consent");
    assert!(err.contains("--accept-eula"), "{err}");
    assert!(
        !root.join("oracle/.eula-accepted").exists(),
        "a rejected gate must never record consent"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `--accept-eula` grants consent for this run and durably records it (mirrors `xtask setup-
/// oracle --accept-eula`'s own identical behavior, `setup_oracle::run`) — the marker file
/// must exist afterward, so a later invocation against the same `repo_root` never re-prompts.
#[test]
fn eula_gate_accepts_and_records_consent_with_the_flag() {
    let root = temp_repo_root("flag-grants");
    eula_gate(&root, true).expect("must accept with --accept-eula");
    assert!(root.join("oracle/.eula-accepted").exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// A repo root with consent already recorded (e.g. from an earlier `setup-oracle` run) must
/// never re-prompt, even without `--accept-eula` on this particular invocation — the same
/// "unattended after first run" behavior TEST-D41/D43 require of `setup-oracle` itself.
#[test]
fn eula_gate_accepts_when_consent_already_recorded() {
    let root = temp_repo_root("pre-recorded");
    std::fs::create_dir_all(root.join("oracle")).expect("must create oracle dir");
    std::fs::write(root.join("oracle/.eula-accepted"), b"").expect("must write marker");
    eula_gate(&root, false).expect("must accept when the marker already exists");
    let _ = std::fs::remove_dir_all(&root);
}
