use xtask::quarantine::{insert_quarantine_attr, scan_quarantined};

#[test]
fn insert_quarantine_attr_adds_new_attribute() {
    let source = "#[test]\nfn flaky_thing() {\n    assert!(true);\n}\n";
    let result = insert_quarantine_attr(
        source,
        "flaky_thing",
        "https://github.com/org/repo/issues/9",
        "network hiccup",
    )
    .expect("flaky_thing must be found");

    assert!(result.contains(
        "#[ignore = \"quarantined: https://github.com/org/repo/issues/9 — network hiccup\"]"
    ));

    let ignore_line = result
        .lines()
        .position(|l| l.contains("#[ignore = \"quarantined:"))
        .expect("ignore line must be present");
    let test_line = result
        .lines()
        .position(|l| l.trim() == "#[test]")
        .expect("#[test] line must be present");
    assert_eq!(ignore_line + 1, test_line);
}

#[test]
fn insert_quarantine_attr_replaces_existing_ignore() {
    let source = "#[ignore = \"old reason\"]\n#[test]\nfn flaky_thing() {\n    assert!(true);\n}\n";
    let result = insert_quarantine_attr(
        source,
        "flaky_thing",
        "https://github.com/org/repo/issues/9",
        "network hiccup",
    )
    .expect("flaky_thing must be found");

    let ignore_count = result.lines().filter(|l| l.contains("#[ignore")).count();
    assert_eq!(ignore_count, 1);
    assert!(!result.contains("old reason"));
}

#[test]
fn insert_quarantine_attr_returns_none_when_fn_missing() {
    let source = "#[test]\nfn something_else() {}\n";
    let result = insert_quarantine_attr(source, "flaky_thing", "https://x/9", "reason");
    assert!(result.is_none());
}

#[test]
fn scan_quarantined_finds_inserted_entry() {
    let source = "#[test]\nfn flaky_thing() {\n    assert!(true);\n}\n";
    let inserted = insert_quarantine_attr(
        source,
        "flaky_thing",
        "https://github.com/org/repo/issues/9",
        "network hiccup",
    )
    .expect("flaky_thing must be found");

    let entries = scan_quarantined(&inserted, "f.rs");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].fn_name, "flaky_thing");
    assert_eq!(entries[0].issue_url, "https://github.com/org/repo/issues/9");
}

#[test]
fn scan_quarantined_ignores_unlinked_ignore() {
    let source = "#[ignore]\n#[test]\nfn x() {}\n";
    let entries = scan_quarantined(source, "f.rs");
    assert_eq!(entries.len(), 0);
}
