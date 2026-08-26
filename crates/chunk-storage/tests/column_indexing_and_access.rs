//! Column indexing formulas and `BlockStateColumn`/`BiomeColumn` get/set access
//! (M2-B01 Deliverables, `column.rs`).

use rc_chunk_storage::{
    biome_index, block_index, local_block_y, section_index_for_y, BiomeColumn, BiomeId,
    BlockStateColumn, BlockStateId, Palette, PaletteThresholds,
};

#[test]
fn block_index_matches_vanilla_axis_order() {
    assert_eq!(block_index(0, 0, 0), 0);
    assert_eq!(block_index(1, 0, 0), 1);
    assert_eq!(block_index(0, 0, 1), 16);
    assert_eq!(block_index(0, 1, 0), 256);
    assert_eq!(block_index(15, 15, 15), 4095);
}

#[test]
fn biome_index_matches_vanilla_axis_order_at_quart_resolution() {
    assert_eq!(biome_index(0, 0, 0), 0);
    assert_eq!(biome_index(1, 0, 0), 1);
    assert_eq!(biome_index(0, 0, 1), 4);
    assert_eq!(biome_index(0, 1, 0), 16);
    assert_eq!(biome_index(3, 3, 3), 63);
}

#[test]
fn section_index_for_y_boundaries() {
    assert_eq!(section_index_for_y(-64), 0);
    assert_eq!(section_index_for_y(-49), 0);
    assert_eq!(section_index_for_y(-48), 1);
    assert_eq!(section_index_for_y(319), 23);
}

#[test]
#[should_panic]
fn section_index_for_y_panics_above_range() {
    section_index_for_y(320);
}

#[test]
#[should_panic]
fn section_index_for_y_panics_below_range() {
    section_index_for_y(-65);
}

#[test]
fn local_block_y_wraps_per_section() {
    assert_eq!(local_block_y(-64), 0);
    assert_eq!(local_block_y(-49), 15);
    assert_eq!(local_block_y(-48), 0);
    assert_eq!(local_block_y(319), 15);
}

fn assert_indirect_with_two_entries<T: std::fmt::Debug>(palette: &Palette<T>) {
    match palette {
        Palette::Indirect { entries, .. } => assert_eq!(entries.len(), 2),
        other => panic!("expected Indirect with 2 entries, got {other:?}"),
    }
}

#[test]
fn block_state_column_get_set_across_section_boundary() {
    let mut column = BlockStateColumn::new(BlockStateId(0), PaletteThresholds::blocks(15));
    assert!(column.set(5, -49, 8, BlockStateId(42))); // last Y of section 0
    assert!(column.set(5, -48, 8, BlockStateId(43))); // first Y of section 1
    assert_eq!(column.get(5, -49, 8), BlockStateId(42));
    assert_eq!(column.get(5, -48, 8), BlockStateId(43));

    assert_indirect_with_two_entries(column.section(0).palette());
    assert_indirect_with_two_entries(column.section(1).palette());
}

#[test]
fn biome_column_get_set_across_section_boundary() {
    let mut column = BiomeColumn::new(BiomeId(0), PaletteThresholds::biomes(4));
    assert!(column.set(1, -49, 2, BiomeId(1))); // last quart-Y of section 0
    assert!(column.set(1, -48, 2, BiomeId(2))); // first quart-Y of section 1
    assert_eq!(column.get(1, -49, 2), BiomeId(1));
    assert_eq!(column.get(1, -48, 2), BiomeId(2));

    assert_indirect_with_two_entries(column.section(0).palette());
    assert_indirect_with_two_entries(column.section(1).palette());
}

#[test]
fn block_state_column_set_same_value_returns_false() {
    let mut column = BlockStateColumn::new(BlockStateId(0), PaletteThresholds::blocks(15));
    assert!(!column.set(0, -64, 0, BlockStateId(0)));
}
