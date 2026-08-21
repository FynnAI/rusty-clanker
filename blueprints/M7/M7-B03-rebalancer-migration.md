# M7-B03 — Rebalancer & Partition Migration (`rc-cluster`)

| Field | Content |
|---|---|
| ID | M7-B03 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | M7-B01 (`rc-transport-net::NetworkTransport` — `NodeDirectory`, the per-`(from,to)`-pair QUIC stream mapping, CLUSTER-D9's mid-migration redirect via `resolve()` re-check + `ControlFrame::NotOwner` + `NOT_OWNER_GRACE_WINDOW` — reused unmodified, restated in full below, §E). M7-B02 (`rc-cluster` — `NodeId(pub String)`, `Epoch(pub u64)`, `ClusterConfigEpoch`, `DirectoryEntry{node,epoch}`, `RegionLease{region,node,epoch}`/`is_current`, `ClusterNode::{propose_assign_region, propose_unassign_region, directory, raft, node_id}`, `DirectoryCache::{lease_of, config_epoch, snapshot}`, `ClusterError` — the exact, already-fixed API this blueprint's own new modules build additively on top of, inside the same crate; restated in full below, §B). |
| Implements | CLUSTER-D2 (load-driven rebalancing — the placement/hysteresis algorithm, restated exactly and made concrete, §C). CLUSTER-D3 (migratability ceiling — restated exactly, §D). CLUSTER-D4 (no colocation constraints beyond region atomicity — restated verbatim, including `05-game-mechanics.md`'s confirmed MECH-D14 caveat, §D). CLUSTER-D5 (raft-committed directory as sole ownership authority — the exact epoch-bump ordering this blueprint's migration protocol obeys, §E). CLUSTER-D6/D9 (border-halo traffic and mid-migration message routing travel unmodified over `NetworkTransport`; restated as the reason this blueprint adds **zero** message-routing code, §E.4). CLUSTER-D8 (co-location migration for a hot cross-node border — restated as one more `RebalancerAction` source, §C.5). CLUSTER-D16's foundation (epoch/lease-fencing primitives this blueprint's abort path reuses; the takeover *algorithm* itself remains a sibling blueprint's job, restated, §F). CLUSTER-D17 (durability window — why the staging write must be durable before the epoch bump, §E.3). CLUSTER-D19 (epoch/lease fencing — the exact check that makes a half-migration safe, §F.3). CLUSTER-D24 (pre-warming — this blueprint's own region-data half, boundary against `M6-B07`'s player-facing half, §G). ARCH-D6 (monolithic merge/split — precedence rules against cluster reassignment, §H). ARCH-D19 (per-region tick-duration EWMA — the load signal this blueprint aggregates, sourced via `M6-B02`'s `RegionMetricsSnapshot`, restated, §C.2). WORLD-D19/D20 (`RegionManifest`/`ChunkSnapshot` — the on-disk formats this blueprint's own `MigrationEnvelope` wraps as opaque bytes, restated, §E.2). TEST-D45/D46 (test-first changeset boundary). TEST-D50 (CI-is-authority). |
| Crates touched | `rc-cluster` (`crates/cluster/`) only — six new modules added to the crate M7-B02 already scaffolded. **Zero** new external dependencies (every type used below is already in `rc-cluster`'s M7-B02 `Cargo.toml`: `serde`, `postcard`, `thiserror`, `tracing`, `tokio`, `parking_lot`, plus `rc-messaging` for `RegionId`). **Zero** new intra-workspace dependency edges — restated and justified as this blueprint's own binding resolution of a flagged corpus discrepancy, §A. Not `rc-scheduler`, not `rc-chunk-storage`, not `rc-transport-net`, not `rusty-clanker-server` — every one of those is touched only through a narrow trait boundary this blueprint defines and a future composition-root blueprint fulfills, named explicitly at each point below. |
| Estimated scope | L — the same class of stated, deliberate size exception `M6-B07`/`M7-B01`/`M7-B02` already established: this is the one blueprint that fixes `rc-cluster`'s complete rebalancer policy, migration envelope format, and the four trait boundaries (`LoadReportSink`, `RegionFreezeController`, `MigrationStore`, `RegionPrewarmHint`) plus `MigrationCoordinator`'s own protocol driver, all of which a future composition-root blueprint wires against real `rc-scheduler`/`rc-chunk-storage` types. Splitting the rebalancer policy from the migration protocol would force each half to restate the other's shared vocabulary (`NodeLoadReport`, migratability-ceiling exclusion, lifecycle-pinning) from scratch. |

## Goal & Done definition

Give `rc-cluster` the load-driven rebalancer and the cross-node partition-migration coordinator CLUSTER-D2–D5/D8/D19/D24 describe — entirely as pure, testable logic plus four narrow trait boundaries this blueprint defines and consumes, mirroring the exact "define the trait, prove it with an in-process fake, defer the real wire/scheduler integration to a sibling blueprint" split `M0-B02`→`M0-B03` and `M7-B02`'s own `RaftNetworkFactory`/`JoinClient` split already established — because the real freeze/serialize hooks live in `rc-scheduler` and the real staged-blob storage lives in `rc-chunk-storage`'s not-yet-built `ObjectStoreBackend`, and `rc-cluster` may not gain a Cargo dependency on either (§A). Concretely: (1) `NodeLoadReport`/`LoadReportSink` — the shape and ingestion point every node's own periodic load sample arrives through, regardless of which future blueprint supplies the wire transport that carries it there; (2) `evaluate_placement` — CLUSTER-D2's exact least-loaded-placement/40%-of-mean/3-window-hysteresis algorithm as a pure function over a load-report snapshot, directly unit-testable against synthetic matrices with known-optimal answers; (3) CLUSTER-D3's migratability-ceiling exclusion and its "request an ARCH-D6 split instead" fallback, expressed as one more `RebalancerAction` variant; (4) `MigrationEnvelope`/`RegionSnapshotPayload` — the exact, versioned, postcard-encoded wire/staging format this blueprint owns, wrapping opaque chunk-snapshot and entity-snapshot byte blobs a future blueprint's real `RegionFreezeController` produces; (5) `MigrationCoordinator` — the six-phase migration protocol driver (freeze → serialize → stage → epoch-bump → destination-restore → source-cleanup), with a precisely specified abort/rollback path fenced by `RegionLease::is_current`; (6) `RegionPrewarmHint` — the region-data pre-warm seam, with an explicit boundary statement against `M6-B07`'s own player-facing pre-warm; (7) the precedence rule between ARCH-D6's same-node lifecycle events and this blueprint's cross-node reassignment.

Done when:

- [ ] `cargo build -p rc-cluster --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-cluster`.
- [ ] `placement_picks_the_known_optimal_assignment_on_synthetic_matrices` (a table-driven test over hand-derived load matrices with a known-correct answer) passes.
- [ ] `hysteresis_requires_three_consecutive_over_threshold_windows` and `hysteresis_resets_when_delta_drops_back_under_threshold` both pass.
- [ ] `migratability_ceiling_excludes_oversized_region_and_requests_split_instead` passes.
- [ ] `migration_envelope_round_trips_byte_exact` (postcard round-trip over synthetic opaque byte blobs, proving the envelope format itself is lossless — the "region state serialization" identity guarantee this blueprint owns) passes.
- [ ] `migration_pending_inbound_preserves_order_no_loss_no_duplication` (a `proptest!` property test) passes.
- [ ] `migration_happy_path_calls_freeze_serialize_stage_bump_in_order` and `migration_abort_before_epoch_bump_resumes_source_and_never_commits` both pass.
- [ ] `migration_fencing_rejects_resume_after_a_newer_epoch_committed` passes — the epoch-fencing safety property.
- [ ] `prewarm_hint_fires_before_the_eventual_migration_decision` passes.
- [ ] `rebalancer_never_selects_a_lifecycle_pinned_region` passes — the ARCH-D6 precedence rule.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-cluster`'s dependency-graph edges are byte-for-byte unchanged from M7-B02 (only `rc-messaging`, restated §A); no `SIM`/`NETRENDER` rule is newly touched.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-cluster` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### §A — Scope boundary and this blueprint's own binding resolution of a flagged corpus discrepancy

`12-workspace-structure.md`'s WS-D3 rule 2 prose is unambiguous: "`rc-scheduler`... may never appear as a dependent of, nor depend on... `rc-cluster`." Its own dependency-graph *diagram*, however, draws an edge `cluster --> sched`. `M7-B02`'s own Context §A already flagged this exact contradiction as "a genuine inconsistency between that document's prose and its own diagram," naming "whoever authors the rebalancer/takeover blueprint" — this blueprint — as the one to reconcile it. **This blueprint's binding resolution: the prose wins, the diagram edge is not exercised by anything this blueprint builds.** `rc-cluster`'s own `Cargo.toml` gains **zero** new dependencies — not `rc-scheduler`, not `rc-chunk-storage`, not `rc-transport-net`. Every piece of information the rebalancer/migration coordinator needs that would otherwise require one of those edges instead crosses a narrow trait boundary this blueprint defines here and a future composition-root blueprint implements against the real crate, exactly the pattern `M7-B01`'s `NodeDirectory`/`NetworkTransportMetricsSink` and `M7-B02`'s `RaftNetworkFactory`/`JoinClient` already established for the identical reason (dependency-direction discipline, restated once per §A of each of those blueprints). Four trait boundaries exist in this blueprint, named in full below: `LoadReportSink` (§C.2 — replaces a would-be `rc-scheduler` pull with a push the caller performs), `RegionFreezeController` (§E.1 — replaces a would-be `rc-scheduler` call with an injected trait), `MigrationStore` (§E.2 — replaces a would-be `rc-chunk-storage`/`object_store` call), and `RegionPrewarmHint` (§G) — plus one further, non-trait, composition-root-owned obligation this blueprint names but does not build: the "detect I am the fresh owner of an unregistered region" watch loop `MigrationCoordinator::accept_incoming` is called from (§E.5). This blueprint ships **zero** implementations of any of the four traits beyond in-process test fakes — real wiring into `rusty-clanker-server`/`rc-scheduler`/`rc-chunk-storage` is a future composition-root-extension blueprint's job, restated explicitly in Constraints.

### §B — `rc-cluster`'s already-fixed API this blueprint builds on (M7-B02, restated)

`RegionId(pub u64)` (`rc-messaging`, M0-B02, imported unmodified). `NodeId(pub String)` — operator-chosen, `Display`, `Hash`/`Eq`. `Epoch(pub u64)` — per-region fencing counter, `Epoch::FIRST = Epoch(1)`, `.next()`. `DirectoryEntry { node: NodeId, epoch: Epoch }`. `RegionLease { region: RegionId, node: NodeId, epoch: Epoch }` with `is_current(&self, presented_epoch: Epoch) -> bool` (`self.epoch == presented_epoch`) — this is CLUSTER-D19's fencing token, made concrete by M7-B02, and the exact primitive §F.3's abort-safety check below is built on. `DirectoryCache::lease_of(&self, region: RegionId) -> Option<RegionLease>` — non-blocking, may be CLUSTER-D7-bounded-stale on a follower (M7-B02 §E's own consistency contract, unchanged here). `ClusterNode::propose_assign_region(&self, region: RegionId, node: NodeId) -> Result<DirectoryCommandResponse, ClusterError>` — the exact call this blueprint's epoch-bump step (§E.3) issues; `propose_unassign_region` identically for retirement. `ClusterNode::{directory(), raft(), node_id()}`. `ClusterError` (`NoReachableSeed`, `NotLeader`, `AlreadyInitialized`, `Storage`, `RaftFatal`, `RaftClientWrite`) — this blueprint's own new fallible operations use a **new**, separate error type (`MigrationError`, §E.1), never overloading `ClusterError` with unrelated failure modes, matching this project's own "one error type per genuinely distinct failure domain" convention (`NetworkTransportBuildError` kept separate from `TransportError`, `M7-B01` §K, for the identical reason).

### §C — The load-driven rebalancer (CLUSTER-D2, restated and made concrete)

CLUSTER-D2, verbatim: "Node-level rebalancing is dynamic and load-driven... every 30 s, the raft leader evaluates each node's aggregate load (sum of its owned regions' ARCH-D19 tick-duration EWMAs) and reassigns individual regions from the most-loaded to the least-loaded node using a least-loaded-node placement strategy, hysteresis-gated (a reassignment is proposed only if the load delta between busiest and quietest node exceeds 40% of cluster mean for 3 consecutive evaluation windows, 90 s)."

**§C.1 — Constants (seed defaults, calibration-pending — the identical status every other numeric threshold in this corpus carries, ARCH-D6/D19's own framing).**

```rust
pub const REBALANCE_EVAL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
pub const REBALANCE_HYSTERESIS_WINDOWS: u32 = 3;
pub const REBALANCE_HYSTERESIS_DELTA_RATIO: f64 = 0.40;
/// CLUSTER-D3's migratability ceiling (§D).
pub const MIGRATABILITY_MAX_ENTITY_COUNT: u32 = 4096;
pub const MIGRATABILITY_MAX_BYTES: u64 = 64 * 1024 * 1024;
/// This blueprint's own new seed default (§G): fire the region-data pre-warm hint the
/// moment hysteresis evidence first starts accumulating, giving the destination up to
/// two more evaluation windows (60s) of head start before the actual freeze begins.
pub const PREWARM_HINT_AT_WINDOW: u32 = 1;
```

**§C.2 — How node load reaches the rebalancer (the `rc-scheduler` pull this blueprint deliberately does not perform).**

ARCH-D19's tick-duration EWMA and `M6-B02`'s `MetricsRegistry::snapshot(&pool) -> MetricsSnapshot { regions: Vec<RegionMetricsSnapshot { region_id, tick_duration_ewma_ms: Option<f64>, .. }>, .. }` are both `rc-scheduler` types this crate never imports (§A). Instead, this blueprint defines the **shape** the caller must produce and the **sink** it pushes into:

```rust
/// One node's own periodic self-report — or a report this node received from a peer
/// over whatever transport a future blueprint wires (§A; this blueprint does not care
/// which). Produced, once per REBALANCE_EVAL_INTERVAL, by summing every region a node
/// currently owns' own `tick_duration_ewma_ms` (M6-B02's own value, `None` treated as
/// `0.0` — a freshly spawned region with no completed tick yet contributes no load).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeLoadReport {
    pub node: NodeId,
    pub total_load_ms: f64,
    pub regions: Vec<RegionLoadSample>,
    pub sampled_at_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RegionLoadSample {
    pub region: RegionId,
    pub load_ms: f64,
    pub entity_count: u32,
    /// Approximate serialized `World`-slice size this region would produce if migrated
    /// right now (§D's ceiling input) — a cheap, possibly-stale estimate is acceptable;
    /// this blueprint never trusts it for anything beyond candidate *exclusion*.
    pub approx_snapshot_bytes: u64,
    /// `true` while an ARCH-D6 split/merge for this exact region is pending admission
    /// or mid-execution on its owning node (§H's precedence rule) — set by whatever
    /// future blueprint builds `NodeLoadReport`s from real `RegionManager` state.
    pub lifecycle_pinned: bool,
}

/// The ingestion seam (§A) — implemented by `RebalancerEngine` itself (§C.4). A future
/// blueprint's real wire transport calls `ingest` once per received report, for both
/// this node's own local self-sample and every peer report it receives; this crate
/// never decides *how* a report crosses the network.
pub trait LoadReportSink: Send + Sync + 'static {
    fn ingest(&self, report: NodeLoadReport);
}
```

**§C.3 — `evaluate_placement`: the pure algorithm, directly unit-testable.**

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum RebalancerAction {
    /// CLUSTER-D2's own reassignment — the eligible region on the busiest node with the
    /// **highest** individual `load_ms` (this blueprint's own binding resolution of "which
    /// region": moving the single largest eligible contributor relieves the busiest
    /// node's load in the fewest migrations — CLUSTER-D2's own text names only the
    /// destination-selection rule, "least-loaded-node placement," leaving source-region
    /// selection open) is assigned to the globally least-loaded reporting node.
    MigrateRegion { region: RegionId, source: NodeId, dest: NodeId },
    /// CLUSTER-D3's ceiling fallback (§D): every eligible-by-load candidate on the
    /// busiest node also exceeds the ceiling — request an ordinary ARCH-D6 split on its
    /// current node instead of migrating, and skip reassignment this window.
    RequestSplit { region: RegionId, node: NodeId },
    /// §G's region-data pre-warm hint, fired at `PREWARM_HINT_AT_WINDOW`.
    PrewarmHint { region: RegionId, likely_dest: NodeId },
}

#[derive(Clone, Debug, Default)]
pub struct HysteresisState {
    consecutive_windows_over_threshold: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct RebalancerConfig {
    pub hysteresis_windows: u32,
    pub delta_ratio: f64,
    pub max_entity_count: u32,
    pub max_bytes: u64,
    pub prewarm_at_window: u32,
}
impl Default for RebalancerConfig {
    fn default() -> Self {
        Self {
            hysteresis_windows: REBALANCE_HYSTERESIS_WINDOWS,
            delta_ratio: REBALANCE_HYSTERESIS_DELTA_RATIO,
            max_entity_count: MIGRATABILITY_MAX_ENTITY_COUNT,
            max_bytes: MIGRATABILITY_MAX_BYTES,
            prewarm_at_window: PREWARM_HINT_AT_WINDOW,
        }
    }
}

/// CLUSTER-D2/D3's complete placement decision, one evaluation window at a time. Pure:
/// no I/O, no leader check (the caller, `RebalancerEngine::tick`, §C.4, gates this on
/// "am I currently raft leader" before ever calling it — CLUSTER-D2's "the raft leader
/// evaluates" is a caller-side gate, not a property of the algorithm itself). Returns
/// zero or more actions for this window: at most one `MigrateRegion`/`RequestSplit`
/// (mutually exclusive — a window either reassigns or splits, never both), plus zero or
/// more `PrewarmHint`s (§G — fired independently of whether the migration itself fires
/// this window).
pub fn evaluate_placement(
    reports: &std::collections::HashMap<NodeId, NodeLoadReport>,
    hysteresis: &mut HysteresisState,
    config: &RebalancerConfig,
) -> Vec<RebalancerAction>;
```

Algorithm, stated precisely: (1) if `reports.len() < 2`, reset `hysteresis` to `0` and return `[]` — no rebalancing target exists with fewer than two nodes reporting. (2) Compute `mean = sum(total_load_ms) / reports.len()`; find `busiest`/`quietest` by `total_load_ms`. (3) `delta = busiest.total_load_ms - quietest.total_load_ms`; `threshold = config.delta_ratio * mean`. (4) If `delta <= threshold`: reset `hysteresis.consecutive_windows_over_threshold = 0`, return `[]` (still evaluate and possibly emit `PrewarmHint`s below is **not** done in this branch — a hint with no trending migration ahead of it would be a wasted prefetch, §G). (5) If `delta > threshold`: increment `hysteresis.consecutive_windows_over_threshold`. If the new count equals exactly `config.prewarm_at_window` (default `1`), additionally compute the same busiest-region/quietest-node selection step 6 would use and emit `RebalancerAction::PrewarmHint { region, likely_dest: quietest.node }` **in addition to** whatever step 6 returns for this same window (steps 5/6 are not mutually exclusive — window 1 may emit only a hint, window `hysteresis_windows` emits the real action). (6) If the count is `< config.hysteresis_windows`, return whatever step 5 already produced (possibly empty, possibly one hint) — the reassignment itself does not fire yet. (7) If the count reaches `config.hysteresis_windows`: reset the counter to `0` (a fresh trend must accumulate for the *next* reassignment — prevents firing every window once past threshold, matching ARCH-D6's own hysteresis-reset-on-fire precedent) and select the source region — among `busiest.regions` where `!lifecycle_pinned`, partition into `eligible = entity_count <= max_entity_count && approx_snapshot_bytes <= max_bytes` and `oversized`. If `eligible` is non-empty, pick the max-`load_ms` entry, return `[MigrateRegion { region, source: busiest.node, dest: quietest.node }]` (merged with any step-5 hint already collected this call). If `eligible` is empty but `oversized` is non-empty, pick the max-`load_ms` oversized entry, return `[RequestSplit { region, node: busiest.node }]`. If both are empty (every region on the busiest node is `lifecycle_pinned`), return `[]` — nothing eligible to act on this window, counter already reset.

**§C.4 — `RebalancerEngine`: the leader-gated driver wrapping the pure algorithm.**

```rust
pub struct RebalancerEngine<F, S, P>
where F: RegionFreezeController, S: MigrationStore, P: RegionPrewarmHint {
    reports: parking_lot::RwLock<std::collections::HashMap<NodeId, NodeLoadReport>>,
    hysteresis: parking_lot::Mutex<HysteresisState>,
    config: RebalancerConfig,
    freeze_controller: std::sync::Arc<F>,
    store: std::sync::Arc<S>,
    prewarm: std::sync::Arc<P>,
}

impl<F: RegionFreezeController, S: MigrationStore, P: RegionPrewarmHint> RebalancerEngine<F, S, P> {
    pub fn new(config: RebalancerConfig, freeze_controller: std::sync::Arc<F>, store: std::sync::Arc<S>, prewarm: std::sync::Arc<P>) -> Self;

    /// Call once per `REBALANCE_EVAL_INTERVAL`, from any node, regardless of
    /// leadership — cheap when not leader (one metrics read, no action). Only the
    /// current raft leader (`cluster.raft().metrics().borrow().current_leader ==
    /// Some(cluster.node_id().clone())`) actually calls `evaluate_placement` and acts
    /// on its output; every other node's call is a no-op beyond that one check, so
    /// leadership changing mid-run never requires special-casing — whichever node is
    /// leader *this* window simply starts acting from its own already-accumulated
    /// `reports`/`hysteresis` state (both are ordinary local fields, not raft-committed,
    /// per CLUSTER-D13's own "never carries... simulation/telemetry data" boundary —
    /// restated: a fresh leader's hysteresis counter starts at whatever ingest() calls
    /// it has already locally observed, never a synchronized cluster-wide value; this is
    /// an accepted, bounded imprecision — a leadership change can delay, never
    /// duplicate or corrupt, a pending reassignment, since `MigrationCoordinator`'s own
    /// epoch-fenced protocol, §E, is the actual safety boundary, not this counter).
    pub fn tick(&self, cluster: &crate::ClusterNode) -> Vec<RebalancerAction>;
}

impl<F: RegionFreezeController, S: MigrationStore, P: RegionPrewarmHint> LoadReportSink for RebalancerEngine<F, S, P> {
    fn ingest(&self, report: NodeLoadReport) { /* self.reports.write().insert(report.node.clone(), report); */ }
}
```

`tick`'s own body, beyond the leadership gate: `evaluate_placement(&self.reports.read(), &mut self.hysteresis.lock(), &self.config)`, returned as-is to the caller — `RebalancerEngine::tick` does **not** itself execute `RebalancerAction`s (spawn a migration, call `request_split`, fire a prewarm hint); it returns the decision, and a **separate**, explicit call — `RebalancerEngine::execute(&self, cluster, action) -> impl Future<Output = Result<(), MigrationError>>` (Deliverables) — performs the actual side effect for one action, dispatching `MigrateRegion` to `MigrationCoordinator::migrate_out` (§E.3), `RequestSplit` to `self.freeze_controller.request_split(region)`, `PrewarmHint` to `self.prewarm.hint_prewarm(region, likely_dest)`. Splitting decision from execution is what makes `evaluate_placement`'s own acceptance tests (Acceptance tests, below) exercise the algorithm with zero I/O and zero trait fakes at all — exactly the property the "synthetic load matrices, known-optimal assignments" requirement asks for.

**§C.5 — CLUSTER-D8's co-location migration, restated as an ordinary `RebalancerAction` source.**

CLUSTER-D8: a cross-node region-border pair sustaining `BorderUpdateEvent` traffic above ARCH-D11's own hot-border threshold "instead triggers a co-location migration: the rebalancer... moves the quieter of the two regions onto the same node as its hot neighbor, out of band from the normal 30 s window." This blueprint's own resolution: the hot-border **detection** (counting `BorderUpdateEvent` traffic per cross-node pair) is `NetworkTransport`'s own observability surface (`M7-B01` §K's `NetworkTransportMetricsSink::on_batch_flushed`, tagged by variant — already specified, not this blueprint's job to re-detect), so this blueprint adds **one more, narrower entry point**, bypassing the 30s/3-window gate entirely (CLUSTER-D8's own "out of band" wording): `RebalancerEngine::force_colocate(&self, cluster, quiet_region: RegionId, hot_region_owner: NodeId) -> impl Future<Output = Result<(), MigrationError>>` (Deliverables) — a direct call to `MigrationCoordinator::migrate_out(cluster, quiet_region, hot_region_owner)` (§E.3), skipping `evaluate_placement`/`HysteresisState` entirely, since CLUSTER-D8's own text is explicit this is not subject to the ordinary hysteresis gate. The caller (a future composition-root or `rc-transport-net`-metrics-consuming blueprint) is responsible for deciding *when* the hot-border threshold is crossed and which region is "the quiet one" — this blueprint only provides the one-call migration entry point once that decision is made, mirroring `RegionFreezeController::request_split`'s identical "this crate provides the call, not the decision," `M7-B02` §A item 2's own established framing for `propose_assign_region`.

### §D — Migratability ceiling and colocation constraints (CLUSTER-D3/D4, restated exactly)

CLUSTER-D3, verbatim: "A region above a configurable migratability ceiling (default: entity count > 4,096 or serialized `World`-slice size > 64 MiB) is never chosen as a cross-node rebalancing candidate directly. If such a region is also hot, the rebalancer instead requests an ordinary ARCH-D6 split first (on its current node, at near-zero cost) and re-evaluates the resulting smaller regions for migration on the next window." §C.3 step 7 implements this exactly: `eligible`/`oversized` partitioning by `entity_count <= MIGRATABILITY_MAX_ENTITY_COUNT && approx_snapshot_bytes <= MIGRATABILITY_MAX_BYTES`, falling back to `RequestSplit` only when every load-eligible candidate is oversized. No region is ever migrated above the ceiling — the algorithm structurally cannot produce a `MigrateRegion` action naming an oversized region, since `eligible`'s own filter excludes it before the `max_by(load_ms)` selection runs.

CLUSTER-D4, verbatim, restated in full (this blueprint's own binding colocation rule — there is no other): "No colocation constraints exist beyond region atomicity (a region is always wholly owned by exactly one node; ownership is never split mid-tick). Because ARCH-D24's addressing is already location-transparent, portal pairs, ender-pearl/projectile flight, and any other long-range cross-region interaction are ordinary `RegionTransferRequest`/`BorderUpdateEvent` traffic regardless of whether the crossing happens to be intra-node or inter-node — no special-casing is needed or added here. Confirmed by `05-game-mechanics.md`'s Interfaces section: no mechanic defines a colocation invariant beyond ordinary region atomicity, with the sole caveat that MECH-D14 declines cross-border block-ownership *transfer* for piston-pushed blocks specifically (a scope limitation self-healing via CLUSTER-D8, not a colocation requirement)." Direct consequence for `evaluate_placement` (§C.3): destination selection is **unconstrained** beyond "not the source node itself" — no affinity, no anti-affinity, no per-region destination exclusion list exists anywhere in this blueprint's algorithm, and none should ever be added without a joint revision of CLUSTER-D4 itself.

### §E — The migration protocol, end to end

**§E.1 — `MigrationError` and the `RegionFreezeController` trait boundary (§A).**

```rust
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("freeze of region {0:?} did not reach a clean sync point within the timeout")]
    FreezeTimedOut(RegionId),
    #[error("serialize failed for region {0:?}: {1}")]
    SerializeFailed(RegionId, String),
    #[error("staging store write failed for region {0:?} epoch {1:?}: {2}")]
    StageWriteFailed(RegionId, crate::ids::Epoch, String),
    #[error("staging store read failed for region {0:?} epoch {1:?}: {2}")]
    StageReadFailed(RegionId, crate::ids::Epoch, String),
    #[error("migration envelope decode failed: {0}")]
    EnvelopeDecode(String),
    #[error("migration envelope format_version mismatch: expected {expected}, found {found}")]
    EnvelopeVersion { expected: u16, found: u16 },
    #[error("epoch-bump commit failed for region {0:?}: {1}")]
    EpochBumpFailed(RegionId, #[source] crate::ClusterError),
    #[error("restore failed for region {0:?}: {1}")]
    RestoreFailed(RegionId, String),
    #[error("resume refused: this node's lease for region {0:?} is no longer current (epoch fencing, §F.3)")]
    FencedResume(RegionId),
}

/// A migration's live handle over a frozen region — opaque to `rc-cluster` beyond the
/// `region`/`epoch` it was frozen at. A future blueprint's real implementation carries
/// whatever `rc-scheduler` state it needs (e.g. a `RegionId` plus an internal "paused"
/// flag on `RegionManager`) behind this type — this crate never inspects it beyond the
/// two public fields every implementation must expose for this crate's own logging/
/// assertions.
pub struct FrozenRegionHandle {
    pub region: RegionId,
    pub frozen_at_epoch: crate::ids::Epoch,
}

/// The seam replacing a would-be `rc-scheduler` dependency (§A). A conforming
/// implementation (a future composition-root blueprint, bridging to real
/// `RegionManager`/`EdfScheduler` types) MUST uphold, precisely:
/// - `freeze`: stop admitting new ticks for `region` (deregister from EDF admission,
///   `M6-B07` §E's `EdfScheduler::unregister_region`) and, if a tick for `region` is
///   currently in flight, wait for it to reach its own Stage-10 sync point (ARCH-D9)
///   before returning — never interrupt a tick mid-stage. Also marks `region`
///   merge/split-ineligible for the duration of the freeze (§H's precedence rule) —
///   this crate cannot enforce that itself, it is a binding contract on the
///   implementation.
/// - `serialize`: produce the region's full state as opaque bytes — every currently-
///   loaded chunk's `ChunkSnapshot` (WORLD-D20, `rc_chunk_storage::encode_snapshot`,
///   `M2-B04`), every live entity's snapshot (ARCH-D10's own `EntitySnapshot` shape,
///   `rc-messaging`, M0-B02), and every message still sitting un-drained in the
///   region's own inbound queue at the moment of freeze (§E.4) — assembled by the
///   implementation into the exact `MigrationEnvelope` shape §E.2 fixes; this crate
///   never constructs that envelope itself for the `serialize` call, only decodes/
///   re-encodes it at the coordinator level (§E.3).
/// - `resume`: abort path — reverse `freeze`, resuming ordinary ticking for `region` on
///   THIS node, re-registering it with the EDF scheduler and un-pinning it from
///   merge/split eligibility. MUST first re-check `RegionLease::is_current` (§F.3) and
///   refuse (return the frozen handle's region as still-not-live, never silently
///   resume) if this node's lease has already been superseded by a newer epoch.
/// - `discard`: success path on the source — tear down `region`'s local bookkeeping
///   permanently (deregister from transport, drop the frozen handle); `region` no
///   longer exists on this node afterward.
/// - `restore`: destination side — decode a `MigrationEnvelope`'s inner blobs, spawn
///   `region`'s chunks/entities into a fresh local `World`, register with the EDF
///   scheduler and `NetworkTransport::register_region`, and begin ordinary ticking.
/// - `request_split`: CLUSTER-D3's fallback (§D) — request `RegionManager`'s own
///   ordinary ARCH-D6 split machinery act on `region` (its existing `force_split`
///   or organic-trigger path); this call only requests, it does not itself split.
/// - `lifecycle_state`: read-only — whether `region` currently has an ARCH-D6
///   split/merge pending or in flight (§H).
pub trait RegionFreezeController: Send + Sync + 'static {
    fn freeze(&self, region: RegionId) -> impl std::future::Future<Output = Result<FrozenRegionHandle, MigrationError>> + Send;
    fn serialize(&self, handle: &FrozenRegionHandle) -> impl std::future::Future<Output = Result<RegionSnapshotPayload, MigrationError>> + Send;
    fn resume(&self, handle: FrozenRegionHandle, current_lease: Option<crate::RegionLease>) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;
    fn discard(&self, handle: FrozenRegionHandle) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;
    fn restore(&self, region: RegionId, payload: RegionSnapshotPayload) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;
    fn request_split(&self, region: RegionId) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;
    fn lifecycle_state(&self, region: RegionId) -> RegionLifecycleState;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionLifecycleState { Idle, SplitPending, MergePending, Frozen }
```

**§E.2 — `MigrationEnvelope`/`RegionSnapshotPayload`: this blueprint's own versioned wire/staging format.**

```rust
/// This blueprint's own internal compatibility counter (mirrors WORLD-D20's
/// `RC_CHUNK_SNAPSHOT_VERSION` and CLUSTER-D12's postcard-everywhere convention) —
/// independent of both Mojang's `DataVersion` and `ChunkSnapshot`'s own
/// `RC_CHUNK_SNAPSHOT_VERSION` axis. Exact-match required, no migration, identical
/// policy to every other versioning axis this corpus defines.
pub const MIGRATION_ENVELOPE_FORMAT_VERSION: u16 = 1;

/// The complete migration payload — one region's full state plus its pending inbound
/// backlog, ready to stage (§E.3) and later restore (§E.5). The three `Vec<Vec<u8>>`
/// fields are OPAQUE to `rc-cluster` (§A): `chunk_snapshots[i]` is exactly one
/// `rc_chunk_storage::encode_snapshot(&ChunkSnapshot)` output (WORLD-D20, M2-B04) per
/// currently-loaded chunk; `entity_snapshots[i]` is exactly one entity's
/// `EntitySnapshot`-shaped bytes (`entity_id`/`source_chunk`/`component_data`,
/// ARCH-D10, postcard-encoded by the producing `RegionFreezeController`
/// implementation — this crate never decodes it); `pending_inbound[i]` is exactly one
/// postcard-encoded `rc_messaging::RegionMessage` that was sitting un-drained in the
/// region's own inbound queue at freeze time (§E.4). This crate only encodes/decodes
/// the OUTER envelope — never the inner blobs' own contents.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MigrationEnvelope {
    pub format_version: u16,
    pub region: RegionId,
    pub source_epoch: crate::ids::Epoch,
    pub chunk_snapshots: Vec<Vec<u8>>,
    pub entity_snapshots: Vec<Vec<u8>>,
    pub pending_inbound: Vec<Vec<u8>>,
}

/// Opaque, ready-to-stage bytes — always exactly `postcard::to_allocvec(&MigrationEnvelope)`
/// prefixed with nothing further (unlike `ChunkSnapshot`'s own 2-byte-prefix-outside-
/// the-body convention, this blueprint keeps `format_version` as an ordinary struct
/// field — WORLD-D20's own prefix trick exists specifically so a peek can happen
/// without decoding the body; this envelope is never peeked, only fully decoded on
/// receipt, so the extra mechanism buys nothing here and this blueprint does not adopt
/// it, a deliberate, cited divergence from that one precedent).
pub struct RegionSnapshotPayload(pub Vec<u8>);

pub fn encode_migration_envelope(envelope: &MigrationEnvelope) -> RegionSnapshotPayload;
pub fn decode_migration_envelope(payload: &RegionSnapshotPayload) -> Result<MigrationEnvelope, MigrationError>;
```

`decode_migration_envelope` rejects (never panics on) a `format_version` that does not equal `MIGRATION_ENVELOPE_FORMAT_VERSION` with `MigrationError::EnvelopeVersion`, and any postcard decode failure with `MigrationError::EnvelopeDecode` — the identical two-failure-mode shape `M2-B04`'s own `decode_snapshot` already established for `ChunkSnapshot`, restated here for a second, independent format.

**§E.3 — `MigrationCoordinator`: the six-phase protocol, exact ordering.**

```rust
pub trait MigrationStore: Send + Sync + 'static {
    fn write_staging(&self, region: RegionId, epoch: crate::ids::Epoch, payload: &RegionSnapshotPayload) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;
    fn read_staging(&self, region: RegionId, epoch: crate::ids::Epoch) -> impl std::future::Future<Output = Result<Option<RegionSnapshotPayload>, MigrationError>> + Send;
    fn delete_staging(&self, region: RegionId, epoch: crate::ids::Epoch) -> impl std::future::Future<Output = Result<(), MigrationError>> + Send;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MigrationOutcome { Completed, AbortedBeforeEpochBump }

pub struct MigrationCoordinator<F: RegionFreezeController, S: MigrationStore> {
    freeze_controller: std::sync::Arc<F>,
    store: std::sync::Arc<S>,
}

impl<F: RegionFreezeController, S: MigrationStore> MigrationCoordinator<F, S> {
    pub fn new(freeze_controller: std::sync::Arc<F>, store: std::sync::Arc<S>) -> Self;

    /// SOURCE-side driver — the six phases below, in this exact order. Never called
    /// concurrently for the same `region` (the caller, `RebalancerEngine`, §C.4, holds
    /// this as an implicit invariant — a region already `Frozen`, per
    /// `lifecycle_state`, is excluded from `evaluate_placement`'s own candidate pool by
    /// construction, §H).
    pub async fn migrate_out(&self, cluster: &crate::ClusterNode, region: RegionId, dest: crate::NodeId) -> Result<MigrationOutcome, MigrationError>;

    /// DESTINATION-side reactive handler (§E.5) — called by a future composition-root
    /// blueprint once it observes (via its own `DirectoryCache` polling/watch loop,
    /// out of this crate's scope, §A) that this node now owns `region` per a freshly
    /// committed epoch it did not locally request.
    pub async fn accept_incoming(&self, cluster: &crate::ClusterNode, region: RegionId) -> Result<(), MigrationError>;
}
```

`migrate_out`'s exact phase sequence — every phase's failure triggers the abort path (§F) unless otherwise noted:

1. **Freeze** (source). `let handle = self.freeze_controller.freeze(region).await?;` — bounded by the caller's own timeout policy (Deliverables' `FREEZE_TIMEOUT` constant, a seed default `Duration::from_secs(5)`; a real implementation that cannot reach a clean sync point within this window returns `MigrationError::FreezeTimedOut`, triggering abort below — this crate does not itself enforce the timeout via a `tokio::time::timeout` wrapper inside `migrate_out`, since a real `freeze` implementation's own internal wait loop is better positioned to honor it precisely against the region's actual tick cadence; this crate's own fakes, §E's acceptance tests, honor it directly).
2. **Serialize** (source). `let payload = self.freeze_controller.serialize(&handle).await?;` — produces the `RegionSnapshotPayload` per §E.1's binding contract on the implementation.
3. **Stage** (source, still under the OLD epoch — this write is legal because the source node's own lease has not yet been superseded, CLUSTER-D19). `self.store.write_staging(region, handle.frozen_at_epoch, &payload).await?;` — **must complete and be confirmed durable before phase 4 proceeds** (CLUSTER-D17's own durability-window rationale, restated: bumping the epoch before the staged blob is durable would leave a window where the new owner is authoritative but no durable copy of the region's state exists anywhere reachable, defeating the entire point of shared storage, CLUSTER-D18).
4. **Epoch bump** (the moment ownership legally transfers, CLUSTER-D5). `let response = cluster.propose_assign_region(region, dest.clone()).await.map_err(|e| MigrationError::EpochBumpFailed(region, e))?;` — on success, ownership is now `dest`'s per the raft-committed directory; on failure, phase 3's staged blob is deleted (best-effort, `store.delete_staging`) and the abort path (§F.1) runs — this is the **last** point at which abort-and-resume-on-source is safe.
5. **Destination fetch + restore** — **not** performed by `migrate_out` itself (a different process's own `accept_incoming` call, §E.5); `migrate_out` returns `Ok(MigrationOutcome::Completed)` immediately after phase 4's successful commit, **without** waiting for the destination to actually finish restoring. This is deliberate: `migrate_out` blocking on a cross-process round trip here would tie this coordinator's own async task to the destination's own, independently-paced restore work, and CLUSTER-D9/`M7-B01`'s own mid-migration message routing (§E.4) already makes the gap between phase 4 and the destination's actual `register_region` call safe for ordinary traffic without this coordinator's help.
6. **Source cleanup** — a **separate**, best-effort call the caller makes once it independently observes (via the same `DirectoryCache` mechanism `accept_incoming`'s own caller uses, or a direct confirmation the destination may choose to send out-of-band, both out of this crate's scope) that the destination has finished restoring: `self.freeze_controller.discard(handle).await?;`. Until this call happens, the source's own local bookkeeping for `region` remains present but permanently inert (the region will never tick again on this node — its lease is gone, §F.3 would refuse any attempt to resume it) — a harmless, bounded resource leak until cleanup runs, never a correctness hazard, restated explicitly as this blueprint's own accepted trade-off rather than left implicit.

**§E.4 — Mid-migration message routing: restated, zero new code.**

CLUSTER-D9, already fully resolved by `M7-B01` (§H/§J of that blueprint, restated here in full since this blueprint's own protocol depends on it): `NetworkTransport::send` re-resolves the destination `RegionId -> NodeId` via `NodeDirectory::resolve` on **every** call, with no cache of its own; when a receiving node gets a `RegionMessage` batch addressed to a `RegionId` it does not have registered, it replies `ControlFrame::NotOwner`, and the sender short-circuits repeat sends to that exact stale `(region, node)` pair to `Backpressure` for `NOT_OWNER_GRACE_WINDOW` (200ms, `M7-B01`'s own seed default) before retrying fresh. **Applied to this blueprint's own protocol precisely:** between phase 4 (epoch bump, this blueprint's own step) and the destination's eventual `register_region` call (part of `accept_incoming`, §E.5, a **different** blueprint's real wiring), any third region's ordinary send to `region` already resolves — correctly — to `dest`, since `resolve()` reads the now-updated raft-committed directory; it simply arrives at a `dest` that has not yet locally registered `region`, gets `NotOwner`'d, and self-heals via the grace-window retry the instant `dest` does register — **zero new message-routing code exists in this blueprint**, because `M7-B01`'s own mechanism already, correctly, absorbs exactly this gap by construction. This blueprint's only binding requirement on the ordering: phase 4 (epoch bump) and `accept_incoming`'s own eventual `register_region` call should happen "close together" as a matter of *quality* (minimizing the backpressure window ordinary senders experience), never as a matter of *correctness* — correctness holds regardless of how large the gap is, bounded only by `M7-B01`'s own retry/backoff numbers, already fixed elsewhere.

The region's own **pending inbound backlog at freeze time** (messages that arrived and sat in `region`'s local `NetworkTransport` inbound queue while frozen but un-ticked, §E.1's `freeze` contract) is a **different** concern from the routing question above — those messages already, correctly, reached the source node before freeze (ordinary `ARCH-D29` FIFO-per-pair delivery, untouched by this blueprint) but were never drained into a Stage-1 apply because the region stopped ticking. `serialize` (phase 2) is responsible for including every one of them, in original arrival order, as `MigrationEnvelope.pending_inbound` (§E.2) — `restore` (destination side, §E.5) applies them, in that same order, at the destination region's own first post-restore Stage 1, **before** any newly-arrived post-migration traffic (which by definition arrives only after `register_region`, strictly later). This is what makes "none lost, none duplicated, none reordered per pair" hold across the freeze/migrate boundary specifically — the one guarantee `M7-B01`'s own transport-level property does **not** by itself cover, since a frozen-but-not-yet-drained inbox is invisible to `try_recv`-level reasoning alone. This blueprint's own acceptance test (`migration_pending_inbound_preserves_order_no_loss_no_duplication`) proves exactly this envelope-level property.

**§E.5 — `accept_incoming`: the destination side, restated.**

```rust
pub async fn accept_incoming(&self, cluster: &crate::ClusterNode, region: RegionId) -> Result<(), MigrationError> {
    let lease = cluster.directory().lease_of(region);
    let Some(lease) = lease else { return Err(MigrationError::RestoreFailed(region, "no current lease for region".into())) };
    let Some(payload) = self.store.read_staging(region, lease.epoch).await? else {
        return Err(MigrationError::StageReadFailed(region, lease.epoch, "no staged blob at current epoch".into()));
    };
    self.freeze_controller.restore(region, payload).await?;
    self.store.delete_staging(region, lease.epoch).await.ok(); // best-effort cleanup, never fails the call
    Ok(())
}
```

The **detection** of "I am now the fresh owner of an unregistered region" (the condition that triggers a caller to invoke `accept_incoming` at all) is explicitly **not** built by this blueprint — it requires polling or watching `DirectoryCache::snapshot()` against the set of regions this node's own `RegionFreezeController`/`RegionManager` currently has registered, diffing the two, which needs the exact `rc-scheduler` visibility §A already ruled out of this crate's own dependency graph. This is named here, precisely, as the one piece of "region-data pre-warm... boundary" work (§G) and destination-side reconciliation this blueprint's own scope stops short of — a future composition-root-extension blueprint's job, restated in Constraints, mirroring `M7-B02`'s own identically-shaped "first-join node resolution... depends on `03`/`05` data this document does not yet have visibility into" open item.

### §F — Failure during migration: abort/rollback rules, and why epoch fencing makes a half-migration safe

**§F.1 — Failure before the epoch bump (phases 1–3): trivially safe, always reversible.** Nothing durable has changed ownership-wise — `CLUSTER-D5`'s own text is explicit that the raft-committed directory is the *sole* source of truth, and phase 4 is the only phase that touches it. A failure at freeze, serialize, or stage-write triggers: (a) best-effort `self.store.delete_staging(...)` for whatever partial blob phase 3 may have written (a blob at an epoch that never became the directory's current value is never referenced by anything and is safe to leave for a future GC sweep even if this delete itself fails — restated as a non-fatal cleanup step, never blocking the abort); (b) `self.freeze_controller.resume(handle, cluster.directory().lease_of(region)).await` — passing the **current** lease read fresh at abort time (not the one captured at freeze) is the one binding requirement this phase imposes, feeding directly into §F.3's fencing check inside `resume` itself.

**§F.2 — Failure after the epoch bump (phase 4 succeeded): ownership has legally moved, and "abort" in the source-resumes sense no longer exists.** Once `propose_assign_region` commits, `dest` is the raft-committed owner — `MigrationCoordinator::migrate_out` returns `Ok(MigrationOutcome::Completed)` regardless of whether `dest` ever actually finishes restoring (§E.3 phase 5's own stated reasoning). If `dest` then fails to complete `accept_incoming` (crashes, never observes the new ownership, or fails to read the staged blob), this is — by design, not by accident — **structurally identical** to CLUSTER-D16's own ordinary node-failure-takeover scenario: a node (here, `dest`, mid-restore rather than mid-tick) holds a region no one is currently ticking, and the raft leader's own failure-detection (`HealthMonitor`, `M7-B02` §H, unmodified) eventually reassigns it to a live node via the identical `propose_assign_region` mechanism this blueprint's own phase 4 already uses — possibly back to the original source (if still alive and least-loaded) or to a third node entirely (CLUSTER-D4's "no colocation constraints" applies here too — any live node is a legal destination). **This blueprint does not implement that reassignment itself** — CLUSTER-D16's takeover *algorithm* is explicitly a sibling blueprint's job (`M7-B02` §A item 2, restated here for the identical reason: it needs cluster-wide load data this blueprint's own `evaluate_placement` already has a shape for, but the *decision to reassign on failure* is a distinct trigger, not this blueprint's own 30s-window trigger). What this blueprint's own protocol guarantees, and is responsible for, is only that the durable prerequisite for that future takeover to succeed already exists the instant phase 4 commits: the staged blob at `(region, the-new-epoch)` is durable (phase 3's own ordering requirement, restated) and readable by *any* node the eventual takeover names, not only the originally intended `dest` — `read_staging` is keyed by `(region, epoch)`, never by a specific node identity, so this holds by construction.

**§F.3 — The epoch-fencing check that makes the whole thing safe under a network partition (CLUSTER-D19, restated and made concrete).** The one residual risk `CLUSTER-D19` names is a "zombie" old owner that has not yet noticed it lost ownership (a GC pause or transient partition, not process death) continuing to act. Applied here: suppose phase 4 commits (epoch bumped to `dest`), but the source node — due to a network partition, not a crash — never observes the commit and, on some retry/recovery path, attempts `resume` anyway. **Binding rule, enforced inside every conforming `RegionFreezeController::resume` implementation, restated as this blueprint's own non-negotiable safety requirement:** `resume` MUST re-read this node's own current lease for `region` (`cluster.directory().lease_of(region)`, passed in fresh by the caller per §F.1) and refuse — return `Err(MigrationError::FencedResume(region))`, never silently resume ticking — unless that lease's `node` field still names *this* node. Concretely: `match current_lease { Some(lease) if lease.node == self.local_node_id && lease.epoch == handle.frozen_at_epoch => { /* safe to resume */ }, _ => return Err(MigrationError::FencedResume(region)) }`. A region refused this way is **never** silently resumed — the implementation instead tears it down locally exactly as `discard` would (the zombie source, once it finally observes the truth, has nothing left to clean up beyond its own already-stale local state), never attempting to write chunk data or accept player connections for a region it no longer legitimately owns. This is the direct, concrete instantiation of CLUSTER-D19's own text ("a node may write to shared storage or be routed to by the proxy only while presenting the current epoch") applied to this blueprint's own one additional write path (`resume`) that CLUSTER-D19's original text did not enumerate by name. `migration_fencing_rejects_resume_after_a_newer_epoch_committed` (Acceptance tests) proves exactly this rule against a fake `RegionFreezeController` and a real `ClusterNode`-adjacent lease fixture.

### §G — Region-data pre-warm: the boundary against `M6-B07`'s player-facing pre-warm (CLUSTER-D24)

CLUSTER-D24, verbatim: "a node proactively opens (but does not use) its QUIC control-channel path to each spatially-neighboring node for every player within 2 chunks of that neighbor's region boundary, before any crossing occurs." That mechanism — opening a QUIC control-stream path ahead of an individual **player's** handoff (CLUSTER-D22) — is triggered by **player proximity to a border** and is owned by a future proxy/composition-root blueprint building on `M6-B07`'s own player-routing machinery (`M4-B08`'s `PlayerRouting`, generalized by `M6-B07` §H) — restated here only to draw the line, not to implement it. **This blueprint's own region-data pre-warm is a different mechanism, triggered by a different signal, serving a different purpose:** it is triggered by the **rebalancer's own hysteresis trend** (§C.3 step 5, firing at `PREWARM_HINT_AT_WINDOW`, before the actual migration decision at window 3), not by any player's position, and its payload is **region chunk data**, not a connection path — the intent is that a future composition-root blueprint's real `RegionPrewarmHint` implementation begins an async, best-effort, cache-only prefetch of `region`'s canonical per-chunk objects (`WORLD-D18`'s `ObjectStoreBackend` shape, not the migration-only staging path §E.2 defines — pre-warm reads the *ordinary*, already-durable canonical objects, since no staged migration blob exists yet at hint time) on the *likely* destination node, so that by the time an actual `migrate_out` call reaches phase 5 (§E.3), the destination's own eventual `restore` call has less cold-cache work to do. **Non-goal, restated:** this blueprint does not implement the prefetch itself (no `rc-chunk-storage` dependency, §A) — only the hint signal and its exact firing point.

```rust
/// The region-data pre-warm seam (§A, §G) — implemented by a future composition-root/
/// storage-integration blueprint. Best-effort: a call that never completes, or fails
/// silently, has no correctness consequence — `MigrationCoordinator::migrate_out`'s own
/// phase 5 never depends on a prior hint having fired or succeeded.
pub trait RegionPrewarmHint: Send + Sync + 'static {
    fn hint_prewarm(&self, region: RegionId, likely_dest: NodeId);
}
```

### §H — Precedence: ARCH-D6 (same-node lifecycle) vs. this blueprint's cross-node reassignment

ARCH-D6's own merge/split runs **unconditionally**, on every node, in every mode (CLUSTER-D6/D26 already establish cluster mode changes nothing about how ARCH-D6 fires — restated here only for this blueprint's own precedence statement, not re-derived). **Binding precedence rule, this blueprint's own resolution, stated plainly:** ARCH-D6 always wins; this blueprint's own rebalancer never competes with it, only defers to it or reacts to its aftermath. Concretely, two consequences already threaded through §C/§D/§E above, restated together here for clarity: (1) `evaluate_placement` (§C.3) excludes any `lifecycle_pinned` region from candidacy entirely (`RegionLoadSample.lifecycle_pinned`, §C.2) — a region currently mid-ARCH-D6-split-or-merge is never selected as a migration source, full stop, not merely deprioritized; (2) CLUSTER-D3's own ceiling fallback (§D) *is* this blueprint's one deliberate point of cooperation with ARCH-D6 — rather than inventing a second splitting mechanism, an oversized-and-hot region is handed to ARCH-D6's own existing split machinery via `request_split`, and only re-considered for migration on a later window once ARCH-D6 has already produced smaller fragments (CLUSTER-D3's own text, restated). The reverse direction — a region this blueprint has just frozen (§E.1) or is actively migrating — is likewise excluded from ARCH-D6's own eligibility, but that exclusion is enforced entirely *inside* `RegionFreezeController::freeze`'s own binding contract (a region that is not ticking cannot trigger ARCH-D6's own EWMA-based hysteresis check at all, since that check only ever runs as part of `tick_region_concurrent`'s own after-dispatch hook, `M6-B07` §D — a frozen region structurally never reaches that hook) — restated as a consequence of `freeze`'s own contract, not a second, independent enforcement mechanism this blueprint adds.

## Deliverables

### `crates/cluster/src/rebalancer/mod.rs` (new)

```rust
mod engine;
mod load;
mod policy;

pub use engine::RebalancerEngine;
pub use load::{LoadReportSink, NodeLoadReport, RegionLoadSample};
pub use policy::{
    evaluate_placement, HysteresisState, RebalancerAction, RebalancerConfig,
    MIGRATABILITY_MAX_BYTES, MIGRATABILITY_MAX_ENTITY_COUNT, PREWARM_HINT_AT_WINDOW,
    REBALANCE_EVAL_INTERVAL, REBALANCE_HYSTERESIS_DELTA_RATIO, REBALANCE_HYSTERESIS_WINDOWS,
};
```

### `crates/cluster/src/rebalancer/load.rs` (new)

`NodeLoadReport`, `RegionLoadSample`, `LoadReportSink` exactly as fixed in Context §C.2.

### `crates/cluster/src/rebalancer/policy.rs` (new)

Every constant, `RebalancerAction`, `HysteresisState`, `RebalancerConfig` (+ `Default`), and `evaluate_placement` exactly as fixed in Context §C.1/§C.3.

### `crates/cluster/src/rebalancer/engine.rs` (new)

```rust
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use crate::{ClusterNode, NodeId, RegionPrewarmHint};
use super::{HysteresisState, LoadReportSink, NodeLoadReport, RebalancerAction, RebalancerConfig};
use crate::migration::{MigrationCoordinator, MigrationError, MigrationStore, RegionFreezeController};

pub struct RebalancerEngine<F: RegionFreezeController, S: MigrationStore, P: RegionPrewarmHint> {
    // private: reports, hysteresis, config, coordinator: MigrationCoordinator<F, S>,
    // freeze_controller, prewarm
}

impl<F: RegionFreezeController, S: MigrationStore, P: RegionPrewarmHint> RebalancerEngine<F, S, P> {
    pub fn new(config: RebalancerConfig, freeze_controller: Arc<F>, store: Arc<S>, prewarm: Arc<P>) -> Self;

    /// §C.4 — pure decision only, no side effect.
    pub fn tick(&self, cluster: &ClusterNode) -> Vec<RebalancerAction>;

    /// §C.4 — performs exactly one action's side effect.
    pub async fn execute(&self, cluster: &ClusterNode, action: RebalancerAction) -> Result<(), MigrationError>;

    /// §C.5 — CLUSTER-D8's out-of-band co-location migration, bypassing the
    /// hysteresis gate entirely.
    pub async fn force_colocate(&self, cluster: &ClusterNode, quiet_region: rc_messaging::RegionId, hot_region_owner: NodeId) -> Result<(), MigrationError>;
}

impl<F: RegionFreezeController, S: MigrationStore, P: RegionPrewarmHint> LoadReportSink for RebalancerEngine<F, S, P> {
    fn ingest(&self, report: NodeLoadReport);
}
```

### `crates/cluster/src/migration/mod.rs` (new)

```rust
mod coordinator;
mod envelope;
mod traits;

pub use coordinator::{MigrationCoordinator, MigrationOutcome, MigrationStore};
pub use envelope::{
    decode_migration_envelope, encode_migration_envelope, MigrationEnvelope,
    RegionSnapshotPayload, MIGRATION_ENVELOPE_FORMAT_VERSION,
};
pub use traits::{
    FrozenRegionHandle, MigrationError, RegionFreezeController,
    RegionLifecycleState, FREEZE_TIMEOUT, STAGE_WRITE_TIMEOUT,
};
```

### `crates/cluster/src/migration/traits.rs` (new)

`MigrationError`, `FrozenRegionHandle`, `RegionLifecycleState`, `RegionFreezeController` exactly as fixed in Context §E.1, plus:

```rust
pub const FREEZE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub const STAGE_WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
```

### `crates/cluster/src/migration/envelope.rs` (new)

`MigrationEnvelope`, `RegionSnapshotPayload`, `MIGRATION_ENVELOPE_FORMAT_VERSION`, `encode_migration_envelope`, `decode_migration_envelope` exactly as fixed in Context §E.2.

### `crates/cluster/src/migration/coordinator.rs` (new)

`MigrationStore`, `MigrationOutcome`, `MigrationCoordinator` (`new`, `migrate_out`, `accept_incoming`) exactly as fixed in Context §E.3/§E.5.

### `crates/cluster/src/rebalancer_prewarm.rs` (new — kept a single small file, not a submodule, per its own single-trait size)

`RegionPrewarmHint` exactly as fixed in Context §G.

### `crates/cluster/src/lib.rs` (modify — additive; every existing M7-B02 line unchanged)

```rust
pub mod migration;
pub mod rebalancer;
mod rebalancer_prewarm;

pub use rebalancer_prewarm::RegionPrewarmHint;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46):** the test changeset is every file listed below plus every new `src/**/*.rs` file from Deliverables with every function body replaced with `todo!()` (fields, derives, doc comments, trait definitions, and every constant's *value* stay exactly as specified), plus the `lib.rs` diff. The implementation changeset (Implementation steps, below) fills in real bodies only; it must not modify any file under `crates/cluster/tests/`, must not add/remove/rename a test case, and must not weaken an assertion.

### `crates/cluster/tests/rebalancer_support/mod.rs` (test-only, not a deliverable)

`fn synthetic_report(node: &str, total_load_ms: f64, regions: &[(u64, f64, u32, u64, bool)]) -> NodeLoadReport` — builds a `NodeLoadReport` from a compact tuple list `(region_id, load_ms, entity_count, approx_bytes, lifecycle_pinned)`, for terse table-driven tests. `struct FakeFreezeController` — records every call (`freeze`/`serialize`/`resume`/`discard`/`restore`/`request_split`) into a shared `Mutex<Vec<String>>` call log plus configurable per-region failure injection (`fail_at: Option<(RegionId, &'static str)>`) and a configurable `lifecycle_state` map; its `serialize` returns a small, deterministic synthetic `RegionSnapshotPayload` (an envelope with one fake chunk blob, one fake entity blob, and whatever `pending_inbound` the test pre-seeds) rather than any real chunk/entity data. `struct FakeMigrationStore` — an in-memory `Mutex<HashMap<(RegionId, Epoch), RegionSnapshotPayload>>`, with a configurable write/read failure switch. `struct FakePrewarm` — records every `hint_prewarm` call into a shared log.

### `crates/cluster/tests/rebalancer_policy.rs`

1. `placement_picks_the_known_optimal_assignment_on_synthetic_matrices` — a table of hand-derived scenarios, each asserted independently: (a) two nodes, `A` at `100ms` (one region, `60ms`, eligible), `B` at `10ms` — delta `90 > 0.4 * 55 = 22`; drive `evaluate_placement` three times (simulating 3 windows) with an unchanged matrix; assert the first two calls return `[]`, the third returns exactly `[MigrateRegion { region: <A's region>, source: A, dest: B }]`. (b) Three nodes, `A` at `200ms` (regions `120ms`/`80ms`, both eligible), `B` at `50ms`, `C` at `40ms` — assert the eventual migration targets `dest: B` (the globally least-loaded, not merely "some other node") and `region` = the `120ms` one (max-load-eligible on the busiest node, not the `80ms` one).
2. `hysteresis_requires_three_consecutive_over_threshold_windows` — the same two-node over-threshold matrix as test 1a, but call `evaluate_placement` only twice; assert both calls return `[]` and `hysteresis.consecutive_windows_over_threshold == 2` (white-box field access — `HysteresisState`'s fields may be `pub(crate)` for exactly this test's own benefit, restated as an allowed test-only visibility choice).
3. `hysteresis_resets_when_delta_drops_back_under_threshold` — two windows over threshold, then one window with a matrix where `delta <= threshold`; assert that window's own return is `[]` and `consecutive_windows_over_threshold == 0`; a subsequent two more over-threshold windows do **not** yet trigger a migration (proving the reset, not merely "eventually fires again by coincidence").
4. `migratability_ceiling_excludes_oversized_region_and_requests_split_instead` — busiest node's only region has `entity_count: 5000` (over `MIGRATABILITY_MAX_ENTITY_COUNT`); drive to the third window; assert the result is exactly `[RequestSplit { region, node: <busiest> }]`, never a `MigrateRegion`.
5. `migratability_ceiling_falls_through_to_next_eligible_region` — busiest node owns two regions, one oversized (higher `load_ms`) and one eligible (lower `load_ms`); assert the third window's result migrates the **eligible** one, not `RequestSplit`.
6. `rebalancer_never_selects_a_lifecycle_pinned_region` — busiest node's only region has `lifecycle_pinned: true`; drive to the third window; assert the result is `[]` (not a migration, not a split-request — §H's precedence rule: a pinned region is excluded from candidacy entirely).
7. `prewarm_hint_fires_before_the_eventual_migration_decision` — the same test-1a matrix; assert window 1's own return contains exactly one `PrewarmHint { region, likely_dest }` matching what window 3's eventual `MigrateRegion` later names, and windows 2 return `[]` (no repeated hint).
8. `fewer_than_two_reporting_nodes_never_acts` — a `reports` map with exactly one entry; assert every call returns `[]` regardless of load values, and `hysteresis` stays reset.

### `crates/cluster/tests/migration_envelope.rs`

1. `migration_envelope_round_trips_byte_exact` — construct a `MigrationEnvelope` with non-trivial `chunk_snapshots`/`entity_snapshots`/`pending_inbound` (synthetic byte vectors, deliberately including an empty `Vec<u8>` entry in each list as an edge case); `encode_migration_envelope` then `decode_migration_envelope`; assert the decoded value equals the original exactly.
2. `decode_rejects_wrong_format_version` — hand-construct bytes for a version-`2` envelope (this test's own local postcard encoding, not calling `encode_migration_envelope` at all — simulating a future format this build does not understand); assert `decode_migration_envelope` returns `Err(MigrationError::EnvelopeVersion { expected: 1, found: 2 })`, never a panic, never a partial/garbage decode.
3. `decode_rejects_garbage_bytes_never_panics` — `decode_migration_envelope(&RegionSnapshotPayload(vec![0xFF, 0x00, 0x01]))` returns `Err(_)`.

### `crates/cluster/tests/migration_pending_inbound.rs`

`migration_pending_inbound_preserves_order_no_loss_no_duplication` (a `proptest!` property test) — generates a `Vec<u8>` of length `0..50` (each element a distinguishing marker byte); builds a `MigrationEnvelope` whose `pending_inbound` is exactly `markers.iter().map(|m| vec![*m]).collect()`; round-trips through encode/decode; asserts the decoded `pending_inbound`, flattened back to markers, equals the original sequence exactly — same order, same count, no duplicate, no drop. This is the envelope-level half of §E.4's "none lost, none duplicated, none reordered" property (the transport-level half is `M7-B01`'s own, already-proven, out of this blueprint's own test scope).

### `crates/cluster/tests/migration_happy_path.rs` (uses `rebalancer_support`)

Uses a real, single-node, in-memory `ClusterNode` (mirrors `M7-B02`'s own `bootstrap_flows.rs` test-1 setup exactly: `bootstrap: true`, `StorageLocation::InMemory`, `seeds: vec![]`, awaited to leadership) so `propose_assign_region` is real, not faked — only `RegionFreezeController`/`MigrationStore` are fakes.

1. `migration_happy_path_calls_freeze_serialize_stage_bump_in_order` — `MigrationCoordinator::migrate_out(&cluster, RegionId(1), NodeId::new("node-b"))`; assert `Ok(MigrationOutcome::Completed)`; assert the `FakeFreezeController`'s own call log is exactly `["freeze(1)", "serialize(1)"]` (never `resume`/`discard` — those are the destination/cleanup calls, not part of `migrate_out`'s own phase set beyond freeze+serialize per §E.3); assert `FakeMigrationStore` holds exactly one entry keyed `(RegionId(1), Epoch::FIRST)`; assert `cluster.directory().lease_of(RegionId(1)) == Some(RegionLease { region: RegionId(1), node: NodeId::new("node-b"), epoch: Epoch::FIRST })` (the epoch bump genuinely committed).
2. `migration_abort_before_epoch_bump_resumes_source_and_never_commits` — configure `FakeFreezeController` to fail `serialize` for `RegionId(2)`; call `migrate_out`; assert `Err(MigrationError::SerializeFailed(..))`; assert the call log includes `"resume(2)"` (never `"discard(2)"`); assert `cluster.directory().lease_of(RegionId(2)).is_none()` (never committed — proving phase 4 genuinely never ran).
3. `accept_incoming_reads_the_exact_epoch_the_lease_names` — pre-populate `FakeMigrationStore` with a payload at `(RegionId(3), Epoch(1))` and a **different**, distinguishable payload at `(RegionId(3), Epoch(2))`; commit `propose_assign_region(RegionId(3), ...)` twice (yielding a real committed lease at `Epoch(2)`); call `accept_incoming`; assert `FakeFreezeController`'s `restore` call received the `Epoch(2)` payload, never the `Epoch(1)` one.

### `crates/cluster/tests/migration_fencing.rs` (uses `rebalancer_support`)

`migration_fencing_rejects_resume_after_a_newer_epoch_committed` — real `ClusterNode`; `propose_assign_region(RegionId(9), NodeId::new("node-a"))` (this node), capturing a `FrozenRegionHandle { region: RegionId(9), frozen_at_epoch: Epoch::FIRST }` by hand (simulating an in-progress freeze); **then** commit a second `propose_assign_region(RegionId(9), NodeId::new("node-b"))` (simulating a concurrent takeover/reassignment superseding the frozen node while it was mid-freeze); call `self.freeze_controller.resume(handle, cluster.directory().lease_of(RegionId(9)))` against a `FakeFreezeController` whose own `resume` implementation performs exactly §F.3's binding check; assert `Err(MigrationError::FencedResume(RegionId(9)))`, and assert the call log never contains anything suggesting the region actually resumed ticking (implementer's own log-shape choice, asserted via the fake's own recorded outcome field).

## Implementation steps

1. **`rebalancer/load.rs`, `migration/traits.rs`, `migration/envelope.rs`.** Plain data types, constants, and the two encode/decode functions (straightforward `postcard::to_allocvec`/`postcard::from_bytes` calls plus the version check). Observable: these three files compile standalone; `migration_envelope.rs` tests pass.
2. **`rebalancer/policy.rs`.** Implement `evaluate_placement` exactly per Context §C.3's seven-step algorithm. Observable: `rebalancer_policy.rs`'s eight tests pass.
3. **`rebalancer/engine.rs`.** `RebalancerEngine::{new, tick, execute, force_colocate}`, `LoadReportSink for RebalancerEngine` — `tick` is a thin leadership-gate wrapper around `evaluate_placement`; `execute` dispatches each `RebalancerAction` variant to the corresponding trait call or `MigrationCoordinator::migrate_out`. Observable: compiles against `migration::MigrationCoordinator`'s already-typed signature (step 4).
4. **`migration/coordinator.rs`.** `MigrationCoordinator::{new, migrate_out, accept_incoming}` exactly per Context §E.3/§E.5's phase sequence and the `MigrationStore` trait. Observable: `migration_happy_path.rs` and `migration_fencing.rs` pass.
5. **`rebalancer_prewarm.rs`, `lib.rs`.** `RegionPrewarmHint` trait; add the three new module declarations/re-exports. Observable: `cargo build -p rc-cluster` succeeds with zero `todo!()` remaining.
6. **Run the full acceptance suite.** `cargo nextest run -p rc-cluster` — every test named in Acceptance tests passes, across all six new test files, **and** every pre-existing M7-B02 test (`raft_cluster_inprocess.rs`, `lease_fencing.rs`, `bootstrap_flows.rs`, `redb_store_roundtrip.rs`, `directory_cache_concurrency.rs`) still passes unmodified — this blueprint touches no M7-B02 file.
7. **Doctests, lints.** `cargo test --doc -p rc-cluster`; `cargo run -p xtask -- fmt-check`; `cargo run -p xtask -- lint`; `cargo run -p xtask -- lint-deps` — the last passes trivially, restated: this blueprint adds zero new `Cargo.toml` lines to `rc-cluster` at all (§A) — `lint-deps`'s own dependency-graph check has nothing new to evaluate.
8. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/cluster/tests/` (including `tests/rebalancer_support/mod.rs`) is committed first, alongside `todo!()`-stubbed (but otherwise complete: full field lists, full derive lists, full trait definitions, full doc comments, exact constant values) new `src/**/*.rs` files. The implementation changeset (Implementation steps 1–8) fills in real bodies only — it must not edit a test file, must not add/remove/rename a test case, and must not weaken an assertion (in particular, `migration_fencing.rs`'s exact epoch-supersession check and `rebalancer_policy.rs`'s exact hysteresis-reset assertions must survive unchanged).

(b) **Zero new external or intra-workspace dependencies.** `rc-cluster`'s `Cargo.toml` is not modified by this blueprint at all — every type this blueprint's deliverables use (`serde`, `postcard`, `thiserror`, `tracing`, `parking_lot`, `rc_messaging::RegionId`) is already present from M7-B02. Do not add `rc-scheduler`, `rc-chunk-storage`, `rc-transport-net`, `tokio` beyond what M7-B02 already pins, or any other crate under any circumstance — this is the direct, binding consequence of §A's own dependency-direction resolution, not merely a preference.

(c) **No Mojang or third-party reimplementation code.** This blueprint is pure distributed-systems/scheduling-policy logic derived solely from `13-cluster-architecture.md`'s CLUSTER-D2–D5/D8/D19/D24 and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30).

(d) **No `unsafe` code.** Every type and function in this blueprint's Deliverables is implementable in 100% safe Rust.

(e) **Scope boundary — do not implement beyond this blueprint's one crate (§A, restated).** This blueprint does not implement: a real `RegionFreezeController` bridging to `rc-scheduler`'s `RegionManager`/`EdfScheduler` (a future composition-root-extension blueprint); a real `MigrationStore` bridging to `rc-chunk-storage`'s not-yet-built `ObjectStoreBackend`/WORLD-D20 staging path (a future `03-world-chunks-persistence.md`-owned blueprint, per `M7-B02`'s own identical exclusion); a real `RegionPrewarmHint` implementation or the `DirectoryCache`-diffing watch loop that decides when to call `accept_incoming` (both a future composition-root blueprint, §E.5); CLUSTER-D16's own takeover-algorithm decision logic (a sibling blueprint, §F.2); CLUSTER-D22's player-connection handoff or CLUSTER-D24's player-facing pre-warm (both `M6-B07`/a future proxy blueprint's own scope, §G's own boundary statement). Do not add placeholder implementations of any of these as a shortcut — every trait boundary named above is fixed precisely by this blueprint; fulfilling it for real is not.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-cluster --all-features
cargo nextest run -p rc-cluster
cargo test --doc -p rc-cluster
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-cluster` runs 8 (`rebalancer_policy.rs`) + 3 (`migration_envelope.rs`) + 1 (`migration_pending_inbound.rs`, one property-test case regardless of internal proptest-generated input count, consistent with this corpus's own established framing) + 3 (`migration_happy_path.rs`) + 1 (`migration_fencing.rs`) = 16 test cases named in Acceptance tests, plus every pre-existing M7-B02 test unmodified — all pass. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
