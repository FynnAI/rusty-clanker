use std::collections::BTreeMap;

use xtask::datagen::codegen::generate;
use xtask::datagen::reports::{
    BlockReport, BlockStateReport, BlocksReport, RegistriesReport, RegistryEntryReport,
    RegistryReport,
};

fn file_content<'a>(files: &'a [(String, String)], name: &str) -> &'a str {
    &files
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no file named {name} in generated output"))
        .1
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

    let generated = generate(&registries, &blocks);
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

    let generated = generate(&registries, &BlocksReport::new());
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

    let generated = generate(&registries, &BlocksReport::new());
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
                    states: vec![BlockStateReport { id, default: true }],
                },
            );
        }
        (registries, blocks)
    }

    let a = make_pair(false);
    let b = make_pair(true);

    assert_eq!(generate(&a.0, &a.1).files, generate(&b.0, &b.1).files);
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
            }],
        },
    );
    blocks.insert(
        "minecraft:oak_door".to_string(),
        BlockReport {
            states: vec![
                BlockStateReport {
                    id: 5655,
                    default: false,
                },
                BlockStateReport {
                    id: 5680,
                    default: true,
                },
                BlockStateReport {
                    id: 5718,
                    default: false,
                },
            ],
        },
    );

    let generated = generate(&RegistriesReport::new(), &blocks);
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
            }],
        },
    );

    let generated = generate(&registries, &blocks);

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
