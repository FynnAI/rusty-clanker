//! test-matrix: boundaries=waived(pure geometry self-test, no world interaction) orientations=waived(pure geometry self-test, no real placement) self=waived(pure geometry self-test, no actor) composition=waived(pure geometry self-test, no block composition) nondefault-state=waived(pure geometry self-test, no block state)
//! `redstone_wire_capture::wire_slot_origin`'s own self-tests (M3.5-B03 governance
//! changeset, "redstone wire capture builds at floor level near spawn") — written
//! before the implementation changeset that adds `wire_slot_origin`/`WIRE_SLOT_A`/
//! `WIRE_SLOT_B`/`WIRE_SLOT_MARGIN_*` themselves (TEST-D45). Pure arithmetic, no
//! live server: pins the exact `index % 2` alternation `capture_contraption_over_
//! wire` depends on, and proves the two slots' own maximal bounding boxes
//! (`WIRE_SLOT_MARGIN_*`, padding the widest real corpus contraption footprint
//! ever measured — `WIRE_SLOT_A`'s own doc comment has the field-report citation)
//! never overlap each other or `protocol_session.rs`'s own scripted-session
//! placement area, and that both slots stay within a short (<= 48 block) walk of
//! spawn and of each other.

use rc_paritybot::redstone_wire_capture::{
    WIRE_SLOT_A, WIRE_SLOT_B, WIRE_SLOT_MARGIN_X_NEG, WIRE_SLOT_MARGIN_X_POS,
    WIRE_SLOT_MARGIN_Z_NEG, WIRE_SLOT_MARGIN_Z_POS, wire_slot_origin,
};

/// `protocol_session.rs::slot_for(index) = (index as i32 * 4, 0, 24)`, run once
/// per `BlockKind::ALL` kind (12 kinds, indices `0..12`) — the scripted session's
/// own real placement row. Restated here as plain constants (never imported —
/// this file pins the *real* coordinates the session uses on disk today, not
/// merely whatever this crate's own code happens to compute them as right now)
/// so a future change to either module's own geometry has to touch this file's
/// own constants too, not silently drift apart from it.
const SESSION_SLOT_COUNT: i32 = 12;
const SESSION_SLOT_SPACING: i32 = 4;
const SESSION_SLOT_Z: i32 = 24;
/// `protocol_session.rs::run_protocol_session`'s own `session/dig_stone_survival`
/// step: `dig_target = placement_capture::absolute(floor_y, (0, 0, 0))`.
const SESSION_DIG_TARGET: (i32, i32) = (0, 0);
/// `protocol_session.rs::run_protocol_session`'s own `session/move` step:
/// `move_target = placement_capture::absolute(floor_y, (3, 1, 3))`.
const SESSION_MOVE_TARGET: (i32, i32) = (3, 3);

/// `(min_x, max_x, min_z, max_z)` — every rectangle in this file lives on the
/// horizontal plane only (every real overlap concern here separates on `x`/`z`,
/// never `y` — `WIRE_SLOT_A`'s own doc comment: both slots share the identical
/// `y` convention, and the scripted session's own placement lands at the same
/// real world layer this pass's own contraption row 0 does).
type Rect = (i32, i32, i32, i32);

fn padded_rect(origin: (i32, i32, i32)) -> Rect {
    (
        origin.0 - WIRE_SLOT_MARGIN_X_NEG,
        origin.0 + WIRE_SLOT_MARGIN_X_POS,
        origin.2 - WIRE_SLOT_MARGIN_Z_NEG,
        origin.2 + WIRE_SLOT_MARGIN_Z_POS,
    )
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.0 <= b.1 && b.0 <= a.1 && a.2 <= b.3 && b.2 <= a.3
}

fn point_in_rect(point: (i32, i32), rect: Rect) -> bool {
    point.0 >= rect.0 && point.0 <= rect.1 && point.1 >= rect.2 && point.1 <= rect.3
}

fn horizontal_distance(a: (i32, i32), b: (i32, i32)) -> f64 {
    (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f64).sqrt()
}

#[test]
fn wire_slot_origin_alternates_by_index_parity() {
    for index in 0..102usize {
        let expected = if index.is_multiple_of(2) {
            WIRE_SLOT_A
        } else {
            WIRE_SLOT_B
        };
        assert_eq!(wire_slot_origin(index), expected, "index {index}");
    }
}

#[test]
fn the_two_slots_own_padded_bounding_boxes_never_overlap() {
    let a = padded_rect(WIRE_SLOT_A);
    let b = padded_rect(WIRE_SLOT_B);
    assert!(!rects_overlap(a, b), "slot A {a:?} overlaps slot B {b:?}");
}

#[test]
fn neither_slot_overlaps_the_scripted_sessions_own_placement_row() {
    let session_min_x = 0;
    let session_max_x = (SESSION_SLOT_COUNT - 1) * SESSION_SLOT_SPACING;
    let session_rect: Rect = (session_min_x, session_max_x, SESSION_SLOT_Z, SESSION_SLOT_Z);

    for slot in [WIRE_SLOT_A, WIRE_SLOT_B] {
        let rect = padded_rect(slot);
        assert!(
            !rects_overlap(rect, session_rect),
            "slot {slot:?} (padded {rect:?}) overlaps the scripted session's own \
             placement row {session_rect:?}"
        );
    }
}

#[test]
fn neither_slot_overlaps_the_scripted_sessions_own_move_or_dig_targets() {
    for slot in [WIRE_SLOT_A, WIRE_SLOT_B] {
        let rect = padded_rect(slot);
        assert!(
            !point_in_rect(SESSION_DIG_TARGET, rect),
            "slot {slot:?} (padded {rect:?}) overlaps session/dig_stone_survival's \
             own target {SESSION_DIG_TARGET:?}"
        );
        assert!(
            !point_in_rect(SESSION_MOVE_TARGET, rect),
            "slot {slot:?} (padded {rect:?}) overlaps session/move's own target \
             {SESSION_MOVE_TARGET:?}"
        );
    }
}

#[test]
fn each_slots_own_walk_distance_from_spawn_or_the_other_slot_is_within_48_blocks() {
    // A fresh flat world's own spawn point sits at (0, *, 0) — the only reading
    // under which `protocol_session.rs::run_protocol_session`'s own `session/move`
    // target, `(3, 1, 3)`, is the short real hop its own doc comment claims it is.
    let spawn = (0i32, 0i32);
    let a_xz = (WIRE_SLOT_A.0, WIRE_SLOT_A.2);
    let b_xz = (WIRE_SLOT_B.0, WIRE_SLOT_B.2);

    let spawn_to_a = horizontal_distance(spawn, a_xz);
    let spawn_to_b = horizontal_distance(spawn, b_xz);
    let a_to_b = horizontal_distance(a_xz, b_xz);

    assert!(spawn_to_a <= 48.0, "spawn to slot A: {spawn_to_a}");
    assert!(spawn_to_b <= 48.0, "spawn to slot B: {spawn_to_b}");
    assert!(a_to_b <= 48.0, "slot A to slot B: {a_to_b}");
}
