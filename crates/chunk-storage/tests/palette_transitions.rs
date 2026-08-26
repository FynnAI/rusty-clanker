//! Every `PalettedContainer` strategy-boundary crossing (M2-B01 Deliverables, `palette.rs`).

use rc_chunk_storage::{BiomeId, BlockStateId, Palette, PalettedContainer, PaletteThresholds};

/// `Indirect` covers `2..=4` distinct values at `1..=2` bits; `Direct` triggers at the
/// 5th distinct value.
fn small_thresholds() -> PaletteThresholds {
    PaletteThresholds {
        indirect_floor_bits: 1,
        max_indirect_bits: 2,
        direct_bits: 4,
    }
}

#[test]
fn starts_single_value() {
    let container = PalettedContainer::new_single(BlockStateId(5), 8, small_thresholds());
    assert!(matches!(
        container.palette(),
        Palette::SingleValue(BlockStateId(5))
    ));
    assert_eq!(container.bits_per_entry(), 0);
    assert!(container.raw_words().is_empty());
    for i in 0..8 {
        assert_eq!(container.get(i), BlockStateId(5));
    }
}

#[test]
fn single_value_set_same_value_is_a_noop_that_stays_single_value() {
    let mut container = PalettedContainer::new_single(BlockStateId(5), 8, small_thresholds());
    assert!(!container.set(3, BlockStateId(5)));
    assert!(matches!(
        container.palette(),
        Palette::SingleValue(BlockStateId(5))
    ));
}

#[test]
fn single_value_to_indirect_at_floor_bits() {
    let mut container = PalettedContainer::new_single(BlockStateId(5), 8, small_thresholds());
    assert!(container.set(3, BlockStateId(7)));
    match container.palette() {
        Palette::Indirect {
            entries,
            bits_per_entry,
        } => {
            assert_eq!(entries, &vec![BlockStateId(5), BlockStateId(7)]);
            assert_eq!(*bits_per_entry, 1);
        }
        other => panic!("expected Indirect, got {other:?}"),
    }
    assert_eq!(container.get(3), BlockStateId(7));
    for i in [0, 1, 2, 4, 5, 6, 7] {
        assert_eq!(container.get(i), BlockStateId(5));
    }
}

#[test]
fn indirect_grows_bit_width_within_itself() {
    let mut container = PalettedContainer::new_single(BlockStateId(5), 8, small_thresholds());
    assert!(container.set(3, BlockStateId(7)));

    assert!(container.set(4, BlockStateId(9)));
    match container.palette() {
        Palette::Indirect {
            entries,
            bits_per_entry,
        } => {
            assert_eq!(entries.last(), Some(&BlockStateId(9)));
            assert_eq!(*bits_per_entry, 2);
        }
        other => panic!("expected Indirect, got {other:?}"),
    }
    assert_eq!(container.get(3), BlockStateId(7));
    assert_eq!(container.get(4), BlockStateId(9));
    for i in [0, 1, 2, 5, 6, 7] {
        assert_eq!(container.get(i), BlockStateId(5));
    }
}

#[test]
fn indirect_promotes_to_direct_past_max_indirect_bits() {
    let mut container = PalettedContainer::new_single(BlockStateId(5), 8, small_thresholds());
    assert!(container.set(3, BlockStateId(7)));
    assert!(container.set(4, BlockStateId(9)));

    assert!(container.set(5, BlockStateId(2)));
    assert!(matches!(container.palette(), Palette::Indirect { .. }));

    assert!(container.set(6, BlockStateId(11)));
    assert!(matches!(
        container.palette(),
        Palette::Direct { bits_per_entry: 4 }
    ));

    let expected = [
        BlockStateId(5),
        BlockStateId(5),
        BlockStateId(5),
        BlockStateId(7),
        BlockStateId(9),
        BlockStateId(2),
        BlockStateId(11),
        BlockStateId(5),
    ];
    for (i, expected_value) in expected.into_iter().enumerate() {
        assert_eq!(container.get(i), expected_value, "index {i}");
    }
}

#[test]
fn direct_set_never_changes_palette_shape() {
    let mut container = PalettedContainer::new_single(BlockStateId(5), 8, small_thresholds());
    container.set(3, BlockStateId(7));
    container.set(4, BlockStateId(9));
    container.set(5, BlockStateId(2));
    container.set(6, BlockStateId(11));
    assert!(matches!(
        container.palette(),
        Palette::Direct { bits_per_entry: 4 }
    ));

    // NB: `small_thresholds().direct_bits == 4`, so a raw value written through this
    // Direct container must itself fit in 4 bits (`0..=15`) to round-trip -- `6` (an
    // unused-so-far distinct value) exercises the same "Direct never changes shape"
    // property the blueprint's own `BlockStateId(99)` literal intended, without
    // exceeding the fixture's own 4-bit direct width (`99` needs 7 bits and would
    // truncate on write, which is a fixture-value defect, not an implementation one).
    container.set(0, BlockStateId(6));
    assert!(matches!(
        container.palette(),
        Palette::Direct { bits_per_entry: 4 }
    ));
    assert_eq!(container.get(0), BlockStateId(6));
}

#[test]
fn single_value_can_jump_straight_to_direct() {
    // Deliberately degenerate: the floor already exceeds the ceiling, so the 2nd
    // distinct value (`bits = max(9, 1) = 9 > max_indirect_bits(8)`) forces a direct
    // promotion, skipping `Indirect` entirely.
    let thresholds = PaletteThresholds {
        indirect_floor_bits: 9,
        max_indirect_bits: 8,
        direct_bits: 10,
    };
    let mut container = PalettedContainer::new_single(BlockStateId(0), 4, thresholds);
    assert!(container.set(0, BlockStateId(1)));
    assert!(matches!(
        container.palette(),
        Palette::Direct { bits_per_entry: 10 }
    ));
    assert_eq!(container.get(0), BlockStateId(1));
    for i in 1..4 {
        assert_eq!(container.get(i), BlockStateId(0));
    }
}

#[test]
fn real_block_and_biome_thresholds_reach_indirect_and_stay_there_for_typical_content() {
    let mut blocks =
        PalettedContainer::new_single(BlockStateId(0), 4096, PaletteThresholds::blocks(15));
    // Air (the seed) plus three more distinct values -- 4 distinct total, mirroring
    // M1-B05's own superflat section (air/bedrock/dirt/grass).
    blocks.set(0, BlockStateId(1));
    blocks.set(1, BlockStateId(2));
    blocks.set(2, BlockStateId(3));
    match blocks.palette() {
        Palette::Indirect {
            entries,
            bits_per_entry,
        } => {
            assert_eq!(*bits_per_entry, 4);
            assert_eq!(entries.len(), 4);
        }
        other => panic!("expected Indirect, got {other:?}"),
    }

    let biomes = PalettedContainer::new_single(BiomeId(0), 64, PaletteThresholds::biomes(4));
    assert!(matches!(biomes.palette(), Palette::SingleValue(_)));
}
