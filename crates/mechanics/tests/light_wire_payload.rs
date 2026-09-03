//! M4-B07 — `build_update_light_payload` acceptance tests (Context §12): the exact
//! per-section bucketing algorithm -- structural, variant-only dispatch, never a scan
//! of `Data`'s own nibble content.

use rc_chunk_storage::{LightColumn, LightNibbles};
use rc_mechanics::light::build_update_light_payload;

#[test]
fn untracked_section_contributes_to_neither_mask() {
    let column = LightColumn::new_uninitialized();
    let payload = build_update_light_payload(&column);

    assert_eq!(payload.sky_light_mask, 0);
    assert_eq!(payload.empty_sky_light_mask, 0);
    assert!(payload.sky_light_arrays.is_empty());
    assert_eq!(payload.block_light_mask, 0);
    assert_eq!(payload.empty_block_light_mask, 0);
    assert!(payload.block_light_arrays.is_empty());
}

#[test]
fn filled_zero_section_goes_into_empty_mask_only() {
    let mut column = LightColumn::new_uninitialized();
    column.section_mut(0).sky = LightNibbles::Filled(0);
    let payload = build_update_light_payload(&column);

    assert_eq!(payload.sky_light_mask & 1, 0);
    assert_eq!(payload.empty_sky_light_mask & 1, 1);
    assert!(payload.sky_light_arrays.is_empty());
}

#[test]
fn filled_nonzero_section_contributes_non_empty_mask_and_materialized_array() {
    let mut column = LightColumn::new_uninitialized();
    column.section_mut(1).sky = LightNibbles::Filled(15);
    let payload = build_update_light_payload(&column);

    assert_eq!(payload.sky_light_mask & (1 << 1), 1 << 1);
    assert_eq!(payload.empty_sky_light_mask & (1 << 1), 0);
    assert_eq!(payload.sky_light_arrays.len(), 1);
    assert_eq!(payload.sky_light_arrays[0], [0xFFu8; 2048]);
}

#[test]
fn all_zero_data_section_still_contributes_non_empty_mask_and_array() {
    let mut column = LightColumn::new_uninitialized();
    column.section_mut(2).sky = LightNibbles::Data(Box::new([0u8; 2048]));
    let payload = build_update_light_payload(&column);

    assert_eq!(payload.sky_light_mask & (1 << 2), 1 << 2);
    assert_eq!(payload.empty_sky_light_mask & (1 << 2), 0);
    assert_eq!(payload.sky_light_arrays.len(), 1);
    assert_eq!(payload.sky_light_arrays[0], [0u8; 2048]);
}

#[test]
fn nonuniform_data_section_contributes_array_and_mask_bit() {
    let mut arr = [0u8; 2048];
    arr[0] = 0x0F;
    let mut column = LightColumn::new_uninitialized();
    column.section_mut(3).sky = LightNibbles::Data(Box::new(arr));
    let payload = build_update_light_payload(&column);

    assert_eq!(payload.sky_light_mask & (1 << 3), 1 << 3);
    assert_eq!(payload.empty_sky_light_mask & (1 << 3), 0);
    assert_eq!(payload.sky_light_arrays.len(), 1);
    assert_eq!(payload.sky_light_arrays[0], arr);
}

#[test]
fn arrays_appear_in_ascending_section_index_order() {
    let mut arr5 = [0u8; 2048];
    arr5[0] = 0x05;
    let mut arr2 = [0u8; 2048];
    arr2[0] = 0x02;

    let mut column = LightColumn::new_uninitialized();
    // Constructed in reverse order (5 then 2) to prove the function itself sorts by
    // index rather than merely preserving caller/insertion order.
    column.section_mut(5).sky = LightNibbles::Data(Box::new(arr5));
    column.section_mut(2).sky = LightNibbles::Data(Box::new(arr2));

    let payload = build_update_light_payload(&column);

    assert_eq!(payload.sky_light_arrays.len(), 2);
    assert_eq!(payload.sky_light_arrays[0], arr2);
    assert_eq!(payload.sky_light_arrays[1], arr5);
}
