use xtask::case_matrix::{
    CategoryValue, MECHANIC_TEST_PREFIXES, check_case_matrix, file_requires_case_matrix,
    find_and_parse_header,
};
use xtask::forbidden_patterns::PatternViolation;

#[test]
fn file_requires_case_matrix_matches_every_documented_prefix() {
    for prefix in MECHANIC_TEST_PREFIXES {
        let basename = format!("{prefix}foo");
        assert!(
            file_requires_case_matrix(&basename),
            "expected {basename:?} (built from prefix {prefix:?}) to be triggered"
        );
    }
}

#[test]
fn file_requires_case_matrix_rejects_every_documented_exempt_file() {
    for basename in [
        "auth_cipher",
        "play_movement_packet_roundtrip",
        "play_player_input_packet",
        "block_event_reentrant_queue",
        "random_tick_positions",
        "cross_region_border",
        "scheduled_tick_ordering",
        "direction_vanilla_ordinal",
    ] {
        assert!(
            !file_requires_case_matrix(basename),
            "expected {basename:?} to be exempt"
        );
    }
}

#[test]
fn find_and_parse_header_accepts_a_well_formed_all_yes_line() {
    let content = "//! test-matrix: boundaries=yes orientations=yes self=yes composition=yes nondefault-state=yes\n";
    let matrix = find_and_parse_header(content).expect("must parse");
    assert_eq!(matrix.boundaries, CategoryValue::Yes);
    assert_eq!(matrix.orientations, CategoryValue::Yes);
    assert_eq!(matrix.self_interaction, CategoryValue::Yes);
    assert_eq!(matrix.composition, CategoryValue::Yes);
    assert_eq!(matrix.nondefault_state, CategoryValue::Yes);
}

#[test]
fn find_and_parse_header_accepts_a_waived_reason_containing_spaces_and_punctuation() {
    let content = "//! test-matrix: boundaries=waived(pure lookup table, no Y-coordinate — see foo.rs) orientations=yes self=yes composition=yes nondefault-state=yes\n";
    let matrix = find_and_parse_header(content).expect("must parse");
    assert_eq!(
        matrix.boundaries,
        CategoryValue::Waived("pure lookup table, no Y-coordinate — see foo.rs".to_string())
    );
}

#[test]
fn find_and_parse_header_rejects_missing_line() {
    let content = "//! just a normal doc comment, no header here\n";
    assert!(find_and_parse_header(content).is_err());
}

#[test]
fn find_and_parse_header_rejects_out_of_order_keys() {
    // self before orientations.
    let content = "//! test-matrix: boundaries=yes self=yes orientations=yes composition=yes nondefault-state=yes\n";
    assert!(find_and_parse_header(content).is_err());
}

#[test]
fn find_and_parse_header_rejects_a_value_that_is_neither_yes_nor_waived() {
    let content = "//! test-matrix: boundaries=maybe orientations=yes self=yes composition=yes nondefault-state=yes\n";
    assert!(find_and_parse_header(content).is_err());
}

#[test]
fn check_case_matrix_flags_missing_header_on_a_triggered_file() {
    let content = "//! just a normal doc comment\n#[test]\nfn plain_case() {}\n";
    let violations = check_case_matrix("mining_foo.rs", content);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0],
        PatternViolation::MissingCaseMatrixHeader { .. }
    ));
}

#[test]
fn check_case_matrix_passes_a_fully_waived_header_with_no_backing_tests_required() {
    let content = "//! test-matrix: boundaries=waived(r1) orientations=waived(r2) self=waived(r3) composition=waived(r4) nondefault-state=waived(r5)\n";
    let violations = check_case_matrix("mining_foo.rs", content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_case_matrix_flags_an_unbacked_yes_category() {
    let content = "//! test-matrix: boundaries=waived(r1) orientations=yes self=waived(r3) composition=waived(r4) nondefault-state=waived(r5)\n#[test]\nfn plain_case() {}\n";
    let violations = check_case_matrix("mining_foo.rs", content);
    assert_eq!(violations.len(), 1);
    match &violations[0] {
        PatternViolation::CaseMatrixCategoryUnbacked { category, .. } => {
            assert_eq!(category, "orientations");
        }
        other => panic!("expected CaseMatrixCategoryUnbacked, got {other:?}"),
    }
}

#[test]
fn check_case_matrix_accepts_a_yes_category_backed_by_the_facing_alias() {
    let content = "//! test-matrix: boundaries=waived(r1) orientations=yes self=waived(r3) composition=waived(r4) nondefault-state=waived(r5)\n#[test]\nfn checks_wall_torch_facing_case() {}\n";
    let violations = check_case_matrix("mining_foo.rs", content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_case_matrix_accepts_a_yes_category_backed_by_the_chain_alias() {
    let content = "//! test-matrix: boundaries=waived(r1) orientations=waived(r2) self=waived(r3) composition=yes nondefault-state=waived(r5)\n#[test]\nfn repeater_chain_relays_signal_end_to_end() {}\n";
    let violations = check_case_matrix("mining_foo.rs", content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_case_matrix_is_silent_on_a_file_that_does_not_match_the_trigger() {
    let content = "// no header at all, this file is exempt\nfn helper() {}\n";
    let violations = check_case_matrix("auth_cipher.rs", content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_case_matrix_flags_two_candidate_header_lines_as_ambiguous() {
    let content = "//! test-matrix: boundaries=yes orientations=yes self=yes composition=yes nondefault-state=yes\n//! test-matrix: boundaries=yes orientations=yes self=yes composition=yes nondefault-state=yes\n";
    let violations = check_case_matrix("mining_foo.rs", content);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0],
        PatternViolation::MissingCaseMatrixHeader { .. }
    ));
}
