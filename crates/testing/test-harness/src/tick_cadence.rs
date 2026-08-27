//! M3-B08's TPS-measurement leg (Acceptance Criterion 2, `11-roadmap-milestones.md`:
//! "sustained tick rate measured... within ±1% of the target 20 TPS"). Parses the
//! `--tick-log` a real `rusty-clanker-server` process appends to (Context, "TPS
//! measurement — reusing M0-B06's formula and threshold, reimplemented as an external
//! log read") and reapplies M0-B06's own `measured_tps = N/T`, `drift_ratio =
//! measured_tps/target_tps - 1.0`, `\|drift_ratio\| <= tolerance` soak-pass formula —
//! restated here as a black-box, log-based measurement, since `rc-scheduler`'s own
//! in-process `RegionTickHistogram`/`SoakReport` code is unreachable from `xtask`,
//! which only ever sees the server as an opaque subprocess.
//!
//! Deviation from M0-B06's own window ("first tick's start to last tick's
//! completion"): this log carries only one timestamp per line — the wall-clock
//! instant a tick's own completion was appended, `elapsed_ms` — so `duration_secs` is
//! measured from the *first* logged sample's `elapsed_ms` to the *last* logged
//! sample's `elapsed_ms`, one tick's worth of bias out of several thousand samples
//! over a full run, immaterial to a ±1% gate (Context).

use std::path::Path;

/// One parsed line of a `--tick-log` file (Context). `Serialize` is included so
/// `fixture_tick_writer` (this blueprint's own Tier-1 self-test fixture binary) can
/// construct the identical NDJSON shape a real `rusty-clanker-server` would write,
/// rather than hand-formatting a JSON string.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TickLogEntry {
    pub tick: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct TpsReport {
    pub sample_count: u64,
    pub duration_secs: f64,
    pub measured_tps: f64,
    pub drift_ratio: f64,
    pub within_tolerance: bool,
}

/// Parses `path` as newline-delimited JSON `TickLogEntry` records (Context's exact
/// `--tick-log` format). Malformed/empty lines are skipped, never a hard error — the
/// identical "a partially-flushed log is expected, not exceptional" tolerance
/// `save_cadence::parse_save_event_log` (M2-B08) already establishes.
pub fn parse_tick_log(path: &Path) -> std::io::Result<Vec<TickLogEntry>> {
    let content = std::fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<TickLogEntry>(trimmed) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

/// Pure: M0-B06's own `measured_tps = N/T`, `drift_ratio = measured_tps/target - 1.0`
/// formula (Context has the exact interval-based adaptation). Uses only the first and
/// last entries' `elapsed_ms` plus the total entry count — every entry strictly
/// between them contributes to `sample_count` but not to the timing window itself
/// (module doc comment's own restated deviation).
///
/// Panics if `entries.len() < 2` (nothing to measure a rate from) — a caller-level
/// bug, never a legitimate "server produced too few samples" case this function
/// should paper over silently.
pub fn analyze_tps(entries: &[TickLogEntry], target_tps: f64, tolerance: f64) -> TpsReport {
    assert!(
        entries.len() >= 2,
        "analyze_tps requires at least two tick-log samples to measure a rate from, got {}",
        entries.len()
    );
    let first = entries.first().expect("length checked above");
    let last = entries.last().expect("length checked above");

    let duration_secs = (last.elapsed_ms - first.elapsed_ms) as f64 / 1000.0;
    let sample_count = entries.len() as u64 - 1;
    let measured_tps = sample_count as f64 / duration_secs;
    let drift_ratio = measured_tps / target_tps - 1.0;
    let within_tolerance = drift_ratio.abs() <= tolerance;

    TpsReport {
        sample_count,
        duration_secs,
        measured_tps,
        drift_ratio,
        within_tolerance,
    }
}
