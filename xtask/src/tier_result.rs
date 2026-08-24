//! One machine-readable verb/tier result (TEST-D40). Every xtask verb in this
//! blueprint that is not nextest-driven writes exactly one of these as pretty JSON
//! to `target/verify/<tier>.json`.
//!
//! Deviation from M0-B08's literal spec: `TierResult`/`Status`/`CaseResult` also
//! derive `serde::Deserialize` here (the spec only lists `Serialize`) because
//! `tier1::run` (Deliverables, `xtask/src/tier1.rs`) must re-read each sub-verb's
//! already-written JSON file back into a `TierResult` rather than re-deriving it —
//! that round-trip is impossible without `Deserialize`. Recorded as a forced
//! deviation in this blueprint's final report.

use std::path::Path;
use std::process::ExitCode;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TierResult {
    pub tier: String,
    pub status: Status,
    pub cases: Vec<CaseResult>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CaseResult {
    pub name: String,
    pub status: Status,
    pub detail: Option<String>,
}

/// Fixed output root every verb writes under.
pub const VERIFY_OUT_DIR: &str = "target/verify";

impl TierResult {
    /// Starts an empty result for `tier` (e.g. `"path-guard"`); status is computed
    /// by `finalize`, not tracked incrementally.
    pub fn new(tier: impl Into<String>) -> Self {
        todo!()
    }

    pub fn push(&mut self, name: impl Into<String>, status: Status, detail: Option<String>) {
        todo!()
    }

    /// Sets `self.status` to `Fail` if any case is `Fail`, else `Pass`, and returns self.
    pub fn finalize(self) -> Self {
        todo!()
    }

    /// `Status::Pass` iff every case passed — the value `finalize` computes.
    pub fn overall(cases: &[CaseResult]) -> Status {
        todo!()
    }
}

/// Writes `result` as pretty JSON to `<VERIFY_OUT_DIR>/<result.tier>.json`, creating
/// parent directories as needed.
pub fn write(result: &TierResult) -> std::io::Result<()> {
    todo!()
}

/// Pure variant `write` delegates to, taking an explicit output root — the form
/// acceptance tests exercise directly against a tempdir.
pub fn write_to(root: &Path, result: &TierResult) -> std::io::Result<()> {
    todo!()
}

/// `Status::Pass` -> `ExitCode::SUCCESS`, `Status::Fail` -> `ExitCode::FAILURE`.
pub fn exit_code_for(status: Status) -> ExitCode {
    todo!()
}
