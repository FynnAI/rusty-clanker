//! `PalettedContainer::get`/`set` property tests (M2-B01 Deliverables).

use std::collections::HashMap;

use proptest::prelude::*;
use rc_chunk_storage::{BlockStateId, PalettedContainer, PaletteThresholds};

/// `PaletteThresholds::blocks(15)`'s own Direct-path bit width bound -- every generated
/// raw value stays representable at every palette strategy this container can reach.
const MAX_RAW_VALUE: u32 = 1u32 << 15;

fn writes_strategy() -> impl Strategy<Value = Vec<(usize, u32)>> {
    proptest::collection::vec((0usize..64, 0u32..MAX_RAW_VALUE), 0..=50)
}

proptest! {
    #[test]
    fn set_then_get_returns_the_written_value(
        entry_count in 4u16..=64,
        writes in writes_strategy(),
    ) {
        let thresholds = PaletteThresholds::blocks(15);
        let mut container = PalettedContainer::new_single(BlockStateId(0), entry_count, thresholds);
        for (raw_index, raw_value) in writes {
            let index = raw_index % entry_count as usize;
            container.set(index, BlockStateId(raw_value));
            prop_assert_eq!(container.get(index), BlockStateId(raw_value));
        }
    }

    #[test]
    fn every_untouched_index_keeps_the_single_value_or_its_last_written_value(
        entry_count in 4u16..=64,
        writes in writes_strategy(),
    ) {
        let thresholds = PaletteThresholds::blocks(15);
        let mut container = PalettedContainer::new_single(BlockStateId(0), entry_count, thresholds);
        let mut oracle: HashMap<usize, u32> = HashMap::new();
        for (raw_index, raw_value) in writes {
            let index = raw_index % entry_count as usize;
            container.set(index, BlockStateId(raw_value));
            oracle.insert(index, raw_value);
        }
        for index in 0..entry_count as usize {
            let expected = oracle.get(&index).copied().unwrap_or(0);
            prop_assert_eq!(container.get(index), BlockStateId(expected));
        }
    }

    #[test]
    fn bits_per_entry_never_exceeds_thresholds_or_shrinks_once_grown(
        entry_count in 4u16..=64,
        writes in writes_strategy(),
    ) {
        let thresholds = PaletteThresholds::blocks(15);
        let mut container = PalettedContainer::new_single(BlockStateId(0), entry_count, thresholds);
        let mut last_bits = container.bits_per_entry();
        for (raw_index, raw_value) in writes {
            let index = raw_index % entry_count as usize;
            container.set(index, BlockStateId(raw_value));
            let bits = container.bits_per_entry();
            prop_assert!(bits <= thresholds.direct_bits);
            prop_assert!(bits >= last_bits);
            last_bits = bits;
        }
    }
}
