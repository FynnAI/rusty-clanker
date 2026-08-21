# M6-B06 — Multi-Region Acceptance Harness

| Field | Content |
|---|---|
| ID | M6-B06 |
| Milestone | M6 — Scale & Optimization: Multi-Region Throughput |
| Prerequisites | M6-B01 (`rc_paritybot::loadtest` — `MultiRegionScenario`/`RegionCellGroup`/`BotGroupSpec`/`HotnessProfile`/`FaultInjectionEntry`/`parse_scenario`/`validate`/`extract_region_layout`/`write_region_layout_file`/`RegionLayoutSpec`/`extract_fault_injection_schedule`/`write_fault_injection_schedule`/`FaultInjectionSchedule`/`plan_bot_layout`/`MultiRegionScenarioConfig`/`MultiRegionScenarioReport`/`run_multi_region_scenario`, all reused unmodified except one additive field this blueprint adds to `MultiRegionScenarioConfig` (Context §C.3); the shipped worked-example scenario `crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron`, reused as this blueprint's own authoritative acceptance-run scenario, byte-for-byte, never forked; §B's still-open composition-root contract, restated in full and extended — never re-derived). M6-B02 (`rc_scheduler::metrics` — `MetricsRegistry`/`MetricsSnapshot`/`RegionMetricsSnapshot`/`PoolUtilizationSample`/`HistogramSnapshot`/`LifecycleEvent`, consumed here only through a local, file-boundary mirror per M6-B03 §D's own established "never a Cargo dependency edge" discipline — restated in full in Context §D, never imported directly). M6-B03 (`rc_test_harness::calibration`'s `CalibrationReport`/`CalibrationVerdict` shapes and, specifically, §H's governance-changeset path for landing a calibrated ARCH-D6/ARCH-D19 value into `docs/planning/01-server-architecture.md` — this blueprint's own §F check verifies that path was actually walked, restated in full, never re-implemented). M6-B04 (`xtask::reference_host::{TierId, ReferenceHostSpec, ReferenceHostTier, load_spec, tier_by_id, HostFingerprint, probe_host, match_tier, is_match, AuthoritativeRunReport, gate, write_authoritative_report_json}` — this blueprint is the "future blueprint" M6-B04's own Context names as the one obligated to call `gate` and write the wrapped value, restated as binding, discharged here). M6-B05 (`xtask::release::detect_region_layout_support` — reused unmodified as this blueprint's own fail-closed composition-root detector, never reimplemented; §L's fail-closed `ReleaseError::RegionLayoutContractMissing` pattern, restated as this blueprint's own template; and, since this blueprint lands last among the three `.github/workflows/ci.yml`-touching M6 blueprints, one narrow reconciliation edit to M6-B05's own already-merged `release` job — its `if:` condition, and nothing else about it — per Context §G.1). M0-B04 (`rc_scheduler::pool` — ARCH-D18 baseline/hard-cap formula, restated). M0-B06 (`rc_scheduler` region model — ARCH-D6/D7's exact pinned numbers and the M0 soak test's own `measured_tps = N/T`, `drift_ratio = measured_tps/target - 1.0`, `|drift_ratio| <= tolerance` convention, restated verbatim in Context §B.1 — the exact convention this blueprint's own AC1/AC3 TPS gates generalize to N concurrently-ticking regions). M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to, VERIFY_OUT_DIR, exit_code_for}`, `xtask::path_guard::{PROTECTED_PATHS, ChangesetType, check_paths}`, the `Changeset-Type` trailer convention — reused unmodified). M1-B04 (`ClientInformation`'s `view_distance: i8` server-bound Configuration field, protocol packet `0x00` — the exact mechanism this blueprint's own view-distance-10 requirement is realized through, restated). M3-B08 (`rc_test_harness::process::{ManagedServer, ManagedServerConfig, spawn_server}`, established additive-CLI-flag extension pattern; the `M<n>ReportResult` wraps `TierResult` via `#[serde(flatten)]` template, `build_report` pure-aggregation-tested-against-synthetic-inputs pattern, and the "perturbed input caught by the leg" harness-self-test convention — all restated as this blueprint's own template). M4-B09 (contrast: the no-oracle/no-`Mode` report shape, confirming this blueprint's own real-run leg belongs in the oracle/subprocess-dependent category, not M4-B09's every-PR one). M5-B10 (`rc_test_harness::throughput_log::{RegionTickLogEntry, parse_region_tick_log, RegionTickPercentileReport, analyze_region_tick_percentiles}`, reused unmodified for this blueprint's own diagnostic-only per-region tick-duration percentile reporting; the `throughput_log.rs` module — a `crates/testing/test-harness` module with zero `rc-scheduler` dependency, parsing data a real composition root writes — is this blueprint's own direct template for its new `metrics_snapshot_log.rs` sibling module; the `m5-acceptance` CI job's `schedule`/`workflow_dispatch`-only placement and its own honest "first meaningfully-green run is a later, separate milestone-acceptance signal, not this blueprint's own Done state" framing, restated). |
| Implements | `11-roadmap-milestones.md`'s M6 Acceptance Criteria 1–3, verbatim (Context §B) — this blueprint IS their concrete, agent-executable measurement. ARCH-D6/D7/D18/D19/D20 (verification targets only — never redefined here). TEST-D32/PERF-D58 (reference-host tiers — cross-referenced via M6-B04, never restated a second time except by exact citation). TEST-D37 (Tier-3/manual-trigger placement for the real run, restated in full, Context §G). TEST-D40 (machine-readable report format). TEST-D45/D46/D50/D52 (test-first changeset boundary, protected-path coverage, CI-is-authority, verifier re-run — restated). |
| Crates touched | `crates/testing/test-harness/` (`rc-test-harness`, additive: new `metrics_snapshot_log.rs` module; `process.rs` extended with three new `ManagedServerConfig` fields). `crates/testing/paritybot/` (`rc-paritybot`, additive: one new field on `MultiRegionScenarioConfig`; one new worked-example scenario `scenarios/loadtest/m6_acceptance_smoke.ron`). `xtask` (additive: `src/m6_report.rs`, one new `Command::M6Report` variant, `.github/workflows/ci.yml` extended). **Not** `crates/scheduler/`, **not** `crates/server/` — Context §A restates, in full, why the real composition-root/coalesced-dispatch work this blueprint's real run depends on is pinned as a contract on a still-future sibling blueprint, never implemented here. |
| Estimated scope | L |

## Goal & Done definition

Give M6's own three acceptance criteria (`11-roadmap-milestones.md`) a precise, agent-executable, machine-readable measurement and a single `xtask m6-report` entry point that produces it — exactly as M3-B08/M4-B09/M5-B10 did for their own milestones. Concretely: (1) a precise, restated pass/fail definition for each of the three criteria, resolving every ambiguity the milestone text itself leaves open (what "sustained... across all regions" means numerically, what "near-zero dedicated CPU" and "the coalesced-tick path actually engaged" require as evidence, what degradation-tolerance window a fault-injected region gets before its own TPS drop must be observed); (2) one small, precisely-specified extension to the still-open M6-B01 §B composition-root contract (a `--metrics-snapshot-log` flag and one new field on the real `RegionMetricsSnapshot`) that a future composition-root/coalesced-dispatch blueprint must satisfy for the real run to become checkable — restated as a binding contract addition, never implemented on the server side by this blueprint; (3) a local, dependency-free mirror of M6-B02's `MetricsSnapshot` shape plus the pure per-region TPS/pool/CPU/dispatch analysis functions the three criteria are evaluated against, each independently proven correct against synthetic data — including the three mandatory harness self-tests ("an artificially capped pool must fail criterion 1," "a fake that burns CPU in the quiet region must fail criterion 2," "a fake where siblings also degrade must fail criterion 3"); (4) `xtask m6-report`, wired to reuse M6-B01's scenario/fanout machinery, M6-B05's fail-closed composition-root detector, and M6-B04's reference-host fingerprint gate — the real, full-scale run's own final report is always an `AuthoritativeRunReport<M6ReportResult>` (M6-B04), never a bare, ungated claim; (5) a smaller, proportionally-scaled companion scenario for PR-tier smoke exercise of the harness's own plumbing (never of the real 20-TPS claim itself, which cannot be verified without the still-missing composition root); (6) a governance-landed check confirming M6-B03's calibrated ARCH-D6/ARCH-D19 values have actually replaced `01`'s seed defaults via a real governance changeset — honestly reported as still-failing today, since no such changeset has landed as of this blueprint's own drafting, exactly mirroring M5-B10's own "a correctly-reported failure, not a bug" framing for a dependency this blueprint's own Tier-1 gate does not wait on.

This blueprint does **not** implement the real `RegionManager`-driven, network-facing, many-region composition root on `rusty-clanker-server`, and does **not** implement ARCH-D19's actual coalesced single-work-item dispatch mechanism — both remain, as of every blueprint through M6-B05, real, separate, not-yet-written work (Context §A). This blueprint's own Tier-1 Done state is proven entirely against synthetic data and a stub `--help` fixture; the real, full 200-bot/15-minute/reference-host-gated acceptance run is wired, correct-by-construction, and fails closed with an actionable message until that future work lands — the identical, now five-times-established "harness proven hermetically now, real green is a later, separate signal" split this whole M6 lineage (and M3/M5 before it) already uses.

Done when:

- [ ] `cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask`, using **only** synthetic in-memory data or the shipped RON scenario files — no real `rusty-clanker-server` build, no real oracle, no real EDF/coalesced-dispatch composition root, required to go green.
- [ ] Every pre-existing M6-B01/M6-B04/M6-B05 `xtask`/`rc-paritybot`/`rc-test-harness` test still passes, byte-for-byte unmodified.
- [ ] The three mandatory harness self-tests (`artificially_capped_pool_fails_ac1`, `cpu_burning_quiet_region_fails_ac2`, `siblings_also_degrade_fails_ac3`) all pass, each proving the named failure mode is actually caught, not merely asserted possible.
- [ ] `cargo run -p xtask -- m6-report --help` prints usage with zero panics.
- [ ] `cargo run -p xtask -- m6-report --scenario <path> --out-dir <dir>` (no `--server-bin`) validates the scenario, writes the derived `region-layout.ron`/`fault-injection-schedule.ron` artifacts, and exits 0 for both shipped scenarios (`eight_region_mixed.ron`, `m6_acceptance_smoke.ron`) — no real connection is attempted, mirroring M6-B01 §K's identical two-mode shape.
- [ ] `cargo run -p xtask -- m6-report --scenario <path> --out-dir <dir> --server-bin <stub-binary-lacking-the-contract> --reference-tier dev-workstation` fails closed with the exact, actionable `RegionLayoutOrMetricsSnapshotContractMissing` message, exit non-zero, `target/verify/m6-acceptance.json` reporting `status: "fail"` — proven without building a real `rusty-clanker-server`.
- [ ] `m6_acceptance_smoke.ron` validates (`rc_paritybot::loadtest::validate`), has exactly 8 `RegionCellGroup`s, exactly one `BotGroupSpec` with `bot_count == 0`, and exactly one `FaultInjectionEntry`.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changeset (labeled per Constraints) — every new path already falls under an existing `PROTECTED_PATHS` row, proven by this blueprint's own `path_guard_already_covers_m6_b06s_own_new_paths` test.
- [ ] `.github/workflows/ci.yml`'s `on.workflow_dispatch.inputs` block carries the new `job` choice input, and `reference-host-gate`'s and M6-B05's `release` job's own `if:` conditions each check it (Context §G.1) — a `workflow view`/YAML-parse check, not a runtime CI assertion (neither job has a runner to dispatch to yet).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-test-harness -p rc-paritybot` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). This blueprint's own edit to `.github/workflows/ci.yml` — extending M6-B04's already-`workflow_dispatch`-only `reference-host-gate` job with the real `m6-report` step — adds no new Tier-1 job and does not change that job's own never-a-merge-gate status (M6-B04 Context: "Cloud-runner reality"); that job's first meaningfully-green run happens only once the composition-root/coalesced-dispatch work lands, mirroring M5-B10's `m5-acceptance` job's identical framing — not a condition of this blueprint's own Done state.

## Context (self-contained)

### §A — The dependency this blueprint does *not* wait on, restated and extended

Every M6 blueprint through M6-B05 states, in its own words, that no reviewed blueprint has yet built the real `RegionManager`-driven, network-facing, many-region composition root on `rusty-clanker-server` (M6-B01 §B; M6-B02's Scope-boundary note; M6-B03 §A; M6-B05 §L). The identical statement is still true at this blueprint's own drafting, and a second, closely related gap sits alongside it: **ARCH-D19's actual coalesced single-work-item dispatch for a quiet region is likewise not implemented by any landed blueprint** — M0-B06 itself flags this "not implemented, stubbed, or delegated to any other M0 blueprint," and M6-B02/M6-B03 both independently confirm the gap is still open as of their own drafting.

This blueprint's own task — "wire M6's three acceptance criteria" — is real, bounded, buildable work distinct from either gap: defining precise pass/fail semantics, building the pure evaluation functions those semantics require, and wiring one orchestration entry point that reuses M6-B01's scenario/fanout machinery and M6-B05's fail-closed detector is fully achievable, and fully Tier-1-provable, without either piece of missing scheduler/server work existing. This blueprint therefore does the same thing every prior M6 blueprint already did with its own missing dependency: it pins the exact contract shape a future sibling blueprint must satisfy (Context §D), builds and proves its own machinery entirely against synthetic data and a stub fixture, and leaves the real run wired, correct-by-construction, and honestly fail-closed until that future work lands.

### §B — M6's three acceptance criteria, verbatim, and this blueprint's own precise reading of each

From `11-roadmap-milestones.md`, quoted in full:

1. *"20 TPS sustained across all regions for a 15-minute run with 200 simulated bots distributed across at least 8 independently-ticking regions at view distance 10, on the milestone's documented reference host, with RC-WorkerPool utilization staying under its hard cap (ARCH-D18)."*
2. *"A region with 0 players coalesces onto a shared worker (ARCH-D19's coalesced-tick path) and is measured, via per-region CPU attribution metrics, to contribute near-zero dedicated CPU."*
3. *"A fault-injection test deliberately overloads one region: sibling regions hold 20 TPS while only the overloaded region's own TPS degrades (ARCH-D7's 'only a region that cannot keep up degrades its own TPS'), confirmed automatically."*

**§B.1 — AC1, precise reading.** "Sustained... across all regions" is read as: **every one of the run's ≥8 regions independently satisfies M0-B06's own already-established convention** (restated verbatim, generalized from 8 synthetic regions to N real ones): for that region, over the observed run span, `measured_tps = N_ticks / T_seconds`, `drift_ratio = measured_tps / 20.0 - 1.0`, and `|drift_ratio| <= 0.01` (`TPS_TOLERANCE`, Deliverables) — an **average-rate** measure over the full sustained window, never a per-tick instantaneous bound, exactly matching M0-B06's own "an average-rate measurement over the sustained run, not a per-tick instantaneous bound" framing. This is deliberately **not** `14`'s own PERF-D59/TEST-D32 SLO-1 framing ("p99 per-region tick time ≤ 50 ms") — that is a *different*, percentile-of-tick-duration metric answering a different question ("how bad is the worst tick"), not "TPS sustained." This blueprint reports both: the drift-ratio check is the **gating** AC1a measure (the literal, direct reading of the milestone's own "TPS sustained" wording, and the identical convention M0's own headline acceptance criterion already established); M5-B10's own `analyze_region_tick_percentiles` (reused unmodified) supplies a **diagnostic-only** p50/p99/max tick-duration report per region, cross-referenced against `14`'s PERF-D59 per-stage/aggregate nominal-load budgets and TEST-D32's SLO-1 — informational, never itself gating AC1, mirroring M5-B10 §C's own established gating-vs-diagnostic layering discipline exactly. "Pool utilization staying under its hard cap (ARCH-D18)" is read as **AC1b**: every `PoolUtilizationSample` (M6-B02) sampled throughout the run has `at_hard_cap == false` — `at_hard_cap` is M6-B02's own field, `true` exactly when `worker_count == hard_cap` (ARCH-D18's own growth ceiling, M0-B04 — `sample_and_maybe_resize` never grows the pool past `hard_cap` by construction), reused, never redefined here.

**§B.2 — AC2, precise reading.** Split into two independently-checked sub-parts, both required: **AC2a** — the zero-player region's own `RegionMetricsSnapshot.near_zero_dedicated_cpu` (M6-B02, already a sustained-≥40-consecutive-tick derived boolean by that field's own definition — no further cross-snapshot aggregation is needed here) is `true` at the run's own final sampled snapshot for that region. **AC2b** — "the coalesced-tick path actually engaged," read literally and separately from AC2a's CPU-cost evidence (a region *could* show a near-zero CPU reading from many cheap fine-grained tasks rather than from the actual ARCH-D19 coalesced single-work-item path — AC2b is the mechanism-level proof AC2a alone cannot supply): the zero-player region's own dispatch evidence (Context §D's `last_tick_task_count` contract addition) reports `Some(1)` (exactly one tagged task dispatched that tick — the coalesced-path signature) at the run's own final sampled snapshot. **Honestly restated, not glossed over:** because the real coalesced-dispatch mechanism does not exist in any blueprint through M6-B05 (§A), `last_tick_task_count` is `None` in every real snapshot any composition root can produce today — AC2b therefore correctly, honestly reports `CoalescedDispatchEvidence::NotYetInstrumented` (Deliverables) rather than a false pass, and the overall AC2 case fails, until a future blueprint both builds the real mechanism and populates this field.

**§B.3 — AC3, precise reading, with an explicit degradation-tolerance window.** Two independently-checked sub-parts, both required: **AC3a (target degrades)** — using the identical `analyze_region_tps` machinery as AC1a, computed over the fault-injection window with a settle offset (`AC3_FAULT_SETTLE_TICKS = 40` — ARCH-D6's own already-established "2 seconds/40 ticks is how long 'sustained' means in this corpus," reused for consistency rather than inventing a fifth unrelated number, exactly mirroring M6-B02's own identical reuse of that same window for its own near-zero-CPU threshold): the overloaded region's own `drift_ratio`, computed over `[fault.tick_start + AC3_FAULT_SETTLE_TICKS, fault.tick_end)`, is `<= AC3_DEGRADED_DRIFT_THRESHOLD` (`-0.05` — a seed default, deliberately far below the ±1% healthy tolerance so genuine degradation is unambiguous, and comfortably clearable by a fault-injection multiplier M6-B01 §G's own text already states is chosen "large enough to push that region's own tick time well past the 50 ms budget for a sustained window"). **AC3b (siblings hold)** — every *other* region's own `drift_ratio`, computed over the identical fault window, satisfies AC1a's own `|drift_ratio| <= 0.01` healthy tolerance — the literal "siblings hold 20 TPS" reading, with **zero** relaxation during the fault window (this is the entire point of ARCH-D7's isolation claim: a sibling's own tolerance band does not widen just because a neighbor is overloaded).

### §C — The pinned scenarios: reuse, never fork

**§C.1 — The real, authoritative run.** `crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron` (M6-B01, already shipped) is reused byte-for-byte as this blueprint's own real AC1/AC2/AC3 scenario — the identical fixture M6-B05 §E.1 already pinned as the canonical PGO/BOLT profile-collection workload, for the identical reason: 8 non-adjacent 1-cell regions, 200 bots across a five-profile hotness mix, one region with `bot_count == 0` (AC2's literal shape), one `FaultInjectionEntry` naming exactly one region at a sustained high multiplier partway through the run (AC3's literal shape), `merge_split_enabled: false`, `duration_ticks: 18_000` (15 real minutes at the harness's own 50 ms logical tick, M6-B01 §F). This blueprint authors **no** new full-scale scenario file — reusing the one maintained artifact that already satisfies all three acceptance criteria simultaneously is what keeps "the acceptance workload and the profiling workload are one maintained artifact" true in the strongest sense, mirroring M6-B05 §E.1's own identical rationale verbatim.

**§C.2 — The PR-tier smoke companion, a genuinely separate, smaller worked example.** A runtime override of `eight_region_mixed.ron`'s own `duration_ticks` (rather than authoring a second file) was considered and rejected: `FaultInjectionEntry.tick_start`/`tick_end` are absolute tick numbers fixed *inside* the scenario's own real 18,000-tick run; truncating `duration_ticks` alone (without also rescaling the fault window) would silently make the fault injection never fire in a compressed run, and proportionally rescaling every tick-bearing field generically is exactly the kind of "clever runtime scaling of hand-authored data" this corpus's own established convention avoids (M6-B01 §C: "one file, hand-authored or code-generated, versioned"). This blueprint instead ships a genuinely separate, independently-authored, proportionally-smaller worked example, `crates/testing/paritybot/scenarios/loadtest/m6_acceptance_smoke.ron` (new): 8 `RegionCellGroup`s (identical non-adjacent 1-cell layout discipline to `eight_region_mixed.ron`), `bot_groups` summing to **35** bots across the identical five-`HotnessProfile` mix as `eight_region_mixed.ron` (one `IdleStandaround` group at `bot_count: 0` — the same AC2 shape, at smaller scale; the other seven groups at `bot_count: 5` each), `merge_split_enabled: false`, one `FaultInjectionEntry` naming one `BuildBreakChurn`- or `CombatCluster`-hosting region with `tick_start: 600, tick_end: 1100, load_multiplier: 8.0`, `duration_ticks: 1_200` (60 real seconds). This is a **plumbing** smoke fixture only — Context §G restates precisely why a smaller bot count/duration proves the harness's own scenario-derivation/fanout/report-aggregation machinery works end to end, but can never, by itself, stand in for AC1's own literal 200-bot/15-minute/8-region claim.

**§C.3 — One additive field this blueprint adds to `MultiRegionScenarioConfig` (M6-B01 §H): the declared client view distance.** AC1's own "at view distance 10" clause is realized, per the vanilla protocol itself, entirely client-side: `ClientInformation.view_distance: i8` (M1-B04, server-bound Configuration packet `0x00`) is the field a connecting client declares, and is what the composition root's own chunk-send radius is computed from — there is no separate server-side "view distance" CLI surface this blueprint needs to invent. `MultiRegionScenarioConfig` (M6-B01 §H) gains one new field, `pub client_view_distance: u8` (no default — every caller states it explicitly, since a silently-wrong view distance would silently invalidate AC1's own literal wording); this blueprint's own `m6_report::run` always passes `10` for both scenarios (Context §B/§G). **Moderate-confidence flag, honestly stated, per this corpus's own established convention for exactly this class of uncertainty (mirroring M1-B06's repeated azalea-event-name caveat):** the exact azalea `ClientBuilder`/`Account` API surface for setting an outgoing `ClientInformation.view_distance` value before/at Configuration handshake time is not independently re-verified against azalea's own current documentation by this blueprint — confirm the exact call shape at implementation time; M6-B01's own `runner.rs` internal bot-task body (Implementation step 7 there) is the one call site this blueprint's own implementation changeset touches to thread the new field through.

### §D — Extending the M6-B01 §B contract: `--metrics-snapshot-log` and the `last_tick_task_count` field

M6-B01 §B restates a four-item contract binding on whichever future blueprint wires the real composition root (`--region-layout`, `RC_REGION_LAYOUT`, `--fault-injection-schedule`, `--region-lifecycle-log`) — reused here **unmodified**, item for item. This blueprint's own AC1b/AC2 checks need one piece of data that contract does not yet supply: a periodic, machine-readable dump of `rc_scheduler::metrics::MetricsRegistry::snapshot()`'s own output (M6-B02). This blueprint adds exactly **one** new item to that same contract, restated with the identical "binding on a future sibling blueprint, not implemented here" framing:

5. **`--metrics-snapshot-log <path>`** — a new `rusty-clanker-server` CLI flag. Every `METRICS_SNAPSHOT_POLL_INTERVAL_TICKS = 100` ticks (5 real seconds at 20 TPS — the identical cadence M5-B10 §I.2 already established for its own periodic per-bot polling, reused for consistency rather than inventing a sixth unrelated cadence), the composition root calls `MetricsRegistry::snapshot(&pool)` and appends one line of pretty-printed-as-a-single-line JSON, exactly `rc_scheduler::metrics::snapshot::MetricsSnapshot`'s own already-fixed `Serialize` shape (M6-B02), to `path`. Absent the flag, this hook never fires — zero overhead on every other build/test path, mirroring every prior optional-flag addition in this lineage (M3-B08's `--tick-log`, M5-B10's `--region-tick-log` family).

**One additive field this blueprint also pins on the real, future `RegionMetricsSnapshot` (M6-B02's own type, `crates/scheduler/src/metrics/snapshot.rs`):**

```rust
// Addition to rc_scheduler::metrics::snapshot::RegionMetricsSnapshot (M6-B02),
// binding on whichever future blueprint implements ARCH-D19's real coalesced
// single-work-item dispatch mechanism — NOT implemented by this blueprint,
// which touches no file under crates/scheduler/ (Constraints).
pub last_tick_task_count: Option<u32>,
// `Some(n)` — the count of `region_tagged_task`-wrapped tasks this region's
// most recently completed tick actually dispatched through RC-WorkerPool
// (`n == 1` is the coalesced-path signature; `n > 1` is the ordinary
// fine-grained-batch signature, M6-B02's own existing per-task tagging
// mechanism already counts these correctly regardless of dispatch shape,
// Context: "Coalesced-tick-path CPU accounting," M6-B02). `None` until that
// future mechanism exists — never a placeholder `Some(0)`/`Some(1)` guess.
```

This blueprint's own `metrics_snapshot_log.rs` (Deliverables, `crates/testing/test-harness/`) defines a **local, field-for-field mirror** of `MetricsSnapshot`/`RegionMetricsSnapshot`/`PoolUtilizationSample`/`HistogramSnapshot` — including this one additive field, present from day one on the mirror side even though the real type does not carry it yet — never a direct `rc-scheduler` dependency, exactly mirroring M6-B03 §D's own "file boundary, never a Cargo dependency edge" rule for a `crates/testing/*` crate (restated: `rc-scheduler` has never depended on, nor been depended on by, any `crates/testing/*` crate through M6-B03's own drafting, and this blueprint introduces no exception).

### §E — The local `MetricsSnapshot` mirror and pure per-region TPS analysis

```rust
// crates/testing/test-harness/src/metrics_snapshot_log.rs — public API surface

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MetricsSnapshotEntry {
    pub captured_at_unix_ms: u64,
    pub pool: PoolUtilizationSampleMirror,
    pub edf_violation_count: u64,
    pub regions: Vec<RegionMetricsSnapshotMirror>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct PoolUtilizationSampleMirror {
    pub worker_count: usize,
    pub hard_cap: usize,
    pub active_worker_count: usize,
    pub backlog_depth: usize,
    pub size_utilization_ratio: f64,
    pub busy_fraction: f64,
    pub at_hard_cap: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RegionMetricsSnapshotMirror {
    pub region_id: u64,
    pub cpu_time_ewma_ms: Option<f64>,
    pub near_zero_dedicated_cpu: bool,
    pub tick_duration_ewma_ms: Option<f64>,
    pub tick_duration_histogram: Option<HistogramSnapshotMirror>,
    /// Context §D's own additive contract field — `None` until a future
    /// blueprint's real coalesced-dispatch mechanism populates it.
    pub last_tick_task_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct HistogramSnapshotMirror {
    pub sample_count: u64,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

/// Parses `path` as newline-delimited JSON `MetricsSnapshotEntry` records —
/// identical "skip a malformed line, never abort the whole parse" tolerance
/// every prior NDJSON parser in this lineage already established
/// (`parse_tick_log`, `parse_region_tick_log`).
pub fn parse_metrics_snapshot_log(path: &std::path::Path) -> std::io::Result<Vec<MetricsSnapshotEntry>>;

/// Pure: every distinct `region_id` present anywhere in `entries`, ascending,
/// deduplicated.
pub fn distinct_region_ids(entries: &[MetricsSnapshotEntry]) -> Vec<u64>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionTpsResult {
    pub region_id: u64,
    pub measured_tps: f64,
    pub drift_ratio: f64,
    pub within_tolerance: bool,
    /// `N` — the tick-count span the measurement covers (last snapshot's
    /// cumulative `tick_duration_histogram.sample_count` minus the first's).
    pub sample_span: u64,
}

/// Pure: M0-B06's own `measured_tps = N/T`, `drift_ratio = measured_tps/target - 1.0`,
/// `within_tolerance = drift_ratio.abs() <= tolerance` convention (Context
/// §B.1), generalized per-region. Finds `region_id`'s first and last entry
/// (by ascending `captured_at_unix_ms`) among `entries` whose own
/// `tick_duration_histogram` is `Some` (a region snapshotted before its own
/// first completed tick carries `None` and is skipped as a candidate
/// boundary — never treated as a zero-duration sample). `N = last.sample_count
/// - first.sample_count` (the real cumulative per-region tick counter,
/// M6-B02, monotonic since that region's own `register_region` call —
/// never reset mid-run), `T = (last.captured_at_unix_ms -
/// first.captured_at_unix_ms) / 1000.0`. Returns `None` if fewer than two
/// `Some`-histogram entries exist for `region_id`, or if the computed `T <=
/// 0.0` — both are "insufficient data to measure," reported honestly by the
/// caller (`evaluate_ac1`/`evaluate_ac3`, `xtask::m6_report`) as a hard
/// failure, never silently treated as a pass.
pub fn analyze_region_tps(
    entries: &[MetricsSnapshotEntry],
    region_id: u64,
    target_tps: f64,
    tolerance: f64,
) -> Option<RegionTpsResult>;
```

### §F — The calibration-governance-landed check

`11-roadmap-milestones.md`'s own M6 goal, restated: *"replace `01`'s seed threshold defaults with calibrated values."* M6-B03 §H builds the full governance path for landing one — but, per M6-B03 §A/Open Questions, `--mode real-sweep` (the mode that would actually drive a real calibration sweep against real reference-host measurements) is **not implemented by M6-B03 itself**, explicitly deferred to "a future sibling blueprint... once the composition root it depends on exists." Since that composition root does not exist through this blueprint's own drafting either (§A), **no real calibration run has ever been possible, and no governance changeset landing a calibrated ARCH-D6/ARCH-D19 value has ever landed** — this is a genuine, structural, currently-open gap this blueprint states honestly rather than papering over.

```rust
/// Pure: a deliberately simple, conservative substring check (mirroring
/// `detect_region_layout_support`'s own identical discipline, M6-B05) —
/// `true` iff `arch_doc_text` contains the literal marker
/// `"Calibrated against"` (M6-B03 §H item 3's own worked example rationale
/// text, "Calibrated against the M6 reference-host load profile, M6-B03" —
/// a governance changeset landing a real calibrated value is expected to
/// carry a one-sentence rationale containing this literal phrase, per M6-B03
/// §H item 3's own binding text). A false positive (this phrase appearing
/// for an unrelated reason) is judged acceptably unlikely for a document
/// this small and this tightly governed (TEST-D46 protects it); a false
/// negative only ever means "this blueprint's own check is a little too
/// eager to report the gap still open," never "silently claims a landed
/// value that never landed."
pub fn calibration_values_landed(arch_doc_text: &str) -> bool;
```

`xtask::m6_report::run` reads the real, repository-committed `docs/planning/01-server-architecture.md` (resolved via the identical `CARGO_MANIFEST_DIR`-parent workspace-root convention M6-B04's own `reference_host::run` already established) and calls `calibration_values_landed` on its content — this is expected, honestly, to report `false` today (Acceptance tests' own `calibration_governance_case_currently_fails_against_the_real_committed_doc`, mirroring M5-B10's own identical "a correctly-reported failure, not a bug" framing for its own still-unsatisfied dependency). `--calibration-report <path>` (repeatable CLI flag, Deliverables) lets a caller attach one or more already-produced `xtask calibrate`-output `calibration-report.json` paths (M6-B03) as supporting evidence in the final `M6ReportResult.calibration_report_paths` field — attached, never embedded into any doc prose, mirroring M6-B03 §H item 3's own binding custody rule verbatim.

### §G — CI tier placement, restated

`09-testing-quality.md`'s TEST-D37 Tier 3, restated verbatim (already quoted in full by M6-B04, restated once more here since this blueprint's own real run is the concrete thing that tier gates): *"manually triggered before cutting a version tag, real reference hardware — never GitHub-hosted shared runners, which are not representative for performance decisions."* This blueprint's own real invocation (`--server-bin` present) is Tier-3-shaped by construction: it needs a real, running, reference-host-fingerprint-matched `rusty-clanker-server`, a real 200-bot fanout, and a real 15-minute sustained window — none of which belongs inside a `< 10 min` Tier-1 PR budget (TEST-D37's own Tier-1 framing) even once the composition root exists. This blueprint's Deliverables extend M6-B04's already-`workflow_dispatch`-only `reference-host-gate` job (`.github/workflows/ci.yml`) with the real `m6-report` step, filling in the literal `TODO(future blueprint, per M6-B01 §B)` comment M6-B04 itself left there — never adding a second, competing job. No new Tier-1 CI job is added by this blueprint: the scenario-validation-only smoke path (`--server-bin` absent), the fail-closed-detection self-test, and every pure evaluator self-test all run inside the already-existing `gates`/`guardrails` Tier-1 jobs' own `cargo run -p xtask -- test`/`tier1` invocation, exactly mirroring every prior M6 blueprint's own "no new CI job — nothing real to gate yet" Done-state framing.

### §G.1 — Reconciling the `reference-host-gate`/`release` `workflow_dispatch` trigger overlap

By this blueprint's own drafting, `.github/workflows/ci.yml` carries two independent, already-merged `workflow_dispatch`-only jobs targeting this milestone: M6-B04's `reference-host-gate` (extended by this blueprint, above) and M6-B05's `release`. Both are gated solely by `if: github.event_name == 'workflow_dispatch'`, with no job-selecting input distinguishing them — a single manual dispatch fires the whole workflow, so an operator intending to exercise only one (say, cutting a release via M6-B05's pipeline) would also attempt to launch `reference-host-gate` at the same time, and vice versa, each needing its own, possibly different self-hosted runner label. This blueprint is the last of the three to land (Wave 3, `M6-B00-index.md`'s own dependency graph — it needs all five other M6 blueprints merged), so it is the one that reconciles the overlap, per that index's own stated resolution:

- The shared `on.workflow_dispatch.inputs` block gains one new choice input, `job`, alongside M6-B04's existing `tier` input: `type: choice, description: "Which workflow_dispatch job to run", options: [reference-host-gate, release], required: true, default: reference-host-gate`.
- `reference-host-gate`'s own `if:` condition becomes `github.event_name == 'workflow_dispatch' && inputs.job == 'reference-host-gate'` (Deliverables, below) — every other line of that job, including this blueprint's own newly-added `m6-report` step, is unchanged.
- M6-B05's already-merged `release` job's own `if:` condition is changed, by this blueprint, from `github.event_name == 'workflow_dispatch'` to `github.event_name == 'workflow_dispatch' && inputs.job == 'release'` — the one line of `release`'s own YAML this blueprint touches; every other line of that job is unchanged. This is the identical kind of narrow, reconciliation-only edit to a sibling blueprint's already-landed file M6-B04 itself models for M6-B05's `xtask::release::detect_region_layout_support` (reused unmodified, never reimplemented) — here applied to one YAML condition rather than a Rust function.

A single manual dispatch now runs exactly the job an operator selected via `job`, never both.

### §H — The M6 completion report

```json
{
  "declared_tier": "m6-acceptance",
  "fingerprint": { "...": "HostFingerprint, M6-B04" },
  "tier_match": [ "...": "Vec<FieldCheck>, M6-B04" ],
  "authoritative": true,
  "report": {
    "tier": "m6-acceptance",
    "status": "pass",
    "cases": [
      { "name": "AC1a_region_tps_within_one_percent_sustained", "status": "pass", "detail": "8/8 regions within +/-1% of 20 TPS over the full 900s window" },
      { "name": "AC1b_pool_utilization_under_hard_cap", "status": "pass", "detail": "worker_count never reached hard_cap across 180 sampled snapshots" },
      { "name": "AC2a_zero_player_region_near_zero_cpu", "status": "pass" },
      { "name": "AC2b_coalesced_dispatch_path_engaged", "status": "fail", "detail": "last_tick_task_count is None for every snapshot — ARCH-D19's coalesced-dispatch mechanism is not yet instrumented (see M6-B06 Context §B.2/§D)" },
      { "name": "AC3a_overloaded_region_degrades_within_window", "status": "pass" },
      { "name": "AC3b_sibling_regions_hold_tps", "status": "pass" },
      { "name": "M6_calibrated_values_landed_via_governance", "status": "fail", "detail": "no 'Calibrated against' marker found in docs/planning/01-server-architecture.md — see M6-B03 §H" }
    ],
    "scenario_path": "crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron",
    "region_count": 8,
    "bot_count": 200,
    "client_view_distance": 10,
    "run_duration_ticks": 18000,
    "per_region_tps": [ "...": "Vec<RegionTpsResult>, diagnostic" ],
    "zero_player_region_label": "spawn-quiet",
    "overloaded_region_label": "east-hot",
    "calibration_report_paths": []
  }
}
```

`M6ReportResult` (below) wraps `xtask::tier_result::TierResult` exactly as `M1ReportResult`/`M2ReportResult`/`M3ReportResult`/`M4ReportResult`/`M5ReportResult` already do — `status: Fail` the instant any one case is `Fail` (`TierResult::finalize`'s own already-established fail-on-any rule, unmodified). **The one deliberate lineage evolution this blueprint introduces, restated and justified, not merely asserted:** the real run's own final artifact is always `xtask::reference_host::AuthoritativeRunReport<M6ReportResult>` (M6-B04), never a bare `M6ReportResult` — M6 is the first milestone whose own acceptance-criterion text literally requires "the milestone's documented reference host" (AC1), so it is the first milestone whose own completion report must be gated by M6-B04's `gate` function rather than merely written; this discharges M6-B04's own Context obligation verbatim ("whichever future blueprint first assembles that real report must call `reference_host::gate`... and write the wrapped value — never the bare report"). The scenario-only path (no `--server-bin`) writes a bare, ungated `M6ReportResult`-wrapped `TierResult` instead — there is no real run to gate in that mode.

## Deliverables

### `crates/testing/test-harness/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod metrics_snapshot_log;
```

### `crates/testing/test-harness/src/metrics_snapshot_log.rs` (new)

Exactly Context §E's signatures.

### `crates/testing/test-harness/src/process.rs` (modify — extend `ManagedServerConfig`, additive only)

```rust
pub struct ManagedServerConfig {
    // ...every existing field from M1-B06/M2-B08/M3-B08/M5-B10 unchanged...
    /// New (M6-B06): passed as `--region-layout <path>` when `Some` (M6-B01 §B item 1, reused).
    pub region_layout: Option<std::path::PathBuf>,
    /// New (M6-B06): passed as `--fault-injection-schedule <path>` when `Some` (M6-B01 §B item 3, reused).
    pub fault_injection_schedule: Option<std::path::PathBuf>,
    /// New (M6-B06): passed as `--metrics-snapshot-log <path>` when `Some` (Context §D item 5).
    pub metrics_snapshot_log: Option<std::path::PathBuf>,
}
```

`spawn_server`'s own body gains three more conditional `["--flag", path]` argument pushes, identical shape to M3-B08's/M5-B10's own additions. `capture_stdout: true` (already an existing field, M3-B08) is reused, never re-added, so `RC_REGION_LAYOUT`'s stdout line (M6-B01 §B item 2) is observable exactly the way M3-B08's own `RC_REGION_COUNT` line already is.

### `crates/testing/paritybot/src/loadtest/runner.rs` (modify — one new field on `MultiRegionScenarioConfig`, additive)

```rust
pub struct MultiRegionScenarioConfig {
    // ...every existing field from M6-B01 unchanged (scenario, server_host, server_port, out_dir, resource_limits)...
    /// New (M6-B06, Context §C.3): the value every bot's own `ClientInformation.view_distance`
    /// field (M1-B04) declares at Configuration handshake time. No default —
    /// every caller states it explicitly.
    pub client_view_distance: u8,
}
```

The per-bot azalea task body (M6-B01's own Implementation step 7) gains one call setting this value before/at Configuration — implementer's freedom for the exact azalea call shape (Context §C.3's own moderate-confidence flag).

### `crates/testing/paritybot/scenarios/loadtest/m6_acceptance_smoke.ron` (new — worked example)

Exactly Context §C.2's shape: 8 regions, 35 bots total (one `IdleStandaround` group at `bot_count: 0`, seven other groups at `bot_count: 5` across the remaining four `HotnessProfile` variants), `merge_split_enabled: false`, one `FaultInjectionEntry` (`tick_start: 600, tick_end: 1100, load_multiplier: 8.0`), `duration_ticks: 1_200`.

### `xtask/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod m6_report;
```

### `xtask/src/m6_report.rs` (new)

```rust
use crate::tier_result::TierResult;
use rc_test_harness::metrics_snapshot_log::{
    MetricsSnapshotEntry, RegionTpsResult, analyze_region_tps, distinct_region_ids,
    PoolUtilizationSampleMirror,
};

pub const TPS_TOLERANCE: f64 = 0.01;                    // M0's ±1% convention, restated (Context §B.1)
pub const AC3_FAULT_SETTLE_TICKS: u64 = 40;              // ARCH-D6's own reused "sustained" window (Context §B.3)
pub const AC3_DEGRADED_DRIFT_THRESHOLD: f64 = -0.05;     // seed default (Context §B.3)
pub const METRICS_SNAPSHOT_POLL_INTERVAL_TICKS: u64 = 100; // Context §D item 5

pub const OUT_PATH: &str = "target/verify/m6-acceptance.json";

/// Pure: does `help_text` advertise BOTH `--region-layout` (via
/// `xtask::release::detect_region_layout_support`, M6-B05, reused unmodified)
/// AND `--metrics-snapshot-log` (this blueprint's own new item, Context §D)?
/// Both are required before this blueprint's own real run is attempted.
pub fn detect_m6_composition_root_support(help_text: &str) -> bool;

#[derive(Debug, thiserror::Error)]
pub enum M6ReportError {
    #[error(
        "rusty-clanker-server does not yet implement M6-B01 §B's --region-layout \
         contract and/or M6-B06's own --metrics-snapshot-log addition (Context §D) \
         — the real 200-bot/8-region/15-minute acceptance run cannot execute yet. \
         This is a known, tracked dependency gap (see M6-B06's own Context §A/§D), \
         not a bug in this harness. Run with no --server-bin to exercise the \
         scenario-validation-only smoke path instead."
    )]
    RegionLayoutOrMetricsSnapshotContractMissing,
    // ... other variants (build/spawn failure, log-parse I/O error) added by the
    // implementer as ordinary error handling; this variant's exact message text
    // is the one load-bearing, tested string.
}

/// Pure: `PoolUtilizationSampleMirror` for every sample must have `at_hard_cap
/// == false` (Context §B.1) — the aggregate AC1b verdict, plus the first
/// offending sample if any, for diagnostic detail.
pub fn evaluate_ac1_pool(samples: &[PoolUtilizationSampleMirror]) -> (bool, Option<PoolUtilizationSampleMirror>);

#[derive(Debug, Clone)]
pub struct Ac1Outcome {
    pub per_region_tps: Vec<RegionTpsResult>,
    pub all_regions_within_tolerance: bool,
    pub pool_stayed_under_hard_cap: bool,
    pub worst_pool_sample: Option<PoolUtilizationSampleMirror>,
    pub passed: bool,
}

/// Pure: for every id in `region_ids`, calls `analyze_region_tps(entries, id,
/// 20.0, TPS_TOLERANCE)` — a `None` result (insufficient data) counts as a
/// failure for that region, never a skip. `pool_stayed_under_hard_cap` via
/// `evaluate_ac1_pool` over every `entries[..].pool` in order.
/// `passed = all_regions_within_tolerance && pool_stayed_under_hard_cap`.
pub fn evaluate_ac1(entries: &[MetricsSnapshotEntry], region_ids: &[u64]) -> Ac1Outcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoalescedDispatchEvidence { Engaged, NotEngaged, NotYetInstrumented }

#[derive(Debug, Clone)]
pub struct Ac2Outcome {
    pub near_zero_cpu: bool,
    pub dispatch_evidence: CoalescedDispatchEvidence,
    pub passed: bool,
}

/// Pure: `near_zero_cpu` reads `regions[..].near_zero_dedicated_cpu` from the
/// LAST entry in `entries` carrying `zero_player_region_id` (Context §B.2 —
/// the field is already a sustained-window derived boolean; no further
/// cross-snapshot aggregation is needed). `dispatch_evidence`: from that same
/// last entry's `last_tick_task_count` — `Some(1) -> Engaged`, `Some(n) if n
/// != 1 -> NotEngaged`, `None -> NotYetInstrumented`. `passed = near_zero_cpu
/// && dispatch_evidence == Engaged`.
pub fn evaluate_ac2(entries: &[MetricsSnapshotEntry], zero_player_region_id: u64) -> Ac2Outcome;

#[derive(Debug, Clone)]
pub struct Ac3Outcome {
    pub target_tps_during_fault: Option<RegionTpsResult>,
    pub target_degraded: bool,
    pub sibling_tps_during_fault: Vec<RegionTpsResult>,
    pub siblings_held_tps: bool,
    pub passed: bool,
}

/// Pure: `fault_window_entries` is already filtered (by the caller — I/O-adjacent
/// wall-clock windowing, `run`'s own job, below) to the settle-offset fault
/// window (Context §B.3). `target_degraded = target_tps_during_fault.map(|r|
/// r.drift_ratio <= AC3_DEGRADED_DRIFT_THRESHOLD).unwrap_or(false)` (a `None`
/// target measurement is a failure, never a skip). `sibling_tps_during_fault`
/// = `analyze_region_tps(fault_window_entries, id, 20.0, TPS_TOLERANCE)` for
/// every `id` in `region_ids` except `overloaded_region_id` — a `None` result
/// for any sibling is likewise a hard failure. `siblings_held_tps = ` every
/// sibling's own `within_tolerance == true`. `passed = target_degraded &&
/// siblings_held_tps`.
pub fn evaluate_ac3(
    fault_window_entries: &[MetricsSnapshotEntry],
    region_ids: &[u64],
    overloaded_region_id: u64,
) -> Ac3Outcome;

#[derive(serde::Serialize)]
pub struct M6ReportResult {
    #[serde(flatten)]
    pub automated: TierResult,             // tier = "m6-acceptance"; 7 cases, Context §B/§F/§H's table
    pub scenario_path: String,
    pub region_count: usize,
    pub bot_count: u32,
    pub client_view_distance: u8,
    pub run_duration_ticks: u64,
    pub per_region_tps: Vec<RegionTpsResult>,
    pub zero_player_region_label: String,
    pub overloaded_region_label: String,
    pub calibration_report_paths: Vec<String>,
}

/// Pure aggregation (Acceptance tests exercise this directly against
/// already-computed `Ac1Outcome`/`Ac2Outcome`/`Ac3Outcome`/`bool` inputs — the
/// three mandatory harness self-tests all ultimately assert on this
/// function's own output, never merely on the lower-layer `evaluate_ac*`
/// functions in isolation, mirroring M3-B08's own established "aggregation
/// itself, not just its inputs, is proven" discipline). Builds the seven
/// cases from Context §B/§F/§H and `finalize`s the wrapped `TierResult`.
#[allow(clippy::too_many_arguments)]
pub fn build_report(
    scenario_path: &str,
    region_count: usize,
    bot_count: u32,
    client_view_distance: u8,
    run_duration_ticks: u64,
    ac1: &Ac1Outcome,
    ac2: &Ac2Outcome,
    ac3: &Ac3Outcome,
    calibration_landed: bool,
    zero_player_region_label: &str,
    overloaded_region_label: &str,
    calibration_report_paths: &[String],
) -> M6ReportResult;

/// Pure: scans `stdout` for a line matching `RC_REGION_LAYOUT=<json>` (M6-B01
/// §B item 2) and parses the JSON object into a label->`RegionId` map — `None`
/// if no such line is present or it fails to parse. Mirrors M3-B08's own
/// `parse_region_count_line` exactly.
pub fn parse_region_layout_stdout_line(stdout: &[String]) -> Option<std::collections::HashMap<String, u64>>;

pub struct M6ReportArgs {
    pub scenario: std::path::PathBuf,
    pub out_dir: std::path::PathBuf,
    /// `None`: validate + derive artifacts only, exit 0, no connection
    /// attempted (mirrors M6-B01 §K's identical two-mode shape). `Some`: the
    /// real, fail-closed-or-full-run path.
    pub server_bin: Option<std::path::PathBuf>,
    /// Required whenever `server_bin` is `Some` — no default, mirroring
    /// M6-B04's own `host-fingerprint --tier` CLI discipline of never
    /// silently assuming a tier.
    pub reference_tier: Option<String>,
    pub server_host: String,             // default "127.0.0.1"
    pub server_port: u16,                // default 25567
    pub calibration_report: Vec<std::path::PathBuf>,  // repeatable, Context §F
}

/// CLI entry point (`xtask m6-report`): parses+validates the scenario,
/// derives `region-layout.ron`/`fault-injection-schedule.ron` into
/// `out_dir` (M6-B01's own `extract_region_layout`/`write_region_layout_file`
/// / `extract_fault_injection_schedule`/`write_fault_injection_schedule`,
/// reused unmodified) and returns if `server_bin` is `None`. Otherwise:
/// builds the candidate binary's own `--help` text (a real subprocess call,
/// `<server_bin> --help`), checks `detect_m6_composition_root_support` —
/// `false` fails closed with `M6ReportError::RegionLayoutOrMetricsSnapshotContractMissing`
/// before spawning anything further. `true`: spawns the real server via
/// `rc_test_harness::process::spawn_server` with `region_layout`/
/// `fault_injection_schedule`/`metrics_snapshot_log` all `Some`,
/// `client_view_distance: 10` threaded into
/// `rc_paritybot::loadtest::MultiRegionScenarioConfig`, runs
/// `run_multi_region_scenario` for the scenario's own `duration_ticks`
/// inside one `tokio::runtime::Runtime::new()?.block_on(..)` (mirrors every
/// prior `m<n>_report.rs`'s identical isolation pattern), tears the server
/// down, parses `RC_REGION_LAYOUT` (`parse_region_layout_stdout_line`) to
/// resolve the scenario's own zero-bot/fault-injected region labels to real
/// `RegionId`s, parses the metrics-snapshot-log
/// (`metrics_snapshot_log::parse_metrics_snapshot_log`), computes the fault
/// window (scenario-start wall-clock, recorded locally at the moment
/// `run_multi_region_scenario` was invoked, plus `fault.tick_start +
/// AC3_FAULT_SETTLE_TICKS` / `fault.tick_end` at the scenario's own 50 ms
/// logical-tick period — an approximate, coarse windowing scheme, honestly
/// flagged: real network/scheduling jitter means this is not exact to the
/// millisecond, adequate given AC3's own generous, well-separated
/// tolerance/threshold margins, Context §B.3), calls `evaluate_ac1`/
/// `evaluate_ac2`/`evaluate_ac3`, reads+scans the real
/// `docs/planning/01-server-architecture.md` via `calibration_values_landed`,
/// calls `build_report`, then — always for the real-run path — wraps via
/// `xtask::reference_host::gate(report, xtask::reference_host::probe_host(),
/// declared_tier)` (declared_tier resolved from `reference_tier` via
/// `xtask::reference_host::{load_spec, tier_by_id, TierId::parse}`) and
/// writes the wrapped `AuthoritativeRunReport<M6ReportResult>` to `OUT_PATH`
/// via `xtask::reference_host::write_authoritative_report_json`. Returns the
/// matching `ExitCode` (`SUCCESS` iff `automated.status == Status::Pass` —
/// `authoritative` does not itself gate the exit code, mirroring M6-B04's own
/// framing that a non-authoritative report is still fully readable/useful,
/// never silently discarded).
pub fn run(args: &M6ReportArgs) -> std::process::ExitCode;
```

### `xtask/src/main.rs` (modify — one new `Command` variant, dispatched identically to every prior addition)

```rust
/// M6-B06: drives the M6 acceptance harness (200-bot/8-region/15-minute
/// throughput + coalescing + fault-injection-isolation) against a real,
/// freshly-spawned `rusty-clanker-server` and a declared reference-host tier,
/// and writes `target/verify/m6-acceptance.json`.
M6Report {
    #[arg(long)] scenario: std::path::PathBuf,
    #[arg(long)] out_dir: std::path::PathBuf,
    #[arg(long)] server_bin: Option<std::path::PathBuf>,
    #[arg(long)] reference_tier: Option<String>,
    #[arg(long, default_value = "127.0.0.1")] server_host: String,
    #[arg(long, default_value_t = 25567)] server_port: u16,
    #[arg(long)] calibration_report: Vec<std::path::PathBuf>,
},
```

### `.github/workflows/ci.yml` (modify — extend M6-B04's already-`workflow_dispatch`-only `reference-host-gate` job; add the `job` selector input (Context §G.1) and update `reference-host-gate`'s and M6-B05's `release` job's own `if:` conditions to match; every other line of every other job byte-for-byte unchanged)

Add, to the workflow's existing top-level `on.workflow_dispatch.inputs` block (alongside M6-B04's own `tier` input, unchanged): `job: { type: choice, description: "Which workflow_dispatch job to run", options: [reference-host-gate, release], required: true, default: reference-host-gate }` (Context §G.1).

```yaml
  reference-host-gate:
    # ...M6-B04's own steps unchanged through "Upload fingerprint result"...
    if: github.event_name == 'workflow_dispatch' && inputs.job == 'reference-host-gate'
    # (was `if: github.event_name == 'workflow_dispatch'`, M6-B04 — narrowed
    # per Context §G.1 so a manual dispatch aimed at M6-B05's `release` job
    # does not also launch this one.)

      - name: Build rusty-clanker-server (monolithic, release)
        run: cargo build --release -p rusty-clanker-server --no-default-features --features monolithic

      - name: m6-report
        run: |
          cargo run -p xtask -- m6-report \
            --scenario crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron \
            --out-dir target/m6-acceptance \
            --server-bin target/release/rusty-clanker-server \
            --reference-tier ${{ inputs.tier }}
        # Discharges M6-B04's own literal TODO comment (Context §G) — this is
        # the "real M6 acceptance load-test/SLO suite" step that comment
        # names, wrapped via xtask::reference_host::gate before writing, per
        # M6-B04's own binding instruction.

      - name: Upload m6-acceptance report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: m6-acceptance-${{ inputs.tier }}
          path: target/verify/m6-acceptance.json
          if-no-files-found: warn

  release:
    # ...M6-B05's own job, every line unchanged except the one below (Context §G.1)...
    if: github.event_name == 'workflow_dispatch' && inputs.job == 'release'
    # (was `if: github.event_name == 'workflow_dispatch'`, M6-B05 — narrowed
    # per Context §G.1 so a manual dispatch aimed at `reference-host-gate`
    # does not also launch this one.)
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test-authoring changeset is every file below, plus every new `src/*.rs` file in Deliverables with each function body `todo!()`-stubbed (struct/enum shapes, derives, doc comments, and constant *values* stay exactly as specified), plus the additive fields on `ManagedServerConfig`/`MultiRegionScenarioConfig` and the `m6-report` CLI variant, similarly stubbed. Per M6-B01/M6-B04/M6-B05's own established precedent, this changeset is exempt from `path-guard`'s protected-path check by construction. The governance changeset (Implementation steps, below; labeled `Changeset-Type: governance`, never `implementation`) fills in real bodies only and must not modify any test file listed below.

### `crates/testing/test-harness/tests/metrics_snapshot_log_parsing.rs`

1. `parse_metrics_snapshot_log_reads_valid_ndjson` — a temp file with 3 well-formed lines → 3 parsed entries, field-for-field correct.
2. `parse_metrics_snapshot_log_skips_malformed_lines` — 2 valid lines interleaved with 1 malformed line → exactly 2 parsed entries, no error.
3. `distinct_region_ids_returns_unique_sorted_ids` — entries covering region ids `[3, 1, 3, 2]` across several snapshots → `[1, 2, 3]`.

### `crates/testing/test-harness/tests/metrics_snapshot_log_tps_analysis.rs`

4. `analyze_region_tps_on_time_within_tolerance` — two synthetic entries for `region_id: 1`, `tick_duration_histogram.sample_count` stepping from `100` to `18100` (18,000 ticks) and `captured_at_unix_ms` stepping by exactly `900_000` (900s) → `measured_tps` within `1e-9` of `20.0`, `within_tolerance == true`.
5. `analyze_region_tps_lagged_region_outside_tolerance` — same tick delta over `1_080_000` ms (1080s, an 83.3%-of-real-time rate) → `measured_tps ≈ 16.67`, `within_tolerance == false`.
6. `analyze_region_tps_returns_none_for_unobserved_region` — entries covering only region ids `[1, 2]` → `analyze_region_tps(&entries, 99, 20.0, 0.01) == None`.
7. `analyze_region_tps_boundary_at_exactly_one_percent_passes` — a delta producing `drift_ratio == -0.01` exactly → `within_tolerance == true` (`<=`, not `<`).

### `xtask/tests/m6_report_contract_detection.rs`

8. `detect_m6_composition_root_support_true_when_both_flags_present` — a stub help text containing both `"--region-layout <PATH>"` and `"--metrics-snapshot-log <PATH>"` → `true`.
9. `detect_m6_composition_root_support_false_when_metrics_snapshot_log_absent` — help text with `--region-layout` present but `--metrics-snapshot-log` absent → `false` (proving both are independently required, not merely `--region-layout` alone as M6-B05's own narrower detector already checks).

### `xtask/tests/m6_report_ac1_evaluation.rs`

10. `evaluate_ac1_passes_on_healthy_synthetic_data` — 8 synthetic region ids, each with two snapshots yielding `measured_tps` within tolerance, and every `PoolUtilizationSampleMirror.at_hard_cap == false` → `Ac1Outcome.passed == true`.
11. **`artificially_capped_pool_fails_ac1`** (mandatory harness self-test) — identical healthy per-region TPS data as test 10, but one sampled `PoolUtilizationSampleMirror` has `worker_count == hard_cap` (`at_hard_cap: true`) → `Ac1Outcome.pool_stayed_under_hard_cap == false`, `Ac1Outcome.passed == false`, `all_regions_within_tolerance` still `true` (proving the two AC1 sub-checks are independently attributed, neither masking the other).
12. `evaluate_ac1_flags_specific_lagging_region_by_id` — 8 healthy regions plus one lagging region (drift beyond tolerance) → `Ac1Outcome.passed == false`, and `per_region_tps` names the specific lagging `region_id` with `within_tolerance == false` while every other entry reports `true`.

### `xtask/tests/m6_report_ac2_evaluation.rs`

13. `evaluate_ac2_passes_when_near_zero_and_dispatch_engaged` — the zero-player region's last snapshot has `near_zero_dedicated_cpu: true`, `last_tick_task_count: Some(1)` → `Ac2Outcome.passed == true`, `dispatch_evidence == Engaged`.
14. **`cpu_burning_quiet_region_fails_ac2`** (mandatory harness self-test) — the zero-player region's last snapshot has `near_zero_dedicated_cpu: false` (a fake that keeps burning CPU) → `Ac2Outcome.near_zero_cpu == false`, `Ac2Outcome.passed == false`.
15. `evaluate_ac2_reports_not_yet_instrumented_when_task_count_absent` — `near_zero_dedicated_cpu: true`, `last_tick_task_count: None` (the pre-M6-B07 state; M6-B07's coalesced-dispatch implementation populates this field in a complete build, and this case guards against running acceptance on a build without it) → `Ac2Outcome.dispatch_evidence == NotYetInstrumented`, `Ac2Outcome.passed == false` — proving AC2 correctly fails on the mechanism gap alone even when the CPU evidence alone would otherwise pass.

### `xtask/tests/m6_report_ac3_evaluation.rs`

16. `evaluate_ac3_passes_when_only_target_degrades` — fault-window entries: target region's `drift_ratio ≈ -0.30` (well past `-0.05`), every sibling's `within_tolerance == true` → `Ac3Outcome.passed == true`.
17. **`siblings_also_degrade_fails_ac3`** (mandatory harness self-test) — identical target-region data as test 16, but one sibling's own fault-window `drift_ratio` also falls outside `±0.01` → `Ac3Outcome.target_degraded == true`, `Ac3Outcome.siblings_held_tps == false`, `Ac3Outcome.passed == false` — proving the two AC3 sub-checks are independently attributed.
18. `evaluate_ac3_fails_when_target_does_not_degrade_enough` — target region's fault-window `drift_ratio == -0.02` (worse than healthy, but not past `AC3_DEGRADED_DRIFT_THRESHOLD`) → `Ac3Outcome.target_degraded == false`, `Ac3Outcome.passed == false`.

### `xtask/tests/m6_report_calibration_governance.rs`

19. `calibration_values_landed_detects_marker` — a synthetic doc-text string containing `"Calibrated against the M6 reference-host load profile, M6-B03"` → `true`.
20. `calibration_values_landed_false_when_marker_absent` — arbitrary unrelated doc text → `false`.
21. `calibration_governance_case_currently_fails_against_the_real_committed_doc` — reads the real, repository-committed `docs/planning/01-server-architecture.md` and calls `calibration_values_landed` on it directly → `false` today (an honestly-expected-red assertion, mirroring M5-B10's own identical "a correctly-reported failure, not a bug" framing — this test's own doc comment states this explicitly, and Constraints (f) forbids ever "fixing" it by weakening the assertion rather than by a real governance changeset landing).

### `xtask/tests/m6_report_region_layout_parsing.rs`

22. `parse_region_layout_stdout_line_finds_the_json_object` — `parse_region_layout_stdout_line(&["some other line".into(), r#"RC_REGION_LAYOUT={"spawn-quiet":3,"east-hot":7}"#.into()])` → `Some({"spawn-quiet": 3, "east-hot": 7})`.
23. `parse_region_layout_stdout_line_returns_none_when_absent` — no matching line → `None`.

### `xtask/tests/m6_report_smoke_scenario.rs`

24. `m6_acceptance_smoke_scenario_validates_and_has_eight_regions_with_one_zero_bot_group` — parses the shipped `m6_acceptance_smoke.ron`, `validate` returns `Ok(())`, `regions.len() == 8`, exactly one `bot_groups` entry has `bot_count == 0`, `bot_groups.iter().map(|g| g.bot_count).sum::<u32>() == 35`, exactly one `fault_injection` entry.
25. `m6_report_scenario_only_mode_writes_derived_artifacts` — `xtask m6-report --scenario <m6_acceptance_smoke.ron path> --out-dir <tempdir>` (no `--server-bin`) exits 0; `<tempdir>/region-layout.ron`/`<tempdir>/fault-injection-schedule.ron` both exist and parse back into `RegionLayoutSpec`/`FaultInjectionSchedule` (M6-B01); `target/verify/m6-acceptance.json` exists with `status: "pass"`.
26. `m6_report_real_run_fails_closed_when_contract_missing` — drives `m6_report::run` (or a lower-level function it calls, implementer's choice of the exact seam, mirroring M6-B05's identical test-14 discipline) with a stubbed "plain build produced a binary whose `--help` lacks `--metrics-snapshot-log`" fixture, `server_bin: Some(..)` → the process exits non-zero and `target/verify/m6-acceptance.json`'s reports `status: "fail"` with `detail` containing the exact substring `"M6-B01 §B"` and `"M6-B06's own Context §A/§D"` (Context's error text) — proving the actionable-message requirement mechanically.

### `xtask/tests/m6_report_build_report.rs`

27. `build_report_aggregates_all_seven_cases_and_serializes_with_flattened_fields` — every `evaluate_ac*` input passing, `calibration_landed: true`; serialize to `serde_json::Value` → top-level object has `tier`, `status`, `cases` (7 entries, exact names from Context §H) **and** `scenario_path`, `region_count`, `bot_count`, `client_view_distance`, `run_duration_ticks`, `per_region_tps`, `zero_player_region_label`, `overloaded_region_label`, `calibration_report_paths` as sibling keys, `status == "pass"`.
28. `build_report_status_is_fail_the_instant_any_case_fails` — every input passing except `ac2.passed == false` → `automated.status == Status::Fail`, and specifically the two `AC2*` cases (by name) are `Fail` while every `AC1*`/`AC3*` case is `Pass` (proving the failure is correctly attributed, not smeared).

### `xtask/tests/m6_report_path_guard_coverage.rs`

29. `path_guard_already_covers_m6_b06s_own_new_paths` — mirroring M6-B01/M6-B05's identical self-test exactly: `path_guard::check_paths(ChangesetType::Implementation, &["crates/testing/test-harness/src/metrics_snapshot_log.rs".into(), "crates/testing/paritybot/scenarios/loadtest/m6_acceptance_smoke.ron".into(), "xtask/src/m6_report.rs".into()])` → every path reports exactly one violation, all three against an already-existing row (`crates/testing/test-harness/**`, `crates/testing/paritybot/**`, `xtask/**` respectively) — `assert_eq!(violations.len(), 3)`, proving no `path_guard.rs` edit was needed.

## Implementation steps

1. **`crates/testing/test-harness/src/metrics_snapshot_log.rs`.** Implement the mirror types and `parse_metrics_snapshot_log`/`distinct_region_ids`/`analyze_region_tps` exactly per Context §E. Observable: `metrics_snapshot_log_parsing.rs` and `metrics_snapshot_log_tps_analysis.rs` pass.
2. **`process.rs`.** Add the three new `ManagedServerConfig` fields and `spawn_server`'s three conditional argument pushes. Observable: `cargo build -p rc-test-harness` still succeeds; existing call sites unaffected.
3. **`runner.rs`.** Add `client_view_distance` to `MultiRegionScenarioConfig` and thread it through the per-bot azalea task body (Context §C.3's own moderate-confidence flag — confirm the exact azalea call shape at this step). Observable: `cargo build -p rc-paritybot` still succeeds.
4. **`m6_acceptance_smoke.ron`.** Author exactly per Context §C.2/Deliverables. Observable: `m6_acceptance_smoke_scenario_validates_and_has_eight_regions_with_one_zero_bot_group` passes.
5. **`xtask/src/m6_report.rs` — pure pieces first.** `detect_m6_composition_root_support`, `evaluate_ac1_pool`, `evaluate_ac1`, `evaluate_ac2`, `evaluate_ac3`, `parse_region_layout_stdout_line`, `calibration_values_landed`, `build_report`. Observable: tests 8–21, 27–28 pass.
6. **`xtask/src/m6_report.rs` — `run`'s orchestration.** Implement the scenario-only path first (validate + derive artifacts, no connection), then the real-target path (`--help` probe, fail-closed gate, spawn/run/teardown, log parsing, fault-window computation, `reference_host::gate` wrapping, report write) per Context/Deliverables' own doc comment. Observable: tests 25–26 pass; no real `rusty-clanker-server` build is attempted by any test in this blueprint's own suite (test 26 uses a stubbed fixture, exactly mirroring M6-B05's test 14).
7. **`xtask/src/main.rs`.** Add the `M6Report` variant and its `match` arm. Observable: `cargo run -p xtask -- m6-report --help` prints usage.
8. **Path-guard coverage proof.** Add `xtask/tests/m6_report_path_guard_coverage.rs`. Observable: test 29 passes with zero edits to `xtask/src/path_guard.rs`.
9. **`.github/workflows/ci.yml`.** Extend M6-B04's `reference-host-gate` job exactly per Deliverables; add the `job` choice input to the shared `on.workflow_dispatch.inputs` block and narrow `reference-host-gate`'s and M6-B05's `release` job's own `if:` conditions to match (Context §G.1) — every other line of every other job's YAML untouched. Confirm the workflow file still parses (`gh workflow view ci.yml`) — not required to pass yet (Context §G).
10. **Run the full acceptance suite.** `cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask` — every test named above passes. Commit this blueprint's governance changeset with `Changeset-Type: governance` (Constraints).

## Constraints & forbidden actions

(a) **Test-first, changeset boundary.** All 29 acceptance tests above are written and committed before the functions/types they exercise exist (`todo!()`-stubbed where needed for a compiling red state). The subsequent implementation changeset never modifies any of the eight test files listed above, and never weakens, deletes, or `#[ignore]`s any case in them — in particular, `calibration_governance_case_currently_fails_against_the_real_committed_doc`'s own `false`-expecting assertion must survive unchanged until a real M6-B03 governance changeset actually lands (proven by that same test flipping to green on that day, never by editing the test).

(b) **Protected paths, and this blueprint's own changeset label.** Every file this blueprint's Deliverables touch already falls under an existing `PROTECTED_PATHS` row (`crates/testing/test-harness/**`, `crates/testing/paritybot/**`, `xtask/**` — proven by acceptance test 29) — per this lineage's own established convention, the entire changeset that creates this blueprint's files is labeled `Changeset-Type: governance`, never `implementation`.

(c) **No new external dependencies beyond the already-pinned set.** Every type/function this blueprint adds uses crates already present in the relevant `Cargo.toml` (`serde`/`serde_json`/`thiserror` in `rc-test-harness`; `serde`/`serde_json`/`ron` in `rc-paritybot`; `clap`/`serde_json`/`thiserror` already in `xtask`) — no new line is added to any `[dependencies]` table anywhere in this blueprint's own deliverables.

(d) **No Mojang or third-party reimplementation source.** Every numeric constant this blueprint introduces (`TPS_TOLERANCE`, `AC3_FAULT_SETTLE_TICKS`, `AC3_DEGRADED_DRIFT_THRESHOLD`, `METRICS_SNAPSHOT_POLL_INTERVAL_TICKS`, the smoke scenario's own bot/tick counts) is this blueprint's own original methodology choice, either directly reused from an already-established corpus convention (M0-B06's ±1% TPS rule, ARCH-D6's 40-tick sustain window) or a freshly-stated, justified seed default — never cross-checked against any Mojang or third-party source, since none makes a vanilla-parity claim.

(e) **This blueprint touches no file under `crates/scheduler/` or `crates/server/`.** Every Rust type this blueprint's own Context §D names as an addition to the *real* `RegionMetricsSnapshot`, and every new CLI flag it names as an addition to the *real* `rusty-clanker-server`, is a specification a future sibling blueprint implements — this blueprint's own `cargo build`/`cargo nextest` gates never depend on that future code existing, mirroring M6-B01 §B/M6-B05 §L's identical constraint verbatim.

(f) **The calibration-governance-landed case is a real, honest measurement, never a placeholder pass.** `calibration_values_landed` is called against the real, repository-committed `docs/planning/01-server-architecture.md` — the implementer must not hard-code, mock, or otherwise bypass this real file read in the shipped `m6_report::run` implementation (test-only fixtures are fine in acceptance tests 19–20; `run` itself always reads the real file).

(g) **Unsafe-code policy — none permitted.** Every deliverable in this blueprint is safe Rust — no `unsafe` block anywhere in this blueprint's own new code.

(h) **This blueprint's real-run path never spawns more than one real `rusty-clanker-server` process, and its own Tier-1 gate never spawns one at all** — mirroring M6-B01's own binding self-limit for its own real-connection tests, restated here: every one of this blueprint's 29 acceptance tests uses only synthetic in-memory data, the shipped RON scenario files (parsed, never connected against), or a stubbed `--help` fixture.

## Verification commands

```
cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features
cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask
cargo test --doc -p rc-test-harness -p rc-paritybot
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- m6-report --help
```

All run headless, identically, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43) — no real `rusty-clanker-server`, no oracle, no network access required for any of them. CI green on both OS legs, clean checkout, is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Open questions

- **The real 200-bot/8-region/15-minute/reference-host-gated acceptance run remains blocked** until a future sibling blueprint implements `rusty-clanker-server`'s `RegionManager`-driven multi-region composition root, wires `MetricsRegistry`/`RcExecutorBuilder::with_metrics`/`RegionManager::new_with_metrics` (M6-B02) into that composition root, implements ARCH-D19's real coalesced-dispatch mechanism and populates `last_tick_task_count` (Context §D), and implements `xtask calibrate --mode real-sweep` (M6-B03) plus a real governance changeset landing a calibrated ARCH-D6/ARCH-D19 value. This blueprint's own Done checklist does not depend on any of that landing (mirroring every prior M6 blueprint's identical framing); `.github/workflows/ci.yml`'s `reference-host-gate` job will continue to fail closed, correctly and informatively, until it does.
- **The fault-window's wall-clock computation (`m6_report::run`'s own "scenario-start-plus-tick-offset" arithmetic, Deliverables)** is deliberately approximate, not millisecond-exact, given real network/scheduling jitter — whether a future revision should instead derive the fault window from `--region-lifecycle-log`-adjacent server-side timestamps (more precise, but requiring a further composition-root contract addition) rather than harness-side wall-clock arithmetic is left open, since AC3's own tolerance/threshold margins are already generous enough that the current approach is adequate.
- **`AC3_DEGRADED_DRIFT_THRESHOLD = -0.05` and `AC3_FAULT_SETTLE_TICKS = 40`** are this blueprint's own seed defaults, carrying the identical "concrete number now, calibration-pending" status every other unpinned numeric threshold in this corpus carries — revisiting them once a real fault-injection run's own measured data exists is a natural candidate for a future M6-B03-style calibration pass, not built here.
- **Whether `M6ReportResult`'s own `AuthoritativeRunReport`-wrapping convention (Context §H) should become the standard shape every future `M<n>-report` adopts**, retroactively or not, for milestones with their own reference-host-pinned acceptance text, is left to whichever future milestone's own acceptance-harness blueprint next needs it — not retrofitted onto M1–M5's already-shipped, unwrapped report shapes by this blueprint.
