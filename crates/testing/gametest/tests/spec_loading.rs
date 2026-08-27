//! M3-B07 — `load_spec`'s validation rules, `bounding_box`, `world_origin_for`, and
//! the committed corpus manifest's own integrity (blueprint Acceptance tests,
//! `spec_loading.rs`). Synthetic in-memory data plus the five committed `.ron` files
//! — no oracle, no network.

use std::path::PathBuf;

use rc_gametest::spec::{
    Category, ContraptionSpec, PlacedBlock, ScriptedAction, bounding_box, load_spec,
    world_origin_for,
};
use xtask::fixture_manifest::verify_manifest;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/redstone")
}

fn shipped_ron_files() -> Vec<PathBuf> {
    let dir = corpus_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    files.sort();
    files
}

#[test]
fn loads_all_five_shipped_ron_files() {
    let files = shipped_ron_files();
    assert_eq!(
        files.len(),
        5,
        "expected exactly 5 committed .ron fixtures, found {files:?}"
    );

    for path in &files {
        let spec = load_spec(path)
            .unwrap_or_else(|err| panic!("load_spec({}) failed: {err}", path.display()));
        assert!(
            !spec.blocks.is_empty(),
            "{}: blocks must be non-empty",
            path.display()
        );
        assert!(
            spec.max_ticks <= rc_gametest::spec::MAX_TICKS,
            "{}: max_ticks {} exceeds MAX_TICKS",
            path.display(),
            spec.max_ticks
        );
    }
}

fn write_synthetic(name: &str, contents: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rc_gametest_spec_loading_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write synthetic RON");
    path
}

#[test]
fn rejects_max_ticks_above_cap() {
    let path = write_synthetic(
        "max_ticks_above_cap.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/max_ticks_above_cap",
            category: PulseGenerator,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 201,
            blocks: [(pos: (0, 0, 0), vanilla_state: "minecraft:stone", state_id: 1)],
            actions: [],
        )"#,
    );

    let err = load_spec(&path).expect_err("max_ticks above the cap must be rejected");
    assert!(matches!(
        err,
        rc_gametest::spec::SpecError::MaxTicksExceeded { max_ticks: 201, .. }
    ));
}

#[test]
fn rejects_action_tick_at_or_above_max_ticks() {
    let path = write_synthetic(
        "action_tick_out_of_range.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/action_tick_out_of_range",
            category: PulseGenerator,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [(pos: (0, 0, 0), vanilla_state: "minecraft:stone", state_id: 1)],
            actions: [(tick: 5, pos: (0, 0, 0), vanilla_state: "minecraft:air", state_id: 0)],
        )"#,
    );

    let err = load_spec(&path).expect_err("an action at tick == max_ticks must be rejected");
    assert!(matches!(
        err,
        rc_gametest::spec::SpecError::ActionTickOutOfRange {
            tick: 5,
            max_ticks: 5,
            ..
        }
    ));
}

#[test]
fn rejects_empty_blocks() {
    let path = write_synthetic(
        "empty_blocks.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/empty_blocks",
            category: PulseGenerator,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [],
            actions: [],
        )"#,
    );

    let err = load_spec(&path).expect_err("empty blocks must be rejected");
    assert!(matches!(err, rc_gametest::spec::SpecError::NoBlocks { .. }));
}

#[test]
fn bounding_box_covers_every_block_and_action_position() {
    let spec = ContraptionSpec {
        id: "test/synthetic/bounding_box".to_string(),
        category: Category::PulseGenerator,
        description: "synthetic".to_string(),
        quirk: "synthetic".to_string(),
        max_ticks: 5,
        blocks: vec![
            PlacedBlock {
                pos: (0, 0, 0),
                vanilla_state: "minecraft:stone".to_string(),
                state_id: 1,
                has_analog_state: false,
            },
            PlacedBlock {
                pos: (2, 1, -1),
                vanilla_state: "minecraft:stone".to_string(),
                state_id: 1,
                has_analog_state: false,
            },
        ],
        actions: vec![ScriptedAction {
            tick: 1,
            pos: (-1, 3, 0),
            vanilla_state: "minecraft:air".to_string(),
            state_id: 0,
        }],
    };

    let (min, max) = bounding_box(&spec);
    assert_eq!(min, (-1, 0, -1));
    assert_eq!(max, (2, 3, 0));
}

#[test]
fn world_origin_for_is_64_spaced_and_deterministic() {
    assert_eq!(world_origin_for(0), (0, 4, 0));
    assert_eq!(world_origin_for(3), (192, 4, 0));
    assert_eq!(world_origin_for(3), world_origin_for(3));
}

#[test]
fn manifest_verifies_clean_against_the_five_shipped_ron_files() {
    let dir = corpus_dir();
    let manifest_path = dir.join("manifest.json");
    let violations = verify_manifest(&manifest_path, &dir);
    let details: Vec<String> = violations
        .iter()
        .map(|v| format!("{} [{}]: {}", v.path, v.kind, v.message))
        .collect();
    assert!(
        violations.is_empty(),
        "manifest.json must verify clean against the committed .ron files, violations: {details:?}"
    );
}
