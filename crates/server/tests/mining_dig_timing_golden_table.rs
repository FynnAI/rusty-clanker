//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only); hardness varies by kind, not by property)
//! M3-B03 acceptance test: the survival dig-timing golden table (MECH-D61, Context's own
//! pinned formula/constants) -- pure, no sockets, calling `destroy_speed`/`ticks_to_break`
//! directly. See `blueprints/M3/M3-B03-breaking-placing.md`, Acceptance tests,
//! "`crates/server/tests/mining_dig_timing_golden_table.rs`".
//!
//! Every row's own hand-computed expected tick count is restated exactly from the
//! blueprint's own table; every one has been independently re-derived and cross-checked
//! against this file's own implementation before being committed here (including the two
//! resolved ambiguities `mining.rs`'s own top-of-file doc comment records: `has_correct_
//! tool_for_drops`'s `None`-bypasses-kind-too reading, load-bearing for rows 9/10/16, and
//! the `nearest_direction6` sign fix, irrelevant to this file's own dig-timing-only scope).

use rusty_clanker_server::play::{
    DestroySpeed, DigProperties, PlaceableBlockKind, ToolKind, ToolMaterial, destroy_speed,
    dig_properties, ticks_to_break,
};

struct Row {
    label: &'static str,
    props: DigProperties,
    tool: (ToolMaterial, ToolKind),
    efficiency: u8,
    haste: u8,
    fatigue: u8,
    water: bool,
    airborne: bool,
    expected_ticks: u64,
}

fn stone() -> DigProperties {
    dig_properties(PlaceableBlockKind::Stone)
}

fn dirt() -> DigProperties {
    DigProperties {
        hardness: 0.5,
        effective_tool: ToolKind::Shovel,
        min_tier_for_drops: None,
    }
}

fn grass_block() -> DigProperties {
    DigProperties {
        hardness: 0.6,
        effective_tool: ToolKind::Shovel,
        min_tier_for_drops: None,
    }
}

fn piston() -> DigProperties {
    dig_properties(PlaceableBlockKind::Piston)
}

fn chest() -> DigProperties {
    dig_properties(PlaceableBlockKind::Chest)
}

fn furnace() -> DigProperties {
    dig_properties(PlaceableBlockKind::Furnace)
}

fn hopper() -> DigProperties {
    dig_properties(PlaceableBlockKind::Hopper)
}

const BARE_HAND: (ToolMaterial, ToolKind) = (ToolMaterial::None, ToolKind::None);

fn rows() -> Vec<Row> {
    vec![
        Row {
            label: "1: Stone / bare hand",
            props: stone(),
            tool: BARE_HAND,
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 150,
        },
        Row {
            label: "2: Stone / Wood pickaxe",
            props: stone(),
            tool: (ToolMaterial::Wood, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 23,
        },
        Row {
            label: "3: Stone / Iron pickaxe",
            props: stone(),
            tool: (ToolMaterial::Iron, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 8,
        },
        Row {
            label: "4: Stone / Diamond pickaxe",
            props: stone(),
            tool: (ToolMaterial::Diamond, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 6,
        },
        Row {
            label: "5: Stone / Diamond pickaxe / Efficiency 5",
            props: stone(),
            tool: (ToolMaterial::Diamond, ToolKind::Pickaxe),
            efficiency: 5,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 2,
        },
        Row {
            label: "6: Dirt / bare hand",
            props: dirt(),
            tool: BARE_HAND,
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 15,
        },
        Row {
            label: "7: Dirt / Wood shovel",
            props: dirt(),
            tool: (ToolMaterial::Wood, ToolKind::Shovel),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 8,
        },
        Row {
            label: "8: Grass Block / Wood shovel",
            props: grass_block(),
            tool: (ToolMaterial::Wood, ToolKind::Shovel),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 9,
        },
        Row {
            label: "9: Piston / bare hand",
            props: piston(),
            tool: BARE_HAND,
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 45,
        },
        Row {
            label: "10: Piston / Iron pickaxe",
            props: piston(),
            tool: (ToolMaterial::Iron, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 8,
        },
        Row {
            label: "11: Chest / bare hand",
            props: chest(),
            tool: BARE_HAND,
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 75,
        },
        Row {
            label: "12: Chest / Wood axe",
            props: chest(),
            tool: (ToolMaterial::Wood, ToolKind::Axe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 38,
        },
        Row {
            label: "13: Furnace / bare hand",
            props: furnace(),
            tool: BARE_HAND,
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 350,
        },
        Row {
            label: "14: Furnace / Wood pickaxe",
            props: furnace(),
            tool: (ToolMaterial::Wood, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 175,
        },
        Row {
            label: "15: Furnace / Stone pickaxe",
            props: furnace(),
            tool: (ToolMaterial::Stone, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 27,
        },
        Row {
            label: "16: Hopper / Iron pickaxe",
            props: hopper(),
            tool: (ToolMaterial::Iron, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 15,
        },
        Row {
            label: "17: Stone / Iron pickaxe / Mining Fatigue II",
            props: stone(),
            tool: (ToolMaterial::Iron, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 2,
            water: false,
            airborne: false,
            expected_ticks: 84,
        },
        Row {
            label: "18: Stone / Iron pickaxe / Haste II",
            props: stone(),
            tool: (ToolMaterial::Iron, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 2,
            fatigue: 0,
            water: false,
            airborne: false,
            expected_ticks: 6,
        },
        Row {
            label: "19: Stone / Iron pickaxe / water, no Aqua Affinity",
            props: stone(),
            tool: (ToolMaterial::Iron, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: true,
            airborne: false,
            expected_ticks: 38,
        },
        Row {
            label: "20: Stone / Iron pickaxe / water + airborne",
            props: stone(),
            tool: (ToolMaterial::Iron, ToolKind::Pickaxe),
            efficiency: 0,
            haste: 0,
            fatigue: 0,
            water: true,
            airborne: true,
            expected_ticks: 188,
        },
    ]
}

#[test]
fn golden_table_matches_hand_computed_tick_counts_exactly() {
    for row in rows() {
        let speed = destroy_speed(
            row.props,
            row.tool,
            row.efficiency,
            row.haste,
            row.fatigue,
            row.water,
            row.airborne,
        );
        let ticks = ticks_to_break(speed);
        assert_eq!(
            ticks, row.expected_ticks,
            "row {} expected {} ticks, got {}",
            row.label, row.expected_ticks, ticks
        );
    }
}

#[test]
fn redstone_wire_torch_repeater_comparator_are_always_instant() {
    for kind in [
        PlaceableBlockKind::RedstoneWire,
        PlaceableBlockKind::RedstoneTorch,
        PlaceableBlockKind::Repeater,
        PlaceableBlockKind::Comparator,
    ] {
        let props = dig_properties(kind);
        // Deliberately a "slow" combination (bare hand + Mining Fatigue IV) -- still
        // instant, unconditionally (Context: hardness == 0 is a special case, not derived
        // from the general formula).
        let speed = destroy_speed(
            props,
            (ToolMaterial::None, ToolKind::None),
            0,
            0,
            4,
            false,
            false,
        );
        assert_eq!(
            speed,
            DestroySpeed::Instant,
            "{kind:?} must always be Instant"
        );
    }
}

#[test]
fn bedrock_is_unbreakable() {
    // `Bedrock` is not itself a `PlaceableBlockKind` (breakable-in-survival-never but never
    // placeable), so this test constructs its own synthetic `DigProperties` directly.
    let bedrock = DigProperties {
        hardness: -1.0,
        effective_tool: ToolKind::None,
        min_tier_for_drops: None,
    };
    let combos: [(ToolMaterial, ToolKind); 3] = [
        (ToolMaterial::None, ToolKind::None),
        (ToolMaterial::Diamond, ToolKind::Pickaxe),
        (ToolMaterial::Netherite, ToolKind::Pickaxe),
    ];
    for tool in combos {
        let speed = destroy_speed(bedrock, tool, 5, 2, 0, false, false);
        assert_eq!(
            speed,
            DestroySpeed::Unbreakable,
            "bedrock must always be Unbreakable"
        );
    }
}
