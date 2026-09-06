//! M4-B10 (Context §G) — every button and pressure-plate state registers an explicit empty
//! collision shape (`noCollision`, MECH-D84's own "sturdy on no face for any support kind"
//! consequence chain), never `ShapeTable::lookup`'s own `default_full_cube()` fallback.
//! Not part of TEST-D55's own `crates/{server,mechanics}/tests/` trigger scope (§2.4) --
//! no `test-matrix` header required.

use rc_physics::{Face, SupportKind, tier1_shape_table};
use rc_registries::block_state_properties::range_of;
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};

/// All 14 button and all 16 pressure-plate block ids (Context §G) -- an independent literal
/// list, mirroring `rc-mechanics::redstone::piston::DESTROY_RANGE_BLOCK_IDS`'s own identical
/// "restate the 30 ids directly" convention (this crate cannot depend on `rc-mechanics`,
/// WS-D3, so it cannot read `BUTTON_BLOCKS`/`PRESSURE_PLATE_BLOCKS` directly).
const ALL_BUTTON_AND_PLATE_BLOCK_IDS: &[BlockId] = &[
    block_id::STONE_BUTTON,
    block_id::POLISHED_BLACKSTONE_BUTTON,
    block_id::OAK_BUTTON,
    block_id::SPRUCE_BUTTON,
    block_id::BIRCH_BUTTON,
    block_id::JUNGLE_BUTTON,
    block_id::ACACIA_BUTTON,
    block_id::DARK_OAK_BUTTON,
    block_id::PALE_OAK_BUTTON,
    block_id::MANGROVE_BUTTON,
    block_id::CHERRY_BUTTON,
    block_id::BAMBOO_BUTTON,
    block_id::CRIMSON_BUTTON,
    block_id::WARPED_BUTTON,
    block_id::STONE_PRESSURE_PLATE,
    block_id::POLISHED_BLACKSTONE_PRESSURE_PLATE,
    block_id::OAK_PRESSURE_PLATE,
    block_id::SPRUCE_PRESSURE_PLATE,
    block_id::BIRCH_PRESSURE_PLATE,
    block_id::JUNGLE_PRESSURE_PLATE,
    block_id::ACACIA_PRESSURE_PLATE,
    block_id::DARK_OAK_PRESSURE_PLATE,
    block_id::PALE_OAK_PRESSURE_PLATE,
    block_id::MANGROVE_PRESSURE_PLATE,
    block_id::CHERRY_PRESSURE_PLATE,
    block_id::BAMBOO_PRESSURE_PLATE,
    block_id::CRIMSON_PRESSURE_PLATE,
    block_id::WARPED_PRESSURE_PLATE,
    block_id::LIGHT_WEIGHTED_PRESSURE_PLATE,
    block_id::HEAVY_WEIGHTED_PRESSURE_PLATE,
];

#[test]
fn every_button_and_plate_state_has_an_explicit_empty_shape_row() {
    let table = tier1_shape_table();
    for &block in ALL_BUTTON_AND_PLATE_BLOCK_IDS {
        let range = range_of(block);
        for raw in range.first.0..=range.last.0 {
            assert!(
                table.lookup(raw).shape.is_empty(),
                "block {block:?} state {raw} must have an explicit empty shape row"
            );
        }
    }
}

#[test]
fn buttons_and_plates_are_sturdy_on_no_face_for_any_support_kind() {
    let table = tier1_shape_table();
    const FACES: [Face; 6] = [
        Face::Down,
        Face::Up,
        Face::North,
        Face::South,
        Face::West,
        Face::East,
    ];
    const KINDS: [SupportKind; 3] = [SupportKind::Full, SupportKind::Center, SupportKind::Rigid];

    for &block in ALL_BUTTON_AND_PLATE_BLOCK_IDS {
        let range = range_of(block);
        // One representative state per block -- the first in its own range.
        let raw = range.first.0;
        for face in FACES {
            for kind in KINDS {
                assert!(
                    !table.is_face_sturdy(raw, face, kind),
                    "block {block:?} state {raw} must not be sturdy on {face:?} for {kind:?}"
                );
            }
        }
    }
}
