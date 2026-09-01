//! M3 field-report test-authoring (Root Cause 1, "placeholder id table"): pure, no-socket
//! literal-id regression tests for `mining::tier1_oriented_state_table()`. Before this fix,
//! every non-default oriented entry was `<default-state id> + <arbitrary direction index>`
//! (`mining.rs`'s own former doc comment: "internally consistent for this table's own routing
//! tests, not claimed to be a real vanilla id") -- every literal value asserted below was
//! decoded by hand from the local datagen reference
//! (`mc-research/26.2/datagen/generated/reports/blocks.json`, protocol 776, never committed --
//! this project's own established reference-source convention) and is restated here as the
//! authoritative expectation, per block, with the exact decoded property string from the
//! `states` array in a comment beside each assertion. This file also doubles as the "loud
//! test-time integrity anchor" the task brief asks for: every default-orientation assertion
//! below is, by `tier1_oriented_entries()`'s own construction, an assertion that
//! `rc_registries::generated_v776::block_states::default_state::*` still holds the exact value
//! this file's own literal comments were verified against -- a future pinned-version bump that
//! regenerates those constants differently fails one of these assertions immediately and
//! loudly, rather than silently mis-placing every block of that kind from then on.
//!
//! Every assertion here goes through the identical `tier1_oriented_state_table()` +
//! `resolve_orientation()` calls `mining::apply_placement` itself uses -- never a hand-copied
//! constant re-derivation -- so this file also transitively pins those two functions' own
//! wiring together, not merely the raw literal table in isolation.

use rusty_clanker_server::play::{
    Face, PlaceableBlockKind, resolve_orientation, tier1_oriented_state_table,
};

/// `resolve_orientation` + `tier1_oriented_state_table().lookup` in one step -- the identical
/// two-step lookup `mining::apply_placement` itself performs for a block placed by clicking
/// `clicked_face` while looking at `(yaw_degrees, pitch_degrees)`.
fn placed_id(
    kind: PlaceableBlockKind,
    clicked_face: Face,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> u32 {
    let selection = resolve_orientation(kind, clicked_face, yaw_degrees, pitch_degrees)
        .expect("every (kind, face, yaw, pitch) combination this file passes is a legal placement");
    tier1_oriented_state_table().lookup(selection.kind, selection.orientation)
}

/// Repeater/comparator/chest/furnace-family/hopper's own placement orientation comes from the
/// player's own horizontal look direction alone (`resolve_orientation`'s shared match arm:
/// `nearest_horizontal_direction4(yaw).opposite()`), never `clicked_face` -- `Face::North` is
/// passed as an arbitrary placeholder for every call below that does not itself vary
/// `clicked_face`. `yaw` values below are chosen so the RESULTING facing (`look.opposite()`)
/// is the direction named in each test, per `nearest_horizontal_direction4`'s own convention
/// (yaw 0 -> look South -> facing North; 180 -> look North -> facing South; 90 -> look West ->
/// facing East; 270 -> look East -> facing West).
const YAW_FACING_NORTH: f32 = 0.0;
const YAW_FACING_SOUTH: f32 = 180.0;
const YAW_FACING_EAST: f32 = 90.0;
const YAW_FACING_WEST: f32 = 270.0;

#[test]
fn repeater_ids_match_blocks_json_delay1_locked_false_powered_false() {
    // REPEATER: properties `delay`[1,2,3,4] x `facing`[north,south,west,east] x
    // `locked`[true,false] x `powered`[true,false]; base 7034 = delay=1,north,locked=true,
    // powered=true; default (facing=north) 7037 = delay=1,north,locked=false,powered=false.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Repeater,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        7037, // delay=1, facing=north, locked=false, powered=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Repeater,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        7041, // delay=1, facing=south, locked=false, powered=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Repeater,
            Face::North,
            YAW_FACING_WEST,
            0.0
        ),
        7045, // delay=1, facing=west, locked=false, powered=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Repeater,
            Face::North,
            YAW_FACING_EAST,
            0.0
        ),
        7049, // delay=1, facing=east, locked=false, powered=false
    );
}

#[test]
fn comparator_ids_match_blocks_json_mode_compare_powered_false() {
    // COMPARATOR: properties `facing`[north,south,west,east] x `mode`[compare,subtract] x
    // `powered`[true,false]; base 11263 = north,compare,powered=true; default (facing=north)
    // 11264 = north,compare,powered=false.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Comparator,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        11264, // facing=north, mode=compare, powered=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Comparator,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        11268, // facing=south, mode=compare, powered=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Comparator,
            Face::North,
            YAW_FACING_WEST,
            0.0
        ),
        11272, // facing=west, mode=compare, powered=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Comparator,
            Face::North,
            YAW_FACING_EAST,
            0.0
        ),
        11276, // facing=east, mode=compare, powered=false
    );
}

#[test]
fn chest_ids_match_blocks_json_type_single_waterlogged_false() {
    // CHEST: properties `facing`[north,south,west,east] x `type`[single,left,right] x
    // `waterlogged`[true,false]; base 3987 = north,single,waterlogged=true; default
    // (facing=north) 3988 = north,single,waterlogged=false.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Chest,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        3988, // facing=north, type=single, waterlogged=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Chest,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        3994, // facing=south, type=single, waterlogged=false
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Chest, Face::North, YAW_FACING_WEST, 0.0),
        4000, // facing=west, type=single, waterlogged=false
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Chest, Face::North, YAW_FACING_EAST, 0.0),
        4006, // facing=east, type=single, waterlogged=false
    );
}

#[test]
fn furnace_ids_match_blocks_json_lit_false() {
    // FURNACE: properties `facing`[north,south,west,east] x `lit`[true,false]; base 5327 =
    // north,lit=true; default (facing=north) 5328 = north,lit=false. Before this fix, a
    // South-facing furnace (`+1` under the old placeholder arithmetic) silently landed on
    // 5329 = north,lit=TRUE instead -- flipping `lit`, never touching `facing` at all, exactly
    // the reported "furnace randomly lit" symptom.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Furnace,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        5328, // facing=north, lit=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Furnace,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        5330, // facing=south, lit=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Furnace,
            Face::North,
            YAW_FACING_WEST,
            0.0
        ),
        5332, // facing=west, lit=false
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Furnace,
            Face::North,
            YAW_FACING_EAST,
            0.0
        ),
        5334, // facing=east, lit=false
    );
}

#[test]
fn blast_furnace_ids_match_blocks_json_lit_false() {
    // BLAST_FURNACE: identical shape to FURNACE, base 20762 = north,lit=true; default
    // (facing=north) 20763 = north,lit=false.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::BlastFurnace,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        20763,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::BlastFurnace,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        20765,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::BlastFurnace,
            Face::North,
            YAW_FACING_WEST,
            0.0
        ),
        20767,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::BlastFurnace,
            Face::North,
            YAW_FACING_EAST,
            0.0
        ),
        20769,
    );
}

#[test]
fn smoker_ids_match_blocks_json_lit_false() {
    // SMOKER: identical shape to FURNACE, base 20754 = north,lit=true; default (facing=north)
    // 20755 = north,lit=false.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Smoker,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        20755,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Smoker,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        20757,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Smoker,
            Face::North,
            YAW_FACING_WEST,
            0.0
        ),
        20759,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Smoker,
            Face::North,
            YAW_FACING_EAST,
            0.0
        ),
        20761,
    );
}

#[test]
fn wall_torch_ids_match_blocks_json_lit_true() {
    // REDSTONE_WALL_TORCH: properties `facing`[north,south,west,east] x `lit`[true,false];
    // default (facing=north) 6887 = north,lit=true. A freshly placed torch is always lit (its
    // support is not yet powered). `clicked_face` -- not yaw -- drives `resolve_orientation`'s
    // torch branch (`face_to_direction(horizontal)`). Before this fix, `+offset` used North=0/
    // South=1/East=2/West=3 with no `lit` stride at all: e.g. `Face::South` (`offset=1`)
    // produced 6888 = north,lit=FALSE -- wrong facing AND wrong (unlit) `lit`, exactly the
    // reported "torches never power anything" symptom (an unlit torch outputs no signal).
    assert_eq!(
        placed_id(PlaceableBlockKind::RedstoneTorch, Face::North, 0.0, 0.0),
        6887, // facing=north, lit=true
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::RedstoneTorch, Face::South, 0.0, 0.0),
        6889, // facing=south, lit=true
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::RedstoneTorch, Face::West, 0.0, 0.0),
        6891, // facing=west, lit=true
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::RedstoneTorch, Face::East, 0.0, 0.0),
        6893, // facing=east, lit=true
    );
}

#[test]
fn floor_torch_id_matches_blocks_json_default_lit_true() {
    // REDSTONE_TORCH (floor, `Face::Up`): single `lit`[true,false] property; default 6885 =
    // lit=true -- unaffected by Root Cause 1 (this row was never touched by the old placeholder
    // arithmetic, `Orientation::None` uses the default id directly), asserted here for
    // completeness alongside the rest of this file's own per-block coverage.
    assert_eq!(
        placed_id(PlaceableBlockKind::RedstoneTorch, Face::Up, 0.0, 0.0),
        6885,
    );
}

#[test]
fn hopper_ids_match_blocks_json_enabled_true() {
    // HOPPER: properties `enabled`[true,false] x `facing`[down,north,south,west,east]; default
    // (facing=down) 11313 = enabled=true,down. Before this fix, `HOPPER.0 + 10` (chosen only to
    // "sit safely past every direction_offset value 0..=5") landed on 11323 -- ten states past
    // the end of `hopper`'s own 10-state range (11313..=11322) entirely, inside the *next*
    // registered block's own id space (`minecraft:quartz_block`) -- exactly the reported "hopper
    // placed facing down becomes QUARTZ" symptom.
    assert_eq!(
        placed_id(PlaceableBlockKind::Hopper, Face::Up, 0.0, 0.0),
        11313, // enabled=true, facing=down (Face::Up -> clicked top -> opposite Down -> clamped Down)
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Hopper, Face::Down, 0.0, 0.0),
        11313, // enabled=true, facing=down (Face::Down -> clicked bottom -> opposite Up -> clamped Down)
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Hopper, Face::South, 0.0, 0.0),
        11314, // enabled=true, facing=north (clicked south face -> opposite North)
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Hopper, Face::North, 0.0, 0.0),
        11315, // enabled=true, facing=south (clicked north face -> opposite South)
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Hopper, Face::East, 0.0, 0.0),
        11316, // enabled=true, facing=west (clicked east face -> opposite West)
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Hopper, Face::West, 0.0, 0.0),
        11317, // enabled=true, facing=east (clicked west face -> opposite East)
    );
}

#[test]
fn piston_ids_match_blocks_json_extended_false() {
    // PISTON: properties `extended`[true,false] x `facing`[north,east,south,west,up,down]
    // (this six-value order is DISTINCT from every horizontal-only block above -- it interleaves
    // the two vertical directions rather than listing them last); base 2257 =
    // extended=true,north; default (facing=north) 2263 = extended=false,north.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Piston,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        2263, // extended=false, facing=north
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Piston,
            Face::North,
            YAW_FACING_EAST,
            0.0
        ),
        2264, // extended=false, facing=east
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Piston,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        2265, // extended=false, facing=south
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::Piston,
            Face::North,
            YAW_FACING_WEST,
            0.0
        ),
        2266, // extended=false, facing=west
    );
    // A steep downward look (`pitch=90`, `look.y < 0` per `look_vector`'s own `y = -sin(pitch)`
    // convention) resolves to `nearest_direction6`'s own `Down`, whose `.opposite()` is `Up`:
    // looking down at the floor and placing a piston there gives a floor piston that pushes
    // UPWARD (`FACING=Up`) -- looking steeply up resolves the opposite way (`FACING=Down`).
    assert_eq!(
        placed_id(PlaceableBlockKind::Piston, Face::North, 0.0, 90.0),
        2267, // extended=false, facing=up (player looked steeply down)
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::Piston, Face::North, 0.0, -90.0),
        2268, // extended=false, facing=down (player looked steeply up)
    );
}

#[test]
fn sticky_piston_ids_match_blocks_json_extended_false() {
    // STICKY_PISTON: identical shape to PISTON, base 2235 = extended=true,north; default
    // (facing=north) 2241 = extended=false,north.
    assert_eq!(
        placed_id(
            PlaceableBlockKind::StickyPiston,
            Face::North,
            YAW_FACING_NORTH,
            0.0
        ),
        2241,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::StickyPiston,
            Face::North,
            YAW_FACING_EAST,
            0.0
        ),
        2242,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::StickyPiston,
            Face::North,
            YAW_FACING_SOUTH,
            0.0
        ),
        2243,
    );
    assert_eq!(
        placed_id(
            PlaceableBlockKind::StickyPiston,
            Face::North,
            YAW_FACING_WEST,
            0.0
        ),
        2244,
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::StickyPiston, Face::North, 0.0, 90.0),
        2245, // extended=false, facing=up (player looked steeply down)
    );
    assert_eq!(
        placed_id(PlaceableBlockKind::StickyPiston, Face::North, 0.0, -90.0),
        2246, // extended=false, facing=down (player looked steeply up)
    );
}

#[test]
fn stone_and_wire_default_ids_are_unaffected_by_root_cause_1() {
    // Both of these rows were ALREADY the real generated default id before this fix
    // (`Orientation::None` uses `STONE.0`/`REDSTONE_WIRE.0` directly, no arithmetic) -- asserted
    // here only for this file's own complete per-`PlaceableBlockKind` coverage.
    assert_eq!(placed_id(PlaceableBlockKind::Stone, Face::Up, 0.0, 0.0), 1,);
    assert_eq!(
        placed_id(PlaceableBlockKind::RedstoneWire, Face::Up, 0.0, 0.0),
        5171, // east=none, north=none, power=0, south=none, west=none
    );
}
