//! `xtask::setup_oracle`'s own offline self-tests (M0-B08): the TEST-D41 consent gate's two
//! unattended paths (marker file, `RC_ORACLE_EULA_ACCEPTED=1`) and the pure `harness_dirs`
//! layout. Nothing here reaches the network.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use xtask::setup_oracle::{consent_already_given, harness_dirs, record_consent};

const CONSENT_ENV_VAR: &str = "RC_ORACLE_EULA_ACCEPTED";

/// `consent_already_given` reads `RC_ORACLE_EULA_ACCEPTED` straight from the process
/// environment, and each consent test below sets or clears that variable to establish its
/// own precondition. `cargo-nextest` gives every `#[test]` its own OS process, so those
/// writes cannot race there -- but `cargo test` (libtest) runs every test in one binary in
/// the SAME process with thread-based parallelism by default, and this project's own
/// verification commands run both (`cargo nextest run -p xtask` and `cargo test -p xtask`),
/// so one test's `remove_var` could land between another test's `set_var` and its
/// assertion. This lock serializes every environment-touching test regardless of which
/// runner drives them (the same pattern as `verify_claims_cli.rs`'s `CWD_LOCK`). A poisoned
/// lock (an earlier test panicked while holding it) is recovered rather than propagated --
/// the guarded state is trivially `()`, so there is nothing to actually be inconsistent.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Exclusive ownership of `RC_ORACLE_EULA_ACCEPTED` for the duration of one test: takes
/// `ENV_LOCK`, sets the variable to `value` (clears it for `None`), and clears it again on
/// `Drop` -- on panic too -- so no test ever observes a value another test left behind.
struct ConsentEnv {
    _lock: MutexGuard<'static, ()>,
}

impl ConsentEnv {
    fn set(value: Option<&str>) -> Self {
        let lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: `ENV_LOCK` is held from here until this guard is dropped, and every access
        // to the process environment in this test binary (these writes and
        // `temp_repo_root`'s `std::env::temp_dir()` read) happens only while holding it, so
        // no other thread can be reading or writing the environment concurrently.
        unsafe {
            match value {
                Some(value) => std::env::set_var(CONSENT_ENV_VAR, value),
                None => std::env::remove_var(CONSENT_ENV_VAR),
            }
        }
        Self { _lock: lock }
    }
}

impl Drop for ConsentEnv {
    fn drop(&mut self) {
        // SAFETY: as in `set` -- `_lock` is released only after this body has run.
        unsafe {
            std::env::remove_var(CONSENT_ENV_VAR);
        }
    }
}

/// A fresh, uniquely named temp repo root. Takes the live `ConsentEnv` guard because
/// `std::env::temp_dir()` reads the process environment (see `ENV_LOCK`).
fn temp_repo_root(_env: &ConsentEnv, label: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "rc-xtask-setup-oracle-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("must create temp repo root");
    root
}

#[test]
fn consent_missing_by_default() {
    let env = ConsentEnv::set(None);
    let root = temp_repo_root(&env, "missing");
    assert!(!consent_already_given(&root));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn consent_true_after_record_consent() {
    let env = ConsentEnv::set(None);
    let root = temp_repo_root(&env, "recorded");
    record_consent(&root).expect("record_consent must succeed");
    assert!(consent_already_given(&root));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn consent_true_via_env_var() {
    let env = ConsentEnv::set(Some("1"));
    let root = temp_repo_root(&env, "env-var");
    assert!(consent_already_given(&root));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn harness_dirs_returns_three_paths_under_oracle_root() {
    // `harness_dirs` is pure path construction (touches neither disk nor environment), so a
    // synthetic root suffices -- and keeps this test outside `ENV_LOCK` entirely.
    let root = PathBuf::from("rc-xtask-setup-oracle-harness-dirs");
    let dirs = harness_dirs(&root, "26.2");
    assert_eq!(dirs.len(), 3);
    let expected_prefix = root.join("oracle").join("26.2").join("harness");
    let mut tails: Vec<String> = Vec::new();
    for dir in &dirs {
        assert!(dir.starts_with(&expected_prefix));
        tails.push(
            dir.file_name()
                .expect("each harness dir must have a file name")
                .to_string_lossy()
                .to_string(),
        );
    }
    tails.sort();
    assert_eq!(tails, vec!["scenarios", "seeds", "working"]);
}
