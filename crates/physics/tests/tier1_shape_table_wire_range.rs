//! M3 field-report fix (Task 2, "wire's conductor classification"): `tier1_shape_table()` must
//! register redstone wire's *entire* reachable id range as its own real shape, not only the
//! single zero-power/no-connections default id (5171) — the M3-B02/M3-B04-era registration this
//! fix replaces, which made `rc_mechanics::redstone::signal::is_conductor` wrongly resolve every
//! *other* wire id as a full-cube conductor (`docs/findings-for-planning.md`'s own "wire
//! own-state writeback attempt reverted" entry — root cause, not the writeback itself).
//!
//! Range cited directly off `datagen-output/26.2/generated/reports/blocks.json`'s own
//! `minecraft:redstone_wire` entry (protocol 776): `power` (`0..=15`) x `east`/`north`/`south`/
//! `west` (each `up`/`side`/`none`) — `4011..=5306` inclusive, contiguous, 1296 states.
//!
//! M4-B10 blueprint author's finding (M3 field-report correction, re-verified against the
//! ASSET-D18(f) reference): this table stores the block's real COLLISION shape (the shape every
//! consumer -- entity collision, `is_conductor`, `is_face_sturdy`, placement obstruction --
//! actually reads), never its visual OUTLINE. `redstone_wire` registers `.noCollision()`
//! (`Blocks.java`), and `BlockBehaviour.getCollisionShape` is `this.hasCollision ?
//! state.getShape(...) : Shapes.empty()` -- `hasCollision = false` unconditionally, so wire's
//! own real collision shape is `Shapes.empty()`, not the flat, full-footprint 1/16-tall slab its
//! own `RedStoneWireBlock.getShape` override still returns for rendering/selection only. This
//! file's own former "flat, non-full hitbox" claim (`WIRE_BOX_MAX_Y = 0.0625` below) restated
//! that outline as if it were the collision shape this table must hold -- corrected here to
//! assert emptiness instead, the exhaustive-range-registration property (never falling through
//! to `default_full_cube()`) unchanged.

use rc_physics::tier1_shape_table;

const WIRE_MIN: u32 = 4011;
const WIRE_MAX: u32 = 5306; // inclusive
const WIRE_STATE_COUNT: u32 = WIRE_MAX - WIRE_MIN + 1;

#[test]
fn every_reachable_wire_id_is_registered_empty() {
    assert_eq!(WIRE_STATE_COUNT, 1296, "blocks.json's own wire state count");

    for id in WIRE_MIN..=WIRE_MAX {
        let props = tier1_shape_table().lookup(id);
        // source: Blocks.java (REDSTONE_WIRE registers `.noCollision()`) + BlockBehaviour's own
        // `getCollisionShape` default body (`hasCollision ? getShape(...) : Shapes.empty()`).
        assert!(
            props.shape.is_empty(),
            "wire id {id} must resolve to Shapes.empty() (its own real collision shape), not \
             fall through to `default_full_cube()` and not keep the outline's own flat \
             1/16-block-tall hitbox either"
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
