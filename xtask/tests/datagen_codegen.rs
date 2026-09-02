use std::collections::BTreeMap;

use xtask::datagen::codegen::{WORLDGEN_REGISTRIES, generate};
use xtask::datagen::reports::{
    BlockReport, BlockStateReport, BlocksReport, OrderedProperties, OrderedValueList,
    RegistriesReport, RegistryEntryReport, RegistryReport,
};

fn file_content<'a>(files: &'a [(String, String)], name: &str) -> &'a str {
    &files
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no file named {name} in generated output"))
        .1
}

/// Adds one placeholder entry for every `WORLDGEN_REGISTRIES` name not already present in
/// `registries`, leaving any already-present entry (and every other registry) untouched.
///
/// M0-B07's own five pre-existing fixtures below (`generates_registries_module_sorted_by_
/// protocol_id` through `generated_files_compile_standalone`) each build a minimal
/// `RegistriesReport` targeted at one specific `registries.rs`/`block_states.rs` assertion,
/// predating M1-B04's own `generate_registry_entries_rs` extension — which requires every
/// `WORLDGEN_REGISTRIES` name to resolve or `generate` panics (Context, "Registry-entries
/// codegen extension"). Wrapping each pre-existing call site's `registries` argument through
/// this helper is the minimal, additive fix that keeps `generate` callable from those
/// fixtures without altering a single one of their own already-asserted-on entries: this
/// only ever adds unrelated placeholder registries elsewhere in the map, so every
/// pre-existing assertion (`content.contains(...)`, position comparisons, exact `.files`
/// equality between two differently-ordered builds) continues to hold unchanged.
fn with_full_worldgen_registries(mut registries: RegistriesReport) -> RegistriesReport {
    for (i, name) in WORLDGEN_REGISTRIES.iter().enumerate() {
        registries.entry((*name).to_string()).or_insert_with(|| {
            let mut entries = BTreeMap::new();
            entries.insert(
                format!("minecraft:placeholder_{i}"),
                RegistryEntryReport { protocol_id: 0 },
            );
            RegistryReport {
                default: None,
                entries,
            }
        });
    }
    registries
}

#[test]
fn generates_registries_module_sorted_by_protocol_id() {
    // Source-text order deliberately lists `stone` (protocol_id 1) before `air`
    // (protocol_id 0), to prove the output order comes from an explicit sort by
    // protocol_id rather than from incidental map/text order.
    let json = r#"
    {
      "minecraft:block": {
        "entries": {
          "minecraft:stone": { "protocol_id": 1 },
          "minecraft:air": { "protocol_id": 0 }
        }
      }
    }
    "#;
    let registries: RegistriesReport = serde_json::from_str(json).unwrap();
    let blocks = BlocksReport::new();

    let generated = generate(&with_full_worldgen_registries(registries), &blocks);
    let content = file_content(&generated.files, "registries.rs");

    let pos0 = content
        .find("RegistryEntryId(0)")
        .expect("RegistryEntryId(0) not found in generated output");
    let pos1 = content
        .find("RegistryEntryId(1)")
        .expect("RegistryEntryId(1) not found in generated output");
    assert!(
        pos0 < pos1,
        "expected RegistryEntryId(0)'s line before RegistryEntryId(1)'s line, got positions {pos0} and {pos1}"
    );
}

#[test]
fn sanitizes_slash_and_keyword_identifiers() {
    let mut registries = RegistriesReport::new();
    let mut biome_entries = BTreeMap::new();
    biome_entries.insert(
        "minecraft:plains".to_string(),
        RegistryEntryReport { protocol_id: 0 },
    );
    registries.insert(
        "minecraft:worldgen/biome".to_string(),
        RegistryReport {
            default: None,
            entries: biome_entries,
        },
    );
    let mut type_entries = BTreeMap::new();
    type_entries.insert(
        "minecraft:foo".to_string(),
        RegistryEntryReport { protocol_id: 0 },
    );
    registries.insert(
        "minecraft:type".to_string(),
        RegistryReport {
            default: None,
            entries: type_entries,
        },
    );

    let generated = generate(
        &with_full_worldgen_registries(registries),
        &BlocksReport::new(),
    );
    let content = file_content(&generated.files, "registries.rs");

    assert!(
        content.contains("pub mod worldgen_biome {"),
        "missing sanitized slash module in:\n{content}"
    );
    assert!(
        content.contains("pub mod type_ {"),
        "missing keyword-escaped module in:\n{content}"
    );
}

#[test]
fn sanitizes_entry_name_colliding_with_reserved_count_const() {
    // Real 26.2 data (`minecraft:worldgen/placement_modifier_type`) registers an entry
    // literally named "count", which collides with the per-module `COUNT` aggregate
    // constant every registry module emits — this reproduces that exact collision on a
    // minimal synthetic fixture (`generated_files_compile_standalone`'s own rustc check
    // does not catch this, since its fixtures never happened to include a "count" entry).
    let mut registries = RegistriesReport::new();
    let mut entries = BTreeMap::new();
    entries.insert(
        "minecraft:count".to_string(),
        RegistryEntryReport { protocol_id: 0 },
    );
    entries.insert(
        "minecraft:block".to_string(),
        RegistryEntryReport { protocol_id: 1 },
    );
    registries.insert(
        "minecraft:worldgen/placement_modifier_type".to_string(),
        RegistryReport {
            default: None,
            entries,
        },
    );

    let generated = generate(
        &with_full_worldgen_registries(registries),
        &BlocksReport::new(),
    );
    let content = file_content(&generated.files, "registries.rs");

    // The entry's own escaped const, and the module's own aggregate count, must both
    // be present and distinct — i.e. this must not be the same `COUNT` identifier
    // defined twice (which would fail to compile).
    assert!(
        content.contains("pub const COUNT_: RegistryEntryId = RegistryEntryId(0);"),
        "missing escaped entry const in:\n{content}"
    );
    assert!(
        content.contains("pub const COUNT: u32 = 2;"),
        "missing module aggregate count in:\n{content}"
    );
}

#[test]
fn output_is_independent_of_input_insertion_order() {
    fn make_pair(reverse: bool) -> (RegistriesReport, BlocksReport) {
        let mut registries = RegistriesReport::new();
        // Fixed name->id mapping, independent of insertion order: reversing insertion
        // order below must not change *which* id belongs to *which* name, only the
        // order the `.insert()` calls happen in.
        let mut names = vec![
            ("minecraft:a", 0u32),
            ("minecraft:b", 1u32),
            ("minecraft:c", 2u32),
        ];
        if reverse {
            names.reverse();
        }
        for (name, id) in names {
            let mut entries = BTreeMap::new();
            entries.insert(
                format!("{name}_entry"),
                RegistryEntryReport { protocol_id: id },
            );
            registries.insert(
                name.to_string(),
                RegistryReport {
                    default: None,
                    entries,
                },
            );
        }

        let mut blocks = BlocksReport::new();
        let mut block_names = vec![("minecraft:one", 0u32), ("minecraft:two", 1u32)];
        if reverse {
            block_names.reverse();
        }
        for (name, id) in block_names {
            blocks.insert(
                name.to_string(),
                BlockReport {
                    states: vec![BlockStateReport {
                        id,
                        default: true,
                        properties: OrderedProperties::default(),
                    }],
                    properties: OrderedValueList::default(),
                },
            );
        }
        (registries, blocks)
    }

    let a = make_pair(false);
    let b = make_pair(true);

    assert_eq!(
        generate(&with_full_worldgen_registries(a.0), &a.1).files,
        generate(&with_full_worldgen_registries(b.0), &b.1).files
    );
}

#[test]
fn block_states_module_reports_correct_counts_and_default_ids() {
    let mut blocks = BlocksReport::new();
    blocks.insert(
        "minecraft:air".to_string(),
        BlockReport {
            states: vec![BlockStateReport {
                id: 0,
                default: true,
                properties: OrderedProperties::default(),
            }],
            properties: OrderedValueList::default(),
        },
    );
    blocks.insert(
        "minecraft:oak_door".to_string(),
        BlockReport {
            states: vec![
                BlockStateReport {
                    id: 5655,
                    default: false,
                    properties: OrderedProperties::default(),
                },
                BlockStateReport {
                    id: 5680,
                    default: true,
                    properties: OrderedProperties::default(),
                },
                BlockStateReport {
                    id: 5718,
                    default: false,
                    properties: OrderedProperties::default(),
                },
            ],
            properties: OrderedValueList::default(),
        },
    );

    let generated = generate(
        &with_full_worldgen_registries(RegistriesReport::new()),
        &blocks,
    );
    let content = file_content(&generated.files, "block_states.rs");

    assert!(content.contains("pub const BLOCK_TYPE_COUNT: u32 = 2;"));
    assert!(content.contains("pub const BLOCK_STATE_COUNT: u32 = 4;"));
    assert!(content.contains("BlockStateId = BlockStateId(0)"));
    assert!(content.contains("BlockStateId = BlockStateId(5680)"));
}

#[test]
fn generated_files_compile_standalone() {
    let mut registries = RegistriesReport::new();
    let mut entries = BTreeMap::new();
    entries.insert(
        "minecraft:air".to_string(),
        RegistryEntryReport { protocol_id: 0 },
    );
    registries.insert(
        "minecraft:block".to_string(),
        RegistryReport {
            default: None,
            entries,
        },
    );
    let mut blocks = BlocksReport::new();
    blocks.insert(
        "minecraft:air".to_string(),
        BlockReport {
            states: vec![BlockStateReport {
                id: 0,
                default: true,
                properties: OrderedProperties::default(),
            }],
            properties: OrderedValueList::default(),
        },
    );

    let generated = generate(&with_full_worldgen_registries(registries), &blocks);

    let out_dir = std::env::temp_dir().join(format!(
        "rc_xtask_codegen_compile_check_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&out_dir).unwrap();

    for (name, content) in &generated.files {
        let file_path = out_dir.join(name);
        std::fs::write(&file_path, content).unwrap();

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
        assert!(status.success(), "rustc failed to compile {name}");
    }
}

// --- M1-B04's own cases: `registry_entries.rs`'s codegen extension. ---

#[test]
fn registry_entries_generates_sorted_by_protocol_id() {
    // Source-text order deliberately lists `the_nether` (protocol_id 1) before
    // `overworld` (protocol_id 0), mirroring `generates_registries_module_sorted_by_
    // protocol_id`'s own proof that output order comes from an explicit sort, never
    // incidental map/text order.
    let json = r#"
    {
      "minecraft:dimension_type": {
        "entries": {
          "minecraft:the_nether": { "protocol_id": 1 },
          "minecraft:overworld": { "protocol_id": 0 }
        }
      }
    }
    "#;
    let registries: RegistriesReport = serde_json::from_str(json).unwrap();
    let registries = with_full_worldgen_registries(registries);

    let generated = generate(&registries, &BlocksReport::new());
    let content = file_content(&generated.files, "registry_entries.rs");

    let pos_overworld = content
        .find("minecraft:overworld")
        .expect("minecraft:overworld not found in generated output");
    let pos_nether = content
        .find("minecraft:the_nether")
        .expect("minecraft:the_nether not found in generated output");
    assert!(
        pos_overworld < pos_nether,
        "expected overworld (protocol_id 0) before the_nether (protocol_id 1), got positions {pos_overworld} and {pos_nether}"
    );
}

#[test]
fn registry_entries_preserves_full_original_identifier_strings() {
    let mut registries = RegistriesReport::new();
    let mut entries = BTreeMap::new();
    entries.insert(
        "minecraft:plains".to_string(),
        RegistryEntryReport { protocol_id: 0 },
    );
    registries.insert(
        "minecraft:worldgen/biome".to_string(),
        RegistryReport {
            default: None,
            entries,
        },
    );
    let registries = with_full_worldgen_registries(registries);

    let generated = generate(&registries, &BlocksReport::new());
    let content = file_content(&generated.files, "registry_entries.rs");

    // The full, unsanitized, original identifier string — never a mangled/uppercased
    // form like `registries.rs`'s own `sanitize_const_name`-based output.
    assert!(
        content.contains("\"minecraft:plains\""),
        "missing verbatim identifier string in:\n{content}"
    );
}

#[test]
fn registry_entries_emits_top_level_registries_table() {
    let registries = with_full_worldgen_registries(RegistriesReport::new());
    let generated = generate(&registries, &BlocksReport::new());
    let content = file_content(&generated.files, "registry_entries.rs");

    let table_start = content
        .find("pub static REGISTRIES: &[(&str, &[&str])]")
        .expect("missing top-level REGISTRIES table");
    let table = &content[table_start..];

    let mut last_pos: Option<usize> = None;
    for name in WORLDGEN_REGISTRIES {
        let needle = format!("({name:?}, ");
        let pos = table
            .find(&needle)
            .unwrap_or_else(|| panic!("missing entry for {name} in the REGISTRIES table"));
        if let Some(last) = last_pos {
            assert!(
                pos > last,
                "expected {name} to appear after the previous entry, in WORLDGEN_REGISTRIES's own fixed order"
            );
        }
        last_pos = Some(pos);
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[test]
fn registry_entries_panics_on_missing_worldgen_registry() {
    let mut registries = RegistriesReport::new();
    for name in WORLDGEN_REGISTRIES {
        if *name == "minecraft:enchantment" {
            continue;
        }
        let mut entries = BTreeMap::new();
        entries.insert(
            format!("{name}_entry"),
            RegistryEntryReport { protocol_id: 0 },
        );
        registries.insert(
            (*name).to_string(),
            RegistryReport {
                default: None,
                entries,
            },
        );
    }

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(|| {
        xtask::datagen::codegen::generate_registry_entries_rs(&registries)
    });
    std::panic::set_hook(previous_hook);

    let err = result.expect_err("expected a panic for the missing WORLDGEN_REGISTRIES entry");
    let message = panic_message(&*err);
    assert!(
        message.contains("minecraft:enchantment"),
        "panic message should name the missing registry, got: {message}"
    );
}

#[test]
fn generate_still_emits_three_files_and_existing_two_unchanged() {
    let mut registries = RegistriesReport::new();
    let mut entries = BTreeMap::new();
    entries.insert(
        "minecraft:stone".to_string(),
        RegistryEntryReport { protocol_id: 1 },
    );
    entries.insert(
        "minecraft:air".to_string(),
        RegistryEntryReport { protocol_id: 0 },
    );
    registries.insert(
        "minecraft:block".to_string(),
        RegistryReport {
            default: None,
            entries,
        },
    );
    let registries = with_full_worldgen_registries(registries);

    let mut blocks = BlocksReport::new();
    blocks.insert(
        "minecraft:air".to_string(),
        BlockReport {
            states: vec![BlockStateReport {
                id: 0,
                default: true,
                properties: OrderedProperties::default(),
            }],
            properties: OrderedValueList::default(),
        },
    );

    let generated = generate(&registries, &blocks);
    assert_eq!(
        generated.files.len(),
        3,
        "expected registries.rs, block_states.rs, and registry_entries.rs"
    );
    assert_eq!(generated.files[0].0, "registries.rs");
    assert_eq!(generated.files[1].0, "block_states.rs");
    assert_eq!(generated.files[2].0, "registry_entries.rs");

    // `generate_registries_rs`/`generate_block_states_rs` themselves are untouched by this
    // blueprint's addition — the same fixed content shape M0-B07's own tests already pin
    // continues to hold verbatim.
    let registries_rs = file_content(&generated.files, "registries.rs");
    assert!(registries_rs.contains("pub mod block {"));
    assert!(registries_rs.contains("pub const AIR: RegistryEntryId = RegistryEntryId(0);"));
    assert!(registries_rs.contains("pub const STONE: RegistryEntryId = RegistryEntryId(1);"));

    let block_states_rs = file_content(&generated.files, "block_states.rs");
    assert!(block_states_rs.contains("pub const BLOCK_TYPE_COUNT: u32 = 1;"));
    assert!(block_states_rs.contains("BlockStateId = BlockStateId(0)"));
}
