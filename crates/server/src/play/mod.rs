//! M1-B05 — the minimal Play state: a hand-built 11x11-chunk superflat placeholder world
//! (M1 integration fix, rounds 4-5: `chunk::PLACEHOLDER_RADIUS_CHUNKS`'s own doc comment
//! -- a real client needs every render-safe-area chunk's own full neighborhood sent to
//! render it, not merely that chunk itself; round 5 pushed the send radius to `5` so the
//! resulting 9x9 render-safe area, and the unmeshed edge one ring beyond it, both sit far
//! enough from spawn to match a real server's own edge-of-view-distance behavior),
//! the exact Play-entry clientbound packet sequence, keep-alive/timeout handling, and the
//! first real `rc-scheduler` region ticking at 20 TPS. See `blueprints/M1/M1-B05-play-
//! superflat.md` for the full design.

mod block_action;
mod chunk;
mod connection;
mod keepalive;
mod mining;
mod movement;
/// Public, not private as this blueprint's own Deliverables sketch first suggested --
/// forced deviation: this blueprint's own acceptance tests (`play_chunk_set.rs`,
/// `play_session_handoff.rs`) decode named Play packet types (`LoginPlay`,
/// `SynchronizePlayerPosition`, `LevelChunkWithLight`, ...) via `decode_one::<T>`
/// directly, which is impossible from an external integration-test crate unless this
/// module -- and therefore every type in it -- is genuinely `pub`, not merely declared
/// and re-exported piecemeal. The `chunk` module stays crate-internal (`pub(crate)`,
/// above -- not fully private any more, but still invisible outside this crate): every
/// acceptance test that needs chunk-byte assertions writes its own test-local decode
/// mirror instead (Acceptance tests' own explicit instruction, "not a production
/// deliverable").
pub mod packets;
mod persistence;
mod registry_resolvers;
mod world;

pub use block_action::{
    BlockActionKind, ChunkIndex, DebugBlockInfo, ENTITY_INTERACTION_RANGE, Face,
    PendingBlockAction, debug_query_block, resolve_place_position, seed_chunk_column,
    target_position, to_storage_biome_id, to_storage_id,
};
pub use connection::{PlayerProfile, enter_play};
pub use keepalive::{DisconnectReason, KeepAliveAction, KeepAliveDriver};
pub use mining::{
    BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER, BLOCK_INTERACTION_RANGE_CREATIVE,
    BLOCK_INTERACTION_RANGE_SURVIVAL, BreakOutcome, DestroyOutcome, DestroySpeed, DestroyState,
    DigProperties, GameModeState, HeldItem, HeldItemStub, Orientation, OrientedStateTable,
    PlaceOutcome, PlaceableBlockKind, PlacementSelection, RejectReason, StopOutcome, TickOutcome,
    ToolKind, ToolMaterial, abort_destroy, apply_placement, begin_destroy, destroy_speed,
    dig_properties, dig_properties_for_raw_state, finalize_break, has_correct_tool_for_drops,
    is_within_block_interaction_range, look_vector, nearest_direction6,
    nearest_horizontal_direction4, raycast_reach, resolve_orientation, settle_neighbor_updates,
    stop_destroy, tick_destroy_state, ticks_to_break, tier1_oriented_state_table,
};
pub use movement::{
    ChunkBlockShapeSource, MISMATCH_TOLERANCE_SQ, MovementOutcome, POSITION_CLAMP_HORIZONTAL,
    POSITION_CLAMP_VERTICAL, PendingMoveReport, PendingMovementPacket, PlayerMotion,
    SPEED_CHECK_THRESHOLD, TeleportState, clamp_position, evaluate_movement, eye_position,
    merge_move_report,
};
pub use persistence::{DEFAULT_SAVE_INTERVAL_TICKS, PlayerPersistenceConfig, PlayerSessionStore};
pub use world::{
    HARDCODED_REGION_ID, HardcodedWorld, PlayerMarker, SYNCHRONIZED_REGISTRIES, Stage4Counters,
};
