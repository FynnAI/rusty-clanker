# M6-B02 — Metrics & Per-Region CPU Attribution

| Field | Content |
|---|---|
| ID | M6-B02 |
| Milestone | M6 — Scale & Optimization: Multi-Region Throughput |
| Prerequisites | M0-B04 (`rc-scheduler::pool::RcWorkerPool` — work-stealing pool, elastic sizing, `worker_count()`/`backlog_depth()`; this blueprint adds two small getters to its already-merged `worker_pool.rs`, both restated in full below). M0-B05 (`rc-scheduler`'s RC-Executor — `RcExecutor`, `RcExecutorBuilder`, `RegionState`, `TickReport`, `DomainGroup`, `Stage`, and the 11-stage pipeline dispatch this blueprint instruments at its wave-dispatch and inline-call sites; every type/method used below is restated exactly as M0-B05 fixed it, never modified in shape). M0-B06 (`rc-scheduler`'s region model — `ManagedRegion`, `RegionManager`, `GridCell`, `LifecycleOutcome`, the ARCH-D19 EWMA formula and the ARCH-D6 merge/split protocols this blueprint reads from and journals; restated exactly as M0-B06 fixed it). |
| Implements | ARCH-D18 (RC-WorkerPool hard-cap/utilization tracking); ARCH-D19 (EWMA — a new instance of the identical α=0.2 formula `ManagedRegion` already applies to tick duration, applied here to CPU-time; the hot/quiet coalesced-tick CPU-accounting mechanism M6's acceptance criterion 2 needs); ARCH-D20 (EDF admission-violation counter, exact definition and zero-violation assertability); ARCH-D6 (merge/split lifecycle journal, the calibration input `11-roadmap-milestones.md`'s M6 goal — "replace `01`'s seed threshold defaults with calibrated values" — depends on); ARCH-D7 (per-region independent degradation — measured, not implemented, by this blueprint); CLUSTER-D28 (the general `tracing`-plus-metrics, no-bundled-backend observability pattern — this blueprint is its monolithic-mode instance; every cluster-specific field of CLUSTER-D28's own required-metrics list — message latency, handoff counters, raft/QUIC gauges — is out of scope, M7); TEST-D30 (Tracy feature-gating pattern, restated and narrowed); TEST-D31 (the metric feed a bot-swarm load-test harness collects); PERF-D11 (adjacent debug-only allocation-counter precedent, contrasted against this blueprint's own always-on production counters); PERF-D52 (Tracy overhead ceiling, contrasted against this blueprint's own tighter, always-on attribution budget). |
| Crates touched | `rc-scheduler` (`crates/scheduler/`) only |
| Estimated scope | L |

## Goal & Done definition

Give `rc-scheduler` the measurement layer M6's own three acceptance criteria (`11-roadmap-milestones.md`) are checked against — none of it exists yet, since M0–M5 built the mechanisms (`RcWorkerPool`, `RcExecutor`, `RegionManager`) but never instrumented them. Concretely: (1) a `MetricsRegistry` that attributes RC-WorkerPool thread-CPU-time to `(RegionId, Stage)` pairs by tagging every dispatched task, precise enough to prove a 0-player coalesced region's dedicated CPU cost is near-zero (criterion 2); (2) RC-WorkerPool utilization sampling against ARCH-D18's hard cap (criterion 1); (3) an EDF admission-violation counter with an exact, checkable definition and zero-violation assertability, so a fault-injection test (criterion 3) can assert siblings never got starved by an overloaded region; (4) a tick-duration histogram/EWMA export that reads — never recomputes — `ManagedRegion`'s own already-correct EWMA; (5) a bounded merge/split event journal feeding ARCH-D6 threshold calibration; (6) one machine-readable JSON export format the M6 bot-swarm harness (a sibling, not-yet-written blueprint) polls. Every piece is wired additively into `rc-scheduler`'s already-merged M0-B04/B05/B06 code without changing any existing public signature's shape or any existing test's observable behavior — a fresh `RcExecutor`/`RegionManager` built without opting into metrics behaves byte-for-byte as it did before this blueprint landed.

Done when:

- [ ] `cargo build -p rc-scheduler --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler` on both OS legs.
- [ ] Every pre-existing M0-B04/B05/B06 test (`pool_*`, `tick_clock_drift`, `compute_waves_conflict_graph`, `access_compatibility`, `registration_validation`, `pipeline_ordering`, `sync_points`, `determinism`, `lifecycle_hysteresis`) still passes, byte-for-byte unmodified, proving this blueprint's additions are behavior-preserving when metrics are not opted into.
- [ ] `cpu_attribution_matches_pinned_synthetic_cost` (attribution-accuracy against a known-cost synthetic workload) passes, including its near-zero/coalesced-path sub-case.
- [ ] `edf_violation_counter_flags_deliberate_violation` and `edf_violation_counter_zero_on_clean_run` both pass.
- [ ] `attribution_overhead_within_budget_at_realistic_granularity` (the overhead self-measurement test) passes.
- [ ] `metrics_wrapping_does_not_change_tick_determinism` passes at worker counts `{1, 2, 8}` (this blueprint's own instrumentation is proven observationally inert on `World` state and message sequence).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's one new normal dependency, `hdrhistogram`, touches neither `rc-messaging`'s Rule 3 nor `rc-mod-api`'s Rule 4 sets, and `rc-scheduler` gains no edge into `NETRENDER` (Rule 2 unaffected).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Scope boundary: what this blueprint owns, what it explicitly does not

This blueprint builds the **measurement** layer only. It does not implement: the real-time EDF admission scheduler itself (ARCH-D20's actual "which overdue region wins the Injector" decision — a sibling, not-yet-written M6 blueprint's job; this blueprint only defines the violation-detection contract that scheduler must feed events into); ARCH-D19's actual coalesced single-work-item dispatch for quiet regions (M0-B06 already flagged this "not implemented, stubbed, or delegated to any other M0 blueprint" — still true here; this blueprint's attribution mechanism is dispatch-granularity-agnostic by construction, so it measures correctly the moment that dispatch path exists, without needing to pre-empt its design); the bot-swarm load-testing harness that drives 200 bots across ≥8 regions and polls this blueprint's JSON export (a sibling M6 blueprint); PGO/BOLT and Tier-3 SLO gating (`14-performance-engineering.md`'s own Section G/I, a different M6 blueprint's scope per the milestone's own Scope bullet: "`14` owns this milestone's concrete Tier-3 release-gate content... as measurable acceptance-criteria inputs"); OTLP/OpenTelemetry export (CLUSTER-D28's own crate pins are explicitly deferred by `12-workspace-structure.md`'s Open Questions "until that decision is actually implemented," and CLUSTER-D28's concrete required-metrics list is cluster-mode-specific — message latency, handoff, raft, QUIC — none of which exists in M6's monolithic-only scope; this blueprint's JSON export is the concrete "machine-readable series" M6 actually needs, and its `tracing` spans are the exact substrate a future OTLP exporter would consume without this blueprint needing to add OTLP crates itself).

### Where this lives, and why extending already-merged files is correct here

`rc-scheduler`'s Crate Manifest responsibility (`12-workspace-structure.md`) is "RC-Executor, RC-WorkerPool, the 11-stage tick pipeline driver, region lifecycle, the ARCH-D8 conflict graph, the Tokio<->RC-WorkerPool boundary types" — its own operational metrics are a natural, in-scope cross-cutting concern of the scheduler itself, not a separate crate; introducing a new workspace member would additionally require revising `12`'s own ratified Crate Manifest, out of a blueprint's authority. Three already-merged files gain small, additive modifications (new fields defaulting to `None`/absent, new constructor variants, conditional bodies inside already-existing private dispatch code) — no existing public signature's *shape* changes:

- `crates/scheduler/src/pool/worker_pool.rs` (M0-B04): two new public getters reading fields that already exist privately.
- `crates/scheduler/src/executor.rs` (M0-B05): one new private field (`metrics: Option<Arc<MetricsRegistry>>`), one new builder method (`RcExecutorBuilder::with_metrics`), and `tick_region`'s already-private dispatch bodies gain a conditional wrap at each of its wave-dispatch and inline-call sites — exactly the two "`// pool dispatch`" extension points M0-B05's own Context already named as future-modification sites, plus the three inline (Stage 1/4/10) call sites, on the same conditional basis.
- `crates/scheduler/src/region_manager.rs` (M0-B06): one new private field, one new constructor variant (`RegionManager::with_metrics`), and `spawn_region`/`tick_region`/`record_synthetic_tick`/`execute_merge`/`execute_split`'s bodies gain conditional metrics calls at the exact points they already compute the values this blueprint exports (tick-duration sample, `LifecycleOutcome`).

Every one of these three files' pre-existing tests must continue to pass unmodified — the "Done when" checklist's second item is this blueprint's own commitment to that, verified directly by `metrics_wrapping_does_not_change_tick_determinism` re-running M0-B05's own determinism scenario with metrics attached.

### The core design decision: precise per-task region tagging, not sampling

The instruction this blueprint must resolve: "how worker time is attributed to regions on a work-stealing pool — the exact accounting mechanism: per-task region tagging + thread-time sampling vs precise per-task timing." **Decision: precise per-task timing via per-task region/stage tagging, never periodic sampling.** Rationale, restated concretely:

M6 runs many regions concurrently on one shared `RcWorkerPool` (200 bots, ≥8 regions, ARCH-D18/D19's elastic pool) — a region's tick is not one contiguous span on one thread; it is many small tasks (ARCH-D19's 32–128-entity/chunk batches) scattered across whichever worker threads stole them, at whatever wall-clock moments work-stealing happened to schedule them. A **sampling** approach (periodically polling "which region is each worker currently executing") would need to run often enough to catch sub-millisecond tasks reliably, and would systematically under-count exactly the case M6's acceptance criterion 2 most needs precision for — a 0-player region's near-zero coalesced tick is the *hardest* case for a sampler to ever land a sample on, being the shortest-lived, least-frequent unit of work in the whole pool. **Precise per-task timing** has no such blind spot: every task, however short, self-reports its own exact cost the instant it finishes, regardless of how rarely it runs. The mechanism (Deliverables, below): every task submitted to `RcWorkerPool` on behalf of a region is wrapped, at its construction site, by a closure that reads the *executing* thread's own cumulative thread-CPU-time immediately before and after running the real task body, and atomically adds the delta to that `(RegionId, Stage)` pair's accumulator in a shared `MetricsRegistry`. The wrapper travels with the task through work-stealing (it *is* the task, cast to `Job`) — attribution is therefore correct regardless of which worker thread ultimately steals and runs it, with zero coordination needed between workers.

### Why thread-CPU-time, not wall-clock, for attribution — and the resulting two-clock design

`ManagedRegion`'s own tick-duration EWMA (M0-B06) is deliberately **wall-clock** (`Instant`-based): it measures what ARCH-D7's 50 ms budget actually governs — real elapsed time, including any contention the tick experienced waiting for a worker. This blueprint's CPU-attribution mechanism measures a **different** quantity for a **different** purpose — ARCH-D18's hard-cap/near-zero-dedicated-CPU questions are about how much *work* a region actually costs the pool, which must exclude any time a worker thread spent parked, stolen-from, or executing a *different* region's task while this region's own task sat queued. Thread-CPU-time (a per-thread OS-maintained counter of only-this-thread's-actually-scheduled-and-running time) is exactly that exclusion; wall-clock is not. The two metrics are therefore computed with two different clocks, on purpose, and this blueprint states that contrast explicitly rather than leaving it implicit: **tick duration = wall-clock** (unchanged, M0-B06's own), **CPU cost = thread-CPU-time** (new, this blueprint).

### Clock discipline — platform primitives, restated concretely

Wall-clock reuses `std::time::Instant` exactly as M0-B04/M0-B06 already established (no new decision). Thread-CPU-time is a primitive no prior blueprint has needed:

- **Linux:** `nix::time::clock_gettime(ClockId::CLOCK_THREAD_CPUTIME_ID)` — `nix` is already a `rc-scheduler` Linux-only dependency (pinned `0.31.3`, added by M0-B04 for its own `SCHED_RR` call), so this is a new *call*, not a new dependency. Resolution is sub-microsecond on any current Linux kernel (a hardware-timestamp-backed counter, not a polled timer — no PERF-D53-style coarse-granularity concern applies here).
- **Windows:** `GetThreadTimes(GetCurrentThread(), &creation, &exit, &kernel, &user)` (via the already-pinned `windows` crate, `0.62.2`, already a `rc-scheduler` Windows-only dependency from M0-B04 with the `Win32_Foundation`/`Win32_System_Threading` features this call also needs — no new feature flag required), summing `kernel + user` (both `FILETIME`, 100 ns units) into a `Duration`. **Moderate-confidence flag, honestly stated per this corpus's own established convention for exact platform-API behavior** (mirroring M0-B04's own Windows-timer/Linux-scheduler notes): `GetThreadTimes`'s *practical* resolution on modern Windows 10/11 is scheduler-quantum-based (typically sub-millisecond for a thread with real CPU-bound work), not the coarse ~15.6 ms multimedia-timer granularity PERF-D53 documents for a *different* API family — but this has not been independently re-verified against the installed `windows` 0.62.2 crate's exact wrapper shape or measured on real target hardware; confirm both at implementation time (`cargo doc -p windows`, plus a quick real measurement) before trusting the Windows leg's attribution precision at sub-100-µs task granularity.
- **Any other target, or either call failing:** returns `None`, never panics — the wrapper transparently falls back to a wall-clock (`Instant`) delta for that one task, attributing *something* rather than dropping the sample (mirroring PERF-D57/PERF-D55's own established "graceful fallback, never fatal" house style for a platform primitive that cannot be assumed universally available).

### Overhead budget and feature-gating — restated per PERF's instrumentation-overhead policy

Two prior decisions set precedent, and this blueprint deliberately does **not** follow either one exactly, stating why: PERF-D11's debug-only allocation counter is `cfg(debug_assertions)`-only with zero release cost, because it is a developer hot-path-discipline check with no production relevance. PERF-D52's Tracy overhead ceiling (5%, informational, Tier-3-only) applies to a feature that is "never in the default release build" (TEST-D30) — a developer-run desktop tool. **This blueprint's core counters are neither.** M6's own acceptance criteria (near-zero dedicated CPU for a 0-player region; siblings holding 20 TPS while one region degrades) are properties that must be checkable in a real, release-mode, under-load server — the exact opposite of a debug-only or opt-in-desktop-tool measurement. The core registry (per-region/stage CPU attribution, pool utilization, the EDF-violation counter, tick-duration EWMA/histogram export, the lifecycle journal) is therefore **always compiled in, no Cargo feature gate, on both debug and release builds** — and precisely because it is always-on, this blueprint fixes a correspondingly tighter design budget than PERF-D52's Tracy ceiling:

> **`ATTRIBUTION_OVERHEAD_BUDGET_RATIO = 0.01`** — the aggregate added cost of wrapping every RC-WorkerPool task with region/stage CPU-time attribution must not exceed 1% of the pool's own aggregate busy CPU time, measured at realistic task granularity (ARCH-D19's 32–128-entity/chunk batch unit — tens of microseconds or more per task — never a degenerate near-zero-cost synthetic task, whose ratio would be dominated by measurement noise rather than the wrapper's actual fixed cost). This is a *design target*; `attribution_overhead_within_budget_at_realistic_granularity` (Acceptance tests) checks it with a deliberately looser CI-stability tolerance, exactly mirroring `TickClock`'s own established "tight algorithmic proof, loose real-time smoke test" pattern (M0-B04).

Separately, ordinary `tracing::info_span!`/`tracing::debug!` instrumentation (unconditional — `tracing`'s own design already makes an unsubscribed span/event effectively free) is added at the same wrap points, purely as event/field data, costing nothing beyond what `tracing` itself already costs whether or not this blueprint exists. **Tracy visualization is not this blueprint's concern at all**: TEST-D30's own rationale text already states the reason — "Reusing spans already required for OTLP tracing... rather than hand-instrumenting a second, profiler-specific set of markers means Tracy visualization costs nothing beyond the observability instrumentation the project needs anyway." Because the spans this blueprint adds are ordinary, subscriber-agnostic `tracing` spans, the `tracy` Cargo feature and the `tracing_tracy::TracyLayer::new().init()` call both belong entirely at the **composition-root binary** level (whichever crate owns `main.rs` and therefore owns installing a global `tracing` subscriber) — `rc-scheduler` is a library with no such installation point, needs no `tracy`-specific dependency, and this blueprint adds none. This is restated as a Constraint, not merely a Context note, since it is easy to mistakenly assume a `tracy` feature belongs on every crate that emits spans.

### Per-task region/stage tagging — the exact wrap points in `tick_region`

M0-B05's `RcExecutor::tick_region` dispatches each domain group's `compute_waves` output via `pool.run_batch(tasks)` for Stages 6 (`AiPhysics`), 8 (`Lighting`), 9 (`ChunkSerialize`), 11 (`NetCodec`) — this blueprint wraps every task pushed into that `Vec` with `attribution::region_tagged_task(region.id, stage, metrics.clone(), original_task)` whenever `self.metrics.is_some()`. Stage 4 (`BlockRedstone`) runs each system via a direct, single-worker `.run()` call, never through `run_batch` (ARCH-D13's sequential collapse) — this blueprint wraps that direct call with `attribution::measure_inline(region.id, Stage::ScheduledBlockTick, metrics, || system.run(...))` instead. Stages 1 and 10 (the two ARCH-D9 sync points — message drain, command-buffer apply/flush) are likewise direct, single-worker calls on whichever thread is running `tick_region` itself; they are wrapped with `measure_inline` the same way, tagged `Stage::PreTickSync`/`Stage::PostTickFlush` respectively, for completeness of the per-stage breakdown (their cost is typically small, but a consistently-wrapped total is what makes `StageMetricsSnapshot`'s per-stage sums actually sum to the tick's real total CPU cost). Stages 2, 3, 5, 7 accept no domain-group registration at M0–M5 (M0-B05's own Context) and are therefore not wrapped — there is nothing dispatched there yet to attribute; a future blueprint that first registers real content into one of those stages extends the wrap list the identical way, not this blueprint's job to pre-empt.

### Near-zero dedicated CPU — the quantitative acceptance threshold, with rationale

M6 acceptance criterion 2's own wording ("measured, via per-region CPU attribution metrics, to contribute near-zero dedicated CPU") leaves "near-zero" undefined; this blueprint fixes it, the same "blueprint-phase concrete pin, calibration-pending" status every other unpinned numeric threshold in this corpus already carries (ARCH-D6/D19's own seed defaults, `01`'s Open Questions):

> **`is_near_zero_dedicated_cpu(region)` is `true`** iff that region's CPU-time EWMA (this blueprint's own new EWMA instance, α = 0.2 — Context: "Why thread-CPU-time...") has stayed at or under **2% of `tick_budget_ms`** (`NEAR_ZERO_CPU_THRESHOLD_RATIO = 0.02` — `1.0 ms` at the production `50 ms` budget) for at least **40 consecutive ticks** (`NEAR_ZERO_SUSTAINED_TICKS = 40` — reusing ARCH-D6's own split-hysteresis window count for consistency with an already-established "how long is 'sustained'" answer in this corpus, rather than inventing a fifth unrelated number). **Rationale for `2%`:** it sits comfortably above this blueprint's own `1%` attribution-overhead floor (so the threshold is never accidentally tripped by measurement noise alone) while remaining an order of magnitude below `14`'s own PERF-D59 per-stage nominal-load budget table (whose stages individually range from `0.1`–`4.0` ms *each* at nominal, non-idle load) — giving clear separation between "structurally idle, coalesced onto a shared worker" and "doing real, if modest, per-tick work."

### EDF admission-violation counter — exact definition, and why it is fed, not computed, by this blueprint

ARCH-D20: "each region's deadline = `last_tick_start + 50ms`; RC-Executor's Injector serves overdue regions before on-time regions regardless of arrival order." No M0 blueprint implements the actual admission decision (M0-B04's Context: "ARCH-D20's actual cross-region EDF admission *decision*... is a separate blueprint's job"; M0-B06's round-robin driver is explicitly *not* that decision either). This blueprint therefore cannot observe "did the scheduler pick correctly" from the inside — instead it defines a **feed-in contract** any current or future admission mechanism (the sibling M6 blueprint that implements the real EDF loop) calls into, and a precise, checkable violation rule over that feed:

- `record_deadline_ready(region, deadline, now)` — call the instant a region becomes due (its own `TickClock::is_overdue` first returns `true`), recording `region -> deadline` in an internal `waiting` set. A later call for an already-waiting `region` overwrites its entry (a region has at most one outstanding readiness at a time — `TickClock`'s own sequential deadline model already guarantees this).
- `record_admission(region, deadline, admitted_at)` — call the instant the scheduler actually starts ticking `region` to fulfil `deadline`. **Violation definition:** for every *other* entry `(other_region, other_deadline)` still present in `waiting` at this moment with `other_deadline < deadline` (a region whose own deadline is strictly earlier — i.e. more overdue — than the one just admitted, and which was ready and waiting), one `EdfViolation` is recorded. `region`'s own entry is then removed from `waiting` (it has been admitted).

**Zero-violation assertability, directly from this definition:** a scheduler that always admits the globally-earliest-deadline ready region first can never leave a strictly-earlier-deadline entry sitting in `waiting` at the moment of any admission — the violation count stays exactly `0` for the entire run, by construction, not by statistical luck. A deliberately-violating test feed (admit a later-deadline region while an earlier-deadline one is still marked ready) produces a non-zero count immediately and deterministically — exactly the "deliberately forced violation → counted; clean run → zero" property this blueprint's acceptance tests must prove.

### Coalesced-tick-path CPU accounting

ARCH-D19's actual quiet-region single-work-item coalescing is not implemented by any blueprint yet (Context: "Scope boundary"). This blueprint's attribution mechanism needs no foreknowledge of that dispatch shape to measure it correctly once it exists: whether a region's tick submits many fine-grained tasks (hot) or exactly one coalesced task (quiet), every submitted task is wrapped and tagged with that region's id the identical way — the per-region accumulator sums correctly regardless of how many tasks constituted the tick. This blueprint's own attribution-accuracy test (Acceptance tests, below) proves this directly: one synthetic "hot" region dispatches many small tagged tasks, one synthetic "quiet" region dispatches a single tagged task with a tiny pinned cost, and both are measured through the identical `MetricsRegistry` API — the quiet region's measured EWMA falls under the near-zero threshold, the hot region's does not, with no special-cased "coalesced" code path anywhere in this blueprint's own implementation.

### Pool utilization (ARCH-D18)

`RcWorkerPool` (M0-B04) already tracks, privately, `worker_count_cache` and `active_workers` (an `Arc<AtomicUsize>` incremented/decremented around every executed job) but exposes only `worker_count()` publicly, and never exposes `hard_cap` at all (a config value baked in at construction, `RcWorkerPoolConfig.hard_cap`, private thereafter). This blueprint adds two small, additive public getters (Deliverables) reading these already-existing fields — no new bookkeeping, no new lock, no behavior change to the pool itself. `PoolUtilizationSample` (Deliverables) is a plain point-in-time snapshot combining `worker_count()`, the new `hard_cap()`, the new `active_worker_count()`, and the already-public `backlog_depth()` into the two ratios criterion 1 needs: `size_utilization_ratio` (is the elastic pool approaching ARCH-D18's hard cap) and `busy_fraction` (is the pool, at its current size, actually saturated).

### Tick-duration histogram + EWMA export — reading, not reimplementing

`ManagedRegion::tick_duration_ewma_ms()` (M0-B06) is already the correct, already-tested ARCH-D19 EWMA for tick *duration* (wall-clock) — this blueprint never recomputes it. `RegionManager::tick_region`/`record_synthetic_tick`'s bodies already compute a `sample_ms` value and already call `ManagedRegion::record_tick_duration(sample_ms)` internally; this blueprint's only addition at that exact point is one more call, `metrics.record_tick_duration_sample(region.id, sample_ms, region.tick_duration_ewma_ms())`, forwarding the value M0-B06 already computed into (a) this blueprint's own bounded-memory streaming percentile tracker (`TickDurationHistogram`, wrapping `hdrhistogram` — the production-suitable counterpart to `RegionTickHistogram::from_samples`'s batch, end-of-10-minute-run computation, which cannot run continuously on an unbounded production server without retaining every raw sample forever) and (b) the snapshot's own `tick_duration_ewma_ms` field, a straight passthrough of M0-B06's own already-correct value.

### Merge/split lifecycle journal

Every non-`None` `LifecycleOutcome` `RegionManager::execute_merge`/`execute_split` already produces (M0-B06) is journaled, when metrics are attached, as one `LifecycleEvent` in a bounded ring buffer (`DEFAULT_LIFECYCLE_JOURNAL_CAPACITY = 4096` — a seed default, oldest-evicted-first, sized generously above any plausible single M6 load-test run's merge/split event count; a production server's own harness pulls via `drain_lifecycle_events()` on its own polling cadence rather than requiring an unbounded buffer). Each event records both regions'/fragments' cell counts and the triggering EWMA value(s) — exactly the inputs `11-roadmap-milestones.md`'s M6 goal ("replace `01`'s seed threshold defaults with calibrated values") needs to analyze after a real load run: how close to the 90%/10%-of-budget thresholds a real trigger actually fired, and how balanced real splits actually came out.

### Claims to verify (TEST-D57)

- None.

## Deliverables

### `crates/scheduler/Cargo.toml` (modify — one new normal dependency; promote `serde_json` from dev-only)

```toml
[dependencies]
# ... all M0-B04/B05/B06 entries unchanged (rc-core, rc-messaging, rc-mod-host, bevy_ecs,
# thiserror, crossbeam-deque, crossbeam-utils, parking_lot, core_affinity, tracing, serde) ...
serde_json = { workspace = true }   # moved here from [dev-dependencies] — JSON snapshot export
                                     # (metrics::snapshot::write_snapshot_json) is production
                                     # functionality now, not test-only
hdrhistogram = { workspace = true } # this blueprint's own new pin — see root Cargo.toml diff below

# [dev-dependencies] no longer needs its own serde_json line (already pulled in above; Cargo
# makes every [dependencies] entry available to dev-dependencies/tests automatically).

# [target.'cfg(windows)'.dependencies] / [target.'cfg(target_os = "linux")'.dependencies] and
# [features] soak-tests = [] are all M0-B04/B06's own, unchanged.
```

### Root `Cargo.toml` (modify — one new `[workspace.dependencies]` entry; `12-workspace-structure.md`'s own governance is that this table is the single version source of truth, WS-D7)

```toml
[workspace.dependencies]
# ... every existing entry unchanged ...
hdrhistogram = "7.5.4"   # rc-scheduler's metrics::histogram, this blueprint's own new pin
```

**Moderate-confidence flag, honestly stated:** `7.5.4` is this blueprint's own best-available pin, not independently re-verified against crates.io at the time this blueprint was written (this corpus's own established convention for exactly this situation — see M0-B04's `windows`/`nix` API-surface notes). Re-verify the current published version at implementation time and update both `Cargo.toml` locations together if a newer patch exists; nothing about this blueprint's design depends on the exact patch digit.

### `crates/scheduler/src/lib.rs` (modify — add one module declaration/re-export; every existing line from M0-B04/B05/B06 stays untouched)

```rust
pub mod metrics;
```

### `crates/scheduler/src/pool/worker_pool.rs` (modify — two new public getters, additive; every existing item unchanged)

```rust
impl RcWorkerPool {
    // ... every existing method (new, with_config, spawn, run_batch, worker_count,
    // backlog_depth, sample_and_maybe_resize, wait_idle) unchanged ...

    /// The pool's configured hard cap (ARCH-D18) — `baseline.saturating_mul(2)` in
    /// `Elastic` mode, `fixed_size.max(1)` (its own permanent, unchanging size) in
    /// `Deterministic` mode. Previously private; this blueprint's own `PoolUtilizationSample`
    /// (Context: "Pool utilization") is its first consumer.
    pub fn hard_cap(&self) -> usize;

    /// Current count of workers actively executing a job (as opposed to idle/parked) —
    /// a point-in-time snapshot of the already-existing `active_workers` counter every
    /// worker increments/decrements around running a job (M0-B04's Implementation step 2).
    /// Previously private; read-only, adds no new bookkeeping.
    pub fn active_worker_count(&self) -> usize;
}
```

### `crates/scheduler/src/executor.rs` (modify — one new private field, one new builder method; `tick_region`'s existing body gains conditional wraps; every public signature's *shape* unchanged)

```rust
use std::sync::Arc;
use crate::metrics::{self, MetricsRegistry};

pub struct RcExecutor {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [CompiledGroup; 5],
    metrics: Option<Arc<MetricsRegistry>>,   // new field; `None` for every executor built
                                              // without calling `with_metrics` — the exact
                                              // pre-this-blueprint behavior, unchanged
}

impl RcExecutorBuilder {
    // ... `new`, `register_system`, `build` all unchanged in signature ...

    /// Opts this executor's `tick_region` into per-`(RegionId, Stage)` CPU attribution
    /// (Context: "Per-task region/stage tagging"). Not calling this method (the default)
    /// leaves `tick_region` byte-for-byte identical to its pre-M6-B02 behavior — this
    /// blueprint's own `metrics_wrapping_does_not_change_tick_determinism` test proves
    /// that opting *in* is likewise observationally inert on `World` state/message
    /// sequence, only adding side-channel timing.
    pub fn with_metrics(self, metrics: Arc<MetricsRegistry>) -> Self;
}

impl RcExecutor {
    // `spawn_region`'s signature and behavior are entirely unchanged — metrics registration
    // for a region is `RegionManager::spawn_region`'s job (below), not RcExecutor's, since
    // RcExecutor's own `spawn_region` has no RegionId parameter to register under (M0-B05's
    // RegionState.id is allocated by the *caller*, after this call returns).

    // `tick_region`'s signature is entirely unchanged:
    pub fn tick_region(&self, region: &mut RegionState, pool: &crate::pool::RcWorkerPool, transport: &dyn rc_messaging::Transport) -> TickReport;
    // Its body (Implementation steps) gains conditional wraps at every wave-dispatch and
    // inline-call site (Context) using `self.metrics.as_ref()` — `None` means every wrap
    // helper is skipped entirely (not even the clock read happens), preserving exact
    // pre-existing performance and behavior when metrics are not opted into.
}
```

### `crates/scheduler/src/region_manager.rs` (modify — one new private field, one new constructor; `spawn_region`/`tick_region`/`record_synthetic_tick`/`execute_merge`/`execute_split` bodies gain conditional metrics calls; every public signature's *shape* unchanged)

```rust
use std::sync::Arc;
use crate::metrics::MetricsRegistry;

pub struct RegionManager<'e> {
    executor: &'e RcExecutor,
    regions: std::collections::HashMap<rc_messaging::RegionId, ManagedRegion>,
    directory: crate::directory::RegionDirectory,
    id_alloc: crate::directory::RegionIdAllocator,
    tick_budget_ms: f64,
    metrics: Option<Arc<MetricsRegistry>>,   // new field
}

impl<'e> RegionManager<'e> {
    // `new` unchanged in signature and behavior (`metrics: None`).

    /// As `new`, additionally attaching `metrics` — every region this manager spawns is
    /// auto-registered with `metrics` (Context), every tick's duration sample and every
    /// `LifecycleOutcome` this manager produces is forwarded into it. Not calling this
    /// constructor (using `new` instead) is behaviorally identical to before this blueprint.
    pub fn new_with_metrics(executor: &'e RcExecutor, tick_budget_ms: f64, metrics: Arc<MetricsRegistry>) -> Self;

    // Every other public method's signature (spawn_region, region, region_mut, region_ids,
    // neighbors_of, tick_region, record_synthetic_tick, force_split, force_merge) is
    // completely unchanged — see Implementation steps for their conditional body additions.
}
```

### `crates/scheduler/src/metrics/mod.rs`

```rust
pub mod attribution;
pub mod edf;
pub mod histogram;
pub mod lifecycle_journal;
mod os;
pub mod registry;
pub mod snapshot;

pub use attribution::{measure_inline, region_tagged_task, ATTRIBUTION_OVERHEAD_BUDGET_RATIO};
pub use edf::EdfViolation;
pub use histogram::{HistogramSnapshot, TickDurationHistogram};
pub use lifecycle_journal::{LifecycleEvent, LifecycleEventKind};
pub use registry::{
    MetricsConfig, MetricsRegistry, PoolUtilizationSample, TickCpuCost,
    NEAR_ZERO_CPU_THRESHOLD_RATIO, NEAR_ZERO_SUSTAINED_TICKS, CPU_EWMA_ALPHA,
};
pub use snapshot::{MetricsSnapshot, RegionMetricsSnapshot, StageMetricsSnapshot, write_snapshot_json};
```

### `crates/scheduler/src/metrics/attribution.rs`

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};
use rc_messaging::RegionId;
use crate::pipeline::Stage;
use crate::metrics::registry::MetricsRegistry;

/// This blueprint's stated production overhead design target (Context: "Overhead budget
/// and feature-gating") — an *always-on* cost, ten times tighter than PERF-D52's
/// Tracy-feature 5% ceiling because, unlike Tracy, this cannot be compiled out of a
/// release build.
pub const ATTRIBUTION_OVERHEAD_BUDGET_RATIO: f64 = 0.01;

/// Reads the calling thread's own cumulative CPU time via `crate::metrics::os`
/// (platform-dispatched, Context: "Clock discipline"). Falls back to a wall-clock
/// `Instant` reading — tagged so the caller can tell which clock actually produced a
/// given delta, though every consumer in this blueprint treats both uniformly as "elapsed
/// cost" — if the platform primitive is unavailable or fails. Never panics.
fn measure<R>(f: impl FnOnce() -> R) -> (R, Duration) {
    match crate::metrics::os::thread_cpu_time_now() {
        Some(before) => {
            let result = f();
            let after = crate::metrics::os::thread_cpu_time_now().unwrap_or(before);
            (result, after.saturating_sub(before))
        }
        None => {
            let start = Instant::now();
            let result = f();
            (result, start.elapsed())
        }
    }
}

/// Wraps `f` (a task about to be submitted to `RcWorkerPool`, either via `spawn`'s
/// `'static` bound or `run_batch`'s scoped `'a` bound — this signature is generic over
/// `'a` so both call shapes are satisfied by one function) so that, wherever it actually
/// executes, the elapsed cost (Context: "per-task region tagging") is attributed to
/// `(region, stage)` in `registry` before returning. The wrapper travels with the task
/// through work-stealing — attribution is correct regardless of which worker thread
/// ultimately runs it.
pub fn region_tagged_task<'a, F: FnOnce() + Send + 'a>(
    region: RegionId,
    stage: Stage,
    registry: Arc<MetricsRegistry>,
    f: F,
) -> Box<dyn FnOnce() + Send + 'a> {
    Box::new(move || {
        let (_, elapsed) = measure(f);
        registry.record_task_cpu_time(region, stage, elapsed);
    })
}

/// As `region_tagged_task`, for a closure called *directly* on the current thread
/// (Stage 1/4/10's single-worker inline call sites, Context) rather than dispatched
/// through the pool — returns `f`'s own return value unchanged.
pub fn measure_inline<R>(region: RegionId, stage: Stage, registry: &MetricsRegistry, f: impl FnOnce() -> R) -> R {
    let (result, elapsed) = measure(f);
    registry.record_task_cpu_time(region, stage, elapsed);
    result
}
```

### `crates/scheduler/src/metrics/os/mod.rs`, `windows.rs` (cfg windows), `linux.rs` (cfg linux)

```rust
// os/mod.rs — crate-private (no `pub`), matching M0-B04's own `pool/os/` convention.
#[cfg(target_os = "windows")]
pub(crate) use windows_impl::thread_cpu_time_now;
#[cfg(target_os = "linux")]
pub(crate) use linux_impl::thread_cpu_time_now;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub(crate) fn thread_cpu_time_now() -> Option<std::time::Duration> { None }
```

```rust
// os/windows.rs — GetThreadTimes(GetCurrentThread(), ...) -> kernel + user FILETIME,
// converted (100ns units) to Duration. `None` on any Win32 failure — never panics.
pub(crate) fn thread_cpu_time_now() -> Option<std::time::Duration>;
```

```rust
// os/linux.rs — nix::time::clock_gettime(ClockId::CLOCK_THREAD_CPUTIME_ID) -> TimeSpec,
// converted to Duration. `None` on `Errno` — never panics.
pub(crate) fn thread_cpu_time_now() -> Option<std::time::Duration>;
```

### `crates/scheduler/src/metrics/edf.rs`

```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use parking_lot::Mutex;
use rc_messaging::RegionId;

/// ARCH-D20's exact, checkable violation definition (Context: "EDF admission-violation
/// counter"). Nanosecond offsets are relative to `EdfTracker`'s own construction-time
/// epoch (`Instant` is not `Serialize`) — never negative in practice, since every
/// timestamp this type ever observes is taken after that construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EdfViolation {
    pub admitted_region: RegionId,
    pub admitted_deadline_ns_since_epoch: u64,
    pub waiting_region: RegionId,
    pub waiting_deadline_ns_since_epoch: u64,
    pub detected_at_ns_since_epoch: u64,
}

pub(crate) struct EdfTracker {
    waiting: Mutex<HashMap<RegionId, Instant>>,
    violation_count: AtomicU64,
    epoch: Instant,
}

impl EdfTracker {
    pub(crate) fn new() -> Self;

    /// `region` has just become due as of `now` — record it as waiting for admission.
    /// Overwrites any prior still-waiting entry for `region` (Context: "at most one
    /// outstanding readiness per region").
    pub(crate) fn record_deadline_ready(&self, region: RegionId, deadline: Instant);

    /// `region` has just been admitted to fulfil `deadline` at `admitted_at`. Returns
    /// every violation detected by *this* call (Context's exact rule); also increments
    /// the cheap cumulative `violation_count()`. Removes `region`'s own waiting entry.
    pub(crate) fn record_admission(&self, region: RegionId, deadline: Instant, admitted_at: Instant) -> Vec<EdfViolation>;

    pub(crate) fn violation_count(&self) -> u64;
}
```

### `crates/scheduler/src/metrics/histogram.rs`

```rust
use hdrhistogram::Histogram;

/// A bounded-memory streaming percentile tracker for one region's per-tick duration
/// samples — the production-suitable counterpart to `RegionTickHistogram::from_samples`'s
/// batch, end-of-run computation (M0-B06), which retains every raw sample and is only
/// viable for a bounded soak run, not indefinite production operation. This type does
/// **not** recompute ARCH-D19's EWMA (Context: "export don't reimplement") — it tracks a
/// separate, percentile-shaped view of the identical duration samples `ManagedRegion`
/// already EWMA-smooths.
pub struct TickDurationHistogram {
    inner: Histogram<u64>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct HistogramSnapshot {
    pub sample_count: u64,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

impl TickDurationHistogram {
    /// `Histogram::new_with_bounds(1, 10_000_000, 3)` — value range 1 microsecond to
    /// 10 seconds (generously above any plausible single-tick duration, including a
    /// badly-overrun one), 3 significant figures (`hdrhistogram`'s own recommended
    /// default). Fixed bucket count from this range — memory footprint is bounded
    /// regardless of how long the server runs or how many samples are recorded.
    pub fn new() -> Self;

    /// Records one tick's duration in milliseconds, converted to whole microseconds
    /// (sub-microsecond precision is not meaningful at tick granularity). Saturates to
    /// the configured range's max rather than panicking on an out-of-range value.
    pub fn record_ms(&mut self, duration_ms: f64);

    pub fn snapshot(&self) -> HistogramSnapshot;
}
```

### `crates/scheduler/src/metrics/lifecycle_journal.rs`

```rust
use std::collections::VecDeque;
use std::time::Instant;
use parking_lot::Mutex;
use rc_messaging::RegionId;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LifecycleEventKind {
    Split {
        old: RegionId, new_a: RegionId, new_b: RegionId,
        old_cell_count: usize, new_a_cell_count: usize, new_b_cell_count: usize,
        ewma_ms_at_trigger: f64,
    },
    Merged {
        old_a: RegionId, old_b: RegionId, new: RegionId,
        old_a_cell_count: usize, old_b_cell_count: usize,
        combined_ewma_ms_at_trigger: f64,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LifecycleEvent {
    pub kind: LifecycleEventKind,
    /// The triggering region's own `tick_counter` at the moment of the event.
    pub tick_counter: u64,
    pub wall_clock_ms_since_registry_start: u64,
}

pub(crate) struct LifecycleJournal {
    capacity: usize,
    buffer: Mutex<VecDeque<LifecycleEvent>>,
    start: Instant,
}

impl LifecycleJournal {
    pub(crate) fn new(capacity: usize) -> Self;

    /// Appends `event`; evicts the oldest entry first if already at `capacity`
    /// (Context: "Merge/split lifecycle journal" — a memory bound, not a correctness
    /// concern for a harness that drains regularly).
    pub(crate) fn push(&self, event: LifecycleEvent);

    /// Drains and returns every currently-buffered event (the primary consumption API —
    /// a harness accumulates its own full-run history across repeated drains).
    pub(crate) fn drain(&self) -> Vec<LifecycleEvent>;
}
```

### `crates/scheduler/src/metrics/registry.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::{Mutex, RwLock};
use rc_messaging::RegionId;
use crate::pipeline::Stage;
use crate::metrics::edf::{EdfTracker, EdfViolation};
use crate::metrics::histogram::TickDurationHistogram;
use crate::metrics::lifecycle_journal::{LifecycleEvent, LifecycleJournal};

/// Reused verbatim from ARCH-D19 (M0-B06's own already-established constant) for this
/// blueprint's *new* CPU-time EWMA instance (Context: "Why thread-CPU-time... two-clock
/// design") — a distinct instance from `ManagedRegion`'s own tick-duration EWMA, not a
/// second computation of the same one.
pub const CPU_EWMA_ALPHA: f64 = 0.2;
/// Context: "Near-zero dedicated CPU" — 2% of `tick_budget_ms`.
pub const NEAR_ZERO_CPU_THRESHOLD_RATIO: f64 = 0.02;
/// Context: "Near-zero dedicated CPU" — reuses ARCH-D6's own split-hysteresis window.
pub const NEAR_ZERO_SUSTAINED_TICKS: u32 = 40;
/// Context: "Merge/split lifecycle journal" — a seed default.
pub const DEFAULT_LIFECYCLE_JOURNAL_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Debug)]
pub struct MetricsConfig {
    pub lifecycle_journal_capacity: usize,
    pub near_zero_cpu_threshold_ratio: f64,
    pub near_zero_sustained_ticks: u32,
    pub cpu_ewma_alpha: f64,
    /// Must match the `RegionManager`/`ManagedRegion` this registry is attached to
    /// (ARCH-D7 — `50.0` in production).
    pub tick_budget_ms: f64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            lifecycle_journal_capacity: DEFAULT_LIFECYCLE_JOURNAL_CAPACITY,
            near_zero_cpu_threshold_ratio: NEAR_ZERO_CPU_THRESHOLD_RATIO,
            near_zero_sustained_ticks: NEAR_ZERO_SUSTAINED_TICKS,
            cpu_ewma_alpha: CPU_EWMA_ALPHA,
            tick_budget_ms: 50.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TickCpuCost {
    pub total_ns: u64,
    /// Indexed by `Stage as u8 - 1` (Stages are numbered 1..=11).
    pub per_stage_ns: [u64; 11],
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PoolUtilizationSample {
    pub worker_count: usize,
    pub hard_cap: usize,
    pub active_worker_count: usize,
    pub backlog_depth: usize,
    /// `worker_count as f64 / hard_cap as f64` — ARCH-D18 headroom tracking.
    pub size_utilization_ratio: f64,
    /// `active_worker_count as f64 / worker_count.max(1) as f64` — saturation at the
    /// pool's *current* size.
    pub busy_fraction: f64,
    pub at_hard_cap: bool,
}

/// The central, always-on (Context: "Overhead budget and feature-gating") metrics
/// registry. `Send + Sync`; every mutation method takes `&self`, shared across every
/// RC-WorkerPool worker thread via `Arc<MetricsRegistry>`.
pub struct MetricsRegistry {
    config: MetricsConfig,
    regions: RwLock<HashMap<RegionId, Arc<RegionCpuState>>>,
    edf: EdfTracker,
    journal: LifecycleJournal,
    histograms: RwLock<HashMap<RegionId, Mutex<TickDurationHistogram>>>,
    tick_ewmas: RwLock<HashMap<RegionId, f64>>,   // ManagedRegion's own value, passed through
}

struct RegionCpuState {
    per_stage_tick_delta_ns: [AtomicU64; 11],
    lifetime_total_ns: AtomicU64,
    cpu_ewma_ns: Mutex<Option<f64>>,
    consecutive_near_zero_ticks: AtomicU64,
}

impl MetricsRegistry {
    pub fn new(config: MetricsConfig) -> Arc<Self>;

    /// Registers `region` for tracking. Idempotent. Called once per region by
    /// `RegionManager::spawn_region` (and again, with a fresh id, by
    /// `execute_merge`/`execute_split` for the region(s) a lifecycle event produces).
    pub fn register_region(&self, region: RegionId);
    /// Removes `region`'s tracking state — a merge/split permanently retires the old id
    /// (M0-B06's own invariant); this is the metrics-side mirror of that retirement.
    pub fn unregister_region(&self, region: RegionId);

    /// The hot-path call `region_tagged_task`/`measure_inline` make on every task
    /// completion — one `fetch_add`, `O(1)`. A read-lock-guarded `HashMap` lookup (the
    /// map's *write* path is cold, `register_region`-only). A call for an unregistered
    /// `region` is a silent no-op — never panics — since a dropped bookkeeping sample is
    /// preferable to crashing a live tick over an observability gap.
    pub fn record_task_cpu_time(&self, region: RegionId, stage: Stage, elapsed: Duration);

    /// Called once per region per tick, after every stage of that tick has run
    /// (`RegionManager`'s driver, Implementation steps): reads-and-resets that region's
    /// per-stage-tick deltas, updates the CPU-time EWMA and the near-zero
    /// consecutive-tick counter, and returns the just-completed tick's own per-stage/
    /// total CPU cost. A no-op (returns `TickCpuCost::default()`) for an unregistered region.
    pub fn end_tick_attribution(&self, region: RegionId) -> TickCpuCost;

    /// `true` iff `region`'s CPU-time EWMA has stayed at or under
    /// `config.near_zero_cpu_threshold_ratio * config.tick_budget_ms` for at least
    /// `config.near_zero_sustained_ticks` consecutive ticks (Context: "Near-zero
    /// dedicated CPU"). `false` for an unregistered region — never panics.
    pub fn is_near_zero_dedicated_cpu(&self, region: RegionId) -> bool;

    /// ARCH-D18 pool-utilization snapshot (Context) — reads `pool`'s own already-public
    /// (plus this blueprint's two new) getters; touches no state of its own.
    pub fn sample_pool_utilization(&self, pool: &crate::pool::RcWorkerPool) -> PoolUtilizationSample;

    pub fn record_deadline_ready(&self, region: RegionId, deadline: Instant);
    /// Returns every violation this specific admission detected (Context: "EDF
    /// admission-violation counter"); `edf_violation_count()` is the cheap cumulative total.
    pub fn record_admission(&self, region: RegionId, deadline: Instant, admitted_at: Instant) -> Vec<EdfViolation>;
    pub fn edf_violation_count(&self) -> u64;

    /// Forwards `duration_ms` into this region's `TickDurationHistogram` and records
    /// `tick_duration_ewma_ms` (already computed by `ManagedRegion` — Context: "export
    /// don't reimplement", never recomputed here) for the next `snapshot()` call.
    /// Auto-creates that region's histogram on first call if `register_region` was
    /// already invoked for it; a silent no-op otherwise (same unregistered-region rule
    /// as `record_task_cpu_time`).
    pub fn record_tick_duration_sample(&self, region: RegionId, duration_ms: f64, tick_duration_ewma_ms: Option<f64>);

    pub fn record_lifecycle_event(&self, event: LifecycleEvent);
    pub fn drain_lifecycle_events(&self) -> Vec<LifecycleEvent>;

    /// Assembles the full `MetricsSnapshot` (`metrics::snapshot`) — the machine-readable
    /// series a harness polls (Context). Draining the lifecycle journal is part of this
    /// call (its events become `MetricsSnapshot::lifecycle_events_since_last_snapshot`).
    pub fn snapshot(&self, pool: &crate::pool::RcWorkerPool) -> crate::metrics::snapshot::MetricsSnapshot;
}
```

### `crates/scheduler/src/metrics/snapshot.rs`

```rust
use rc_messaging::RegionId;
use crate::metrics::registry::PoolUtilizationSample;
use crate::metrics::histogram::HistogramSnapshot;
use crate::metrics::lifecycle_journal::LifecycleEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegionMetricsSnapshot {
    pub region_id: RegionId,
    pub cpu_time_ewma_ms: Option<f64>,
    pub cpu_time_last_tick_ms: f64,
    pub near_zero_dedicated_cpu: bool,
    /// Read straight from `ManagedRegion::tick_duration_ewma_ms()`, never recomputed.
    pub tick_duration_ewma_ms: Option<f64>,
    pub tick_duration_histogram: Option<HistogramSnapshot>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct StageMetricsSnapshot {
    /// `Stage as u8` (1..=11) — kept as a plain integer rather than adding
    /// `serde::Serialize`/`Deserialize` to M0-B05's `Stage` enum, to avoid touching that
    /// already-merged file at all for this cosmetic purpose.
    pub stage_index: u8,
    pub total_cpu_time_ms_across_regions: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub captured_at_unix_ms: u64,
    pub pool: PoolUtilizationSample,
    pub edf_violation_count: u64,
    pub regions: Vec<RegionMetricsSnapshot>,
    pub per_stage: Vec<StageMetricsSnapshot>,
    pub lifecycle_events_since_last_snapshot: Vec<LifecycleEvent>,
}

/// Writes `snapshot` as pretty JSON to `path`, creating parent directories as needed —
/// this blueprint's own resolution of "the machine-readable series the harness
/// consumes" (Context), sibling in spirit to M0-B06's `SoakReport`/M0-B08's
/// `tier_result::write_to`, but a *periodic* snapshot rather than a single end-of-run
/// report: a harness calls `MetricsRegistry::snapshot()` plus this function on its own
/// polling cadence (not fixed by this blueprint — a sibling M6 harness blueprint's call).
pub fn write_snapshot_json(path: &std::path::Path, snapshot: &MetricsSnapshot) -> std::io::Result<()>;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus `crates/scheduler/src/metrics/{mod.rs, attribution.rs, edf.rs, histogram.rs, lifecycle_journal.rs, registry.rs, snapshot.rs, os/{mod.rs,windows.rs,linux.rs}}` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments, and constant *values* stay exactly as specified), plus the `worker_pool.rs`/`executor.rs`/`region_manager.rs`/`lib.rs` diffs' new signatures similarly stubbed (their *existing* bodies are untouched by the test changeset — only the new additions are stubbed), plus both `Cargo.toml` edits. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/scheduler/tests/`.

Every test file below defines its own synthetic marker types/helpers directly in the file, reusing `tests/common/mod.rs`'s existing `empty_bootstrap`/`MockTransport` (M0-B05) where a real `RcExecutor`/`RegionManager` is needed.

### `crates/scheduler/tests/metrics_pool_utilization.rs`

`pool_utilization_sample_reports_worker_count_and_hard_cap`: `RcWorkerPool::with_config(RcWorkerPoolConfig { baseline: 2, hard_cap: 6, mode: Deterministic { fixed_size: 4 }, .. })`. `MetricsRegistry::new(MetricsConfig::default())`. `let sample = registry.sample_pool_utilization(&pool);` assert `sample.worker_count == 4`, `sample.hard_cap == 6`, `sample.backlog_depth == 0`, `sample.at_hard_cap == false`, `sample.size_utilization_ratio` within `1e-9` of `4.0/6.0`. Block all 4 workers via M0-B04's own `block_workers` gate-helper pattern (reused, not redefined — copy the pattern from `pool_resize_hysteresis.rs`'s own helper into this test file's local scope); re-sample; assert `active_worker_count == 4` and `busy_fraction` within `1e-9` of `1.0`. Release the gate, `wait_idle()`.

### `crates/scheduler/tests/metrics_edf_violations.rs`

Every test constructs a fresh `MetricsRegistry::new(MetricsConfig::default())` and works with plain `RegionId`/`Instant` values directly — no real `RcExecutor`/pool needed (Context: "EDF admission-violation counter" is a pure feed-in contract).

1. `edf_violation_counter_zero_on_clean_run` — three regions `a, b, c` with deadlines `t, t+10ms, t+20ms` (earliest first: `a`). Call `record_deadline_ready` for all three (any order). Admit in earliest-deadline-first order: `record_admission(a, t, now)`, `record_admission(b, t+10ms, now)`, `record_admission(c, t+20ms, now)`. Assert `registry.edf_violation_count() == 0` after every call, and each `record_admission` call's own returned `Vec<EdfViolation>` is empty.
2. `edf_violation_counter_flags_deliberate_violation` — two regions `a` (deadline `t`), `b` (deadline `t - 10ms`, i.e. **more** overdue). `record_deadline_ready` for both. Admit `a` **first** (deliberately wrong order): `let violations = registry.record_admission(a_id, t, now);` assert `violations.len() == 1` and `violations[0].waiting_region == b_id`; assert `registry.edf_violation_count() == 1`. Then admit `b`: `record_admission(b_id, t - 10ms, now)`; assert its own returned violations are empty (nothing left waiting with an earlier deadline) and `registry.edf_violation_count()` stays `1` (not double-counted).
3. `edf_admission_removes_the_waiting_entry` — one region `a`; `record_deadline_ready(a, t)`; `record_admission(a, t, now)`. A second, independent `record_admission(a, t2, now2)` for a *new* deadline `t2` with no prior `record_deadline_ready(a, t2)` call sees nothing to compare against (an admission for a region that was never marked waiting is not itself a violation source against anyone) — assert its own returned violations are empty.
4. `edf_deadline_ready_overwrites_a_prior_still_waiting_entry` — `registry.record_deadline_ready(a_id, base)` (`a`'s first, early readiness at `t = base`). `registry.record_deadline_ready(a_id, base + Duration::from_millis(100))` (a second readiness call for the same still-unadmitted region — Context's "at most one outstanding readiness" rule: this must **overwrite**, not add to, the first). `registry.record_deadline_ready(b_id, base + Duration::from_millis(50))` (`b`'s deadline sits strictly *between* `a`'s two readiness calls, deliberately, so this test can actually distinguish "overwrote" from "did not overwrite"). Admit `b` first, at its own deadline: `let violations = registry.record_admission(b_id, base + Duration::from_millis(50), now);`. If the overwrite took effect, `a`'s tracked deadline is now `base + 100ms` (later than `b`'s), so admitting `b` first is correct EDF order — assert `violations.is_empty()`. (Had the overwrite **not** taken effect, `a`'s stale `base` deadline would still be earlier than `b`'s `base + 50ms`, and this assertion would instead observe exactly one violation naming `a` — this is what makes the test an actual proof of the overwrite, not merely a restatement of tests 1–2's two-region case.) Then admit `a` at its true, current deadline: `registry.record_admission(a_id, base + Duration::from_millis(100), now);` assert its own returned violations are empty (`b` was already removed by its own admission, nothing else is left waiting).

### `crates/scheduler/tests/metrics_cpu_attribution.rs` (integration — real `RcWorkerPool`)

Shared helper: `fn busy_spin_micros(us: u64)` — copy of M0-B06's own `busy_spin` (spins via a `black_box`-guarded accumulator, polling `Instant::now()` every 256 iterations) redefined locally in this test file (this blueprint's `src/` deliverables have no dependency on `synthetic_load.rs`'s test-only helper, and duplicating ~10 lines is preferable to adding a cross-module production dependency just for a test fixture).

1. **`cpu_attribution_matches_pinned_synthetic_cost`** (the mandatory attribution-accuracy test): `RcWorkerPool::new(4)`; `let registry = MetricsRegistry::new(MetricsConfig::default());` `registry.register_region(hot_id); registry.register_region(quiet_id);`. Build `tasks: Vec<Box<dyn FnOnce() + Send + '_>>`: 10 tasks tagged `(hot_id, Stage::EntityAiPhysics)` each `busy_spin_micros(200)` (pinned total ≈ 2000 µs = 2 ms), interleaved with 1 task tagged `(quiet_id, Stage::EntityAiPhysics)` doing `busy_spin_micros(200)` (pinned total = 200 µs — a deliberately larger single-task cost than the theoretical minimum, chosen to keep this test's own signal comfortably above whatever platform clock-quantization noise the Windows leg's `GetThreadTimes` primitive may carry at the smallest task sizes, per Context's own honestly-flagged uncertainty there) — all built via `attribution::region_tagged_task(..., Arc::clone(&registry), || busy_spin_micros(..))`. `pool.run_batch(tasks)`. `let hot_cost = registry.end_tick_attribution(hot_id); let quiet_cost = registry.end_tick_attribution(quiet_id);`. Define the error bound `fn bound(pinned: Duration) -> Duration { pinned.mul_f64(0.2).max(Duration::from_micros(100)) }` (Context's stated ±20%-or-100µs-floor rule). Assert `hot_cost.total_ns` is within `bound(Duration::from_micros(2000)).as_nanos()` of `2_000_000`; assert `quiet_cost.total_ns` is within `bound(Duration::from_micros(200)).as_nanos()` of `200_000`.
2. **Coalesced-path near-zero sub-case, same test function, continued:** with `MetricsConfig { tick_budget_ms: 50.0, .. Default::default() }` (the production default already — near-zero threshold `= 1000 µs`, comfortably above the quiet region's `200 µs` pinned cost and comfortably below the hot region's `2000 µs`), assert `registry.is_near_zero_dedicated_cpu(quiet_id) == false` immediately after only **one** `end_tick_attribution` call (the near-zero check requires `NEAR_ZERO_SUSTAINED_TICKS` consecutive under-threshold ticks — one sample is not enough). Loop: repeat the quiet region's single 200µs-task dispatch + `end_tick_attribution(quiet_id)` for a further 39 rounds (40 total); assert `is_near_zero_dedicated_cpu(quiet_id) == true` after the 40th. Separately, repeat the hot region's 10×200µs-task dispatch + `end_tick_attribution(hot_id)` for 40 rounds; assert `is_near_zero_dedicated_cpu(hot_id) == false` throughout (2 ms is `4%` of the `50 ms` budget — above the `2%` threshold by design, Context's stated rationale for choosing `2%`).
3. `unregistered_region_cpu_attribution_is_a_silent_no_op` — `record_task_cpu_time` and `end_tick_attribution` called against a `RegionId` never passed to `register_region`; assert no panic, `end_tick_attribution` returns `TickCpuCost::default()`.

### `crates/scheduler/tests/metrics_overhead.rs`

**`attribution_overhead_within_budget_at_realistic_granularity`** (the mandatory overhead self-measurement test): `RcWorkerPool::new(4)` (fixed, deterministic — no elastic resize noise). `const N: usize = 2_000; const TASK_MICROS: u64 = 50;` (a realistic per-task cost, matching ARCH-D19's own named batch-granularity order of magnitude, Context's stated rationale for why this is measured here and not at degenerate near-zero task size). Build `plain: Vec<Box<dyn FnOnce()+Send+'_>>` of `N` tasks each `busy_spin_micros(TASK_MICROS)`, time `pool.run_batch(plain)` via `Instant`, call it `t_plain`. Build `wrapped` identically but each task passed through `attribution::region_tagged_task(fixed_region_id, Stage::EntityAiPhysics, Arc::clone(&registry), || busy_spin_micros(TASK_MICROS))`; time `pool.run_batch(wrapped)`, call it `t_wrapped`. Assert `(t_wrapped.as_secs_f64() - t_plain.as_secs_f64()) / t_plain.as_secs_f64() <= 0.05` — this test's own CI-stability tolerance (`5%`), deliberately looser than the `1%` production design target (`ATTRIBUTION_OVERHEAD_BUDGET_RATIO`) stated in Context, exactly mirroring `TickClock`'s own established "tight algorithmic claim, loose real-time smoke-test tolerance" pattern (M0-B04's `system_waiter_achieves_tolerance_over_a_short_real_time_run`). A doc comment on this test states both numbers and the mirrored-precedent explicitly.

### `crates/scheduler/tests/metrics_histogram_and_journal.rs`

1. `tick_duration_histogram_snapshot_reports_percentiles` — a fresh `TickDurationHistogram::new()`; `record_ms` for the fixed sample set `[5.0, 10.0, 10.0, 20.0, 45.0]` (5 samples); assert `snapshot().sample_count == 5`, `mean_ms` within `1e-6` of `18.0`, `max_ms == 45.0` (within `hdrhistogram`'s own stated 3-sig-fig precision tolerance — assert `(snapshot.max_ms - 45.0).abs() < 0.1`), and `p50_ms`/`p99_ms` are both `> 0.0` and `<= 45.0` (a loose sanity bound — this test proves the wrapper functions, not `hdrhistogram`'s own internal percentile algorithm, which is the library's job to get right, not this blueprint's to re-prove).
2. `lifecycle_journal_evicts_oldest_when_over_capacity` — `LifecycleJournal::new(3)`; push 5 distinct `LifecycleEvent`s (tagged, e.g., via `tick_counter` values `0..5` for identification); `drain()`; assert exactly 3 entries returned, and their `tick_counter`s are `[2, 3, 4]` (the 3 most recent — oldest-evicted-first, Context).
3. `lifecycle_journal_drain_empties_the_buffer` — push 2 events; `drain()` returns 2; a second `drain()` immediately after returns `0`.
4. `metrics_snapshot_round_trips_through_json` — build a `MetricsRegistry`, register one region, record a tick-duration sample and end one tick's CPU attribution, take a `snapshot(&pool)`, `write_snapshot_json` to a tempdir path, read the file back, `serde_json::from_str::<MetricsSnapshot>`, assert the round-tripped value's `regions.len() == 1` and its one entry's `region_id` matches.

### `crates/scheduler/tests/metrics_determinism_preserved.rs` (integration — reruns M0-B05's own scenario with metrics attached)

**`metrics_wrapping_does_not_change_tick_determinism`** — the exact setup of M0-B05's `determinism.rs`'s `same_final_state_across_worker_counts` (a single pre-existing entity holding `common::A(0)`, four mutually-conflicting `Query<&mut common::A>` systems in `DomainGroup::AiPhysics`, each `a.0 += 1`), but the executor is built via `RcExecutorBuilder::new(common::empty_bootstrap)....with_metrics(MetricsRegistry::new(MetricsConfig::default()))....build()`. For each of `RcWorkerPool::new(n)` with `n` in `{1, 2, 8}`: fresh executor/region, `tick_region` once, read the entity's final `A.0`. Assert all three runs produce `A.0 == 4` — the identical value M0-B05's own unmodified test asserts, proving this blueprint's instrumentation adds zero observable effect on `World` state regardless of worker count.

## Implementation steps

1. **`metrics/os/{mod,windows,linux}.rs`.** Implement the three platform primitives per Context's "Clock discipline" exactly — `GetThreadTimes` (Windows, confirm exact `windows` 0.62.2 wrapper signature via `cargo doc -p windows` at this step, per the crate's own honestly-flagged uncertainty) and `nix::time::clock_gettime(ClockId::CLOCK_THREAD_CPUTIME_ID)` (Linux). Observable: `cargo build -p rc-scheduler --all-features` succeeds for these files in isolation on both OS legs.
2. **`metrics/attribution.rs`.** Implement `measure`, `region_tagged_task`, `measure_inline` exactly per Deliverables — pure wrapping logic, no new state. Observable: compiles against step 1.
3. **`metrics/edf.rs`.** Implement `EdfTracker::{new, record_deadline_ready, record_admission, violation_count}` per Context's exact rule (iterate `waiting`, collect every entry with a strictly earlier deadline than the one being admitted into the returned `Vec`, `fetch_add` the count, then remove the admitted region's own entry). Observable: `metrics_edf_violations.rs`'s four cases pass (once wired through `registry.rs`, next steps — implement `registry.rs`'s EDF passthrough methods now too if sequencing them together is easier).
4. **`metrics/histogram.rs`.** Implement `TickDurationHistogram::{new, record_ms, snapshot}` — `Histogram::new_with_bounds(1, 10_000_000, 3).unwrap()`, `record_ms` converts to `(duration_ms * 1000.0).round() as u64` microseconds and calls `self.inner.saturating_record(..)` (or the installed `hdrhistogram` API's equivalent saturating-record method — confirm exact method name against the crate's own docs at this step, the same honestly-flagged verification discipline this blueprint's Context already applies to the Windows API). `snapshot` reads `self.inner.{len, mean, value_at_percentile(50.0), value_at_percentile(99.0), max}`, converting microseconds back to milliseconds (`/1000.0`). Observable: `metrics_histogram_and_journal.rs` test 1 passes.
5. **`metrics/lifecycle_journal.rs`.** Implement `LifecycleJournal::{new, push, drain}` — a `VecDeque` with `push_back` + `pop_front`-on-overflow, `drain` via `std::mem::take`. Observable: `metrics_histogram_and_journal.rs` tests 2–3 pass.
6. **`metrics/registry.rs`.** Implement `MetricsConfig::default`, `RegionCpuState`'s construction, and every `MetricsRegistry` method per Deliverables' doc comments and Context's formulas (`end_tick_attribution`'s CPU-EWMA update reuses the identical `0.2*sample + 0.8*prev` formula `ManagedRegion::record_tick_duration` already established, restated for a new state instance; the near-zero consecutive-tick counter increments when the just-updated EWMA is `<= near_zero_cpu_threshold_ratio * tick_budget_ms`, else resets to `0`, mirroring `ManagedRegion::record_tick_duration`'s own split-hysteresis reset-on-dip rule). `sample_pool_utilization` reads the pool's public getters (including this blueprint's two new ones, step 8) and computes the two ratios. Observable: `metrics_pool_utilization.rs`, `metrics_cpu_attribution.rs`, `metrics_overhead.rs` all pass.
7. **`metrics/snapshot.rs`.** Implement `write_snapshot_json` (`serde_json::to_string_pretty` + `std::fs::write`, creating parent dirs via `std::fs::create_dir_all` first) and complete `MetricsRegistry::snapshot`'s assembly (iterate `self.regions`, `self.histograms`, `self.tick_ewmas`; drain the journal; sum `per_stage_ns` across every region for `StageMetricsSnapshot`). Observable: `metrics_histogram_and_journal.rs` test 4 passes.
8. **`pool/worker_pool.rs` (M0-B04 extension).** Add `hard_cap()`/`active_worker_count()` — both trivial field reads (`self.hard_cap`, `self.active_workers.load(Ordering::Relaxed)`). Observable: `metrics_pool_utilization.rs` compiles and passes fully.
9. **`executor.rs` (M0-B05 extension).** Add the `metrics` field and `RcExecutorBuilder::with_metrics`. Modify `tick_region`'s body: at each of the two wave-dispatch `run_batch` call sites, when `self.metrics.is_some()`, wrap every task pushed into that call's `tasks: Vec<..>` with `metrics::region_tagged_task(region.id, <that call's Stage>, Arc::clone(metrics), original_task)` before pushing; at the Stage 1/4/10 inline call sites, when metrics are present, wrap the direct call with `metrics::measure_inline(region.id, <Stage>, metrics, || <original call>)`. When `self.metrics` is `None`, every wrap is skipped entirely — the exact pre-existing code path runs unchanged, not merely a wrap-with-a-no-op (avoiding even the branch-predictable cost of an unused `Option::is_some()` check inside the hot dispatch loop where it would matter — hoist the `is_some()` check once per `tick_region` call, not once per task). Observable: `metrics_determinism_preserved.rs` passes; every pre-existing M0-B05 test (`pipeline_ordering.rs`, `sync_points.rs`, `determinism.rs`) still passes unmodified.
10. **`region_manager.rs` (M0-B06 extension).** Add the `metrics` field and `RegionManager::new_with_metrics`. Modify `spawn_region` to call `metrics.register_region(id)` when present. Modify `tick_region`/`record_synthetic_tick` to call `metrics.record_tick_duration_sample(id, sample_ms, region.tick_duration_ewma_ms())` and `metrics.end_tick_attribution(id)` right after the existing `ManagedRegion::record_tick_duration` call. Modify `execute_merge`/`execute_split` to call `metrics.unregister_region` for every retired id, `metrics.register_region` for every new id, and `metrics.record_lifecycle_event(..)` with the full `LifecycleEventKind` populated from values already computed at that point (cell counts, triggering EWMA). Observable: every pre-existing `lifecycle_hysteresis.rs` test still passes unmodified (none of them construct a `RegionManager` via `new_with_metrics`, so none of these new calls ever execute for them).
11. **`lib.rs`.** Add `pub mod metrics;`. Observable: `cargo build -p rc-scheduler --all-features` succeeds workspace-wide with zero `todo!()` remaining.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/scheduler/tests/` is committed first, alongside `todo!()`-stubbed `src/metrics/**/*.rs` files (full field lists, full derives, full doc comments, exact constant values) and the `Cargo.toml`×2/`lib.rs`/`worker_pool.rs`/`executor.rs`/`region_manager.rs` diffs (new signatures stubbed; existing bodies of those three already-merged files untouched by the test changeset). The implementation changeset (Implementation steps) fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case listed in Acceptance tests, and must not weaken an assertion (in particular the `±20%-or-100µs-floor` error bound in `cpu_attribution_matches_pinned_synthetic_cost`, the `2%`/`40-tick` near-zero threshold constants, and the `5%` overhead-test tolerance must survive unchanged).

(b) **No new external dependencies beyond the pinned set named in this blueprint.** `hdrhistogram` (this blueprint's own new pin, root `[workspace.dependencies]` addition) is the *only* genuinely new crate. `serde_json` (promoted from dev-only to normal), `serde`, `parking_lot`, `tracing`, `nix` (Linux), `windows` (Windows) are all already-pinned, already-present `rc-scheduler` dependencies from M0-B04/B05/B06, reused unchanged. Do **not** add `dashmap`, `hdrhistogram-tokio`, `metrics` (the crates.io crate of that literal name), `opentelemetry`, `opentelemetry-otlp`, `tracing-opentelemetry`, `tracing-tracy`, or `tracy-client` — the last three are explicitly out of scope (Context: "Overhead budget and feature-gating" — Tracy/OTLP installation belongs to a future composition-root binary, not this library crate).

(c) **No Mojang or third-party reimplementation code.** Every algorithm here (the CPU-time EWMA, the EDF-violation rule, the near-zero threshold, the histogram/journal wrapping) is derived solely from `01-server-architecture.md`'s ARCH-D18–D20, `13-cluster-architecture.md`'s CLUSTER-D28, `14-performance-engineering.md`'s Section A/I, and this blueprint's own concrete, cited resolutions of what those decisions left open (ASSET-D18/D19/D30). `hdrhistogram`'s own algorithm (HdrHistogram, a public, independently-published data structure with prior art far outside any game-server codebase) is consumed as a dependency, never reimplemented or consulted as "inspiration" from any other project's histogram code.

(d) **This blueprint's instrumentation must never change `tick_region`'s observable behavior.** Every wrap in `executor.rs`/`region_manager.rs` is a pure, side-channel timing/counter observation — it must never alter which components a system reads/writes, when a `CommandQueue` is applied, or the content/order of any emitted `Message<RegionMessage>`. `metrics_wrapping_does_not_change_tick_determinism` (Acceptance tests) is this constraint's own binding proof, not merely a nice-to-have — a change that makes that test's result differ from M0-B05's own unmodified `same_final_state_across_worker_counts` assertion (`A.0 == 4`) is a hard blueprint violation regardless of what any other test says.

(e) **Unsafe-code policy — none permitted.** Every deliverable in this blueprint is safe Rust. `os/windows.rs`'s `GetThreadTimes` call is `unsafe fn` in the `windows` crate's own binding (an FFI boundary this blueprint cannot avoid, mirroring M0-B04's own precedent for its Windows timer/priority calls) and is the **sole** permitted `unsafe` site in this blueprint's deliverables, carrying the same `// SAFETY:` comment discipline M0-B04 already established for its own Windows FFI calls (the call has no preconditions beyond a valid, current thread handle, which `GetCurrentThread()` always provides). `os/linux.rs`'s `nix::time::clock_gettime` call is expected to be a safe wrapper (mirroring `nix::sched::sched_setscheduler`'s own already-established safety in M0-B04) — verify this at implementation time per Context's own flagged uncertainty; if the installed `nix` API requires `unsafe` there too, the identical `// SAFETY:` discipline applies as a second, narrowly bounded site. No other file in this blueprint's deliverables uses `unsafe` under any circumstance.

(f) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: the real-time EDF admission scheduler itself (a sibling M6 blueprint's job — this blueprint only defines and consumes the `record_deadline_ready`/`record_admission` feed-in contract); ARCH-D19's actual coalesced single-work-item dispatch for quiet regions (still not implemented anywhere, per M0-B06's own already-stated limitation — this blueprint's attribution mechanism is merely ready to measure it correctly the moment it exists); the bot-swarm load-testing harness that polls `snapshot()`/`write_snapshot_json` (a sibling M6 blueprint); `14`'s PGO/BOLT pipeline or Tier-3 SLO gating (a different M6 blueprint's scope per the milestone's own Scope bullet); OTLP/OpenTelemetry export or any `tracing-opentelemetry` wiring (CLUSTER-D28's own crate pins remain deferred, per `12`'s Open Questions, until whichever future blueprint actually implements cluster-mode observability — M7, not this one); Tracy subscriber installation (a composition-root binary's job, not `rc-scheduler`'s). Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler --all-features
cargo nextest run -p rc-scheduler
cargo test --doc -p rc-scheduler
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0 on both `ubuntu-24.04` and `windows-2025`. `cargo nextest run -p rc-scheduler` runs every pre-existing M0-B04/B05/B06 test case unmodified, plus this blueprint's own new files (`metrics_pool_utilization.rs`, `metrics_edf_violations.rs` × 4, `metrics_cpu_attribution.rs` × 3, `metrics_overhead.rs`, `metrics_histogram_and_journal.rs` × 4, `metrics_determinism_preserved.rs`) — all pass, with zero flakiness (the overhead test's own `5%` tolerance and the attribution-accuracy test's `±20%-or-100µs` bound are both deliberately generous for shared/virtualized CI runners, per Context/Constraints (a); no test in this suite depends on wall-clock timing accuracy tighter than those two stated, explicit tolerances). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
