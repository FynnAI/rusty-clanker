use std::path::PathBuf;

use xtask::setup_oracle::{consent_already_given, harness_dirs, record_consent};

fn temp_repo_root(label: &str) -> PathBuf {
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
    let root = temp_repo_root("missing");
    unsafe {
        std::env::remove_var("RC_ORACLE_EULA_ACCEPTED");
    }
    assert!(!consent_already_given(&root));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn consent_true_after_record_consent() {
    let root = temp_repo_root("recorded");
    unsafe {
        std::env::remove_var("RC_ORACLE_EULA_ACCEPTED");
    }
    record_consent(&root).expect("record_consent must succeed");
    assert!(consent_already_given(&root));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn consent_true_via_env_var() {
    let root = temp_repo_root("env-var");
    unsafe {
        std::env::set_var("RC_ORACLE_EULA_ACCEPTED", "1");
    }
    assert!(consent_already_given(&root));
    unsafe {
        std::env::remove_var("RC_ORACLE_EULA_ACCEPTED");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn harness_dirs_returns_three_paths_under_oracle_root() {
    let root = temp_repo_root("harness-dirs");
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
    let _ = std::fs::remove_dir_all(&root);
}
