//! WS-D15 (M3.5-B01): runs against the **real** generated `v776` table (no fixtures —
//! the whole point is proving the real registry decodes real vanilla ids). Every
//! anchor below was read directly off the local `blocks.json` report (ASSET-D18(a))
//! and cross-checked against `crates/mechanics/src/redstone/{wire,repeater,comparator,
//! piston,torch}.rs`'s and `crates/server/src/play/mining.rs`'s own already-shipped,
//! independently-derived constants — all agree (TEST-D57, `M3.5-B01-CLAIMS.md`).

use rc_registries::block_state_properties::{
    block_of, is_replaceable, properties, range_of, state_id, with_property,
};
use rc_registries::generated_v776::block_state_properties::{BlockStateRange, block_id};
use rc_registries::generated_v776::block_states::{BlockStateId, default_state};

struct Anchor {
    block: rc_registries::generated_v776::block_state_properties::BlockId,
    base: u32,
    base_props: &'static [(&'static str, &'static str)],
    max: u32,
    max_props: &'static [(&'static str, &'static str)],
    default: u32,
    default_props: &'static [(&'static str, &'static str)],
}

fn anchors() -> Vec<Anchor> {
    vec![
        Anchor {
            block: block_id::REDSTONE_WIRE,
            base: 4011,
            base_props: &[
                ("east", "up"),
                ("north", "up"),
                ("power", "0"),
                ("south", "up"),
                ("west", "up"),
            ],
            max: 5306,
            max_props: &[
                ("east", "none"),
                ("north", "none"),
                ("power", "15"),
                ("south", "none"),
                ("west", "none"),
            ],
            default: 5171,
            default_props: &[
                ("east", "none"),
                ("north", "none"),
                ("power", "0"),
                ("south", "none"),
                ("west", "none"),
            ],
        },
        Anchor {
            block: block_id::REPEATER,
            base: 7034,
            base_props: &[
                ("delay", "1"),
                ("facing", "north"),
                ("locked", "true"),
                ("powered", "true"),
            ],
            max: 7097,
            max_props: &[
                ("delay", "4"),
                ("facing", "east"),
                ("locked", "false"),
                ("powered", "false"),
            ],
            default: 7037,
            default_props: &[
                ("delay", "1"),
                ("facing", "north"),
                ("locked", "false"),
                ("powered", "false"),
            ],
        },
        Anchor {
            block: block_id::COMPARATOR,
            base: 11263,
            base_props: &[
                ("facing", "north"),
                ("mode", "compare"),
                ("powered", "true"),
            ],
            max: 11278,
            max_props: &[
                ("facing", "east"),
                ("mode", "subtract"),
                ("powered", "false"),
            ],
            default: 11264,
            default_props: &[
                ("facing", "north"),
                ("mode", "compare"),
                ("powered", "false"),
            ],
        },
        Anchor {
            block: block_id::PISTON,
            base: 2257,
            base_props: &[("extended", "true"), ("facing", "north")],
            max: 2268,
            max_props: &[("extended", "false"), ("facing", "down")],
            default: 2263,
            default_props: &[("extended", "false"), ("facing", "north")],
        },
        Anchor {
            block: block_id::STICKY_PISTON,
            base: 2235,
            base_props: &[("extended", "true"), ("facing", "north")],
            max: 2246,
            max_props: &[("extended", "false"), ("facing", "down")],
            default: 2241,
            default_props: &[("extended", "false"), ("facing", "north")],
        },
        Anchor {
            block: block_id::CHEST,
            base: 3987,
            base_props: &[
                ("type", "single"),
                ("facing", "north"),
                ("waterlogged", "true"),
            ],
            max: 4010,
            max_props: &[
                ("type", "right"),
                ("facing", "east"),
                ("waterlogged", "false"),
            ],
            default: 3988,
            default_props: &[
                ("type", "single"),
                ("facing", "north"),
                ("waterlogged", "false"),
            ],
        },
        Anchor {
            block: block_id::FURNACE,
            base: 5327,
            base_props: &[("facing", "north"), ("lit", "true")],
            max: 5334,
            max_props: &[("facing", "east"), ("lit", "false")],
            default: 5328,
            default_props: &[("facing", "north"), ("lit", "false")],
        },
        Anchor {
            block: block_id::HOPPER,
            base: 11313,
            base_props: &[("enabled", "true"), ("facing", "down")],
            max: 11322,
            max_props: &[("enabled", "false"), ("facing", "east")],
            default: 11313,
            default_props: &[("enabled", "true"), ("facing", "down")],
        },
        Anchor {
            block: block_id::REDSTONE_TORCH,
            base: 6885,
            base_props: &[("lit", "true")],
            max: 6886,
            max_props: &[("lit", "false")],
            default: 6885,
            default_props: &[("lit", "true")],
        },
        Anchor {
            block: block_id::REDSTONE_WALL_TORCH,
            base: 6887,
            base_props: &[("facing", "north"), ("lit", "true")],
            max: 6894,
            max_props: &[("facing", "east"), ("lit", "false")],
            default: 6887,
            default_props: &[("facing", "north"), ("lit", "true")],
        },
    ]
}

fn sorted(props: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = props
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    v.sort();
    v
}

#[test]
fn decode_anchors() {
    for anchor in anchors() {
        let base_actual = sorted(properties(BlockStateId(anchor.base)));
        assert_eq!(
            base_actual,
            sorted(anchor.base_props),
            "base state {} decoded wrong",
            anchor.base
        );
        let max_actual = sorted(properties(BlockStateId(anchor.max)));
        assert_eq!(
            max_actual,
            sorted(anchor.max_props),
            "max state {} decoded wrong",
            anchor.max
        );
        let default_actual = sorted(properties(BlockStateId(anchor.default)));
        assert_eq!(
            default_actual,
            sorted(anchor.default_props),
            "default state {} decoded wrong",
            anchor.default
        );
    }
}

#[test]
fn block_of_anchors() {
    for anchor in anchors() {
        assert_eq!(block_of(BlockStateId(anchor.base)), anchor.block);
        assert_eq!(block_of(BlockStateId(anchor.max)), anchor.block);
        assert_eq!(block_of(BlockStateId(anchor.default)), anchor.block);
    }
}

#[test]
fn range_of_anchors() {
    for anchor in anchors() {
        assert_eq!(
            range_of(anchor.block),
            BlockStateRange {
                first: BlockStateId(anchor.base),
                last: BlockStateId(anchor.max),
                default: BlockStateId(anchor.default),
            }
        );
    }
}

#[test]
fn with_property_round_trips_repeater_facing() {
    let east = with_property(default_state::REPEATER, "facing", "east");
    assert_eq!(east, Some(BlockStateId(7049)));
    let back = with_property(east.unwrap(), "facing", "north");
    assert_eq!(back, Some(BlockStateId(7037)));
}

#[test]
fn with_property_illegal_value_is_none() {
    assert_eq!(with_property(default_state::REPEATER, "delay", "5"), None);
}

#[test]
fn with_property_unknown_property_is_none() {
    assert_eq!(
        with_property(default_state::REPEATER, "waterlogged", "true"),
        None
    );
}

#[test]
fn state_id_partial_resolution_fills_defaults() {
    let resolved = state_id(block_id::CHEST, &[("facing", "east")])
        .expect("chest with facing=east should resolve");
    let expected_props = sorted(&[
        ("type", "single"),
        ("facing", "east"),
        ("waterlogged", "false"),
    ]);
    assert_eq!(sorted(properties(resolved)), expected_props);
}

#[test]
fn state_id_full_specification_matches_partial() {
    let full = state_id(
        block_id::CHEST,
        &[
            ("type", "left"),
            ("facing", "south"),
            ("waterlogged", "false"),
        ],
    );
    let partial = state_id(block_id::CHEST, &[("type", "left"), ("facing", "south")]);
    assert_eq!(full, partial);
    assert!(full.is_some());
}

#[test]
fn state_id_unknown_property_name_is_none() {
    assert_eq!(state_id(block_id::REPEATER, &[("nonexistent", "x")]), None);
}

#[test]
fn is_replaceable_water_true_stone_false() {
    assert!(is_replaceable(default_state::WATER));
    assert!(!is_replaceable(default_state::STONE));
}

#[test]
fn is_replaceable_snow_layers_one_true_layers_two_false() {
    let layers_one = state_id(block_id::SNOW, &[("layers", "1")]).expect("snow layers=1");
    let layers_two = state_id(block_id::SNOW, &[("layers", "2")]).expect("snow layers=2");
    assert_ne!(is_replaceable(layers_one), is_replaceable(layers_two));
}

// Local, minimal mirror of `xtask::fixture_manifest`'s manifest shape and
// SHA-256-verification logic (TEST-D47) -- deliberately NOT a dependency on the
// `xtask` crate itself, which would create a WS-D3 rule-2 violation (see this
// crate's own `Cargo.toml` `[dev-dependencies]` comment: `rc-mechanics`, a SIM
// crate, already depends on `rc-registries`; `xtask` depends on `rc-auth`, a
// NETRENDER crate).
#[derive(serde::Deserialize)]
struct LocalFixtureManifest {
    entries: Vec<LocalFixtureEntry>,
}

#[derive(serde::Deserialize)]
struct LocalFixtureEntry {
    path: String,
    sha256: String,
}

fn local_sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn manifest_lists_and_verifies_block_state_properties_file() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("generated/v776");
    let manifest_path = manifest_dir.join("MANIFEST.json");
    let manifest_text =
        std::fs::read_to_string(&manifest_path).expect("failed to read MANIFEST.json");
    let manifest: LocalFixtureManifest =
        serde_json::from_str(&manifest_text).expect("failed to parse MANIFEST.json");
    let entry = manifest
        .entries
        .iter()
        .find(|e| e.path == "block_state_properties.rs")
        .expect("MANIFEST.json has no entry for block_state_properties.rs");

    let on_disk = std::fs::read(manifest_dir.join(&entry.path))
        .expect("failed to read block_state_properties.rs from disk");
    let actual_sha256 = local_sha256_hex(&on_disk);
    assert_eq!(
        actual_sha256, entry.sha256,
        "block_state_properties.rs on disk does not match its MANIFEST.json sha256"
    );
}

#[test]
fn every_block_range_is_contiguous_and_matches_state_count() {
    use rc_registries::generated_v776::block_state_properties::STATE_BLOCK;
    use rc_registries::generated_v776::block_states::BLOCK_TYPE_COUNT;

    for raw in 0..BLOCK_TYPE_COUNT {
        let block = rc_registries::generated_v776::block_state_properties::BlockId(raw);
        let range = range_of(block);
        let expected_width = range.last.0 - range.first.0 + 1;
        let actual_width = STATE_BLOCK.iter().filter(|b| **b == block).count() as u32;
        assert_eq!(
            expected_width, actual_width,
            "block {raw}: range width {expected_width} != {actual_width} STATE_BLOCK rows"
        );
    }
}
