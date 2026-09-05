//! M3-B07 — `load_spec`'s validation rules, `bounding_box`, `world_origin_for`, and
//! the committed corpus manifest's own integrity (blueprint Acceptance tests,
//! `spec_loading.rs`). Synthetic in-memory data plus the five committed `.ron` files
//! — no oracle, no network.

use std::path::PathBuf;

use rc_gametest::spec::{
    Category, ContraptionSpec, PlacedBlock, ScriptedAction, SpecError, bounding_box, load_spec,
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
fn loads_every_shipped_ron_file_and_meets_the_ac1_floor() {
    let files = shipped_ron_files();
    // M3 roadmap AC1's own corpus floor (11-roadmap-milestones.md): >= 50 committed
    // contraptions. A floor, not an exact count -- TEST-D42 licenses incremental,
    // code-authored corpus growth, so an exact-count assertion would turn every
    // legitimate addition into a test edit.
    assert!(
        files.len() >= 50,
        "expected at least 50 committed .ron fixtures (AC1 floor), found {}",
        files.len()
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

/// M3 field-report test-authoring (PLAN-D10 Task A3, fixture-support lint): every wire/
/// torch/repeater/comparator/lever block in a spec must have a non-air block in its own
/// support direction at setup — the oracle pops a floating one within a tick of setup.
#[test]
fn rejects_a_floating_wire_with_no_floor_below() {
    let path = write_synthetic(
        "floating_wire.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/floating_wire",
            category: PulseGenerator,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [(pos: (0, 1, 0), vanilla_state: "minecraft:redstone_wire[power=0]", state_id: 4864)],
            actions: [],
        )"#,
    );

    let err = load_spec(&path).expect_err("a wire with no floor below must be rejected");
    assert!(matches!(
        err,
        rc_gametest::spec::SpecError::MissingSupport {
            support_pos: (0, 0, 0),
            ..
        }
    ));
}

#[test]
fn accepts_a_wire_with_a_real_floor_below() {
    let path = write_synthetic(
        "wire_with_floor.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/wire_with_floor",
            category: PulseGenerator,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [
                (pos: (0, 0, 0), vanilla_state: "minecraft:stone", state_id: 1),
                (pos: (0, 1, 0), vanilla_state: "minecraft:redstone_wire[power=0]", state_id: 4864),
            ],
            actions: [],
        )"#,
    );

    load_spec(&path).expect("a wire resting on a real floor must load cleanly");
}

#[test]
fn rejects_a_floating_wall_torch_with_no_mount_behind_its_own_facing() {
    let path = write_synthetic(
        "floating_wall_torch.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/floating_wall_torch",
            category: QcShowcase,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [(pos: (1, 0, 0), vanilla_state: "minecraft:redstone_wall_torch[facing=east,lit=true]", state_id: 6893)],
            actions: [],
        )"#,
    );

    // facing=east: the wall it mounts on sits behind it, at (0, 0, 0) (facing's own
    // opposite direction), never (2, 0, 0).
    let err = load_spec(&path).expect_err("a wall torch with no mount block must be rejected");
    assert!(matches!(
        err,
        rc_gametest::spec::SpecError::MissingSupport {
            support_pos: (0, 0, 0),
            ..
        }
    ));
}

#[test]
fn rejects_a_floating_wall_lever_with_no_mount_behind_its_own_facing() {
    let path = write_synthetic(
        "floating_wall_lever.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/floating_wall_lever",
            category: QcShowcase,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [(pos: (1, 0, 0), vanilla_state: "minecraft:lever[face=wall,facing=east,powered=false]", state_id: 6786)],
            actions: [],
        )"#,
    );

    let err = load_spec(&path).expect_err("a wall lever with no mount block must be rejected");
    assert!(matches!(
        err,
        rc_gametest::spec::SpecError::MissingSupport {
            support_pos: (0, 0, 0),
            ..
        }
    ));
}

#[test]
fn accepts_a_floor_lever_with_a_real_floor_below() {
    let path = write_synthetic(
        "floor_lever.ron",
        r#"ContraptionSpec(
            id: "test/synthetic/floor_lever",
            category: QcShowcase,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [
                (pos: (0, 0, 0), vanilla_state: "minecraft:stone", state_id: 1),
                (pos: (0, 1, 0), vanilla_state: "minecraft:lever[face=floor,facing=east,powered=false]", state_id: 6778),
            ],
            actions: [],
        )"#,
    );

    load_spec(&path).expect("a floor lever resting on a real floor must load cleanly");
}

#[test]
fn the_former_allowlist_ids_now_get_the_support_lint_like_every_other_fixture() {
    // M3.5-B03 follow-up (deliverable 5): the hand-authored `SUPPORT_LINT_ALLOWLIST`
    // this test used to prove skipped `comparator_container_fullness_chest` entirely
    // is gone (`check_support`'s own doc comment) — every real fixture that id ever
    // named was re-geometried with the missing floor cell, so a synthetic spec using
    // that exact same id, still floating, must now fail the support check exactly
    // like any other id would.
    let path = write_synthetic(
        "formerly_allowlisted_floating_comparator.ron",
        r#"ContraptionSpec(
            id: "redstone/comparator/comparator_container_fullness_chest",
            category: ComparatorCircuit,
            description: "synthetic",
            quirk: "synthetic",
            max_ticks: 5,
            blocks: [(pos: (0, 1, 0), vanilla_state: "minecraft:comparator[facing=north,mode=compare,powered=false]", state_id: 11264)],
            actions: [],
        )"#,
    );

    let err = load_spec(&path)
        .expect_err("a floating comparator must fail the support check regardless of id, now that the allowlist is gone");
    assert!(
        matches!(err, SpecError::MissingSupport { .. }),
        "expected MissingSupport, got {err:?}"
    );
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
