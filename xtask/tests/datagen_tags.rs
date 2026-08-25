//! M1 registry-sync-fix follow-up acceptance tests: `xtask::datagen::tags` — tag-tree
//! loading, recursive `#tag` resolution (including `required: false` and cycle detection),
//! and end-to-end generated-source production against a synthetic fixture directory. Never
//! reads the real local data-generator output (`C:\...\mc-research\...`) — that path exists
//! only on the machine this fix was developed on, never in CI or on a fresh checkout.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use xtask::datagen::reports::{RegistriesReport, RegistryEntryReport, RegistryReport};
use xtask::datagen::tags::{TAG_REGISTRIES, generate_tags_rs, load_tag_tree, resolve_tag};

/// A fresh, empty temp directory unique to this test process — never cleaned up afterward
/// (mirrors `datagen_codegen.rs`'s own `generated_files_compile_standalone` precedent), since
/// each test gets its own `std::process::id()`-plus-counter-qualified subdirectory.
fn temp_dir(label: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "rc_xtask_tags_test_{}_{}_{n}",
        std::process::id(),
        label
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_tag_file(tags_root: &Path, registry_dir: &str, tag_path: &str, values: &[&str]) {
    let file_path = tags_root
        .join("data")
        .join("minecraft")
        .join("tags")
        .join(registry_dir)
        .join(format!("{tag_path}.json"));
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    let values_json: Vec<String> = values.iter().map(|v| format!("{v:?}")).collect();
    std::fs::write(
        &file_path,
        format!("{{\"values\": [{}]}}", values_json.join(", ")),
    )
    .unwrap();
}

fn write_tag_file_with_required_false(
    tags_root: &Path,
    registry_dir: &str,
    tag_path: &str,
    plain: &[&str],
    optional_missing_id: &str,
) {
    let file_path = tags_root
        .join("data")
        .join("minecraft")
        .join("tags")
        .join(registry_dir)
        .join(format!("{tag_path}.json"));
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    let mut entries: Vec<String> = plain.iter().map(|v| format!("{v:?}")).collect();
    entries.push(format!(
        "{{\"id\": {optional_missing_id:?}, \"required\": false}}"
    ));
    std::fs::write(
        &file_path,
        format!("{{\"values\": [{}]}}", entries.join(", ")),
    )
    .unwrap();
}

#[test]
fn load_tag_tree_finds_nested_files_by_relative_path() {
    let root = temp_dir("load_nested");
    write_tag_file(
        &root,
        "item",
        "enchantable/armor",
        &["minecraft:diamond_chestplate"],
    );
    write_tag_file(&root, "item", "top_level", &["minecraft:stick"]);

    let tree = load_tag_tree(&root, "item").unwrap();
    assert!(tree.contains_key("enchantable/armor"));
    assert!(tree.contains_key("top_level"));
    assert_eq!(
        tree["enchantable/armor"][0].id,
        "minecraft:diamond_chestplate"
    );
}

#[test]
fn resolve_tag_follows_transitive_sub_tag_references() {
    let root = temp_dir("resolve_transitive");
    write_tag_file(
        &root,
        "item",
        "swords",
        &["minecraft:diamond_sword", "minecraft:iron_sword"],
    );
    write_tag_file(
        &root,
        "item",
        "melee_weapon",
        &["#minecraft:swords", "minecraft:trident"],
    );
    write_tag_file(
        &root,
        "item",
        "weapon",
        &["#minecraft:melee_weapon", "minecraft:bow"],
    );

    let tree = load_tag_tree(&root, "item").unwrap();
    let resolved = resolve_tag("weapon", &tree).unwrap();
    assert_eq!(
        resolved,
        BTreeSet::from([
            "minecraft:diamond_sword".to_string(),
            "minecraft:iron_sword".to_string(),
            "minecraft:trident".to_string(),
            "minecraft:bow".to_string(),
        ])
    );
}

#[test]
fn resolve_tag_deduplicates_diamond_dependencies() {
    // Two branches both reach "minecraft:stick" -- the resolved set must contain it once.
    let root = temp_dir("resolve_diamond");
    write_tag_file(&root, "item", "a", &["minecraft:stick"]);
    write_tag_file(&root, "item", "b", &["minecraft:stick"]);
    write_tag_file(&root, "item", "combined", &["#minecraft:a", "#minecraft:b"]);

    let tree = load_tag_tree(&root, "item").unwrap();
    let resolved = resolve_tag("combined", &tree).unwrap();
    assert_eq!(resolved, BTreeSet::from(["minecraft:stick".to_string()]));
}

#[test]
fn resolve_tag_required_false_drops_missing_reference_silently() {
    let root = temp_dir("resolve_required_false");
    write_tag_file_with_required_false(
        &root,
        "item",
        "mixed",
        &["minecraft:stick"],
        "#minecraft:does_not_exist",
    );

    let tree = load_tag_tree(&root, "item").unwrap();
    let resolved = resolve_tag("mixed", &tree).unwrap();
    assert_eq!(resolved, BTreeSet::from(["minecraft:stick".to_string()]));
}

#[test]
fn resolve_tag_required_true_missing_reference_errors() {
    let root = temp_dir("resolve_required_true_missing");
    write_tag_file(&root, "item", "mixed", &["#minecraft:does_not_exist"]);

    let tree = load_tag_tree(&root, "item").unwrap();
    let err = resolve_tag("mixed", &tree).unwrap_err();
    assert!(
        err.contains("does_not_exist"),
        "error should name the missing tag, got: {err}"
    );
}

#[test]
fn resolve_tag_detects_self_referencing_cycle() {
    let root = temp_dir("resolve_cycle");
    write_tag_file(&root, "item", "a", &["#minecraft:b"]);
    write_tag_file(&root, "item", "b", &["#minecraft:a"]);

    let tree = load_tag_tree(&root, "item").unwrap();
    let err = resolve_tag("a", &tree).unwrap_err();
    assert!(
        err.contains("cycle"),
        "expected a cycle-detection error, got: {err}"
    );
}

#[test]
fn resolve_tag_missing_tag_file_errors_by_name() {
    let root = temp_dir("resolve_missing_file");
    std::fs::create_dir_all(root.join("data/minecraft/tags/item")).unwrap();

    let tree = load_tag_tree(&root, "item").unwrap();
    let err = resolve_tag("never_written", &tree).unwrap_err();
    assert!(err.contains("never_written"));
}

/// Writes a trivial (`{"values": []}`) file for every root `TAG_REGISTRIES` names, so
/// `generate_tags_rs` (which always iterates the real, hardcoded `TAG_REGISTRIES` — not a
/// caller-supplied list) can run to completion against a synthetic fixture directory. Mirrors
/// `datagen_codegen.rs`'s own `with_full_worldgen_registries` precedent for the same reason:
/// keep individual test bodies free to override just the one or two roots they actually
/// assert on.
fn full_synthetic_tags_root(root: &Path) {
    for spec in TAG_REGISTRIES {
        for tag_path in spec.roots {
            write_tag_file(root, spec.dir, tag_path, &[]);
        }
    }
}

fn synthetic_registries_report() -> RegistriesReport {
    let mut registries = RegistriesReport::new();
    let mut block_entries = std::collections::BTreeMap::new();
    block_entries.insert(
        "minecraft:netherrack".to_string(),
        RegistryEntryReport { protocol_id: 285 },
    );
    block_entries.insert(
        "minecraft:magma_block".to_string(),
        RegistryEntryReport { protocol_id: 671 },
    );
    registries.insert(
        "minecraft:block".to_string(),
        RegistryReport {
            default: None,
            entries: block_entries,
        },
    );
    let mut item_entries = std::collections::BTreeMap::new();
    item_entries.insert(
        "minecraft:bow".to_string(),
        RegistryEntryReport { protocol_id: 922 },
    );
    registries.insert(
        "minecraft:item".to_string(),
        RegistryReport {
            default: None,
            entries: item_entries,
        },
    );
    registries
}

#[test]
fn generate_tags_rs_resolves_real_minimal_set_end_to_end() {
    let root = temp_dir("generate_end_to_end");
    full_synthetic_tags_root(&root);
    // Override the three roots this test actually asserts numeric-id content for.
    write_tag_file(
        &root,
        "block",
        "infiniburn_overworld",
        &["minecraft:netherrack", "minecraft:magma_block"],
    );
    write_tag_file(&root, "item", "enchantable/bow", &["minecraft:bow"]);

    let registries = synthetic_registries_report();
    let content = generate_tags_rs(&root, &registries).unwrap();

    assert!(content.contains("pub mod block {"));
    assert!(content.contains("pub mod item {"));
    assert!(content.contains("pub mod enchantment {"));
    assert!(content.contains("pub mod dialog {"));
    assert!(content.contains("pub mod timeline {"));
    assert!(
        content.contains(
            "pub const INFINIBURN_OVERWORLD: TagTable = TagTable { tag_id: \"minecraft:infiniburn_overworld\", entries: &[285, 671] };"
        ),
        "expected sorted, real numeric block ids in:\n{content}"
    );
    assert!(
        content.contains(
            "pub const ENCHANTABLE_BOW: TagTable = TagTable { tag_id: \"minecraft:enchantable/bow\", entries: &[922] };"
        ),
        "expected the real item numeric id in:\n{content}"
    );
    assert!(content.contains("pub static REGISTRIES: &[(&str, &[TagTable])]"));
}

#[test]
fn generate_tags_rs_resolves_dynamic_registry_by_synchronized_registries_index() {
    let root = temp_dir("generate_dynamic_index");
    full_synthetic_tags_root(&root);
    // enchantment/exclusive_set/armor's real content -- protection, blast_protection,
    // fire_protection, projectile_protection -- at indices 28, 3, 11, 27 of the hand-authored
    // ENCHANTMENT_ORDER list (must equal play::world::SYNCHRONIZED_REGISTRIES's own order).
    write_tag_file(
        &root,
        "enchantment",
        "exclusive_set/armor",
        &[
            "minecraft:protection",
            "minecraft:blast_protection",
            "minecraft:fire_protection",
            "minecraft:projectile_protection",
        ],
    );

    let registries = synthetic_registries_report();
    let content = generate_tags_rs(&root, &registries).unwrap();

    assert!(
        content.contains(
            "pub const EXCLUSIVE_SET_ARMOR: TagTable = TagTable { tag_id: \"minecraft:exclusive_set/armor\", entries: &[3, 11, 27, 28] };"
        ),
        "expected sorted dynamic-registry indices in:\n{content}"
    );
}

#[test]
fn generate_tags_rs_errors_when_a_required_root_tag_file_is_missing() {
    let root = temp_dir("generate_missing_root");
    // Deliberately incomplete: only write the block registry's own roots, leaving every
    // other TAG_REGISTRIES root file absent.
    for tag_path in TAG_REGISTRIES[0].roots {
        write_tag_file(&root, "block", tag_path, &[]);
    }

    let registries = synthetic_registries_report();
    let err = generate_tags_rs(&root, &registries).unwrap_err();
    assert!(
        err.contains("no tag JSON file found") || err.contains("failed to read tag directory"),
        "expected a missing-tag-file error, got: {err}"
    );
}

#[test]
fn generate_tags_rs_is_deterministic_across_repeated_calls() {
    let root = temp_dir("generate_deterministic");
    full_synthetic_tags_root(&root);
    let registries = synthetic_registries_report();

    let a = generate_tags_rs(&root, &registries).unwrap();
    let b = generate_tags_rs(&root, &registries).unwrap();
    assert_eq!(a, b);
}

#[test]
fn generated_tags_rs_compiles_standalone() {
    let root = temp_dir("generate_compile_check");
    full_synthetic_tags_root(&root);
    let registries = synthetic_registries_report();
    let content = generate_tags_rs(&root, &registries).unwrap();

    let out_dir = std::env::temp_dir().join(format!(
        "rc_xtask_tags_compile_check_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&out_dir).unwrap();
    let file_path = out_dir.join("tags.rs");
    std::fs::write(&file_path, &content).unwrap();

    let status = std::process::Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("--out-dir")
        .arg(&out_dir)
        .arg(&file_path)
        .status()
        .expect("failed to invoke rustc");
    assert!(
        status.success(),
        "rustc failed to compile generated tags.rs"
    );
}
