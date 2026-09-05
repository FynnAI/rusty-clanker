# M7-B09 — Cluster Mode Acceptance Harness

| Field | Content |
|---|---|
| ID | M7-B09 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | **M7-B01** (`rc-transport-net::NetworkTransport`/`NodeId`/`NodeDirectory` — real loopback-QUIC pattern this blueprint's own Tier-1 handoff proof reuses unmodified, restated §D). **M7-B02** (`rc-cluster::ClusterNode`/`DirectoryCache`/`RegionLease`/`Epoch`/`HealthMonitorConfig` seed defaults — restated §E). **M7-B03** (`RebalancerEngine`/`MigrationCoordinator` — cited only to confirm this blueprint does **not** exercise planned rebalancing, out of scope §A). **M7-B04** (`ObjectStoreBackend`/`RegionManifest` — cited only for the state-loss-bound assertion's own storage-durability reasoning, §E.3; this blueprint never depends on `rc-chunk-storage` directly). **M7-B05** (`TakeoverOrchestrator`/`DirectoryReconciler`/`RegionResumeHandler`/`LeastRegionCountStrategy`/`TakeoverEvent` and, load-bearing, its own §C takeover-time budget decomposition — restated exactly and turned into this blueprint's own AC2 gates, §E). **M7-B06** (`rc-proxy`'s complete shipped surface — `ProxyServer`/`NodeAcceptor`/`ControlFrame`/`ProxyRoutingTable`/`ForwardedIdentity`/`ProxyMetricsSink`/`ProxyConnectionId`, its own 12 pre-existing Tier-1 tests — restated §D, never re-implemented). **M7-B07** (`crates/server/src/play/cluster_handoff.rs`'s `classify_player_crossing`/`send_control`/`try_recv_control`, and — load-bearing — its own already-shipped, already-Tier-1-green `crates/server/tests/play_cluster_handoff_walk.rs::player_crosses_a_live_node_boundary_within_budget_with_bounded_position_delta` test, which is this blueprint's own primary AC1 proof, extended rather than duplicated, §D). **M7-B08** (`ClusterConfig`/`resolve_role`/`ClusterNodeComposition`, `deploy/cluster/docker-compose.cluster-test.yml`, `xtask compose-topology-gate`, and — load-bearing, restated honestly — its own Context §A items 1 and 3: no concrete `openraft::RaftNetworkFactory`/`JoinClient` over a real network exists yet, and `main.rs`'s `ClusterProxy`/`ClusterNode` arms exit `EXIT_CLUSTER_INTEGRATION_PENDING` rather than actually serving; this blueprint's own real, multi-process legs inherit that exact gap, restated once, §A). **M6-B01** (`rc_paritybot::loadtest` — `MultiRegionScenario`/`plan_bot_layout`/`run_multi_region_scenario`/`eight_region_mixed.ron`, reused byte-for-byte, one additive field only, §F). **M6-B02** (`MetricsRegistry`/`RegionMetricsSnapshot` shape — consumed only through M6-B06's already-established local mirror, never a direct dependency). **M6-B04** (`xtask::reference_host::{TierId, ReferenceHostSpec, HostFingerprint, probe_host, match_tier, gate, AuthoritativeRunReport}` — this blueprint's own real, full-scale run is gated through it exactly as M6-B06's was, restated §I). **M6-B06** (`M6ReportResult`/`build_report`/`evaluate_ac1`/`evaluate_ac2`/`evaluate_ac3`, `metrics_snapshot_log.rs`'s `MetricsSnapshotEntry`/`analyze_region_tps` mirror, and — load-bearing, reused **unmodified** as this blueprint's own AC4 evaluator — restated in full §H). **M0-B08** (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to, VERIFY_OUT_DIR, exit_code_for}`, `xtask::path_guard::{PROTECTED_PATHS, ChangesetType, check_paths}` — reused unmodified). **M3-B07** (`rc-gametest`'s `RedstoneTrace`/`TickSnapshot`/`BlockObservation`/`ContraptionSpec`/`diff_traces`/`load_spec`, and its own committed, fully-specified five-contraption corpus — two of which this blueprint names by ID and reuses byte-for-byte, never forked, §G). **M3-B08** (`rc_test_harness::process::{ManagedServer, ManagedServerConfig, spawn_server}`, the established additive-CLI-flag/additive-struct-field extension pattern, the `M<n>ReportResult`-wraps-`TierResult`-via-`#[serde(flatten)]` template, the "harness self-test proves a perturbed input is actually caught" convention — all reused as this blueprint's own direct template). |
| Implements | `11-roadmap-milestones.md`'s M7 Acceptance Criteria 1–4, verbatim (restated and precisely defined, §B). CLUSTER-D7 (≤30 ms p99 cross-node latency budget, co-located-topology assumption — reused as this blueprint's own compose-topology validity argument, §C). CLUSTER-D8 (hot-border co-location migration and its own graceful-degradation-to-N-tick-lag fallback — restated as the documented, honestly-unexercised latency allowance in the AC3 redstone-split assertion, §G.4). CLUSTER-D16/D17/D19/D22 (takeover, durability bound, epoch fencing, six-step handoff — consumed via M7-B05/M7-B07's own already-fixed budgets and mechanisms, never re-derived). CLUSTER-D26/D27 (compile-in/activate-by-config split — the literal subject of AC4, restated §H). TEST-D34/D37 (CI matrix and tier placement — restated §I for every one of this blueprint's own legs). TEST-D40 (machine-readable report format — this blueprint's own `target/verify/m7-acceptance.json`, §I). TEST-D45/D46/D50 (test-first changeset boundary, protected-path coverage, CI-is-authority — restated). |
| Crates touched | `crates/testing/test-harness/` (`rc-test-harness`, additive: `process.rs` gains two inherent methods on `ManagedServer`; new `cluster_continuity.rs` module). `crates/testing/paritybot/` (`rc-paritybot`, additive: one new field on `MultiRegionScenarioConfig`; zero new scenario files — `eight_region_mixed.ron` is reused byte-for-byte). `crates/testing/gametest/` (`rc-gametest`, additive: new `cluster_replay.rs` module; one new committed RON fixture, `corpus/redstone/m7_cluster_split_layout.ron`). `xtask` (additive: `src/m7_report.rs`; one new `Command::M7Report` variant; `.github/workflows/ci.yml` extended with one new job plus a one-line reconciliation of M7-B08's already-merged `compose-topology-gate` job's own `if:` condition, Context §A.1). `deploy/cluster/` (additive, non-code: `docker-compose.m7-acceptance.yml`, a compose **overlay** — never a modification of M7-B08's own already-committed `docker-compose.cluster-test.yml`). **Not** any file under `crates/cluster/`, `crates/transport-net/`, `crates/proxy/`, or `crates/server/src/play/cluster_handoff.rs` — every mechanism this blueprint measures is already built by a prerequisite blueprint; this blueprint adds **zero** production cluster-mode behavior, only measurement and orchestration. |
| Estimated scope | L, explicitly oversized against `00-blueprint-spec.md`'s ~800-line/~300-line-Context guidance — the same class of stated, deliberate exception `M6-B06`/`M7-B02`/`M7-B06`/`M7-B08` already established. Four acceptance criteria, each needing its own topology/measurement/gate, plus one combined completion report and one CI-tier reconciliation, share enough vocabulary (the compose overlay, the takeover-budget decomposition, the report-wrapping convention) that splitting them into separate blueprints would force each to restate the other three's shared plumbing from scratch while leaving no single blueprint that actually proves M7 is "done." |

## Goal & Done definition

Wire all four of M7's acceptance criteria (`11-roadmap-milestones.md`, quoted verbatim in §B) into one precise, agent-executable, machine-readable measurement, exactly as `M3-B08`/`M5-B10`/`M6-B06` did for their own milestones — reusing every already-built mechanism named in Prerequisites and adding **zero** new cluster-mode production behavior. Concretely: (1) **AC1 (cross-node border crossing)** — extend `M7-B07`'s own already-shipped, already-Tier-1-green real-loopback-QUIC handoff test with a packet-level "zero Login/Configuration re-entry" assertion (the precise, exact reading of "zero client-visible disconnect/loading screen") and wire the identical measurement onto the real compose topology as an honestly-gated Tier-3 leg; (2) **AC2 (node-kill takeover)** — a real `SIGKILL` of one real `rusty-clanker-server` cluster-node subprocess in the compose topology, `TakeoverOrchestrator`'s own `TakeoverEvent` stream and `DirectoryCache` state used to measure the detection/reassignment/resume-I/O budget decomposition `M7-B05` §C already fixed, plus an unaffected-sibling-region zero-interruption assertion and an exact, reconstructable state-loss-bound assertion per CLUSTER-D17; (3) **AC3 (3-node+2-proxy 200-bot run, plus the redstone-corpus cross-node split)** — `M6-B01`'s `eight_region_mixed.ron` scenario re-run through a two-proxy fan-out extension, evaluated by `M6-B06`'s own evaluators unmodified, **plus** two named, already-shipped `M3-B07` contraptions (`redstone/pulse/torch_inverter_basic`, `redstone/update_order/two_torch_and_gate`) replayed with their own bounding boxes deliberately straddling a pinned, two-node region border, bit-identity asserted via `M3-B07`'s own `diff_traces` against a monolithic reference capture of the identical contraptions; (4) **AC4 (monolithic no-regression)** — `M6-B06`'s own harness re-invoked, completely unmodified, against the identical cluster-capable build with no `[cluster]` config, plus a new runtime-level "no live cluster thread/socket/log line" observation this blueprint adds as a complement to `M7-B08`'s own compile-time inertness proof; (5) one combined, machine-readable `M7CompletionReport` (four criterion sub-reports plus the AC4 sub-report, all TEST-D40-shaped); (6) precise CI-tier placement for all four legs, restated, including a Finding reconciling `M7-B08`'s already-merged `compose-topology-gate` job into the `job`-choice-input scheme `M6-B06` §G.1 already established for its sibling `workflow_dispatch`-only jobs; (7) four harness self-tests, each proving one of this blueprint's own evaluators actually catches the specific failure mode named in this blueprint's own task brief.

This blueprint does **not** implement `M7-B08`'s own still-open Context §A items 1 (a concrete, real-network `RaftNetworkFactory`/`JoinClient`) or 3 (`main.rs`'s `ClusterProxy`/`ClusterNode` real-serving wiring) — no blueprint through `M7-B08` has built either. Every one of this blueprint's own **multi-process, real-subprocess, docker-compose-topology** legs (AC2's real kill; AC3's real 200-bot/2-proxy run and the redstone-split capture-from-a-live-cluster leg; AC1's own real-compose leg) is wired, correct-by-construction, and fails closed with the identical, actionable, already-established `EXIT_CLUSTER_INTEGRATION_PENDING` signal until that work lands — restated honestly, not glossed over, exactly `M7-B08` §G's own "the honest gate" framing, never a condition of this blueprint's own Tier-1 Done state. **AC1 is the one criterion with a genuine, real, already-green exception**: `M7-B07`'s own handoff test needs no `main.rs` role-wiring at all (it constructs `ProxyServer`/`NodeAcceptor` as library types directly, over real loopback QUIC, against in-process `ClusterNode` test doubles for raft) — this blueprint's own extension of that test is therefore real, Tier-1, and green **today**. **AC4 is the one criterion whose wiring is fully real and provable today independent of both open gaps** — it exercises no cluster networking at all, only `resolve_role`'s config-presence gate (`M7-B08`, already shipped) and `M6-B06`'s own harness (already shipped); its own **real, full 200-bot/15-minute** run remains gated by `M6`'s own pre-existing `reference-host-gate` mechanism (`M6-B04`/`M6-B06`), a fact about `M6`'s own milestone state this blueprint does not re-litigate, only re-invokes unmodified.

Done when:

- [ ] `cargo build -p rc-test-harness -p rc-paritybot -p rc-gametest -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-test-harness -p rc-paritybot -p rc-gametest -p xtask -p rusty-clanker-server`, using **only** synthetic in-memory data, `M7-B07`'s own already-real in-process loopback-QUIC fixture, or the shipped `M3-B07` RON fixtures — no docker, no compose, no real oracle, no real multi-process cluster, required to go green.
- [ ] Every pre-existing `M7-B01`..`M7-B08` test still passes, byte-for-byte unmodified — in particular every one of `M7-B06`'s 12 `rc-proxy` cases and `M7-B07`'s own `play_cluster_handoff_walk.rs` case.
- [ ] The four mandatory harness self-tests (`dropped_packet_during_handoff_fails_ac1`, `slow_takeover_fails_ac2`, `perturbed_cross_node_replay_fails_ac3`, `live_cluster_thread_despite_no_config_fails_ac4`) all pass, each proving the named failure mode is actually caught, not merely asserted possible.
- [ ] `ac1_zero_reentry_extends_the_real_handoff_walk` passes: reusing `M7-B07`'s own real fixture unmodified, asserts zero Login/Configuration-phase packets are observed on the client's own connection for the full duration of the scripted crossing.
- [ ] `cargo run -p xtask -- m7-report --help` prints usage with zero panics.
- [ ] `cargo run -p xtask -- m7-report --criterion 4 --server-bin <cluster-capable-binary>` (no `[cluster]` config) exercises AC4's real wiring end-to-end against a real, single, non-cluster-configured `rusty-clanker-server` process and produces a `target/verify/m7-acceptance.json` with a `pass`/`fail` AC4 section — the one criterion whose real leg this blueprint's own Tier-1 gate can exercise without docker.
- [ ] `cargo run -p xtask -- m7-report --criterion 1|2|3 --server-bin <bin> --compose` (no live docker daemon) fails closed with the exact, actionable `ClusterIntegrationPending` message this blueprint defines, exit non-zero, `target/verify/m7-acceptance.json` reporting the requested criterion's own section as `status: "fail"` with that message — proven without docker.
- [ ] `crates/testing/gametest/corpus/redstone/m7_cluster_split_layout.ron` validates (`rc_paritybot::loadtest::validate`-shaped region-layout checks, restated §G.2), names exactly two `RegionCellGroup`s sharing the `x = 0` cell boundary, and both named contraption IDs' own bounding boxes (computed via `M3-B07`'s already-public `bounding_box` helper) straddle that boundary when placed at this blueprint's own pinned world origins.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changeset (labeled per Constraints) — every new path already falls under an existing `PROTECTED_PATHS` row or is added by this blueprint's own additive edit, proven by `path_guard_already_covers_m7_b09s_own_new_paths`.
- [ ] `.github/workflows/ci.yml`'s `on.workflow_dispatch.inputs.job` choice gains `m7-acceptance` as a fourth option, and `M7-B08`'s own `compose-topology-gate` job's `if:` condition is reconciled to check `inputs.job == 'compose-topology-gate'` (Context §A.1's Finding) — a YAML-parse check, not a runtime CI assertion.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-test-harness -p rc-paritybot -p rc-gametest` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). This blueprint's own new `m7-acceptance-gate` CI job (Deliverables) is `workflow_dispatch`-only and is **not** part of the required Tier-1 status-check set; its own first meaningfully-green run for AC1/AC2/AC3's real-topology legs is a **later** milestone-acceptance signal, gated on `M7-B08`'s own still-open Context §A items 1 and 3 landing — restated honestly, never a condition of this blueprint's own Done state. AC4's real leg's first green run is gated on `M6`'s own pre-existing `reference-host-gate`, likewise not a condition here.

## Context (self-contained)

### §A — Scope boundary: what this blueprint measures, and the one gap every real multi-process leg inherits

This blueprint builds **zero** new cluster-mode production behavior. Every mechanism AC1–AC4 measure already exists, fully specified, in a prerequisite blueprint — this blueprint's own job is precisely "wire the measurement," the identical role `M3-B08`/`M5-B10`/`M6-B06` each already played for their own milestone, restated here for M7. Two genuine, already-named gaps in the prerequisite chain are inherited, not introduced, by this blueprint — restated once here, in full, rather than re-explained at each of the three criteria they affect:

1. **No concrete, real-network `openraft::RaftNetworkFactory`/`JoinClient` exists** (`M7-B08` Context §A item 1: `ClusterNodeComposition::start` is generic over both, and `main.rs`'s `ServerRole::ClusterNode` arm has no concrete type to instantiate them with today, so it exits `EXIT_CLUSTER_INTEGRATION_PENDING`, code `3`).
2. **`rc-proxy`'s own runtime wiring into `rusty-clanker-server::main.rs` does not exist** (`M7-B08` Context §A item 3: `role = "proxy"` parses and validates correctly, but `main.rs`'s `ServerRole::ClusterProxy` arm has no concrete `ProxyServer` construction to hand it to, so it too exits `EXIT_CLUSTER_INTEGRATION_PENDING`) — this is true **despite** `rc-proxy` itself (`ProxyServer`, `NodeAcceptor`, the whole `M7-B06`/`M7-B07` surface) being fully built and Tier-1-proven as a **library**; only the binary's own role-dispatch wiring is missing.

Any of this blueprint's own measurements that requires **real, separate OS processes** speaking real cluster networking — a real multi-node compose topology, a real `SIGKILL`, a real 200-bot run through two real proxy listen ports — therefore cannot reach a genuinely serving state today, for the identical reason `M7-B08`'s own `compose-topology-gate` job cannot yet go green (`M7-B08` Context §G, restated verbatim: *"this blueprint's own `compose-topology-gate` xtask verb and CI job are nonetheless real, complete, runnable code today... What is not claimed is that the job's first real run against a live Docker host is green today."*). This blueprint's own three affected legs (AC1's real-compose leg, AC2's real kill, AC3's real 200-bot/2-proxy run and the redstone-split live-capture leg) all fail closed, honestly and identically, with the `ClusterIntegrationPending` error this blueprint defines (§I) the instant `--compose` is requested — never a placeholder pass, never a silent skip. **The one exception, stated precisely because it is genuinely different**: AC1's *primary* proof (§D) needs neither gap closed, because `M7-B07`'s own test constructs `ProxyServer`/`NodeAcceptor` directly as library types against real loopback QUIC and in-process `ClusterNode` doubles — it never goes through `main.rs` at all. That leg is real and green today; only AC1's *additional*, real-compose-topology leg (§D.3) inherits the gap.

**§A.1 — Finding: `M7-B08`'s `compose-topology-gate` job needs the same `workflow_dispatch` reconciliation `M6-B06` §G.1 already applied to `reference-host-gate`/`release`.** `M6-B06` §G.1 changed both `M6-B04`'s `reference-host-gate` and `M6-B05`'s `release` jobs from a bare `if: github.event_name == 'workflow_dispatch'` to one gated by a shared `inputs.job` choice, specifically because two independent `workflow_dispatch`-only jobs sharing no distinguishing input both fire on a single manual dispatch. `M7-B08` landed its own `compose-topology-gate` job **after** that reconciliation but did not apply the same pattern — its own `if:` condition is still the bare, unqualified `github.event_name == 'workflow_dispatch'` (`M7-B08` Deliverables, `.github/workflows/ci.yml`). This is a real, live inconsistency this blueprint's own CI edit would otherwise make worse (a fourth unconditional job joining the collision) rather than better. **This blueprint's own binding resolution, a narrow, cited, reconciliation-only edit to `M7-B08`'s already-merged file — never a rewrite of that job's own logic**: `compose-topology-gate`'s `if:` gains `&& inputs.job == 'compose-topology-gate'`, the shared `on.workflow_dispatch.inputs.job` choice gains `compose-topology-gate` as a new option (joining `reference-host-gate`/`release` already there per `M6-B06`), and this blueprint's own new `m7-acceptance-gate` job (Deliverables) is added as a **fourth**, sibling, independently-triggered job checking `inputs.job == 'm7-acceptance'` — never folded into `compose-topology-gate`'s own steps, since AC1–AC3's own topology needs (the overlay compose file, §C) and orchestration (`m7-report`, §I) are genuinely this blueprint's own, distinct from `M7-B08`'s bare bring-up/health-poll/teardown job.

### §B — M7's four acceptance criteria, verbatim, and this blueprint's own precise reading of each

From `11-roadmap-milestones.md`, quoted in full (as restated in this blueprint's own task brief):

1. *"A player crosses a region border whose sides are owned by two different node processes, proxy-mediated, ZERO client-visible disconnect/loading screen, end-to-end handoff ≤2 ticks (100 ms) in a co-located topology (CLUSTER-D7/D22 budget)."*
2. *"Killing a node mid-session triggers takeover — failed node's regions resume on a survivor within raft-election-timeout+takeover window, players on unaffected regions observe zero interruption."*
3. *"A 3-node+2-proxy cluster sustains M6's 200-bot/8-region/20-TPS profile with no correctness regression, incl. M3's redstone corpus with two contraptions split across a node boundary staying bit-identical within each node's own regions (cross-node border latency allowed its documented degradation per CLUSTER-D8)."*
4. *"M6's full acceptance still passes unmodified on the same build with no `[cluster]` config — monolithic genuinely unaffected (CLUSTER-D26/D27)."*

**§B.1 — AC1, precise reading.** Two independently-checked sub-parts, both required: **AC1a (timing)** — `M7-B07`'s own already-fixed `HANDOFF_BUDGET_MS = 100` constant (that blueprint's Context §H budget decomposition, restated: typical ≈55 ms, worst-case p99 ≈110 ms — 10 ms over nominal, stated honestly by that blueprint rather than papered over) gates the elapsed time between the source node's own `send_control(Begin)` call and `ProxyRoutingTable::complete_handoff` first observing the flip, exactly as `M7-B07`'s own `play_cluster_handoff_walk.rs::player_crosses_a_live_node_boundary_within_budget_with_bounded_position_delta` case already measures and asserts — reused, never re-measured a second way. **AC1b (zero client-visible disconnect/loading screen, this blueprint's own precise, packet-level definition)** — the client's own observed inbound packet stream, for the full duration of the scripted crossing (from the first scripted step through the last), contains **zero** occurrences of any Login-state or Configuration-state packet (`LoginSuccess`, `FinishConfiguration`, or any packet `rc_protocol::ConnectionState` tags as `Login`/`Configuration`, `M1-B01`/`M1-B02`, restated — `LoginAcknowledged` is sent by the client to the server, never by the server to a client, so it can never occur in this inbound stream and plays no part in this check; the genuine inbound Login-state markers this check actually covers are `LoginDisconnect`, `Hello`, `LoginFinished`, `LoginCompression`, `CustomQuery`, and `CookieRequest`) and **zero** occurrences of a clientbound Play-state `Respawn` packet, covering both a dimension change and an ordinary post-death respawn (`Respawn` is sent for either, and a same-dimension in-world crossing emits neither) — the literal, mechanical proof that no protocol-level re-entry ever happens, distinct from (and narrower/more precise than) `M7-B07`'s own already-checked "the loopback socket never receives a disconnect" (a transport-level check; AC1b is the corresponding protocol-level one, the exact gap the task's own brief names: *"exact packet-level definitions: no Login/Respawn/config re-entry observed client-side"*). AC1a and AC1b are both proven, today, by extending `M7-B07`'s own real fixture (§D) — no compose topology is needed for either.

**§B.2 — AC2, precise reading.** Three independently-checked sub-parts, all required, each a direct turn of `M7-B05`'s own already-fixed machinery into a gated number: **AC2a (takeover-window budget)** — reusing `M7-B05` §C's own exact decomposition (restated §E.1 below) verbatim, three sub-budgets gated separately plus a diagnostic total. **AC2b (unaffected-player zero-interruption)** — every player connected through a region **not** owned by the killed node observes zero tick gaps (its own region's `--region-tick-log` shows strictly monotonic, contiguous tick numbers throughout the kill-and-recovery window) and zero connection interruption (the client's own socket, per AC1b's identical packet-level technique, never observes a disconnect or Login/Configuration re-entry) for the **entire** duration of the affected region's own recovery — the literal "players on unaffected regions observe zero interruption" reading, checked against real traffic continuing to flow, never merely against the absence of an error log line. **AC2c (state-loss bound)** — CLUSTER-D17's own bound, restated and made exact (§E.3): the resumed region's observable state, compared against a scripted, deterministic edit log, differs from the pre-kill state by **exactly** the set of edits made after the region's own last successful persisted save and before the kill — never more (a correctness bug) and never fewer (a false claim of a stronger guarantee than CLUSTER-D17 makes).

**§B.3 — AC3, precise reading.** Two independently-checked sub-parts, both required, each a distinct topology: **AC3a (the 200-bot/8-region profile)** — `M6-B01`'s `eight_region_mixed.ron` scenario, byte-for-byte reused (§F), re-run against the compose topology's two proxy listen ports (a two-endpoint fan-out extension to `M6-B01`'s own bot connector, §F.1) instead of one monolithic server address, evaluated by `M6-B06`'s own `evaluate_ac1`/`evaluate_ac2`/`evaluate_ac3` functions **completely unmodified** (M6's own AC1/AC2/AC3, restated as this blueprint's own AC3a sub-checks — "M6's profile," not a new one) — the delta from `M6-B06`'s own run is exactly and only: (i) region ownership is now split across 3 node processes instead of ticking in one process, and (ii) bots connect through 2 proxy processes instead of 1 monolithic listener; every TPS/pool/CPU-attribution number `M6-B06` already gates is asked identically of the cluster-mode run. **AC3b (the redstone-corpus cross-node split)** — two specific, already-shipped `M3-B07` contraptions (§G.1), placed so their own bounding boxes straddle one pinned cross-node region border (§G.2), replayed live against this blueprint's own compose-topology cluster build and independently against a monolithic reference build, `M3-B07`'s own `diff_traces` asserting the two traces are bit-identical (§G.3) — CLUSTER-D8's own documented cross-border latency-degradation allowance restated as the one explicitly-permitted, and in this compose topology's own co-located deployment (`M7-B08` §G: sub-millisecond bridge-network latency, comfortably inside CLUSTER-D7's ≤30 ms p99 budget) never actually exercised, tolerance (§G.4).

**§B.4 — AC4, precise reading.** Two independently-checked sub-parts, both required: **AC4a (the wiring)** — `M7-B08`'s own already-shipped `absent_cluster_config_leaves_monolithic_path_byte_identical`/`resolve_role_never_reaches_cluster_code_without_a_cluster_table` tests (compile-time/static-reference proof) **plus** this blueprint's own new **runtime-level** observation (§H.2): a real, running `rusty-clanker-server` process, started with the identical cluster-capable binary but no `[cluster]` config, is observed (via its own captured `tracing` output and its own bound-socket enumeration) to open **no** QUIC listener, spawn **no** raft/health-monitor background task, and emit **no** `tracing` event tagged with a cluster-related target — the process-level complement to `M7-B08`'s own symbol-reference-level proof. **AC4b (M6's own criteria, re-run)** — `M6-B06`'s own `M6ReportResult`/`build_report`/`evaluate_ac1`/`evaluate_ac2`/`evaluate_ac3` (that blueprint's AC1–AC3, a **different**, already-fixed set of criteria than this blueprint's own AC1–AC4 — names collide across milestones only because both roadmap entries independently number their own criteria "AC1..AC3/4"; this blueprint always qualifies which milestone's ACn it means) invoked completely unmodified, against the same binary, with no `[cluster]` config — "the wiring," per this blueprint's own task brief, is precisely this re-invocation plus AC4a's inertness proof; whether `M6`'s own real run is *currently* green is a fact about `M6`'s own milestone state (`M6-B04`'s `reference-host-gate`), not re-litigated here.

### §C — The compose topology: `M7-B08`'s base file, extended by a non-modifying overlay

`M7-B08`'s own `deploy/cluster/docker-compose.cluster-test.yml` (already committed, reused **byte-for-byte**, never edited by this blueprint — Constraints) ships three `node-*` services and one `minio` shared-storage service, no proxy service. This blueprint's own `deploy/cluster/docker-compose.m7-acceptance.yml` (new, Deliverables) is a **compose override file** (Docker Compose's own native multi-file merge mechanism, `docker compose -f docker-compose.cluster-test.yml -f docker-compose.m7-acceptance.yml`, zero new tooling dependency) adding exactly two services, `proxy-1`/`proxy-2` (`role = "proxy"`, `directory_seeds` naming `node-a`/`node-b`/`node-c`'s already-declared `[cluster].bind` addresses, `player_bind_addr` published on two distinct host ports for the harness's own bot/client connections to target), plus one additive field on each already-declared `node-*` service: a bind-mounted `--metrics-snapshot-log`/`--region-tick-log` output path onto a new named volume, `m7-results`, shared by every service and, critically, also bind-mounted to the **host** (so this blueprint's own post-run analysis, running outside any container, can read every node's own NDJSON logs directly) — the identical `--metrics-snapshot-log`/`--region-tick-log` contract items `M6-B06`/`M5-B10` already fixed, applied per-node rather than to one monolithic process. This override, being additive-only, changes nothing about `M7-B08`'s own already-tested base-file shape (`compose_file_is_valid_and_matches_declared_services`, `M7-B08` Acceptance tests, remains valid and continues to pass against the unmodified base file).

**Co-located-latency argument, restated exactly (`M7-B08` §G, reused verbatim for this blueprint's own extended topology):** every service, base or overlay, runs on the same single Docker host's own bridge network — sub-millisecond inter-container latency, comfortably inside CLUSTER-D7's ≤30 ms p99 cross-node budget — which is why AC1's 100 ms budget and AC3b's "never actually exercised" CLUSTER-D8 tolerance (§B.3/§G.4) both hold by construction in this specific topology, not merely by hope.

### §D — AC1: extending `M7-B07`'s own real handoff proof

**§D.1 — What already exists, reused unmodified.** `M7-B07`'s own `crates/server/tests/play_cluster_handoff_walk.rs::player_crosses_a_live_node_boundary_within_budget_with_bounded_position_delta` (already merged, already Tier-1-green) drives a real, hand-rolled-protocol TCP test client through a real login-through-proxy sequence (`M7-B06`'s own established pattern), scripts the identical 64-step/+0.5-per-tick westward walk `M4-B08`'s own harness first established, across a boundary owned by two real `NetworkTransport`/`RcExecutor`/`NodeAcceptor` triples, and already asserts (1) the position-continuity property (at most one `None` tick, consecutive `Some` deltas exactly `0.5`) and (2) the `send_control(Begin)`-to-routing-flip timing delta is `< HANDOFF_BUDGET_MS` (100). This is AC1a's own proof, in full — this blueprint does not re-measure it.

**§D.2 — This blueprint's own addition: AC1b's packet-level assertion, a new test in a new file, never modifying `M7-B07`'s own already-merged test.** `crates/server/tests/ac1_zero_reentry.rs` (new, Deliverables) reuses `play_cluster_handoff_walk.rs`'s own fixture-construction helpers (the two-node/one-proxy setup, the scripted walk driver — `M7-B07`'s own test-support module, extended additively per the exact discipline that blueprint's own Constraints (d)/(e) established: a new test file consuming an existing fixture is not a modification of the fixture's owning file) and instruments the test client's own packet-receive loop (already present — the client must decode every inbound packet to drive its own state machine) to record every received packet's `rc_protocol::ConnectionState`/packet-name tag into a `Vec<ObservedPacket>`. After the scripted walk completes, `assert_zero_reentry(&observed, crossing_window_start_tick, crossing_window_end_tick)` (Deliverables, `rc_test_harness::cluster_continuity`) asserts zero `Login`/`Configuration`-tagged packets and zero clientbound `Respawn` packets appear within `[crossing_window_start_tick, crossing_window_end_tick]` — a pure function over the recorded packet log, independently unit-testable against synthetic logs (Acceptance tests, including this blueprint's own AC1 self-test, §K).

**§D.3 — The real-compose leg, honestly gated.** `xtask m7-report --criterion 1 --compose` additionally attempts the identical AC1a/AC1b measurement against the compose topology's own `proxy-1` listen port instead of an in-process fixture — this leg needs `main.rs`'s real `ClusterProxy`/`ClusterNode` role-wiring (§A gap 2) and therefore fails closed with `ClusterIntegrationPending` (§I) until that lands; its own eventual real run reuses the identical `assert_zero_reentry`/timing-delta functions §D.2 already proves correct against synthetic and in-process-real data, so no new measurement logic is needed once the gap closes — only a live topology to point it at.

### §E — AC2: node-kill takeover measurement

**§E.1 — The budget decomposition, reused exactly from `M7-B05` §C, turned into three gated numbers plus a diagnostic total.**

```rust
/// `M7-B05` §C Case 1's own dominant term (a non-leader node's failure) — this
/// blueprint's own acceptance scenario deliberately kills a NON-leader node
/// specifically so the measured budget exercises the LARGER of `M7-B05`'s two
/// cases, subsuming the smaller leader-failure case (750-1500ms) rather than
/// separately re-proving it — restated, not re-derived.
pub const AC2_DETECTION_BUDGET_MS: u64 = 3_000; // HealthMonitorConfig::failure_threshold, M7-B05 §C/M7-B02 §H
/// One raft commit (CLUSTER-D7-reused-as-proxy, M7-B05 §C/§N item 2, ~30ms) plus
/// one full `REGION_ASSIGNMENT_POLL_INTERVAL` cycle (M7-B08, 200ms — the survivor's
/// own region-assignment watcher's worst-case latency to notice the new directory
/// entry and begin spawning), plus 20ms headroom for real docker-bridge jitter.
pub const AC2_REASSIGNMENT_BUDGET_MS: u64 = 250;
/// This blueprint's own seed default for the acceptance scenario's own small
/// (single-cell) region size — calibration-pending, the identical "concrete
/// number now, revisit once real measurements exist" status every other
/// numeric threshold in this corpus carries (ARCH-D6/CLUSTER-D2's own framing).
pub const AC2_RESUME_IO_BUDGET_MS: u64 = 2_000;
/// Diagnostic only — the sum, reported but not independently gated beyond its
/// three components already being gated individually.
pub const AC2_TOTAL_BUDGET_MS: u64 = AC2_DETECTION_BUDGET_MS + AC2_REASSIGNMENT_BUDGET_MS + AC2_RESUME_IO_BUDGET_MS;
```

**§E.2 — Measurement mechanics.** The harness (`m7_report::run_ac2`, Deliverables) instruments four timestamps against the compose topology's own `m7-results`-mounted logs and a `TakeoverEvent` NDJSON stream (this blueprint's own additive `--takeover-event-log <path>` contract item, extending `M7-B05`'s own `on_event: Arc<dyn Fn(TakeoverEvent)>` hook — restated as binding on whichever future composition-root blueprint wires `TakeoverOrchestrator::spawn` for real, the identical "pin the contract shape now, implement the server side later" discipline `M6-B01` §B already established): `t_kill` (the harness's own `ManagedServer::kill_now` call, §J), `t_failed` (the first `TakeoverEvent::FailureObserved` entry in the log), `t_reassigned` (the first `TakeoverEvent::RegionReassigned` entry naming the affected region), `t_resumed` (the survivor's own `--region-tick-log` first post-kill tick entry for that region). `detection_ms = t_failed - t_kill`, `reassignment_ms = t_reassigned - t_failed`, `resume_io_ms = t_resumed - t_reassigned` — each independently gated against its own budget above.

**§E.3 — AC2c, the exact state-loss-bound assertion.** The scripted pre-kill workload (`m7_report`'s own bot script, reusing `M6-B01`'s `BuildBreakChurn` hotness profile's place-then-break cadence, §F) deterministically edits one known block position every `EDIT_PERIOD_TICKS = 20` ticks (one real second) throughout the run, each edit's tick number recorded by the harness itself (not read back from the server — the harness is the ground truth for "what was attempted and when"). `save_interval_ticks` is pinned to `M7-B08`'s own `default_cluster_save_interval_ticks() = 600` (CLUSTER-D17's ≤30s recommendation, restated, unmodified). After the kill and the survivor's own resume, the harness reads the resumed region's own final block states (via a bot connected through the surviving proxy, an ordinary block-read) and computes `expected_visible_edits = edits whose tick <= last_save_tick_before_kill` (derivable from the survivor's own `RegionManifest`-guided resume, `M7-B04` §G.4, cited only for this reasoning, never a direct dependency) — `assert_eq!(observed_final_state, replay_of(expected_visible_edits))`, **never** a fuzzy "at most K blocks differ" check: CLUSTER-D17's own bound is exact (everything up to the last save survives, everything after is lost), so this blueprint's own assertion is exact too, restated as the honest, precise reading rather than a looser approximation.

### §F — AC3a: the 200-bot/8-region profile, fanned out across two proxies

**§F.1 — The one additive field.** `MultiRegionScenarioConfig` (`M6-B01` §H, restated) gains exactly one new field:

```rust
// crates/testing/paritybot/src/loadtest/runner.rs (modify — additive)
pub struct MultiRegionScenarioConfig {
    // ...every existing field from M6-B01/M6-B06 unchanged (scenario, server_host,
    // server_port, out_dir, resource_limits, client_view_distance)...
    /// New (M7-B09). When non-empty, every planned bot connects to one of these
    /// `(host, port)` pairs instead of `server_host`/`server_port`, chosen
    /// round-robin by the bot's own planned index (`index % proxy_fanout.len()`)
    /// — a deterministic, reproducible assignment, never random. When empty (the
    /// default, `Vec::new()`), behavior is BYTE-IDENTICAL to before this field
    /// existed: every bot connects to `server_host`/`server_port` as `M6-B01`/
    /// `M6-B06` already established — this is a strictly additive, backward-
    /// compatible extension, never a breaking change to either blueprint's own
    /// already-shipped call sites.
    pub proxy_fanout: Vec<(String, u16)>,
}
```

`run_multi_region_scenario`'s own per-bot connect call (`M6-B01` §H) resolves its target via one new, tiny, pure helper, `resolve_connect_target(config: &MultiRegionScenarioConfig, bot_index: u32) -> (&str, u16)` (Deliverables) — `proxy_fanout[bot_index as usize % proxy_fanout.len()]` when non-empty, else `(server_host, server_port)` — independently unit-tested against both branches.

**§F.2 — The scenario itself, unmodified.** `eight_region_mixed.ron` (`M6-B01`, already shipped, already `M6-B06`'s own authoritative fixture) is reused byte-for-byte — this blueprint authors **zero** new full-scale scenario file, mirroring `M6-B06` §C.1's own identical "one maintained artifact" rationale. `m7_report::run_ac3a` calls `run_multi_region_scenario` with `proxy_fanout: vec![(proxy_1_host, proxy_1_port), (proxy_2_host, proxy_2_port)]` and every other field identical to `M6-B06`'s own already-fixed invocation (`client_view_distance: 10`, the region-layout/fault-injection-schedule/metrics-snapshot-log contract items, all reused).

**§F.3 — Evaluation, unmodified.** `m7_report::run_ac3a`'s own aggregation calls `M6-B06`'s `evaluate_ac1`/`evaluate_ac2`/`evaluate_ac3` (that blueprint's own `xtask::m6_report` functions, reused as a direct dependency of this blueprint's `xtask` crate — both live in the same `xtask` binary target, an ordinary intra-crate function call, no new dependency edge) against the compose topology's own per-node `--metrics-snapshot-log` outputs, merged across all three node processes into one `entries: Vec<MetricsSnapshotEntry>` stream before being handed to those functions unmodified — the merge is this blueprint's own small, pure, additive function (`merge_per_node_snapshot_logs`, Deliverables), never a change to `M6-B06`'s own evaluators, which remain agnostic to how many processes produced their input.

### §G — AC3b: the redstone-corpus cross-node split

**§G.1 — The two named contraptions, pinned.** `redstone/pulse/torch_inverter_basic` (`M3-B07` corpus entry #1, PulseGenerator, a single self-contained torch/wire inverter — the simplest possible "one small circuit whose own bounding box straddles a border" case) and `redstone/update_order/two_torch_and_gate` (`M3-B07` corpus entry #4, UpdateOrderProbe, two independent torch inputs converging on one wire — chosen specifically because its own two independent inputs let this blueprint place them in **two different** node-owned regions simultaneously, exercising `NEIGHBOR_CHANGED_ORDER`'s fan-out ordering guarantee under two independent, concurrent cross-node border-halo reads rather than only one). Both are among `M3-B07`'s own five **(full)**, already-shipped, already-committed `.ron` fixtures with real, cross-checked `state_id`/`vanilla_state` pairings — this blueprint authors **zero** new contraption content, reusing both files byte-for-byte.

**§G.2 — The split, pinned exactly.** `M0-B06`'s own `GRID_CELL_BLOCKS = 256` (restated, `M6-B01` §D's identical local restatement) places a region-cell boundary at world `x = 0` (between cell `(-1, 0)` and cell `(0, 0)` — floor-division convention, restated: world `x = -1` belongs to cell `(-1, 0)`, world `x = 0` belongs to cell `(0, 0)`). This blueprint's own new fixture, `crates/testing/gametest/corpus/redstone/m7_cluster_split_layout.ron` (new, Deliverables), declares exactly two `RegionCellGroup`s (`M6-B01` §C schema, reused unmodified) — `label: "west-cell", cells: [(-1, 0)]` and `label: "east-cell", cells: [(0, 0)]` — assigned in the compose topology's own `--region-layout` to `node-a` and `node-b` respectively (a fixed, documented assignment this blueprint's own orchestration enforces by construction, never left to whichever node happens to win an ordinary rebalance). `torch_inverter_basic`'s own world origin is pinned at `(0, 64, 8)` — its bounding box (relative `x ∈ [-2, +2]` per its own committed `.ron` file, well under one chunk per `M3-B07`'s own budget) therefore spans world `x ∈ [-2, +2]`, genuinely straddling the boundary: its torch and part of its wire sit in `west-cell`/`node-a`, its output end sits in `east-cell`/`node-b`. `two_torch_and_gate`'s own world origin is pinned at `(0, 64, 40)` (a distinct `z` offset so the two contraptions' bounding boxes never overlap) — its own committed layout places one independent torch input at relative `x = -3` (world `x = -3`, `west-cell`/`node-a`) and the other at relative `x = +3` (world `x = +3`, `east-cell`/`node-b`), with the converging wire and AND-gate output at relative `x = 0` (world `x = 0`, `east-cell`/`node-b`, per the floor-division convention above) — a fixed, documented, non-ambiguous placement.

**§G.3 — Capture and diff, reusing `M3-B07`'s own machinery unmodified.** `crates/testing/gametest/src/cluster_replay.rs` (new, Deliverables) adds exactly one new function, `capture_trace_from_target(spec: &ContraptionSpec, target: SocketAddr, world_origin: (i32, i32, i32)) -> Result<RedstoneTrace, CaptureError>`, built on **the identical bot-connects/places-blocks/records-packets technique `M3-B07`'s own oracle-capture pipeline already uses** (that blueprint's own capture mechanism is protocol-driven, not vanilla-specific by construction — our own engine speaks the identical wire protocol by design, `NET-D1`) — this is a genuinely small, additive extension (a new target-address parameter where `M3-B07`'s own pipeline always spawned and pointed at a local vanilla subprocess) rather than new capture logic, and it imports only `M3-B07`'s already-public `RedstoneTrace`/`ContraptionSpec`/`BlockObservation` types, never modifying `M3-B07`'s own `fetch_corpus.rs`. `m7_report::run_ac3b` calls this function **twice** per contraption: once against the compose topology's own `proxy-1` (or whichever proxy currently routes to the region containing the contraption's own convergence point — resolved once via a directory read, per `M6-B06`'s own established "read the real, dynamically-allocated ids" discipline), producing `trace_cluster`; once against a separately-started, single-node, non-cluster-configured reference build of the identical `rusty-clanker-server` binary (an ordinary `ManagedServer` instance, `M3-B08`'s own established pattern, no compose needed for this half), producing `trace_monolithic`. `M3-B07`'s own `diff_traces(&trace_cluster, &trace_monolithic)` (reused, unmodified) is the bit-identity assertion — `Ok(())` (identical, tick-for-tick, position-for-position, including every `analog` field) is AC3b's own pass condition for that contraption; any `Err` names the first differing tick/position, exactly as `M3-B07`'s own diff report already does.

**§G.4 — CLUSTER-D8's own documented allowance, restated as an explicit, honestly-unexercised tolerance.** CLUSTER-D8 permits ARCH-D11's own border-halo traffic to "degrade gracefully to N-tick propagation lag" under a cross-node topology violating CLUSTER-D7's ≤30 ms budget — this blueprint's own `diff_traces` call above is, by default, a **strict, zero-lag, tick-for-tick** comparison, because §C's own co-located compose topology never actually violates that budget (sub-millisecond bridge-network latency). This blueprint's own assertion therefore correctly, honestly requires bit-identity at every tick in the topology it actually runs against — it does **not** silently build in a lag allowance that would mask a real regression on the one topology this blueprint controls. The allowance is restated here, precisely, as documentation for a **future** cross-AZ/degraded-topology test this blueprint does not itself build (an explicit, named Open Issue, not a gap this blueprint's own assertion papers over): `diff_traces`'s own caller (`run_ac3b`) accepts an optional `max_lag_ticks: u32` parameter (default `0`, this blueprint's own value) precisely so a future blueprint testing a genuinely non-co-located topology can widen it without touching `M3-B07`'s own `diff_traces` signature — a lag-tolerant comparison shifts one trace's own tick indices by up to `max_lag_ticks` before comparing, implemented as this blueprint's own small wrapper (`diff_traces_with_lag_tolerance`, Deliverables), never a change to `M3-B07`'s own strict function.

### §H — AC4: monolithic no-regression

**§H.1 — AC4b, the re-invocation.** `m7_report::run_ac4b` shells out to `cargo run -p xtask -- m6-report --scenario <path> --out-dir <dir> --server-bin <the same cluster-capable binary> --reference-tier <tier>` (`M6-B06`'s own already-fixed CLI, unmodified) with the binary's own config file carrying no `[cluster]` table — `M6ReportResult`'s own `TierResult` is embedded, unmodified, as this blueprint's own `M7CompletionReport.ac4.m6_regression` field (§I). No new evaluation logic exists here; this is purely orchestration.

**§H.2 — AC4a, this blueprint's own new runtime-level inertness check.** `m7_report::run_ac4a` starts one `ManagedServer` (`M3-B08`'s own established process wrapper) with `capture_stdout: true` (already-existing field, reused) and no `[cluster]` config, waits for it to reach ordinary serving readiness (`M6-B07`'s own established readiness signal, unmodified), then asserts, over a bounded observation window: (1) `ManagedServer`'s own captured stdout/stderr contains **zero** lines whose `tracing` target string contains `"cluster"`, `"rc_cluster"`, `"rc_transport_net"`, or `"rc_proxy"` (a simple, deliberately conservative substring scan — mirroring `M6-B06` §F's own identical "a deliberately simple, conservative substring check... a false positive is judged acceptably unlikely" discipline, restated for a different marker set); (2) a TCP connect attempt against every port `M7-B08`'s own `NetworkTransportConfig`/`ProxyConfig` would have bound had cluster mode been active (`config.bind`/`proxy_quic_bind_addr`'s own default port numbers, per `M7-B08`'s port/firewall matrix, restated) is refused (`ConnectionRefused`, never a live listener) — proving no QUIC endpoint was opened. Both checks are pure, synthetic-data-testable functions (`scan_for_cluster_targets(log_lines: &[String]) -> Option<String>`, `probe_ports_are_closed(addrs: &[SocketAddr]) -> Vec<SocketAddr>`, Deliverables) exercised directly by this blueprint's own AC4 self-test (§K) without needing a real process at all.

### §I — The M7 completion report and CI-tier placement

```rust
// xtask/src/m7_report.rs — public API surface (Deliverables gives the full signature list)

#[derive(Debug, thiserror::Error)]
pub enum M7ReportError {
    #[error(
        "this criterion's real leg needs a live, multi-node compose topology, which \
         needs M7-B08's own still-open Context §A items 1 (a concrete, real-network \
         RaftNetworkFactory/JoinClient) and/or 3 (main.rs's ClusterProxy/ClusterNode \
         role wiring) to land first. This is a known, tracked dependency gap (see \
         M7-B09 Context §A), not a bug in this harness. Run with --criterion 4 (no \
         --compose) or --criterion 1 (no --compose) to exercise the legs that are \
         real and green today."
    )]
    ClusterIntegrationPending,
    // ... other variants (docker unavailable, build/spawn failure, log-parse I/O
    // error) added by the implementer as ordinary error handling; this variant's
    // exact message text is the one load-bearing, tested string.
}

#[derive(serde::Serialize)]
pub struct M7CompletionReport {
    pub ac1: Ac1Report,
    pub ac2: Ac2Report,
    pub ac3: Ac3Report,
    pub ac4: Ac4Report,
    /// `Status::Pass` iff every one of the four sections' own `status` is `Pass` —
    /// mirrors `TierResult::finalize`'s "fail on any" rule at the report's own
    /// top level (this report wraps four `TierResult`-shaped sections, not one).
    pub overall: xtask::tier_result::Status,
}
```

Each of `Ac1Report`/`Ac2Report`/`Ac3Report`/`Ac4Report` (Deliverables gives full field lists) wraps an ordinary `xtask::tier_result::TierResult` (`M0-B08`, reused unmodified) via `#[serde(flatten)]` — the identical `M<n>ReportResult` template `M3-B08` first established and every later milestone report (`M4ReportResult`.."M6ReportResult") already followed, restated here for the fourth time, never reinvented. `cargo run -p xtask -- m7-report --criterion <1|2|3|4|all>` (Deliverables) writes `target/verify/m7-acceptance.json` (TEST-D40) always — a partial report (one section only) when a single criterion is requested, or the full `M7CompletionReport` when `all` is requested.

**CI-tier placement, restated per criterion (TEST-D34/D37):**

| Leg | Tier | Needs docker | Blocked on §A gap |
|---|---|---|---|
| AC1a/AC1b, real fixture (§D.1/§D.2) | **Tier 1** (already `M7-B07`'s own case, plus this blueprint's own new `ac1_zero_reentry.rs`) | No | No |
| AC1, real-compose leg (§D.3) | Tier 3 (`m7-acceptance-gate`, `workflow_dispatch`-only) | Yes | Yes (gap 2) |
| AC2, real kill (§E) | Tier 3 (`m7-acceptance-gate`) | Yes | Yes (gaps 1+2) |
| AC3a, 200-bot/2-proxy run (§F) | Tier 3 (`m7-acceptance-gate`) | Yes | Yes (gaps 1+2) |
| AC3b, redstone-split live capture (§G) | Tier 3 (`m7-acceptance-gate`) | Yes (cluster half only — the monolithic-reference half needs only one ordinary `ManagedServer`, no compose) | Yes for the cluster half; No for the monolithic-reference half |
| AC4a, runtime inertness (§H.2) | **Tier 1** (a single ordinary subprocess, `M3-B08`'s own established pattern) | No | No |
| AC4b, M6 re-invocation (§H.1) | Tier 3 (`M6`'s own `reference-host-gate`, unmodified — this blueprint only re-invokes it) | No (compose) but yes (real multi-minute run, reference hardware) | No (an `M6`-owned gate, not this blueprint's own) |
| All four self-tests (§K) | **Tier 1** | No | No |

`m7-acceptance-gate` (Deliverables, `.github/workflows/ci.yml`) is `workflow_dispatch`-only, gated by `inputs.job == 'm7-acceptance'` (§A.1's Finding), brings the topology up via `docker compose -f docker-compose.cluster-test.yml -f docker-compose.m7-acceptance.yml`, runs `cargo run -p xtask -- m7-report --criterion all --compose --server-bin <built-binary> --reference-tier <tier>`, uploads `target/verify/m7-acceptance.json` as a workflow artifact regardless of outcome, and tears the topology down — mirroring `M7-B08`'s `compose-topology-gate` job's own shape exactly, restated once rather than re-derived. Its first meaningfully-green run for AC1's real-compose leg, AC2, and AC3 is a **later** milestone-acceptance signal (§A), never a condition of this blueprint's own Tier-1 Done state — this blueprint's own Done state requires only that the job is real, correctly wired, and fails closed with the exact, actionable message today, the identical "spec now, first green later" split every prior gated-job blueprint in this lineage already established.

### §J — `ManagedServer::kill_now`: the real-`SIGKILL` primitive AC2 needs

```rust
// crates/testing/test-harness/src/process.rs (modify — additive; every existing
// field/method from M1-B06/M2-B08/M3-B08/M5-B10/M6-B06 unchanged)
impl ManagedServer {
    /// Sends an immediate, ungraceful termination signal to the wrapped child
    /// process — `SIGKILL` on Unix, `TerminateProcess` on Windows, both via
    /// `std::process::Child::kill()`'s own already-safe, already-cross-platform
    /// stdlib API (no new dependency). Distinct from this type's own existing
    /// `Drop` impl (a graceful-then-forceful teardown sequence, `M1-B06`) — this
    /// method is for a test that specifically wants to simulate an unplanned
    /// crash mid-session, never for ordinary test cleanup, which continues to
    /// use `Drop` unchanged. Idempotent: calling this on an already-exited child
    /// returns `Ok(())`, never panics or errors.
    pub fn kill_now(&mut self) -> std::io::Result<()>;
    /// The OS process id of the wrapped child, for a test's own log-correlation
    /// needs (e.g. confirming which of three node processes was actually killed).
    pub fn pid(&self) -> u32;
}
```

### Claims to verify (TEST-D57)

- The Login-state packet LoginSuccess is sent to a client only during an actual protocol-level login, never during an ordinary in-world region/border crossing (§B.1, §D.2).
- The Login-state packet LoginAcknowledged is sent by the client to the server only, never by the server to a client, so it can never appear in the client's own observed inbound packet stream; the genuine inbound Login-state markers to check for during an ordinary in-world region/border crossing are LoginDisconnect, Hello, LoginFinished, LoginCompression, CustomQuery, and CookieRequest (§B.1, §D.2).
- The Configuration-state packet FinishConfiguration is sent to a client only during an actual configuration-phase re-entry, never during an ordinary in-world region/border crossing (§B.1, §D.2).
- The clientbound Play-state Respawn packet is sent to a client during an actual dimension change and also during an ordinary post-death respawn that need not change dimension, but a same-dimension in-world region/border crossing never emits it, since the underlying teleport path returns early, before constructing Respawn, whenever the target and current dimensions match (§B.1, §D.2).

## Deliverables

### `crates/testing/test-harness/src/process.rs` (modify — additive, Context §J)

Exactly `ManagedServer::kill_now`/`pid` per Context §J. Every existing field, method, and the existing `Drop` impl: byte-unmodified.

### `crates/testing/test-harness/src/cluster_continuity.rs` (new)

```rust
//! Packet-level continuity assertions for AC1b/AC2b (Context §D.2/§E.2) — a pure,
//! dependency-free mirror module, the same "local mirror, never a Cargo edge into
//! a shipped crate" discipline `metrics_snapshot_log.rs` (M6-B06) already
//! established, applied here to `rc-protocol`'s connection-state vocabulary.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedPhase { Login, Configuration, Play, PlayRespawn }

#[derive(Debug, Clone)]
pub struct ObservedPacket {
    pub tick: u64,
    pub phase: ObservedPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReentryViolation {
    pub tick: u64,
    pub phase: ObservedPhase,
}

/// Pure: the first `ObservedPacket` within `[window_start_tick, window_end_tick]`
/// (inclusive) whose `phase` is `Login`, `Configuration`, or `PlayRespawn` — `None`
/// means AC1b/AC2b's own zero-reentry property holds for this window. Checks in
/// packet order, returns the FIRST violation only (sufficient to fail the case;
/// the caller's own detail string names it precisely).
pub fn find_reentry_violation(
    observed: &[ObservedPacket],
    window_start_tick: u64,
    window_end_tick: u64,
) -> Option<ReentryViolation>;

/// Convenience boolean wrapper.
pub fn assert_zero_reentry(observed: &[ObservedPacket], window_start_tick: u64, window_end_tick: u64) -> bool {
    find_reentry_violation(observed, window_start_tick, window_end_tick).is_none()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum TakeoverEventMirror {
    FailureObserved { dead: String, affected_regions: Vec<u64> },
    RegionReassigned { region: u64, from: String, to: String, new_epoch: u64 },
    ReassignmentDeferred { region: u64, dead: String, reason: String },
    NoLiveNodesAvailable { dead: String, unassigned_regions: Vec<u64> },
    RegionResumed { region: u64, node: String, epoch: u64 },
    RegionEvicted { region: u64, node: String },
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TakeoverEventLogEntry {
    pub captured_at_unix_ms: u64,
    pub event: TakeoverEventMirror,
}

/// Parses `path` as newline-delimited JSON `TakeoverEventLogEntry` records —
/// identical "skip a malformed line, never abort the whole parse" tolerance
/// every prior NDJSON parser in this lineage established (M5-B10, M6-B06).
pub fn parse_takeover_event_log(path: &std::path::Path) -> std::io::Result<Vec<TakeoverEventLogEntry>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TakeoverBudgetResult {
    pub detection_ms: i64,
    pub reassignment_ms: i64,
    pub resume_io_ms: i64,
    pub total_ms: i64,
    pub detection_within_budget: bool,
    pub reassignment_within_budget: bool,
    pub resume_io_within_budget: bool,
    pub passed: bool,
}

/// Pure: Context §E.2's own four-timestamp arithmetic, gated against §E.1's three
/// budget constants (passed explicitly, never hardcoded here, so this function is
/// independently unit-testable against arbitrary budgets — the mechanism this
/// blueprint's own `slow_takeover_fails_ac2` self-test, §K, exercises directly).
pub fn evaluate_takeover_budget(
    t_kill_unix_ms: u64,
    t_failed_unix_ms: u64,
    t_reassigned_unix_ms: u64,
    t_resumed_unix_ms: u64,
    detection_budget_ms: u64,
    reassignment_budget_ms: u64,
    resume_io_budget_ms: u64,
) -> TakeoverBudgetResult;
```

### `crates/testing/test-harness/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod cluster_continuity;
```

### `crates/testing/paritybot/src/loadtest/runner.rs` (modify — additive, Context §F.1)

Exactly `MultiRegionScenarioConfig::proxy_fanout` and `resolve_connect_target` per Context §F.1. Every existing field/function unchanged.

### `crates/testing/gametest/src/cluster_replay.rs` (new)

```rust
//! AC3b's live-capture-from-a-real-target extension (Context §G.3) — reuses
//! M3-B07's already-public trace/spec/diff types unmodified; adds exactly one new
//! capture entry point and one lag-tolerant diff wrapper. Never modifies
//! `crates/testing/gametest/src/fetch_corpus.rs` or any other M3-B07 file.

use rc_gametest::{BlockObservation, ContraptionSpec, RedstoneTrace, TickSnapshot};

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("failed to connect to capture target {0}: {1}")]
    Connect(std::net::SocketAddr, std::io::Error),
    #[error("protocol error while capturing from {0}: {1}")]
    Protocol(std::net::SocketAddr, String),
    #[error("capture timed out after {0:?} waiting for tick {1} to settle")]
    Timeout(std::time::Duration, u64),
}

/// Context §G.3 — the identical bot-connects/places-blocks/records-packets
/// technique M3-B07's own oracle-capture pipeline already implements, retargeted
/// at an arbitrary `target` (our own build, cluster or monolithic) instead of a
/// locally-spawned vanilla subprocess. Places every `spec.blocks` entry at
/// `world_origin + relative_pos`, drives every `spec.actions` entry on schedule,
/// and records one `TickSnapshot` per tick up to `spec.max_ticks`, identically
/// shaped to M3-B07's own `RedstoneTrace.ticks` — the returned value is a real,
/// well-formed `RedstoneTrace` (`format_version: TRACE_FORMAT_VERSION`,
/// `source_jar_sha1: String::new()` — not applicable to a non-vanilla capture,
/// this blueprint's own extension of the field's meaning, documented here rather
/// than left ambiguous) ready for `diff_traces`/`diff_traces_with_lag_tolerance`.
pub fn capture_trace_from_target(
    spec: &ContraptionSpec,
    target: std::net::SocketAddr,
    world_origin: (i32, i32, i32),
) -> Result<RedstoneTrace, CaptureError>;

/// Context §G.4 — CLUSTER-D8's documented allowance, exercised only when
/// `max_lag_ticks > 0` (this blueprint's own default is 0, Context §G.4). Shifts
/// `actual`'s own tick indices by every offset in `0..=max_lag_ticks` and accepts
/// the comparison if ANY shift makes `rc_gametest::diff_traces` report equality —
/// never silently widens the per-tick comparison itself, only the alignment.
/// Never calls `rc_gametest::diff_traces` with a modified `expected`/`actual`
/// beyond the shift itself — the underlying per-position/per-analog comparison is
/// M3-B07's own unmodified function, reused, not re-implemented.
pub fn diff_traces_with_lag_tolerance(
    expected: &RedstoneTrace,
    actual: &RedstoneTrace,
    max_lag_ticks: u32,
) -> Result<(), rc_gametest::TraceDiffError>;
```

### `crates/testing/gametest/corpus/redstone/m7_cluster_split_layout.ron` (new — worked example, Context §G.2)

Exactly the two-`RegionCellGroup` layout (`west-cell: [(-1,0)]` -> `node-a`, `east-cell: [(0,0)]` -> `node-b`) per Context §G.2, using `M6-B01`'s own `RegionLayoutSpec` schema (reused, no new type).

### `xtask/src/m7_report.rs` (new)

```rust
use xtask::tier_result::{Status, TierResult};
use rc_test_harness::cluster_continuity::{
    ObservedPacket, TakeoverEventLogEntry, TakeoverBudgetResult,
    evaluate_takeover_budget, find_reentry_violation, parse_takeover_event_log,
};

pub const AC1_TIMING_BUDGET_MS: u64 = 100; // M7-B07's HANDOFF_BUDGET_MS, reused
pub const AC2_DETECTION_BUDGET_MS: u64 = 3_000;
pub const AC2_REASSIGNMENT_BUDGET_MS: u64 = 250;
pub const AC2_RESUME_IO_BUDGET_MS: u64 = 2_000;
pub const AC2_TOTAL_BUDGET_MS: u64 = AC2_DETECTION_BUDGET_MS + AC2_REASSIGNMENT_BUDGET_MS + AC2_RESUME_IO_BUDGET_MS;
pub const AC2_EDIT_PERIOD_TICKS: u64 = 20;
pub const AC2_SAVE_INTERVAL_TICKS: u64 = 600; // M7-B08's default_cluster_save_interval_ticks()

pub const OUT_PATH: &str = "target/verify/m7-acceptance.json";

#[derive(Debug, thiserror::Error)]
pub enum M7ReportError {
    #[error(
        "this criterion's real leg needs a live, multi-node compose topology, which \
         needs M7-B08's own still-open Context §A items 1 and/or 3 to land first. \
         This is a known, tracked dependency gap (see M7-B09 Context §A), not a bug \
         in this harness. Run with --criterion 4 (no --compose) or --criterion 1 \
         (no --compose) to exercise the legs that are real and green today."
    )]
    ClusterIntegrationPending,
    // ... other variants (docker unavailable, build/spawn failure, parse I/O error)
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct Ac1Report { #[serde(flatten)] pub automated: TierResult, pub timing_ms: Option<i64>, pub reentry_violation: Option<String> }
#[derive(serde::Serialize, Debug, Clone)]
pub struct Ac2Report { #[serde(flatten)] pub automated: TierResult, pub budget: Option<TakeoverBudgetResultSerde>, pub expected_visible_edits: usize, pub observed_matched_edits: usize }
#[derive(serde::Serialize, Debug, Clone)]
pub struct Ac3Report { #[serde(flatten)] pub automated: TierResult, pub ac3a_bot_count: u32, pub ac3a_region_count: usize, pub ac3b_contraptions: Vec<String> }
#[derive(serde::Serialize, Debug, Clone)]
pub struct Ac4Report { #[serde(flatten)] pub automated: TierResult, pub m6_regression_status: Status }

#[derive(serde::Serialize, Debug, Clone, Copy)]
pub struct TakeoverBudgetResultSerde { pub detection_ms: i64, pub reassignment_ms: i64, pub resume_io_ms: i64, pub total_ms: i64 }

#[derive(serde::Serialize, Debug, Clone)]
pub struct M7CompletionReport {
    pub ac1: Ac1Report,
    pub ac2: Ac2Report,
    pub ac3: Ac3Report,
    pub ac4: Ac4Report,
    pub overall: Status,
}

/// Context §H.2 — pure, synthetic-data-testable. A deliberately conservative
/// substring scan (mirrors M6-B06's `calibration_values_landed`, Context §F of
/// that blueprint, for the identical "acceptable false-positive-unlikely,
/// never a false negative that hides a real cluster thread" reasoning).
pub fn scan_for_cluster_targets(log_lines: &[String]) -> Option<String>;

/// Context §H.2 — pure over an injected connect-probe function (dependency-
/// injected so this is unit-testable without a real socket, and so the four
/// mandatory self-tests, Context §K, can supply a fake).
pub fn probe_ports_are_closed(
    addrs: &[std::net::SocketAddr],
    connect: impl Fn(std::net::SocketAddr) -> std::io::Result<()>,
) -> Vec<std::net::SocketAddr>;

/// Context §D.2 — the case-level wrapper `evaluate_ac1`/etc. below call.
pub fn evaluate_ac1(
    timing_ms: i64,
    observed: &[ObservedPacket],
    window_start_tick: u64,
    window_end_tick: u64,
) -> Ac1Report;

/// Context §E.2/§E.3.
pub fn evaluate_ac2(
    events: &[TakeoverEventLogEntry],
    t_kill_unix_ms: u64,
    expected_visible_edits: usize,
    observed_matched_edits: usize,
) -> Ac2Report;

/// Context §F.3/§G.3 — `m6_ac1`/`m6_ac2`/`m6_ac3` are `M6-B06`'s own
/// `Ac1Outcome`/`Ac2Outcome`/`Ac3Outcome` values, passed straight through,
/// unmodified, from a call to that blueprint's own `evaluate_ac1`/`evaluate_ac2`/
/// `evaluate_ac3`. `redstone_diffs` is one `Result<(), rc_gametest::TraceDiffError>`
/// per named contraption (Context §G.1's two entries).
pub fn evaluate_ac3(
    m6_ac1_passed: bool,
    m6_ac2_passed: bool,
    m6_ac3_passed: bool,
    bot_count: u32,
    region_count: usize,
    redstone_diffs: &[(String, Result<(), String>)],
) -> Ac3Report;

/// Context §H.1/§H.2.
pub fn evaluate_ac4(
    cluster_target_found: Option<String>,
    open_ports: &[std::net::SocketAddr],
    m6_regression_status: Status,
) -> Ac4Report;

/// Pure aggregation — builds `M7CompletionReport.overall` per the "fail on any
/// section" rule (Goal & Done). Every mandatory self-test (Context §K) ultimately
/// asserts on one of these four `evaluate_ac*` functions' own output, never
/// merely on a lower-layer helper in isolation — the same discipline `M3-B08`/
/// `M6-B06` already established for their own `build_report`.
pub fn build_report(ac1: Ac1Report, ac2: Ac2Report, ac3: Ac3Report, ac4: Ac4Report) -> M7CompletionReport;

pub struct M7ReportArgs {
    pub criterion: Criterion,           // One(1|2|3|4) | All
    pub server_bin: Option<std::path::PathBuf>,
    pub compose: bool,                  // requires server_bin; requires a live docker daemon
    pub reference_tier: Option<String>, // required whenever the real M6 re-invocation (AC4b) runs
    pub out_dir: std::path::PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Criterion { One, Two, Three, Four, All }

/// The entry point `xtask m7-report` dispatches to. Routes per `args.criterion`;
/// any leg requiring `args.compose` when it is `false`, or requiring
/// M7-B08's still-open gap (Context §A) when `args.compose` is `true`, returns
/// `Err(M7ReportError::ClusterIntegrationPending)` for exactly that leg's own
/// section, leaving every OTHER requested section's own real, achievable-today
/// measurement to run and report normally (AC1's in-process leg, AC4's full
/// leg) — a partial failure never blocks an unrelated, independently-achievable
/// section, mirroring TierResult's own per-case independence.
pub fn run(args: &M7ReportArgs) -> Result<M7CompletionReport, M7ReportError>;
```

### `xtask/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod m7_report;
```

### `xtask/src/main.rs` (modify — one new `Command::M7Report` variant, additive)

`Command::M7Report { criterion: u8_or_str, server_bin: Option<PathBuf>, #[arg(long)] compose: bool, reference_tier: Option<String>, out_dir: PathBuf }`, dispatched to `m7_report::run`, writing `target/verify/m7-acceptance.json` via `xtask::tier_result`-shaped output and exiting via `exit_code_for` on the aggregated `M7CompletionReport.overall` — the same additive-variant shape every prior blueprint's own `Command` extension already established.

### `deploy/cluster/docker-compose.m7-acceptance.yml` (new, non-code)

Exactly Context §C's shape: `proxy-1`/`proxy-2` services (`role = "proxy"`, `directory_seeds` naming the base file's three `node-*` services, distinct published host ports), plus a `m7-results` named volume additively mounted onto every already-declared `node-*` service (via compose's own override-merge semantics — no field of the base file's own `node-*` service definitions is replaced, only extended) carrying each node's own `--metrics-snapshot-log`/`--region-tick-log`/`--takeover-event-log` output paths.

### `deploy/cluster/README.md` (modify — additive section, Context §C/§I)

One new section restating this blueprint's own overlay-file usage (`docker compose -f docker-compose.cluster-test.yml -f docker-compose.m7-acceptance.yml up`) and the `m7-acceptance-gate` job's own honest-gate note, appended after `M7-B08`'s own existing content — every existing line unchanged.

### `.github/workflows/ci.yml` (modify — Context §A.1's Finding, plus one new job)

```yaml
# on.workflow_dispatch.inputs.job.options gains a fourth entry: m7-acceptance
# (alongside the three M6-B06/M7-B08 already established: reference-host-gate,
# release, compose-topology-gate)

compose-topology-gate:
  # ...every existing line unchanged, except:
  if: github.event_name == 'workflow_dispatch' && inputs.job == 'compose-topology-gate'

m7-acceptance-gate:
  name: m7-acceptance-gate
  runs-on: ubuntu-24.04
  if: github.event_name == 'workflow_dispatch' && inputs.job == 'm7-acceptance'
  steps:
    - uses: actions/checkout@v4
    - run: cargo build -p rusty-clanker-server --all-features --release
    - run: >
        docker compose
        -f deploy/cluster/docker-compose.cluster-test.yml
        -f deploy/cluster/docker-compose.m7-acceptance.yml
        up -d
    - run: >
        cargo run -p xtask -- m7-report --criterion all --compose
        --server-bin target/release/rusty-clanker-server
        --reference-tier ${{ inputs.tier || 'm6-acceptance' }}
      continue-on-error: true
    - run: docker compose -f deploy/cluster/docker-compose.cluster-test.yml -f deploy/cluster/docker-compose.m7-acceptance.yml down -v
      if: always()
    - uses: actions/upload-artifact@v4
      if: always()
      with: { name: m7-acceptance-report, path: target/verify/m7-acceptance.json }
```

Mirrors `M7-B08`'s own `compose-topology-gate` job shape exactly (bring up, run, always tear down, always upload), restated once rather than re-derived.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46), restated exactly.** Every file below, plus every `src`/`xtask`/`deploy` file named in Deliverables with executable bodies replaced by `todo!()` (fields, derives, doc comments, and every signature unchanged), is the test-authoring changeset, committed first. The implementation changeset fills in bodies only — it must not modify any already-merged test file anywhere in the workspace, in particular no file under `crates/server/tests/` from `M7-B07` (`play_cluster_handoff_walk.rs` stays byte-unmodified) and no file under `crates/proxy/tests/` from `M7-B06`.

### `crates/testing/test-harness/tests/cluster_continuity.rs` (pure)

1. `zero_reentry_holds_for_a_clean_crossing_log` — a synthetic `Vec<ObservedPacket>` containing only `Play`-phase entries across the window → `find_reentry_violation` returns `None`.
2. `login_packet_inside_window_is_a_violation` — one synthetic `Login`-phase entry at a tick inside the window → `find_reentry_violation` returns `Some(ReentryViolation { tick, phase: Login })` naming that exact tick.
3. `respawn_packet_inside_window_is_a_violation` — one synthetic `PlayRespawn` entry inside the window → violation reported.
4. `reentry_outside_the_window_is_not_a_violation` — a `Login`-phase entry at a tick strictly before `window_start_tick` → `None` (a connection's own ordinary initial login, before any crossing began, must never be mistaken for a mid-crossing reentry).
5. `evaluate_takeover_budget_all_within_budget_passes` — four synthetic timestamps whose deltas are each comfortably under their own budget → `passed: true`, all three sub-flags `true`.
6. `evaluate_takeover_budget_detection_over_budget_fails` — `t_failed - t_kill` deliberately exceeds `detection_budget_ms` → `detection_within_budget: false`, `passed: false`.
7. `parse_takeover_event_log_skips_malformed_lines` — a fixture NDJSON file with one corrupted line among three valid ones → returns the two valid entries, never errors on the whole file.

### `crates/testing/paritybot/tests/proxy_fanout.rs` (pure)

8. `empty_fanout_falls_back_to_server_host_port` — `proxy_fanout: vec![]`, any `bot_index` → `resolve_connect_target` returns `(server_host, server_port)`, byte-identical to `M6-B01`/`M6-B06`'s own pre-existing behavior.
9. `nonempty_fanout_round_robins_deterministically` — `proxy_fanout` with two entries, bot indices `0..=5` → alternates `[0,1,0,1,0,1]`, the identical result across two independent calls (determinism).

### `xtask/tests/m7_report_ac1.rs`

10. `ac1_passes_within_budget_and_no_reentry` — a synthetic `timing_ms = 60`, a clean `observed` log → `Ac1Report.automated.status == Pass`.
11. `ac1_fails_over_budget_timing` — `timing_ms = 150` (over `AC1_TIMING_BUDGET_MS`) → `status == Fail`, detail names the measured value.
12. `dropped_packet_during_handoff_fails_ac1` **(mandatory self-test, Goal & Done)** — a synthetic `observed` log constructed by taking a clean, passing log and injecting one `Login`-phase entry mid-window (simulating a dropped/re-sent packet forcing a client-visible re-login) → `evaluate_ac1`'s own resulting `Ac1Report.automated.status == Fail`, with `reentry_violation` naming the exact injected tick — proves this blueprint's own AC1 evaluator actually catches the failure mode the task brief names, not merely that it is theoretically possible to construct one.

### `xtask/tests/m7_report_ac2.rs`

13. `ac2_passes_with_all_sub_budgets_met_and_exact_edit_reconstruction` — a synthetic `TakeoverEventLogEntry` sequence with all three deltas within budget, `expected_visible_edits == observed_matched_edits` → `Ac2Report.automated.status == Pass`.
14. `ac2_fails_when_reassignment_never_observed` — an event log missing any `RegionReassigned` entry → `Fail`, detail names "no RegionReassigned event observed."
15. `ac2_fails_on_edit_count_mismatch` — `expected_visible_edits != observed_matched_edits` → `Fail`, detail names both counts (never a fuzzy "close enough" pass).
16. `slow_takeover_fails_ac2` **(mandatory self-test)** — a synthetic event log whose `RegionReassigned`/resume timestamps are deliberately stretched past `AC2_REASSIGNMENT_BUDGET_MS`/`AC2_RESUME_IO_BUDGET_MS` → `evaluate_ac2`'s own `Ac2Report.automated.status == Fail`, `budget.reassignment_within_budget == false` and/or `resume_io_within_budget == false` — proves the takeover-window gate actually catches a slow takeover.

### `xtask/tests/m7_report_ac3.rs`

17. `ac3_passes_when_m6_criteria_and_both_redstone_diffs_pass` — `m6_ac1_passed`/`m6_ac2_passed`/`m6_ac3_passed` all `true`, `redstone_diffs` both `Ok(())` → `Ac3Report.automated.status == Pass`.
18. `ac3_fails_when_any_m6_criterion_fails` — `m6_ac2_passed: false`, everything else passing → `Fail`, detail names which M6 criterion failed (never silently absorbed into a generic message).
19. `perturbed_cross_node_replay_fails_ac3` **(mandatory self-test)** — a synthetic `redstone_diffs` entry for `two_torch_and_gate` carrying `Err("tick 12, pos (0,64,40): state_id mismatch 34 != 35".into())` (simulating a perturbed/incorrect cluster-mode replay) with everything else passing → `evaluate_ac3`'s own `Ac3Report.automated.status == Fail`, the failing contraption named in the case detail — proves the bit-identity gate actually catches a divergent replay.
20. `diff_traces_with_lag_tolerance_zero_lag_requires_exact_match` — two synthetic `RedstoneTrace` values differing at one tick, `max_lag_ticks: 0` → `Err(_)`, identical to a bare `diff_traces` call (proving the default tolerance is genuinely zero, per Context §G.4's own "never actually exercised" claim).
21. `diff_traces_with_lag_tolerance_nonzero_lag_accepts_a_shifted_match` — two synthetic traces where `actual` is `expected` shifted by exactly one tick, `max_lag_ticks: 1` → `Ok(())` (proving the tolerance mechanism itself works correctly when a future blueprint needs it, even though this blueprint's own default never engages it).

### `xtask/tests/m7_report_ac4.rs`

22. `ac4_passes_with_no_cluster_targets_and_closed_ports_and_m6_pass` — `cluster_target_found: None`, `open_ports: []`, `m6_regression_status: Pass` → `Ac4Report.automated.status == Pass`.
23. `ac4_fails_when_a_cluster_target_is_found_in_logs` — `cluster_target_found: Some("rc_cluster::health".into())` → `Fail`, detail names the offending target string.
24. `ac4_fails_when_a_cluster_port_is_open` — `open_ports` non-empty → `Fail`, detail names the offending address.
25. `ac4_fails_when_m6_regression_status_is_fail` — `m6_regression_status: Fail` → `Ac4Report.automated.status == Fail` (a monolithic-config run that itself fails M6's own criteria is not "no regression," regardless of AC4a's own inertness proof passing).
26. `live_cluster_thread_despite_no_config_fails_ac4` **(mandatory self-test)** — `scan_for_cluster_targets` called against a synthetic log-line vector containing one line whose target substring is `"rc_transport_net::connection"` (simulating a build that, despite no `[cluster]` config, still spun up a live QUIC listener thread and logged from it) → returns `Some(_)`, and the resulting `evaluate_ac4` call's `Ac4Report.automated.status == Fail` — proves the runtime inertness check actually catches a live cluster thread that the compile-time (`M7-B08`) proof alone would not observe.
27. `probe_ports_are_closed_reports_every_open_port` — an injected fake `connect` closure returning `Ok(())` (simulating an open port) for two of three probed addresses → returns exactly those two addresses.

### `xtask/tests/m7_report_dispatch.rs`

28. `criterion_4_without_compose_runs_ac4_only_and_never_requests_docker` — `M7ReportArgs { criterion: Criterion::Four, compose: false, .. }` against a `--server-bin` pointing at a locally-built binary and a temp config file with no `[cluster]` table → `run` returns `Ok(_)` with a populated `ac4` section and `ac1`/`ac2`/`ac3` sections each marked `Status::Fail` with `M7ReportError::ClusterIntegrationPending`-equivalent detail (never silently omitted — a requested-but-not-run section is reported, not hidden).
29. `criterion_1_without_compose_runs_the_real_inprocess_fixture` — `Criterion::One, compose: false` → `run`'s own `ac1` section reflects a real measurement (this test drives `M7-B07`'s own already-real fixture end to end, asserting `Ac1Report.automated.status == Pass` against the genuine, in-process, real-loopback-QUIC handoff — the one case in this file that is not purely synthetic, proportionate to being this blueprint's own primary real proof).
30. `criterion_2_with_compose_and_no_docker_fails_closed` — `Criterion::Two, compose: true` against an environment with no reachable docker daemon (the test's own injected docker-probe fake reports unavailable) → `Err(M7ReportError::ClusterIntegrationPending)`, `target/verify/m7-acceptance.json` written with `ac2.automated.status == "fail"` and the exact message text.
31. `criterion_3_with_compose_and_no_docker_fails_closed` — identical shape to case 30, for `Criterion::Three`.
32. `path_guard_already_covers_m7_b09s_own_new_paths` — every new path this blueprint's Deliverables introduce, checked against `xtask::path_guard::PROTECTED_PATHS` (`M0-B08`, reused) — `crates/testing/test-harness/**`, `crates/testing/paritybot/**`, `crates/testing/gametest/**` already cover this blueprint's own new files under an existing row; `xtask/**` and `.github/workflows/ci.yml` and `deploy/**` are this blueprint's own `Changeset-Type: governance`-labeled edits (Constraints (b)), the identical labeling `M7-B08` already established for its own analogous xtask-plus-CI-plus-deploy addition.

## Implementation steps

1. **`rc-test-harness`.** Add `ManagedServer::kill_now`/`pid` (Context §J) and `cluster_continuity.rs` (Context, Deliverables). Observable: `crates/testing/test-harness/tests/cluster_continuity.rs`'s 7 cases pass.
2. **`rc-paritybot`.** Add `MultiRegionScenarioConfig::proxy_fanout`/`resolve_connect_target` (Context §F.1). Observable: `proxy_fanout.rs`'s 2 cases pass; every pre-existing `M6-B01`/`M6-B06` call site (which never sets the new field) still compiles and behaves identically.
3. **`rc-gametest`.** Add `cluster_replay.rs` (Context §G.3/§G.4) and `corpus/redstone/m7_cluster_split_layout.ron` (Context §G.2). Observable: the layout fixture's own bounding-box-straddles-boundary check (Goal & Done) passes; `diff_traces_with_lag_tolerance`'s own two cases (20-21) pass.
4. **`xtask` — `m7_report.rs` skeleton.** Every type/constant from Deliverables, every function `todo!()`-stubbed if not already from the test changeset. Observable: crate compiles with `todo!()`s only.
5. **`xtask` — `evaluate_ac1`/`evaluate_ac2`/`evaluate_ac3`/`evaluate_ac4`/`build_report` real bodies.** Observable: cases 10-27 pass, including all four mandatory self-tests (12, 16, 19, 26).
6. **`xtask` — `run`/dispatch real body.** Context §I's routing logic — per-criterion, per-`compose` branching, `ClusterIntegrationPending` short-circuit per unavailable leg, partial-report-never-blocks-an-achievable-section behavior. Observable: cases 28-31 pass.
7. **`crates/server/tests/ac1_zero_reentry.rs`.** Extends `M7-B07`'s own fixture (Context §D.2), never modifying that blueprint's own file. Observable: `ac1_zero_reentry_extends_the_real_handoff_walk` passes against the real, in-process, real-loopback-QUIC fixture.
8. **`deploy/cluster/docker-compose.m7-acceptance.yml`, `README.md` addition.** Per Deliverables. Observable: a plain-text structural check (mirroring `M7-B08`'s own `compose_file_is_valid_and_matches_declared_services`) confirms `proxy-1`/`proxy-2` service markers and the `m7-results` volume mount are present.
9. **`.github/workflows/ci.yml`.** Context §A.1's Finding (the `compose-topology-gate` reconciliation) plus the new `m7-acceptance-gate` job. Observable: `actionlint`/GitHub's own workflow-syntax validation accepts the file; neither new/changed job fires on `push`/`pull_request`.
10. **Run the full acceptance suite.** `cargo nextest run -p rc-test-harness -p rc-paritybot -p rc-gametest -p xtask -p rusty-clanker-server` — every case named above, plus every pre-existing `M7-B01`..`M7-B08` test, passes.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- path-guard`, `-- test` — all exit 0.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50); confirm `m7-acceptance-gate` is correctly excluded from the required-status-check set (a repository-settings check, not a code check — implementer/reviewer verification step, mirroring `M6-B04`/`M6-B06`/`M7-B08`'s own identical step).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file under `crates/testing/test-harness/tests/cluster_continuity.rs`, `crates/testing/paritybot/tests/proxy_fanout.rs`, `xtask/tests/m7_report_*.rs`, and `crates/server/tests/ac1_zero_reentry.rs` is committed first, alongside `todo!()`-stubbed `src` files carrying every field/derive/signature already fixed. The implementation changeset fills in real bodies only.

(b) **Changeset labeling.** Per this lineage's established `Changeset-Type` convention (`M0-B08`, reused): `crates/testing/**`'s test-authoring and implementation changesets are labeled `test-authoring`/`implementation` normally; the `xtask/**`, `.github/workflows/ci.yml`, and `deploy/**` changes are committed as one separate, `Changeset-Type: governance`-labeled changeset, mirroring `M7-B08`'s own identical precedent for an analogous xtask-plus-CI-plus-deploy addition.

(c) **No new external dependencies.** Every crate this blueprint touches already carries every dependency this blueprint needs (`std::process::Child::kill()` for §J; `serde`/`thiserror` already present everywhere they are used); no new `[workspace.dependencies]` entry is added.

(d) **This blueprint adds zero cluster-mode production behavior.** Every type and function in Deliverables lives under `crates/testing/`, `xtask/`, or `deploy/`, or is a genuinely new *test* file under `crates/server/tests/`/`crates/testing/*/tests/` — never a change to `crates/cluster/`, `crates/transport-net/`, `crates/proxy/`, or any production file under `crates/server/src/`. An implementer who finds themselves modifying a file under any of those paths has misread this blueprint's own scope and must stop.

(e) **`M7-B01`..`M7-B08`'s already-shipped mechanism is never re-implemented, duplicated, or modified.** In particular: `M7-B07`'s own `play_cluster_handoff_walk.rs` is extended by a **new**, separate test file (Context §D.2), never edited in place; `M7-B08`'s own `docker-compose.cluster-test.yml` is extended by a **new**, separate override file (Context §C), never edited in place; `M6-B06`'s own `evaluate_ac1`/`evaluate_ac2`/`evaluate_ac3`/`M6ReportResult` are called, unmodified, never re-implemented under a new name; `M3-B07`'s own `diff_traces`/`RedstoneTrace`/`ContraptionSpec` are imported and reused, unmodified.

(f) **No Mojang or third-party reimplementation code.** Every mechanism here is derived solely from `11-roadmap-milestones.md`'s M7 acceptance criteria and this blueprint's own cited resolutions of prerequisite blueprints' already-fixed APIs and gaps (ASSET-D18/D19/D30).

(g) **Scope boundary.** This blueprint does not implement: a concrete, real-network `openraft::RaftNetworkFactory`/`JoinClient` (`M7-B08`'s own Context §A item 1 — a genuinely separate, not-yet-written blueprint's job); `rusty-clanker-server::main.rs`'s real `ClusterProxy`/`ClusterNode` role-serving wiring (`M7-B08`'s own Context §A item 3, likewise separate); `CLUSTER-D2`'s periodic rebalancer or any planned-migration exercise (`M7-B03`'s own crate, out of this blueprint's own scope per Context §A's header); a cross-AZ/degraded-topology test exercising CLUSTER-D8's own lag tolerance for real (Context §G.4's own named Open Issue). Do not add placeholder implementations of any of these as a shortcut to a green `m7-acceptance-gate` run.

(h) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust, including the subprocess-kill machinery (`std::process::Child::kill()`'s own already-safe API).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-test-harness -p rc-paritybot -p rc-gametest -p xtask -p rusty-clanker-server --all-features
cargo nextest run -p rc-test-harness -p rc-paritybot -p rc-gametest -p xtask -p rusty-clanker-server
cargo test --doc -p rc-test-harness -p rc-paritybot -p rc-gametest
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- m7-report --help
```

Expected: every command exits 0. `cargo nextest run` additionally runs: 7 (`cluster_continuity.rs`) + 2 (`proxy_fanout.rs`) + 2 (`diff_traces_with_lag_tolerance` cases, inside `xtask/tests/m7_report_ac3.rs`) + 5 (`m7_report_ac1.rs`) + 4 (`m7_report_ac2.rs`) + 5 (`m7_report_ac3.rs`, remaining) + 6 (`m7_report_ac4.rs`) + 4 (`m7_report_dispatch.rs`) + 1 (`ac1_zero_reentry.rs`) = 36 new cases, plus every pre-existing test across `M7-B01`..`M7-B08`, `M6-B01`/`M6-B04`/`M6-B06`, and `M3-B07`/`M3-B08` — all still passing, byte-for-byte unmodified. `cargo run -p xtask -- m7-report --criterion all --compose ...` is **not** part of this command list — it runs only under the separate, `workflow_dispatch`-only `m7-acceptance-gate` CI job (Context §I). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
