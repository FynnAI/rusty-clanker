use xtask::datagen::codegen::{generate_block_state_properties_rs, is_state_replaceable};
use xtask::datagen::reports::{
    BlockReport, BlockStateReport, BlocksReport, OrderedProperties, OrderedValueList,
};

/// Builds one `BlockStateReport` with the given `id`/`default` flag and
/// `(name, value)` property pairs, in the exact order given (mirrors a real report's
/// own per-state key order — Context §3.2/§3.4).
fn state(id: u32, default: bool, props: &[(&str, &str)]) -> BlockStateReport {
    BlockStateReport {
        id,
        default,
        properties: OrderedProperties(
            props
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        ),
    }
}

fn block(states: Vec<BlockStateReport>) -> BlockReport {
    BlockReport {
        states,
        properties: OrderedValueList::default(),
    }
}

/// `minecraft:redstone_wire`'s real state range (4011..=5306, §5.2 of the blueprint)
/// built as 1296 contiguous states — required by `generate_block_state_properties_rs`'s
/// own per-block `last - first + 1 == states.len()` malformed-report defense
/// (Implementation step 8). Only the first (4011) and last (5306) states carry the
/// real, report-verified property set (TEST-D57); every state in between is an inert,
/// property-less filler — this test only asserts on the two real endpoints.
fn redstone_wire_fixture_states() -> Vec<BlockStateReport> {
    let mut states = Vec::with_capacity(1296);
    states.push(state(
        4011,
        true,
        &[
            ("east", "up"),
            ("north", "up"),
            ("power", "0"),
            ("south", "up"),
            ("west", "up"),
        ],
    ));
    for id in 4012..5306 {
        states.push(state(id, false, &[]));
    }
    states.push(state(
        5306,
        false,
        &[
            ("east", "none"),
            ("north", "none"),
            ("power", "15"),
            ("south", "none"),
            ("west", "none"),
        ],
    ));
    states
}

#[test]
fn redstone_wire_state_zero_decodes_to_reported_properties() {
    let mut blocks = BlocksReport::new();
    blocks.insert(
        "minecraft:air".to_string(),
        block(vec![state(0, true, &[])]),
    );
    blocks.insert(
        "minecraft:redstone_wire".to_string(),
        block(redstone_wire_fixture_states()),
    );

    let content = generate_block_state_properties_rs(&blocks);

    let pos_east_up = content
        .find("(\"east\", \"up\")")
        .expect("missing (\"east\", \"up\") in:\n");
    let pos_north_up = content
        .find("(\"north\", \"up\")")
        .expect("missing (\"north\", \"up\")");
    let pos_power_0 = content
        .find("(\"power\", \"0\")")
        .expect("missing (\"power\", \"0\")");
    let pos_south_up = content
        .find("(\"south\", \"up\")")
        .expect("missing (\"south\", \"up\")");
    let pos_west_up = content
        .find("(\"west\", \"up\")")
        .expect("missing (\"west\", \"up\")");
    assert!(
        pos_east_up < pos_north_up
            && pos_north_up < pos_power_0
            && pos_power_0 < pos_south_up
            && pos_south_up < pos_west_up,
        "expected state 4011's row (east, north, power, south, west) in report order, got positions \
         {pos_east_up}, {pos_north_up}, {pos_power_0}, {pos_south_up}, {pos_west_up} in:\n{content}"
    );

    let pos_east_none = content
        .find("(\"east\", \"none\")")
        .expect("missing (\"east\", \"none\")");
    let pos_north_none = content
        .find("(\"north\", \"none\")")
        .expect("missing (\"north\", \"none\")");
    let pos_power_15 = content
        .find("(\"power\", \"15\")")
        .expect("missing (\"power\", \"15\")");
    let pos_south_none = content
        .find("(\"south\", \"none\")")
        .expect("missing (\"south\", \"none\")");
    let pos_west_none = content
        .find("(\"west\", \"none\")")
        .expect("missing (\"west\", \"none\")");
    assert!(
        pos_east_none < pos_north_none
            && pos_north_none < pos_power_15
            && pos_power_15 < pos_south_none
            && pos_south_none < pos_west_none,
        "expected state 5306's row (east, north, power, south, west) in report order, got positions \
         {pos_east_none}, {pos_north_none}, {pos_power_15}, {pos_south_none}, {pos_west_none} in:\n{content}"
    );
}

#[test]
fn block_ranges_first_last_default_are_read_not_derived() {
    let mut blocks = BlocksReport::new();
    // Deliberately scrambled Vec order (102, 100, 101) -- ids are still contiguous
    // (100..=102), proving first/last/default come from `.min()`/`.max()`/the
    // `default`-flagged entry, never from Vec position.
    blocks.insert(
        "minecraft:test_block".to_string(),
        block(vec![
            state(102, false, &[]),
            state(100, false, &[]),
            state(101, true, &[]),
        ]),
    );

    let content = generate_block_state_properties_rs(&blocks);

    assert!(
        content.contains(
            "BlockStateRange { first: BlockStateId(100), last: BlockStateId(102), default: BlockStateId(101) }"
        ),
        "missing expected BLOCK_RANGES row in:\n{content}"
    );
}

#[test]
fn state_property_order_matches_report_order_not_alphabetical() {
    let mut blocks = BlocksReport::new();
    blocks.insert(
        "minecraft:chest_like".to_string(),
        block(vec![state(
            0,
            true,
            &[
                ("type", "single"),
                ("facing", "north"),
                ("waterlogged", "true"),
            ],
        )]),
    );

    let content = generate_block_state_properties_rs(&blocks);

    let pos_type = content
        .find("(\"type\", \"single\")")
        .expect("missing type tuple");
    let pos_facing = content
        .find("(\"facing\", \"north\")")
        .expect("missing facing tuple");
    let pos_waterlogged = content
        .find("(\"waterlogged\", \"true\")")
        .expect("missing waterlogged tuple");
    assert!(
        pos_type < pos_facing && pos_facing < pos_waterlogged,
        "expected type, facing, waterlogged (report order), not alphabetical, got positions \
         {pos_type}, {pos_facing}, {pos_waterlogged} in:\n{content}"
    );
}

#[test]
fn block_id_module_orders_blocks_alphabetically_by_full_name() {
    let mut blocks = BlocksReport::new();
    blocks.insert(
        "minecraft:stone".to_string(),
        block(vec![state(0, true, &[])]),
    );
    blocks.insert(
        "minecraft:andesite".to_string(),
        block(vec![state(1, true, &[])]),
    );

    let content = generate_block_state_properties_rs(&blocks);

    let pos_andesite = content
        .find("pub const ANDESITE: BlockId")
        .expect("missing block_id::ANDESITE");
    let pos_stone = content
        .find("pub const STONE: BlockId")
        .expect("missing block_id::STONE");
    assert!(
        pos_andesite < pos_stone,
        "expected ANDESITE (alphabetically first) before STONE, got positions {pos_andesite} and {pos_stone}"
    );
}

#[test]
fn is_state_replaceable_matches_listed_block_by_name() {
    assert!(is_state_replaceable("water", &[]));
    assert!(!is_state_replaceable("stone", &[]));
}

#[test]
fn is_state_replaceable_snow_requires_layers_one() {
    assert!(is_state_replaceable(
        "snow",
        &[("layers".to_string(), "1".to_string())]
    ));
    assert!(!is_state_replaceable(
        "snow",
        &[("layers".to_string(), "2".to_string())]
    ));
}

#[test]
fn generated_block_state_properties_compiles_standalone() {
    let mut blocks = BlocksReport::new();
    blocks.insert(
        "minecraft:air".to_string(),
        block(vec![state(0, true, &[])]),
    );
    blocks.insert(
        "minecraft:chest".to_string(),
        block(vec![
            state(1, true, &[("type", "single"), ("facing", "north")]),
            state(2, false, &[("type", "single"), ("facing", "south")]),
        ]),
    );

    let content = generate_block_state_properties_rs(&blocks);

    // `block_state_properties.rs` references `super::block_states::BlockStateId`, so
    // (mirroring `datagen_codegen.rs`'s own `generated_files_compile_standalone`
    // technique) it must be compiled alongside a minimal stand-in `block_states`
    // module providing that same type, wired together via a tiny crate-root `lib.rs`.
    let out_dir = std::env::temp_dir().join(format!(
        "rc_xtask_block_state_properties_compile_check_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&out_dir).unwrap();

    let block_states_stub = "#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct BlockStateId(pub u32);\n";
    std::fs::write(out_dir.join("block_states.rs"), block_states_stub).unwrap();
    std::fs::write(out_dir.join("block_state_properties.rs"), &content).unwrap();
    let lib_rs = "pub mod block_states;\npub mod block_state_properties;\n";
    let lib_path = out_dir.join("lib.rs");
    std::fs::write(&lib_path, lib_rs).unwrap();

    let status = std::process::Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("--crate-type")
        .arg("lib")
        .arg("--out-dir")
        .arg(&out_dir)
        .arg(&lib_path)
        .status()
        .expect("failed to invoke rustc");
    assert!(
        status.success(),
        "rustc failed to compile block_state_properties.rs"
    );
}

#[test]
fn output_is_independent_of_input_insertion_order() {
    fn make(reverse: bool) -> BlocksReport {
        let mut names = vec![
            ("minecraft:air", state(0, true, &[])),
            (
                "minecraft:chest",
                state(1, true, &[("type", "single"), ("facing", "north")]),
            ),
        ];
        if reverse {
            names.reverse();
        }
        let mut blocks = BlocksReport::new();
        for (name, st) in names {
            blocks.insert(name.to_string(), block(vec![st]));
        }
        blocks
    }

    let a = generate_block_state_properties_rs(&make(false));
    let b = generate_block_state_properties_rs(&make(true));
    assert_eq!(a, b);
}
