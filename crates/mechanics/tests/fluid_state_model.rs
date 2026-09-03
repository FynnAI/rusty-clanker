//! M4-B06 — the fluid blockstate<->fluidstate duality (Context §A).

use rc_chunk_storage::BlockStateId;
use rc_mechanics::fluid::state::{FluidBlockRanges, FluidKind, FluidState};

#[test]
fn legacy_level_round_trips_flowing_states() {
    for amount in 1u8..=7 {
        for falling in [true, false] {
            let state = FluidState::flowing(FluidKind::Water, amount, falling);
            // Context §A: legacy_level = (8 - amount.min(8)) + if falling {8} else {0}.
            let expected_level = (8 - amount.min(8)) + if falling { 8 } else { 0 };
            assert_eq!(
                state.to_legacy_level(),
                expected_level,
                "amount={amount} falling={falling}"
            );

            let round_tripped = FluidState::from_legacy_level(FluidKind::Water, expected_level);
            assert_eq!(
                round_tripped,
                FluidState::flowing(FluidKind::Water, amount, falling),
                "amount={amount} falling={falling} did not round-trip through level {expected_level}"
            );
        }
    }
}

#[test]
fn amount_eight_non_falling_collides_with_source() {
    // Context §A's own documented vanilla quirk: a non-source flowing state at full amount
    // (8), non-falling, encodes to legacy level 0 -- the same level a genuine source encodes
    // to -- and the inverse mapping always resolves level 0 to Source, never back to
    // Flowing{amount:8, falling:false}.
    let flowing_full = FluidState::flowing(FluidKind::Water, 8, false);
    assert_eq!(flowing_full.to_legacy_level(), 0);

    let decoded = FluidState::from_legacy_level(FluidKind::Water, 0);
    assert_eq!(decoded, FluidState::source(FluidKind::Water));
    assert_ne!(decoded, flowing_full);
}

#[test]
fn source_amount_is_hardcoded_eight() {
    assert_eq!(FluidState::source(FluidKind::Lava).amount(), 8);
    assert!(FluidState::source(FluidKind::Lava).is_source());
    assert!(!FluidState::source(FluidKind::Lava).falling());
}

#[test]
fn own_height_matches_amount_over_nine() {
    let state = FluidState::flowing(FluidKind::Water, 3, false);
    // Context §A: `f32` division, not `f64` narrowed -- bit-exact against the same `f32`
    // expression, not merely "close enough".
    assert_eq!(state.own_height(), 3.0f32 / 9.0f32);

    let source = FluidState::source(FluidKind::Lava);
    assert_eq!(source.own_height(), 8.0f32 / 9.0f32);
}

#[test]
fn ranges_reject_non_sixteen_width() {
    // Fifteen-wide (100..115) -- one short of the required 16.
    let result = FluidBlockRanges::new(
        (BlockStateId(100), BlockStateId(115)),
        (BlockStateId(300), BlockStateId(316)),
    );
    assert_eq!(result, None);

    let result = FluidBlockRanges::new(
        (BlockStateId(200), BlockStateId(216)),
        (BlockStateId(300), BlockStateId(315)),
    );
    assert_eq!(result, None);
}

#[test]
fn ranges_kind_of_and_state_of_round_trip() {
    let ranges = FluidBlockRanges::new(
        (BlockStateId(200), BlockStateId(216)),
        (BlockStateId(300), BlockStateId(316)),
    )
    .expect("both ranges are exactly 16-wide");

    assert_eq!(ranges.kind_of(BlockStateId(205)), Some(FluidKind::Water));
    assert_eq!(ranges.kind_of(BlockStateId(199)), None);
    assert_eq!(ranges.kind_of(BlockStateId(310)), Some(FluidKind::Lava));
    assert_eq!(ranges.kind_of(BlockStateId(316)), None);

    for offset in 0u32..16 {
        let id = BlockStateId(200 + offset);
        let expected = FluidState::from_legacy_level(FluidKind::Water, offset as u8);
        assert_eq!(ranges.state_of(id), Some(expected));
        assert_eq!(ranges.to_block_state_id(expected), id);
    }
    // Offset 0 -> level 0 -> Source (test 2's own documented behavior), not Flowing{amount:8,
    // falling:false}.
    assert_eq!(
        ranges.state_of(BlockStateId(200)),
        Some(FluidState::source(FluidKind::Water))
    );
}
