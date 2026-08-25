//! M1 registry-sync-fix follow-up acceptance tests: `xtask::datagen::tags` — tag-tree
//! loading, recursive `#tag` resolution (including `required: false` and cycle detection),
//! registry discovery (static/dynamic/nested matches, unresolved-directory reporting), and
//! end-to-end generated-source production against a synthetic fixture directory. Never reads
//! the real local data-generator output (`C:\...\mc-research\...`) — that path exists only on
//! the machine this fix was developed on, never in CI or on a fresh checkout.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use xtask::datagen::reports::{RegistriesReport, RegistryEntryReport, RegistryReport};
use xtask::datagen::tags::{discover_registries, generate_tags_rs, load_tag_tree, resolve_tag};

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

// --- resolve_tag / load_tag_tree: unchanged behavior, still exercised directly. ---

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

// --- discover_registries: static / dynamic / nested-dynamic matches, unresolved reporting. ---

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
fn discover_registries_matches_a_static_top_level_directory() {
    let root = temp_dir("discover_static");
    write_tag_file(&root, "block", "infiniburn_overworld", &[]);

    let (discovered, unresolved) =
        discover_registries(&root, &synthetic_registries_report()).unwrap();
    assert!(unresolved.is_empty());
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].registry_id, "minecraft:block");
    assert!(!discovered[0].is_dynamic);
}

#[test]
fn discover_registries_matches_a_dynamic_top_level_directory() {
    let root = temp_dir("discover_dynamic");
    write_tag_file(&root, "dialog", "quick_actions", &[]);

    let (discovered, unresolved) =
        discover_registries(&root, &synthetic_registries_report()).unwrap();
    assert!(unresolved.is_empty());
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].registry_id, "minecraft:dialog");
    assert!(discovered[0].is_dynamic);
}

#[test]
fn discover_registries_matches_a_nested_dynamic_registry_but_not_its_ungrouped_siblings() {
    // Mirrors the real `worldgen/` tree: `worldgen/biome` is a real SYNCHRONIZED_REGISTRIES
    // entry, `worldgen/structure` is not (never part of the client-facing wire set).
    let root = temp_dir("discover_nested");
    write_tag_file(&root, "worldgen/biome", "is_overworld", &[]);
    write_tag_file(&root, "worldgen/structure", "on_ocean_explorer_map", &[]);

    let (discovered, unresolved) =
        discover_registries(&root, &synthetic_registries_report()).unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].registry_id, "minecraft:worldgen/biome");
    assert!(discovered[0].is_dynamic);
    assert_eq!(unresolved, vec!["worldgen/structure".to_string()]);
}

#[test]
fn discover_registries_reports_a_leaf_directory_with_no_matching_registry() {
    let root = temp_dir("discover_unresolved_leaf");
    // A top-level directory whose own name matches no known registry, and which has no
    // subdirectories of its own (a genuine leaf) -- reported once, by its own name.
    write_tag_file(&root, "villager_trade", "armorer", &[]);

    let (discovered, unresolved) =
        discover_registries(&root, &synthetic_registries_report()).unwrap();
    assert!(discovered.is_empty());
    assert_eq!(unresolved, vec!["villager_trade".to_string()]);
}

#[test]
fn discover_registries_reports_each_unresolved_leaf_under_an_unresolved_grouping_directory() {
    // Mirrors the real `villager_trade/armorer/level_1.json` shape exactly: `villager_trade`
    // itself has subdirectories (one per profession), none of which match anything either --
    // every such leaf is named individually, never collapsed into a single silent skip.
    let root = temp_dir("discover_unresolved_nested_leaves");
    write_tag_file(&root, "villager_trade/armorer", "level_1", &[]);
    write_tag_file(&root, "villager_trade/butcher", "level_1", &[]);

    let (discovered, unresolved) =
        discover_registries(&root, &synthetic_registries_report()).unwrap();
    assert!(discovered.is_empty());
    assert_eq!(
        unresolved,
        vec![
            "villager_trade/armorer".to_string(),
            "villager_trade/butcher".to_string(),
        ]
    );
}

#[test]
fn discover_registries_sorts_discovered_registries_by_id() {
    let root = temp_dir("discover_sorted");
    write_tag_file(&root, "item", "enchantable/bow", &[]);
    write_tag_file(&root, "block", "infiniburn_overworld", &[]);
    write_tag_file(&root, "dialog", "quick_actions", &[]);

    let (discovered, _) = discover_registries(&root, &synthetic_registries_report()).unwrap();
    let ids: Vec<&str> = discovered.iter().map(|d| d.registry_id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

// --- generate_tags_rs: end-to-end against a synthetic fixture mixing every discovery case. ---

#[test]
fn generate_tags_rs_emits_every_discovered_tag_not_a_curated_subset() {
    let root = temp_dir("generate_complete");
    // Real names, real numeric ids (via synthetic_registries_report) -- but crucially, MORE
    // than one tag per registry and no pre-declared "roots" list; every tag file present must
    // appear in the output.
    write_tag_file(
        &root,
        "block",
        "infiniburn_overworld",
        &["minecraft:netherrack", "minecraft:magma_block"],
    );
    write_tag_file(&root, "block", "some_other_tag", &["minecraft:netherrack"]);
    write_tag_file(&root, "item", "enchantable/bow", &["minecraft:bow"]);
    write_tag_file(&root, "villager_trade", "armorer", &[]); // orphan -- must not appear

    let registries = synthetic_registries_report();
    let (content, unresolved) = generate_tags_rs(&root, &registries).unwrap();

    assert!(content.contains("pub mod block {"));
    assert!(content.contains("pub mod item {"));
    assert!(!content.contains("pub mod villager_trade"));
    assert!(
        content.contains(
            "pub const INFINIBURN_OVERWORLD: TagTable = TagTable { tag_id: \"minecraft:infiniburn_overworld\", entries: &[285, 671] };"
        ),
        "expected sorted, real numeric block ids in:\n{content}"
    );
    assert!(
        content.contains(
            "pub const SOME_OTHER_TAG: TagTable = TagTable { tag_id: \"minecraft:some_other_tag\", entries: &[285] };"
        ),
        "expected the second, non-curated block tag to also be emitted in:\n{content}"
    );
    assert!(
        content.contains(
            "pub const ENCHANTABLE_BOW: TagTable = TagTable { tag_id: \"minecraft:enchantable/bow\", entries: &[922] };"
        ),
        "expected the real item numeric id in:\n{content}"
    );
    assert!(content.contains("pub static REGISTRIES: &[(&str, &[TagTable])]"));
    assert_eq!(unresolved, vec!["villager_trade".to_string()]);
}

#[test]
fn generate_tags_rs_resolves_dynamic_registry_by_synchronized_registries_index() {
    let root = temp_dir("generate_dynamic_index");
    // enchantment/exclusive_set/armor's real content -- protection, blast_protection,
    // fire_protection, projectile_protection -- at indices 28, 3, 11, 27 of
    // SYNCHRONIZED_REGISTRIES_ORDER's own `minecraft:enchantment` entry list (must equal
    // play::world::SYNCHRONIZED_REGISTRIES's own order).
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
    let (content, unresolved) = generate_tags_rs(&root, &registries).unwrap();
    assert!(unresolved.is_empty());

    assert!(
        content.contains(
            "pub const EXCLUSIVE_SET_ARMOR: TagTable = TagTable { tag_id: \"minecraft:exclusive_set/armor\", entries: &[3, 11, 27, 28] };"
        ),
        "expected sorted dynamic-registry indices in:\n{content}"
    );
}

#[test]
fn generate_tags_rs_errors_on_a_tag_referencing_an_unknown_member() {
    let root = temp_dir("generate_unknown_member");
    write_tag_file(&root, "block", "bad", &["minecraft:not_a_real_block"]);

    let registries = synthetic_registries_report();
    let err = generate_tags_rs(&root, &registries).unwrap_err();
    assert!(err.contains("not_a_real_block"));
}

#[test]
fn generate_tags_rs_is_deterministic_across_repeated_calls() {
    let root = temp_dir("generate_deterministic");
    write_tag_file(
        &root,
        "block",
        "infiniburn_overworld",
        &["minecraft:netherrack"],
    );
    write_tag_file(&root, "item", "enchantable/bow", &["minecraft:bow"]);
    write_tag_file(&root, "dialog", "quick_actions", &[]);
    let registries = synthetic_registries_report();

    let a = generate_tags_rs(&root, &registries).unwrap();
    let b = generate_tags_rs(&root, &registries).unwrap();
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
}

#[test]
fn generated_tags_rs_compiles_standalone() {
    let root = temp_dir("generate_compile_check");
    write_tag_file(
        &root,
        "block",
        "infiniburn_overworld",
        &["minecraft:netherrack"],
    );
    write_tag_file(&root, "item", "enchantable/bow", &["minecraft:bow"]);
    write_tag_file(&root, "dialog", "quick_actions", &[]);
    let registries = synthetic_registries_report();
    let (content, _) = generate_tags_rs(&root, &registries).unwrap();

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
