//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance per assertion, no ≥3-component chain — see redstone_wire_piston_support.rs for the cross-behavior chain) nondefault-state=yes
//! M3 field-report test-authoring (MECH-D84): `ShapeTable::is_face_sturdy`/`VoxelShape::
//! face_sturdy` — the per-face sturdiness predicate every tier-1 redstone support check now
//! reads instead of the old full-cube-conductor proxy. Pure `rc-physics`, no world/mechanics
//! involved; every state id is derived off `rc-registries`' generated per-block-state-property
//! registry (`state_id`), never a hand-derived literal — mirrors `piston_shape_table.rs`'s own
//! established convention in this same crate family.
//!
//! Extended piston/sticky_piston base shape (`Block.boxZ(16,4,16)` rotated by facing via
//! `Shapes.rotateAll`, cross-checked against this crate's own pre-existing
//! `piston_head_shape(axis, positive)` per-facing table): full on the two non-facing axes,
//! and on the facing axis `[0, 0.75]` when facing points to that axis's positive end, `[0.25,
//! 1]` otherwise — so only `facing=down`'s own top face reaches the literal `y=1` boundary at
//! all.
//!
//! Chest's own real support shape (`ChestBlock.SHAPE`, `Block.column(14,0,14)`) never reaches
//! `y=1` either, yet vanilla still lets a torch/diode attach to its top (`docs/planning/
//! 05-game-mechanics.md`'s own MECH-D84 row: "chests (torches and diodes stand on them, wire
//! does not)") — `face_sturdy`'s own doc comment has the exact `Full`-vs-`Center`/`Rigid`
//! distinction this requires.

use rc_physics::{Face, SupportKind, tier1_shape_table};
use rc_registries::block_state_properties::state_id;
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state;

const ALL_FACES: [Face; 6] = [
    Face::Up,
    Face::Down,
    Face::North,
    Face::South,
    Face::East,
    Face::West,
];
const ALL_KINDS: [SupportKind; 3] = [SupportKind::Full, SupportKind::Center, SupportKind::Rigid];
const HORIZONTAL_FACINGS: [&str; 4] = ["north", "south", "east", "west"];

fn piston_id(sticky: bool, facing: &str) -> u32 {
    let block = if sticky {
        block_id::STICKY_PISTON
    } else {
        block_id::PISTON
    };
    state_id(block, &[("extended", "true"), ("facing", facing)])
        .unwrap_or_else(|| panic!("piston_id: (extended=true, facing={facing}) must be legal"))
        .0
}

#[test]
fn extended_piston_base_is_never_a_full_cube_per_facing_orientation_case() {
    let table = tier1_shape_table();
    for facing in ["north", "south", "east", "west", "up", "down"] {
        for sticky in [false, true] {
            let props = table.lookup(piston_id(sticky, facing));
            let boxes = props.shape.boxes();
            let is_full_cube = boxes.len() == 1
                && boxes[0].min == rc_physics::Vec3::new(0.0, 0.0, 0.0)
                && boxes[0].max == rc_physics::Vec3::new(1.0, 1.0, 1.0);
            assert!(
                !is_full_cube,
                "extended base facing={facing} sticky={sticky} must not resolve to a full \
                 cube (MECH-D84 — a 4/16 slab is always missing at the facing end)"
            );
        }
    }
}

#[test]
fn extended_piston_base_top_face_full_false_for_horizontal_facings_orientation_case() {
    let table = tier1_shape_table();
    for facing in HORIZONTAL_FACINGS {
        assert!(
            !table.is_face_sturdy(piston_id(false, facing), Face::Up, SupportKind::Full),
            "extended base facing={facing}: Y stays full (not the facing axis), but the \
             TOP face's own in-plane footprint (X,Z) is truncated on whichever of those two \
             axes the facing points along, so it must not read Full"
        );
    }
}

#[test]
fn extended_piston_base_top_face_full_true_for_facing_down_nondefault_case() {
    let table = tier1_shape_table();
    assert!(
        table.is_face_sturdy(piston_id(false, "down"), Face::Up, SupportKind::Full),
        "facing=down: the missing slab sits at Y's negative end, so the top face (Y's \
         positive end) is the untouched full [0,1]x[0,1] footprint"
    );
}

#[test]
fn extended_piston_base_top_face_full_false_for_facing_up_nondefault_case() {
    let table = tier1_shape_table();
    assert!(
        !table.is_face_sturdy(piston_id(false, "up"), Face::Up, SupportKind::Full),
        "facing=up: the missing slab sits at Y's positive end, so no box reaches the literal \
         top boundary at all — nothing can attach there"
    );
}

#[test]
fn hopper_top_face_is_not_full_nondefault_case() {
    let table = tier1_shape_table();
    assert!(
        !table.is_face_sturdy(default_state::HOPPER.0, Face::Up, SupportKind::Full),
        "hopper's real rim has a hollow scooped out of its center — the top face's own \
         footprint is a ring, never the full unit square (MECH-D84: hoppers, unlike chests, \
         support nothing standing on top of them)"
    );
}

#[test]
fn chest_top_face_full_false_center_true_rigid_true_nondefault_case() {
    let table = tier1_shape_table();
    let id = state_id(block_id::CHEST, &[("facing", "north")])
        .expect("chest facing=north is legal")
        .0;
    assert!(
        !table.is_face_sturdy(id, Face::Up, SupportKind::Full),
        "chest's own hitbox stops at 14/16 — never Full-sturdy (dust does not survive on a \
         chest)"
    );
    assert!(
        table.is_face_sturdy(id, Face::Up, SupportKind::Center),
        "chest's own top footprint still covers the tiny 7/16..9/16 center square (a floor \
         torch survives on a chest)"
    );
    assert!(
        table.is_face_sturdy(id, Face::Up, SupportKind::Rigid),
        "chest's own top footprint still covers the 2/16..14/16 square (a repeater/comparator \
         survives on a chest)"
    );
}

#[test]
fn full_cube_is_sturdy_on_every_face_every_kind_composition_case() {
    let table = tier1_shape_table();
    // Any id with no explicit tier-1 entry falls through to `default_full_cube()` — 900_999 is
    // not one of this crate's own registered ids (mirrors `piston_shape_table.rs`'s own
    // identical sentinel-id convention).
    for face in ALL_FACES {
        for kind in ALL_KINDS {
            assert!(
                table.is_face_sturdy(900_999, face, kind),
                "a full cube must be sturdy on every face for every kind (face={face:?}, \
                 kind={kind:?})"
            );
        }
    }
}

#[test]
fn air_is_sturdy_on_no_face_for_no_kind_self_case() {
    let table = tier1_shape_table();
    for face in ALL_FACES {
        for kind in ALL_KINDS {
            assert!(
                !table.is_face_sturdy(default_state::AIR.0, face, kind),
                "air must never be sturdy (face={face:?}, kind={kind:?})"
            );
        }
    }
}
