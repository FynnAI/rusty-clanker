//! M3.5-B02 (WS-D15) test-authoring changeset: pins every hand-authored literal id in
//! `crates/physics/src/shapes.rs`'s own `build_tier1_table()` against `rc-registries`'
//! M3.5-B01-generated per-block-state-property registry (`rc_registries::
//! block_state_properties`), proving the retirement ahead of time (both sides of every
//! equivalence below are values already present in the repository before this blueprint's
//! Implementation changeset runs).

use rc_registries::block_state_properties::{range_of, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;

/// One `build_tier1_table()` literal row, restated here by hand (never imported --
/// `shapes.rs`'s own entries are private to that module) alongside the `(block,
/// properties)` assignment this blueprint's Implementation step gives it.
struct Case {
    literal: u32,
    block: rc_registries::generated_v776::block_state_properties::BlockId,
    props: &'static [(&'static str, &'static str)],
}

#[test]
fn every_current_shapes_rs_literal_is_a_real_generated_id() {
    let cases: Vec<Case> = vec![
        Case {
            literal: 2263,
            block: block_id::PISTON,
            props: &[("extended", "false"), ("facing", "north")],
        },
        Case {
            literal: 2241,
            block: block_id::STICKY_PISTON,
            props: &[("extended", "false"), ("facing", "north")],
        },
        Case {
            literal: 3988,
            block: block_id::CHEST,
            props: &[("facing", "north")],
        },
        Case {
            literal: 3994,
            block: block_id::CHEST,
            props: &[("facing", "south")],
        },
        Case {
            literal: 4000,
            block: block_id::CHEST,
            props: &[("facing", "west")],
        },
        Case {
            literal: 4006,
            block: block_id::CHEST,
            props: &[("facing", "east")],
        },
        Case {
            literal: 11313,
            block: block_id::HOPPER,
            props: &[("facing", "down"), ("enabled", "true")],
        },
        Case {
            literal: 11314,
            block: block_id::HOPPER,
            props: &[("facing", "north"), ("enabled", "true")],
        },
        Case {
            literal: 11315,
            block: block_id::HOPPER,
            props: &[("facing", "south"), ("enabled", "true")],
        },
        Case {
            literal: 11316,
            block: block_id::HOPPER,
            props: &[("facing", "west"), ("enabled", "true")],
        },
        Case {
            literal: 11317,
            block: block_id::HOPPER,
            props: &[("facing", "east"), ("enabled", "true")],
        },
        Case {
            literal: 11318,
            block: block_id::HOPPER,
            props: &[("facing", "down"), ("enabled", "false")],
        },
        Case {
            literal: 11319,
            block: block_id::HOPPER,
            props: &[("facing", "north"), ("enabled", "false")],
        },
        Case {
            literal: 11320,
            block: block_id::HOPPER,
            props: &[("facing", "south"), ("enabled", "false")],
        },
        Case {
            literal: 11321,
            block: block_id::HOPPER,
            props: &[("facing", "west"), ("enabled", "false")],
        },
        Case {
            literal: 11322,
            block: block_id::HOPPER,
            props: &[("facing", "east"), ("enabled", "false")],
        },
        Case {
            literal: 2271,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "north"), ("short", "false"), ("type", "normal")],
        },
        Case {
            literal: 2272,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "north"), ("short", "false"), ("type", "sticky")],
        },
        Case {
            literal: 2275,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "east"), ("short", "false"), ("type", "normal")],
        },
        Case {
            literal: 2276,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "east"), ("short", "false"), ("type", "sticky")],
        },
        Case {
            literal: 2279,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "south"), ("short", "false"), ("type", "normal")],
        },
        Case {
            literal: 2280,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "south"), ("short", "false"), ("type", "sticky")],
        },
        Case {
            literal: 2283,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "west"), ("short", "false"), ("type", "normal")],
        },
        Case {
            literal: 2284,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "west"), ("short", "false"), ("type", "sticky")],
        },
        Case {
            literal: 2287,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "up"), ("short", "false"), ("type", "normal")],
        },
        Case {
            literal: 2288,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "up"), ("short", "false"), ("type", "sticky")],
        },
        Case {
            literal: 2291,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "down"), ("short", "false"), ("type", "normal")],
        },
        Case {
            literal: 2292,
            block: block_id::PISTON_HEAD,
            props: &[("facing", "down"), ("short", "false"), ("type", "sticky")],
        },
        Case {
            literal: 5328,
            block: block_id::FURNACE,
            props: &[],
        },
        Case {
            literal: 20763,
            block: block_id::BLAST_FURNACE,
            props: &[],
        },
        Case {
            literal: 20755,
            block: block_id::SMOKER,
            props: &[],
        },
    ];

    for case in cases {
        let expected = state_id(case.block, case.props)
            .expect("every case's own property set resolves to a real state id")
            .0;
        assert_eq!(
            case.literal, expected,
            "shapes.rs literal {} does not match the generated registry's own id for {:?}",
            case.literal, case.props
        );
    }
}

#[test]
fn wire_repeater_comparator_torch_ranges_match_generated_ranges() {
    let wire = range_of(block_id::REDSTONE_WIRE);
    assert_eq!((wire.first.0, wire.last.0), (4011, 5306));

    let repeater = range_of(block_id::REPEATER);
    assert_eq!((repeater.first.0, repeater.last.0), (7034, 7097));

    let comparator = range_of(block_id::COMPARATOR);
    assert_eq!((comparator.first.0, comparator.last.0), (11263, 11278));

    let torch_floor = range_of(block_id::REDSTONE_TORCH);
    assert_eq!((torch_floor.first.0, torch_floor.last.0), (6885, 6886));

    let torch_wall = range_of(block_id::REDSTONE_WALL_TORCH);
    assert_eq!((torch_wall.first.0, torch_wall.last.0), (6887, 6894));
}
