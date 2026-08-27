//! M3-B07's own live-oracle capture orchestration. Forced deviation from that
//! blueprint's own Deliverables sketch, which places `capture_contraption`/
//! `run_full_corpus_capture` inside `rc_gametest::capture` (final report has the full
//! citation): those two functions are the only pieces of this blueprint's whole
//! capture pipeline that need a *live* `BlockSnapshotView` (this crate's own
//! `packet_capture::connect_and_observe`, azalea-backed) — `rc-gametest` itself must
//! never depend on `rc-paritybot` (that crate's own `Cargo.toml` doc comment has the
//! full citation), so these two functions live here instead, calling back into
//! `rc_gametest::capture`'s azalea-free items
//! (`OracleServerHandle`/`launch_oracle_server`/`send_console_command`/
//! `check_state_id_consistency`) and `rc_gametest::{spec, trace}` directly. Driven as
//! a real OS subprocess (`fetch_corpus_runner`, this crate's own new bin target) by
//! `xtask::corpus::fetch_corpus`, mirroring `idle_stability_runner`/
//! `restart_persistence_runner`'s already-established subprocess pattern exactly —
//! `xtask.exe` itself never links this module or `azalea`.

use std::path::Path;

use rc_gametest::capture::{CaptureError, OracleServerHandle, check_state_id_consistency};
use rc_gametest::spec::{ContraptionSpec, bounding_box, world_origin_for};
use rc_gametest::trace::RedstoneTrace;

use crate::packet_capture::BlockSnapshotView;

/// Full end-to-end capture for one contraption at `world_origin_for(index)` against
/// an already-launched `handle` and an already-connected `view` (blueprint Context,
/// capture pipeline steps 3–10, restated as this function's exact algorithm —
/// freeze, gamerules, teleport, place-with-validation, snapshot tick 0,
/// scripted-action + step loop, snapshot per tick, `fill air` cleanup).
/// `source_jar_sha1` is threaded straight into the resulting `RedstoneTrace`.
pub async fn capture_contraption(
    handle: &mut OracleServerHandle,
    view: &BlockSnapshotView,
    spec: &ContraptionSpec,
    index: usize,
    source_jar_sha1: &str,
) -> Result<RedstoneTrace, CaptureError> {
    todo!()
}

/// Orchestrates the whole corpus: launches one oracle, connects one bot, applies the
/// shared gamerule set once, then calls `capture_contraption` once per `specs`
/// entry (in slice order, using that entry's own index for `world_origin_for`),
/// writing each result via `rc_gametest::trace::write_trace` to
/// `corpus_dir.join(&spec.id).join("trace.postcard")` — skipping (not re-capturing)
/// any contraption whose cached trace's `source_jar_sha1` already matches
/// `source_jar_sha1` (blueprint Context, "Fixture custody").
pub async fn run_full_corpus_capture(
    jar_path: &Path,
    work_dir: &Path,
    corpus_dir: &Path,
    specs: &[ContraptionSpec],
    source_jar_sha1: &str,
) -> Result<Vec<(String, Result<(), CaptureError>)>, CaptureError> {
    todo!()
}
