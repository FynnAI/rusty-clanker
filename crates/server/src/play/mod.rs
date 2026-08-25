//! M1-B05 — the minimal Play state: a hand-built 5x5-chunk superflat placeholder world
//! (M1 integration fix, round 4: `chunk::PLACEHOLDER_RADIUS_CHUNKS`'s own doc comment --
//! a real client needs every originally-visible 3x3-area chunk's own full neighborhood
//! sent to render it, not merely that chunk itself),
//! the exact Play-entry clientbound packet sequence, keep-alive/timeout handling, and the
//! first real `rc-scheduler` region ticking at 20 TPS. See `blueprints/M1/M1-B05-play-
//! superflat.md` for the full design.

mod chunk;
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
pub use world::{HARDCODED_REGION_ID, HardcodedWorld, PlayerMarker, SYNCHRONIZED_REGISTRIES};
