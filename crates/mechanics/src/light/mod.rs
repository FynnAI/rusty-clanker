//! `rc-mechanics::light` -- the Stage-8 light engine (M4-B07): push-model BFS
//! propagator (WORLD-D7), bounded BSP round scheduling (WORLD-D9/ARCH-D16),
//! cross-region propagation (WORLD-D10), and the wire-payload builder for the
//! `Update Light`/`Level Chunk with Light` packets at protocol 776.

pub mod border;
pub mod propagator;
pub mod properties;
pub mod queue;
pub mod section_ops;
pub mod sky_source;
pub mod stage8;
#[cfg(feature = "server-systems")]
pub mod stage8_ecs;
pub mod wire;

pub use border::{apply_inbound_light_border_update, build_light_border_update};
pub use propagator::{
    LightChannel, check_node_block, check_node_sky, propagate_decrease_step,
    propagate_increase_step,
};
pub use properties::{LightProperties, LightPropertiesRegistry, direction_index, shape_occludes};
pub use queue::{
    ALL_DIRECTIONS, ChannelState, DirectionSet, LightDirtyEntry, LightDirtyQueue,
    LightPropagatorState, QueueEntry, all_except, contains, only,
};
pub use section_ops::{
    LIGHT_HEIGHT, LIGHT_MIN_Y, LIGHT_SECTION_COUNT, extract_face, extract_face_from_nibbles,
    get_nibble, inject_face, light_local_y, light_nibble_index, light_section_index_for_y,
    nibble_at, set_nibble, uniform_array, uniform_face,
};
pub use sky_source::{SkyLightSourceColumn, is_sky_edge_occluded};
pub use stage8::{LightTickReport, ParallelDispatch, run_stage8_lighting};
pub use wire::{UpdateLightPayload, build_update_light_payload};
