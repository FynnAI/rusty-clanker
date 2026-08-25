//! M1-B05 — the minimal Play state: a hand-built 3x3-chunk superflat placeholder world,
//! the exact Play-entry clientbound packet sequence, keep-alive/timeout handling, and the
//! first real `rc-scheduler` region ticking at 20 TPS. See `blueprints/M1/M1-B05-play-
//! superflat.md` for the full design.

/// `pub(crate)`, not private — M1 integration fix: `net::configuration_flow`'s own
/// dimension_type inline-NBT payload (Registry Data sync) must stay byte-consistent with
/// this module's own world-height constants (`WORLD_MIN_Y`/`SECTION_COUNT`) rather than
/// duplicating them as disconnected magic numbers, so `net` needs a crate-internal path to
/// this module. Still not `pub` — external crates (this blueprint's own acceptance tests)
/// keep writing their own test-local decode mirrors instead (module doc comment, below).
pub(crate) mod chunk;
mod connection;
mod keepalive;
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
mod world;

pub use connection::{PlayerProfile, enter_play};
pub use keepalive::{DisconnectReason, KeepAliveAction, KeepAliveDriver};
pub use world::{
    HARDCODED_REGION_ID, HardcodedWorld, PLACEHOLDER_WORLDGEN_REGISTRIES, PlayerMarker,
};
