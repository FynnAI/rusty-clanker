//! The soak run's machine-readable report types (Context: "Measurement definition").

use rc_messaging::RegionId;

/// One region's derived tick-duration summary over a soak run (Context's "Measurement
/// definition").
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegionTickHistogram {
    pub region_id: RegionId,
    pub sample_count: u64,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    /// Samples exceeding `tick_budget_ms` — informational, not part of the pass/fail
    /// gate (Context).
    pub over_budget_count: u64,
}

impl RegionTickHistogram {
    /// Computes every derived field from `samples` (per-tick durations in
    /// milliseconds). Percentiles use the nearest-rank method (Context). Panics if
    /// `samples` is empty.
    pub fn from_samples(region_id: RegionId, samples: &[f64], tick_budget_ms: f64) -> Self {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoakStatus {
    Pass,
    Fail,
}

/// The whole soak run's machine-readable report (this blueprint's own resolution of the
/// task's "machine-readable pass/fail output" requirement, complementary to nextest's
/// own JUnit XML — Context). Written as pretty-printed JSON by this blueprint's soak
/// test to a fixed path (Acceptance tests).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SoakReport {
    pub region_count: usize,
    pub target_tps: f64,
    pub target_tick_budget_ms: f64,
    pub wall_clock_duration_secs: f64,
    pub per_region: Vec<RegionTickHistogram>,
    /// `measured_tps / target_tps - 1.0` per region, same order as `per_region`
    /// (Context's exact drift definition).
    pub tps_drift_ratio: Vec<f64>,
    pub zero_panics: bool,
    pub status: SoakStatus,
}
