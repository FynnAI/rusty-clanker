use std::path::PathBuf;

use xtask::tier_result::{Status, TierResult, write_to};

#[test]
fn serializes_with_expected_keys() {
    let mut result = TierResult::new("example-tier");
    result.push("case-one", Status::Pass, None);
    result.push("case-two", Status::Fail, Some("boom".to_string()));
    let result = result.finalize();

    let value = serde_json::to_value(&result).expect("TierResult must serialize");
    assert!(value.get("tier").is_some());
    assert!(value.get("status").is_some());
    assert!(value.get("cases").is_some());
    assert_eq!(value["status"], "fail");
}

#[test]
fn write_to_creates_parent_dirs_and_valid_json() {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "rc-xtask-tier-result-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let mut result = TierResult::new("write-to-check");
    result.push("only-case", Status::Pass, None);
    let result = result.finalize();

    write_to(&root, &result).expect("write_to must create parent dirs and write the file");

    let written_path: PathBuf = root.join("write-to-check.json");
    let text = std::fs::read_to_string(&written_path).expect("file must exist after write_to");
    let _: serde_json::Value = serde_json::from_str(&text).expect("must be valid JSON");

    let _ = std::fs::remove_dir_all(&root);
}
