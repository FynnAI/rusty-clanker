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
use std::time::Duration;

use rc_gametest::capture::{
    CaptureError, OracleServerHandle, check_state_id_consistency, send_console_command,
};
use rc_gametest::spec::{ContraptionSpec, bounding_box, world_origin_for};
use rc_gametest::trace::{
    BlockObservation, RedstoneTrace, TickSnapshot, read_trace_if_current, write_trace,
};

use crate::packet_capture::BlockSnapshotView;

/// Bounded wait for one placement's resulting packet to arrive (blueprint Context,
/// "Rates and limits" budgets one capture step at ≤50 ms; this poll is generous
/// enough to absorb real network/tick-processing jitter without ever hanging
/// forever). Polling (not a single fixed sleep) lets the ambient `LocalSet` driving
/// the azalea client task make progress between checks.
const OBSERVATION_POLL_INTERVAL: Duration = Duration::from_millis(20);
const OBSERVATION_TIMEOUT: Duration = Duration::from_secs(5);
/// The whole-corpus-run bot's offline account name — a single source of truth shared
/// by both `run_full_corpus_capture`'s own `connect_and_observe` call and
/// `capture_contraption`'s per-contraption `tp` target, so the two can never drift
/// apart. Kept at 13 characters, comfortably under vanilla's own 16-character
/// username limit (`ServerboundHello::name`'s `#[limit(16)]`, azalea-protocol
/// `packets/login/s_hello.rs`) — the previous `"rc_fetch_corpus_bot"` (19
/// characters) silently over-ran that limit on write (azalea's own `#[limit]`
/// enforces only on *read*, never on write), which the real oracle's own Hello
/// decoder then rejected outright (`DecoderException: Failed to decode packet
/// 'serverbound/minecraft:hello'`), disconnecting the bot before it ever reached
/// `Play` state — see `docs/findings-for-planning.md` for the full writeup.
const CORPUS_BOT_NAME: &str = "rc_corpus_bot";
/// Real-wall-clock allowance per `tick step 1` (blueprint Context, "Rates and
/// limits": "≤50 ms per step including the snapshot read" is the *budget*, not a
/// hard wait — this is simply how long this module gives the oracle's own tick and
/// the resulting packets time to land before reading the snapshot).
const TICK_STEP_SETTLE: Duration = Duration::from_millis(50);

/// Polls `view` for `pos`'s state id until it matches `expected` or `OBSERVATION_
/// TIMEOUT` elapses, returning whatever was last observed either way (`None` only
/// if `pos`'s containing chunk never loaded at all within the deadline).
///
/// Polling for a *match*, not merely for *any* reported value, is load-bearing now
/// that `BlockSnapshotView::state_id_at` reads azalea's own live world model
/// (`packet_capture`'s own module doc comment): a freshly-loaded chunk reports a
/// real `Some` state immediately, which for a position observed *before* its
/// placement has landed is still the pre-placement value, not an absence — stopping
/// on the first `Some` (this function's own pre-fix behavior) would then report that
/// stale value as if it were already the placement's result. Waiting for the exact
/// expected id sidesteps ever needing to distinguish "stale" from "current" by
/// timestamp; the caller (`capture_contraption`) still surfaces a plain mismatch,
/// unchanged, if the expected id never arrives before the deadline.
async fn wait_for_state_id(
    view: &BlockSnapshotView,
    pos: (i32, i32, i32),
    expected: u32,
) -> Option<u32> {
    let deadline = tokio::time::Instant::now() + OBSERVATION_TIMEOUT;
    let mut last_seen;
    loop {
        last_seen = view.state_id_at(pos);
        if last_seen == Some(expected) {
            return last_seen;
        }
        if tokio::time::Instant::now() >= deadline {
            return last_seen;
        }
        tokio::time::sleep(OBSERVATION_POLL_INTERVAL).await;
    }
}

fn world_pos(origin: (i32, i32, i32), rel: (i32, i32, i32)) -> (i32, i32, i32) {
    (origin.0 + rel.0, origin.1 + rel.1, origin.2 + rel.2)
}

/// `/setblock <world x> <world y> <world z> <vanilla_state>` (blueprint Context,
/// capture pipeline step 7/9 — placement and every scripted action are both plain,
/// immediate `Level.setBlock` calls, neither privileged over the other).
fn issue_setblock(
    handle: &mut OracleServerHandle,
    world_pos: (i32, i32, i32),
    vanilla_state: &str,
) -> Result<(), CaptureError> {
    send_console_command(
        handle,
        &format!(
            "setblock {} {} {} {vanilla_state}",
            world_pos.0, world_pos.1, world_pos.2
        ),
    )
}

/// Snapshots every position in `[bounds_min, bounds_max]` (relative to `origin`)
/// from `view`'s currently-known state, in the trace format's own `(y, z, x)`
/// ascending canonical order — the live-oracle counterpart to `rc_gametest::replay`'s
/// private `snapshot_volume`, reading a `BlockSnapshotView` instead of a
/// `BlockWorldAccess`.
fn snapshot_volume_from_view(
    view: &BlockSnapshotView,
    origin: (i32, i32, i32),
    bounds_min: (i32, i32, i32),
    bounds_max: (i32, i32, i32),
    has_analog: impl Fn((i32, i32, i32)) -> bool,
) -> Vec<BlockObservation> {
    let mut out = Vec::new();
    for y in bounds_min.1..=bounds_max.1 {
        for z in bounds_min.2..=bounds_max.2 {
            for x in bounds_min.0..=bounds_max.0 {
                let rel = (x, y, z);
                let wp = world_pos(origin, rel);
                let state_id = view.state_id_at(wp).unwrap_or(0);
                let analog = if has_analog(rel) {
                    view.analog_at(wp)
                } else {
                    None
                };
                out.push(BlockObservation {
                    pos: rel,
                    state_id,
                    analog,
                });
            }
        }
    }
    out
}

/// Full end-to-end capture for one contraption at `world_origin_for(index)` against
/// an already-launched `handle` and an already-connected `view` (blueprint Context,
/// capture pipeline steps 3–10, restated as this function's exact algorithm —
/// freeze, gamerules, teleport, place-with-validation, snapshot tick 0,
/// scripted-action + step loop, snapshot per tick, `fill air` cleanup).
/// `source_jar_sha1` is threaded straight into the resulting `RedstoneTrace`. Step
/// 3 (freeze) and step 4 (gamerules) are one-time, whole-corpus setup performed by
/// `run_full_corpus_capture`, not repeated here.
pub async fn capture_contraption(
    handle: &mut OracleServerHandle,
    view: &BlockSnapshotView,
    spec: &ContraptionSpec,
    index: usize,
    source_jar_sha1: &str,
) -> Result<RedstoneTrace, CaptureError> {
    let origin = world_origin_for(index);
    let (bounds_min, bounds_max) = bounding_box(spec);
    let has_analog: std::collections::HashSet<(i32, i32, i32)> = spec
        .blocks
        .iter()
        .filter(|b| b.has_analog_state)
        .map(|b| b.pos)
        .collect();
    let has_analog = move |pos: (i32, i32, i32)| has_analog.contains(&pos);

    // Step 6 (teleport): follow the bot to this contraption's origin.
    send_console_command(
        handle,
        &format!(
            "tp {CORPUS_BOT_NAME} {} {} {}",
            origin.0, origin.1, origin.2
        ),
    )?;

    // Step 7: placement, self-validating against the oracle's own observed state id.
    for block in &spec.blocks {
        let wp = world_pos(origin, block.pos);
        issue_setblock(handle, wp, &block.vanilla_state)?;
        let observed = wait_for_state_id(view, wp, block.state_id)
            .await
            .ok_or_else(|| CaptureError::ObservationTimeout {
                contraption_id: spec.id.clone(),
                pos: block.pos,
            })?;
        check_state_id_consistency(block, observed).map_err(|(declared, observed)| {
            CaptureError::StateIdMismatch {
                contraption_id: spec.id.clone(),
                pos: block.pos,
                declared,
                observed,
                vanilla_state: block.vanilla_state.clone(),
            }
        })?;
    }

    // Step 8: tick 0 snapshot.
    let mut ticks = Vec::with_capacity(spec.max_ticks as usize + 1);
    ticks.push(TickSnapshot {
        tick: 0,
        blocks: snapshot_volume_from_view(view, origin, bounds_min, bounds_max, &has_analog),
    });

    // Step 9: scripted actions + tick-step loop.
    for t in 1..=spec.max_ticks as u64 {
        for action in spec.actions.iter().filter(|a| a.tick == t) {
            let wp = world_pos(origin, action.pos);
            issue_setblock(handle, wp, &action.vanilla_state)?;
        }
        send_console_command(handle, "tick step 1")?;
        tokio::time::sleep(TICK_STEP_SETTLE).await;
        ticks.push(TickSnapshot {
            tick: t,
            blocks: snapshot_volume_from_view(view, origin, bounds_min, bounds_max, &has_analog),
        });
    }

    // Step 10 (cleanup half): clear this contraption's footprint before the next
    // `world_origin_for` slot is used — write-and-persist is `run_full_corpus_
    // capture`'s own job, not this function's.
    let min_wp = world_pos(origin, bounds_min);
    let max_wp = world_pos(origin, bounds_max);
    send_console_command(
        handle,
        &format!(
            "fill {} {} {} {} {} {} air",
            min_wp.0, min_wp.1, min_wp.2, max_wp.0, max_wp.1, max_wp.2
        ),
    )?;

    Ok(RedstoneTrace {
        format_version: rc_gametest::trace::TRACE_FORMAT_VERSION,
        contraption_id: spec.id.clone(),
        source_jar_sha1: source_jar_sha1.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        bounds_min,
        bounds_max,
        ticks,
    })
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
    let mut handle = rc_gametest::capture::launch_oracle_server(
        jar_path,
        work_dir,
        25566,
        Duration::from_secs(120),
    )?;

    // Step 3: freeze immediately — the very first command written to stdin.
    send_console_command(&mut handle, "tick freeze")?;
    // Step 4: gamerules — eliminates every non-redstone source of block-state change.
    for gamerule in [
        "gamerule doDaylightCycle false",
        "gamerule doWeatherCycle false",
        "gamerule randomTickSpeed 0",
        "gamerule doMobSpawning false",
    ] {
        send_console_command(&mut handle, gamerule)?;
    }

    // Step 6: one bot connection for the whole corpus run.
    let (view, _observer) = crate::packet_capture::connect_and_observe(
        "127.0.0.1",
        handle.port,
        CORPUS_BOT_NAME,
        Duration::from_secs(30),
    )
    .await
    .map_err(|err| CaptureError::BotConnect(err.to_string()))?;

    let mut results = Vec::with_capacity(specs.len());
    for (index, spec) in specs.iter().enumerate() {
        let trace_path = corpus_dir.join(&spec.id).join("trace.postcard");
        if let Ok(Some(cached)) = read_trace_if_current(&trace_path)
            && cached.source_jar_sha1 == source_jar_sha1
        {
            results.push((spec.id.clone(), Ok(())));
            continue;
        }

        let outcome = capture_contraption(&mut handle, &view, spec, index, source_jar_sha1).await;
        let outcome = match outcome {
            Ok(trace) => write_trace(&trace_path, &trace).map_err(CaptureError::Io),
            Err(err) => Err(err),
        };
        results.push((spec.id.clone(), outcome));
    }

    Ok(results)
}
