//! `rc-gametest` — dev/test-only (TEST-D1, WS-D2 reserved path, first populated by
//! M3-B07). This blueprint's own content is exactly the redstone-corpus
//! infrastructure below (trace format, `ContraptionSpec`, the Stage-4 replay driver,
//! and the azalea-free half of the vanilla-oracle capture pipeline); a future
//! blueprint may extend this same crate with TEST-D14's generic
//! `#[rc_gametest]`/`TestContext` structure-test DSL for non-redstone cases without
//! conflicting with anything here (Context, "Scope boundary").
//!
//! This crate never depends on `rc-paritybot` (`Cargo.toml`'s own doc comment has the
//! full citation) — the two live-oracle capture orchestration functions
//! (`capture_contraption`/`run_full_corpus_capture`) therefore live in
//! `rc_paritybot::corpus_capture` instead, calling back into this crate's own
//! `capture`/`trace`/`spec` items.

pub mod capture;
pub mod replay;
pub mod spec;
pub mod trace;

pub use replay::replay_contraption;
pub use spec::{Category, ContraptionSpec, PlacedBlock, ScriptedAction};
pub use trace::{
    AnalogNotYetComparable, BlockObservation, RedstoneTrace, TRACE_FORMAT_VERSION, TickSnapshot,
    TraceMismatch,
};
