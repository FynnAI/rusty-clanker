use std::path::PathBuf;

use xtask::verify_fixtures::{ManifestEntry, check_manifest};

fn temp_dir(label: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "rc-xtask-verify-fixtures-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("must create temp dir");
    root
}

#[test]
fn no_entries_passes_vacuously() {
    let root = temp_dir("no-entries");
    let result = check_manifest(&root, &[]);
    assert_eq!(result, Vec::<(String, String, String)>::new());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn matching_sha256_passes() {
    let root = temp_dir("matching");
    let file_path = root.join("fixture.bin");
    std::fs::write(&file_path, b"hello fixture").expect("must write fixture file");

    let sha256 = xtask::fixture_manifest::compute_sha256_hex(b"hello fixture");
    let entry = ManifestEntry {
        path: "fixture.bin".to_string(),
        sha256,
        generator: "test-generator-v1".to_string(),
        source_jar_sha1: "deadbeef".to_string(),
    };

    let result = check_manifest(&root, &[entry]);
    assert_eq!(result, Vec::new());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn mismatched_sha256_is_flagged() {
    let root = temp_dir("mismatched");
    let file_path = root.join("fixture.bin");
    std::fs::write(&file_path, b"hello fixture").expect("must write fixture file");

    let entry = ManifestEntry {
        path: "fixture.bin".to_string(),
        sha256: "0".repeat(64),
        generator: "test-generator-v1".to_string(),
        source_jar_sha1: "deadbeef".to_string(),
    };

    let result = check_manifest(&root, &[entry]);
    assert_eq!(result.len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_file_is_flagged() {
    let root = temp_dir("missing-file");
    let entry = ManifestEntry {
        path: "does-not-exist.bin".to_string(),
        sha256: "0".repeat(64),
        generator: "test-generator-v1".to_string(),
        source_jar_sha1: "deadbeef".to_string(),
    };

    let result = check_manifest(&root, &[entry]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].2, "<file missing>");
    let _ = std::fs::remove_dir_all(&root);
}
