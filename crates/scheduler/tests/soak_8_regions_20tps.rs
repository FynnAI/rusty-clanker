//! M0's acceptance criterion 1: an 8-synthetic-region, 20 TPS +/- 1%, continuous
//! 10-minute soak with zero panics and a machine-readable report. Tier 2 (nightly) --
//! gated behind the `soak-tests` feature so `cargo nextest run -p rc-scheduler`
//! (default features, every PR) never even compiles this file.

#![cfg(feature = "soak-tests")]

mod common;

use std::fs;
use std::time::{Duration, Instant};

use rc_core::DimensionId;
use rc_scheduler::pool::{RcWorkerPool, SystemTickWaiter, TickClock};
use rc_scheduler::{
    DomainGroup, GridCell, LifecycleOutcome, RcExecutorBuilder, RegionManager, RegionTickHistogram,
    SoakReport, SoakStatus, SyntheticLoadProfile, bootstrap_default_profile,
    synthetic_system_factory,
};

const REGION_COUNT: usize = 8;
const TARGET_TPS: f64 = 20.0;
const TARGET_TICK_BUDGET_MS: f64 = 50.0;
const DRIFT_TOLERANCE: f64 = 0.01;

/// The full soak run's duration. Defaults to the acceptance criterion's own 600
/// seconds (this default -- the committed value -- must never change, per this
/// blueprint's own Constraints (a)); a caller may shorten it for a local smoke run
/// via `RC_SOAK_DURATION_SECS` without editing this file, e.g. to sanity-check
/// pacing/drift before committing to the full 10-minute run (this blueprint's own
/// Implementation step 9).
fn soak_duration() -> Duration {
    let secs = std::env::var("RC_SOAK_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(600);
    Duration::from_secs(secs)
}

#[test]
fn soak_8_regions_stable_20tps_10min() {
    let mut builder = RcExecutorBuilder::new(bootstrap_default_profile);
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        synthetic_system_factory(),
        vec![],
    );
    let executor = builder
        .build()
        .expect("synthetic_busy_work_system must build without a conflict-graph error");

    let pool = RcWorkerPool::new(4);
    let transport = common::MockTransport::new();
    let mut manager = RegionManager::new(&executor, TARGET_TICK_BUDGET_MS);

    // Spaced two cells apart (never `GridCell::new(_, i, 0)` for consecutive `i`): two
    // adjacent regions with any non-zero load are, by ARCH-D6's own merge rule, a
    // combined-EWMA-under-threshold pair racing toward a 100-tick (5s) merge -- for
    // 8 regions in an unbroken line that fires well inside this test's own run, wiping
    // out the "every outcome is None" invariant this soak is meant to check. Spacing
    // every spawned cell 2 apart keeps all 8 regions mutually non-adjacent (`GridCell`
    // adjacency requires a coordinate difference of exactly 1), so no merge candidate
    // pair ever exists and this test genuinely measures 8 *independent* regions' TPS
    // stability, per its own Goal (Context: "Why round-robin, not one-thread-per-region").
    let mut region_ids = Vec::with_capacity(REGION_COUNT);
    for i in 0..REGION_COUNT {
        let id = manager.spawn_region(
            DimensionId::OVERWORLD,
            [GridCell::new(DimensionId::OVERWORLD, (i * 2) as i32, 0)],
        );
        manager
            .region_mut(id)
            .unwrap()
            .state
            .world
            .insert_resource(SyntheticLoadProfile {
                busy_work_micros: 1500,
            });
        region_ids.push(id);
    }

    let mut clock: TickClock<SystemTickWaiter> = TickClock::new();
    let duration = soak_duration();

    let mut per_region_samples_ms: Vec<Vec<f64>> = vec![Vec::new(); REGION_COUNT];
    let run_start = Instant::now();
    let mut first_tick_start: Option<Instant> = None;
    let mut last_tick_end = run_start;

    while run_start.elapsed() < duration {
        for (idx, &id) in region_ids.iter().enumerate() {
            let tick_start = Instant::now();
            if first_tick_start.is_none() {
                first_tick_start = Some(tick_start);
            }
            let (_report, outcome) = manager.tick_region(id, &pool, &transport);
            let tick_end = Instant::now();
            last_tick_end = tick_end;
            assert_eq!(
                outcome,
                LifecycleOutcome::None,
                "region {id:?} triggered an unexpected merge/split under the soak's \
                 deliberately-far-below-threshold synthetic load"
            );
            per_region_samples_ms[idx]
                .push(tick_end.duration_since(tick_start).as_secs_f64() * 1000.0);
        }
        clock.await_next_tick();
    }

    let first_tick_start = first_tick_start.expect("at least one round must have run");
    let total_wall_clock_secs = last_tick_end.duration_since(first_tick_start).as_secs_f64();

    let mut per_region = Vec::with_capacity(REGION_COUNT);
    let mut tps_drift_ratio = Vec::with_capacity(REGION_COUNT);

    for (idx, &id) in region_ids.iter().enumerate() {
        let samples = &per_region_samples_ms[idx];
        let sample_count = samples.len() as f64;
        let measured_tps = sample_count / total_wall_clock_secs;
        let drift_ratio = measured_tps / TARGET_TPS - 1.0;
        assert!(
            drift_ratio.abs() <= DRIFT_TOLERANCE,
            "region {id:?} drifted {drift_ratio:.4} outside +/-{DRIFT_TOLERANCE} \
             (measured {measured_tps:.4} TPS over {total_wall_clock_secs:.2}s)"
        );
        tps_drift_ratio.push(drift_ratio);
        per_region.push(RegionTickHistogram::from_samples(
            id,
            samples,
            TARGET_TICK_BUDGET_MS,
        ));
    }

    let report = SoakReport {
        region_count: REGION_COUNT,
        target_tps: TARGET_TPS,
        target_tick_budget_ms: TARGET_TICK_BUDGET_MS,
        wall_clock_duration_secs: total_wall_clock_secs,
        per_region,
        tps_drift_ratio,
        zero_panics: true,
        status: SoakStatus::Pass,
    };

    let out_dir = workspace_target_dir().join("soak-report");
    fs::create_dir_all(&out_dir).expect("failed to create target/soak-report");
    let out_path = out_dir.join("region_soak_8x20tps.json");
    let json = serde_json::to_string_pretty(&report).expect("SoakReport must serialize");
    fs::write(&out_path, json).expect("failed to write soak report");
}

/// The workspace root's own `target/` directory -- Cargo always runs an integration
/// test binary with its process working directory set to the *crate's* manifest
/// directory, never the workspace root, so a bare relative `"target/..."` path would
/// land inside `crates/scheduler/target/` instead of the workspace-root `target/` this
/// blueprint's own Verification commands (run from the workspace root) expect. Derived
/// from `CARGO_MANIFEST_DIR` (a compile-time constant Cargo always sets to this crate's
/// own directory) rather than the process's runtime working directory, so it is stable
/// regardless of the caller's own shell cwd.
fn workspace_target_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("crates/scheduler is two directories below the workspace root")
        .join("target")
}
