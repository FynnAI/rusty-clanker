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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = postcard::to_allocvec(trace)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, bytes)
}

/// A normal decode: succeeds regardless of `format_version`'s numeric value (the
/// version-gating policy lives in `read_trace_if_current`, not here) — `Err` only for
/// an absent/unreadable file or bytes that fail to decode as a `RedstoneTrace` at all.
pub fn read_trace(path: &Path) -> Result<RedstoneTrace, TraceReadError> {
    let bytes = std::fs::read(path).map_err(|source| TraceReadError::Io {
        path: path.display().to_string(),
        source,
    })?;
    postcard::from_bytes(&bytes).map_err(|source| TraceReadError::Decode {
        path: path.display().to_string(),
        source,
    })
}

/// `Ok(None)` for "absent, or a `format_version` mismatch" (both are legitimate,
/// silent "must regenerate" signals for `fetch-corpus`'s own cache-hit logic);
/// `Err` only for a genuine I/O or decode failure.
pub fn read_trace_if_current(path: &Path) -> Result<Option<RedstoneTrace>, TraceReadError> {
    if !path.exists() {
        return Ok(None);
    }
    match read_trace(path)? {
        trace if trace.format_version == TRACE_FORMAT_VERSION => Ok(Some(trace)),
        _stale => Ok(None),
    }
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
    let structural_mismatch = |detail: String| DiffError::StructuralMismatch {
        expected_id: expected.contraption_id.clone(),
        actual_id: actual.contraption_id.clone(),
        detail,
    };

    if expected.contraption_id != actual.contraption_id {
        return Err(structural_mismatch("contraption_id differs".to_string()));
    }
    if expected.bounds_min != actual.bounds_min || expected.bounds_max != actual.bounds_max {
        return Err(structural_mismatch(format!(
            "bounds differ: expected {:?}..={:?}, actual {:?}..={:?}",
            expected.bounds_min, expected.bounds_max, actual.bounds_min, actual.bounds_max
        )));
    }
    if expected.ticks.len() != actual.ticks.len() {
        return Err(structural_mismatch(format!(
            "tick count differs: expected {}, actual {}",
            expected.ticks.len(),
            actual.ticks.len()
        )));
    }

    let mut mismatches = Vec::new();
    let mut analog_gaps = Vec::new();

    for (expected_tick, actual_tick) in expected.ticks.iter().zip(actual.ticks.iter()) {
        if expected_tick.tick != actual_tick.tick {
            return Err(structural_mismatch(format!(
                "tick value mismatch at the same sequence index: expected {}, actual {}",
                expected_tick.tick, actual_tick.tick
            )));
        }
        if expected_tick.blocks.len() != actual_tick.blocks.len() {
            return Err(structural_mismatch(format!(
                "tick {}: blocks length differs: expected {}, actual {}",
                expected_tick.tick,
                expected_tick.blocks.len(),
                actual_tick.blocks.len()
            )));
        }
        for (expected_block, actual_block) in
            expected_tick.blocks.iter().zip(actual_tick.blocks.iter())
        {
            if expected_block.pos != actual_block.pos {
                return Err(structural_mismatch(format!(
                    "tick {}: position sequence differs at the same index: expected {:?}, actual {:?}",
                    expected_tick.tick, expected_block.pos, actual_block.pos
                )));
            }
            if expected_block.state_id != actual_block.state_id {
                mismatches.push(TraceMismatch {
                    tick: expected_tick.tick,
                    pos: expected_block.pos,
                    expected_state_id: expected_block.state_id,
                    actual_state_id: actual_block.state_id,
                });
            }
            if expected_block.analog != actual_block.analog {
                analog_gaps.push(AnalogNotYetComparable {
                    tick: expected_tick.tick,
                    pos: expected_block.pos,
                    expected_analog: expected_block.analog,
                    actual_analog: actual_block.analog,
                });
            }
        }
    }

    Ok(DiffReport {
        mismatches,
        analog_gaps,
    })
}
