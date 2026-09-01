//! Subprocess entry point `xtask::corpus::placement_diff` spawns for one side's own
//! capture run — identical forced-deviation precedent to `fetch_corpus_runner`/
//! `load_scenario_runner` (their own doc comments have the full "`xtask.exe` must
//! never link `azalea`" citation, restated here verbatim): `rc_paritybot::
//! placement_capture` needs a real, live bot connection.
//!
//! Unlike `fetch_corpus_runner` (which only ever launches the real oracle — its own
//! capture leg drives every real placement through the oracle's console instead of the
//! bot), this binary launches **either** side's own server process itself: the real
//! vanilla oracle (`rc_gametest::capture::launch_oracle_server`, azalea-free, already
//! usable from this crate) for `oracle`, or our own real `rusty-clanker-server` release
//! binary (`rc_test_harness::process::spawn_server_with_world_dir`, likewise
//! azalea-free) for `ours` — `placement_capture::run_capture` itself is side-agnostic
//! (that module's own doc comment), so which process backs `host:port` is entirely
//! this binary's own concern, never threaded into the capture logic.
//!
//! Usage:
//! ```text
//! placement_diff_runner oracle <jar_path> <work_dir> <out_capture_path> <source_jar_sha1> [only_id]
//! placement_diff_runner ours <server_bin_path> <world_dir> <out_capture_path> [only_id]
//! ```
//! Prints a small line-based result to stdout (no `serde_json` dependency in this
//! crate, Constraint (c), mirroring `fetch_corpus_runner`'s own identical convention):
//! ```text
//! RESULT=OK
//! ```
//! or
//! ```text
//! RESULT=ERROR
//! MESSAGE=<single-line error description>
//! ```
//! The full per-scenario, per-cell capture itself is never printed to stdout — it is
//! written to `<out_capture_path>` via `rc_gametest::placement_trace::write_capture`
//! (postcard), which `xtask::corpus::placement_diff` reads back directly (that crate
//! already links `rc-gametest`, `parity_check.rs`'s own precedent). Exit code 0 iff
//! `RESULT=OK`.

use std::path::PathBuf;
use std::time::Duration;

use rc_gametest::placement_spec::{InteractionScenario, enumerate_scenarios};
use rc_gametest::placement_trace::write_capture;
use rc_paritybot::placement_capture::run_capture;

/// Distinct from `fetch-corpus`'s own fixed `25566` (that verb's own `corpus_capture.rs`
/// doc comment) — never launched concurrently with a `fetch-corpus`/`parity-check`
/// run in practice, but kept on its own port regardless so the two verbs' own oracle
/// processes can never collide if they ever are.
const ORACLE_PORT: u16 = 25567;

fn single_line(text: impl std::fmt::Display) -> String {
    text.to_string().replace('\n', " ")
}

fn fail(message: impl std::fmt::Display) -> std::process::ExitCode {
    println!("RESULT=ERROR");
    println!("MESSAGE={}", single_line(message));
    std::process::ExitCode::FAILURE
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(side) = args.first() else {
        eprintln!(
            "usage: placement_diff_runner oracle <jar_path> <work_dir> <out_capture_path> <source_jar_sha1> [only_id]\n       placement_diff_runner ours <server_bin_path> <world_dir> <out_capture_path> [only_id]"
        );
        return std::process::ExitCode::FAILURE;
    };

    let scenarios = enumerate_scenarios();
    let interactions = InteractionScenario::ALL;
    let side = side.clone();
    let rest = args[1..].to_vec();

    // Every azalea-driven task this run spawns (`placement_capture::run_capture` ->
    // `packet_capture::connect_and_observe` -> `ClientBuilder::start`) is `!Send` and
    // relies on `tokio::task::spawn_local` — this `LocalSet` is the ambient context
    // that keeps it polled for this whole binary's lifetime, mirroring
    // `fetch_corpus_runner`'s own identical `LocalSet::run_until` wrapper.
    let local = tokio::task::LocalSet::new();
    let result = local
        .run_until(async move {
            match side.as_str() {
                "oracle" => run_oracle_side(&rest, &scenarios, &interactions).await,
                "ours" => run_ours_side(&rest, &scenarios, &interactions).await,
                other => Err(format!(
                    "unknown side {other:?} — expected \"oracle\" or \"ours\""
                )),
            }
        })
        .await;

    match result {
        Ok(()) => {
            println!("RESULT=OK");
            std::process::ExitCode::SUCCESS
        }
        Err(message) => fail(message),
    }
}

async fn run_oracle_side(
    args: &[String],
    scenarios: &[rc_gametest::PlacementScenario],
    interactions: &[InteractionScenario],
) -> Result<(), String> {
    if args.len() < 4 || args.len() > 5 {
        return Err(
            "usage: placement_diff_runner oracle <jar_path> <work_dir> <out_capture_path> <source_jar_sha1> [only_id]"
                .to_string(),
        );
    }
    let jar_path = PathBuf::from(&args[0]);
    let work_dir = PathBuf::from(&args[1]);
    let out_capture_path = PathBuf::from(&args[2]);
    let source_jar_sha1 = args[3].clone();
    let only = args.get(4).cloned();

    // Governance fix: a real end-to-end run shares this machine with other real,
    // CPU-heavy work (other verification sessions, the user's own foreground use) --
    // 120s (`fetch_corpus.rs`'s own `launch_oracle_server` call site's identical
    // budget, that verb's own single-purpose machine assumption) was observed live to
    // be too tight for a genuinely contended JVM cold start; 300s absorbs that
    // without meaningfully weakening the "the oracle process is actually broken"
    // signal a real timeout still provides.
    let handle = rc_gametest::capture::launch_oracle_server(
        &jar_path,
        &work_dir,
        ORACLE_PORT,
        Duration::from_secs(300),
    )
    .map_err(|err| format!("failed to launch the oracle server: {err}"))?;

    let capture = run_capture(
        "127.0.0.1",
        handle.port,
        scenarios,
        interactions,
        only.as_deref(),
        format!("oracle:{source_jar_sha1}"),
    )
    .await
    .map_err(|err| format!("capture failed: {err}"))?;

    // `handle` is dropped (killing the oracle process, `OracleServerHandle`'s own
    // guaranteed-teardown `Drop`) only after every real placement this run needed has
    // already happened — held in scope for the whole capture, never released early.
    drop(handle);

    write_capture(&out_capture_path, &capture)
        .map_err(|err| format!("failed to write {}: {err}", out_capture_path.display()))
}

async fn run_ours_side(
    args: &[String],
    scenarios: &[rc_gametest::PlacementScenario],
    interactions: &[InteractionScenario],
) -> Result<(), String> {
    if args.len() < 3 || args.len() > 4 {
        return Err(
            "usage: placement_diff_runner ours <server_bin_path> <world_dir> <out_capture_path> [only_id]"
                .to_string(),
        );
    }
    let server_bin = PathBuf::from(&args[0]);
    let world_dir = PathBuf::from(&args[1]);
    let out_capture_path = PathBuf::from(&args[2]);
    let only = args.get(3).cloned();

    let mut config = rc_test_harness::process::ManagedServerConfig::new(server_bin);
    config.world_dir = Some(world_dir);
    // As the oracle side's own identical fix above: a contended machine can make even
    // our own (normally near-instant) startup slower than `ManagedServerConfig::new`'s
    // own single-purpose-machine 30s default.
    config.startup_timeout = Duration::from_secs(90);
    let managed = rc_test_harness::process::spawn_server_with_world_dir(config)
        .map_err(|err| format!("failed to spawn rusty-clanker-server: {err}"))?;

    let capture = run_capture(
        "127.0.0.1",
        managed.addr.port(),
        scenarios,
        interactions,
        only.as_deref(),
        "ours".to_string(),
    )
    .await
    .map_err(|err| format!("capture failed: {err}"))?;

    // `managed` is dropped only after the capture completes — same guaranteed-teardown
    // discipline as the oracle side above (`ManagedServer`'s own `Drop`).
    drop(managed);

    write_capture(&out_capture_path, &capture)
        .map_err(|err| format!("failed to write {}: {err}", out_capture_path.display()))
}
