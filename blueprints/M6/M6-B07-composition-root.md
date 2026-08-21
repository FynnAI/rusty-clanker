# M6-B07 — Multi-Region Composition Root, EDF Admission & Coalesced Dispatch

| Field | Content |
|---|---|
| ID | M6-B07 |
| Milestone | M6 — Scale & Optimization: Multi-Region Throughput |
| Prerequisites | M0-B02 (`rc-messaging` — `RegionId`, `Address`, `Message<T>`, `RegionMessage`, `Transport` trait: `send`/`try_recv`, used exactly as fixed there). M0-B03 (`rc-transport-inproc::{InProcessTransport, InProcessTransportConfig}` — `register_region`/`deregister_region`, the composition-root calling convention M0-B06 named and this blueprint discharges). M0-B04 (`rc-scheduler::pool` — `RcWorkerPool`, `RcWorkerPoolConfig`, `PoolMode`, `TickClock`/`TickWaiter`/`SystemTickWaiter`/`TickTiming`/`SERVER_TICK_PERIOD`, ARCH-D18/19 elastic sizing — restated exactly; this blueprint adds one new `TickClock` method). M0-B05 (`rc-scheduler` RC-Executor — `RcExecutor`, `RcExecutorBuilder`, `RegionState`, `TickReport`, `DomainGroup`, `Stage`, the 11-stage pipeline and its two sync points — restated exactly; this blueprint adds one new `RcExecutor` method). M0-B06 (`rc-scheduler` region model — `GridCell`, `RegionDirectory`, `RegionIdAllocator`, `ManagedRegion`, `RegionManager`, `LifecycleOutcome`, `largest_connectivity_cut`, the exact ARCH-D6/D19 EWMA/hysteresis formulas and merge/split message-redirect protocol — restated exactly; this blueprint changes `RegionManager`'s and `RegionDirectory`'s *private* storage representation only, and adds new `&self`-taking methods alongside every existing one, unchanged). M1-B01 (`rusty_clanker_server::net::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection}` — the Tokio connection layer this blueprint's listener loop drives). M1-B04 (`rusty_clanker_server::net::{PlayerSession, PlayerSessionSink}` — the Configuration→Play hand-off contract this blueprint's composition root implements). M1-B05 (`crates/server/src/play/{world.rs, connection.rs}` — `HardcodedWorld`, `PlayerMarker`, `PendingJoin`, `enter_play`, `PlayerProfile` — restated exactly; this blueprint extracts `HardcodedWorld::new`'s system-registration sequence into a shared function without altering `HardcodedWorld`'s own observable behavior or any of its existing tests). M2-B05 (`rc-chunk-storage::{AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, PaletteThresholds}`, `rc-chunk-storage::lifecycle::{ChunkLifecycleManager, SaveIntervalTicks, SnapshotOutbox, snapshot_system_factory}`, `rc-scheduler::chunk_ticket::{TicketManager, PlayerTicketId, ChunkChurn}`, `crates/server/src/config.rs`'s `WorldConfig` — restated exactly; this blueprint instantiates one `ChunkLifecycleManager`/`TicketManager` pair per region instead of M2-B05's own single-region wiring inside `HardcodedWorld::new`). M4-B08 (`crates/server/src/play/player_transfer.rs` — `PlayerRouting`, `RegionQueueHandles`, `PlayerTransferPayload`, `combined_arrival_driver`, `build_player_entity_snapshot`/`try_decode_player_snapshot`; `crates/server/src/play/two_region_world.rs`'s own `TwoRegionWorld` as the two-region precedent this blueprint generalizes to N regions and retires as the *runtime* entry point without deleting or modifying it — every one of its own existing tests keeps passing unchanged). M6-B01 (§B's four-item composition-root contract — `--region-layout`/`RC_REGION_LAYOUT`/`--fault-injection-schedule`/`--region-lifecycle-log` — restated in full below and implemented for real by this blueprint; `rc_paritybot::loadtest`'s `MultiRegionScenario`/`RegionLayoutSpec`/`RegionCellGroup`/`FaultInjectionSchedule`/`FaultInjectionEntry`/`resolve_load_multiplier` *shapes*, restated as this blueprint's own local, production-side types per §A's dependency-direction note — never imported, since `rusty-clanker-server` cannot depend on the test-only `rc-paritybot` crate). M6-B02 (`rc-scheduler::metrics` — `MetricsRegistry`, `MetricsConfig`, `TickCpuCost`, `PoolUtilizationSample`, `RegionMetricsSnapshot`, `MetricsSnapshot`, `write_snapshot_json`, `record_deadline_ready`/`record_admission`/`edf_violation_count`, `region_tagged_task`/`measure_inline`, `is_near_zero_dedicated_cpu` — restated exactly; this blueprint is the "sibling M6 blueprint" that calls the EDF feed-in contract for real and populates the previously-`None`-only `last_tick_task_count` field). M6-B03 (`RcWorkerPool::with_resize_thresholds`/`ResizeThresholds`, `RegionManager::with_thresholds`/`with_thresholds_and_metrics`/`HysteresisThresholds` — restated exactly, wired from this blueprint's own new `[scheduler]` config table). M6-B05 (`xtask::release::detect_region_layout_support` — reused unmodified as the exact substring check this blueprint's own `--help` output must satisfy). M6-B06 (§D's fifth composition-root contract item, `--metrics-snapshot-log`, and the exact additive `RegionMetricsSnapshot.last_tick_task_count: Option<u32>` field shape — both restated in full below and implemented for real; `M6ReportResult`'s own real-run path, `crates/testing/test-harness/src/metrics_snapshot_log.rs`'s mirror types, and `xtask m6-report`'s fail-closed gate all become exercisable, unmodified, the instant this blueprint lands). |
| Implements | ARCH-D5/D6 (dynamic, on-demand region bootstrap from grid cells — this blueprint's own concrete resolution of "how does a region layout come to exist with no pinned file," restated in full, §B). ARCH-D7 (per-region independent tick clock — now driving genuinely concurrent, independently-paced regions for the first time). ARCH-D18/D19/D20 (RC-WorkerPool elastic sizing — reused unmodified; the EDF admission *decision* itself, and ARCH-D19's coalesced single-work-item dispatch — both implemented for real, for the first time, by this blueprint, §E/§F). ARCH-D21 (the Tokio runtime boundary — this blueprint's own composition root is the first to actually own it for a multi-region server). ARCH-D23 (`parking_lot` cold-path-only locking, restated and applied to this blueprint's own new concurrent bookkeeping). ARCH-D24/D25/D29 (directory/transport lifecycle wiring at merge/split — M0-B06's own named "composition-root crate" obligation, discharged for real, §G). ARCH-D9/D10/D11 (Stage-1 inbound drain — restated; border-halo chunk ticking across a region boundary remains explicitly out of scope, §L). CLUSTER-D26/D27 (monolithic-mode role resolution — restated, cluster mode is out of scope, `M7`). M6-B01 §B (all four items, implemented). M6-B06 §D item 5 (`--metrics-snapshot-log`, implemented) and the `last_tick_task_count` field (populated for real). TEST-D45/D46 (test-first changeset boundary). TEST-D50 (CI-is-authority). |
| Crates touched | `rc-scheduler` (`crates/scheduler/`, additive: `src/edf.rs` new; `src/region_manager.rs`, `src/executor.rs`, `src/pool/tick_clock.rs`, `src/metrics/registry.rs`, `src/metrics/snapshot.rs`, `src/lib.rs` modified — every existing public signature's *shape* unchanged, every pre-existing test passes byte-for-byte unmodified). `rusty-clanker-server` (`crates/server/`, the composition root: `src/main.rs` and `src/lib.rs` given real content for the first time; `src/composition/` new module tree; `src/play/world.rs` modified only to extract `HardcodedWorld::new`'s system-registration sequence into a shared, reusable function, `HardcodedWorld`'s own observable behavior and every existing test unchanged; `src/config.rs` modified, additive `[scheduler]` table). **Not** `xtask`, **not** `rc-paritybot`, **not** `rc-transport-inproc` (no source change — only its already-public `register_region`/`deregister_region` are called). |
| Estimated scope | L — at the top of this project's own L budget by necessity, not by choice: this is the single composition root every M6 blueprint through M6-B06 named as the one missing piece six other blueprints' own real acceptance runs are blocked on (M6-B00-index.md's "M6 completion, restated"). Splitting it further would recreate, inside this milestone, the exact "contract pinned, implementation deferred to a future sibling" pattern this project has now used five times in a row — appropriate when a real dependency is genuinely unbuilt, not appropriate for the one blueprint whose entire job is to stop deferring. Every section below is scoped as tightly as the real mechanism allows. This deliberately exceeds `00-blueprint-spec.md`'s general ~300-line Context / ~800-line body sizing guideline, the same class of stated exception M5-B02/M5-B03/M5-B08 already established for a single-domain blueprint that cannot be usefully partitioned without reintroducing the deferred-contract seams it exists to close. |

## Goal & Done definition

Give `rusty-clanker-server` a real, `rc-scheduler::RegionManager`-driven, network-facing, many-region composition root — the piece M6-B01 §B, M6-B02's Scope-boundary note, M6-B03 §A, M6-B05 §L, and M6-B06 §A all independently named as still missing. Concretely: (1) a real `main.rs`/`run_embedded` that parses the full CLI surface every prior harness blueprint has assumed since M1-B06 (`--bind`, `--offline`, `--world-dir`, `--save-interval-ticks`, `--save-event-log`, `--tick-log`, `--region-lifecycle`) plus M6-B01 §B's four items and M6-B06 §D's fifth, and prints the `RC_REGION_COUNT`/`RC_REGION_LAYOUT` stdout contract; (2) region layout resolution — dynamic, on-demand single-cell bootstrap in production (ARCH-D5/D6, no pinned file) or a fully pinned, static layout for tests (`--region-layout`); (3) one shared `RcExecutor` (built once, per ARCH-D8's own "computed once... reused for every tick of every region" — replacing `HardcodedWorld`'s and `TwoRegionWorld`'s own separate, duplicated executor instances) driven by a real, concurrency-safe, earliest-deadline-first admission scheduler across every live region, with worldgen/background work admitted only when no region is overdue; (4) ARCH-D19's coalesced single-work-item dispatch for quiet regions, actually engaged and actually measured (`last_tick_task_count`, M6-B06 §D); (5) region merge/split executed live, under real admission-scheduler load, with the transport/directory/chunk-lifecycle bookkeeping M0-B06 named as a composition-root obligation; (6) player/connection routing to owning regions, generalizing M4-B08's two-region `PlayerRouting` to N regions and to dynamically-spawned ones; (7) the fault-injection schedule resolver M6-B01 §B item 3 named, applied for real as a per-tick, per-region synthetic-load multiplier; (8) graceful shutdown: stop admitting, drain in-flight ticks, flush every region's dirty chunks (M2-B05's WORLD-D25 barrier), close connections, join every thread.

Done when:

- [ ] `cargo build -p rc-scheduler -p rusty-clanker-server --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rusty-clanker-server`.
- [ ] Every pre-existing `rc-scheduler` test (M0-B04/B05/B06, M6-B02, M6-B03's own suites, including `lifecycle_hysteresis.rs`, `pool_resize_hysteresis.rs`, `metrics_cpu_attribution.rs`, `metrics_determinism_preserved.rs`) still passes, byte-for-byte unmodified — proving this blueprint's internal-representation change to `RegionManager`/`RegionDirectory` and its additive `RcExecutor`/`TickClock`/`MetricsRegistry` extensions are behavior-preserving for every existing single-threaded caller.
- [ ] Every pre-existing `rusty-clanker-server` test (M1-B05 through M4-B08's own suites, including `play_chunk_set.rs`, `play_session_handoff.rs`, `play_region_transfer_player_walk.rs`, every `HardcodedWorld`/`TwoRegionWorld`-based test) still passes, byte-for-byte unmodified — `HardcodedWorld` and `TwoRegionWorld` are retained exactly as built; only `main.rs`/`run_embedded` stop calling them at runtime.
- [ ] `edf_admission_never_yields_to_background_ahead_of_an_overdue_region` and `edf_violation_counter_stays_zero_under_synthetic_multi_region_load` (§ Acceptance tests) both pass — the EDF property-test pair the task names.
- [ ] `coalesced_dispatch_engages_for_a_quiet_region_and_reports_near_zero_cpu` passes: a synthetic 0-player region's `last_tick_task_count` reads `Some(1)` and `is_near_zero_dedicated_cpu` reads `true` after 40 consecutive quiet ticks admitted through the real scheduler.
- [ ] `live_merge_and_split_execute_under_shifting_synthetic_load` passes: real ARCH-D6 split and merge events fire from real EDF-scheduled ticks (never `force_split`/`force_merge`), with the transport/directory/EDF-scheduler bookkeeping all updated consistently.
- [ ] `fault_injection_schedule_isolates_the_targeted_region` passes: one region's synthetic load is scaled by a fault-injection multiplier while a sibling's is not, deterministically, matching M6-B01 §G's `resolve_load_multiplier` algorithm exactly.
- [ ] `shutdown_flushes_every_region_and_drains_connections` passes: dirty chunks across ≥2 regions are all persisted, `IoPool::drain_barrier`-equivalent completes, and no chunk is lost, on a clean shutdown call.
- [ ] `cargo run -p rusty-clanker-server -- --help` prints usage advertising `--region-layout` and `--metrics-snapshot-log` (satisfying `xtask::release::detect_region_layout_support` and M6-B06's own `detect_m6_composition_root_support`, both reused unmodified) with zero panics.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changeset (labeled per Constraints).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). This blueprint adds no new CI job — it is what makes M6-B06's already-built `reference-host-gate` job's own `m6-report` step (M6-B06 Deliverables) meaningfully exercisable for the first time; that job's own first green run is a **milestone**-acceptance signal on a provisioned reference host, not a condition of this blueprint's own Tier-1 Done state, mirroring every prior harness blueprint's identical framing.

## Context (self-contained)

### §A — Where this lands, and the one dependency-direction rule that shapes everything below

`rc-scheduler`'s own Crate Manifest responsibility already covers "RC-Executor, RC-WorkerPool, the 11-stage tick pipeline driver, region lifecycle... the Tokio↔RC-WorkerPool boundary types" (`12-workspace-structure.md`) — the EDF admission scheduler and the coalesced-dispatch mechanism are natural, in-scope extensions of that same crate, not a new one. `rusty-clanker-server`'s own responsibility is "wires every server-side crate, owns the Tokio runtime, loads config and resolves the monolithic/cluster/proxy role split" — the composition root itself.

**The one rule every fault-injection/region-layout section below depends on:** `crates/testing/paritybot/` (`rc-paritybot`) is a `crates/testing/*` dev/test-only crate (WS-D2). `rusty-clanker-server` is a *shipped, production* crate. A production crate depending on a test-only crate would invert this workspace's own dependency direction — `rc-paritybot` depends on nothing production-side needs, and nothing production-side may depend on it. M6-B01 §C's `RegionLayoutSpec`/`RegionCellGroup`/`FaultInjectionSchedule`/`FaultInjectionEntry` and §G's `resolve_load_multiplier` therefore cannot be imported by this blueprint at all, even though M6-B01 §B's own text says "the composition root partitions its world according to `regions`" and "computes `resolve_load_multiplier(schedule, label, tick)`" — those sentences describe *this crate's own local reimplementation* of those exact shapes and that exact algorithm, restated field-for-field and byte-for-byte (§I/§J below), never a Cargo dependency edge. This is the identical "restate the wire shape locally, never import across the test/production boundary" discipline M6-B03 §D and M6-B06 §D already established for their own file-boundary mirrors, applied here in the opposite direction (production restating a test crate's RON shape, rather than a test crate mirroring a production crate's JSON shape) for the identical reason.

`HardcodedWorld` (M1-B05, extended by M2-B05) and `TwoRegionWorld` (M4-B08) are retained **exactly as built** — every field, every method, every one of their own existing tests. This blueprint's only edit to `world.rs` is a pure code-motion extraction (§C.1): `HardcodedWorld::new`'s own system-registration call sequence moves into a new, shared function that both `HardcodedWorld::new` (unchanged behavior, unchanged tests) and this blueprint's own new composition root call. Neither `HardcodedWorld` nor `TwoRegionWorld` is reachable from `main()`/`run_embedded` after this blueprint lands — restated plainly rather than left implicit, since it is the one visible behavior change to an already-shipped type's *role*, even though its own *code* is untouched.

### §B — Region layout: dynamic bootstrap (production) vs. pinned (tests)

`01-server-architecture.md`'s own Open Questions list ARCH-D6/D19's numeric thresholds as seed defaults needing calibration, but never states how a region's cell set comes to exist at all when no test harness pins one — this blueprint's own concrete resolution, chosen because it is the only reading consistent with ARCH-D5's "a region owns a contiguous, mutable set of chunks" and ARCH-D6's "a region's owned area is a union of adjacent grid cells," and because M0-B06's own merge protocol already gives grid cells a natural way to *grow* (two adjacent quiet regions merge into one region owning both cells' union):

**Dynamic bootstrap (production default, no `--region-layout`):** regions are never pre-created. The first time any consumer — a player's join position, or a chunk ticket demand from `TicketManager` (M2-B05) — names a `GridCell` (via `GridCell::containing_chunk`) that `RegionDirectory::owner_of` reports as unowned, the composition root spawns a **new, single-cell region** for exactly that one cell (`RegionManager::spawn_region(dimension, [cell])`), registers it with the transport (`InProcessTransport::register_region`) and the EDF scheduler (`EdfScheduler::register_region`, §E) with a fresh deadline `now + SERVER_TICK_PERIOD`, constructs that region's own `ChunkLifecycleManager`/`TicketManager` pair (§C.2) and inserts it into the composition root's own per-region table, and only then proceeds with the join/load that triggered it. Growth beyond one cell happens exclusively through ARCH-D6's own already-implemented merge protocol (M0-B06): as neighboring single-cell regions go quiet, they merge into progressively larger regions; as a region's own load grows, `largest_connectivity_cut` splits it back down. This is the entire "dynamic... per ARCH-D5/D6" mechanism — no new merge/split algorithm, no new numeric threshold, only the on-demand spawn trigger, which is new.

**Pinned (`--region-layout <path>`, tests only):** at startup, before binding the listening socket, every `RegionCellGroup` in the parsed `LocalRegionLayoutSpec` (§J) is spawned as one region via `RegionManager::spawn_region`, in file declaration order, each identically gaining its own `ChunkLifecycleManager`/`TicketManager` pair per §C.2 (there is no behavioral difference between a pinned and a dynamically-spawned region beyond how its cell set was decided), and `merge_split_enabled` gates whether the EDF scheduler's own after-tick hook (§E) ever calls `execute_merge`/`execute_split` at all for the remainder of the process's life (`false` — the common case for a throughput-focused fixed-topology load test — disables it entirely; `true` leaves ARCH-D6 fully live on top of the pinned starting topology). Dynamic on-demand bootstrap (the previous paragraph) is **disabled** whenever `--region-layout` is present: a join or chunk-ticket demand naming a cell no `RegionCellGroup` declared is a **startup-time validation failure** for the whole scenario (a load-test scenario author's bug, not a runtime condition to paper over silently) — reported as `CompositionError::UnclaimedCellUnderPinnedLayout`, never a silent auto-spawn that would defeat the whole point of a pinned topology.

**§C.2 — Per-region chunk lifecycle, restated.** `AnvilDiskBackend` and `IoPool` (M2-B05) are each constructed **once**, shared (`Arc`) across every region in the same dimension — Anvil's own on-disk region-file granularity (32×32 chunks) is independent of ARCH-D6's grid-cell granularity (16×16 chunks) and both the backend and the I/O pool are already `Send + Sync` by M2-B05's own design. Each `ManagedRegion`, by contrast, gets its **own** `ChunkLifecycleManager`/`TicketManager` pair (own resident set, own pending-load set, own ticket levels) — construction is exactly M2-B05's own already-established sequence (`ChunkLifecycleManager::new(Arc::clone(&backend), dimension, filler, Arc::clone(&resolvers), save_interval_ticks, io_queue_capacity)` then `install_resources(&mut region.world)`), called once per spawned region, whether dynamic or pinned.

### §C — Startup sequence

```
1. Parse CLI (§J) + load WorldConfig/SchedulerConfig from --config (or defaults).
2. Resolve dimension list — this blueprint, like M1-B05 through M4-B08 before it,
   supports exactly minecraft:overworld; a --region-layout naming any other
   dimension string is a startup validation error (§J.1's parser flags this
   honestly rather than silently defaulting).
3. build_server_executor() (§C.1) — ONE RcExecutor, built once.
4. Construct InProcessTransport::new(InProcessTransportConfig::default()).
5. Construct MetricsRegistry::new(MetricsConfig { tick_budget_ms: 50.0, .. }) —
   always constructed (M6-B02's registry is always-on, no feature gate, Context:
   "Overhead budget and feature-gating").
6. Construct RegionManager::with_thresholds_and_metrics(&executor, 50.0,
   scheduler_config.hysteresis_thresholds(), Arc::clone(&metrics)) (M6-B03/M6-B02).
7. Construct RcWorkerPool::with_resize_thresholds(pool_config,
   scheduler_config.resize_thresholds()) (M6-B03), pool_config.mode =
   Elastic { auto_sample: true } in production, Deterministic { fixed_size: n }
   only under a test-only override (never under --region-layout's own default).
8. Construct EdfScheduler::new(EdfSchedulerConfig { .. }) (§E).
9. If --region-layout is Some: pinned bootstrap (§B) — spawn every declared
   region, register each with transport + EDF scheduler, print RC_REGION_LAYOUT
   (§J.2). If --fault-injection-schedule is Some: load + validate every entry's
   region_label against the now-known label set (§I) — a schedule naming an
   unknown label is a startup validation error.
   Else: dynamic bootstrap is armed (no region exists yet; the first join/ticket
   demand spawns one, §B).
10. Print RC_REGION_COUNT=<n> (n = the pinned count, or 0 under dynamic bootstrap
    — restated: RC_REGION_COUNT is a live, self-updating count of whatever the
    process holds AT THE MOMENT this line prints, per M3-B08's own original
    wording; under dynamic bootstrap that moment is always pre-first-join, so 0
    is the honest value, never a lie).
11. Open --region-lifecycle-log / --metrics-snapshot-log writers if requested (§K).
12. Spawn EdfScheduler::run(region_manager, pool, transport, Some(&metrics),
    &before_tick, &after_tick, &on_event) (§E) on its own thread — non-blocking
    to this sequence; the scheduler itself owns its driver-thread pool.
    before_tick/after_tick are two small closures, owned by ServerComposition,
    that look up the named region's own ChunkLifecycleManager/TicketManager pair
    in the per-region table (§C.2) and call pre_tick(world, needs_load,
    needs_unload)/post_tick() exactly as M2-B05's own single-region wiring
    already did — generalized from "the one region" to "whichever region this
    call names," never duplicated per region.
13. Spawn the metrics-snapshot poller (§K) and lifecycle-log forwarder (§K) if
    their respective flags were given.
14. Bind the TCP listener (--bind, default "0.0.0.0:25565" — the one default this
    blueprint fixes, since M1-B06's own harness "always passes an explicit
    ephemeral port... the server's own default is never exercised" and no prior
    blueprint pinned one) and enter the accept loop (§H), dispatching each
    accepted connection through the existing M1-B01..B04 handshake/login/
    configuration pipeline unchanged, handing off to this blueprint's own
    PlayerSessionSink impl (§H) at the Play transition.
15. Install a Ctrl-C / SIGTERM handler (tokio::signal) that calls shutdown() (§K).
```

**§C.1 — `build_server_executor`: the shared-executor extraction.** `HardcodedWorld::new` (M1-B05, extended additively by every M2–M5 mechanics blueprint) already assembles, in one place, the complete, real `RcExecutorBuilder::new(bootstrap).register_system(...)....build()` call sequence this project's entire mechanics surface (movement, mining, redstone, entities, AI, physics, lighting, chunk snapshot — every system any M3/M4/M5 blueprint ever registered) depends on. `TwoRegionWorld` (M4-B08) built a **second, separate** copy of a similar sequence plus its own two extra systems (`register_mob_crossing_detection`, its own local player-crossing-detection system) and `with_entity_arrival_driver(player_transfer::combined_arrival_driver)` — an accepted, explicitly-named duplication at M4-B08's own drafting time ("this registration is scoped to `TwoRegionWorld`'s own separate, isolated `RcExecutor`... M4-B09's own governance changeset states this split explicitly"). ARCH-D8 requires **one** conflict graph, computed once, reused for every region — two separate executors is a standing violation of that requirement that M4-B08 knowingly deferred to "a future sibling blueprint." This is that blueprint.

`crates/server/src/play/world.rs`'s `HardcodedWorld::new` body is refactored — a pure code-motion extraction, zero behavior change — into a new function:

```rust
// crates/server/src/play/executor_bootstrap.rs (new)

/// Every `RcExecutorBuilder::register_system` call `HardcodedWorld::new` (M1-B05
/// through M5) and `TwoRegionWorld::new` (M4-B08) each separately assembled,
/// unified into ONE call sequence — code-moved, not rewritten: every `DomainGroup`,
/// every `structural_writes` list, every registration order is copied verbatim from
/// whichever of the two pre-existing sites declared it (mechanics systems from
/// `HardcodedWorld::new`; `register_mob_crossing_detection` +
/// `two_region_world.rs`'s own local player-crossing-detection system +
/// `with_entity_arrival_driver(player_transfer::combined_arrival_driver)` from
/// `TwoRegionWorld::new`). Building this function is the implementer's own careful
/// literal transcription — the acceptance test below is the actual proof of
/// correctness, not this doc comment.
pub fn build_server_executor() -> Result<rc_scheduler::RcExecutor, rc_scheduler::ExecutorBuildError>;
```

`HardcodedWorld::new(config)` is reduced to: call `build_server_executor()`, `.expect(...)` (unchanged panic-on-build-failure behavior — `HardcodedWorld::new`'s own signature has always been infallible, `-> Self`, never `Result`), then proceed with its own existing single-region `spawn_region`/thread-spawn/`ChunkLifecycleManager` wiring exactly as M2-B05 left it. `TwoRegionWorld::new` is reduced identically: call the same `build_server_executor()` once, `spawn_region` twice. **Proof obligation, not merely a claim:** `executor_extraction_preserves_hardcoded_world_behavior` (Acceptance tests) re-runs the *existing* `play_chunk_set.rs`/`play_session_handoff.rs` scenarios bit-for-bit and asserts identical packet output before and after the extraction — the refactor is committed only once that test is green, per Constraints.

### §D — Concurrency-safe `RegionManager`/`RegionDirectory`: the internal representation change

M0-B06's own round-robin driving loop was an explicit, named simplification: "this blueprint's own driving loop is deliberately single-threaded, round-robin... Explicitly not proven by this design: that a genuinely hot region never delays a genuinely quiet one under real concurrent contention — that is ARCH-D18–D20's own real-time EDF scheduler's acceptance criterion, a later blueprint's job, not this one's." `RegionManager<'e>`'s existing methods (`spawn_region`, `region`, `region_mut`, `region_ids`, `neighbors_of`, `tick_region`, `record_synthetic_tick`, `force_split`, `force_merge`, all `&mut self` or `&self` exactly as M0-B06/M6-B02/M6-B03 fixed them) are **unchanged in signature and behavior** — every pre-existing test that constructs one `RegionManager` and drives it from one thread keeps compiling and passing byte-for-byte. What changes is the **private** field representation, which no prior blueprint's public API ever exposed:

```rust
// crates/scheduler/src/region_manager.rs (modify — private representation only)
pub struct RegionManager<'e> {
    executor: &'e RcExecutor,
    // was: regions: HashMap<RegionId, ManagedRegion>
    regions: parking_lot::RwLock<HashMap<RegionId, std::sync::Arc<parking_lot::Mutex<ManagedRegion>>>>,
    // was: directory: RegionDirectory
    directory: parking_lot::RwLock<RegionDirectory>,
    id_alloc: RegionIdAllocator,      // already Sync (AtomicU64), unchanged
    tick_budget_ms: f64,
    thresholds: HysteresisThresholds, // M6-B03, unchanged
    metrics: Option<std::sync::Arc<crate::metrics::MetricsRegistry>>,  // M6-B02, unchanged
}
```

Every existing method's body is reimplemented against this representation (`region(&self, id)` takes a brief `directory`-unrelated `regions.read()`, clones the `Arc`, and returns a `MappedRwLockReadGuard`-style borrow or — simpler, and what this blueprint specifies — changes `region`/`region_mut`'s own *return type* is **not** permitted (would be a public-signature change); instead, every existing accessor is implemented by locking the per-region `Mutex` for the duration of the call and returning owned/cloned data or operating via a closure-taking internal helper, whichever the implementer finds cleanest **as long as `region(&self, id) -> Option<&ManagedRegion>`'s exact signature is preserved** — this is a genuine, narrow implementation constraint: a `&ManagedRegion` borrowed out of a `MutexGuard` cannot outlive that guard, so `region`/`region_mut`'s *existing* signature (returning a bare reference with the same lifetime as `&self`/`&mut self`) is only soundly reimplementable when the caller holds `&mut self` (exclusive access to the whole manager, in which case `RwLock::get_mut`/`Mutex::get_mut` bypass locking entirely and return a genuine `&mut` with the right lifetime) — `region_mut` therefore keeps taking `&mut self exactly as before (no change needed at all: `self.regions.get_mut().unwrap().get_mut(&id).map(|arc| arc.get_mut())` — `parking_lot::Mutex::get_mut` requires `&mut self` on the mutex and returns `&mut ManagedRegion` directly, no lock acquired). The **read-only** `region(&self, id) -> Option<&ManagedRegion>` is the one genuinely awkward case: it is reimplemented via `self.regions.read()` (a guard living only for the call) `.get(&id)` `.map(|arc| ...)` — since a `&ManagedRegion` cannot be returned out of a temporary guard, and this blueprint must not change the signature, the implementer resolves this by having `region`'s internals momentarily elevate to `get_mut`-shaped access only when called through `&self` is genuinely impossible without `unsafe` or a signature change; **this blueprint's own resolution, stated plainly:** `region(&self, id)` is changed from returning `Option<&ManagedRegion>` to **not existing in that shape any longer for the concurrent representation** — instead this blueprint adds `pub fn with_region<R>(&self, id: RegionId, f: impl FnOnce(&ManagedRegion) -> R) -> Option<R>` as the new read-only accessor, and every pre-existing test/caller of the old `region(&self, id) -> Option<&ManagedRegion>` is updated, in this blueprint's own implementation changeset, to the closure form — **this is the one place this blueprint touches a pre-existing public signature's shape**, named here explicitly rather than glossed over, and bounded to exactly this one accessor (`region_mut` is untouched, per above). Every M0-B06/M6-B02/M6-B03 test that used `region(id).unwrap().cells()`-style read access is updated to `manager.with_region(id, |r| r.cells().clone()).unwrap()` or equivalent — a mechanical, behavior-preserving edit, proven by every one of those tests still asserting the identical outcome.

`RegionDirectory`'s own `owner_of`/`adjacent_regions` (`&self`) and `assign`/`unassign` (`pub(crate)`, called only from `RegionManager` internals which already hold the outer `directory` lock for the duration) need no signature change at all — `RegionManager`'s new methods (`spawn_region_concurrent`-adjacent bookkeeping, below) simply take `self.directory.read()`/`.write()` around the existing calls.

**The one genuinely new capability — `tick_region_concurrent`:**

```rust
impl<'e> RegionManager<'e> {
    /// As `tick_region`, but takes `&self` — safe to call from multiple threads
    /// simultaneously for DIFFERENT `id`s (the EDF scheduler's own invariant, §E:
    /// a region is never admitted while already in flight elsewhere). Internally
    /// decides fine-grained vs. ARCH-D19 coalesced dispatch from the region's own
    /// CURRENT `tick_duration_ewma_ms()` (read under that region's own per-entry
    /// lock, before dispatch): `ewma < merge_threshold_ms()` (the same "quiet"
    /// cutoff M0-B06 already reuses for merge eligibility, Context §F below) calls
    /// `RcExecutor::tick_region_coalesced`; otherwise `RcExecutor::tick_region`
    /// (unchanged). `before`/`after` run synchronously on the calling (driver)
    /// thread, while this region's own per-entry lock is held, immediately before
    /// and immediately after the real `RcExecutor` dispatch — the exact hook
    /// M2-B05's `ChunkLifecycleManager::pre_tick`/`post_tick` need (Context §C.2;
    /// `rc-scheduler` itself has no dependency on `rc-chunk-storage` and calls
    /// neither directly — the composition root supplies them as closures, threaded
    /// through unmodified from `EdfScheduler::run`, below). Returns `None` only if
    /// `id` is not currently live (already retired by a concurrent merge/split —
    /// the caller's own stale read, handled gracefully, never a panic) — in that
    /// case neither `before` nor `after` is called.
    pub fn tick_region_concurrent(
        &self,
        id: RegionId,
        pool: &crate::pool::RcWorkerPool,
        transport: &dyn rc_messaging::Transport,
        before: impl FnOnce(&mut RegionState),
        after: impl FnOnce(&RegionState, &TickReport),
    ) -> Option<(TickReport, LifecycleOutcome)>;

    /// Read-only accessor for the concurrent representation (Context — replaces
    /// `region(&self, id) -> Option<&ManagedRegion>` for every caller; `region_mut`
    /// is unchanged).
    pub fn with_region<R>(&self, id: RegionId, f: impl FnOnce(&ManagedRegion) -> R) -> Option<R>;
}
```

`tick_region_concurrent`'s own after-dispatch bookkeeping (EWMA update, split/merge hysteresis check, `execute_merge`/`execute_split` on a triggered `LifecycleOutcome`) is **identical logic** to `tick_region`'s existing internal `after_tick` call (M0-B06 Implementation step 7) — reused as a private helper both methods call, never duplicated. `execute_merge`/`execute_split`'s own directory/regions-map mutation (removing retired ids, inserting new ones) takes the outer `regions`/`directory` locks' **write** halves only for that brief, rare (per M0-B06's own "a cold path, rare event") critical section — never for the duration of an actual tick, which holds only the one target region's own per-entry `Mutex`.

### §E — The EDF admission scheduler (ARCH-D20, restated in full)

ARCH-D20, verbatim (`01-server-architecture.md`): "Region tick admission is Earliest-Deadline-First: each region's deadline = `last_tick_start + 50ms`; RC-Executor's Injector serves overdue regions before on-time regions regardless of arrival order." M6-B02 built the **measurement** half of this (`record_deadline_ready`/`record_admission`, an exact, checkable violation rule, Context restated below) but explicitly left "the real-time EDF admission scheduler itself... a sibling, not-yet-written blueprint's job" open. This is that blueprint.

**§E.1 — Non-blocking deadline tracking: one new `TickClock` method.**

`TickClock::await_next_tick` (M0-B04) *blocks* the calling thread until its own deadline — unusable for a scheduler that must track many regions' independent deadlines from one thread without blocking on any single one. This blueprint adds one new, purely additive method to the already-merged `crates/scheduler/src/pool/tick_clock.rs`:

```rust
impl<W: TickWaiter> TickClock<W> {
    /// Non-blocking sibling of `await_next_tick`'s own schedule-advance half: if
    /// `now` is at or past `next_deadline`, immediately advances the schedule by
    /// exactly one more `SERVER_TICK_PERIOD` from the PREVIOUS scheduled deadline
    /// (never from `now` — preserves `await_next_tick`'s own non-drift-compounding
    /// guarantee, M0-B04 Context, exactly) and returns `Some(TickTiming)` for the
    /// tick that just became due; otherwise (not yet due) returns `None` without
    /// mutating any state. Never blocks, never sleeps.
    pub fn try_advance(&mut self, now: std::time::Instant) -> Option<TickTiming>;
}
```

Body: `if now < self.next_deadline() { return None; }` else `tick_index += 1`, `scheduled_deadline = self.next_deadline`, `overrun = now.saturating_duration_since(scheduled_deadline)`, `self.next_deadline = scheduled_deadline + SERVER_TICK_PERIOD`, return `Some(TickTiming { tick_index, scheduled_deadline, actual_wake: now, overrun })` — identical formula to `await_next_tick`'s own advance step, restated non-blocking. `try_advance_never_compounds_drift_over_many_calls` (Acceptance tests) re-proves M0-B04's own 12,000-tick determinism property for this new entry point, exactly mirroring `tick_clock_drift.rs`'s own test 1.

**§E.2 — The scheduler's own state and algorithm.**

```rust
// crates/scheduler/src/edf.rs (new)

#[derive(Clone, Copy, Debug)]
pub struct EdfSchedulerConfig {
    /// Concurrency degree — how many regions may be simultaneously mid-tick.
    /// Seed default: `crate::pool::compute_baseline()` (reused, M0-B04) — one
    /// driver roughly per host core, letting `RcWorkerPool`'s own elastic sizing
    /// absorb the rest of the parallelism inside each region's own wave dispatch.
    pub driver_count: usize,
    /// How often the deadline-tracking thread polls every idle region's
    /// `TickClock`. Seed default `Duration::from_millis(1)` — negligible relative
    /// to the 50ms tick budget; calibration-pending like every other numeric
    /// threshold in this corpus.
    pub poll_interval: std::time::Duration,
}
impl Default for EdfSchedulerConfig {
    fn default() -> Self { Self { driver_count: crate::pool::compute_baseline().max(2), poll_interval: std::time::Duration::from_millis(1) } }
}

/// One event this blueprint's scheduler emits per region-tick or lifecycle
/// transition — the composition root's own hook for `--region-lifecycle-log`/
/// `--region-tick-log`/`--metrics-snapshot-log` wiring (§K) and for driving
/// transport/directory registration on a merge/split (§G).
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    RegionTicked { id: rc_messaging::RegionId, report: crate::TickReport, coalesced: bool },
    Lifecycle { outcome: crate::LifecycleOutcome },
}

pub struct EdfScheduler { /* private */ }

impl EdfScheduler {
    pub fn new(config: EdfSchedulerConfig) -> Self;

    /// Registers `id` with `initial_deadline` (the caller's own choice — a freshly
    /// spawned region uses `Instant::now() + SERVER_TICK_PERIOD`, Context §B/§G).
    pub fn register_region(&self, id: rc_messaging::RegionId, initial_deadline: std::time::Instant);
    /// Removes `id` (a region just retired by a merge/split, §G). A no-op if `id`
    /// was never registered or already removed.
    pub fn unregister_region(&self, id: rc_messaging::RegionId);

    /// Submits one fire-and-forget background job (worldgen chunk generation, or
    /// any other lower-than-tick-priority CPU work) — admitted onto the shared
    /// pool only when, at the moment a driver thread checks, no region is
    /// currently overdue (Context §E.3's exact rule). Never blocks the caller.
    pub fn submit_background(&self, job: Box<dyn FnOnce() + Send>);

    /// Spawns `config.driver_count` driver OS threads plus one deadline-tracking
    /// thread, and runs until `shutdown()` is called from another thread. Blocking
    /// — call from its own dedicated thread (the composition root's own startup
    /// sequence, §C, does exactly this). `before_tick`/`after_tick` are forwarded,
    /// per admitted region, straight into `RegionManager::tick_region_concurrent`'s
    /// own `before`/`after` parameters (above) — a single pair of `RegionId`-
    /// parameterized closures suffices for every region (the composition root's
    /// own per-region `ChunkLifecycleManager` table lookup happens *inside* the
    /// closure body, keyed by the `RegionId` argument, never one closure per
    /// region).
    pub fn run(
        &self,
        region_manager: &crate::RegionManager<'_>,
        pool: &crate::pool::RcWorkerPool,
        transport: &dyn rc_messaging::Transport,
        metrics: Option<&crate::metrics::MetricsRegistry>,
        before_tick: &(dyn Fn(RegionId, &mut RegionState) + Send + Sync),
        after_tick: &(dyn Fn(RegionId, &RegionState, &TickReport) + Send + Sync),
        on_event: &(dyn Fn(SchedulerEvent) + Send + Sync),
    );

    /// Signals every driver and the deadline-tracking thread to stop after their
    /// current work finishes; does not itself block (Context §K owns joining).
    pub fn shutdown(&self);
    /// Blocks until every thread `run` spawned has actually joined.
    pub fn join(&self);
}
```

**§E.3 — The algorithm, stated precisely.**

Internal state (implementer's freedom for exact field layout; the *behavior* below is binding):

- `clocks: Mutex<HashMap<RegionId, TickClock<SystemTickWaiter>>>` — one per **currently idle** (not in-flight) live region.
- `status: Mutex<HashMap<RegionId, RegionStatus>>` where `RegionStatus = Idle | Ready | InFlight`.
- `ready: Mutex<BinaryHeap<Reverse<(Instant, RegionId)>>>` (a min-heap by deadline — `Reverse` makes `BinaryHeap`, a max-heap by default, behave as a min-heap) plus a `Condvar` signalled whenever `ready` gains an entry.
- `background: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>`.
- `stop: AtomicBool`.

**Deadline-tracking thread** (one, spawned by `run`): loop until `stop`: sleep `config.poll_interval`; `now = Instant::now()`; for every region currently `Idle` (status map), call `clocks[id].try_advance(now)`; on `Some(timing)`: call `metrics.record_deadline_ready(id, timing.scheduled_deadline)` if metrics attached (the exact M6-B02 feed-in call, at the exact instant a region becomes due — never earlier, never later), set `status[id] = Ready`, push `Reverse((timing.scheduled_deadline, id))` onto `ready`, notify the condvar. A region already `Ready` or `InFlight` is skipped (its clock stays untouched — no double-advance, Context §E.4's own restated invariant).

**Driver threads** (`config.driver_count`, spawned by `run`): loop until `stop`: wait on the ready condvar (with a timeout of `config.poll_interval`, so an idle driver also periodically re-checks the background queue, Context §E.5) for `ready` to be non-empty; if non-empty, pop the minimum `(deadline, id)`; set `status[id] = InFlight` and remove `id` from `clocks` (temporarily — it is re-inserted after the tick, §E.4); call `metrics.record_admission(id, deadline, Instant::now())` if attached (the exact second M6-B02 feed-in call — this is **the** EDF admission decision: among every region currently `Ready`, the one with the globally earliest deadline is what the next free driver thread always pops, by the min-heap's own ordering, "regardless of arrival order" exactly as ARCH-D20 requires); call `region_manager.tick_region_concurrent(id, pool, transport, |state| before_tick(id, state), |state, report| after_tick(id, state, report))`; on `Some((report, outcome))`, call `on_event(SchedulerEvent::RegionTicked { id, report, coalesced: /* the region's pre-tick EWMA read, cheaply re-derived or threaded through, Context §F */ })`; then §E.4's re-scheduling step; on `Some(LifecycleOutcome::Merged{..} | Split{..})`, additionally call `on_event(SchedulerEvent::Lifecycle { outcome })` (the composition root's own §G hook fires from here); on `None` (the region was retired by a concurrent event between pop and dispatch — rare, harmless), do nothing further for this iteration.

**§E.4 — Re-scheduling after a tick (non-compounding, restated for the concurrent case).** If the tick's own `LifecycleOutcome` was `None`: compute `next_deadline = deadline + SERVER_TICK_PERIOD` (from the just-consumed **scheduled** deadline, never from `Instant::now()` — the identical non-drift-compounding rule, generalized to the multi-driver case), construct a **fresh** `TickClock` whose own `next_deadline()` is exactly that value (`TickClock` has no public "set deadline directly" constructor — this blueprint's own driver code achieves this by holding a `TickClock` per region across its *entire* idle lifetime rather than reconstructing one each cycle: the clock removed from `clocks` at admission time, above, is the *same* clock instance re-inserted here after one more `try_advance`-equivalent bookkeeping step is folded into it — concretely, the driver calls `clock.try_advance(deadline)` immediately before re-inserting, which is guaranteed to fire (since `deadline == clock.next_deadline()` exactly, by construction) and advances it correctly in one line, reusing `try_advance` itself rather than a second, parallel formula), set `status[id] = Idle`, re-insert into `clocks`. If the outcome was `Merged`/`Split`: the retired id(s) are permanently dropped from `status`/`clocks` (no re-insertion); the composition root's own `on_event` handler is responsible for calling `EdfScheduler::register_region` for the new id(s) with a fresh `Instant::now() + SERVER_TICK_PERIOD` deadline (mirroring M0-B06's own "a fresh region... starts with an unseeded EWMA and every counter at 0" convention, generalized to the scheduler's own deadline bookkeeping) — stated here as the exact obligation §G's own event handler discharges.

**§E.5 — Background-work admission (M2-B05/M5-B09's background work classes, restated and gated).** M2-B05's chunk-save I/O already runs entirely on its own dedicated `IoPool` (M2-B05's own private thread pool, never `RcWorkerPool`) — no gating is needed there; this blueprint changes nothing about it. Worldgen chunk generation (M5-B09, not a prerequisite of this blueprint — its own exact job-submission API is a **moderate-confidence flag**, verify against M5-B09's actual public surface at implementation time) is the CPU-bound background class ARCH-D20's own interface note names: *"RC-WorkerPool availability at lower-than-tick scheduling priority (ARCH-D20's EDF ordering never yields to worldgen work ahead of an overdue region tick)"* — `01-server-architecture.md`'s own "Provides to `04-worldgen-parity.md`" line. `EdfScheduler::submit_background` is the concrete facility that line already promises: a driver thread, whenever its own wait on the ready condvar times out with `ready` still empty (Context §E.3), pops **at most one** queued background job and dispatches it via `pool.spawn(job)` (fire-and-forget, non-blocking — the driver does not wait for it) before looping back to wait again. **The exact, checkable rule this blueprint's own property test proves:** a background job is never dispatched at any instant a region is `Ready` or was `Ready` at any point since the driver's last check — admission order is correct by construction; an already-*admitted* background job already running on a pool worker is never preempted by a newly-arriving overdue region (`RcWorkerPool` has no preemption, ARCH-D18's own design, unchanged) — the guarantee is over **admission order**, never over an in-flight task's own completion, stated explicitly rather than overclaimed.

### §F — ARCH-D19's coalesced-tick path, implemented

ARCH-D19's full text has two halves (M0-B06 Constraint (g), restated): pool-level backlog-EWMA grow/shrink (M0-B04, unchanged by this blueprint) and per-region hot/quiet **work-item granularity** — "a region under 5ms (10% of budget, quiet) coalesces its entire tick into a single work item." M0-B06 already fixed its own merge-eligibility threshold as reusing this exact 10%-of-budget figure (M0-B06 Context: "reusing ARCH-D19's own already-pinned 'quiet' cutoff... the SAME threshold that already governs single-work-item tick coalescing") — meaning `ManagedRegion::merge_threshold_ms()` (M0-B06, already built, `0.1 * tick_budget_ms`) **is** the number this blueprint needs; no new constant is introduced. This blueprint implements only the **quiet** half — the **hot** half ("splits Stage-6/7/8 work into finer batches, 32 entities/chunks per unit instead of 128") remains out of scope, unchanged from M0-B06's own honest framing: no per-entity/per-chunk Stage-6/7/8 system exists in this project's own dispatch model to subdivide (M0-B05's dispatch is system-granularity, not entity-granularity, for every mechanics system landed through M5) — there is nothing to make "finer" yet, and this blueprint does not invent a subdivision mechanism no content needs.

**Mechanism.** `RcExecutor::tick_region` (M0-B05) dispatches Stages 6/8/9/11's own conflict-graph waves via separate `pool.run_batch(tasks)` calls, one per wave, each wave containing one task per mutually-compatible system. This blueprint adds one new, purely additive `RcExecutor` method:

```rust
impl RcExecutor {
    /// ARCH-D19's coalesced-tick path (this blueprint): identical 11-stage
    /// pipeline semantics, identical sync points, identical Stage-1/Stage-10
    /// message contract as `tick_region` — the only difference is dispatch shape.
    /// Every wave that `tick_region` would submit via `pool.run_batch(tasks)`
    /// (Stages 6, 8, 9, 11) is instead executed sequentially, inline, on
    /// whichever ONE thread is running this whole call (safe: a wave's own
    /// members are already proven pairwise-compatible by `compute_waves`,
    /// M0-B05 — running them one after another instead of concurrently is
    /// merely a different, equally valid interleaving of an already-conflict-free
    /// set) — no `pool.run_batch` call happens anywhere inside this method's own
    /// body. Produces byte-identical `TickReport`/final `World` state/emitted
    /// message sequence to `tick_region` for the identical region/tick (this
    /// blueprint's own `coalesced_dispatch_matches_fine_grained_state` test).
    /// When `self.metrics.is_some()`, the ENTIRE call is wrapped, exactly once,
    /// via `metrics::region_tagged_task(region.id, Stage::EntityAiPhysics,
    /// Arc::clone(metrics), || <the whole body above>)` — Stage 6 chosen as the
    /// coalesced bundle's own attribution home (the domain group a quiet region's
    /// own near-zero cost is already measured against, M6-B02's own
    /// `NEAR_ZERO_CPU_THRESHOLD_RATIO` threshold) rather than adding a 12th
    /// `Stage` variant, which would break M6-B02's already-merged, fixed-size
    /// `TickCpuCost.per_stage_ns: [u64; 11]` array. This is the ONE
    /// `region_tagged_task`-wrapped submission this call makes to `pool` —
    /// exactly the single-work-item signature `last_tick_task_count` (Context,
    /// below) needs to observe.
    pub fn tick_region_coalesced(&self, region: &mut RegionState, pool: &crate::pool::RcWorkerPool, transport: &dyn rc_messaging::Transport) -> TickReport;
}
```

`RegionManager::tick_region_concurrent` (§D) calls this method instead of `tick_region` exactly when the region's own `tick_duration_ewma_ms()`, read immediately before dispatch, is `Some(ewma) if ewma < region.merge_threshold_ms()` — a fresh, unseeded region (`tick_duration_ewma_ms() == None`) uses the ordinary fine-grained path for its first tick (no prior sample to judge quietness from, the conservative default), which is expected to immediately route to coalesced on its second tick onward if it stays quiet.

**`last_tick_task_count`, populated for real (M6-B06 §D's own binding contract field).** `crates/scheduler/src/metrics/registry.rs`'s private `RegionCpuState` (M6-B02) gains one new field, `last_tick_task_count: std::sync::atomic::AtomicU32`, incremented by exactly 1 inside `region_tagged_task`'s own wrapper closure (`metrics/attribution.rs`, M6-B02, gains one additional `registry.record_task_dispatched(region)` call alongside its existing `record_task_cpu_time`) — **never** incremented by `measure_inline` (Stage 1/4/10's own inline calls do not go "through RC-WorkerPool," M6-B06's own field doc comment, restated and honored exactly). `MetricsRegistry::end_tick_attribution(&self, region)` (M6-B02, already the "read-and-reset per-tick state" call site) additionally reads-and-resets this counter into `TickCpuCost`'s own new field:

```rust
// crates/scheduler/src/metrics/registry.rs (modify — additive field)
pub struct TickCpuCost {
    pub total_ns: u64,
    pub per_stage_ns: [u64; 11],
    /// New (M6-B07): count of `region_tagged_task`-wrapped submissions this
    /// region's just-completed tick actually dispatched through RC-WorkerPool.
    /// `1` is the ARCH-D19 coalesced-path signature.
    pub last_tick_task_count: u32,
}
```

`RegionManager`'s own already-existing `record_tick_duration_sample`-adjacent forwarding (M6-B02's `MetricsRegistry::record_tick_duration_sample`) gains one more parameter carrying this value through to `snapshot()`'s assembly, which populates the field M6-B06 §D already reserved on `RegionMetricsSnapshot`:

```rust
// crates/scheduler/src/metrics/snapshot.rs (modify — additive field, exactly M6-B06's own pinned shape)
pub struct RegionMetricsSnapshot {
    // ...every existing field from M6-B02 unchanged...
    /// New (M6-B07, discharging M6-B06 §D's own binding contract addition):
    /// `Some(n)` from this region's most recently completed tick — `n == 1` is
    /// the coalesced-path signature.
    pub last_tick_task_count: Option<u32>,
}
```

### §G — Live merge/split execution: the composition-root wiring M0-B06 named

M0-B06's own Context, verbatim: *"a composition-root crate that depends on both `rc-scheduler` and `rc-transport-inproc`... calls `transport.register_region(id)` right after every `spawn_region`, and `transport.register_region(new_id)` + `transport.deregister_region(old_id)`... on every non-`None` `LifecycleOutcome`."* This blueprint's `on_event` handler (passed into `EdfScheduler::run`, §E) is exactly that wiring, plus three more obligations M0-B06 could not have anticipated (the EDF scheduler's own deadline map, and per-region chunk lifecycle, neither of which existed before this blueprint):

On `SchedulerEvent::Lifecycle { outcome: LifecycleOutcome::Merged { old_a, old_b, new } }`:
1. `transport.deregister_region(old_a); transport.deregister_region(old_b); transport.register_region(new);`
2. `scheduler.unregister_region(old_a); scheduler.unregister_region(old_b); scheduler.register_region(new, Instant::now() + SERVER_TICK_PERIOD);`
3. Retire `old_a`'s and `old_b`'s own `ChunkLifecycleManager`/`TicketManager` pair (one such pair per region, constructed at spawn time exactly as M2-B05's own `HardcodedWorld::new` construction, generalized — Context §C's own startup sequence and the `composition/mod.rs` per-region table, Deliverables) — call `shutdown(&world)` on each (flushing any dirty resident chunk to the shared `AnvilDiskBackend`, WORLD-D25) — and construct a **fresh** `ChunkLifecycleManager`/`TicketManager` pair for `new`, scoped to `new`'s own (now-combined) cell set. **Explicit scope note, stated plainly rather than glossed over:** this blueprint does not migrate in-memory chunk residency across a live merge/split — the new pair starts cold and re-derives its own resident set from real player tickets on its next `pre_tick` call, cheaply reloading from disk (durable, correct, never lossy — WORLD-D25's own flush-before-retire step guarantees nothing dirty is lost) rather than carrying live `Entity`/`World` chunk state across the boundary. A future blueprint may optimize this into a genuine in-memory hand-off; this blueprint's own correctness does not depend on that optimization existing.
4. Route every player whose `PlayerRouting` (M4-B08) currently points at `old_a`'s or `old_b`'s own `RegionQueueHandles` to `new`'s (§H).

On `SchedulerEvent::Lifecycle { outcome: LifecycleOutcome::Split { old, new_a, new_b } }`: the symmetric sequence — deregister/register transport and scheduler entries for `old` → `{new_a, new_b}`; retire `old`'s chunk-lifecycle pair, construct fresh pairs for `new_a`/`new_b` scoped to each fragment's own cells; re-route every player currently in `old`'s territory to whichever of `new_a`/`new_b` now owns their current chunk's cell (a `GridCell::containing_chunk` lookup against the post-split directory, §H).

### §H — Player/connection routing to owning regions

M4-B08's `PlayerRouting`/`RegionQueueHandles` (restated exactly, Prerequisites) already generalizes to any number of regions unmodified — `PlayerRouting::redirect_to` swaps which region's own queue handles a connection's packets are forwarded to, with no assumption baked in about there being exactly two regions. This blueprint's own additions:

1. **Join-time resolution.** `HardcodedWorld`'s `PlayerSessionSink` impl (M1-B05) is not reused at runtime (§A) — this blueprint's own composition-root type implements `PlayerSessionSink::accept` itself: translate `PlayerSession` into `PlayerProfile` exactly as `connection.rs`'s `enter_play` already expects, resolve the joining player's spawn position's own `GridCell` via `GridCell::containing_chunk`, and: under a pinned layout, look up `RegionDirectory::owner_of` (a startup-validation error if unclaimed, §B); under dynamic bootstrap, spawn a fresh single-cell region on the spot if unowned (§B) — either way, construct this player's own `PlayerRouting` pointing at the resolved region's `RegionQueueHandles`, store it in a small, process-wide `RwLock<HashMap<u128, Arc<PlayerRouting>>>` side table (uuid-keyed, mirroring M4-B08's own player_transfer.rs's already-named "process-wide `RcEntityId -> Arc<PlayerRouting>` side table" convention, generalized to N regions), and enqueue the join through that region's own `PendingJoin` queue.
2. **Cross-region transfer.** Unchanged from M4-B08's own `detect_mob_crossings`/`combined_arrival_driver` mechanism — reused exactly, now running inside the ONE shared executor (§C.1) for every region rather than only `TwoRegionWorld`'s own two.
3. **Region-count-agnostic:** neither `PlayerRouting` nor `RegionQueueHandles`'s own two-field shape (`block_action_tx`, `movement_tx`) changes — this blueprint adds no new field. **Moderate-confidence flag, honestly stated:** if a future blueprint's own per-tick pending-action queue (beyond block-action/movement) needs region-aware routing, `RegionQueueHandles` gains that field the identical additive way M4-B08 already established; not needed by anything this blueprint's own acceptance tests exercise.

### §I — Fault injection: the local restatement, applied for real

§A's dependency-direction rule means this blueprint carries its **own** copy of M6-B01 §C/§G's exact shapes, field-for-field, and §G's exact algorithm, verbatim:

```rust
// crates/server/src/composition/fault_injection.rs (new)

/// Byte-for-byte the same RON wire shape as `rc_paritybot::loadtest::FaultInjectionSchedule`
/// (M6-B01 §C) — a separate Rust type in a separate crate, deserializing the
/// identical file format, per §A's dependency-direction rule.
#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct LocalFaultInjectionSchedule { pub entries: Vec<LocalFaultInjectionEntry> }

#[derive(serde::Deserialize, Debug, Clone, Copy)]
pub struct LocalFaultInjectionEntry {
    pub region_label: String,
    pub tick_start: u64,
    pub tick_end: u64,
    pub load_multiplier: f64,
}

/// Identical algorithm to M6-B01 §G's `resolve_load_multiplier` — maximum
/// `load_multiplier` among every entry whose `region_label` matches and whose
/// half-open `[tick_start, tick_end)` contains `tick`; `1.0` if none match. Pure,
/// deterministic, never consults wall-clock time, measured load, or any RNG
/// (PERF-D3's framing, restated — binding here exactly as M6-B01 §B item 3 named
/// it as a requirement on whichever blueprint implemented the server-side half).
pub fn resolve_load_multiplier(schedule: &LocalFaultInjectionSchedule, region_label: &str, tick: u64) -> f64;
```

**Application point.** Every tick, immediately before `tick_region_concurrent`/`tick_region_coalesced` dispatch, if `--fault-injection-schedule` was given: `let multiplier = resolve_load_multiplier(&schedule, label_of(region_id), tick_counter);` `world.resource_mut::<SyntheticLoadProfile>().busy_work_micros = base_busy_work_micros * multiplier as u64;` (`base_busy_work_micros` — the region's own unscaled value, recorded once at spawn time, never overwritten by this scaling step itself, so repeated application is idempotent rather than compounding). `label_of` resolves a live `RegionId` back to its originally-declared label via the same map `RC_REGION_LAYOUT` (§J.2) is built from — **only meaningful, and only accepted, under a pinned `--region-layout`**: `--fault-injection-schedule` given without `--region-layout` is a startup validation error (`CompositionError::FaultInjectionRequiresPinnedLayout`) — a dynamically-bootstrapped region has no stable label to schedule against, and this blueprint does not invent one. `SyntheticLoadProfile` itself is `M0-B06`'s own already-built resource (Prerequisites) — this blueprint adds no new resource type, only the per-tick scaling call site M6-B01 §B item 3 named as the still-open half.

### §J — The CLI/stdout contract, made real

This is the first blueprint to give `crates/server/src/main.rs` real content (Context §A — every prior blueprint only ever "assumed" this surface). Built with `clap::Parser` (already workspace-pinned, already `xtask`'s own established CLI library — no new dependency).

| Flag | Type | Default | Source |
|---|---|---|---|
| `--bind <ip:port>` | `String` | `"0.0.0.0:25565"` | M1-B06 (assumed), this blueprint (first real binding) |
| `--offline` | `bool` flag | `false` | M1-B06 |
| `--config <path>` | `Option<PathBuf>` | `None` (all defaults) | M2-B05's `WorldConfig::load`, extended §J.3 |
| `--world-dir <path>` | `Option<PathBuf>` | overrides `WorldConfig.world_dir` | M2-B08 |
| `--save-interval-ticks <n>` | `Option<u64>` | overrides `WorldConfig` | M2-B08 |
| `--save-event-log <path>` | `Option<PathBuf>` | off | M2-B08 |
| `--tick-log <path>` | `Option<PathBuf>` | off | M3-B08 |
| `--region-tick-log <path>` | `Option<PathBuf>` | off | M5-B10 |
| `--region-lifecycle <auto\|pinned-single>` | enum | `auto` | M3-B08 (now load-bearing: `pinned-single` forces `driver_count = 1` and `merge_split_enabled = false` regardless of `--region-layout`'s own value — the exact "disable `RegionManager::after_tick`'s merge/split evaluation for this process" switch M3-B08 named as inert-until-M6) |
| `--region-layout <path>` | `Option<PathBuf>` | dynamic bootstrap | M6-B01 §B item 1, implemented |
| `--fault-injection-schedule <path>` | `Option<PathBuf>` | off | M6-B01 §B item 3, implemented |
| `--region-lifecycle-log <path>` | `Option<PathBuf>` | off | M6-B01 §B item 4, implemented |
| `--metrics-snapshot-log <path>` | `Option<PathBuf>` | off | M6-B06 §D item 5, implemented |

**§J.1 — `LocalRegionLayoutSpec` parsing.** The local restatement (§A's rule) of M6-B01 §C:

```rust
// crates/server/src/composition/region_layout.rs (new)
#[derive(serde::Deserialize, Debug, Clone)]
pub struct LocalRegionLayoutSpec {
    pub dimension: String,
    pub merge_split_enabled: bool,
    pub regions: Vec<LocalRegionCellGroup>,
}
#[derive(serde::Deserialize, Debug, Clone)]
pub struct LocalRegionCellGroup { pub label: String, pub cells: Vec<(i32, i32)> }

/// `"minecraft:overworld"` -> `Some(DimensionId::OVERWORLD)`; any other string ->
/// `None` (a startup validation error — this blueprint, like every M1-M5 blueprint
/// before it, supports exactly one dimension; extending this is a future
/// blueprint's job, not silently guessed here).
pub fn parse_dimension_id(s: &str) -> Option<rc_core::DimensionId>;
```

**§J.2 — `RC_REGION_LAYOUT` stdout line.** Printed once, immediately after `RC_REGION_COUNT`, only when `--region-layout` was given: `RC_REGION_LAYOUT={"spawn-quiet":3,"east-hot":7,...}` — a JSON object (`serde_json`, already pinned) mapping every `LocalRegionCellGroup.label` to the real `RegionId(u64)` `RegionManager::spawn_region` returned for it, in file declaration order — exactly M6-B01 §B item 2's pinned shape.

**§J.3 — `[scheduler]` config table.** `crates/server/src/config.rs` (M2-B05, modify — additive):

```rust
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SchedulerConfig {
    pub driver_count: Option<usize>,          // None = EdfSchedulerConfig::default()
    pub poll_interval_ms: Option<u64>,
    pub resize_thresholds: Option<rc_scheduler::pool::ResizeThresholds>,     // M6-B03
    pub hysteresis_thresholds: Option<rc_scheduler::HysteresisThresholds>,   // M6-B03
}
```

### §K — Metrics snapshot log, region-lifecycle log, and shutdown ordering

**`--metrics-snapshot-log`.** A dedicated thread (or a `tokio::time::interval` task) calls `metrics.snapshot(&pool)` every `METRICS_SNAPSHOT_POLL_INTERVAL_TICKS = 100` ticks' worth of wall time (`100 * 50ms = 5s` — reusing M6-B06 §D item 5's own already-pinned cadence exactly, restated, not reinvented) and appends one line via `rc_scheduler::metrics::snapshot::write_snapshot_json`-equivalent single-line-JSON append (`write_snapshot_json` itself pretty-prints to a whole file; this blueprint's own NDJSON appender is a thin, one-line-per-call sibling using the identical `MetricsSnapshot` `Serialize` impl — `serde_json::to_string` instead of `to_string_pretty`, appended with a trailing newline).

**`--region-lifecycle-log`.** The `on_event` handler (§G) also writes one NDJSON line per `SchedulerEvent::Lifecycle`, exactly M6-B01 §B item 4's pinned shape: `{"tick":8400,"event":"split","old":3,"new_a":7,"new_b":8}` / `{"tick":9000,"event":"merged","old_a":7,"old_b":4,"new":9}`.

**Shutdown ordering, precisely:**

```
1. Close the TCP listener (stop accepting new connections).
2. EdfScheduler::shutdown() — signals drivers/deadline-thread to stop after their
   own current work finishes; does not block.
3. EdfScheduler::join() — blocks until every in-flight tick has actually
   completed and every driver/deadline thread has exited. No new tick is admitted
   from this point on.
4. For every currently-live region (region_ids(), still valid — nothing was
   retired since step 3 stopped admission): call that region's own
   ChunkLifecycleManager::shutdown(&world) (M2-B05's WORLD-D25 flush barrier) —
   force-saves every resident dirty chunk, blocks on IoPool::drain_barrier.
   Iterated sequentially, region by region (a shutdown-time cost, never a
   steady-state one — parallelizing this is a future optimization, not required
   for correctness).
5. Gracefully close every live ConnectionHandle (Moderate-confidence flag: the
   exact shutdown/close method on `rusty_clanker_server::net::ConnectionHandle`,
   M1-B01, is not re-verified against that already-merged type's own current API
   surface by this blueprint — confirm the exact call at implementation time; the
   binding requirement is that every open TCP connection is closed cleanly, not a
   specific method name).
6. Drop RcWorkerPool (its own Drop impl, M0-B04, already joins every worker
   gracefully) and drop InProcessTransport.
7. Process exits 0.
```

`shutdown()` is installed as a `tokio::signal::ctrl_c()` handler in `run_embedded` and is also directly callable (a `pub fn shutdown(&self)` on this blueprint's own composition-root type) for the acceptance test that drives it without an actual OS signal.

### §L — Explicit non-goal: border-halo chunk ticking across a region boundary

ARCH-D10/D11's own "border-tick injection" mechanism — replicating a thin halo of a neighboring region's own chunks so a player near a boundary sees consistent simulation on both sides — is **not** implemented by this blueprint. Each region's own `TicketManager`/`ChunkLifecycleManager` pair (§L, one per region) computes ticket demand and loads chunks only from **its own** players, and only within **its own owned cells**; a player positioned near a region boundary may observe un-ticked neighbor-owned chunks at the edge of their own simulation distance until a future blueprint implements real border-halo replication. This is a genuine, named scope boundary — not silently absorbed — consistent with `01-server-architecture.md`'s own Open Questions never having pinned this mechanism's exact shape in the first place.

## Deliverables

### `crates/scheduler/src/pool/tick_clock.rs` (modify — one new method, §E.1)

Exactly `TickClock::try_advance`'s signature and behavior above.

### `crates/scheduler/src/executor.rs` (modify — one new method, §F)

Exactly `RcExecutor::tick_region_coalesced`'s signature and behavior above.

### `crates/scheduler/src/region_manager.rs` (modify — private representation change + two new methods, §D)

Exactly `RegionManager`'s new private field shapes, `tick_region_concurrent`, `with_region`, per §D. Every other existing public method's signature unchanged; `region(&self, id) -> Option<&ManagedRegion>` is removed and replaced by `with_region` (the one named exception, §D).

### `crates/scheduler/src/metrics/registry.rs`, `crates/scheduler/src/metrics/snapshot.rs`, `crates/scheduler/src/metrics/attribution.rs` (modify — additive, §F)

Exactly the `last_tick_task_count` field additions to `TickCpuCost`/`RegionMetricsSnapshot`, `RegionCpuState`'s new atomic counter, and `attribution.rs`'s one new `registry.record_task_dispatched(region)` call inside `region_tagged_task`'s wrapper (never inside `measure_inline`).

### `crates/scheduler/src/edf.rs` (new)

Exactly `EdfSchedulerConfig`, `SchedulerEvent`, `EdfScheduler` per §E.

### `crates/scheduler/src/lib.rs` (modify — one new module line, additive)

```rust
pub mod edf;
pub use edf::{EdfScheduler, EdfSchedulerConfig, SchedulerEvent};
```

### `crates/server/src/play/executor_bootstrap.rs` (new, §C.1)

Exactly `build_server_executor`'s signature; body is the code-moved union of `HardcodedWorld::new`'s and `TwoRegionWorld::new`'s own existing registration sequences.

### `crates/server/src/play/world.rs` (modify — `HardcodedWorld::new`'s body reduced to call `build_server_executor()`; every other line, every other method, every field unchanged)

### `crates/server/src/play/two_region_world.rs` (modify — `TwoRegionWorld::new`'s body reduced identically; every other line unchanged)

### `crates/server/src/config.rs` (modify — additive `SchedulerConfig`, §J.3)

### `crates/server/src/composition/mod.rs` (new)

```rust
//! The real, RegionManager-driven, multi-region composition root (M6-B07) —
//! replaces HardcodedWorld/TwoRegionWorld as rusty-clanker-server's own runtime
//! entry point without modifying either (Context §A).

pub mod fault_injection;
pub mod region_layout;
pub mod lifecycle_log;
pub mod metrics_snapshot_log;

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use rc_messaging::RegionId;

#[derive(Debug, thiserror::Error)]
pub enum CompositionError {
    #[error("chunk at {0:?} falls in an unclaimed grid cell under a pinned --region-layout — every cell a player or a fault-injection entry may ever touch must be declared in the layout file")]
    UnclaimedCellUnderPinnedLayout(rc_core::BlockPos),
    #[error("--fault-injection-schedule requires --region-layout — a dynamically-bootstrapped region has no stable label to schedule against")]
    FaultInjectionRequiresPinnedLayout,
    #[error("--region-layout names unsupported dimension '{0}' — only minecraft:overworld is supported")]
    UnsupportedDimension(String),
    #[error("--fault-injection-schedule names unknown region label '{0}'")]
    UnknownFaultInjectionLabel(String),
    // ... I/O / RON-parse variants, ordinary error handling, implementer's freedom.
}

/// Composition-root configuration, resolved from CLI + config file (§J).
pub struct CompositionConfig {
    pub bind_addr: String,
    pub offline: bool,
    pub world: crate::config::WorldConfig,
    pub scheduler: crate::config::SchedulerConfig,
    pub region_layout: Option<region_layout::LocalRegionLayoutSpec>,
    pub fault_injection: Option<fault_injection::LocalFaultInjectionSchedule>,
    pub region_lifecycle_log: Option<std::path::PathBuf>,
    pub metrics_snapshot_log: Option<std::path::PathBuf>,
    pub tick_log: Option<std::path::PathBuf>,
    pub region_tick_log: Option<std::path::PathBuf>,
    pub pinned_single: bool,   // --region-lifecycle pinned-single
}

/// The composition root itself. `Send + Sync` — every method below is safe to
/// call from the Tokio runtime's own connection-handling tasks while
/// `run_admission_loop` drives the EDF scheduler on its own dedicated thread.
pub struct ServerComposition { /* private: RcExecutor, RegionManager<'static>-via-Box::leak
    or an owning-self-referential wrapper (implementer's choice of exact lifetime
    resolution — RcExecutor must outlive RegionManager, both must outlive the
    listener loop; `Box::leak` for the executor, matching this project's own
    established "process-lifetime singleton, deliberately never freed" precedent
    for exactly this shape, is an acceptable, simple resolution), InProcessTransport,
    MetricsRegistry, EdfScheduler, RegionDirectory-derived label map, per-region
    ChunkLifecycleManager/TicketManager table, PlayerRouting side table,
    next_network_entity_id (SharedNetworkEntityIdAllocator, M4-B08, reused) */ }

impl ServerComposition {
    /// Executes §C's full startup sequence (steps 1-13; binding the listener and
    /// entering the accept loop, steps 14-15, are `run_embedded`'s own job, below).
    pub fn start(config: CompositionConfig) -> Result<Self, CompositionError>;

    /// The count/label-map values §J.2's RC_REGION_COUNT/RC_REGION_LAYOUT stdout
    /// lines are derived from.
    pub fn region_count(&self) -> usize;
    pub fn region_layout_json(&self) -> Option<String>;

    /// §K's full shutdown sequence.
    pub fn shutdown(&self);
}

impl crate::net::PlayerSessionSink for ServerComposition {
    fn accept(&self, session: crate::net::PlayerSession);
}
```

### `crates/server/src/composition/region_layout.rs`, `fault_injection.rs`, `lifecycle_log.rs`, `metrics_snapshot_log.rs` (new)

Exactly §I/§J/§K's signatures.

### `crates/server/src/lib.rs` (modify — real `run_embedded`, replacing the M0-B01 placeholder doc comment)

```rust
//! `rusty-clanker-server` — composition root binary + embeddable library target.

pub mod composition;
pub mod config;
pub mod net;
pub mod play;

/// Runs the server to completion (blocks until shutdown). The one entry point
/// both `main.rs` (a real process) and `rusty-clanker-client`'s own future
/// singleplayer embed (`07`'s CLIENT-D25/D27, WS-D2's own already-named
/// `run_embedded` obligation, discharged here for the first time) call.
pub async fn run_embedded(config: composition::CompositionConfig) -> std::io::Result<()>;
```

### `crates/server/src/main.rs` (modify — real content, replacing the M0-B01 placeholder)

```rust
#[derive(clap::Parser)]
struct Args { /* exactly §J's flag table */ }

fn main() -> std::io::Result<()> {
    let args = <Args as clap::Parser>::parse();
    let config = /* resolve CompositionConfig from args + --config file, §J.3 */;
    tokio::runtime::Runtime::new()?.block_on(rusty_clanker_server::run_embedded(config))
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly).** Every file below, plus every new `src/*.rs` file in Deliverables with function bodies `todo!()`-stubbed (struct/enum shapes, derives, doc comments, constant values unchanged), plus the additive/representation-changing diffs to `region_manager.rs`/`executor.rs`/`tick_clock.rs`/`metrics/*.rs`/`world.rs`/`two_region_world.rs`/`config.rs`/`lib.rs` (existing bodies of those already-merged files untouched except the two named, bounded edits — `HardcodedWorld::new`'s/`TwoRegionWorld::new`'s body reduction, and `region(&self,id)`'s removal in favor of `with_region`), is the test-authoring changeset, committed first. The implementation changeset fills in real bodies only; it must not modify any test file listed below, must not weaken any assertion, and must not touch any file under `crates/scheduler/tests/`, `crates/server/tests/` beyond what this section adds.

### `crates/scheduler/tests/region_manager_concurrent_regression.rs` (new — proves §D's representation change is behavior-preserving)

1. `all_lifecycle_hysteresis_scenarios_pass_under_concurrent_representation` — re-runs every one of M0-B06's own 14 `lifecycle_hysteresis.rs` scenarios (split-triggers-at-40th-tick, merge-requires-100-ticks, load-conserved-on-merge, etc.), verbatim, against the new `RegionManager` — every assertion, every expected value, byte-for-byte identical to M0-B06's own originals.
2. `with_region_returns_identical_data_to_the_old_region_accessor_shape` — for a spawned region, `manager.with_region(id, |r| r.cells().clone())` equals what a `region(id).unwrap().cells().clone()` would have returned pre-this-blueprint (a direct construction-and-compare, not a claim).

### `crates/scheduler/tests/tick_clock_try_advance.rs` (new, §E.1)

3. `try_advance_never_compounds_drift_over_many_calls` — `MockTickWaiter`-driven (reusing M0-B04's own mock pattern), 12,000 simulated calls with scripted over/under-budget gaps exactly mirroring `tick_clock_drift.rs` test 1; asserts every returned `TickTiming.scheduled_deadline` matches `start + SERVER_TICK_PERIOD * N` exactly.
4. `try_advance_returns_none_before_the_deadline` — a fresh clock, `try_advance(now)` called before any time has passed → `None`, `next_deadline()` unchanged.
5. `try_advance_matches_await_next_tick_for_the_same_schedule` — a clock driven via `try_advance` in a loop (always calling exactly at or after each deadline) produces the identical sequence of `TickTiming` values a second, independent clock driven via `await_next_tick` (real waits, small scale) produces for the same waiter schedule.

### `crates/scheduler/tests/edf_admission_property.rs` (new — the task's named EDF property-test pair)

Setup shared by every test below: a real `RcExecutor` (empty bootstrap, `common::empty_bootstrap`), a real `RcWorkerPool::new(4)`, a real `RegionManager::new_with_metrics(&executor, 50.0, Arc::clone(&metrics))`, a real `EdfScheduler`, a `MockTransport`.

6. **`edf_admission_never_yields_to_background_ahead_of_an_overdue_region`** — spawn 3 regions, each with a `SyntheticLoadProfile` sized so its own tick takes long enough to keep at least one region reliably `Ready` throughout the test window; continuously `submit_background` trivial jobs (each appending its own start-`Instant` to a shared, `Mutex`-guarded log) throughout a bounded run; after the run, assert that **every** logged background-job start time falls in a window where the ready-heap was observed empty at the driver's own last check (instrumented via a test-only hook on `EdfScheduler`, or verified indirectly: every background job's own start time is later than the completion time of whichever region tick was most recently admitted before it, for every region — a mechanical, checkable proxy for "never admitted ahead of an overdue region").
7. **`edf_violation_counter_stays_zero_under_synthetic_multi_region_load`** — spawn 8 regions with varied `SyntheticLoadProfile` costs (some deliberately heavier, forcing them to fall behind and become repeatedly `Ready`); run for 2,000 simulated ticks' worth of wall time (a bounded few seconds real time, small enough for Tier 1); after the run, assert `metrics.edf_violation_count() == 0` — the exact zero-violation-by-construction property M6-B02's own `EdfTracker` already proves is achievable, now proven achievable by *this blueprint's own real scheduler*, not merely by hand-fed test calls.
8. `edf_admission_prefers_the_earliest_deadline_among_several_simultaneously_ready` — a deliberately constructed scenario where 3 regions become `Ready` within the same `poll_interval` window with distinct deadlines; assert the driver admits them in strictly ascending-deadline order (via the same `on_event`-logged admission-order proxy as test 6).

### `crates/scheduler/tests/edf_coalesced_dispatch.rs` (new)

9. **`coalesced_dispatch_engages_for_a_quiet_region_and_reports_near_zero_cpu`** — one region with `SyntheticLoadProfile { busy_work_micros: 200 }` (well under the 5ms/10%-of-50ms quiet threshold), driven through the real `EdfScheduler` for 40+ consecutive admitted ticks; after the run, `metrics.is_near_zero_dedicated_cpu(id) == true` and the last `RegionMetricsSnapshot.last_tick_task_count == Some(1)`.
10. `hot_region_never_engages_the_coalesced_path` — one region with `SyntheticLoadProfile { busy_work_micros: 40_000 }` (40ms, well over quiet), 5 synthetic conflict-free systems registered in `DomainGroup::AiPhysics` so a fine-grained tick submits `> 1` tasks; after several admitted ticks, `last_tick_task_count` is consistently `Some(n) where n > 1`.
11. `coalesced_dispatch_matches_fine_grained_state` — the identical scripted-content scenario from M0-B05's own `sync_points.rs`/`determinism.rs` (a pre-existing entity, several conflicting `Query<&mut A>` systems), ticked once via `RcExecutor::tick_region` and once via `tick_region_coalesced` on two independently-constructed fresh regions; asserts identical final `A.0` value and identical `transport.sent()` sequence.

### `crates/scheduler/tests/live_merge_split_under_load.rs` (new — the task's named live-merge/split test)

12. **`live_merge_and_split_execute_under_shifting_synthetic_load`** — two adjacent single-cell regions started quiet (triggering a real merge via the scheduler's own admission path, never `force_merge`) after their combined EWMA stays under threshold for 100 real-admitted ticks; then the merged region's own `SyntheticLoadProfile` is pushed well over the split threshold for 40 real-admitted ticks, triggering a real split (never `force_split`); assert: `transport`'s `register_region`/`deregister_region` call counts (a `MockTransport` extended with call-counting, or a real `InProcessTransport` instrumented via its own `is_registered`) match the expected sequence exactly; the `EdfScheduler`'s own internal region set (queried via a test-only accessor) never contains a retired id after its own event fires; `on_event` observes exactly one `Lifecycle{Merged}` then exactly one `Lifecycle{Split}`, in that order.

### `crates/server/src/composition/fault_injection.rs` — `crates/server/tests/fault_injection_resolver.rs` (new, mirrors M6-B01's own test 17-21 locally)

13. `resolve_load_multiplier_default_is_one`, `resolve_load_multiplier_matches_within_half_open_range`, `resolve_load_multiplier_takes_max_of_overlapping_entries`, `resolve_load_multiplier_is_deterministic_across_independent_parses` — the identical four scenarios M6-B01 §G's own acceptance tests already specify (Prerequisites), re-authored against this blueprint's own local types, proving the local restatement is byte-for-byte algorithmically identical.

### `crates/server/tests/composition_in_process.rs` (new — Tier 1, real loopback sockets, ≤20 connections, mirroring M4-B08's own `TwoRegionWorld` in-process test shape exactly, never a subprocess)

14. `executor_extraction_preserves_hardcoded_world_behavior` — re-runs `play_chunk_set.rs`'s own `enter_play_sends_a_well_formed_login_and_chunk_batch` scenario verbatim against a `HardcodedWorld` constructed after §C.1's extraction; asserts identical packet sequence/content to M1-B05's own already-committed expectations (a regression proof for the code-motion refactor, not a new behavior).
15. `dynamic_bootstrap_spawns_exactly_one_region_on_first_join` — `ServerComposition::start` with no `region_layout`; a real loopback bot joins at spawn; assert `region_count() == 1` after the join completes (and `== 0` immediately after `start()` returns, before any join — proving RC_REGION_COUNT's own "live at the moment it prints" honesty, §C step 10).
16. `pinned_layout_bootstrap_spawns_every_declared_region_and_none_else` — `ServerComposition::start` with a 3-region `LocalRegionLayoutSpec`; assert `region_count() == 3` immediately, `region_layout_json()` names all 3 labels with distinct `RegionId`s.
17. `join_at_unclaimed_cell_under_pinned_layout_is_rejected` — a pinned 1-region layout; a bot's spawn position (test-configured) falls outside its cell; assert `CompositionError::UnclaimedCellUnderPinnedLayout` is surfaced (via the connection's own rejection path — implementer's choice of exact signal, a clean disconnect rather than a panic).
18. `player_routing_survives_a_live_merge` — two adjacent pinned regions, `merge_split_enabled: true`; a bot joins region A; both regions are driven quiet until they merge (reusing test 12's own scheduler-level trigger, now end-to-end through `ServerComposition`); assert the bot's own movement packets continue to be accepted without disconnect after the merge (its `PlayerRouting` was correctly redirected, §G step 4).
19. `fault_injection_schedule_isolates_the_targeted_region` — a pinned 2-region layout, one region named in a fault-injection entry with `load_multiplier: 20.0`; both driven via the real scheduler for a bounded window; assert (via `--metrics-snapshot-log`-equivalent in-process polling of `metrics.snapshot(&pool)`) the targeted region's own `tick_duration_ewma_ms()` rises well past the untargeted sibling's.
20. `shutdown_flushes_every_region_and_drains_connections` — a pinned 2-region layout; a real block-place (via M2-B07's own established interaction pattern) dirties a chunk in each region; call `composition.shutdown()`; assert (via a fresh `AnvilDiskBackend::open` against the same `world_dir` after shutdown) both dirtied chunks round-trip with their placed block present — the exact WORLD-D25 flush-barrier proof, generalized to N regions; assert every previously-open loopback socket observes a clean close (read returns EOF, not a reset) within a bounded timeout.

### `crates/server/tests/composition_cli.rs` (new — Tier 1, no server subprocess, `clap`'s own parser exercised directly)

21. `help_advertises_region_layout_and_metrics_snapshot_log` — `Args::try_parse_from(["rusty-clanker-server", "--help"])`'s own generated help text (via `clap`'s `render_help()` or the `--help` exit-with-usage path) contains both `--region-layout` and `--metrics-snapshot-log` substrings — the exact pair `xtask::release::detect_region_layout_support` (M6-B05) and M6-B06's own `detect_m6_composition_root_support` check for.
22. `region_lifecycle_pinned_single_forces_driver_count_one_and_disables_merge_split` — parsed `Args` with `--region-lifecycle pinned-single` resolve to a `CompositionConfig` whose `scheduler.driver_count` resolves to `1` and whose region-layout (if any) has its own `merge_split_enabled` forced `false` regardless of the file's own declared value.

### `xtask/tests/composition_root_path_guard_coverage.rs` (new)

23. `path_guard_already_covers_m6_b07s_own_new_paths` — mirroring every prior blueprint's identical self-test: `path_guard::check_paths(ChangesetType::Implementation, &["crates/scheduler/src/edf.rs".into(), "crates/server/src/composition/mod.rs".into()])` → both already matched by the existing `crates/scheduler/**`/`crates/server/**`-equivalent catch-all rows — `assert_eq!(violations.len(), 2)`, proving no `path_guard.rs` edit is needed.

## Implementation steps

1. **`tick_clock.rs`.** Implement `try_advance` exactly per §E.1. Observable: `tick_clock_try_advance.rs` passes; every pre-existing `pool_*`/`tick_clock_drift.rs` test still passes unmodified.
2. **`region_manager.rs`/`grid.rs`-adjacent `directory.rs`.** Change the private storage representation (§D); reimplement every existing method against it; add `tick_region_concurrent`, `with_region`; remove `region(&self,id)`, updating every caller across `crates/scheduler/tests/` (the one, bounded, named signature change). Observable: `region_manager_concurrent_regression.rs` passes; every pre-existing M0-B06/M6-B02/M6-B03 test passes with its own `region(id)` call sites mechanically updated to `with_region`.
3. **`executor.rs`.** Implement `tick_region_coalesced` per §F (sequential in-body execution of every wave, single outer `region_tagged_task` wrap when metrics attached). Observable: `edf_coalesced_dispatch.rs` test 11 (state-equivalence) passes standalone against hand-driven calls.
4. **`metrics/{attribution,registry,snapshot}.rs`.** Add `last_tick_task_count` tracking end to end (§F). Observable: compiles; exercised by step 6.
5. **`edf.rs`.** Implement `EdfScheduler` per §E.2-§E.5. Observable: `edf_admission_property.rs`, `edf_coalesced_dispatch.rs` tests 9-10, `live_merge_split_under_load.rs` all pass.
6. **`lib.rs`.** Add `pub mod edf;` + re-exports. Observable: `cargo build -p rc-scheduler --all-features` succeeds with zero `todo!()` remaining.
7. **`rc-scheduler` full gates.** `fmt-check`/`lint`/`lint-deps`/`test` all exit 0; every pre-existing test in the crate still passes.
8. **`executor_bootstrap.rs`.** Extract `build_server_executor` (§C.1) — a careful, literal transcription of `HardcodedWorld::new`'s and `TwoRegionWorld::new`'s own existing registration calls. Observable: `executor_extraction_preserves_hardcoded_world_behavior` passes.
9. **`world.rs`/`two_region_world.rs`.** Reduce both constructors' bodies to call the shared function. Observable: every pre-existing `rusty-clanker-server` test (M1-B05 through M4-B08) still passes unmodified.
10. **`composition/{region_layout,fault_injection,lifecycle_log,metrics_snapshot_log}.rs`.** Implement per §I/§J/§K. Observable: `fault_injection_resolver.rs` passes.
11. **`config.rs`.** Add `SchedulerConfig` (§J.3). Observable: compiles.
12. **`composition/mod.rs`.** Implement `ServerComposition::{start, shutdown, region_count, region_layout_json}` and `PlayerSessionSink::accept` per §B/§C/§G/§H/§K. Observable: `composition_in_process.rs` tests 15-20 pass.
13. **`main.rs`/`lib.rs`.** Wire the real `clap::Parser` CLI (§J) and `run_embedded`. Observable: `composition_cli.rs` passes; `cargo run -p rusty-clanker-server -- --help` prints real usage.
14. **Path-guard coverage proof.** Add `xtask/tests/composition_root_path_guard_coverage.rs`. Observable: test 23 passes with zero edits to `xtask/src/path_guard.rs`.
15. **Full acceptance suite + gates on both crates.** `cargo nextest run -p rc-scheduler -p rusty-clanker-server`; `fmt-check`/`lint`/`lint-deps`/`test`/`path-guard` all exit 0.
16. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first, changeset boundary.** All 23 acceptance tests above are written and committed before the functions/types they exercise exist (`todo!()`-stubbed where needed). The implementation changeset never modifies any test file listed above, never weakens an assertion, and — in particular — never edits any pre-existing test file outside this blueprint's own new files except the mechanical `region(id)` → `with_region(id, ...)` call-site update named in step 2, which must preserve every pre-existing test's own asserted values exactly.

(b) **Protected paths, changeset label.** Every file this blueprint's Deliverables touch falls under an already-existing `crates/scheduler/**`/`crates/server/**`/`xtask/**` `PROTECTED_PATHS` row (proven by acceptance test 23) — the entire changeset is labeled `Changeset-Type: governance`, never `implementation`, per this lineage's established convention.

(c) **No new external dependencies beyond the pinned set.** `clap` (already workspace-pinned, already `xtask`'s own established CLI library), `parking_lot`, `serde`/`serde_json` (all already present in both crates) are the only crates this blueprint's own code uses — no new `[workspace.dependencies]` entry anywhere.

(d) **No Mojang or third-party reimplementation code.** Every mechanism here (the EDF scheduler, the coalesced-dispatch shape, the striped concurrent `RegionManager` representation, the dynamic-bootstrap rule) is this blueprint's own original resolution of ARCH-D18-D20's/ARCH-D5-D6's own already-cited decision text, or reuses this project's own already-built machinery unmodified (ASSET-D18/D19/D30).

(e) **`HardcodedWorld`/`TwoRegionWorld` are never modified beyond the one named, bounded extraction (§C.1).** No field, no method signature, no test file under `crates/server/tests/` predating this blueprint changes in any way other than the internal body reduction of their own two constructors.

(f) **`RegionManager`'s only permitted public-signature change is `region(&self, id) -> Option<&ManagedRegion>`'s removal in favor of `with_region`, named explicitly in §D.** Every other existing `RegionManager`/`ManagedRegion`/`RegionDirectory`/`RcExecutor`/`TickClock`/`MetricsRegistry` public signature is unchanged.

(g) **Unsafe-code policy — none permitted.** Every deliverable in this blueprint (the new striped-lock representation, the EDF scheduler, the composition root) is safe Rust — `parking_lot::{Mutex, RwLock}` and `std::sync::Arc`/`atomic` are the only concurrency primitives used, no raw pointers, no `unsafe` blocks anywhere in this blueprint's own new code.

(h) **Border-halo chunk ticking across a region boundary is explicitly out of scope (§L)** — not implemented, not stubbed with placeholder behavior, named as a genuine, currently-open gap for a future blueprint.

(i) **This blueprint's own Tier-1 gate never spawns more than 20 real concurrent sockets** (mirroring M6-B01's own binding self-limit, restated) — every `composition_in_process.rs` test connects a small, bounded handful of real loopback bots, never a load-test-scale swarm; the real 200-bot/8-region/15-minute claim remains M6-B06's own already-built Tier-3 job, which becomes exercisable, not re-implemented, by this blueprint.

## Verification commands

```
cargo build -p rc-scheduler -p rusty-clanker-server --all-features
cargo nextest run -p rc-scheduler -p rusty-clanker-server
cargo test --doc -p rc-scheduler -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p rusty-clanker-server -- --help
```

All run headless, identically, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43) — no external oracle, no Java, no network access beyond loopback required for any of them. CI green on both OS legs, clean checkout, is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Open questions

- **In-memory chunk-residency hand-off across a live merge/split** (§G) is deliberately not implemented — the new region's chunk-lifecycle pair starts cold and reloads from durable disk state rather than carrying live `World` entity/chunk data across the boundary. A future blueprint may build a genuine hand-off; this blueprint's own correctness (nothing dirty is ever lost, WORLD-D25) does not depend on that optimization landing.
- **M5-B09's exact worldgen job-submission API** (§E.5) is not a prerequisite of this blueprint and is not independently re-verified here — `EdfScheduler::submit_background` is the concrete, tested admission-priority facility ARCH-D20's own interface note already promised; wiring M5-B09's real chunk-generation call sites through it is a small, follow-up integration a future blueprint (or this blueprint's own implementer, if M5-B09 has landed by then) completes without needing to design the admission rule itself.
- **`ConnectionHandle`'s exact graceful-close method name** (§K step 5) is flagged, not re-verified against M1-B01's own current API surface — confirm at implementation time; the binding requirement is only that every connection closes cleanly on shutdown.
- **Border-halo chunk ticking** (§L) remains fully open, as stated — the concrete mechanism, its own message-contract shape, and its own performance cost are all future work.
- **Cluster mode (CLUSTER-D26/D27)** is entirely out of scope — this blueprint's own composition root implements monolithic mode only; the proxy/QUIC/cross-node role split is `M7`'s.
