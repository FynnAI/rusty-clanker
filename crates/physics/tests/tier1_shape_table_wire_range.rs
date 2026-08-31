//! M3 field-report fix (Task 2, "wire's conductor classification"): `tier1_shape_table()` must
//! register redstone wire's *entire* reachable id range as its flat, non-full shape, not only
//! the single zero-power/no-connections default id (5171) — the M3-B02/M3-B04-era registration
//! this fix replaces, which made `rc_mechanics::redstone::signal::is_conductor` wrongly resolve
//! every *other* wire id as a full-cube conductor (`docs/findings-for-planning.md`'s own "wire
//! own-state writeback attempt reverted" entry — root cause, not the writeback itself).
//!
//! Range cited directly off `datagen-output/26.2/generated/reports/blocks.json`'s own
//! `minecraft:redstone_wire` entry (protocol 776): `power` (`0..=15`) x `east`/`north`/`south`/
//! `west` (each `up`/`side`/`none`) — `4011..=5306` inclusive, contiguous, 1296 states.

use rc_physics::tier1_shape_table;

const WIRE_MIN: u32 = 4011;
const WIRE_MAX: u32 = 5306; // inclusive
const WIRE_STATE_COUNT: u32 = WIRE_MAX - WIRE_MIN + 1;

/// Wire's own real hitbox (`shapes.rs`'s own private `wire_shape()`, restated here since it is
/// not exported): a flat layer, full x/z footprint, y `0..0.0625` (1/16 block).
const WIRE_BOX_MAX_Y: f64 = 0.0625;

#[test]
fn every_reachable_wire_id_is_registered_non_full() {
    assert_eq!(WIRE_STATE_COUNT, 1296, "blocks.json's own wire state count");

    for id in WIRE_MIN..=WIRE_MAX {
        let props = tier1_shape_table().lookup(id);
        let boxes = props.shape.boxes();
        assert_eq!(boxes.len(), 1, "wire id {id} must have exactly one box");
        assert_eq!(
            boxes[0].max.y, WIRE_BOX_MAX_Y,
            "wire id {id} must keep wire's own flat 1/16-block-tall hitbox, not fall through \
             to `default_full_cube()`"
        );
    }
}

#[test]
fn ids_just_outside_the_wire_range_are_unaffected() {
    // A regression guard against an off-by-one on either boundary — one id below and one id
    // above the real range must still resolve to the registry's own default-full-cube
    // fallback (both are unregistered, ordinary — if oddly numbered — ids in this test).
    for id in [WIRE_MIN - 1, WIRE_MAX + 1] {
        let props = tier1_shape_table().lookup(id);
        assert_eq!(props.shape.boxes().len(), 1);
        assert_eq!(
            props.shape.boxes()[0].max.y,
            1.0,
            "id {id} just outside wire's own range must stay a full cube"
        );
    }
}
