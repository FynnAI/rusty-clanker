use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use xtask::tier_result::{Status, TierResult};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// `verify_claims::run` resolves `blueprints/<milestone>` relative to the real process
/// cwd (matching `tier_result::VERIFY_OUT_DIR`'s own "target/verify" convention -- see
/// that module's own doc comment), so isolating one test's fixture tree means actually
/// chdir-ing the process. `cargo-nextest` gives every `#[test]` its own OS process, so
/// that chdir cannot race another test there -- but `cargo test` (libtest) runs every
/// test in one binary in the SAME process with thread-based parallelism by default, and
/// this project's own verification commands run both (`cargo nextest run -p xtask` and
/// `cargo test -p xtask`), so this lock serializes every `TempCwd`-using test
/// regardless of which runner drives them. A poisoned lock (an earlier test panicked
/// while holding it) is recovered rather than propagated -- the guarded state is
/// trivially `()`, so there is nothing to actually be inconsistent.
static CWD_LOCK: Mutex<()> = Mutex::new(());

/// A fresh temp dir, chdir'd into for the duration of one test (see `CWD_LOCK`).
/// Restored and removed on `Drop`.
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
            "rc-xtask-verify-claims-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
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

    fn read_result(&self) -> TierResult {
        let path = self.dir.join("target/verify/verify-claims.json");
        let text = std::fs::read_to_string(&path).expect("read verify-claims.json");
        serde_json::from_str(&text).expect("parse verify-claims.json")
    }
}

impl Drop for TempCwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

const EXEMPT_BLUEPRINT: &str = "# M9-B01 — Foo\n\n### Claims to verify (TEST-D57)\n\n- None.\n";

fn required_blueprint(id: &str, claims: &[&str]) -> String {
    let mut s = format!(
        "# {id} — Bar\n\n\
         | Field | Content |\n\
         |---|---|\n\
         | ID | `{id}` |\n\n\
         ### Claims to verify (TEST-D57)\n\n"
    );
    for c in claims {
        s.push_str(&format!("- {c}\n"));
    }
    s
}

fn claims_file(id: &str, rows: &[(&str, &str)]) -> String {
    let mut s = format!(
        "# {id} — Claims Verified (TEST-D57)\n\n\
         | Claim | Source location | Verdict | Verified by | Date |\n\
         |---|---|---|---|---|\n"
    );
    for (claim, verdict) in rows {
        s.push_str(&format!(
            "| {claim} | blocks.json | {verdict} | tester | 2026-09-02 |\n"
        ));
    }
    s
}

#[test]
fn run_passes_when_every_blueprint_is_exempt_or_exactly_matched() {
    let cwd = TempCwd::new("pass");
    cwd.write("blueprints/M9/M9-B01-foo.md", EXEMPT_BLUEPRINT);
    cwd.write(
        "blueprints/M9/M9-B02-bar.md",
        &required_blueprint("M9-B02", &["claim one", "claim two"]),
    );
    cwd.write(
        "blueprints/M9/M9-B02-CLAIMS.md",
        &claims_file(
            "M9-B02",
            &[("claim one", "CONFIRMED"), ("claim two", "CONFIRMED")],
        ),
    );

    // `ExitCode` has no public equality/introspection API in stable Rust -- the
    // written result JSON's own `status` field is the authoritative, testable signal
    // (`tier_result::exit_code_for` derives the exit code purely from that field).
    let _code = xtask::verify_claims::run("M9");
    let result = cwd.read_result();
    assert_eq!(result.status, Status::Pass, "cases: {:?}", result.cases);
}

#[test]
fn run_fails_on_a_missing_claims_file() {
    let cwd = TempCwd::new("missing-claims");
    cwd.write(
        "blueprints/M9/M9-B01-foo.md",
        &required_blueprint("M9-B01", &["claim one"]),
    );

    let _code = xtask::verify_claims::run("M9");
    let result = cwd.read_result();
    assert_eq!(result.status, Status::Fail);
    assert!(result.cases.iter().any(|c| {
        c.detail
            .as_deref()
            .unwrap_or("")
            .contains("missing CLAIMS file")
    }));
}

#[test]
fn run_fails_on_a_row_count_mismatch() {
    let cwd = TempCwd::new("row-count-mismatch");
    cwd.write(
        "blueprints/M9/M9-B01-foo.md",
        &required_blueprint("M9-B01", &["claim one", "claim two", "claim three"]),
    );
    cwd.write(
        "blueprints/M9/M9-B01-CLAIMS.md",
        &claims_file(
            "M9-B01",
            &[("claim one", "CONFIRMED"), ("claim two", "CONFIRMED")],
        ),
    );

    let _code = xtask::verify_claims::run("M9");
    let result = cwd.read_result();
    assert_eq!(result.status, Status::Fail);
    let detail = result
        .cases
        .iter()
        .find_map(|c| c.detail.clone())
        .expect("must have a detail");
    assert!(detail.contains("3 claims"), "detail: {detail}");
    assert!(detail.contains("2 rows"), "detail: {detail}");
}

#[test]
fn run_fails_on_an_uncorrected_wrong_row() {
    let cwd = TempCwd::new("uncorrected-wrong");
    cwd.write(
        "blueprints/M9/M9-B01-foo.md",
        &required_blueprint("M9-B01", &["claim one"]),
    );
    cwd.write(
        "blueprints/M9/M9-B01-CLAIMS.md",
        &claims_file("M9-B01", &[("claim one", "WRONG")]),
    );

    let _code = xtask::verify_claims::run("M9");
    let result = cwd.read_result();
    assert_eq!(result.status, Status::Fail);
}

#[test]
fn run_excludes_the_index_and_completion_report_files() {
    let cwd = TempCwd::new("excludes-index-and-report");
    cwd.write("blueprints/M9/M9-B01-foo.md", EXEMPT_BLUEPRINT);
    cwd.write(
        "blueprints/M9/M9-B00-index.md",
        "# M9-B00 — Milestone Index\n\nSome index prose, no Claims-to-verify heading at all.\n",
    );
    cwd.write(
        "blueprints/M9/M9-COMPLETION-REPORT.md",
        "# M9 — Completion Report\n\nSome completion-report prose.\n",
    );

    let _code = xtask::verify_claims::run("M9");
    let result = cwd.read_result();
    assert_eq!(result.status, Status::Pass, "cases: {:?}", result.cases);
    assert!(
        !result
            .cases
            .iter()
            .any(|c| c.name.contains("index") || c.name.contains("COMPLETION-REPORT"))
    );
}
