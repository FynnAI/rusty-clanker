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
//! `y=1` (its own hitbox tops out at 14/16), so its top face shape is empty and vanilla lets
//! nothing rest, stand, or attach there at all — `face_sturdy`'s own doc comment has the exact
//! algorithm this falls out of.
//!
//! Hopper's own real rim (`HopperBlock`'s six-box outline minus the hollow scooped out of its
//! own top, `crates/physics/src/shapes.rs`'s own `hopper_shape` doc comment has the full box
//! citation) touches `y=1` only along its outer 2px border — exactly `Rigid`'s own required
//! frame, and nothing more — so a hopper's top face reads `Rigid`-sturdy but neither `Full`-
//! nor `Center`-sturdy.
//!
//! `redstone_wire`/`redstone_torch`/`redstone_wall_torch`/`lever` (M3 field-report test-
//! authoring, M4-B10 blueprint author's finding, re-verified against the ASSET-D18(f)
//! reference): all four register `.noCollision()` in `Blocks.java`, and `BlockBehaviour.
//! getCollisionShape` is `this.hasCollision ? state.getShape(...) : Shapes.empty()` --
//! `hasCollision = false` for every one of them, so the COLLISION shape (what `getBlockSupport
//! Shape`'s own default body reads, and therefore what `is_face_sturdy` must read) is `Shapes.
//! empty()` regardless of each block's own non-empty visual OUTLINE (`getShape`, used only for
//! rendering/selection, never for support): a flat box for wire, a centered post for both torch
//! variants, a handle box for every one of the lever's 24 states. `tier1_shape_table()`'s own
//! table must therefore resolve every reachable id in each of these four ranges to `VoxelShape::
//! empty()` -- not merely "non-full" the way the extended-piston-base/chest/hopper cases above
//! already are, but genuinely empty, so `is_face_sturdy` returns `false` on every face for every
//! `SupportKind`, with no exception.

use rc_physics::{Face, SupportKind, tier1_shape_table};
use rc_registries::block_state_properties::{range_of, state_id};
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
fn extended_piston_base_top_face_full_false_center_true_rigid_false_for_horizontal_facings_orientation_case()
 {
    let table = tier1_shape_table();
    for facing in HORIZONTAL_FACINGS {
        let id = piston_id(false, facing);
        assert!(
            !table.is_face_sturdy(id, Face::Up, SupportKind::Full),
            "extended base facing={facing}: Y stays full (not the facing axis), but the \
             TOP face's own in-plane footprint (X,Z) is truncated on whichever of those two \
             axes the facing points along, so it must not read Full"
        );
        assert!(
            table.is_face_sturdy(id, Face::Up, SupportKind::Center),
            "extended base facing={facing}: the truncated footprint still spans past 0.75 or \
             from 0.25, either way covering the centred 7/16..9/16 square — Center-sturdy"
        );
        assert!(
            !table.is_face_sturdy(id, Face::Up, SupportKind::Rigid),
            "extended base facing={facing}: the truncated footprint never reaches the far \
             14/16..1 (or 0..2/16) strip of Rigid's own required outer frame — not Rigid-sturdy"
        );
    }
}

#[test]
fn extended_piston_base_top_face_full_center_rigid_all_true_for_facing_down_nondefault_case() {
    let table = tier1_shape_table();
    let id = piston_id(false, "down");
    for kind in ALL_KINDS {
        assert!(
            table.is_face_sturdy(id, Face::Up, kind),
            "facing=down: the missing slab sits at Y's negative end, so the top face (Y's \
             positive end) is the untouched full [0,1]x[0,1] footprint — sturdy for every kind \
             (kind={kind:?})"
        );
    }
}

#[test]
fn extended_piston_base_top_face_full_center_rigid_all_false_for_facing_up_nondefault_case() {
    let table = tier1_shape_table();
    let id = piston_id(false, "up");
    for kind in ALL_KINDS {
        assert!(
            !table.is_face_sturdy(id, Face::Up, kind),
            "facing=up: the missing slab sits at Y's positive end, so no box reaches the \
             literal top boundary at all — the face shape is empty, so nothing can attach \
             there for any kind (kind={kind:?})"
        );
    }
}

#[test]
fn retracted_piston_base_top_face_full_center_rigid_all_true_composition_case() {
    let table = tier1_shape_table();
    for sticky in [false, true] {
        let block = if sticky {
            block_id::STICKY_PISTON
        } else {
            block_id::PISTON
        };
        let id = state_id(block, &[("extended", "false"), ("facing", "north")])
            .unwrap_or_else(|| panic!("retracted base sticky={sticky} must be legal"))
            .0;
        for kind in ALL_KINDS {
            assert!(
                table.is_face_sturdy(id, Face::Up, kind),
                "a retracted base's own shape is a plain full cube — sturdy for every kind \
                 (sticky={sticky}, kind={kind:?})"
            );
        }
    }
}

#[test]
fn hopper_top_face_full_false_center_false_rigid_true_nondefault_case() {
    let table = tier1_shape_table();
    let id = default_state::HOPPER.0;
    assert!(
        !table.is_face_sturdy(id, Face::Up, SupportKind::Full),
        "hopper's real rim has a hollow scooped out of its center — the top face's own \
         footprint is an outer border frame, never the full unit square"
    );
    assert!(
        !table.is_face_sturdy(id, Face::Up, SupportKind::Center),
        "the hollow scooped out of the rim's own top swallows the tiny 7/16..9/16 center \
         square whole — a hopper's top is never Center-sturdy (unlike a chest's)"
    );
    assert!(
        table.is_face_sturdy(id, Face::Up, SupportKind::Rigid),
        "the rim's own outer border exactly matches Rigid's required outer frame (everything \
         outside 2/16..14/16) — a repeater/comparator survives on a hopper"
    );
}

#[test]
fn chest_top_face_full_center_rigid_all_false_nondefault_case() {
    let table = tier1_shape_table();
    let id = state_id(block_id::CHEST, &[("facing", "north")])
        .expect("chest facing=north is legal")
        .0;
    for kind in ALL_KINDS {
        assert!(
            !table.is_face_sturdy(id, Face::Up, kind),
            "chest's own hitbox stops at 14/16 — its top face never reaches the literal y=1 \
             boundary at all, so the face shape is empty and nothing can rest, stand, or \
             attach on a chest for any kind (kind={kind:?})"
        );
    }
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

fn lever_id(face: &str, facing: &str, powered: bool) -> u32 {
    state_id(
        block_id::LEVER,
        &[
            ("face", face),
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
    .unwrap_or_else(|| panic!("lever face={face} facing={facing} powered={powered} must be legal"))
    .0
}

/// M3 field-report test-authoring (M4-B10 blueprint author's finding): `redstone_wire` registers
/// `.noCollision()` (`Blocks.java`) -- every reachable id (the full `power`x`east`x`north`x
/// `south`x`west` cross-product, not merely the default) must resolve to `VoxelShape::empty()`,
/// never the flat outline box `crates/physics/src/shapes.rs`'s own pre-fix table stored.
#[test]
fn redstone_wire_every_state_has_empty_collision_shape_orientation_case() {
    let table = tier1_shape_table();
    let range = range_of(block_id::REDSTONE_WIRE);
    for id in range.first.0..=range.last.0 {
        assert!(
            table.lookup(id).shape.is_empty(),
            "redstone_wire id={id}: noCollision() in the reference means Shapes.empty(), \
             regardless of the outline's own flat 1/16-tall box"
        );
        for face in ALL_FACES {
            for kind in ALL_KINDS {
                assert!(
                    !table.is_face_sturdy(id, face, kind),
                    "redstone_wire id={id} face={face:?} kind={kind:?}: an empty collision \
                     shape is never sturdy on any face"
                );
            }
        }
    }
}

/// As above, for `redstone_torch` (floor) and `redstone_wall_torch` -- both register
/// `.noCollision()` and both share this table's own `torch_shape()` outline (the centered post),
/// but the COLLISION shape both must resolve to is empty, not that post.
#[test]
fn redstone_torch_and_wall_torch_every_state_has_empty_collision_shape_orientation_case() {
    let table = tier1_shape_table();
    for block in [block_id::REDSTONE_TORCH, block_id::REDSTONE_WALL_TORCH] {
        let range = range_of(block);
        for id in range.first.0..=range.last.0 {
            assert!(
                table.lookup(id).shape.is_empty(),
                "{block:?} id={id}: noCollision() in the reference means Shapes.empty(), \
                 regardless of the outline's own centered post box"
            );
            for face in ALL_FACES {
                for kind in ALL_KINDS {
                    assert!(
                        !table.is_face_sturdy(id, face, kind),
                        "{block:?} id={id} face={face:?} kind={kind:?}: an empty collision \
                         shape is never sturdy on any face"
                    );
                }
            }
        }
    }
}

/// As above, for every one of `lever`'s real 24 states (3 `face` x 4 `facing` x 2 `powered`) --
/// `LEVER` also registers `.noCollision()`. This is also the direct proof of the M4-B10 defect
/// itself: a CEILING lever's own outline box touches the literal `y=1` boundary with a footprint
/// that covers the centred 2x2-pixel square, so the pre-fix table (storing that outline as the
/// looked-up shape) answered `is_face_sturdy(ceiling_lever_id, Face::Up, SupportKind::Center)
/// == true` -- exactly the "a face-attached block can find a Center-sturdy face on a lever...
/// where vanilla finds none" consequence the defect names. The fix makes every one of these 24
/// ids resolve to `VoxelShape::empty()`, so every assertion below (including the `Face::Up`/
/// `SupportKind::Center` case for `face=ceiling`) must read `false`.
#[test]
fn lever_every_state_has_empty_collision_shape_and_no_sturdy_face_orientation_case() {
    let table = tier1_shape_table();
    for face in ["floor", "wall", "ceiling"] {
        for facing in ["north", "south", "west", "east"] {
            for powered in [false, true] {
                let id = lever_id(face, facing, powered);
                assert!(
                    table.lookup(id).shape.is_empty(),
                    "lever face={face} facing={facing} powered={powered} id={id}: \
                     noCollision() in the reference means Shapes.empty(), regardless of the \
                     outline's own handle box"
                );
                for f in ALL_FACES {
                    for kind in ALL_KINDS {
                        assert!(
                            !table.is_face_sturdy(id, f, kind),
                            "lever face={face} facing={facing} powered={powered} id={id} \
                             checked-face={f:?} kind={kind:?}: an empty collision shape is \
                             never sturdy on any face"
                        );
                    }
                }
            }
        }
    }
}
