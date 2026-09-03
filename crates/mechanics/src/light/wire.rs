//! The pure, protocol-crate-decoupled payload builder for the `Update Light`/
//! `Level Chunk with Light` wire fields at protocol 776 (M4-B07 Context §12).

use rc_chunk_storage::{LightColumn, LightNibbles};

use crate::light::section_ops::uniform_array;

/// The six wire-relevant fields, computed as plain data.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UpdateLightPayload {
    pub sky_light_mask: u32,
    pub block_light_mask: u32,
    pub empty_sky_light_mask: u32,
    pub empty_block_light_mask: u32,
    pub sky_light_arrays: Vec<[u8; 2048]>,
    pub block_light_arrays: Vec<[u8; 2048]>,
}

/// Context §12's exact per-section bucketing algorithm -- a structural, variant-only
/// dispatch, never a scan of `Data`'s own nibble content.
pub fn build_update_light_payload(column: &LightColumn) -> UpdateLightPayload {
    let mut payload = UpdateLightPayload::default();

    for (index, section) in column.sections().iter().enumerate() {
        let bit = 1u32 << index;
        match &section.sky {
            LightNibbles::Uninitialized => {}
            LightNibbles::Filled(0) => {
                payload.empty_sky_light_mask |= bit;
            }
            LightNibbles::Filled(v) => {
                payload.sky_light_mask |= bit;
                payload.sky_light_arrays.push(uniform_array(*v));
            }
            LightNibbles::Data(arr) => {
                payload.sky_light_mask |= bit;
                payload.sky_light_arrays.push(**arr);
            }
        }
        match &section.block {
            LightNibbles::Uninitialized => {}
            LightNibbles::Filled(0) => {
                payload.empty_block_light_mask |= bit;
            }
            LightNibbles::Filled(v) => {
                payload.block_light_mask |= bit;
                payload.block_light_arrays.push(uniform_array(*v));
            }
            LightNibbles::Data(arr) => {
                payload.block_light_mask |= bit;
                payload.block_light_arrays.push(**arr);
            }
        }
    }

    payload
}
