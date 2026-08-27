//! The versioned, bit-exact `RedstoneTrace` format (blueprint Context, "The trace
//! format — exact schema") plus `diff_traces`, the structural-precondition-checked,
//! bit-exact comparison `xtask parity-check redstone` drives.

use std::path::Path;

pub const TRACE_FORMAT_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct RedstoneTrace {
    pub format_version: u32,
    pub contraption_id: String,
    pub source_jar_sha1: String,
    pub tool_version: String,
    pub bounds_min: (i32, i32, i32),
    pub bounds_max: (i32, i32, i32),
    pub ticks: Vec<TickSnapshot>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TickSnapshot {
    pub tick: u64,
    /// Sorted ascending by `(pos.1, pos.2, pos.0)` (y, z, x) — every position in
    /// `[bounds_min, bounds_max]`, no omissions (Context: "why full-volume").
    pub blocks: Vec<BlockObservation>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockObservation {
    pub pos: (i32, i32, i32),
    pub state_id: u32,
    pub analog: Option<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum TraceReadError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("postcard decode error reading {path}: {source}")]
    Decode {
        path: String,
        source: postcard::Error,
    },
}

/// One bit-exact divergence between an `expected` (captured) and `actual` (replayed)
/// trace at a specific tick and position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceMismatch {
    pub tick: u64,
    pub pos: (i32, i32, i32),
    pub expected_state_id: u32,
    pub actual_state_id: u32,
}

/// A separate, non-fatal-for-this-blueprint diagnostic (Context: "Comparator analog
/// value: forward-compatible, not solved here") — an `analog` field disagreement is
/// never folded into `TraceMismatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalogNotYetComparable {
    pub tick: u64,
    pub pos: (i32, i32, i32),
    pub expected_analog: Option<u8>,
    pub actual_analog: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffReport {
    pub mismatches: Vec<TraceMismatch>,
    pub analog_gaps: Vec<AnalogNotYetComparable>,
}

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("structural mismatch between traces for {expected_id} vs {actual_id}: {detail}")]
    StructuralMismatch {
        expected_id: String,
        actual_id: String,
        detail: String,
    },
}

/// Serializes via `postcard` (already workspace-pinned, CLUSTER-D12) — the format
/// this blueprint's git-ignored `corpus/redstone/<id>/trace.postcard` cache uses.
pub fn write_trace(path: &Path, trace: &RedstoneTrace) -> std::io::Result<()> {
    todo!()
}

/// A normal decode: succeeds regardless of `format_version`'s numeric value (the
/// version-gating policy lives in `read_trace_if_current`, not here) — `Err` only for
/// an absent/unreadable file or bytes that fail to decode as a `RedstoneTrace` at all.
pub fn read_trace(path: &Path) -> Result<RedstoneTrace, TraceReadError> {
    todo!()
}

/// `Ok(None)` for "absent, or a `format_version` mismatch" (both are legitimate,
/// silent "must regenerate" signals for `fetch-corpus`'s own cache-hit logic);
/// `Err` only for a genuine I/O or decode failure.
pub fn read_trace_if_current(path: &Path) -> Result<Option<RedstoneTrace>, TraceReadError> {
    todo!()
}

/// Structural precondition: `expected.contraption_id == actual.contraption_id`,
/// identical `bounds_min`/`bounds_max`, identical `ticks.len()`, and every
/// `TickSnapshot.tick` value appears in the same position in both — violated by a
/// caller bug (mismatched contraption/trace pairing), never by a legitimate parity
/// divergence.
pub fn diff_traces(
    expected: &RedstoneTrace,
    actual: &RedstoneTrace,
) -> Result<DiffReport, DiffError> {
    todo!()
}
