//! M2-B08's save-interval cadence analysis (Acceptance Criterion 3, `11-roadmap-
//! milestones.md`: "the configured save interval is measured, over a 30-minute run, to
//! fire within ±1 tick of its configured cadence"). Parses the `--save-event-log` a real
//! `rusty-clanker-server` process appends to (Context, "CLI/diagnostic surface") and
//! checks every consecutive same-region gap against the configured interval, tolerance
//! ±1 tick.

use std::path::Path;

/// One parsed line of a `--save-event-log` file (Context: `{"tick": u64, "region_id":
/// string, "elapsed_ms": u64}`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SaveEvent {
    pub tick: u64,
    pub region_id: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct CadenceViolation {
    /// Index into the parsed event sequence of the *later* of the two events whose
    /// gap violated tolerance.
    pub at_index: usize,
    pub expected_interval_ticks: u64,
    pub actual_interval_ticks: i64,
}

#[derive(Debug, Clone)]
pub struct CadenceReport {
    pub event_count: usize,
    pub violations: Vec<CadenceViolation>,
}

impl CadenceReport {
    pub fn within_tolerance(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Parses `path` as newline-delimited JSON `SaveEvent` records. Malformed/empty lines
/// are skipped, never a hard error (Context: "a partially-flushed log at the moment of
/// reading is expected, not exceptional").
pub fn parse_save_event_log(path: &Path) -> std::io::Result<Vec<SaveEvent>> {
    let content = std::fs::read_to_string(path)?;
    let mut events = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<SaveEvent>(trimmed) {
            events.push(event);
        }
    }
    Ok(events)
}

/// For every consecutive pair of events **restricted to the same `region_id`**, computes
/// `actual = events[i].tick as i64 - events[i-1].tick as i64` and records a
/// `CadenceViolation` whenever `(actual - expected_interval_ticks as i64).abs() > 1`
/// (AC3's own literal "±1 tick" tolerance). The very first event for a given
/// `region_id` never produces a violation.
pub fn analyze_cadence(events: &[SaveEvent], expected_interval_ticks: u64) -> CadenceReport {
    let mut last_tick_by_region: std::collections::HashMap<&str, u64> =
        std::collections::HashMap::new();
    let mut violations = Vec::new();

    for (index, event) in events.iter().enumerate() {
        if let Some(&last_tick) = last_tick_by_region.get(event.region_id.as_str()) {
            let actual = event.tick as i64 - last_tick as i64;
            if (actual - expected_interval_ticks as i64).abs() > 1 {
                violations.push(CadenceViolation {
                    at_index: index,
                    expected_interval_ticks,
                    actual_interval_ticks: actual,
                });
            }
        }
        last_tick_by_region.insert(event.region_id.as_str(), event.tick);
    }

    CadenceReport {
        event_count: events.len(),
        violations,
    }
}
