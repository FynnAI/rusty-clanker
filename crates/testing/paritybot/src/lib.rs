//! `rc-paritybot` — dev/test-only (TEST-D1/TEST-D8): the azalea-based bot driver.
//! `idle_stability` is this blueprint's own scenario; a future two-server comparator
//! (TEST-D9/TEST-D10, starting at M3+) wraps this same module's function twice, once
//! per server, rather than replacing it.

pub mod block_entity_persistence;
pub mod chunk_decode_check;
pub mod corpus_capture;
pub mod idle_stability;
pub mod load_scenario;
pub mod packet_capture;
pub mod packet_recorder;
pub mod placement_capture;
pub mod restart_persistence;
pub mod vanilla_registry_defaults;
