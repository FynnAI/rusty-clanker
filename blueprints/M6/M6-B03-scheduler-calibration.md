# M6-B03 — Scheduler Calibration: Methodology, Pipeline & Governance

| Field | Content |
|---|---|
| ID | M6-B03 |
| Milestone | M6 — Scale & Optimization: Multi-Region Throughput |
| Prerequisites | M6-B02 (`rc_scheduler::metrics::MetricsRegistry` only — the one M6-B02 type this blueprint's body actually references, as the parameter type on the rarely-used `with_thresholds_and_metrics` sibling constructor, §E; this blueprint's own synthetic-sweep pipeline takes no dependency on M6-B02's `MetricsSnapshot`/`PoolUtilizationSample`/`LifecycleEvent`/`EdfViolation` types or its `write_snapshot_json` function — none is referenced anywhere in this blueprint's own Context, Deliverables, Acceptance tests, or Implementation steps). M0-B04 (`rc_scheduler::pool` — `RcWorkerPool`, `RcWorkerPoolConfig`, `PoolMode`, the exact ARCH-D19 grow/shrink pseudocode and its four named constants, restated in full below). M0-B06 (`rc_scheduler` region model — `ManagedRegion`, `RegionManager`, `GridCell`, `LifecycleOutcome`, the exact ARCH-D6 split/merge pseudocode and its pinned ratios/windows, restated in full below). M0-B08 (`xtask::tier_result`, `xtask::path_guard::{PROTECTED_PATHS, ChangesetType, check_paths, glob_match}`, the `Changeset-Type` trailer convention, reused unmodified). M3-B08 (`rc_test_harness::tick_cadence`'s pure-analysis-module shape, and `crates/testing/test-harness/src/bin/fixture_tick_writer.rs`'s "a small synthetic-fixture-writing binary in one crate, consumed by a pure-analysis module in another, connected only by a file — never a Cargo dependency edge" pattern, reused as this blueprint's own architecture below). |
| Implements | ARCH-D6 (merge/split thresholds and hysteresis windows — the calibration target); ARCH-D19 (pool grow/shrink backlog-EWMA thresholds — the calibration target); ARCH-D20 (EDF admission — folded in as a zero-violation *constraint* on ARCH-D19's own candidates, restated §C, since ARCH-D20 itself pins no numeric threshold); TEST-D45/D46/D49/D50/D52 (the governance-changeset process this blueprint's own calibrated-value promotion path must follow, restated in full, §H); `11-roadmap-milestones.md`'s M6 goal line ("replace `01`'s seed threshold defaults with calibrated values"). |
| Crates touched | `rc-scheduler` (`crates/scheduler/`) — additive only, two already-merged files extended, one new `[[bin]]` target. `rc-test-harness` (`crates/testing/test-harness/`) — additive only, one new module tree. `xtask` — additive only, one new verb, one new `PROTECTED_PATHS` row. `CONTRIBUTING.md` (extended — one new table row). |
| Estimated scope | L |

## Goal & Done definition

Build the calibration **methodology and pipeline** that replaces ARCH-D6's and ARCH-D19's seed-default numeric thresholds with measured values, and the **governance path** that lands a calibrated value as a reviewed change to `01-server-architecture.md`'s decision text — without waiting on the real multi-region, wall-clock-paced, EDF-admission-driven composition root, which no blueprint through M6-B02 has built (M6-B01 §B, M6-B02's own Scope-boundary note, both restated §A below). Concretely: (1) two small, additive, non-breaking extensions to already-merged `rc-scheduler` code that turn ARCH-D6's/ARCH-D19's currently-hardcoded threshold numbers into runtime-settable values, defaulting to today's exact pinned constants; (2) a new `calibration_sweep_point` binary in `rc-scheduler` that replays one scripted synthetic load series through the *real* `ManagedRegion`/`RcWorkerPool` hysteresis code under one candidate threshold set and writes one machine-readable series file; (3) a new, pure `rc_test_harness::calibration` analysis module — objective function, thrash detection, constrained-argmin selection, before/after comparison — that turns a set of those series files into a `CalibrationReport`, provably correct against constructed scenarios with an analytically known optimal answer; (4) an `xtask calibrate` verb wiring both together; (5) the governance path, restated exactly, for landing a calibrated value into `01-server-architecture.md`; (6) sensitivity documentation and a recalibration-trigger stance. Driving the pipeline against a real, live, many-region server is explicitly **not** built here — §A states that dependency precisely, the same honest boundary M6-B01/M6-B02 already drew for their own not-yet-existing dependencies.

Done when:

- [ ] `cargo build -p rc-scheduler -p rc-test-harness -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rc-test-harness -p xtask` on both OS legs.
- [ ] Every pre-existing M0-B04/B05/B06/M6-B02 test (`pool_*`, `tick_clock_drift`, `lifecycle_hysteresis`, `soak_8_regions_20tps`, `compute_waves_conflict_graph`, `access_compatibility`, `registration_validation`, `pipeline_ordering`, `sync_points`, `determinism`, the M6-B02 metrics suite) still passes, byte-for-byte unmodified — this blueprint's two `rc-scheduler` extensions are additive sibling constructors only, never a signature change to `RcWorkerPoolConfig`, `RcWorkerPool::new`/`with_config`, `RegionManager::new`/`new_with_metrics`, or `ManagedRegion::new`'s crate-internal call sites' *observable* defaults.
- [ ] `known_optimal_merge_split_threshold_is_recovered` and `known_optimal_pool_sizing_threshold_is_recovered` (the constructed-known-answer acceptance tests) both pass — the analysis pipeline finds the analytically-correct recommendation within the stated bounds for both calibration targets.
- [ ] `calibration_report_schema_round_trips` passes (serialize, deserialize, field-for-field equality).
- [ ] `governance_changeset_shape_dry_run` passes — proving a hypothetical changeset touching exactly `docs/planning/01-server-architecture.md` plus this blueprint's own new `rc-scheduler` default-value source lines is correctly recognized by `path_guard::check_paths` as requiring `ChangesetType::Governance`.
- [ ] `cargo run -p xtask -- calibrate --help` prints usage with zero panics; `cargo run -p xtask -- calibrate --mode synthetic-sweep --target merge-split --scenario split_ramp --out-dir <dir>` (no `--host`/`--port` — none exists yet, §A) runs the full sweep→analysis pipeline against the real `rc-scheduler` binary and writes a valid `CalibrationReport` to `<dir>/calibration-report.json` and `target/verify/calibrate.json`, exiting 0.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changeset (labeled per Constraints), and `path_guard_protects_01s_calibration_target_text` proves the new `PROTECTED_PATHS` row actually matches `docs/planning/01-server-architecture.md`.
- [ ] `CONTRIBUTING.md`'s TEST-D46 protected-path table includes a row for `docs/planning/01-server-architecture.md` matching the new `PROTECTED_PATHS` entry (§H item 6).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler -p rc-test-harness` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). No new nightly/release job is added by this blueprint — the real reference-host validation run (§A, §H) is a later, sibling blueprint's job once the composition root exists, mirroring M6-B01's identical "no new CI job — nothing real to gate yet" stance.

## Context (self-contained)

### §A — The dependency this blueprint does *not* wait on, restated

M6-B01 §B: "As of every blueprint this lineage has produced through M5-B10, `rusty-clanker-server`'s composition root has never been `rc-scheduler::RegionManager`-driven... Building it is real, separate work... this blueprint's own task is explicitly scoped apart from [it]." M6-B02's own Scope-boundary note: "[this blueprint] does not implement... the real-time EDF admission scheduler itself... a sibling, not-yet-written M6 blueprint's job." Both statements are still true at this blueprint's own drafting. **This blueprint is not that sibling blueprint either** — "the calibration methodology + execution blueprint" is real, bounded, buildable work distinct from "wire `RegionManager`/`RcExecutor`/`RcWorkerPool` into a live, network-facing, many-region server," and attempting both in one blueprint would violate the Blueprint Spec's own sizing rule (≤2 days, ≤800 lines).

This blueprint therefore builds and proves its **entire pipeline against the real `ManagedRegion`/`RcWorkerPool` hysteresis code, replayed synthetically** — no real server, no real network, no real bot swarm, no oracle — mirroring exactly the same "harness proven correct against synthetic/fake targets; the real end-to-end run is a later, separate green" split M6-B01 §H/Goal, M5-B10 §A.3, and M3-B08's `fixture_tick_writer`+`tick_cadence` split all already established for their own harnesses. Two concrete consequences, both restated as binding constraints rather than left implicit:

1. **ARCH-D20 compliance cannot be *measured* by this blueprint** — no real admission loop exists to violate or comply with. This blueprint's `CalibrationReport::edf_compliance` field is always `EdfCompliance::NotApplicable` for every report this blueprint's own pipeline can actually produce; `Verified`/`Violated` are schema states a *future* real-server-sourced report will populate once the composition-root sibling blueprint lands and can feed `MetricsSnapshot.edf_violation_count` into this same report shape. This is not a gap silently left open — §H states exactly what closes it.
2. **The real M6 acceptance-criteria numbers (200 bots/≥8 regions/15 minutes/documented reference host) are not produced by this blueprint.** This blueprint's own Done-state is the pipeline's correctness on constructed, known-answer synthetic workloads (Goal & Done, above) — identical in spirit to M6-B01's own "proves the harness... is itself correct... entirely against synthetic/fake targets" Done-state.

### §B — ARCH-D6 seed values, restated exactly (the merge/split calibration target)

From `01-server-architecture.md`'s ARCH-D6, and M0-B06's own concrete pin of the merge threshold ARCH-D6 leaves unnumbered:

| Parameter | Current seed value | Source |
|---|---|---|
| `split_threshold_ratio` | `0.9` (90% of `tick_budget_ms`) | ARCH-D6, verbatim |
| `split_sustained_ticks` | `40` (2 s at 50 ms/tick) | ARCH-D6, verbatim |
| `merge_threshold_ratio` | `0.1` (10% of `tick_budget_ms`, **summed** across the adjacent pair) | M0-B06's own pin, reusing ARCH-D19's already-established "quiet" cutoff |
| `merge_sustained_ticks` | `100` (5 s) | M0-B06's own pin |
| `ewma_alpha` | `0.2` | ARCH-D19, shared |

At the production `tick_budget_ms = 50.0` (ARCH-D7) these are the literal `45 ms`/`40 ticks` split and `5 ms`/`100 ticks` merge numbers ARCH-D6/M0-B06 already state. `01`'s own Open Questions: "ARCH-D6/D19's numeric thresholds... are seed defaults for the blueprint phase; final values require a reference server and load-testing harness to calibrate, not analysis alone" — this blueprint is that harness's *pipeline*; §A states what it does not yet have to run against for real.

### §C — ARCH-D19 seed values, restated exactly (the pool-sizing calibration target) — and ARCH-D20's role

From M0-B04, verbatim numeric pins:

| Parameter | Current seed value | Source |
|---|---|---|
| `backlog_ewma_alpha` | `0.2` (`BACKLOG_EWMA_ALPHA`) | ARCH-D19 |
| `backlog_grow_multiplier` | `2.0` (`BACKLOG_GROW_MULTIPLIER`) | ARCH-D19 |
| `grow_streak_threshold` | `3` samples (`GROW_STREAK_THRESHOLD`) | ARCH-D19 |
| `shrink_idle_streak_threshold` | `100` samples = 5 s (`SHRINK_IDLE_STREAK_THRESHOLD`) | ARCH-D19 |
| sample cadence | `50 ms` (`POOL_RESIZE_SAMPLE_INTERVAL`, derived, not itself swept — see Constraints) | ARCH-D19 |

**Not a calibration target of this blueprint:** ARCH-D18's `baseline`/`hard_cap` sizing (structural, host/cgroup-derived — M6's own acceptance criterion 1 treats the hard cap as a fixed ceiling to stay under, never a value to tune) and ARCH-D19's *second half* — the hot/quiet dispatch-**granularity** split (32-entity/chunk fine batches above 35 ms EWMA, one coalesced item below 5 ms). M0-B06 and M6-B02 both independently confirm, as of their own drafting, that this granularity mechanism is **not implemented by any landed blueprint** ("explicitly not implemented, stubbed, or delegated to any other M0 blueprint," M0-B06 Constraint (g); restated unchanged by M6-B02's own Scope-boundary note). This blueprint cannot calibrate a threshold pair (35 ms/5 ms, 32/128) that gates a code path which does not exist — §I and Open Questions state this precisely rather than inventing numbers for unbuilt code.

**ARCH-D20, restated:** "each region's deadline = `last_tick_start + 50ms`; RC-Executor's Injector serves overdue regions before on-time regions regardless of arrival order." This decision pins **zero numeric thresholds** — it is a pure ordering rule, not a tunable. There is therefore nothing for this blueprint to *sweep* for ARCH-D20. What this blueprint does instead: ARCH-D20 compliance (an admitted-region-never-jumps-a-strictly-earlier-deadline-region, M6-B02's own exact violation definition) becomes a **hard admissibility constraint** on every ARCH-D19 candidate this blueprint's pool-sizing objective function considers — a pool that shrinks too aggressively (a too-short `shrink_idle_streak_threshold`) removes exactly the worker capacity a newly-overdue region needs, which is the concrete mechanism by which an ARCH-D19 miscalibration would manifest as an ARCH-D20 violation. §F states the constraint's exact form; §A states why this blueprint can only apply it vacuously (`NotApplicable`) today.

### §D — Where the code lives, and why (mirrors M3-B08's `fixture_tick_writer`/`tick_cadence` split exactly)

Two crates, connected only by a file — **never** a new Cargo dependency edge:

- **`rc-scheduler`** gains a new `[[bin]]` target, `calibration_sweep_point`, that links directly against the crate's own real `ManagedRegion`/`RegionManager`/`RcWorkerPool` types (no new dependency — it is *part of* `rc-scheduler`) and, for one CLI-supplied candidate threshold set and one built-in scripted scenario, replays that scenario through the real hysteresis code and writes one series file.
- **`rc-test-harness`** gains a new, pure `calibration` module — no `rc-scheduler` dependency at all — that deserializes a set of those series files (a plain, restated JSON shape, field-for-field identical to what the bin target writes, exactly the discipline M6-B01 §B already used for its own future-contract types) and runs the analysis pipeline.
- **`xtask`** (already a dependent of `rc-test-harness` since M1-B06 — no new Cargo.toml line anywhere in this blueprint) gains `calibrate`, which shells out to `calibration_sweep_point` once per sweep-grid point (via `xshell`, mirroring `fetch_data.rs`'s own subprocess pattern) and calls `rc_test_harness::calibration`'s pure functions directly on the results.

**Why not a direct dependency instead of a file boundary:** `rc-scheduler` has never depended on, nor been depended on by, any `crates/testing/*` crate (M0-B04 through M6-B02's own dependency lists confirm this); introducing that edge for one calibration tool would be a new, unprecedented direction in the dependency graph for a benefit the file-boundary approach already delivers for free. The file boundary is also what keeps `rc_test_harness::calibration` equally able to analyze a *real*, future `MetricsSnapshot`-derived series (§A, §H) without caring which producer wrote it.

**Why the sweep replay needs the real hysteresis code, not a hand-rolled model:** a calibration tool that only validates its own model of ARCH-D6/D19's behavior, rather than the actual `ManagedRegion`/`RcWorkerPool` code paths, could recommend a threshold pair that the real implementation does not actually realize — the whole point of "known-optimal, tool must find it within bounds" (this blueprint's own acceptance-test mandate) is proving the *real* code's response curve is what gets measured, not a model of it.

### §E — Making ARCH-D6/D19's thresholds runtime-settable — additive, non-breaking

Both extensions are **new sibling constructors**, mirroring the exact pattern M6-B02 already established for this same file (`RegionManager::new_with_metrics` sitting alongside `RegionManager::new`) — no existing public signature's *shape* changes, so every pre-existing test (M0-B04's `pool_resize_hysteresis.rs`, M0-B06's `lifecycle_hysteresis.rs`, M6-B02's own suite) continues to compile and pass unmodified, exercising the identical default behavior it always has.

**Pool side** (`crates/scheduler/src/pool/worker_pool.rs`):

```rust
/// The four ARCH-D19 grow/shrink numbers, previously module constants only
/// (`GROW_STREAK_THRESHOLD`, `SHRINK_IDLE_STREAK_THRESHOLD`, `BACKLOG_EWMA_ALPHA`,
/// `BACKLOG_GROW_MULTIPLIER`), now also expressible as a runtime value. The four
/// constants are unchanged and are exactly `ResizeThresholds::default()`'s source —
/// restated, not duplicated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResizeThresholds {
    pub grow_streak_threshold: u32,
    pub shrink_idle_streak_threshold: u32,
    pub backlog_ewma_alpha: f64,
    pub backlog_grow_multiplier: f64,
}

impl Default for ResizeThresholds {
    /// `{ grow_streak_threshold: GROW_STREAK_THRESHOLD, shrink_idle_streak_threshold:
    /// SHRINK_IDLE_STREAK_THRESHOLD, backlog_ewma_alpha: BACKLOG_EWMA_ALPHA,
    /// backlog_grow_multiplier: BACKLOG_GROW_MULTIPLIER }` — the exact pre-existing
    /// pinned numbers, unchanged.
    fn default() -> Self;
}

impl RcWorkerPool {
    /// As `with_config`, additionally overriding the resize thresholds
    /// `sample_and_maybe_resize` reads (stored inside the already-existing
    /// `state: parking_lot::Mutex<PoolState>` — one new `PoolState` field, no new
    /// lock). `with_config(config)` is now defined as
    /// `with_resize_thresholds(config, ResizeThresholds::default())` — restated as an
    /// exact behavioral equivalence, not merely a claim, proven by
    /// `existing_hysteresis_behavior_unchanged_by_default_thresholds` (Acceptance tests).
    pub fn with_resize_thresholds(config: RcWorkerPoolConfig, thresholds: ResizeThresholds) -> Self;
}
```

`sample_and_maybe_resize`'s body (M0-B04's own pseudocode, Context) changes in exactly four places: every occurrence of `GROW_STREAK_THRESHOLD`, `SHRINK_IDLE_STREAK_THRESHOLD`, `BACKLOG_EWMA_ALPHA` (the `0.2 * raw + 0.8 * ewma` literals), and `2.0 * n as f64` (`BACKLOG_GROW_MULTIPLIER`) is replaced by a read of `pool_state.resize_thresholds.<field>` — already inside the lock that pseudocode already holds, zero new locking. The four module constants stay exported (`Default`'s source, and still directly useful documentation of "what production actually runs today").

**Region side** (`crates/scheduler/src/managed_region.rs`, `crates/scheduler/src/region_manager.rs`):

```rust
/// The five ARCH-D6 numbers M0-B06 hardcoded as method-body literals (`0.9`, `40`,
/// `0.1`, `100`) and the shared ARCH-D19 EWMA α, now also expressible as a runtime
/// value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HysteresisThresholds {
    pub split_threshold_ratio: f64,
    pub split_sustained_ticks: u32,
    pub merge_threshold_ratio: f64,
    pub merge_sustained_ticks: u32,
    pub ewma_alpha: f64,
}

impl Default for HysteresisThresholds {
    /// `{ 0.9, 40, 0.1, 100, 0.2 }` — §B's table, unchanged.
    fn default() -> Self;
}

impl<'e> RegionManager<'e> {
    /// As `new`, additionally overriding every `ManagedRegion` this manager spawns
    /// with `thresholds` instead of `HysteresisThresholds::default()`. `new(executor,
    /// tick_budget_ms)` is now defined as `with_thresholds(executor, tick_budget_ms,
    /// HysteresisThresholds::default())`; `new_with_metrics` (M6-B02) gains an
    /// identical `with_thresholds_and_metrics` sibling for the (rare, this
    /// blueprint's own synthetic-sweep path never needs it — §D, no MetricsRegistry
    /// involved) case both a non-default threshold set and metrics attribution are
    /// wanted together.
    pub fn with_thresholds(executor: &'e RcExecutor, tick_budget_ms: f64, thresholds: HysteresisThresholds) -> Self;
    pub fn with_thresholds_and_metrics(executor: &'e RcExecutor, tick_budget_ms: f64, thresholds: HysteresisThresholds, metrics: std::sync::Arc<crate::metrics::MetricsRegistry>) -> Self;
}
```

`ManagedRegion::new` (`pub(crate)`, three call sites — `spawn_region`, `execute_merge`, `execute_split`, all inside `region_manager.rs`) gains one new parameter, `thresholds: HysteresisThresholds`, threaded through from `RegionManager`'s own now-stored `thresholds` field; `split_threshold_ms()`/`merge_threshold_ms()` read `self.thresholds.split_threshold_ratio * self.tick_budget_ms`/`self.thresholds.merge_threshold_ratio * self.tick_budget_ms` instead of the literals `0.9`/`0.1`; `record_tick_duration`/`update_merge_candidate` compare against `self.thresholds.split_sustained_ticks`/`merge_sustained_ticks` instead of `40`/`100`; the EWMA update reads `self.thresholds.ewma_alpha` instead of the literal `0.2`. Because `ManagedRegion::new` is crate-private, this is an ordinary internal signature change with exactly three, in-crate, mechanically-updated call sites — not a public API break.

### §F — Objective function, admissibility, and thrash — defined once, precisely, per target

Both targets share one shape: replay a **scripted synthetic series** (a fixed sample-by-sample script with one, at most, embedded "true regime change" marker) through the real code under one candidate threshold set, collect the resulting event stream, and score it.

**Merge/split** (`MergeSplitParams` = `HysteresisThresholds`'s five fields):

```
series := (scripted tick-duration samples, ticks 0..N)
regime_change_tick: Option<u64>   // None for a "should stay flat" baseline series
expected_reaction: Split | Merged  // only meaningful when regime_change_tick is Some
observed := every LifecycleOutcome::{Split, Merged} RegionManager::record_synthetic_tick
            returned while replaying `series` under this candidate, tick-stamped

false_trigger_count := count of `observed` events strictly before `regime_change_tick`
                        (or, if regime_change_tick is None, every observed event at all)
reaction_ticks :=
    if regime_change_tick is None: None   // nothing scripted to react to; cost is never
                                            // compared for this point (below)
    else: match first `observed` event at or after `regime_change_tick`:
        Some(e) if e.direction == expected_reaction -> Some(e.tick - regime_change_tick)
        Some(_) | None                              -> None   // wrong direction, or never reacted
thrash_event_count := count_lifecycle_thrash(observed, MERGE_SPLIT_THRASH_WINDOW_TICKS)
    // MERGE_SPLIT_THRASH_WINDOW_TICKS = 400 (seed default — 4x the longer, 100-tick,
    // merge window, giving comfortable margin against a legitimate split-then-later-
    // merge-for-unrelated-reasons pair being misclassified as thrash)
    // a thrash event = any `observed[i]` whose direction differs from `observed[i-1]`'s
    // AND falls within `window` ticks of it (the first event can never be a thrash event)

admissible := false_trigger_count == 0
           && thrash_event_count == 0
           && (regime_change_tick.is_none() || reaction_ticks.is_some())
cost := reaction_ticks   // only defined, and only compared, for admissible points;
                          // regime_change_tick == None points are never cost-compared,
                          // only used to prove false_trigger_count == 0 baselines
```

**Pool sizing** (`PoolSizingParams` = `ResizeThresholds`'s four fields, `backlog_ewma_alpha` swept jointly with `backlog_grow_multiplier`):

```
series := (scripted per-sample backlog-inducing script: which real jobs are pushed/
           released, sample by sample, via the block-workers-and-push-jobs technique
           M0-B04's own pool_resize_hysteresis.rs tests already establish)
regime_change_sample: Option<u64>
expected_reaction: Grow | Shrink
observed := worker_count() deltas across consecutive sample_and_maybe_resize() calls
            while replaying `series` under this candidate, sample-index-stamped

false_trigger_count, reaction_samples, thrash_event_count, admissible, cost
    -- identical definitions to merge/split, substituting "sample" for "tick" and
       POOL_THRASH_WINDOW_SAMPLES = 40 (seed default -- well above
       GROW_STREAK_THRESHOLD's own 3-sample reaction window so a legitimate fast grow
       is never itself misclassified as thrash, and well under
       SHRINK_IDLE_STREAK_THRESHOLD's 100-sample window)
edf_compliance := EdfCompliance::NotApplicable   -- always, for a synthetic-sweep-
    sourced point (§A); the field exists on every SweepResult so a future
    real-server-sourced point can report Verified/Violated in the identical shape
    (§H) without a schema change
```

**Selection (`select_recommended`, pure, deterministic):** among admissible points, `argmin(cost)`; ties broken (1) by smallest summed absolute relative deviation from the current §B/§C seed values (prefer minimal change over an equally-good alternative — avoids needless churn, matching CLAUDE.md's "decisions are never made to save work" read in reverse: don't spend a change on zero net benefit), then (2) by ascending field-declaration-order lexicographic comparison of the param struct (final, fully deterministic tie-break — the same discipline `largest_connectivity_cut`, M0-B06, and `resolve_load_multiplier`, M6-B01, already established for this corpus). **Verdict:** `Recommend` iff the argmin's params differ from the seed by more than `1e-9` in any field **and** its `cost` improves on the seed point's own `cost` (when the seed point is itself admissible and present in the sweep — always true, §G's sweep-spec validation requires it) by at least `MIN_MEANINGFUL_IMPROVEMENT_RATIO = 0.05` (5% — a seed default, the same "concrete number now, calibration-pending" status every other unpinned threshold in this corpus carries); `NoChangeNeeded` if the seed point is itself the argmin, or improves on it by less than that ratio; `Inconclusive` if **no** point in the sweep is admissible at all — an honest, reportable failure state, never a silently-defaulted recommendation.

### §G — The sweep spec and the analysis pipeline

```rust
// rc_test_harness::calibration — no rc-scheduler dependency (§D)

pub const MERGE_SPLIT_THRASH_WINDOW_TICKS: u64 = 400;
pub const POOL_THRASH_WINDOW_SAMPLES: u64 = 40;
pub const MIN_MEANINGFUL_IMPROVEMENT_RATIO: f64 = 0.05;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct MergeSplitParams { pub split_threshold_ratio: f64, pub split_sustained_ticks: u32, pub merge_threshold_ratio: f64, pub merge_sustained_ticks: u32, pub ewma_alpha: f64 }
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct PoolSizingParams { pub grow_streak_threshold: u32, pub shrink_idle_streak_threshold: u32, pub backlog_ewma_alpha: f64, pub backlog_grow_multiplier: f64 }

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleDirection { Split, Merged }
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeDirection { Grow, Shrink }

/// One `calibration_sweep_point` output file's exact shape (§D) — the merge/split variant.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MergeSplitSweepSeries {
    pub params: MergeSplitParams,
    pub scenario_name: String,
    pub regime_change_tick: Option<u64>,
    pub expected_reaction: Option<LifecycleDirection>,  // required iff regime_change_tick is Some
    pub events: Vec<(u64, LifecycleDirection)>,          // (tick, direction), ascending tick order
}

/// The pool-sizing variant.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PoolSizingSweepSeries {
    pub params: PoolSizingParams,
    pub scenario_name: String,
    pub regime_change_sample: Option<u64>,
    pub expected_reaction: Option<ResizeDirection>,
    pub events: Vec<(u64, ResizeDirection)>,             // (sample_index, direction)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum EdfCompliance { NotApplicable, Verified, Violated }
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum CalibrationVerdict { Recommend, NoChangeNeeded, Inconclusive }
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum CalibrationTarget { MergeSplit, PoolSizing }

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct SweepResult<P> { pub params: P, pub scenario_name: String, pub false_trigger_count: u32, pub reaction: Option<f64>, pub thrash_event_count: u32, pub admissible: bool, pub cost: Option<f64>, pub edf_compliance: EdfCompliance }

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct CalibrationReport<P> {
    pub schema_version: u32,          // = 1
    pub generated_at_unix_ms: u64,
    pub target: CalibrationTarget,
    pub seed_values: P,
    pub sweep: Vec<SweepResult<P>>,
    pub recommended: Option<P>,       // None iff verdict == Inconclusive
    pub verdict: CalibrationVerdict,
    pub notes: Vec<String>,
}

/// Pure: `count_lifecycle_thrash`/`count_resize_thrash` — the shared thrash rule
/// (§F) applied to either event-direction-pair type via one generic helper keyed on
/// `(tick_or_sample: u64, direction: impl PartialEq)`.
pub fn count_thrash<D: PartialEq + Copy>(events: &[(u64, D)], window: u64) -> u32;

/// Pure: scores one `MergeSplitSweepSeries` into a `SweepResult<MergeSplitParams>`
/// per §F's exact merge/split definitions.
pub fn score_merge_split(series: &MergeSplitSweepSeries) -> SweepResult<MergeSplitParams>;
/// Pure: as above, for `PoolSizingSweepSeries`.
pub fn score_pool_sizing(series: &PoolSizingSweepSeries) -> SweepResult<PoolSizingParams>;

/// Pure: `true` iff some `SweepResult` in `sweep` has `params == seed_values` (by
/// `PartialEq`) — the precondition `select_recommended` requires. A caller (the
/// `xtask calibrate` orchestrator) calls this immediately after grid construction,
/// before ever invoking `calibration_sweep_point`, so a malformed sweep spec fails
/// fast with an actionable message rather than surfacing as a `select_recommended`
/// panic deep in the pipeline.
pub fn validate_sweep_includes_seed<P: PartialEq>(seed_values: &P, sweep_params: &[P]) -> bool;

/// Pure, deterministic (§F's exact tie-break order): builds the full
/// `CalibrationReport` from a set of already-scored `SweepResult`s plus the current
/// seed values. Panics if `seed_values` (by field equality) is not itself present
/// among `sweep`'s own `params` — `validate_sweep_includes_seed` (above) is what a
/// caller checks first to turn that panic into a normal, reported error instead.
pub fn select_recommended<P: PartialEq + Copy + serde::Serialize>(target: CalibrationTarget, seed_values: P, sweep: Vec<SweepResult<P>>, now_unix_ms: u64) -> CalibrationReport<P>;

/// One sweep-grid candidate, evenly spaced across `[seed * (1.0 - spread), seed *
/// (1.0 + spread)]` for a float field, or the nearest-integer equivalent for a u32
/// field, always including the seed value itself as one grid point (this function's
/// own binding postcondition — Acceptance tests prove it). `steps` is the total grid
/// size including the seed point.
pub fn build_ratio_grid(seed: f64, spread: f64, steps: u32) -> Vec<f64>;
pub fn build_count_grid(seed: u32, spread: f64, steps: u32) -> Vec<u32>;
```

**Built-in scenarios** (fixed, hand-constructed, analytically known-answer — the acceptance-test corpus and this blueprint's own worked examples; `calibration_sweep_point` §H below is what actually replays them):

| Name | Target | Script (informal) | `regime_change_*` | `expected_reaction` |
|---|---|---|---|---|
| `split_ramp` | MergeSplit | ticks 0..199 at 20 ms (a two-cell region); ticks 200.. at 48 ms (comfortably above every grid candidate's own threshold, so it triggers on the *first* post-200 sample the candidate's own sustained-tick window allows) | `Some(200)` | `Split` |
| `stable_no_trigger` | MergeSplit | all 500 ticks at 25 ms (comfortably under every grid candidate's threshold) | `None` | — |
| `grow_ramp` | PoolSizing | `baseline: 2, hard_cap: 6`; both workers blocked and 200 no-op jobs queued from sample 0 (comfortably above every grid candidate's own `backlog_grow_multiplier × 2`) | `Some(0)` | `Grow` |
| `stable_no_resize` | PoolSizing | `baseline: 2, hard_cap: 6`; no blocking, no jobs, ever | `None` | — |

Because `split_ramp`/`grow_ramp` push their scripted value far past *every* grid candidate's own threshold, each candidate's `reaction` is provably, analytically, exactly its own `split_sustained_ticks`/`grow_streak_threshold` value — the fastest-reacting zero-false-trigger candidate is therefore always the grid's smallest sustained-tick/streak-threshold entry, a closed-form known answer this blueprint's acceptance tests assert against directly, never approximated.

### §H — Governance path, restated exactly

Landing a calibrated value is a **dedicated governance changeset**, never folded into an implementation changeset — restating TEST-D45/D46/D49/D50/D52 as they apply here, not a new process:

1. **Run.** `cargo run -p xtask -- calibrate --mode synthetic-sweep --target <merge-split|pool-sizing> --scenario <name> --out-dir <dir>` produces `<dir>/calibration-report.json` (this blueprint's own Tier-1-tested pipeline, §D–§G) and, once the composition-root sibling blueprint exists, an equivalent `--mode real-sweep --host <addr> --port <port>` run against M6-B01's real bot-swarm scenarios feeds real `MetricsSnapshot` series through the identical `rc_test_harness::calibration` scoring functions (§A — not built by this blueprint, the schema is ready for it, §G's `EdfCompliance::{Verified, Violated}` variants exist for exactly this future producer).
2. **Verdict gate.** Only a `CalibrationVerdict::Recommend` report proceeds; `NoChangeNeeded`/`Inconclusive` reports are attached to the tracked issue/PR discussion as evidence the current seed value already stands, never silently discarded (TEST-D51's "never simply deleted... always tracked" spirit, applied to a calibration attempt rather than a quarantined test).
3. **Changeset shape.** A single commit (or PR) whose HEAD carries `Changeset-Type: governance`, touching exactly: `docs/planning/01-server-architecture.md` (ARCH-D6's or ARCH-D19's decision-text numbers, rewritten to state the new values as current truth — **no changelog, no "previously X" framing**, per CLAUDE.md's binding "docs are current-state only" rule; the rationale cell may carry one sentence, e.g. "Calibrated against the M6 reference-host load profile, M6-B03") and, in the identical changeset, the corresponding `rc-scheduler` `Default` impl (`ResizeThresholds::default()` or `HysteresisThresholds::default()`, §E) updated to match — these two edits are a single, atomic, mechanically-paired number change, not two independent ones. The `CalibrationReport` JSON itself is attached to the PR/commit as supporting evidence (a build artifact or a linked file), **not** embedded into `01`'s own prose.
4. **Why `rc-scheduler`'s source is *not* a new `PROTECTED_PATHS` row.** TEST-D46 protects tests, fixtures, verification tooling, and *SLO/budget tables* — `ResizeThresholds::default()`/`HysteresisThresholds::default()` are ordinary production constants implementing an already-authorized planning decision, the identical category M0-B06 itself already wrote as plain Rust literals in an ordinary implementation changeset. Only the planning-document decision *text* needs governance protection; this blueprint's own new `PROTECTED_PATHS` row (below) reflects exactly that line, no more.
5. **New protected path.** `docs/planning/01-server-architecture.md` was not previously protected (only `docs/planning/09-testing-quality.md`, M0-B08's row 13, was) — this blueprint adds one new row, mirroring M0-B08's own row-14 precedent for the exact same kind of gap:

   ```rust
   ProtectedPath { pattern: "docs/planning/01-server-architecture.md", reason: "ARCH-D6/D19/D20 numeric-threshold decision text (M6-B03 calibration governance target)" },
   ```

6. **`CONTRIBUTING.md` update.** M0-B08's own Done-when checklist commits to keeping `CONTRIBUTING.md`'s documented protected-path list complete and in sync with `xtask::path_guard::PROTECTED_PATHS` — this blueprint's own new row (item 5, above) inherits that same standing obligation. Append one row to `CONTRIBUTING.md`'s existing TEST-D46 protected-path table for `docs/planning/01-server-architecture.md`, reason "ARCH-D6/D19/D20 numeric-threshold decision text (M6-B03 calibration governance target)" — mirroring, word for word, the identical obligation this milestone's sibling M6-B04 discharges for its own new row.
7. **Verification.** The changeset's own required tier is Tier 1 (this row's presence is proven by `path_guard_protects_01s_calibration_target_text`, Acceptance tests) plus a from-clean-checkout re-run of this blueprint's own `xtask calibrate` invocation reproducing the identical `CalibrationVerdict` (TEST-D50/D52's "CI is sole authority... verifier agent re-runs, never trusts the implementer's own report" — restated as this specific changeset's own re-verification step, "the report attached is reproducible, not asserted").

### §I — Sensitivity: which parameters interact

- **ARCH-D6 vs ARCH-D19 (calibration order).** A region splitting sooner (a lower `split_threshold_ratio`) creates more, smaller regions sooner, changing the *aggregate* backlog shape ARCH-D19's own grow/shrink logic reacts to — sweeping both independently, each against the *other's current seed value*, risks each converging against a load shape the other's own soon-to-change behavior will invalidate. **Resolution, binding on any real (non-synthetic) calibration run:** sweep ARCH-D6 first, holding ARCH-D19 at its current seed; only once ARCH-D6's governance changeset has landed does an ARCH-D19 sweep run against the *newly*-calibrated ARCH-D6 thresholds. A fully joint/coupled sweep is a heavier future refinement, out of this blueprint's scope (Open Questions).
- **ARCH-D19 vs the near-zero-CPU threshold (M6-B02's `NEAR_ZERO_CPU_THRESHOLD_RATIO`/`NEAR_ZERO_SUSTAINED_TICKS`).** A pool that shrinks eagerly changes which regions stay measurably "quiet" long enough to cross M6-B02's own near-zero window; this blueprint does not sweep M6-B02's thresholds (they are that blueprint's own pin, not this one's target) but flags the interaction so a future joint pass considers it.
- **ARCH-D19's unimplemented hot/quiet batch-granularity half vs its own grow/shrink half.** Once that mechanism lands (§C), the batch-size split changes how much backlog one hot region contributes per resize sample, directly changing the grow trigger's sensitivity — **both halves must be recalibrated together** the first time that mechanism exists, never the grow/shrink half alone against a batch-granularity value that predates it.
- **ARCH-D19 shrink threshold vs ARCH-D20 compliance.** Restated from §C: an aggressively short `shrink_idle_streak_threshold` is the concrete mechanism by which a pool-sizing miscalibration becomes an EDF violation (removed capacity exactly when a region goes overdue) — this is *why* §F folds ARCH-D20 compliance in as a constraint on ARCH-D19's own candidates rather than treating the two as unrelated.

### §J — Recalibration-cadence stance

Never a fixed calendar cadence — mirrors `12`'s WS-D4 toolchain-bump discipline ("bumped deliberately... never silently") and `14`'s PERF-D64 ("recalibrated as an explicit seed default each blueprint-phase cycle rather than pinned once and left to age"), both of which tie a number's revisit to a concrete triggering *event*. Triggers:

1. The reference-host specification (a sibling M6 blueprint's own deliverable) changes.
2. ARCH-D18's baseline/hard-cap sizing logic changes (a PERF-D57 cgroup-detection revision, or a reference-host hardware-class change) — ARCH-D19's ratios are defined relative to pool size.
3. ARCH-D8's stage set or `14`'s PERF-D59 per-stage budget table changes — every ratio here (`0.9`, `0.1`, `2.0×`) is relative to `tick_budget_ms`.
4. The ARCH-D19 hot/quiet batch-granularity mechanism (§C) lands for the first time — a mandatory *joint* recalibration of both ARCH-D19 halves (§I).
5. A Tier-3 SLO regression (`09`'s TEST-D32, `14`'s PERF-D60) whose root cause traces to scheduler thrash or a missed admission deadline — on-demand, from monitoring, never scheduled.
6. `13-cluster-architecture.md`'s cluster mode (M7) reuses these thresholds under `NetworkTransport`'s different latency profile — cluster mode may need its own calibration pass, explicitly out of this blueprint's monolithic-only scope (M6's own boundary note).

### Claims to verify (TEST-D57)

- None.

## Deliverables

### `crates/scheduler/src/pool/worker_pool.rs` (modify — additive; `ResizeThresholds`, `with_resize_thresholds`; every existing item's shape unchanged)

Exactly §E's `ResizeThresholds`/`Default`/`RcWorkerPool::with_resize_thresholds` signatures; `PoolState` gains one new field `resize_thresholds: ResizeThresholds`; `sample_and_maybe_resize`'s body reads from it at the four sites named in §E.

### `crates/scheduler/src/managed_region.rs`, `crates/scheduler/src/region_manager.rs` (modify — additive; `HysteresisThresholds`, `RegionManager::with_thresholds`/`with_thresholds_and_metrics`; every existing item's shape unchanged)

Exactly §E's `HysteresisThresholds`/`Default`/`RegionManager` sibling-constructor signatures; `ManagedRegion` gains one new private field `thresholds: HysteresisThresholds`, threaded through `ManagedRegion::new`'s three in-crate call sites; `split_threshold_ms`/`merge_threshold_ms`/`record_tick_duration`/`update_merge_candidate` read from it per §E.

### `crates/scheduler/Cargo.toml` (modify — one new `[[bin]]`)

```toml
[[bin]]
name = "calibration_sweep_point"
path = "src/bin/calibration_sweep_point.rs"
```

(No new `[dependencies]` line — `serde`/`serde_json`/`clap` are already present, `serde_json` promoted to a normal dependency by M6-B02; `clap` is added here as `rc-scheduler`'s own first CLI-parsing need, already workspace-pinned at `4.6.6` per `xtask`'s own `12`-fixed version, no new `[workspace.dependencies]` entry.)

```toml
[dependencies]
clap = { version = "4.6.6", features = ["derive"] }
```

### `crates/scheduler/src/bin/calibration_sweep_point.rs` (new)

```rust
/// `calibration_sweep_point --target {merge-split|pool-sizing} --scenario <name>
/// --params <json> --out <path>`. `--params` deserializes into
/// `rc_scheduler`-local `MergeSplitParams`/`PoolSizingParams` mirror structs
/// (field-for-field identical to `rc_test_harness::calibration`'s own — §D, no
/// crate dependency, restated locally). `--scenario` selects one of §G's built-in
/// scripts (`split_ramp`/`stable_no_trigger` for `merge-split`,
/// `grow_ramp`/`stable_no_resize` for `pool-sizing`) baked into this binary as
/// literal Rust data — never read from a file (this blueprint's own scenarios are
/// fixed, code-native, TEST-D42-style data). Replays the script through a real
/// `RegionManager::with_thresholds`/`RcWorkerPool::with_resize_thresholds` instance
/// (never real sleeping — `record_synthetic_tick`/`sample_and_maybe_resize` calls
/// only, `PoolMode::Elastic { auto_sample: false }`), collects the resulting
/// `LifecycleOutcome`/`worker_count()`-delta event stream tick/sample-stamped, and
/// writes one `MergeSplitSweepSeries`/`PoolSizingSweepSeries`-shaped JSON file
/// (§G's exact field names, restated locally, no `rc-test-harness` dependency) to
/// `--out`. Exits 2 on an unknown `--scenario`/malformed `--params`, 0 otherwise —
/// never panics on bad CLI input.
fn main() -> std::process::ExitCode;
```

### `crates/testing/test-harness/src/lib.rs` (modify — one new `pub mod` line)

```rust
pub mod calibration;
```

### `crates/testing/test-harness/src/calibration/mod.rs`, `series.rs`, `thrash.rs`, `report.rs` (new)

Exactly §G's types/constants/functions, split: `series.rs` (`MergeSplitParams`/`PoolSizingParams`/`LifecycleDirection`/`ResizeDirection`/`MergeSplitSweepSeries`/`PoolSizingSweepSeries`), `thrash.rs` (`count_thrash`, the two window constants), `report.rs` (`EdfCompliance`/`CalibrationVerdict`/`CalibrationTarget`/`SweepResult`/`CalibrationReport`/`score_merge_split`/`score_pool_sizing`/`select_recommended`/`validate_sweep_includes_seed`/`build_ratio_grid`/`build_count_grid`/`MIN_MEANINGFUL_IMPROVEMENT_RATIO`), re-exported at `mod.rs`.

### `xtask/src/lib.rs` (modify — one new `pub mod` line)

```rust
pub mod calibrate;
```

### `xtask/src/calibrate.rs` (new)

```rust
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CalibrateMode { SyntheticSweep }   // `RealSweep` is a future sibling
                                             // blueprint's own addition once the
                                             // composition root exists (§A, §H) —
                                             // not a variant this blueprint defines
                                             // speculatively.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CalibrateTarget { MergeSplit, PoolSizing }

pub struct CalibrateArgs {
    pub mode: CalibrateMode,
    pub target: CalibrateTarget,
    pub scenario: String,
    pub out_dir: std::path::PathBuf,
    /// Grid spread/step count (§G's `build_ratio_grid`/`build_count_grid`) — seed
    /// defaults `spread: 0.5, steps: 7` if omitted.
    pub grid_spread: Option<f64>,
    pub grid_steps: Option<u32>,
}

/// `SyntheticSweep`: builds the grid (§G), shells out to `calibration_sweep_point`
/// once per grid point (via `xshell`, `cargo run -p rc-scheduler --bin
/// calibration_sweep_point --release -- ...` — `--release` so a 7-point ×
/// 100+-sample-per-point grid stays comfortably inside Tier 1's <10 min budget),
/// deserializes each resulting series file, scores it (`rc_test_harness::calibration
/// ::score_*`), calls `select_recommended`, writes `<out_dir>/calibration-report.json`
/// and `target/verify/calibrate.json` (TEST-D40). Never touches network, never
/// spawns `rusty-clanker-server`, never requires the oracle.
pub fn run(args: &CalibrateArgs) -> std::process::ExitCode;
```

### `xtask/src/main.rs` (modify — one new `Command` variant, dispatched identically to every prior addition)

```rust
/// This blueprint's M6-B03 calibration verb.
Calibrate {
    #[arg(long, value_enum)] mode: calibrate::CalibrateMode,
    #[arg(long, value_enum)] target: calibrate::CalibrateTarget,
    #[arg(long)] scenario: String,
    #[arg(long)] out_dir: std::path::PathBuf,
    #[arg(long)] grid_spread: Option<f64>,
    #[arg(long)] grid_steps: Option<u32>,
},
```

### `xtask/src/path_guard.rs` (modify — one new `PROTECTED_PATHS` row, §H item 5)

### `CONTRIBUTING.md` (modify — append one row to the existing TEST-D46 protected-path table, §H item 6)

Add a row for `docs/planning/01-server-architecture.md`, reason "ARCH-D6/D19/D20 numeric-threshold decision text (M6-B03 calibration governance target)" — the same table `M0-B08`'s own Done-when already commits to documenting in full.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** every test file below, plus `crates/scheduler/src/pool/worker_pool.rs`'s/`managed_region.rs`'s/`region_manager.rs`'s new items, `crates/scheduler/src/bin/calibration_sweep_point.rs`, and `crates/testing/test-harness/src/calibration/{mod.rs,series.rs,thrash.rs,report.rs}` with function bodies `todo!()`-stubbed (fields/derives/doc comments/constant *values* stay exact), plus both `Cargo.toml` diffs and `xtask`'s `lib.rs`/`main.rs`/`path_guard.rs` diffs, are committed first (`Changeset-Type: test-authoring`). The implementation changeset (`Changeset-Type: governance` — Constraints) fills in real bodies only; it must not modify any file under `crates/*/tests/`.

### `crates/scheduler/tests/resize_thresholds_default_matches_constants.rs` (new)

1. `default_resize_thresholds_matches_pinned_constants` — `ResizeThresholds::default() == ResizeThresholds { grow_streak_threshold: GROW_STREAK_THRESHOLD, shrink_idle_streak_threshold: SHRINK_IDLE_STREAK_THRESHOLD, backlog_ewma_alpha: BACKLOG_EWMA_ALPHA, backlog_grow_multiplier: BACKLOG_GROW_MULTIPLIER }`.
2. `existing_hysteresis_behavior_unchanged_by_default_thresholds` — re-runs M0-B04's own `grow_fires_on_third_not_second_consecutive_sample`/`shrink_fires_on_100th_not_99th_consecutive_idle_sample` scenarios (Context restated, not imported — this file constructs its own pool), once via `RcWorkerPool::with_config(cfg)` and once via `RcWorkerPool::with_resize_thresholds(cfg, ResizeThresholds::default())`, asserting byte-identical `worker_count()` trajectories at every sampled step — the exact-equivalence proof §E's doc comment promises.
3. `nondefault_grow_streak_threshold_changes_the_reaction_tick` — `with_resize_thresholds(cfg, ResizeThresholds { grow_streak_threshold: 1, ..Default::default() })`; block workers, push backlog; `sample_and_maybe_resize()` once; assert `worker_count()` already grew (vs. the default's 3-sample requirement) — proves the override actually reaches `sample_and_maybe_resize`'s body, not merely stored inertly.

### `crates/scheduler/tests/hysteresis_thresholds_default_matches_pinned.rs` (new)

4. `default_hysteresis_thresholds_matches_b06_pins` — `HysteresisThresholds::default() == HysteresisThresholds { split_threshold_ratio: 0.9, split_sustained_ticks: 40, merge_threshold_ratio: 0.1, merge_sustained_ticks: 100, ewma_alpha: 0.2 }`.
5. `existing_lifecycle_hysteresis_unchanged_by_default_thresholds` — re-runs M0-B06's own `split_triggers_at_exactly_40th_consecutive_over_threshold_tick` scenario once via `RegionManager::new(&executor, 50.0)` and once via `RegionManager::with_thresholds(&executor, 50.0, HysteresisThresholds::default())`, asserting identical `LifecycleOutcome` sequences.
6. `nondefault_split_ratio_changes_the_trigger_point` — `with_thresholds(&executor, 50.0, HysteresisThresholds { split_threshold_ratio: 0.5, ..Default::default() })`; feed 40 samples of `30.0` (below the *default* 45 ms threshold, above a 0.5×50=25 ms one); assert `LifecycleOutcome::Split` fires on the 40th sample — proves the override reaches `split_threshold_ms()`.

### `crates/scheduler/tests/calibration_sweep_point_bin.rs` (new — Tier 1, real subprocess, mirrors M3-B08's `fixture_tick_writer_self_test.rs`)

7. `sweep_point_split_ramp_reaction_matches_sustained_ticks_exactly` — for each of `split_sustained_ticks ∈ {20, 40, 60}` (holding every other param at its default), runs `calibration_sweep_point --target merge-split --scenario split_ramp --params <json> --out <tmp>` as a real subprocess (`xshell`, mirroring `fixture_tick_writer_self_test.rs`'s own established pattern), parses the resulting JSON, and asserts `events == [(200 + sustained_ticks, Split)]` exactly — the closed-form known answer §G states.
8. `sweep_point_grow_ramp_reaction_matches_streak_threshold_exactly` — the pool-sizing analog, for `grow_streak_threshold ∈ {1, 3, 8}`, asserting `events == [(streak_threshold - 1, Grow)]` (0-indexed sample count to the triggering sample).
9. `sweep_point_stable_scenarios_produce_zero_events` — `stable_no_trigger`/`stable_no_resize` under the default params both produce `events == []`.
10. `sweep_point_rejects_unknown_scenario_with_exit_code_2` — `--scenario bogus` exits `2`, prints an actionable message, writes no output file.

### `crates/testing/test-harness/tests/calibration_thrash.rs` (new — pure, no subprocess)

11. `count_thrash_detects_direction_reversal_within_window` — `[(0, Split), (10, Merged)]`, `window: 20` → `1`. `[(0, Split), (500, Merged)]`, `window: 20` → `0` (outside the window).
12. `count_thrash_same_direction_never_counts` — `[(0, Split), (10, Split), (20, Split)]`, any window → `0`.
13. `count_thrash_first_event_never_counts` — `[(0, Split)]`, any window → `0`.

### `crates/testing/test-harness/tests/calibration_scoring.rs` (new — pure)

14. `score_merge_split_computes_reaction_and_admissibility` — a hand-built `MergeSplitSweepSeries { regime_change_tick: Some(200), expected_reaction: Some(Split), events: vec![(240, Split)], .. }` → `reaction == Some(40.0)`, `false_trigger_count == 0`, `thrash_event_count == 0`, `admissible == true`.
15. `score_merge_split_flags_pre_regime_event_as_false_trigger` — `events: vec![(50, Split), (240, Split)]`, same `regime_change_tick` → `false_trigger_count == 1`, `admissible == false`.
16. `score_merge_split_none_regime_change_requires_zero_events_for_admissibility` — `regime_change_tick: None, events: vec![]` → `admissible == true`, `cost == None` (never compared); `events: vec![(10, Split)]` → `false_trigger_count == 1`, `admissible == false`.
17. `score_pool_sizing_mirrors_merge_split_shape` — the `PoolSizingSweepSeries` analog of test 14.

### `crates/testing/test-harness/tests/calibration_report.rs` (new — pure; **the acceptance-test mandate's central proof**)

18. `known_optimal_merge_split_threshold_is_recovered` — build five `MergeSplitParams` candidates via `build_count_grid(40, 0.5, 5)` for `split_sustained_ticks` (holding every other field at `HysteresisThresholds::default()`'s values), construct each candidate's `SweepResult` **analytically** (per §G's closed-form: `events == [(200 + candidate.split_sustained_ticks, Split)]`, `false_trigger_count: 0`, `thrash_event_count: 0`) rather than via a real subprocess (this test's own job is the *analysis* pipeline, not the sweep-point binary — test 7 already covers that), call `select_recommended`; assert `verdict == Recommend`, `recommended.unwrap().split_sustained_ticks == 20` (the grid's smallest candidate — provably optimal, since every candidate is admissible and cost is monotonic in `split_sustained_ticks` by the scenario's own construction) — **the "tool must find it within bounds" acceptance criterion, proven exactly, not approximately**.
19. `known_optimal_pool_sizing_threshold_is_recovered` — the `grow_streak_threshold` analog of test 18, same structure, asserting `recommended.unwrap().grow_streak_threshold` equals the grid's smallest candidate.
20. `seed_already_optimal_yields_no_change_needed` — a grid whose seed point is itself the unique cost-minimizer (constructed so every non-seed candidate scores strictly worse) → `verdict == NoChangeNeeded`, `recommended.is_none()` is **false** (recommended still reports the seed's own params, per §F — `NoChangeNeeded` is a verdict on *whether to act*, not an absence of a value).
21. `all_candidates_inadmissible_yields_inconclusive` — every candidate in a hand-built grid has `false_trigger_count > 0` → `verdict == Inconclusive`, `recommended.is_none()`.
22. `tie_break_prefers_minimal_deviation_from_seed` — two candidates tie on `cost` exactly; the one numerically closer to the seed value is selected — proves §F's tie-break order, not just its existence.
23. `calibration_report_schema_round_trips` — `serde_json::to_string` then `from_str` on a hand-built `CalibrationReport<MergeSplitParams>` with a non-empty `sweep`/`notes` → `assert_eq!` against the original (`CalibrationReport`/`SweepResult` derive `PartialEq`, §G).

### `xtask/tests/calibrate_verb.rs` (new)

24. `calibrate_help_exits_zero` — `cargo run -p xtask -- calibrate --help` exits 0.
25. `calibrate_synthetic_sweep_merge_split_produces_a_report` — `xtask calibrate --mode synthetic-sweep --target merge-split --scenario split_ramp --out-dir <tmp>` exits 0; `<tmp>/calibration-report.json` parses as `CalibrationReport<MergeSplitParams>` with `verdict == Recommend` (a real, if small, end-to-end run — the one test in this file that actually shells out to `calibration_sweep_point`, several times, still comfortably under a few seconds since every sample is synchronous); `target/verify/calibrate.json` exists with `status: "pass"`.
26. `calibrate_rejects_unknown_scenario` — `--scenario bogus` → nonzero exit, `target/verify/calibrate.json`'s `status: "fail"` naming the bad scenario.

### `xtask/tests/calibrate_path_guard_coverage.rs` (new)

27. `path_guard_protects_01s_calibration_target_text` — `path_guard::check_paths(ChangesetType::Implementation, &["docs/planning/01-server-architecture.md".into()])` → exactly one violation against the new row (§H item 5) — proves the row actually matches, not merely that it was added.
28. `governance_changeset_shape_dry_run` — the "changeset-shape validation" this blueprint's own task explicitly asks for: `path_guard::check_paths(ChangesetType::Governance, &["docs/planning/01-server-architecture.md".into(), "crates/scheduler/src/pool/worker_pool.rs".into(), "crates/scheduler/src/managed_region.rs".into()])` → `assert_eq!(violations.len(), 0)` (a governance changeset is permitted to touch the protected doc row; the two `rc-scheduler` source files are not protected paths at all, §H item 4, so they contribute no violation regardless of changeset type — both facts proven by the same call). A companion assertion, `path_guard::check_paths(ChangesetType::Implementation, &["docs/planning/01-server-architecture.md".into()])`, re-proves test 27's point that the *same* path under `Implementation` **does** violate — the contrast is the actual governance-gate proof.

## Implementation steps

1. **`worker_pool.rs`.** Add `ResizeThresholds`/`Default`, the new `resize_thresholds` field on `PoolState`, `with_resize_thresholds`, and thread `resize_thresholds` through `sample_and_maybe_resize`'s existing four literal sites (§E). Redefine `with_config` as calling `with_resize_thresholds(config, ResizeThresholds::default())`. Observable: tests 1–3 pass; every pre-existing `pool_*` test still passes unmodified.
2. **`managed_region.rs`, `region_manager.rs`.** Add `HysteresisThresholds`/`Default`, `ManagedRegion`'s new private field and its three in-crate call-site updates, `RegionManager::with_thresholds`/`with_thresholds_and_metrics`; redefine `new`/`new_with_metrics` in terms of the new siblings with `HysteresisThresholds::default()`. Observable: tests 4–6 pass; every pre-existing `lifecycle_hysteresis`/M6-B02 test still passes unmodified.
3. **`crates/scheduler/Cargo.toml`, `src/bin/calibration_sweep_point.rs`.** Add the `[[bin]]` entry and `clap` dependency; implement `main` per Deliverables — local `MergeSplitParams`/`PoolSizingParams`/series mirror structs, the four built-in scenario scripts (§G's table, as literal Rust `match` arms), the replay loop (`RegionManager::with_thresholds` + repeated `record_synthetic_tick` calls for merge-split; `RcWorkerPool::with_resize_thresholds` + the block-workers-and-push-jobs technique + repeated `sample_and_maybe_resize` calls for pool-sizing), JSON output via `serde_json::to_writer_pretty`. Observable: tests 7–10 pass.
4. **`rc-test-harness`'s `calibration` module.** Implement `series.rs`, `thrash.rs` (`count_thrash`, generic over any `PartialEq + Copy` direction type), `report.rs` (`score_merge_split`/`score_pool_sizing`/`select_recommended`/`build_ratio_grid`/`build_count_grid`) exactly per §F/§G. Observable: tests 11–23 pass.
5. **`xtask`'s `calibrate` verb.** Implement `CalibrateArgs`/`run` — grid construction, subprocess dispatch to `calibration_sweep_point` (via `xshell`, `--release` build), deserialization, scoring, `select_recommended`, report + `TierResult` writing. Wire `Command::Calibrate` in `main.rs`. Observable: tests 24–26 pass.
6. **`path_guard.rs`.** Add the one new `PROTECTED_PATHS` row (§H item 5). Observable: tests 27–28 pass.
7. **`CONTRIBUTING.md`.** Append the one new protected-path table row (§H item 6).
8. **Run the full acceptance suite.** `cargo nextest run -p rc-scheduler -p rc-test-harness -p xtask` — every test named above passes, plus every pre-existing M0-B04/B05/B06/M6-B02 test unmodified-and-green. Commit this blueprint's implementation changeset with `Changeset-Type: governance` (Constraints — this blueprint's own files, like every prior harness/tooling blueprint's, necessarily touch `xtask/**`).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/scheduler/tests/`, `crates/testing/test-harness/tests/`, and `xtask/tests/` named above is committed first, alongside `todo!()`-stubbed (full signatures/derives/doc comments/constant values) source files. The implementation changeset fills in bodies only — no test file is ever edited, no assertion weakened, no `split_sustained_ticks == 20`/`grow_streak_threshold`-style exact-value assertion loosened to a range.

(b) **This blueprint's own changeset is labeled `governance`, not `implementation`** — it touches `xtask/**` (`path_guard.rs`'s new row) and `docs/planning/01-server-architecture.md` is the row's *target*, not something this blueprint edits itself (§H's actual doc-text rewrite is a *later*, separate governance changeset — this blueprint only builds the row and the pipeline that will one day produce that later changeset's numbers). Mirrors M0-B08/M6-B01's identical self-labeling precedent.

(c) **No new external dependencies beyond the pinned set.** `clap` (`rc-scheduler`'s one new line, already workspace-pinned `4.6.6` per `xtask`'s own established version) is the only new `Cargo.toml` line anywhere in this blueprint; `serde`/`serde_json`/`thiserror`/`xshell` are already present in every crate that uses them here.

(d) **No Mojang or third-party reimplementation source.** Every numeric constant this blueprint introduces (`MERGE_SPLIT_THRASH_WINDOW_TICKS`, `POOL_THRASH_WINDOW_SAMPLES`, `MIN_MEANINGFUL_IMPROVEMENT_RATIO`, the built-in scenario scripts' own sample values) is this blueprint's own original methodology choice, restated with its derivation shown inline (§F/§G) — never cross-checked against any Mojang or third-party source, since none makes a vanilla-parity claim.

(e) **`calibration_sweep_point` and `xtask calibrate`'s `synthetic-sweep` mode never touch the network, never spawn `rusty-clanker-server`, never require the oracle** — every replay is in-process (the bin target) or a local subprocess of that same bin target (the xtask verb), matching TEST-D31's own "bot-swarm load testing... never in ordinary CI" framing by simply never invoking a bot swarm at all in this blueprint's own Tier-1 gate.

(f) **`POOL_RESIZE_SAMPLE_INTERVAL` (50 ms, the pool's sample cadence) is not itself a sweep parameter of this blueprint.** ARCH-D19's own arithmetic ("100 consecutive ticks (5s)... only consistent at 50ms/sample," M0-B04's Context) derives every other pool-sizing number *from* that cadence; sweeping the cadence itself would silently redefine what "100 samples" means mid-sweep, a materially different and out-of-scope calibration question.

(g) **`--mode real-sweep` is not implemented by this blueprint** (§A, §H) — `CalibrateMode` names only `SyntheticSweep`; adding a `RealSweep` variant is explicitly a future sibling blueprint's own addition, once the composition root it depends on exists.

## Verification commands

```
cargo build -p rc-scheduler -p rc-test-harness -p xtask --all-features
cargo nextest run -p rc-scheduler -p rc-test-harness -p xtask
cargo test --doc -p rc-scheduler -p rc-test-harness
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- calibrate --help
```

All run headless, identically, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43) — no oracle, no Java, no network access, no real `rusty-clanker-server` build required for any of them.

## Open questions

- **A fully joint (not sequential) ARCH-D6/ARCH-D19 sweep** (§I) is a heavier refinement this blueprint does not build — the sequential-order resolution stated there is binding on any real calibration run until a joint methodology is separately designed.
- **The ARCH-D19 hot/quiet batch-granularity mechanism's own two thresholds (35 ms/5 ms) and two batch sizes (32/128)** cannot be calibrated by this blueprint or any other until the dispatch-granularity mechanism itself is implemented (§C) — whichever blueprint first builds it should extend `ResizeThresholds` (or a sibling struct) with those four numbers and reuse this blueprint's `count_thrash`/`select_recommended`/grid-construction machinery unmodified, rather than inventing a second calibration pipeline.
- **The composition-root sibling blueprint's exact interface to `xtask calibrate --mode real-sweep`** (§A, §H, Constraint (g)) is not designed here — it plausibly reuses M6-B01 §B's `--region-layout`/`RC_REGION_LAYOUT`/`--region-lifecycle-log` contract directly (a lifecycle-log NDJSON stream is already exactly `LifecycleEvent`-shaped) plus a new `--resize-thresholds`/`--hysteresis-thresholds` CLI flag on the same composition root, deserializing this blueprint's own `ResizeThresholds`/`HysteresisThresholds` JSON shapes — left to that blueprint's own Context to pin precisely against whatever the composition root's actual config-loading shape turns out to be.
- **Whether a real reference-host `real-sweep` run should widen the grid beyond this blueprint's `spread: 0.5, steps: 7` seed defaults**, and how many independent replicate runs per grid point a real (network-jitter-affected) measurement needs before its own `reaction`/`thrash_event_count` numbers are trustworthy (this blueprint's synthetic replay is exactly repeatable; a real run is not) — an open statistical-methodology question for whichever blueprint implements `--mode real-sweep`, not resolved here.
