use xtask::fixture_manifest::{build_manifest, compute_sha256_hex, verify_manifest};

#[test]
fn sha256_matches_known_vector() {
    assert_eq!(
        compute_sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn build_manifest_populates_every_field_per_entry() {
    let files = vec![
        ("registries.rs".to_string(), b"content-a".to_vec()),
        ("block_states.rs".to_string(), b"content-b".to_vec()),
    ];
    let manifest = build_manifest(
        776,
        "26.2",
        &files,
        "xtask-codegen/0.1.0",
        "deadbeef00000000000000000000000000000000",
    );

    assert_eq!(manifest.entries.len(), 2);
    for (name, bytes) in &files {
        let entry = manifest
            .entries
            .iter()
            .find(|e| &e.path == name)
            .unwrap_or_else(|| panic!("no manifest entry for {name}"));
        assert_eq!(entry.sha256, compute_sha256_hex(bytes));
        assert_eq!(entry.generator_tool_version, "xtask-codegen/0.1.0");
        assert_eq!(
            entry.source_jar_sha1,
            "deadbeef00000000000000000000000000000000"
        );
    }
}

fn write_fixture_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rc_xtask_fixture_manifest_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn verify_manifest_passes_on_untampered_files() {
    let dir = write_fixture_dir();
    let files = vec![
        ("registries.rs".to_string(), b"content-a".to_vec()),
        ("block_states.rs".to_string(), b"content-b".to_vec()),
    ];
    for (name, bytes) in &files {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    let manifest = build_manifest(776, "26.2", &files, "xtask-codegen/0.1.0", "deadbeef");
    let manifest_path = dir.join("MANIFEST.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    assert!(verify_manifest(&manifest_path, &dir).is_empty());
}

#[test]
fn verify_manifest_detects_hash_mismatch() {
    let dir = write_fixture_dir();
    let files = vec![
        ("registries.rs".to_string(), b"content-a".to_vec()),
        ("block_states.rs".to_string(), b"content-b".to_vec()),
    ];
    for (name, bytes) in &files {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    let manifest = build_manifest(776, "26.2", &files, "xtask-codegen/0.1.0", "deadbeef");
    let manifest_path = dir.join("MANIFEST.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    std::fs::write(dir.join("registries.rs"), b"tampered-content").unwrap();

    let violations = verify_manifest(&manifest_path, &dir);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].kind, "hash_mismatch");
    assert_eq!(violations[0].path, "registries.rs");
}

#[test]
fn verify_manifest_detects_missing_file() {
    let dir = write_fixture_dir();
    let files = vec![
        ("registries.rs".to_string(), b"content-a".to_vec()),
        ("block_states.rs".to_string(), b"content-b".to_vec()),
    ];
    for (name, bytes) in &files {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    let manifest = build_manifest(776, "26.2", &files, "xtask-codegen/0.1.0", "deadbeef");
    let manifest_path = dir.join("MANIFEST.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    std::fs::remove_file(dir.join("registries.rs")).unwrap();

    let violations = verify_manifest(&manifest_path, &dir);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].kind, "missing");
    assert_eq!(violations[0].path, "registries.rs");
}
