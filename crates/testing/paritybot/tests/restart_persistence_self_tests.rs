//! `restart_persistence`'s own pure-function self-tests (Acceptance tests) — no
//! network/process involved; `apply_actions`/`observe_state` are exercised only by a
//! real `m2-report` run (Context).

use rc_paritybot::restart_persistence::{compare_state, expected_state};

#[test]
fn matching_state_produces_no_mismatches() {
    assert!(compare_state(&expected_state(), &expected_state()).is_empty());
}

#[test]
fn wrong_block_state_is_reported() {
    let mut actual = expected_state();
    actual.blocks[0].1 = actual.blocks[0].1.wrapping_add(1);
    let mismatches = compare_state(&expected_state(), &actual);
    assert_eq!(mismatches.len(), 1, "got {mismatches:?}");
}

#[test]
fn wrong_health_is_reported() {
    let mut actual = expected_state();
    actual.health = 19.0;
    let mismatches = compare_state(&expected_state(), &actual);
    assert_eq!(mismatches.len(), 1, "got {mismatches:?}");
    assert!(mismatches[0].contains("health"));
}

#[test]
fn multiple_mismatches_are_all_reported_independently() {
    let mut actual = expected_state();
    actual.blocks[0].1 = actual.blocks[0].1.wrapping_add(1);
    actual.blocks[2].1 = actual.blocks[2].1.wrapping_add(1);
    let mismatches = compare_state(&expected_state(), &actual);
    assert_eq!(mismatches.len(), 2, "got {mismatches:?}");
}
