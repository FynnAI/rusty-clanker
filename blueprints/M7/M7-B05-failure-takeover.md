# M7-B05 — Node-Failure Takeover (`rc-cluster::takeover`)

| Field | Content |
|---|---|
| ID | M7-B05 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | M7-B02 (`rc-cluster` — `ClusterNode`, `DirectoryCache`, `RegionLease`, `Epoch`, `ClusterConfigEpoch`, `NodeId`, `ClusterError`, `HealthMonitor`/`NodeHealthEvent`/`HealthMonitorConfig`, `ClusterAdminApi`, `TypeConfig` — every one restated exactly below; this blueprint is the "future takeover-orchestration blueprint" M7-B02 §A item 2 and §H name explicitly as the intended consumer of `ClusterNode::take_health_events()`). **M7-B03 and M7-B04 exist as merged siblings but are deliberately not formal prerequisites of this blueprint** — this blueprint takes no Cargo dependency and no public-API dependency on either (Context §A explains why the split lands entirely in `rc-cluster`). It derives every fact it needs directly from `docs/planning/03-world-chunks-persistence.md` (WORLD-D17/D18/D19/D20/D22/D23, already fully pinned with concrete Rust signatures — restated in full in Context §F) and `docs/planning/13-cluster-architecture.md` (CLUSTER-D2/D3/D16/D17/D18/D19/D20–D24, restated throughout), and it defines one narrow, consumed trait (`RegionResumeHandler`, Context §F/§G) at the exact boundary where a real shared-storage implementation (`ObjectStoreBackend`, WORLD-D17) would plug in — the identical "define the trait here, implement it in a sibling blueprint" split M7-B01 already used for `NodeDirectory` and M7-B02 already used for `RaftNetworkFactory`/`JoinClient`. M7-B04 §G.4 ("manifest-guided takeover-resume, the exact sequence") independently derives, from the same planning-doc facts, precisely the read-side sequence this blueprint's own `RegionResumeHandler::resume_region` contract needs a future implementation to perform — the two blueprints are confirmed mutually consistent, cross-referenced in full in Context §F. Whoever implements `RegionResumeHandler` against `ObjectStoreBackend` follows that sequence; whoever writes the still-missing composition-root/proxy blueprints wires this blueprint's `TakeoverOrchestrator`/`DirectoryReconciler` into a running process (Context §A). |
| Implements | CLUSTER-D16 (node-failure takeover, this blueprint's primary subject — restated and made concrete in full). CLUSTER-D2 (least-loaded placement strategy, restated and **narrowed** for the immediate-takeover case specifically, with the narrowing justified as a finding, Context §D). CLUSTER-D15 (failure detection — consumed via M7-B02's `HealthMonitor`, not reimplemented). CLUSTER-D17 (durability model / data-loss bound — restated honestly, Context §H). CLUSTER-D18 (shared-storage single-writer requirement — the storage-side half of the fencing proof, Context §L). CLUSTER-D19 (epoch/lease fencing — made concrete for the zombie-node case, Context §L). CLUSTER-D20/D21/D23 (proxy connection-termination/control-channel placement — restated as the **contract** a future `rc-proxy` blueprint must satisfy for convergence to be safe, Context §J/§K; this blueprint does not implement `rc-proxy`). CLUSTER-D22 (planned-handoff protocol — restated as the baseline this blueprint's own failure-specific extension deliberately diverges from, with the divergence justified, Context §K). CLUSTER-D7 (cross-node latency budget — restated as one input to the takeover-time budget decomposition, Context §C). WORLD-D17/D18/D19/D22/D23 (storage-backend trait, object layout, region manifest, load-pipeline routing, save pipeline — restated exactly as this blueprint's `RegionResumeHandler` contract's own expected implementation strategy, Context §F). ARCH-D19 (per-region tick-duration EWMA — restated only to explain why it is **not** used as this blueprint's placement signal, Context §D). WS-D3 (dependency-graph hard rules — this blueprint's own resolution of the `cluster --> sched` diagram/prose inconsistency M7-B02 §A item 2 flagged, Context §D). TEST-D45/D46 (test-first changeset boundary). TEST-D50 (CI-is-authority). ASSET-D18/D19/D30 (no Mojang/third-party source consulted). |
| Crates touched | `rc-cluster` (`crates/cluster/`) only — one new file, `crates/cluster/src/takeover.rs`, plus an additive `pub use` line in `crates/cluster/src/lib.rs`, plus new test files under `crates/cluster/tests/` and one additive extension to the existing `crates/cluster/tests/support/mod.rs` (a new `FakeResumeHandler`/`FakeSharedStorage` test double, appended, never replacing any of M7-B02's own existing support types). No workspace-root `Cargo.toml` edit — this blueprint introduces zero new external dependencies (every type it needs — `tokio`, `parking_lot`, `thiserror`, `tracing` — is already in `rc-cluster`'s own `Cargo.toml` from M7-B02). Not `rc-chunk-storage`, not `rc-transport-net`, not `rusty-clanker-server`, not `rc-proxy` — every one of those is a separate, not-yet-written sibling blueprint's job, named precisely at each point this blueprint stops short of it (Context §A). |
| Estimated scope | L, explicitly oversized against `00-blueprint-spec.md`'s general sizing guideline — the same class of stated exception `M6-B07`/`M7-B02` already established. CLUSTER-D16's takeover sequence has eight genuinely interdependent sub-problems (failure declaration, placement, raft commit, resume, fencing, in-flight-message handling, directory/proxy convergence, player fate) that share one state machine and one set of acceptance tests; splitting them across multiple blueprints would force each one to restate the same `ClusterNode`/`DirectoryCache`/`RegionLease` API surface from scratch while leaving the actual failure-to-resumed-and-serving path unproven by any single blueprint's own tests. |

## Goal & Done definition

Give `rc-cluster` the takeover orchestration CLUSTER-D16 describes and M7-B02 deliberately left unbuilt: on raft-detected node failure, reassign the dead node's region leases to live survivors (least-loaded by current region count, Context §D), and — on every node, symmetrically — react to a directory change that makes *this* node a region's new owner by loading that region from shared, durable storage through a resume path only *this* blueprint defines the contract for (`RegionResumeHandler`, Context §F), beginning to tick only once the epoch-fenced lease is locally confirmed current. Concretely: (1) `TakeoverOrchestrator` — consumes `ClusterNode::take_health_events()`, and on every `NodeHealthEvent::Failed`, re-derives the dead node's currently-owned regions fresh from `DirectoryCache` (never from cached state — the re-entrancy property that makes repeated/overlapping failures safe, Context §M) and calls `ClusterNode::propose_assign_region` once per affected region against a pluggable `PlacementStrategy` (default: fewest-currently-owned-regions, deterministic tie-break); (2) `DirectoryReconciler` — a symmetric, per-node background watcher that diffs successive `DirectoryCache` snapshots against this node's own identity and calls `RegionResumeHandler::resume_region`/`evict_region` exactly once per gain/loss, idempotently; (3) the `RegionResumeHandler` trait itself, the exact contract a future shared-storage-backend blueprint's implementation must satisfy (manifest-guided load, WORLD-D19); (4) an honest, precise takeover-time budget decomposition and data-loss bound, cited from already-fixed constants; (5) a precise, restated answer for what a player on the failed node experiences, distinct from — and honestly narrower a guarantee than — CLUSTER-D22's zero-disconnect planned-handoff case; (6) the zombie-node fencing proof sketch, made concrete against `RegionLease::is_current` and a storage-side conditional-write contract.

Done when:

- [ ] `cargo build -p rc-cluster --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-cluster`.
- [ ] Every pre-existing `rc-cluster` test (M7-B02's own five test files) still passes, byte-for-byte unmodified.
- [ ] `three_node_kill_resumes_regions_on_survivor_within_bounded_window` and `three_node_kill_preserves_directory_state_across_the_gap` (the "3-node in-process kill tests" the task names) both pass.
- [ ] `zombie_node_writes_are_rejected_after_reassignment` passes: a revived stale-epoch "writer" is rejected by the fake shared-storage double's own conditional-write check, proving the storage-side fencing half of CLUSTER-D19 (this blueprint does not, and cannot, prove the proxy-routing half without `rc-proxy` — restated honestly as a scope boundary, Context §L).
- [ ] `unaffected_region_directory_state_is_uninterrupted_through_a_sibling_failure` passes (the crate-appropriate proxy for acceptance criterion 2's "zero interruption," Context §A/§J).
- [ ] `concurrent_directory_reads_never_observe_a_stale_route_as_current` passes (the proxy-convergence-race primitive this crate can actually prove, Context §J).
- [ ] `cascading_failure_during_takeover_converges_every_region_to_one_live_owner` passes (repeated-failure / takeover-during-takeover safety).
- [ ] `least_region_count_placement_is_deterministic_and_balances_across_multiple_regions` (a `proptest!` property test) passes.
- [ ] `directory_reconciler_is_idempotent_against_already_resident_regions` and `directory_reconciler_evicts_on_unassign_and_on_reassignment_away` both pass.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-cluster` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). `lint-deps` passes trivially — this blueprint adds no new crate and no new dependency-graph edge at all (Context §D resolves the `cluster --> sched` question by needing no such edge).

## Context (self-contained)

### §A — Scope boundary: what this blueprint builds, and its relationship to three sibling blueprints that now exist in the corpus

`rc-cluster`'s own Crate Manifest responsibility (`12-workspace-structure.md`) already names "failure detection/takeover" as content this crate owns — this blueprint is the concrete fulfillment of that line, continuing directly from M7-B02's own explicit deferral ("this blueprint builds the primitives CLUSTER-D16 depends on... but does not implement... the decision of which live node gets the failed node's regions... a sibling blueprint's job"). Three genuinely separate blueprints sit adjacent to this one, each now merged in the corpus, named precisely so no reader mistakes this blueprint for covering them — and this blueprint takes a Cargo dependency on none of the three (Context, "Why this crate," below):

1. **M7-B04, the shared-storage-backend blueprint** (`ObjectStoreBackend`, `rc-chunk-storage::cluster_storage`, WORLD-D17/D18). `RegionResumeHandler` (§F) is the exact trait boundary M7-B04's implementation plugs into, defined here because CLUSTER-D16's takeover semantics (what "resume" must observably do, in what order, under what fencing) are this blueprint's subject, not that one's. Every fact this blueprint states about *how* a resume implementation would work (manifest-guided load, WORLD-D19) is already fully pinned, with concrete Rust signatures, in `03-world-chunks-persistence.md` — restated in §F, not invented here — and independently confirmed consistent with M7-B04 §G.4's own derivation of the identical read-side sequence from the same source facts (§F, cross-referenced in full).
2. **M7-B08, the cluster-mode composition-root blueprint** (`rusty-clanker-server`, mirroring `M6-B07` for cluster mode — CLUSTER-D26/D27's role/config wiring, `NetworkTransport`/`ClusterNode` construction, the real `openraft::RaftNetworkFactory`/`JoinClient` wire implementations M7-B02 §A item 1 named). This blueprint's `TakeoverOrchestrator`/`DirectoryReconciler` are constructed and `spawn`ed by that blueprint's own startup sequence, exactly as `EdfScheduler::run` is `M6-B07`'s own composition root's job to call, not `rc-scheduler`'s own job to self-drive.
3. **M7-B06, the proxy blueprint** (`rc-proxy`, CLUSTER-D20–D24). This blueprint restates, as a **contract** (§J/§K), exactly what a conforming `rc-proxy` implementation must do for takeover-time routing convergence and player-connection continuity to hold — it does not implement `rc-proxy` itself, and every acceptance test in this blueprint that would need a real proxy is instead written against the crate-local primitive that makes the proxy's own eventual behavior safe (directory-read consistency, epoch fencing), restated explicitly at each such test.

**Why this crate, not `rusty-clanker-server` or a new crate.** `WS-D3` rule 2 forbids `rc-scheduler`/`rc-mechanics` from any edge, in either direction, to `rc-cluster`. `12-workspace-structure.md`'s own Dependency Graph diagram nonetheless draws `cluster --> sched` — flagged by M7-B02 §A item 2 as "a genuine inconsistency between that document's prose and its own diagram... for whoever authors the rebalancer/takeover blueprint to reconcile." **This blueprint's own resolution, stated as a finding for `12`'s next revision:** the diagram edge is not needed and should be removed. CLUSTER-D2's own cited precedent (Akka Cluster Sharding's default `LeastShardAllocationStrategy`) picks the target with the fewest *currently-assigned* shards as its baseline behavior — a signal this crate already has in full, locally, with zero network round trip, from `DirectoryCache::snapshot()` (Context §D). No `rc-scheduler` EWMA data is read anywhere in this blueprint's own placement path, so no `rc-cluster --> rc-scheduler` edge is created, and none is needed. CLUSTER-D2's own EWMA-weighted refinement remains exactly what M7-B02 already scoped it as — a separate, not-yet-written *periodic* rebalancer blueprint's job — and if that future blueprint does need cross-node EWMA visibility, it is that blueprint's job to design the (still-unbuilt) cross-node load-reporting mechanism, not a retroactive amendment to this one.

### §B — Failure detection → declaration flow, restated exactly (CLUSTER-D15, M7-B02 §H)

CLUSTER-D15's mechanism, restated: raft's own leader-driven `AppendEntries` heartbeat and election-timeout are the **sole** failure-detection mechanism — no second gossip/phi-accrual/SWIM layer, and this blueprint adds none. M7-B02's `HealthMonitor` already turns that mechanism into the one event this blueprint consumes: `NodeHealthEvent::Failed { id }`, emitted **only on whichever node currently holds raft leadership** — a follower's own `HealthMonitor` instance observes `RaftMetrics.replication == None` (M7-B02 §H: "`Some()` only when this node is leader") and therefore never emits `Failed`/`Recovered` itself, only `LeadershipChanged`. This one fact is load-bearing for this blueprint's design and is restated here precisely because it resolves an ambiguity M7-B02 §I item 6 left open ("if `client_write` does not auto-forward [to the leader], callers must retry"): **`TakeoverOrchestrator` never needs that retry path for its own `propose_assign_region` calls under ordinary single-failure operation**, because the only node whose orchestrator instance ever *sees* a `Failed` event is, by `HealthMonitor`'s own construction, the very node `client_write` would need to reach anyway. The one case this guarantee does not cover — leadership changing *mid-processing* of an already-received `Failed` event — is handled by this blueprint's own re-entrant design (§M), not by a retry-against-a-different-leader loop.

**No separate "declaration quorum."** `HealthMonitorConfig`'s own debounce (`failure_threshold`, restated in §C below) already is the declaration mechanism — a peer whose replicated `LogId` has not advanced for longer than that duration is declared `Failed`. `TakeoverOrchestrator` reacts to a received `Failed` event with **zero additional delay** — layering a second debounce on top would both violate CLUSTER-D15's "no second detection layer" intent and needlessly widen CLUSTER-D16's own "invoked immediately rather than waiting for the next 30s window" requirement.

### §C — Takeover-time budget decomposition, pinned from already-fixed constants

Every constant below is `RaftTuning::default()`/`HealthMonitorConfig::from_raft_tuning`'s own seed default, restated verbatim from M7-B02 §H/Deliverables: `heartbeat_interval_ms: 250`, `election_timeout_min_ms: 750`, `election_timeout_max_ms: 1500`, `HealthMonitorConfig { failure_threshold: 2 * election_timeout_max_ms = 3000ms, poll_interval: 200ms }`. `11-roadmap-milestones.md`'s own M7 acceptance criterion 2 (restated in this task's own briefing) names the budget as **"raft-election-timeout + takeover window"** — not CLUSTER-D22's tight ≤2-tick(100ms) planned-handoff figure — and this blueprint's own decomposition below is the concrete arithmetic behind that phrase, split into the two genuinely different cases §B's leader-only-detection fact creates:

- **Case 1 — a non-leader node fails.** The current leader's `HealthMonitor` must first observe the dead peer's replication stall for the full `failure_threshold` window before emitting `Failed` — **up to 3000ms**, dominating this case's budget (a real, seed-default, calibration-pending number this blueprint neither shrinks nor pretends is smaller — restated honestly per this task's own instruction). Then: one `propose_assign_region` raft commit per affected region (bounded, in a co-located topology, by CLUSTER-D7's own ≤30ms p99 figure — reused here as the best available proxy for "one raft round trip between co-located nodes," since CLUSTER-D7 itself is scoped to `RegionMessage`/data-plane traffic, not raft RPCs specifically; restated as a **moderate-confidence reuse**, not a literal re-derivation, Context §N). Then: resume I/O time (§F) — **workload-dependent, not a number this document can pin**: bounded by shared-storage round-trip latency times the region's own chunk-object count, which this blueprint states honestly rather than inventing a false constant for.
- **Case 2 — the leader itself fails.** No `HealthMonitor` anywhere is watching it (§B) — instead, an ordinary raft election must complete first, bounded by the election-timeout range itself (**750–1500ms**, raft's own randomized backoff), after which the *newly elected* leader treats the deposed leader as immediately failed with **no additional `failure_threshold` wait** — restated as this blueprint's own deliberate optimization, justified precisely: a leader election only happens because the old leader already stopped being heard from, so requiring a *second*, independent confirmation window before reacting would only add latency without adding safety. `propose_assign_region` and resume I/O time then apply exactly as Case 1.

Both cases are **materially larger** than CLUSTER-D22's planned-handoff budget, and this blueprint states that difference as load-bearing, not incidental: a planned handoff moves a live entity between two already-healthy nodes inside two ticks; failure takeover first has to *notice* a node stopped existing, which structurally cannot be faster than raft's own detection mechanism allows.

### §D — Surviving-node selection policy: least-loaded by current region count

CLUSTER-D16's own text reuses CLUSTER-D2's "least-loaded strategy, invoked immediately." CLUSTER-D2's own decision text describes an EWMA-tick-duration-sum signal; its own **rationale** cites Akka Cluster Sharding's `LeastShardAllocationStrategy` as the validated pattern being mirrored — and that cited strategy's own real default behavior (a verifiable, independent fact about Akka, not an invention) picks the target with the fewest *currently-assigned* shards, not a CPU-weighted figure. **This blueprint's own resolution, stated as a deliberate, justified narrowing for the immediate-takeover case specifically** (not a silent rewrite of CLUSTER-D2's general text, which continues to govern the separate, not-yet-written periodic rebalancer): placement for CLUSTER-D16's immediate reassignment uses **region count**, read directly from `DirectoryCache::snapshot()` — zero network round trip, zero cross-node data-visibility problem, and available the instant this blueprint's own code runs. Two independent reasons this is the right choice for *this* path specifically, both restated precisely: (1) ARCH-D19's own EWMA is tracked **per-region, locally, by `rc-scheduler`** — no mechanism anywhere in the corpus makes one node's EWMA visible to another node's placement decision, and inventing one here would mean designing an entire cross-node load-reporting protocol as a side effect of a failure-recovery blueprint, itself a scope violation of this blueprint's own narrow subject; (2) takeover is time-critical (§C) — blocking a life-safety-critical reassignment decision on a fresh cross-node poll would directly work against the very budget this blueprint is trying to minimize.

```rust
/// Picks the least-loaded of `candidates` by CURRENT region count (this blueprint's
/// own resolution of CLUSTER-D2/D16's "least-loaded strategy" for the immediate-
/// takeover case, Context §D — NOT ARCH-D19's per-region EWMA, which no node can see
/// cross-node without machinery this blueprint deliberately does not build).
/// Deterministic: ties broken by lowest `NodeId` (`Ord`, string comparison, already
/// derived on `NodeId`, M7-B02 §B/Deliverables) so two orchestrator runs presented
/// with the identical `counts` snapshot always pick the identical target.
pub trait PlacementStrategy: Send + Sync + 'static {
    fn select(&self, candidates: &[NodeId], current_counts: &std::collections::HashMap<NodeId, usize>) -> Option<NodeId>;
}

/// The default, and — until a future periodic-rebalancer blueprint supplies an
/// EWMA-weighted alternative — the only implementation this blueprint ships.
pub struct LeastRegionCountStrategy;

impl PlacementStrategy for LeastRegionCountStrategy {
    fn select(&self, candidates: &[NodeId], current_counts: &std::collections::HashMap<NodeId, usize>) -> Option<NodeId>;
    // Body: candidates.iter().min_by_key(|n| (current_counts.get(n).copied().unwrap_or(0), (*n).clone()))
    //   .cloned() — `None` only if `candidates` is empty (Context §E's "no live nodes" case).
}
```

### §E — The takeover sequence, exactly

Triggered by one `NodeHealthEvent::Failed { id: dead }` (§B), running entirely inside `TakeoverOrchestrator` (Deliverables), on whichever node's `HealthMonitor` emitted it (§B's guarantee: always the current leader):

1. **Re-derive affected regions fresh, never from cached state.** `let affected: Vec<RegionId> = node.directory().snapshot().into_iter().filter(|(_, entry)| entry.node == dead).map(|(region, _)| region).collect();` — reading live at the moment of handling, not at the moment the `Failed` event was *queued* (the mpsc channel may have delivered it late under load), so a region `dead` already lost to some *earlier* reassignment (this blueprint's own re-entrancy property, §M) is correctly excluded without any extra bookkeeping. If `affected.is_empty()`, emit `TakeoverEvent::FailureObserved { dead, affected_regions: vec![] }` and return — `dead` owned nothing needing reassignment (already handled, or never owned anything).
2. **Enumerate live candidates.** Read the current raft voter membership from `node.raft().metrics().borrow().membership_config` (an already-warm, non-blocking read — `openraft`'s own metrics watch channel, no raft round trip; **moderate-confidence flag**, Context §N item 1: the exact accessor chain to enumerate voter `NodeId`s from `membership_config` should be confirmed against `openraft` 0.9.25's own `StoredMembership`/`Membership` docs at implementation time), excluding `dead` itself. If this set is empty, emit `TakeoverEvent::NoLiveNodesAvailable { dead, unassigned_regions: affected }` and stop — a total-cluster-loss condition this blueprint can only report, never resolve (there is no live node left to reassign to).
3. **Assign regions one at a time, updating a local running tally.** Seed `counts: HashMap<NodeId, usize>` from the **current** `snapshot()` (excluding `dead`'s own now-vacated entries, which are being reassigned, not counted against anyone yet). For each region in `affected`, sorted by `RegionId` (deterministic iteration order — restated because it makes every test in this blueprint reproducible): call `placement.select(&live_candidates, &counts)`; on `Some(target)`, call `node.propose_assign_region(region, target).await`; on `Ok(response)`, increment `counts[target] += 1` (so the *next* region in this same pass sees the update — the mechanism that actually spreads multiple regions evenly rather than piling them onto one survivor) and emit `TakeoverEvent::RegionReassigned { region, from: dead, to: target, new_epoch: response.new_epoch.expect("AssignRegion always returns Some") }`; on `Err(ClusterError::NotLeader)`, emit `TakeoverEvent::ReassignmentDeferred { region, dead, reason: "lost leadership mid-pass".into() }` and **stop this whole pass** (§B/§M: the newly elected leader's own orchestrator instance will independently re-derive and retry — see step 1's re-entrancy, which is exactly what makes stopping here safe rather than lossy); on any other `Err`, emit a `ReassignmentDeferred` with that error's message and continue to the next region (one region's proposal failure does not block the rest — best-effort, not all-or-nothing).
4. **The raft commit itself is the "declaration."** There is no separate "declare node dead" step distinct from the `AssignRegion` commits themselves — the moment the first such commit lands, `DirectoryCache` on every node (bounded by §C's own commit-propagation figure) reflects the new owner, which is simultaneously the fencing mechanism (§L) and the convergence signal every other component (§G's `DirectoryReconciler`, a future `rc-proxy`'s own directory watch) reacts to.

### §F — `RegionResumeHandler`: the region-resume contract, manifest-guided load restated exactly (WORLD-D17/D18/D19/D22/D23)

CLUSTER-D16, verbatim: "Each newly-assigned node loads its new region(s) from the last durably-persisted snapshot on shared storage — never from the dead node's memory, which is gone." `03-world-chunks-persistence.md` already fully specifies, with concrete Rust signatures, exactly what a conforming implementation of this contract does — restated here in full so this blueprint needs nothing further from that document at implementation time:

**The `ChunkStorageBackend` trait, restated exactly (WORLD-D17, confirmed identical in `M2-B03`'s own Deliverables):**

```rust
pub trait ChunkStorageBackend: Send + Sync + 'static {
    fn read_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>) -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>) -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}
```

`epoch: Option<u64>` is already, deliberately, a bare primitive — not a `rc_cluster::Epoch` — precisely so `rc-chunk-storage` (a `SimServer`-group crate, WS-D3 rule 2) never needs a dependency on this `NetServer`-group crate. **This blueprint's own resolution, restated as the calling convention every consumer of `RegionResumeHandler` must follow:** a conforming implementation converts `RegionLease.epoch.0` (this crate's own concrete `u64`, M7-B02 §F) into that bare `Some(u64)` argument at the call site — the identical calling convention M7-B02 §F already flagged as "a future shared-storage-backend blueprint... passes `Some(lease.epoch.0)`," restated here as binding rather than merely anticipated.

**`RegionManifest`, restated exactly (WORLD-D19) — the dirty-region tracking mechanism this blueprint's own resume path reads on load:**

```rust
// Restated from 03-world-chunks-persistence.md's own decision text, not redefined
// by this blueprint — the exact struct a resume-handler implementation constructs
// by reading one postcard-encoded object at a well-known key,
// world/<dim>/manifests/<region_id>.postcard.
struct RegionManifest {
    region_id: u64,                                    // RegionId's own concrete u64
    epoch: u64,                                          // the epoch this manifest was last written under
    last_saved_tick: u64,
    chunk_object_versions: std::collections::HashMap<ChunkKey, ObjectVersion>,
}
```

**A finding, stated plainly rather than silently resolved either way:** WORLD-D20's own decision text groups "CLUSTER-D2's... planned rebalancing[] step, **and** CLUSTER-D16's takeover bootstrap" as both reading from the same *transient staging* snapshot path (`world/<dim>/staging/<region_id>-<epoch>.postcard`, a `postcard`-encoded, already-decoded `ChunkSnapshot`). **This blueprint reads that pairing narrowly, and states why:** a staging snapshot can only exist if a live, cooperating source node wrote it as part of a *controlled* flush — exactly what CLUSTER-D2/D3's planned migration does, and exactly what a node that has just **crashed** cannot do. This blueprint's own `RegionResumeHandler` contract therefore specifies the **WORLD-D19/WORLD-D22 canonical-object path** — fetch the region's `RegionManifest`, then fetch exactly the chunk objects it lists (WORLD-D18's per-`(ChunkKey, RegionFileKind)` objects), then route each through the identical Stage-1 structural-command insertion point WORLD-D22 already defines for an ordinary cold chunk-load ("a completed chunk is delivered as an ordinary structural command consumed at a region's Stage 1... one insertion point for both 'loaded from disk' and 'freshly generated' chunks") — never the staging-snapshot fast path, which remains exclusively a *planned*-migration optimization for a separate, not-yet-written blueprint to build. A future revision of `13`/`03` may want to correct WORLD-D20's own text to reflect this narrower reading; this blueprint does not edit that document, only states its own binding interpretation.

```rust
/// This blueprint's one new external-facing error type — every fallible
/// `RegionResumeHandler` method returns this. Never `ClusterError` (§A: this trait
/// is implemented outside `rc-cluster`, by a future storage-integration blueprint —
/// its own error type must not force that blueprint to depend on `rc-cluster` just
/// to report a storage failure).
#[derive(Debug, thiserror::Error)]
pub enum ResumeError {
    #[error("no RegionManifest found for region {region:?} in shared storage")]
    ManifestNotFound { region: rc_messaging::RegionId },
    #[error("shared-storage read failed: {0}")]
    Storage(String),
    #[error("presented epoch is no longer current — a newer AssignRegion has already superseded this lease")]
    EpochStale,
}

/// The exact contract CLUSTER-D16's "loads from the last durably-persisted snapshot"
/// resolves to (Context §F) — implemented by a future shared-storage-integration
/// blueprint (built on `ObjectStoreBackend`/`ChunkStorageBackend`, WORLD-D17, plus
/// `rc-scheduler`'s `RegionManager::spawn_region`/`ChunkLifecycleManager` pair,
/// M6-B07 §C.2's own established construction sequence — none of which `rc-cluster`
/// itself may depend on, WS-D3 rule 2). `rc-cluster` only defines and consumes this
/// trait, exactly the `NodeDirectory`/`JoinClient` pattern M7-B01/M7-B02 already
/// established.
pub trait RegionResumeHandler: Send + Sync + 'static {
    /// Load `lease.region` from shared storage under `lease.epoch` (manifest-guided,
    /// WORLD-D19/D22, Context §F) and begin ticking it locally once loaded. Every
    /// storage read/write this call performs MUST present `Some(lease.epoch.0)` as
    /// the `ChunkStorageBackend` epoch argument (Context §F's calling convention).
    /// Idempotent: a second call for an already-resident `lease.region` under the
    /// SAME epoch is a no-op returning `Ok(())`; a second call under a NEWER epoch
    /// re-resumes (the epoch bump itself is a legitimate reassignment, Context §M).
    fn resume_region(&self, lease: crate::types::RegionLease) -> Result<(), ResumeError>;
    /// Tear down local ticking/registration for `region` — this node no longer owns
    /// it (an `UnassignRegion` commit, or an `AssignRegion` naming a different node).
    /// Idempotent: a region not currently resident is a no-op.
    fn evict_region(&self, region: rc_messaging::RegionId);
    /// Non-blocking, local-only query — never touches storage. Lets
    /// `DirectoryReconciler` (Context §G) skip a redundant `resume_region` call for a
    /// region this node already loaded through some other path (ordinary startup
    /// bootstrap, M6-B07 §B — never itself a `rc-cluster` concern, only an input this
    /// trait's own idempotency depends on).
    fn is_resident(&self, region: rc_messaging::RegionId) -> bool;
}
```

**Cross-reference, confirmed consistent:** M7-B04 §G.4 ("manifest-guided takeover-resume, the exact sequence") independently derives, from the same `03-world-chunks-persistence.md` source facts restated above, precisely the read-side sequence a conforming `RegionResumeHandler::resume_region` implementation performs: read `RegionManifest`, do not eagerly bulk-load its chunk list, and let the first write's CAS confirm the new epoch as current (M7-B04 §G.4 step 4). The two blueprints were derived independently against the same planning-doc facts and are mutually consistent — a future implementation of this trait against `ObjectStoreBackend` follows M7-B04 §G.4's sequence directly, with `resume_region` calling `ObjectStoreBackend::read_region_manifest` and returning `ResumeError::ManifestNotFound` only when that call returns `Ok(None)` for a region this node's own directory entry says it now owns.

### §G — `DirectoryReconciler`: the symmetric, per-node resume/evict watcher

Runs identically on **every** node (not only the leader — the leader-only asymmetry belongs to `TakeoverOrchestrator`/§B, not here), because directory *convergence* — noticing "I now own a region" or "I no longer own a region" — is a property every node needs regardless of raft role, whether the change came from this blueprint's own takeover path, a future planned-migration blueprint, or ordinary startup bootstrap naming this node directly.

```rust
pub struct DirectoryReconcilerConfig {
    /// How often the reconciler diffs `DirectoryCache::config_epoch()` against its
    /// own last-seen value. Seed default `Duration::from_millis(50)` — one
    /// `SERVER_TICK_PERIOD` (M0-B04's own already-fixed constant, reused here for
    /// consistency, not re-derived), well inside CLUSTER-D7's ≤30ms-per-hop latency
    /// budget's own order of magnitude and cheap relative to a directory read.
    pub poll_interval: std::time::Duration,
}

impl Default for DirectoryReconcilerConfig {
    fn default() -> Self { Self { poll_interval: std::time::Duration::from_millis(50) } }
}

pub struct DirectoryReconciler { /* private */ }

impl DirectoryReconciler {
    /// Spawns a background `tokio` task (the composition root's own runtime, per
    /// ARCH-D21 — this crate's existing convention, M7-B02's `HealthMonitor::spawn`
    /// uses the identical pattern) that polls `node.directory()` at
    /// `config.poll_interval`, diffing successive `snapshot()`s restricted to
    /// entries where `entry.node == node.node_id()` against the reconciler's own
    /// prior such set: a region newly appearing in that filtered set (or reappearing
    /// under a strictly greater epoch than last observed) calls
    /// `handler.resume_region` UNLESS `handler.is_resident(region)` already reports
    /// `true` under the identical epoch (idempotency, Context §F); a region present
    /// in the reconciler's own prior filtered set but absent (or now owned by a
    /// different `NodeId`) from the current one calls `handler.evict_region`. Skips
    /// its own diff entirely (a cheap no-op poll) whenever
    /// `DirectoryCache::config_epoch()` is unchanged since the last poll — avoiding a
    /// full `snapshot()` clone on every idle poll tick.
    pub fn spawn(
        config: DirectoryReconcilerConfig,
        node: std::sync::Arc<crate::ClusterNode>,
        handler: std::sync::Arc<dyn RegionResumeHandler>,
        on_event: std::sync::Arc<dyn Fn(TakeoverEvent) + Send + Sync>,
    ) -> DirectoryReconcilerHandle;
}

pub struct DirectoryReconcilerHandle { /* private */ }

impl DirectoryReconcilerHandle {
    /// Signals the background task to stop after its current poll iteration.
    pub fn shutdown(&self);
    /// Blocks until the spawned task has actually stopped.
    pub async fn join(self);
}
```

### §H — Replay/loss semantics: the exact bound, stated honestly

CLUSTER-D17, restated verbatim: cluster mode's data-loss window on node failure is bounded by **time since that region's last successful persisted save**, never a stronger guarantee — `NetworkTransport` (M7-B01) upholds ARCH-D29's exactly-once/FIFO contract only "while both endpoints remain alive"; anything in flight to or from a node at the moment it dies is lost with **no cross-restart delivery guarantee**. This blueprint adds nothing to that bound and does not attempt to shrink it — the honest consequence, restated plainly rather than glossed: **every tick of simulation this region completed after its last successful WORLD-D23 autosave, and before the node died, is gone.** CLUSTER-D17's own recommendation (≤30s per dirty region in cluster mode, versus vanilla's 5-minute/6000-tick default) is this blueprint's own assumed operating configuration — a cluster operator running the vanilla-default 5-minute interval in cluster mode is accepting a materially larger loss window, and this blueprint does not silently narrow that fact. **What this specifically means for the resumed region's observable state:** block changes, redstone state, mob positions, and any other region-owned data revert to whatever WORLD-D19's manifest last captured — not a partial/best-effort reconstruction, a clean jump backward to the last save point, identical in *character* to a vanilla server's own crash-recovery story, just with a tighter interval.

### §I — In-flight message handling: undeliverable `(from, to)` streams to the dead node

Fully specified already by M7-B01 §K, restated here only to tie it to this blueprint's own concern (what happens to `RegionMessage` traffic destined for, or in flight from, a node that dies mid-takeover-window): every per-pair writer task and the connector task treat the dead node's `quinn::ConnectionError` identically — the connection is marked down, every pair routed through it returns `Backpressure` from `send()`, and a background reconnect loop retries with exponential backoff. **This self-heals automatically, by construction, the moment this blueprint's own `AssignRegion` commit propagates** (M7-B01 §H/§K's own words: "the raft directory commits a new owner for the affected regions and `resolve()` starts returning a different `NodeId` for them — no special-cased 'was this a takeover' branch exists in this crate") — this blueprint adds **zero** new logic to `rc-transport-net`, because none is needed: `NetworkTransport`'s own directory-driven redial mechanism (§H, already built) is exactly the mechanism that makes a stale outbound stream to a dead node self-correct once this blueprint's own reassignment commits. Any message that was already mid-transit to or from the dead node at the exact instant of death is lost with no recovery — the same "process lifetime" bound §H already states, restated here as this blueprint's own confirmation rather than a new fact.

### §J — Directory/proxy convergence: the sequencing that prevents routing to a ghost

CLUSTER-D5, restated: "the proxy... routes exclusively by the current raft-committed value, never by a locally cached belief... once that cache is known stale." CLUSTER-D21, restated: a proxy "independently subscrib[es] to the same raft-committed directory... via a lightweight watch stream to any live node" — the identical `DirectoryCache`/raft mechanism this blueprint already builds on, not a second directory. **The safety property this blueprint actually needs to prove is narrower than "the proxy's cache converges instantly"** — it needs only that a proxy which *has not yet* observed a reassignment cannot cause a *durable* mistake, because M7-B02 §E's own consistency contract already bounds convergence time (a follower's `DirectoryCache` updates "bounded by one `AppendEntries` round-trip," i.e., inside CLUSTER-D7's ≤30ms figure) and CLUSTER-D19's epoch fencing (§L) makes a stale route merely *inefficient* (one wasted hop, correctable via `NetworkTransport`'s own `NotOwner` self-heal, §I), never *unsafe* (a write from the wrong presenter is unconditionally rejected regardless of how the routing decision that produced it was made). This blueprint's own contribution to that proof is `concurrent_directory_reads_never_observe_a_stale_route_as_current`'s own primitive assertion (Acceptance tests): a `RegionLease` value, once read, is an immutable snapshot whose own `is_current` check against any *later* epoch always fails — the exact mechanism that makes it safe for a future `rc-proxy` implementation to hold a `RegionLease`-shaped cache entry for up to CLUSTER-D7's own bound without a correctness gap, restated here as the binding contract that blueprint must rely on rather than re-derive.

### §K — Players on the failed node: the fate this blueprint derives, stated as a decision

`13-cluster-architecture.md` fixes CLUSTER-D22's protocol only for the **planned** crossing case and explicitly leaves "first-join node resolution" as an Open Question needing "joint resolution once [`03`/`05`] are written." A player whose region was ticking on a node that just **crashed** is neither an ordinary planned crossing (no live `HandoffBegin` can ever be sent — the source is gone) nor an ordinary first join (their connection, and the proxy's own knowledge of who they are, already exists) — a genuine gap this blueprint must resolve rather than leave implicit, since M7's own acceptance criterion 2 depends on stating precisely what "unaffected regions observe zero interruption" implies, by clean omission, about *affected* ones.

**This blueprint's own decision, restated as binding for whoever builds `rc-proxy`:** a player on the failed node's region experiences **a pause bounded by this blueprint's own takeover-time budget (§C) plus a state rollback bounded by CLUSTER-D17's persistence-interval loss bound (§H) — but never a raw socket disconnect.** The mechanism, restated as the contract a conforming `rc-proxy` must implement (this blueprint does not implement it, per §A item 3): (1) the proxy's own QUIC connection to the dead node observably fails (the identical `ConnectionLostReason` signal M7-B01 §K already defines, applied symmetrically to a proxy↔node link per CLUSTER-D23's "own dedicated stream within the same... QUIC connection" framing); (2) the proxy marks every `connection_id` it was forwarding to that node as pending, and **buffers** (never drops) their inbound packets — reusing CLUSTER-D22 step 2's own buffering behavior verbatim, the one piece of the planned-handoff protocol this blueprint's own failure case *does* reuse unmodified; (3) the proxy watches its own directory-cache view (§J) for each pending connection's region to reappear under a new, live owner; (4) once that owner reports the region as loaded and ticking (a **coarse, per-region** readiness signal — `TakeoverEvent::RegionReassigned`'s own arrival, or, once `RegionResumeHandler::resume_region` returns `Ok`, an equivalent per-node event a future composition-root blueprint would need to surface to the proxy — **not** a per-player `HandoffReady`, since no live entity transfer occurs here: the player's own entity is not part of a region's WORLD-D19-manifested chunk data at all, per-player state living separately in playerdata, `05-game-mechanics.md`'s own domain), the proxy re-drives that connection through the new owner's **ordinary player-join entry point** — functionally a transparent rejoin the proxy performs on the player's behalf, never surfaced to the client as a new login. This is deliberately, and honestly, a **weaker** guarantee than CLUSTER-D22's zero-interruption planned case — restated plainly: the player is paused and rolled back, not seamlessly continued — while still strictly stronger than a raw "Connection Lost" disconnect, because the proxy's own socket-owning role (CLUSTER-D20) never depended on the now-dead node's liveness in the first place.

### §L — Split-brain protection: the zombie-node fencing proof, made concrete

CLUSTER-D19, restated: raft's single-leader property guarantees the `RegionId -> NodeId` table itself never holds two simultaneously-committed conflicting values; the residual risk is a "zombie" old owner (network-partitioned, not actually dead) that has not yet noticed it lost ownership and keeps acting. This blueprint's own concrete proof sketch, in two independent halves:

1. **Storage-side (the actual safety boundary).** A zombie node `X`, still believing itself the owner under epoch `E_old`, continues issuing `ChunkStorageBackend::write_chunk(..., epoch: Some(E_old))` calls for as long as it remains partitioned — this is **not itself unsafe**, because WORLD-D18's own conditional-put contract ("carrying CLUSTER-D19's epoch-fencing token") means the shared-storage backend rejects any write whose presented epoch is not the currently-stored one, unconditionally, regardless of how stale `X`'s own local belief is or how long it has been partitioned. `X`'s continued in-isolation ticking during the partition is therefore wasted work, never a durability hazard: nothing it computes is ever durably observed by any other party unless the partition heals before a newer epoch commits — restated as: **the safety boundary is the storage backend's own conditional-write check, not how quickly `X`'s local directory view converges.**
2. **Directory-side (what happens once `X` reconnects).** The moment `X`'s own raft instance catches up (bounded, once reconnected, by ordinary log replay, not a fixed constant this blueprint pins), its own `DirectoryCache` reflects the newer `DirectoryEntry` for every region it lost, and its own local `DirectoryReconciler` (§G) observes `entry.node != self.node_id()` and calls `handler.evict_region` — tearing down `X`'s own local ticking/registration before it can route or write anything further through the ordinary (non-zombie) path.

**One residual, explicitly bounded, non-hazardous risk, stated honestly rather than omitted:** `X`, while still partitioned, may continue emitting stale `RegionMessage` traffic (e.g. a `BorderUpdateEvent`) toward a neighboring region on some other node. CLUSTER-D6 already characterizes this payload as a **read-only mirror** ("self-heal-via-merge intent") — a stale halo update from a zombie is a bounded, cosmetic border-margin inconsistency, self-correcting on the next legitimate update, categorically different from — and never escalating into — a canonical-state write. This blueprint states this distinction explicitly rather than silently conflating "any zombie traffic" with "an unsafe zombie write."

**What this blueprint's own tests can, and cannot, prove.** `zombie_node_writes_are_rejected_after_reassignment` proves half (1) against a fake, in-process shared-storage double implementing the same conditional-write contract WORLD-D18 describes. Half (2) is already proven by M7-B02's own pre-existing `directory_cache_stays_consistent_under_concurrent_readers_and_one_writer` and this blueprint's own reconciler tests (§ Acceptance tests). Neither half proves the *proxy*-routing side of CLUSTER-D19's own "or be routed to by the proxy only while presenting the current epoch" sentence — that half genuinely requires `rc-proxy` to exist, and this blueprint states that limitation plainly rather than papering over it with a test that would only be exercising this crate's own already-proven primitives a second time under a misleading name.

### §M — Repeated-failure (takeover-during-takeover) safety

Two overlapping failure scenarios, both handled correctly by the same mechanism — re-deriving affected regions fresh from `DirectoryCache` at the start of every `Failed` handling pass (§E step 1), never from any state cached across passes or across `Failed` events:

- **The just-reassigned survivor also fails before finishing resume.** Node `A` dies; region `R` is reassigned to `B` (an `AssignRegion(R, B)` commit, bumping `R`'s epoch to `E+1`); `B` itself dies before `RegionResumeHandler::resume_region` ever completes (or even starts) for `R`. `B`'s own partial state, if any existed, was never durably written anywhere (§L half 1 — no write ever succeeds without first *finishing* a resume that establishes a valid, current-epoch-presenting handler instance). `B`'s own eventual `Failed` declaration (§B/§C, from whichever node is leader at that point) re-derives `R` as one of `B`'s currently-owned regions (per the **current** `DirectoryCache`, which still shows `B` as `R`'s owner — the failed resume never got far enough to change that) and reassigns it again, to some live `C`, bumping the epoch to `E+2`. No step anywhere requires `B` to have made *any* progress for this to be correct — the directory, not `B`'s own local state, is the only thing any later step reads.
- **A brand-new failure arrives while an earlier pass is still running.** `TakeoverOrchestrator`'s `Failed`-handling passes (§E) are processed **one event at a time, non-overlapping**, from the single `mpsc::UnboundedReceiver<NodeHealthEvent>` `ClusterNode::take_health_events()` hands it — this blueprint's own binding design choice, restated explicitly: a second `Failed` event queued while an earlier pass is still issuing `propose_assign_region` calls simply waits its turn in the channel, processed by the next loop iteration once the current pass's `for` loop (§E step 3) finishes. Because step 3's own `for` loop re-reads `counts` only from the *start-of-pass* snapshot plus its own within-pass increments (never a second live directory read mid-loop), a second node's concurrent failure is never interleaved into the *same* pass's placement math — it gets its own, later, independently-consistent pass instead. Every epoch bump anywhere in this whole sequence is monotonic by construction (M7-B02 §B: `Epoch::next` is always `self.0 + 1`, applied only inside `RedbStateMachine::apply`'s own single-writer, transactionally-ordered path) — no interleaving of two passes' commits can ever produce a non-monotonic epoch sequence for the same region, regardless of arrival order.

### §N — Moderate-confidence flags and reconciliation steps (verify at implementation time)

1. **`openraft::RaftMetrics.membership_config`'s exact voter-enumeration accessor.** §E step 2 assumes some method chain on `StoredMembership<NodeId, BasicNode>`/`Membership<NodeId, BasicNode>` yields the current voter `NodeId` set; the exact method name (`.membership().voter_ids()`, or similar) should be confirmed against `openraft` 0.9.25's own docs before writing `takeover.rs`'s body — a compile-time-only risk (caught immediately by `cargo build`), never a silent correctness bug.
2. **CLUSTER-D7's ≤30ms figure as a raft-commit-latency proxy (§C).** CLUSTER-D7 is scoped to `RegionMessage`/data-plane traffic; this blueprint reuses it as the best available stand-in for "one raft round trip in a co-located topology" because no other pinned figure exists for that specific quantity — flagged explicitly as a reuse, not a literal re-derivation, so a future revision that measures raft's own commit latency separately can correct this without this blueprint having silently asserted a false precision.
3. **`tokio::sync::mpsc::UnboundedReceiver`'s ownership by `TakeoverOrchestrator::spawn`.** `ClusterNode::take_health_events()` is callable exactly once (M7-B02 Deliverables) — this blueprint's own `spawn` signature (Deliverables) takes the receiver by value, consistent with that one-shot contract; the implementer should confirm no other code path in a future composition-root blueprint also expects to call `take_health_events()` itself (it cannot — the second call returns `None` by M7-B02's own design), a note worth restating explicitly in that future blueprint's own Prerequisites rather than silently discovered.

## Deliverables

### `crates/cluster/src/takeover.rs` (new)

```rust
//! Node-failure takeover (CLUSTER-D16) and the symmetric per-node directory
//! reconciliation every gain/loss of a region drives (Context §E/§G). Depends only
//! on this crate's own already-fixed types (`ClusterNode`, `DirectoryCache`,
//! `RegionLease`, `Epoch`, `NodeId`, `NodeHealthEvent`) — no new external
//! dependency, no new intra-workspace edge (Context §A/§D).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rc_messaging::RegionId;

use crate::{ClusterNode, Epoch, NodeId, health::NodeHealthEvent, types::RegionLease};

/// One observable event this module emits — the composition-root/test hook for
/// `--cluster-lifecycle-log`-shaped wiring a future blueprint adds, and this
/// blueprint's own acceptance tests' assertion surface.
#[derive(Debug, Clone)]
pub enum TakeoverEvent {
    FailureObserved { dead: NodeId, affected_regions: Vec<RegionId> },
    RegionReassigned { region: RegionId, from: NodeId, to: NodeId, new_epoch: Epoch },
    ReassignmentDeferred { region: RegionId, dead: NodeId, reason: String },
    NoLiveNodesAvailable { dead: NodeId, unassigned_regions: Vec<RegionId> },
    RegionResumed { region: RegionId, node: NodeId, epoch: Epoch },
    RegionEvicted { region: RegionId, node: NodeId },
}

/// This blueprint's one new error type for `RegionResumeHandler` (Context §F) —
/// never `ClusterError`, since a future storage-integration blueprint implements
/// this trait without depending on `rc-cluster`'s own error type.
#[derive(Debug, thiserror::Error)]
pub enum ResumeError {
    #[error("no RegionManifest found for region {region:?} in shared storage")]
    ManifestNotFound { region: RegionId },
    #[error("shared-storage read failed: {0}")]
    Storage(String),
    #[error("presented epoch is no longer current")]
    EpochStale,
}

/// Context §D — the least-loaded-by-region-count placement seam. `LeastRegionCountStrategy`
/// is this blueprint's own sole implementation; a future periodic-rebalancer
/// blueprint may supply an EWMA-weighted alternative without any change here.
pub trait PlacementStrategy: Send + Sync + 'static {
    fn select(&self, candidates: &[NodeId], current_counts: &HashMap<NodeId, usize>) -> Option<NodeId>;
}

pub struct LeastRegionCountStrategy;

impl PlacementStrategy for LeastRegionCountStrategy {
    fn select(&self, candidates: &[NodeId], current_counts: &HashMap<NodeId, usize>) -> Option<NodeId>;
}

/// Context §F — the exact "load from shared, durable storage" contract CLUSTER-D16
/// depends on. Implemented outside `rc-cluster` (a future storage-integration
/// blueprint, built on WORLD-D17's `ChunkStorageBackend`) — this crate only defines
/// and consumes it.
pub trait RegionResumeHandler: Send + Sync + 'static {
    fn resume_region(&self, lease: RegionLease) -> Result<(), ResumeError>;
    fn evict_region(&self, region: RegionId);
    fn is_resident(&self, region: RegionId) -> bool;
}

/// Context §E's takeover-sequence orchestrator. Runs symmetrically on every node —
/// only ever *does* anything on whichever node's `HealthMonitor` is currently
/// emitting `Failed` events, which by construction is always the current raft
/// leader (Context §B).
pub struct TakeoverOrchestrator { /* private */ }

impl TakeoverOrchestrator {
    pub fn new(placement: Arc<dyn PlacementStrategy>) -> Self;

    /// Spawns a background `tokio` task draining `health_events` one event at a
    /// time (Context §M's own non-overlap guarantee), running Context §E's sequence
    /// per `Failed` event, calling `node.propose_assign_region` and emitting
    /// `on_event` at each named point. `node` is `Arc`-shared with the caller (the
    /// composition root already holds one for its own other cluster wiring).
    pub fn spawn(
        self,
        node: Arc<ClusterNode>,
        health_events: tokio::sync::mpsc::UnboundedReceiver<NodeHealthEvent>,
        on_event: Arc<dyn Fn(TakeoverEvent) + Send + Sync>,
    ) -> TakeoverOrchestratorHandle;
}

pub struct TakeoverOrchestratorHandle { /* private */ }

impl TakeoverOrchestratorHandle {
    pub fn shutdown(&self);
    pub async fn join(self);
}

/// Context §G — the symmetric per-node resume/evict watcher.
#[derive(Copy, Clone, Debug)]
pub struct DirectoryReconcilerConfig {
    pub poll_interval: Duration,
}

impl Default for DirectoryReconcilerConfig {
    fn default() -> Self;
}

pub struct DirectoryReconciler { /* private */ }

impl DirectoryReconciler {
    pub fn spawn(
        config: DirectoryReconcilerConfig,
        node: Arc<ClusterNode>,
        handler: Arc<dyn RegionResumeHandler>,
        on_event: Arc<dyn Fn(TakeoverEvent) + Send + Sync>,
    ) -> DirectoryReconcilerHandle;
}

pub struct DirectoryReconcilerHandle { /* private */ }

impl DirectoryReconcilerHandle {
    pub fn shutdown(&self);
    pub async fn join(self);
}
```

### `crates/cluster/src/lib.rs` (modify — additive only)

Add `mod takeover;` to the module list and:

```rust
pub use takeover::{
    DirectoryReconciler, DirectoryReconcilerConfig, DirectoryReconcilerHandle,
    LeastRegionCountStrategy, PlacementStrategy, RegionResumeHandler, ResumeError,
    TakeoverEvent, TakeoverOrchestrator, TakeoverOrchestratorHandle,
};
```

No other line in `lib.rs` changes — every existing `pub use` (M7-B02's own) is untouched.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46):** the test changeset is every file below plus `takeover.rs` with every function body replaced by `todo!()` (every field, derive, and signature above stays exactly as specified) plus the additive extension to `crates/cluster/tests/support/mod.rs` (§ below) — itself committed in the test-authoring changeset, since it is test-support code, not production code, mirroring M7-B02's own identical treatment of its `tests/support/mod.rs`. The implementation changeset fills in real bodies only; it must not edit any test file, must not touch M7-B02's own five pre-existing test files, must not add/remove/rename a test case named below, and must not weaken an assertion.

### `crates/cluster/tests/support/mod.rs` (extend — additive only, M7-B02's own existing content untouched)

Add, alongside M7-B02's own `InProcessRouter`/`InProcessRaftNetwork`/`InProcessJoinClient`:

- `FakeSharedStorage` — a `parking_lot::Mutex<HashMap<(RegionId, /* chunk key placeholder */ u64), (u64 /* epoch */, Vec<u8>)>>`-backed test double with one method, `fn conditional_write(&self, region: RegionId, key: u64, epoch: u64, payload: Vec<u8>) -> Result<(), &'static str>`, implementing WORLD-D18's own conditional-put contract directly: succeeds and records `(epoch, payload)` only if no entry exists yet for `(region, key)` or the existing entry's stored epoch is `< epoch`; otherwise returns `Err("stale epoch")` without mutating anything — this is the concrete proof surface `zombie_node_writes_are_rejected_after_reassignment` uses.
- `FakeResumeHandler` — implements `RegionResumeHandler` against a `parking_lot::Mutex<HashMap<RegionId, RegionLease>>` (the "resident set"): `resume_region` inserts (or, under a strictly newer epoch, replaces) the entry and records the call in a shared `Mutex<Vec<(RegionId, Epoch)>>` log; `evict_region` removes the entry and logs it; `is_resident` checks presence. Exposes `fn resident_regions(&self) -> Vec<(RegionId, RegionLease)>` and `fn call_log(&self) -> Vec<ResumeCallRecord>` (an enum `{ Resumed(RegionId, Epoch), Evicted(RegionId) }`) for test assertions.
- `RecordingTakeoverEvents` — a `Mutex<Vec<TakeoverEvent>>`-backed `Fn(TakeoverEvent)` closure factory, `fn recorder() -> (Arc<dyn Fn(TakeoverEvent) + Send + Sync>, Arc<Mutex<Vec<TakeoverEvent>>>)`.

### `crates/cluster/tests/takeover_kill_tests.rs`

Uses `crates/cluster/tests/support` (`mod support;`).

1. `three_node_kill_resumes_regions_on_survivor_within_bounded_window` — construct a 3-node in-process cluster exactly as M7-B02's own `raft_cluster_inprocess.rs` test 2 does (`mod support`'s `InProcessRouter`); the leader commits `propose_assign_region(RegionId(1), <the non-leader node's id>)` and `propose_assign_region(RegionId(2), <same non-leader node's id>)`, converged on all three; spawn `TakeoverOrchestrator` (with `LeastRegionCountStrategy`) on **every** node, each fed that node's own `take_health_events()` receiver; spawn `DirectoryReconciler` + a fresh `FakeResumeHandler` per **surviving** node; **drop** the target non-leader node's `ClusterNode` (removing it from the router, simulating a hard kill — identical technique to M7-B02's own leader-failover test); poll (bounded by `election_timeout_max_ms * 4` per this crate's own established test-timeout convention) until both `RegionId(1)`/`RegionId(2)` appear in exactly one of the two surviving nodes' `FakeResumeHandler::resident_regions()`, under a strictly greater epoch than their pre-kill value; assert this converges within the bound, and assert each region resumed on exactly one survivor (never both, never neither).
2. `three_node_kill_preserves_directory_state_across_the_gap` — as test 1, but additionally polls a **third**, wholly unrelated region (`RegionId(9)`, pre-assigned to the node that is *not* killed and never touched by this test's own kill) throughout the entire kill-to-resume window, asserting `DirectoryCache::lease_of(RegionId(9))` on every surviving node returns the identical, unchanged `RegionLease` at every poll — never `None`, never a torn value — the crate-appropriate proxy for "unaffected regions observe zero interruption" this blueprint's own scope can prove (Context §A).

### `crates/cluster/tests/zombie_fencing.rs`

`zombie_node_writes_are_rejected_after_reassignment` — no raft/`openraft` involved, a pure logic test against `FakeSharedStorage` (`support::FakeSharedStorage`) plus `RegionLease::is_current`: a lease at `epoch = Epoch(3)` (simulating node `X`'s last-known-good state before a simulated partition) successfully writes once via `conditional_write(region, key, 3, payload)`; a **second**, independently-committed lease reassignment bumps the region's epoch to `Epoch(4)` (simulated directly — this test does not need real raft, only the fact that a newer epoch now exists) and a write under that newer epoch succeeds; a **third** write attempt presenting the **stale** `epoch = 3` (simulating zombie `X`, still using its old lease, unaware of the reassignment) is asserted `Err("stale epoch")`, and the storage map's own recorded payload for that key is asserted **unchanged** from the `epoch = 4` write (proving rejection, not silent overwrite).

### `crates/cluster/tests/proxy_convergence_primitives.rs`

`concurrent_directory_reads_never_observe_a_stale_route_as_current` — a `proptest!` property test (no `openraft` involved, pure `RegionLease`/`Epoch` logic, extending M7-B02's own `lease_fencing.rs` pattern to the proxy-relevant question specifically): generates a sequence of 2..20 `AssignRegion`-shaped epoch bumps for one region; for each *prefix* of that sequence, capture the `RegionLease` a hypothetical proxy would have cached "as of" that prefix's final epoch; assert that lease's `is_current` check against every *later* prefix's own final epoch is `false`, and against its own matching epoch is `true` — restated as: a proxy's own cached `RegionLease`, however stale, can never be mistaken for current once a newer epoch exists, which is exactly the property that makes CLUSTER-D19's storage/routing fencing sound regardless of how slowly a particular cache converges (Context §J/§L).

### `crates/cluster/tests/cascading_failure.rs`

`cascading_failure_during_takeover_converges_every_region_to_one_live_owner` — a 4-node in-process cluster (`support`'s router, extended to 4 members the same way M7-B02's own 3-node test constructs 3); the leader assigns `RegionId(1)` and `RegionId(2)` to node `B`; `TakeoverOrchestrator`+`DirectoryReconciler`+fresh `FakeResumeHandler` spawned on every node exactly as `takeover_kill_tests.rs` test 1; **drop node `A`** (the original leader, forcing a leadership election — Case 2 of Context §C) with `RegionId(1)`/`RegionId(2)` still owned by `B`, which remains alive; once a new leader is elected among `{B, C, D}` and (separately, a genuinely concurrent event, not sequenced by the test) **drop node `B`** too, before or during its own resume of anything; poll (bounded, generously, at `election_timeout_max_ms * 8` to allow for two sequential detection windows) until every region originally owned by `A` or `B` has a **single**, consistent final owner among the two remaining live nodes `{C, D}`, confirmed both via `DirectoryCache::lease_of` on every survivor agreeing, and via that owner's own `FakeResumeHandler::resident_regions()` containing the region under the directory's own final epoch; assert no region is ever resident (per any survivor's `FakeResumeHandler`) on **two** different nodes simultaneously at any point this test samples.

### `crates/cluster/tests/placement_strategy.rs`

`least_region_count_placement_is_deterministic_and_balances_across_multiple_regions` — a `proptest!` property test, no raft involved: generates a set of 2..8 synthetic `NodeId`s and an initial `HashMap<NodeId, usize>` of arbitrary region counts (`0..20` each); for a sequence of 1..10 sequential `select` calls (each incrementing the chosen node's own count in the map before the next call, exactly as Context §E step 3 does), assert (a) every call's result is `Some` (never `None` for a non-empty candidate set) and is always a member of the candidate set; (b) re-running the identical sequence against a freshly-cloned identical starting map produces the identical sequence of choices (determinism); (c) after the full sequence, the maximum count across all candidates minus the minimum count across all candidates never exceeds what it was before the sequence started, plus the sequence length (a loose but real balancing bound — this test does not assert perfect balance, only that placement never *concentrates* load onto an already-most-loaded node while a less-loaded one exists among the candidates, checked directly against `min_by_key`'s own definition rather than restated as a separate formula).

### `crates/cluster/tests/directory_reconciler.rs`

Uses `crates/cluster/tests/support` (`mod support;`).

1. `directory_reconciler_is_idempotent_against_already_resident_regions` — a single in-process `ClusterNode` (bootstrap, as M7-B02's own `bootstrap_flows.rs` test 1); a `FakeResumeHandler` pre-seeded (before spawning the reconciler) with `RegionId(1)` already resident at `Epoch(1)` via a direct call to its own test-only seeding method; the node commits `propose_assign_region(RegionId(1), <this node's own id>)` (matching what a fresh region-1-assignment under `Epoch::FIRST` would look like); spawn `DirectoryReconciler`; poll `handler.call_log()` for `2 * poll_interval` and assert **no** `Resumed(RegionId(1), _)` entry appears — the pre-seeded residency (simulating "already loaded via ordinary startup," Context §G) correctly suppresses a redundant resume call.
2. `directory_reconciler_evicts_on_unassign_and_on_reassignment_away` — a 2-node in-process cluster; leader assigns `RegionId(3)` to node `B`; `DirectoryReconciler` + fresh `FakeResumeHandler` spawned on `B`; poll until `handler.call_log()` shows `Resumed(RegionId(3), Epoch(1))`; leader then commits `propose_unassign_region(RegionId(3))`; poll until `handler.call_log()` additionally shows `Evicted(RegionId(3))`; repeat with a **second** region (`RegionId(4)`, assigned to `B` then **reassigned** to a third node rather than unassigned) asserting the identical eviction behavior fires on `B` once the reassignment-away commits, and — on the third node's own separately-spawned reconciler/handler pair — a matching `Resumed(RegionId(4), Epoch(2))` fires there.

## Implementation steps

1. **`takeover.rs` skeleton + `lib.rs`.** Every type/trait/signature from Deliverables, `todo!()` bodies where the test changeset does not already fix one. Observable: `cargo build -p rc-cluster` fails only on `todo!()`, not on missing types.
2. **`PlacementStrategy`/`LeastRegionCountStrategy`.** Trivial body per Context §D's own stated formula. Observable: `cargo nextest run -p rc-cluster --test placement_strategy` passes.
3. **`ResumeError`, `TakeoverEvent`.** Plain data, derive-backed.
4. **`DirectoryReconciler`.** Implement Context §G's diff algorithm exactly: a `tokio::spawn`ed loop, `tokio::time::interval(config.poll_interval)`, each tick reads `node.directory().config_epoch()`, skips the rest of the body if unchanged since the last tick, otherwise takes `node.directory().snapshot()`, filters to `entry.node == node.node_id()`, diffs against the task's own locally-held prior filtered set (a plain `HashMap<RegionId, Epoch>` the task owns), calling `handler.resume_region`/`emit RegionResumed`/update-local-set for a gained-or-newer-epoch entry (skipping the call, but still updating the local set, when `handler.is_resident(region)` already reports `true` under the identical epoch) and `handler.evict_region`/`emit RegionEvicted`/remove-from-local-set for a region present in the prior set but absent (or now owned elsewhere) in the current one. Observable: `cargo nextest run -p rc-cluster --test directory_reconciler` passes.
5. **`TakeoverOrchestrator`.** A `tokio::spawn`ed loop draining `health_events.recv().await` one event at a time; on `Some(NodeHealthEvent::Failed { id })`, run Context §E steps 1–3 exactly (fresh `snapshot()` read, live-candidate enumeration via `node.raft().metrics()`, the per-region `for` loop with its own running `counts` tally and the `NotLeader`-stops-the-pass rule); on `Some(NodeHealthEvent::Recovered { .. })`/`LeadershipChanged { .. }`, no action (this blueprint's own scope is failure reassignment only); on `None` (sender dropped), the task exits. Observable: `cargo nextest run -p rc-cluster --test takeover_kill_tests --test cascading_failure` passes.
6. **`tests/support/mod.rs`'s new additions get real bodies** (`FakeSharedStorage`, `FakeResumeHandler`, `RecordingTakeoverEvents`) — straightforward `parking_lot::Mutex`-guarded maps/vecs per Acceptance tests' own description.
7. **Run the full acceptance suite.** `cargo nextest run -p rc-cluster` — every test named in Acceptance tests passes, across all six new test files, alongside every one of M7-B02's own five pre-existing files unchanged.
8. **Doctests, lints.** `cargo test --doc -p rc-cluster`; `cargo run -p xtask -- fmt-check`; `cargo run -p xtask -- lint`; `cargo run -p xtask -- lint-deps` (trivial pass, Header field — no new dependency-graph edge).
9. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/cluster/tests/` this blueprint adds (including its own additive extension to `tests/support/mod.rs`) is committed first, alongside `todo!()`-stubbed (but field/derive/signature-complete) `takeover.rs` and the additive `lib.rs` edit. The implementation changeset fills in real bodies only — it must not touch any of M7-B02's own five pre-existing test files, must not add/remove/rename any test case named in this blueprint's own Acceptance tests, and must not weaken any assertion (in particular, `cascading_failure.rs`'s "no region resident on two nodes simultaneously" check and `zombie_fencing.rs`'s "payload unchanged after a rejected write" check must survive unchanged).

(b) **No new external dependencies, no new intra-workspace dependency-graph edge.** Every crate `takeover.rs` uses (`std`, `tokio`, `parking_lot`, `thiserror`) is already in `rc-cluster`'s own `Cargo.toml` from M7-B02. Do not add `rc-scheduler`, `rc-chunk-storage`, `rc-transport-net`, `rc-messaging`'s own transport crates, or `rc-proxy` as a dependency of `rc-cluster` under any circumstance — Context §A/§D's entire resolution of the `cluster --> sched` question depends on this blueprint needing none of them.

(c) **Scope boundary — do not implement beyond this blueprint's one crate (Context §A).** This blueprint does not implement: `ObjectStoreBackend`/`ChunkStorageBackend`'s concrete cluster-mode implementation (a future storage-integration blueprint — `RegionResumeHandler` is the exact trait boundary that blueprint fulfills, not a placeholder to fill in here); `rc-proxy` or any of CLUSTER-D20–D24's connection-forwarding/buffering/rejoin machinery (Context §K's own contract is what that future blueprint must satisfy, restated, not implemented, here); a cluster-mode composition-root wiring `TakeoverOrchestrator`/`DirectoryReconciler` into a real `rusty-clanker-server` process (a future composition-root blueprint's job, mirroring `M6-B07`); CLUSTER-D2's own EWMA-weighted periodic rebalancer (a separate, not-yet-written blueprint — this one's `PlacementStrategy` trait is the seam that blueprint extends, not replaces). Do not add placeholder implementations of any of these as a shortcut — every one stays exactly as unimplemented as this blueprint's own Deliverables show it.

(d) **No Mojang or third-party reimplementation code.** Every type and algorithm here is derived solely from `13-cluster-architecture.md`'s CLUSTER-D2/D7/D15–D24, `03-world-chunks-persistence.md`'s WORLD-D17–D23, and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30). The one external, independently-verifiable fact this blueprint cites outside the corpus — Akka Cluster Sharding's own default `LeastShardAllocationStrategy` behavior (Context §D) — is cited as prior-art validation for a design choice, exactly the same citation discipline CLUSTER-D2's own rationale already uses for the identical source, never as consulted source code.

(e) **No `unsafe` code.** Every type and function in this blueprint's Deliverables is implementable in 100% safe Rust.

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

Expected: every command exits 0. `cargo nextest run -p rc-cluster` runs every one of M7-B02's own 11 pre-existing test cases (unchanged) plus this blueprint's own 2 (`takeover_kill_tests.rs`) + 1 (`zombie_fencing.rs`) + 1 (`proxy_convergence_primitives.rs`) + 1 (`cascading_failure.rs`) + 1 (`placement_strategy.rs`) + 2 (`directory_reconciler.rs`) = 8 new test cases — all pass. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
