//! The versioned, bit-exact `RedstoneTrace` format (blueprint Context, "The trace
//! format — exact schema") plus `diff_traces`, the structural-precondition-checked,
//! bit-exact comparison `xtask parity-check redstone` drives.

use std::path::Path;

/// Bumped from `1` in M3.5-B03's own follow-up governance changeset (deliverable 7,
/// `docs/findings-for-planning.md`): `RedstoneTrace` gained `spec_sha256`. Postcard is
/// not self-describing (no field tags), so a struct-shape change is not itself safely
/// detectable by attempting the full decode against a cached file the OLD shape wrote
/// — `read_trace_if_current` therefore probes this leading field first (mirrors
/// `protocol_capture::PROTOCOL_CAPTURE_FORMAT_VERSION`'s own identical bump and
/// rationale, restated here since this module has no dependency on that one) and
/// rejects a mismatch before ever attempting the full decode.
pub const TRACE_FORMAT_VERSION: u32 = 2;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct RedstoneTrace {
    pub format_version: u32,
    pub contraption_id: String,
    pub source_jar_sha1: String,
    /// M3.5-B03 follow-up (deliverable 7): SHA-256 (lowercase hex, `xtask::
    /// fixture_manifest::compute_sha256_hex`'s own identical format — this crate
    /// never depends on `xtask` in production code, so the hash is computed
    /// independently at each writer's own call site, never imported) of the exact
    /// committed `.ron` fixture bytes this trace was captured from — `fetch-corpus`'s
    /// own cache-currency check (`corpus_capture.rs`) used to treat a matching
    /// `source_jar_sha1` alone as "this cached trace is still current," which stayed
    /// true forever for an edited fixture (nothing about the *jar* changes when only
    /// the fixture's own geometry does) and silently kept serving a stale capture. A
    /// trace this field is not meaningful for (`replay_contraption`'s own in-process
    /// "ours" side, produced fresh every run, never cached to disk) leaves this empty
    /// — mirrors `source_jar_sha1`'s own identical "Replay has no jar provenance"
    /// convention.
    pub spec_sha256: String,
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

/// The leading-field-only probe `read_trace_if_current` decodes first (mirrors
/// `protocol_capture::FormatVersionProbe`'s own identical rationale, restated here:
/// postcard reads a struct's fields strictly in declaration order with no look-ahead,
/// so deserializing this single-field prefix of the real `RedstoneTrace` shape
/// correctly consumes only `format_version`'s own bytes regardless of what — if
/// anything, on any trace format version old or new — follows).
#[derive(serde::Deserialize)]
struct TraceFormatVersionProbe {
    format_version: u32,
}

/// `Ok(None)` for "absent, or a `format_version` mismatch" (both are legitimate,
/// silent "must regenerate" signals for `fetch-corpus`'s own cache-hit logic);
/// `Err` only for a genuine I/O or decode failure. The version check itself reads
/// only `TraceFormatVersionProbe`'s own leading field first, never the full
/// `RedstoneTrace` shape (M3.5-B03 follow-up, deliverable 7): postcard's own lack of
/// field tags means attempting the full decode against an old-format cache file
/// (predating a `RedstoneTrace` shape change such as `spec_sha256`'s own addition)
/// risks silently misaligned garbage rather than a clean parse failure, since
/// `format_version` sits at a fixed leading position in every shape version this
/// module has ever shipped — the exact same risk `protocol_capture`'s own module
/// doc comment documents in full for its own identical-shaped problem.
pub fn read_trace_if_current(path: &Path) -> Result<Option<RedstoneTrace>, TraceReadError> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|source| TraceReadError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let (probe, _rest): (TraceFormatVersionProbe, &[u8]) = postcard::take_from_bytes(&bytes)
        .map_err(|source| TraceReadError::Decode {
            path: path.display().to_string(),
            source,
        })?;
    if probe.format_version != TRACE_FORMAT_VERSION {
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
