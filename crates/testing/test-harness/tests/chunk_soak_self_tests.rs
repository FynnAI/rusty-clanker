//! `chunk_soak`'s own self-tests (Acceptance tests) — proving `run_soak`'s analysis is
//! correct against a fake before it is ever trusted against a real `AnvilDiskBackend`.

use rc_test_harness::chunk_soak::{
    PaletteShape, generate_chunk_payload, palette_shape_for, run_soak,
};
use rc_test_harness::fixtures::corrupting_backend::{CorruptingBackend, InMemoryHonestBackend};

#[test]
fn honest_backend_reports_zero_mismatches() {
    let report = run_soak(&InMemoryHonestBackend::new(), 42, 30);
    assert!(
        report.zero_mismatches(),
        "unexpected mismatches: {:?}",
        report.mismatches
    );
    assert_eq!(report.total, 30);
}

#[test]
fn corrupting_backend_is_caught_by_the_checksum_leg() {
    let report = run_soak(&CorruptingBackend::new(), 42, 30);
    assert!(!report.zero_mismatches());
    assert_eq!(report.mismatches.len(), 30);
    for outcome in &report.mismatches {
        assert!(!outcome.round_trip_identical);
    }
}

#[test]
fn same_seed_generates_identical_payloads_across_two_calls() {
    assert_eq!(generate_chunk_payload(7, 3), generate_chunk_payload(7, 3));
}

#[test]
fn different_indices_generate_different_payloads() {
    assert_ne!(generate_chunk_payload(7, 3), generate_chunk_payload(7, 4));
}

#[test]
fn palette_shape_cycles_across_three_indices() {
    assert_eq!(palette_shape_for(0), PaletteShape::SingleValue);
    assert_eq!(palette_shape_for(1), PaletteShape::Indirect);
    assert_eq!(palette_shape_for(2), PaletteShape::Direct);
    assert_eq!(palette_shape_for(3), PaletteShape::SingleValue);
}
