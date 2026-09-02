//! test-matrix: boundaries=waived(range first/last endpoints are asserted as direct equality checks against range_of, not swept as a behavioral boundary condition) orientations=yes self=waived(no player/actor entity in this suite's own domain model -- pure id arithmetic) composition=waived(single-block id arithmetic per test, no multi-component redstone chain) nondefault-state=yes
//! M3.5-B02 (WS-D15) test-authoring changeset: pins every hand-derived block-state-id
//! constant/formula this blueprint retires (`crates/mechanics/src/redstone/{wire,torch,
//! repeater,comparator,piston,registration}.rs`) against `rc-registries`' M3.5-B01-generated
//! per-block-state-property registry (`rc_registries::block_state_properties`). Every hand
//! formula below is restated inline (never imported from the production modules it mirrors)
//! so this file is what pins the two in agreement — exactly the discipline §5 of
//! `blueprints/M3.5/M3.5-B02-retire-hand-authored-id-tables.md` specifies. Every assertion
//! here is true today, before the Implementation changeset retires any production arithmetic
//! (both sides of every equivalence below are values already present in the repository) —
//! this changeset only adds a second, generated-registry-backed way to compute the same
//! values, proving the retirement ahead of time.

use rc_registries::block_state_properties::{range_of, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state;

#[test]
fn wire_range_matches_generated_table() {
    let range = range_of(block_id::REDSTONE_WIRE);
    // source: blocks.json
    assert_eq!(range.first.0, 4011);
    // source: blocks.json
    assert_eq!(range.last.0, 5306);
}

const WIRE_SIDE_STRINGS: [&str; 3] = ["up", "side", "none"];

/// Restates `wire.rs`'s own pre-retirement formula (`WIRE_BASE + east*432 + north*144 +
/// power*9 + south*3 + west`) — never imported.
fn wire_hand_formula(east: u32, north: u32, power: u8, south: u32, west: u32) -> u32 {
    const WIRE_BASE: u32 = 4011;
    WIRE_BASE + east * 432 + north * 144 + u32::from(power) * 9 + south * 3 + west
}

#[test]
fn wire_state_id_matches_hand_formula() {
    // `east`/`north`/`south`/`west` each ∈ {0=up,1=side,2=none} at least once; `power` ∈
    // {0,7,15} at least once.
    let mut cases: Vec<(u32, u32, u8, u32, u32)> = vec![
        (0, 0, 0, 0, 0),
        (1, 1, 0, 1, 1),
        (2, 2, 0, 2, 2),
        (0, 1, 7, 2, 0),
        (2, 0, 15, 1, 2),
    ];
    // A few more mixed combinations for good measure -- still a representative sample, not
    // an exhaustive 3^4 * 16 sweep.
    cases.push((1, 2, 15, 0, 1));
    cases.push((0, 2, 7, 1, 0));

    for (east, north, power, south, west) in cases {
        let expected = wire_hand_formula(east, north, power, south, west);
        let actual = state_id(
            block_id::REDSTONE_WIRE,
            &[
                ("east", WIRE_SIDE_STRINGS[east as usize]),
                ("north", WIRE_SIDE_STRINGS[north as usize]),
                ("power", &power.to_string()),
                ("south", WIRE_SIDE_STRINGS[south as usize]),
                ("west", WIRE_SIDE_STRINGS[west as usize]),
            ],
        )
        .expect("every legal wire property combination resolves to a real state id")
        .0;
        assert_eq!(
            actual, expected,
            "east={east} north={north} power={power} south={south} west={west}"
        );
    }
}

#[test]
fn torch_floor_range_matches_generated_table() {
    let range = range_of(block_id::REDSTONE_TORCH);
    // source: blocks.json
    assert_eq!(range.first.0, 6885);
    // source: blocks.json
    assert_eq!(range.last.0, 6886);
}

#[test]
fn torch_wall_range_matches_generated_table() {
    let range = range_of(block_id::REDSTONE_WALL_TORCH);
    // source: blocks.json
    assert_eq!(range.first.0, 6887);
    // source: blocks.json
    assert_eq!(range.last.0, 6894);
}

fn diode_facing_str(idx: u32) -> &'static str {
    match idx {
        0 => "north",
        1 => "south",
        2 => "west",
        3 => "east",
        other => panic!("diode_facing_str: index must be 0..=3, got {other}"),
    }
}

#[test]
fn repeater_nondefault_delay_locked_powered_range_and_state_id_match() {
    let range = range_of(block_id::REPEATER);
    // source: blocks.json
    assert_eq!(range.first.0, 7034);
    // source: blocks.json
    assert_eq!(range.last.0, 7097);

    for delay in [1u32, 4u32] {
        for facing_idx in 0..4u32 {
            for locked in [true, false] {
                for powered in [true, false] {
                    // Restates `repeater.rs`'s own pre-retirement formula: `7034 + (delay-1)*16
                    // + facing_idx*4 + !locked*2 + !powered` -- never imported.
                    let expected = 7034
                        + (delay - 1) * 16
                        + facing_idx * 4
                        + u32::from(!locked) * 2
                        + u32::from(!powered);
                    let actual = state_id(
                        block_id::REPEATER,
                        &[
                            ("delay", &delay.to_string()),
                            ("facing", diode_facing_str(facing_idx)),
                            ("locked", if locked { "true" } else { "false" }),
                            ("powered", if powered { "true" } else { "false" }),
                        ],
                    )
                    .expect("every legal repeater property combination resolves")
                    .0;
                    assert_eq!(
                        actual, expected,
                        "delay={delay} facing_idx={facing_idx} locked={locked} powered={powered}"
                    );
                }
            }
        }
    }
}

#[test]
fn comparator_range_and_state_id_match() {
    let range = range_of(block_id::COMPARATOR);
    // source: blocks.json
    assert_eq!(range.first.0, 11263);
    // source: blocks.json
    assert_eq!(range.last.0, 11278);

    for mode_idx in 0..2u32 {
        let mode_str = if mode_idx == 0 { "compare" } else { "subtract" };
        for facing_idx in 0..4u32 {
            for powered in [true, false] {
                // Restates `comparator.rs`'s own pre-retirement formula: `11263 + facing_idx*4
                // + mode_idx*2 + !powered` -- never imported.
                let expected = 11263 + facing_idx * 4 + mode_idx * 2 + u32::from(!powered);
                let actual = state_id(
                    block_id::COMPARATOR,
                    &[
                        ("facing", diode_facing_str(facing_idx)),
                        ("mode", mode_str),
                        ("powered", if powered { "true" } else { "false" }),
                    ],
                )
                .expect("every legal comparator property combination resolves")
                .0;
                assert_eq!(
                    actual, expected,
                    "facing_idx={facing_idx} mode={mode_str} powered={powered}"
                );
            }
        }
    }
}

#[test]
fn piston_and_sticky_piston_ranges_match() {
    let piston = range_of(block_id::PISTON);
    // source: blocks.json
    assert_eq!(piston.first.0, 2257);
    // source: blocks.json
    assert_eq!(piston.last.0, 2268);
    let sticky = range_of(block_id::STICKY_PISTON);
    // source: blocks.json
    assert_eq!(sticky.first.0, 2235);
    // source: blocks.json
    assert_eq!(sticky.last.0, 2246);
}

#[test]
fn piston_head_range_matches_generated_table() {
    let range = range_of(block_id::PISTON_HEAD);
    // source: blocks.json
    assert_eq!(range.first.0, 2269);
    // source: blocks.json
    assert_eq!(range.last.0, 2292);
}

fn piston_facing_str(idx: u32) -> &'static str {
    match idx {
        0 => "north",
        1 => "east",
        2 => "south",
        3 => "west",
        4 => "up",
        5 => "down",
        other => panic!("piston_facing_str: index must be 0..=5, got {other}"),
    }
}

#[test]
fn piston_state_id_matches_hand_formula() {
    for sticky in [true, false] {
        let block = if sticky {
            block_id::STICKY_PISTON
        } else {
            block_id::PISTON
        };
        let base = if sticky { 2235 } else { 2257 };
        for extended in [true, false] {
            for facing_idx in 0..6u32 {
                // Restates `piston.rs`'s own pre-retirement formula: `base(sticky) +
                // !extended*6 + facing_idx` -- never imported.
                let expected = base + u32::from(!extended) * 6 + facing_idx;
                let actual = state_id(
                    block,
                    &[
                        ("extended", if extended { "true" } else { "false" }),
                        ("facing", piston_facing_str(facing_idx)),
                    ],
                )
                .expect("every legal piston property combination resolves")
                .0;
                assert_eq!(
                    actual, expected,
                    "sticky={sticky} extended={extended} facing_idx={facing_idx}"
                );
            }
        }
    }
}

#[test]
fn piston_head_facing_orientation_id_matches_hand_formula() {
    for facing_idx in 0..6u32 {
        for sticky in [true, false] {
            // Restates `piston.rs`'s own pre-retirement formula: `2269 + facing_idx*4 + 2 +
            // sticky as u32` (`short` fixed `false`) -- never imported.
            let expected = 2269 + facing_idx * 4 + 2 + u32::from(sticky);
            let actual = state_id(
                block_id::PISTON_HEAD,
                &[
                    ("facing", piston_facing_str(facing_idx)),
                    ("short", "false"),
                    ("type", if sticky { "sticky" } else { "normal" }),
                ],
            )
            .expect("every legal piston_head property combination resolves")
            .0;
            assert_eq!(actual, expected, "facing_idx={facing_idx} sticky={sticky}");
        }
    }
}

#[test]
fn redstone_block_id_matches_generated_default() {
    // source: blocks.json
    assert_eq!(default_state::REDSTONE_BLOCK.0, 11311);
}

#[test]
fn respawn_anchor_range_matches_generated_table() {
    let range = range_of(block_id::RESPAWN_ANCHOR);
    // source: blocks.json
    assert_eq!(range.first.0, 21821);
    // source: blocks.json
    assert_eq!(range.last.0, 21825);
}

#[test]
fn destroy_and_block_entity_immovable_literals_match_generated_defaults() {
    // Restates `piston.rs`'s own pre-retirement `DESTROY_IDS`/`BLOCK_ENTITY_IMMOVABLE_IDS`
    // literal arrays (both private consts, unreachable from this external test crate) --
    // never imported.
    const DESTROY_IDS: [u32; 5] = [5171, 6885, 6887, 7037, 11264];
    const BLOCK_ENTITY_IMMOVABLE_IDS: [u32; 5] = [3988, 5328, 20763, 20755, 11313];

    assert_eq!(
        DESTROY_IDS,
        [
            default_state::REDSTONE_WIRE.0,
            default_state::REDSTONE_TORCH.0,
            default_state::REDSTONE_WALL_TORCH.0,
            default_state::REPEATER.0,
            default_state::COMPARATOR.0,
        ]
    );
    assert_eq!(
        BLOCK_ENTITY_IMMOVABLE_IDS,
        [
            default_state::CHEST.0,
            default_state::FURNACE.0,
            default_state::BLAST_FURNACE.0,
            default_state::SMOKER.0,
            default_state::HOPPER.0,
        ]
    );
}

#[test]
fn bedrock_obsidian_family_ids_match_generated_defaults() {
    // Restates `piston.rs`'s own pre-retirement literals -- never imported.
    const BEDROCK_ID: u32 = 85;
    const OBSIDIAN_ID: u32 = 3369;
    const CRYING_OBSIDIAN_ID: u32 = 21820;
    const REINFORCED_DEEPSLATE_ID: u32 = 32085;

    assert_eq!(BEDROCK_ID, default_state::BEDROCK.0);
    assert_eq!(OBSIDIAN_ID, default_state::OBSIDIAN.0);
    assert_eq!(CRYING_OBSIDIAN_ID, default_state::CRYING_OBSIDIAN.0);
    assert_eq!(
        REINFORCED_DEEPSLATE_ID,
        default_state::REINFORCED_DEEPSLATE.0
    );
}
