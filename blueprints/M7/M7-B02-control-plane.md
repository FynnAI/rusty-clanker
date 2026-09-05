# M7-B02 — Cluster Control Plane (`rc-cluster`)

| Field | Content |
|---|---|
| ID | M7-B02 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | M0-B02 (`rc-messaging` — `RegionId(pub u64)`, restated exactly; this blueprint's directory is keyed by this exact type, imported, never redefined). M6-B07 (the monolithic composition root — restated in Context §A only to fix the exact boundary this blueprint stops at: cluster-role wiring into `main.rs`/`rusty-clanker-server` is explicitly **not** this blueprint's job, matching M6-B07's own "Cluster mode... entirely out of scope... the proxy/QUIC/cross-node role split is `M7`'s" statement). |
| Implements | CLUSTER-D1 (the `RegionId -> NodeId` hop this crate's directory realizes). CLUSTER-D5 (raft-committed directory as sole ownership authority). CLUSTER-D13 (embedded `openraft` + `redb`, control-plane/data-plane split). CLUSTER-D14 (discovery/bootstrap). CLUSTER-D15 (health/failure detection mechanism). CLUSTER-D16's **foundation** (lease/epoch primitives; the takeover *algorithm* itself is a sibling blueprint's job, restated in Context §A). CLUSTER-D19 (epoch/lease fencing). CLUSTER-D26/D27 (this crate's own dependency-set discipline and config-shape restatement; TOML *parsing* is explicitly out of scope, Context §A). CLUSTER-D28 (`tracing` spans; OTLP export wiring explicitly deferred, Context §A). ASSET-D18/D19/D30 (no Mojang/third-party source consulted — this blueprint is pure distributed-systems/library-integration work). TEST-D45/D46 (test-first changeset boundary). TEST-D50 (CI-is-authority). |
| Crates touched | `rc-cluster` (`crates/cluster/`) only — a brand-new crate this blueprint scaffolds from nothing (no prior M7 blueprint has touched `crates/cluster/`). No workspace-root `Cargo.toml` edit: every external dependency this blueprint needs (`openraft`, `redb`, `postcard`, `serde`, `thiserror`, `tracing`, `tokio`, `parking_lot`) is already pinned in `[workspace.dependencies]` by `12-workspace-structure.md`. Not `rusty-clanker-server`, not `rc-proxy`, not `rc-transport-net`, not `xtask` — restated in full in Context §A and Constraints. |
| Estimated scope | L, explicitly oversized against `00-blueprint-spec.md`'s general ~800-line/~300-line-Context sizing guideline — the same class of stated exception `M6-B07` already established for a single composition-root-adjacent blueprint that cannot be usefully split without reintroducing exactly the "contract pinned, implementation deferred to a future sibling" seam pattern this crate exists to close for the cluster control plane specifically. This is the **one** blueprint that fixes `rc-cluster`'s complete redb schema, its `openraft` trait wiring, its directory/lease/bootstrap/health/membership API — splitting those across multiple blueprints would force every one of them to restate the same `TypeConfig`/schema from scratch or awkwardly cross-reference a sibling for types used on every page. |

## Goal & Done definition

Build `rc-cluster` — the whole control plane CLUSTER-D13 describes: an embedded `openraft` 0.9.25 raft group per cluster (one voter/learner per node process), backed by `redb` 4.2.0 for local raft-log and state-machine persistence, whose replicated state is **exactly** the `RegionId -> NodeId` directory (with a monotonic per-region fencing epoch, CLUSTER-D19), raft's own node membership (voters/learners, each carrying a dial address), and a cluster-wide config-epoch counter — never game/simulation state, never chunk data, never `RegionMessage` traffic (CLUSTER-D13's own "never carries per-tick simulation data" line, upheld by construction: nothing in this crate's dependency graph reaches `rc-scheduler`, `rc-messaging`'s `RegionMessage`, or any chunk-storage type). On top of that raft group, expose: (1) a read-mostly local `DirectoryCache` with a precisely bounded staleness contract for `RegionId -> NodeId` lookups; (2) `RegionLease`, the concrete realization of CLUSTER-D19's epoch-fencing token, with the exact "is this write/route still valid" check every future storage-backend and message-routing consumer calls; (3) CLUSTER-D14's discovery/bootstrap flow (single-node bootstrap, joint-consensus join, rejoin-after-restart with no re-bootstrap); (4) node identity that is operator-chosen and persisted purely by being the raft membership key redb already durably stores; (5) CLUSTER-D15's health/failure-detection signal, derived from raft's own leader-side replication metrics, with zero second gossip/heartbeat layer; (6) membership-change primitives (add-learner, promote-to-voter, remove-member); (7) `tracing` instrumentation across every operation above.

Everything this crate needs from the network (raft-to-raft RPCs between node processes; a joining node's "please add me" call to a seed) is behind two small, `openraft`-native or `openraft`-adjacent injected traits (`openraft::RaftNetworkFactory<TypeConfig>`, this blueprint's own `ClusterAdminApi`) — exactly the same "define the trait, prove it with an in-process test double, defer the real wire transport to a sibling blueprint" split `M0-B02`/`M0-B03` already used for `rc-messaging`'s `Transport` trait. This blueprint ships **zero** QUIC code and **zero** dependency on `rc-transport-net`.

Done when:

- [ ] `cargo build -p rc-cluster --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-cluster`.
- [ ] `raft_bootstraps_single_node_and_commits_first_directory_entry`, `raft_three_node_join_flow_converges_on_one_leader`, and `directory_update_propagates_to_every_node_within_one_election_timeout` (the "raft single/3-node in-process cluster tests... directory update propagation, leader failover" set my task names) all pass.
- [ ] `leader_failover_elects_a_new_leader_and_directory_stays_readable` passes: killing (dropping) the in-process leader's `Raft` handle causes the remaining two nodes to elect a new leader within `2 * election_timeout_max_ms`, and every surviving node's `DirectoryCache` keeps answering `lease_of` correctly throughout.
- [ ] `fencing_rejects_a_stale_epoch_after_reassignment` (a `proptest!` property test) passes: for any sequence of `AssignRegion` commits to the same region, a `RegionLease::is_current` check against any epoch strictly older than the latest committed one always returns `false`, and against the latest committed one always returns `true`.
- [ ] `rejoin_after_restart_resumes_without_re_bootstrap_or_re_join` passes: a node's `RedbLogStore`/`RedbStateMachine` are reopened against their own already-populated `redb::Database` (no `bootstrap`, no `seeds` consulted) and correctly resume raft with all prior directory state intact.
- [ ] `redb_log_store_roundtrips_append_read_truncate_purge_and_vote` and `redb_state_machine_roundtrips_apply_snapshot_and_install` pass — direct, `Raft`-bypassing unit tests against `RedbLogStore`/`RedbStateMachine` alone.
- [ ] `directory_cache_stays_consistent_under_concurrent_readers_and_one_writer` passes: 8 reader threads calling `lease_of`/`snapshot` concurrently with one writer thread calling `apply_commit` 10,000 times never observe a torn/inconsistent entry (an entry's `node` and `epoch` always come from the same commit).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0 against this crate.
- [ ] `cargo test --doc -p rc-cluster` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). `lint-deps` passes trivially for this blueprint: `rc-cluster`'s only intra-workspace dependency is `rc-messaging`, and WS-D3's rules name no forbidden edge this blueprint's own Cargo.toml creates (restated in full in Context §A's dependency-graph discussion).

## Context (self-contained)

### §A — Scope boundary: what this blueprint builds, what it deliberately does not, and why

**What ships.** `rc-cluster` is a self-contained library crate. Its only intra-workspace dependency is `rc-messaging` (for `RegionId`). It has no Cargo features of its own — WS-D5(a)'s `cluster` feature gate lives entirely on `rusty-clanker-server`'s manifest (`rc-cluster` is simply present-or-absent as one of that binary's `optional = true` dependencies; nothing inside this crate is conditionally compiled). This blueprint does not touch `rusty-clanker-server`, `rc-proxy`, `rc-transport-net`, or `xtask` — every one of those is a **separate, not-yet-written M7 blueprint's job**, named explicitly at each point below where this blueprint stops short of it.

**Three genuinely out-of-scope items, named precisely (never silently glossed over):**

1. **The real network.** `openraft` needs a working `RaftNetworkFactory<TypeConfig>` to let node processes actually talk raft RPCs to each other, and this blueprint's own join flow needs a way for a joining node to ask a seed "please add me as a learner." Both are network-transport concerns this blueprint deliberately does not solve with real wire code — it defines the exact trait boundary (`openraft::RaftNetworkFactory<TypeConfig>`, this blueprint's own `ClusterAdminApi`, Deliverables) and proves the boundary's *correctness* with an in-process test double (three `openraft::Raft<TypeConfig>` handles in one test process, routed by a plain `HashMap`, the same technique `openraft`'s own test suite uses) — exactly the `M0-B02` (`Transport` trait, no implementation) → `M0-B03` (`InProcessTransport`, real implementation) split, mapped onto this crate: a future sibling M7 blueprint supplies the QUIC-backed `RaftNetworkFactory` (most plausibly built on `rc-transport-net`'s connection-pooling primitives, opening a dedicated raft-RPC stream per (node, node) QUIC connection CLUSTER-D11 already establishes, the same "own dedicated stream within an existing connection" pattern CLUSTER-D23 already uses for the proxy control channel) and the QUIC-backed `ClusterAdminApi` transport. This blueprint ships zero `quinn`/QUIC code and takes zero dependency on `rc-transport-net`.
2. **CLUSTER-D16's takeover *algorithm*.** This blueprint builds the primitives CLUSTER-D16 depends on — `RegionLease`, epoch fencing, the health-failure signal (§F below), and `ClusterNode::propose_assign_region` as the exact call a takeover makes — but does **not** implement "on raft-detected node failure, decide which live node gets the failed node's regions and call `propose_assign_region` for each" itself. That decision (CLUSTER-D2's least-loaded placement strategy, applied out-of-band on failure) needs each node's own tick-duration EWMA (ARCH-D19) to place fairly — data `rc-scheduler` owns. `12-workspace-structure.md`'s WS-D3 rule 2 bars any `rc-cluster`↔`rc-scheduler` Cargo dependency in either direction, so that data crosses via a trait boundary (`LoadReportSink`, `M7-B03`'s own rebalancer/takeover blueprint) a future composition-root wiring pushes into, never a Cargo edge — `rc-cluster`'s own Cargo.toml, as built here, has no dependency on `rc-scheduler` at all, and none is ever added.
3. **CLUSTER-D28's OTLP export wiring.** This blueprint instruments every operation it owns with `tracing` spans/events (already-pinned `tracing = "0.1.44"`, zero new dependency) — but does not install a global `tracing_subscriber` registry or pin `tracing-opentelemetry`/`opentelemetry-otlp` versions. Installing a process-wide subscriber is a well-established Rust convention reserved for the final binary, never a library crate (a library that called `tracing_subscriber::fmt().init()` itself would silently clobber whatever subscriber the embedding binary wanted) — and `rusty-clanker-server`'s `main.rs`, the only place such a call could correctly live, does not exist as a target of any blueprint yet. `12-workspace-structure.md`'s own `[workspace.dependencies]` comment on `tracing` ("`tracing-opentelemetry` + OTLP exporter versions pinned when D28 is implemented") is read here precisely: "D28 is implemented" at the point a subscriber is actually installed, not at the point spans first exist — this blueprint is the latter, not the former.

### §B — `NodeId`, `Epoch`, `ClusterConfigEpoch` — the three identifiers this crate owns

**`NodeId` is an operator-chosen string, never allocated by this crate.** CLUSTER-D27's config shape fixes `node_id = "node-a"  # stable identity, persisted across restarts` — a human-picked label, not a counter this crate mints (unlike `RegionId`/`RcEntityId`, which use allocators specifically because nothing outside the engine ever needs to *choose* their value). "Persisted across restarts" is achieved for free: `NodeId` is exactly the raft membership key `redb` already durably stores as part of ordinary raft log/state-machine persistence — there is no separate identity file, no separate allocation step. `openraft` 0.9's `NodeId` associated type dropped its earlier `Copy` bound specifically to support non-`Copy` custom types including bare `String` (verified: [openraft `declare_raft_types!`/NodeId customization](https://docs.rs/openraft/latest/openraft/docs/getting_started/index.html), upgrade-guide `upgrade_08_09` notes) — this blueprint defines a thin newtype (`pub struct NodeId(pub String)`) rather than using `String` directly, matching this project's own established convention of newtyping every identifier (`RegionId(u64)`, `RcEntityId(u64)`) for type safety at call sites, at the one-line cost of a `Display` impl and a derive list `openraft::NodeId`'s trait needs (Context §I's moderate-confidence flag list covers the exact derive set to verify).

**`Epoch(pub u64)` is per-region**, not per-node and not a single cluster-wide value — CLUSTER-D19's own text is explicit ("every raft-committed **ownership entry** carries a monotonic epoch number"), and an ownership entry is one `RegionId`'s row in the directory. Starts at `Epoch(1)` on a region's first `AssignRegion` commit (there is no `Epoch(0)`; an unassigned region simply has no `DirectoryEntry` at all, `Option::None`, never a zero-epoch placeholder), and increments by exactly `1` on every subsequent `AssignRegion` commit for that same region, regardless of whether the new owner differs from the old one (Context §D restates why: this is the simplest rule that is still trivially correct, and a caller that only ever calls `propose_assign_region` when ownership genuinely changes — which is the only caller CLUSTER-D2/D16 ever describes — never observes a spurious bump anyway).

**`ClusterConfigEpoch(pub u64)` is the one cluster-wide counter** ("cluster config epoch" in this crate's own task brief) — incremented by exactly `1` on **every** entry this node's state machine applies, whether a `DirectoryCommand::AssignRegion`/`UnassignRegion` (a directory change) or an `openraft`-internal `EntryPayload::Membership` entry (a membership change, add/remove/promote). This gives directory-cache consumers (§E) one cheap, single-integer "has anything at all changed since I last synced" signal covering both axes CLUSTER-D13 names ("the `RegionId -> NodeId` table, cluster node membership... and partition-boundary bookkeeping") without needing to diff two separate counters.

### §C — `TypeConfig`: the concrete `openraft::RaftTypeConfig`

```rust
openraft::declare_raft_types!(
    pub TypeConfig:
        D = DirectoryCommand,
        R = DirectoryCommandResponse,
        NodeId = NodeId,
        Node = openraft::BasicNode,
);
```

`Node = openraft::BasicNode` (not a custom type) is a deliberate reuse, not an omission: `BasicNode { pub addr: String }` already carries exactly the one piece of per-node data this crate's own membership bookkeeping needs — the peer's dial address (CLUSTER-D27's `bind = "0.0.0.0:7777"`) — and `openraft`'s own internal membership tracking (`StoredMembership<NodeId, Node>`, surfaced on `RaftMetrics.membership_config`) already durably persists it as part of ordinary raft state; inventing a richer custom `Node` type today would duplicate data `openraft` already owns for zero present benefit (verified: [`RaftTypeConfig`/`declare_raft_types!` field defaults](https://docs.rs/openraft/0.9.25/openraft/macro.declare_raft_types.html) — `Node` defaults to `BasicNode` precisely for this common case). `Entry`, `SnapshotData`, `Responder`, `AsyncRuntime` are left at their macro defaults (`openraft::impls::Entry<Self>`, `Cursor<Vec<u8>>`, `openraft::impls::OneshotResponder<Self>`, `openraft::impls::TokioRuntime`) — nothing in this blueprint's scope needs a non-default choice for any of the four.

`DirectoryCommand`/`DirectoryCommandResponse` (the raft-log application payload and its per-entry response) are this crate's own types, restated in full in Deliverables — every field already `serde`-derived (this crate's local convention, not `rc-messaging`'s CLUSTER-D12 promise, which governs `Message<RegionMessage>` specifically; there is no relationship between the two — this crate's raft-log entries never cross `NetworkTransport`).

### §D — The redb schema: table layout, exact

One `redb::Database` per node process, opened once at `ClusterNode` construction and shared (`Arc`) between the log store and the state machine (two logically separate `openraft` responsibilities, one physical file — `redb` natively supports multiple independent tables in one file, verified: [redb 4.2.0 basic usage](https://docs.rs/redb/4.2.0/redb/)). Five `TableDefinition` constants, declared once in `store/schema.rs` and used by both `RedbLogStore` and `RedbStateMachine`:

| Table | Key | Value | Holds |
|---|---|---|---|
| `LOG_TABLE` | `u64` (log index) | `&[u8]` (postcard `openraft::Entry<TypeConfig>`) | Every raft log entry, `Normal` and `Membership` alike — `openraft`'s own log, restated as bytes this crate controls the encoding of. |
| `RAFT_META_TABLE` | `&str` | `&[u8]` (postcard) | Fixed keys: `"vote"` → `Vote<NodeId>`; `"last_purged_log_id"` → `Option<LogId<NodeId>>`. |
| `SM_META_TABLE` | `&str` | `&[u8]` (postcard) | Fixed keys: `"last_applied"` → `Option<LogId<NodeId>>`; `"last_membership"` → `StoredMembership<NodeId, openraft::BasicNode>`; `"config_epoch"` → `ClusterConfigEpoch`; `"current_snapshot_meta"` → `Option<SnapshotMeta<NodeId, openraft::BasicNode>>`. |
| `SM_DIRECTORY_TABLE` | `u64` (`RegionId.0`) | `&[u8]` (postcard `DirectoryEntry`) | The live `RegionId -> DirectoryEntry` directory (CLUSTER-D1/D5) — this crate's whole raison d'être. A region's absence from this table means "unowned," never a sentinel row. |
| `SNAPSHOT_TABLE` | `&str` (generated snapshot id) | `&[u8]` (raw bytes) | One full snapshot blob per `build_snapshot`/`install_snapshot` call — the postcard-serialized `(Vec<(RegionId, DirectoryEntry)>, ClusterConfigEpoch)` pair a snapshot restore rebuilds `SM_DIRECTORY_TABLE`/`SM_META_TABLE` from. |

Every value is `postcard`-encoded (already-pinned, `postcard = "1.1.3"`, CLUSTER-D12's own wire-format choice reused here for a second, unrelated purpose — one project-wide serialization convention rather than a second one invented for redb specifically) into a `Vec<u8>`, inserted as `&value_bytes[..]` (Context §I flags redb's exact `insert`/`get` generic-bound shape for `&[u8]`-valued tables as a verify-at-implementation-time item — the fetched docs show a `TableDefinition<&str, u64>` example only; this crate's own `&[u8]`-valued tables are a straightforward, well-precedented extension but the implementer should confirm the exact call shape against `redb` 4.2.0's own docs before writing it).

### §E — `DirectoryCache`: the read-mostly local cache and its exact consistency contract

`DirectoryCache` is a plain, transport-agnostic in-memory structure — `parking_lot::RwLock<HashMap<RegionId, DirectoryEntry>>` plus one `ClusterConfigEpoch` field (ARCH-D23's own "cold-path bookkeeping... `parking_lot`" pattern reused here for a hot-read/cold-write structure, same rationale). It has exactly one writer per process: `RedbStateMachine::apply` calls `apply_commit` synchronously, in the same call that durably writes to `SM_DIRECTORY_TABLE` (redb write first, cache update second — Context §H's crash-safety note explains the ordering) — never from any other code path. Reads (`lease_of`, `config_epoch`, `snapshot`) never block on redb, never block on raft, never cross a network hop.

**The exact consistency contract, restated for `B01`/`B06`'s benefit (a future `rc-transport-net` blueprint's send-time lookups; a future `rc-proxy` blueprint's routing table) — this is the one thing every future consumer of this type must get right:**

- On the node that is **currently the raft leader**, `DirectoryCache` is updated the instant a `DirectoryCommand` is committed **and applied locally** — no network round-trip, because the leader's own `apply()` call is what performs the commit's local effect. Staleness here is bounded by `openraft`'s own internal apply-loop scheduling latency (sub-millisecond in practice, not a number this blueprint pins).
- On a **follower** node, `DirectoryCache` updates only once that follower's own local raft instance has received and applied the replicated entry — bounded by one `AppendEntries` round-trip from the current leader, which is exactly CLUSTER-D7's already-pinned ≤30ms p99 cross-node latency budget (this blueprint does not introduce a second, competing latency figure — it inherits CLUSTER-D7's).
- **Reads never go through a raft linearizable-read barrier** (`openraft`'s own `ensure_linearizable`/deprecated `is_leader` are never called by this crate's own reads) — `lease_of` answers from whatever this node's own raft instance has applied so far, which may be momentarily behind the true cluster-wide committed state if this node is a lagging follower. This is a **deliberate** design choice, not an oversight: CLUSTER-D5's own text — "never by a locally cached belief about 'who owns what' once that cache is **known** stale" — licenses exactly this: a bounded-staleness cache is safe to read from as long as staleness is bounded (it is, by CLUSTER-D7) and the actual point of use re-checks freshness where it matters. That re-check is `RegionLease::is_current` (§F) at the point of an actual storage write or proxy route — **safety comes from epoch-fencing at the point of use, never from read-time freshness**, which is the entire reason CLUSTER-D19's fencing mechanism exists in the first place: it is what makes a cheap, non-linearizable local cache safe to build at all.

### §F — `RegionLease`: CLUSTER-D19's fencing token, made concrete

```rust
pub struct RegionLease { pub region: RegionId, pub node: NodeId, pub epoch: Epoch }
impl RegionLease {
    pub fn is_current(&self, presented_epoch: Epoch) -> bool { self.epoch == presented_epoch }
}
```

**"Grant"** is exactly one `AssignRegion` commit — the epoch it bumps to *is* the lease's fencing token from that moment forward. **"Renew" does not exist as a distinct operation** — restated precisely because this blueprint's own task brief uses the phrase "lease grant/renew/expiry" and a literal reading might expect a periodic renewal RPC; CLUSTER-D19's actual text describes pure epoch supersession, never a wall-clock TTL a holder must periodically refresh, and this blueprint follows the planning document over the looser paraphrase per `00-blueprint-spec.md`'s own governance rule ("where a blueprint and a planning document conflict, the planning document wins"). A lease is valid **indefinitely**, with no renewal traffic, until a *newer* `AssignRegion` commit for the same region supersedes it. **"Expiry"** is exactly that supersession: the instant a strictly greater epoch commits for a region, every holder of the old `(node, epoch)` pair is fenced — `is_current` against the old epoch now returns `false` everywhere the new epoch has propagated (bounded by §E's own staleness contract). `M2-B03`'s `ChunkStorageBackend` trait already reserves the exact wire shape this fencing token rides on: every method already accepts an `epoch: Option<u64>` parameter ("accepted on every method for signature compatibility with `ObjectStoreBackend` — a later milestone"), which this blueprint's `Epoch(pub u64)` is the literal, matching concrete value for — a future shared-storage-backend blueprint (`ObjectStoreBackend`, CLUSTER-D18) passes `Some(lease.epoch.0)` and its own conditional-write check is exactly `RegionLease::is_current`'s logic, re-implemented at the storage layer against whatever epoch it independently reads back. This blueprint does not implement that storage-side check itself (out of `rc-cluster`'s crate boundary) — it only guarantees the token's shape and semantics are exactly what that future check needs.

### §G — Discovery/bootstrap (CLUSTER-D14), restated precisely

`ClusterNodeConfig` carries `node_id`, `bind_addr` (this node's own dial address, stored as its `BasicNode.addr` once it joins/bootstraps), `storage: StorageLocation`, `seeds: Vec<String>`, `bootstrap: bool` — a direct restatement of CLUSTER-D27's `node_id`/`bind`/`seeds`/`bootstrap` TOML fields as constructor arguments (parsing the actual TOML file is explicitly out of scope, §A).

**The algorithm** (`ClusterNode::open_or_bootstrap`, Implementation steps §3 has the full pseudocode):

1. Open (or create) the local `redb::Database` at `config.storage` and construct `RedbLogStore`/`RedbStateMachine` against it.
2. Call `openraft::Raft::new(config.node_id, raft_config, network_factory, log_store, state_machine).await` — this **already** resumes any prior persisted state (log, vote, applied state machine) if this node has run before; nothing further is needed for a genuine restart.
3. Check whether this node's own store was freshly empty (`log_store`'s `get_log_state()` reports no entries **and** `read_vote()` reports `None` — both conditions, since a node that only ever voted in an election but was never itself a member still has a non-empty vote) before deciding what happens next:
   - **Fresh + `config.bootstrap == true`:** call `raft.initialize(BTreeMap::from([(config.node_id.clone(), BasicNode { addr: config.bind_addr.clone() })])).await` — CLUSTER-D14's "very first node of a brand-new cluster... self-votes into a single-member raft cluster." A `bootstrap = true` on an **already-initialized** raft instance is a no-op for this branch (the "fresh" check above already routes it to the "not fresh" branch below, so `raft.initialize` — which itself errors on a non-pristine instance — is simply never called; the operator-discipline requirement CLUSTER-D14 states ("never set on any subsequent node") is enforced here defensively, not merely trusted).
   - **Fresh + `config.bootstrap == false`:** the join flow — for each address in `config.seeds`, in order, call the injected `ClusterAdminApi`-shaped client's `request_join` against that seed (Deliverables' `JoinClient` trait); the first success wins and the loop stops; if every seed fails, return `ClusterError::NoReachableSeed`. This realizes CLUSTER-D14's "the join is proposed as an `openraft` joint-consensus membership change" — the seed, on receiving the request, calls its own `ClusterAdminApi::admit_learner_and_promote(joining_id, joining_node)`, which is exactly `raft.add_learner(..., true)` (blocking until log-caught-up) followed by `raft.change_membership(...)` — two ordinary `openraft` joint-consensus steps, not a bespoke protocol.
   - **Not fresh:** do nothing further — `raft.new` already resumed this node's prior role. `config.seeds`/`config.bootstrap` are **not consulted** on this path (a genuine rejoin-after-restart never re-runs the join flow, and never re-bootstraps).
4. Construct the shared `DirectoryCache`, spawn the `HealthMonitor` (§H) against `raft.metrics()`, and return the assembled `ClusterNode`.

### §H — Health/failure detection (CLUSTER-D15), restated precisely, plus this blueprint's own concrete signal design

CLUSTER-D15's mechanism, restated exactly: **raft's own leader-driven `AppendEntries` heartbeat and election-timeout are the sole failure-detection mechanism — no second gossip/phi-accrual/SWIM layer.** This blueprint adds nothing to that mechanism; it only turns the signal `openraft` already computes internally into an application-observable event stream, because nothing upstream of `openraft`'s own `RaftMetrics` watch channel exposes "peer X appears to be down" as a first-class notification a takeover-orchestration consumer (CLUSTER-D16, a sibling blueprint) can subscribe to without polling raw metrics itself.

**Verified mechanism** ([`RaftMetrics<NID,N>` field list](https://docs.rs/openraft/0.9.25/openraft/metrics/struct.RaftMetrics.html)): a **leader's own** `RaftMetrics.replication: Option<BTreeMap<NodeId, Option<LogId<NodeId>>>>` field ("`Some()` only when this node is leader") maps every other member to the last log id the leader believes it has replicated to. This crate's own `HealthMonitor` — only ever meaningfully active while `RaftMetrics.current_leader == Some(this node's own id)` — polls `raft.metrics()` at `HealthMonitorConfig::poll_interval` and, for each peer in the `replication` map, tracks (purely locally, never persisted, never raft-committed — this is a liveness *observation*, not control-plane state) the wall-clock instant that peer's mapped `LogId` last **changed**. A peer whose `LogId` has not advanced for longer than `HealthMonitorConfig::failure_threshold` emits `NodeHealthEvent::Failed { id }`; once it resumes advancing, `NodeHealthEvent::Recovered { id }`. `RaftMetrics.current_leader` changing (including this node itself becoming or ceasing to be leader) emits `NodeHealthEvent::LeadershipChanged { leader }`. Both `poll_interval` and `failure_threshold` are seed defaults (`Duration::from_millis(200)`/`2 * election_timeout_max_ms`) — calibration-pending, exactly the same "seed default, not yet load-tested" framing `01`/`13`'s own ARCH-D6/D19 and CLUSTER-D2/D3 thresholds already carry.

### §I — Moderate-confidence flags and reconciliation steps (verify at implementation time)

Collected here for the implementer's convenience — each is independently low-risk (a compile-time signature mismatch, never a silent correctness bug, since every one is caught by `cargo build` the moment it is wrong):

1. **`openraft`'s `serde` Cargo feature.** The workspace pin (`openraft = "0.9.25"`, no `features = [...]` block) may or may not enable `serde` by default; `NodeId`'s trait bound requires it ("only available when the `serde` crate feature is enabled"). This blueprint's own `Cargo.toml` (Deliverables) declares `openraft = { workspace = true, features = ["serde"] }` explicitly, safe whether or not it is already a default (Cargo unions feature sets).
2. **`openraft::NodeId`'s exact blanket-impl coverage for a custom newtype.** `String` is confirmed directly usable as `NodeId`; whether `pub struct NodeId(pub String)` (this blueprint's own newtype, §B) gets the `openraft::NodeId` trait automatically via a blanket impl, or needs one explicit `impl openraft::NodeId for NodeId {}` line, is unverified. If `cargo build` reports `NodeId` (this crate's type) does not implement `openraft::NodeId`, add that one line — always safe, never a duplicate-impl conflict, since no blanket impl can simultaneously exist and require this line.
3. **`redb`'s exact `insert`/`get` generic-bound shape for `TableDefinition<K, &[u8]>` tables.** Confirmed pattern from `redb` 4.2.0's own docs uses `TableDefinition<&str, u64>` (`table.insert("key", &123)`); this blueprint's `&[u8]`-valued tables are a straightforward extension but the precise `impl Borrow<...>` bound `insert`/`get` expect for a `&[u8]` value type should be confirmed against `redb` 4.2.0's own `Table`/`ReadableTable` docs before writing `store/log_store.rs`/`store/state_machine.rs`.
4. **`redb::backends::InMemoryBackend`'s exact constructor and `Builder::create_with_backend`'s exact bound on `StorageBackend`.** Confirmed to exist ("acts as temporal in-memory database storage"); exact constructor signature (`InMemoryBackend::new()` assumed, zero-argument) unverified in fine detail — used only by this blueprint's own tests (§ Acceptance tests), never by production code, so a signature mismatch here blocks only test compilation, never a production build.
5. **`RaftStateMachine::apply`'s exact handling of non-`Normal` (`Membership`/`Blank`) log entries.** This blueprint's own design (Implementation steps §5) pushes a placeholder `DirectoryCommandResponse` (sentinel `region: RegionId(0)`, `new_epoch: None`) for any entry whose `EntryPayload` is not `Normal(DirectoryCommand)`, on the understanding that no real caller ever reads the `R` value openraft returns for a membership-change entry (that path's own response comes from `openraft`'s internal `ClientWriteResponse` construction for `add_learner`/`change_membership`, not from this crate's `apply`'s return value) — confirm this against `openraft` 0.9.25's own `apply` contract (whether `Vec<Self::R>` must have exactly one entry per input entry regardless of payload kind, which this design already assumes) before finalizing.
6. **openraft's exact `RaftSnapshotBuilder`/snapshot-installation flow field names** (`SnapshotMeta`'s exact field list, `Snapshot<C>`'s exact shape) — this blueprint's Implementation steps describe the *content* this crate serializes into a snapshot (§D's `SNAPSHOT_TABLE` row) precisely; the exact `openraft`-side struct field names wrapping that content should be cross-checked against `openraft` 0.9.25's own `openraft::Snapshot`/`openraft::SnapshotMeta` docs at implementation time.

### Claims to verify (TEST-D57)

- None.

## Deliverables

### `crates/cluster/Cargo.toml` (new)

```toml
[package]
name = "rc-cluster"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-messaging = { path = "../messaging" }
openraft = { workspace = true, features = ["serde"] }
redb = { workspace = true }
postcard = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }
parking_lot = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

No workspace-root `Cargo.toml` edit (Header field, restated): every one of the eight normal dependencies above is already present in `[workspace.dependencies]` at the version `12-workspace-structure.md` pins.

### `crates/cluster/src/lib.rs`

```rust
//! `rc-cluster` — the cluster control plane (CLUSTER-D1/D5/D13-D19): an embedded
//! `openraft` group per cluster backed by `redb`, replicating exactly the
//! `RegionId -> NodeId` directory, raft's own node membership, and a cluster-wide
//! config-epoch counter — never game/simulation state. No network implementation
//! ships here (see this blueprint's Context §A) — `openraft::RaftNetworkFactory` and
//! this crate's own `JoinClient` are injected trait boundaries a sibling blueprint
//! fulfills with real QUIC transport.

mod admin;
mod directory;
mod error;
mod health;
mod ids;
mod node;
mod store;
mod types;

pub use admin::ClusterAdminApi;
pub use directory::DirectoryCache;
pub use error::ClusterError;
pub use health::{HealthMonitor, HealthMonitorConfig, NodeHealthEvent};
pub use ids::{ClusterConfigEpoch, Epoch, NodeId};
pub use node::{ClusterNode, ClusterNodeConfig, JoinClient, RaftTuning, StorageLocation};
pub use store::{RedbLogStore, RedbStateMachine};
pub use types::{DirectoryCommand, DirectoryCommandResponse, DirectoryEntry, RegionLease, TypeConfig};
```

### `crates/cluster/src/ids.rs`

```rust
/// A cluster node's operator-chosen, stable identity (CLUSTER-D14/D27: `node_id`).
/// Never allocated by this crate — persisted purely by being the raft membership key
/// `redb` already durably stores (this blueprint's Context §B).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self;
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

// Context §I item 2: add `impl openraft::NodeId for NodeId {}` here only if `cargo
// build` reports the blanket impl does not already cover this newtype.

/// A per-region fencing epoch (CLUSTER-D19). Starts at `Epoch(1)` on a region's first
/// assignment; increments by exactly 1 on every subsequent `AssignRegion` commit for
/// that same region. There is no `Epoch(0)` — an unassigned region has no
/// `DirectoryEntry`, never a zero-epoch placeholder (Context §B).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Epoch(pub u64);

impl Epoch {
    /// The first epoch ever assigned to a region: `Epoch(1)`.
    pub const FIRST: Epoch = Epoch(1);
    /// This epoch's immediate successor (`Epoch(self.0 + 1)`).
    pub fn next(self) -> Epoch;
}

/// The one cluster-wide monotonic counter (Context §B): incremented by exactly 1 on
/// every entry this node's state machine applies, directory change or membership
/// change alike.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct ClusterConfigEpoch(pub u64);

impl ClusterConfigEpoch {
    pub const ZERO: ClusterConfigEpoch = ClusterConfigEpoch(0);
    pub fn next(self) -> ClusterConfigEpoch;
}
```

### `crates/cluster/src/types.rs`

```rust
use crate::ids::{ClusterConfigEpoch, Epoch, NodeId};
use rc_messaging::RegionId;

openraft::declare_raft_types!(
    /// This crate's concrete `openraft::RaftTypeConfig` (Context §C). `Node` reuses
    /// `openraft::BasicNode` (an `{ addr: String }` wrapper) rather than a custom
    /// type — it already carries the one per-node datum this crate's own membership
    /// bookkeeping needs.
    pub TypeConfig:
        D = DirectoryCommand,
        R = DirectoryCommandResponse,
        NodeId = NodeId,
        Node = openraft::BasicNode,
);

/// The raft-log application payload (`TypeConfig::D`) — the ONLY mutations this
/// crate's state machine ever applies. Region directory changes exclusively; node
/// membership changes route through `openraft`'s own `EntryPayload::Membership`,
/// never through this enum (Context §C).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DirectoryCommand {
    /// Assign (or reassign) `region` to `node`, bumping that region's epoch by 1 (or
    /// setting it to `Epoch::FIRST` if this is the region's first-ever assignment).
    AssignRegion { region: RegionId, node: NodeId },
    /// Remove `region`'s directory entry entirely (e.g. after an ARCH-D6 merge
    /// retires one side's `RegionId` — a future rebalancer blueprint's call).
    UnassignRegion { region: RegionId },
}

/// The per-entry response (`TypeConfig::R`) a successful `Raft::client_write` of a
/// `DirectoryCommand` returns. For `UnassignRegion`, `new_epoch` is `None`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirectoryCommandResponse {
    pub region: RegionId,
    pub new_epoch: Option<Epoch>,
    pub config_epoch: ClusterConfigEpoch,
}

/// One region's currently-committed ownership record (Context §D/§E) — the state
/// machine's per-`RegionId` row.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirectoryEntry {
    pub node: NodeId,
    pub epoch: Epoch,
}

/// CLUSTER-D19's fencing token, made concrete (Context §F). Obtained only via
/// `DirectoryCache::lease_of`/`ClusterNode::lease_of` — never constructed directly by
/// a caller outside this crate's own internals.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RegionLease {
    pub region: RegionId,
    pub node: NodeId,
    pub epoch: Epoch,
}

impl RegionLease {
    /// True iff `presented_epoch` still matches this lease's epoch at the moment this
    /// value was read — the exact check a storage backend's own conditional write
    /// (WORLD-D17, a separate blueprint) or a proxy's own routing decision performs
    /// before proceeding (Context §F).
    pub fn is_current(&self, presented_epoch: Epoch) -> bool;
}
```

### `crates/cluster/src/error.rs`

```rust
/// This crate's one error type — every fallible public method returns
/// `Result<_, ClusterError>`.
#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("no seed in the configured list accepted this node's join request")]
    NoReachableSeed,
    #[error("this node's raft instance is not the current leader")]
    NotLeader,
    #[error("this node's local store already contains raft state; bootstrap/join was not attempted")]
    AlreadyInitialized,
    #[error("redb storage error: {0}")]
    Storage(#[from] redb::Error),
    #[error("openraft fatal error: {0}")]
    RaftFatal(String),
    #[error("openraft client-write error: {0}")]
    RaftClientWrite(String),
}
```

### `crates/cluster/src/store/mod.rs`, `schema.rs`, `log_store.rs`, `state_machine.rs`

```rust
// crates/cluster/src/store/schema.rs
use redb::TableDefinition;

/// Raft log entries. Key: log index. Value: postcard `openraft::Entry<TypeConfig>`.
pub const LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");
/// Fixed-key raft bookkeeping: "vote", "last_purged_log_id" (Context §D table).
pub const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");
/// Fixed-key state-machine bookkeeping: "last_applied", "last_membership",
/// "config_epoch", "current_snapshot_meta" (Context §D table).
pub const SM_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sm_meta");
/// The live directory. Key: `RegionId.0`. Value: postcard `DirectoryEntry`.
pub const SM_DIRECTORY_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("sm_directory");
/// Snapshot blobs, keyed by a generated snapshot id.
pub const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");
```

```rust
// crates/cluster/src/store/log_store.rs
use std::sync::Arc;
use crate::types::TypeConfig;

/// `openraft::RaftLogStorage<TypeConfig>` + `RaftLogReader<TypeConfig>` backed by one
/// shared `redb::Database` table, `LOG_TABLE` (Context §D). Not `Clone` — holds the
/// shared `Arc<redb::Database>`, cheap to construct a second handle from the same
/// `Arc` if `openraft`'s own `get_log_reader` needs an independent reader instance
/// (Implementation steps §4 covers this).
pub struct RedbLogStore {
    // private: db: Arc<redb::Database>
}

impl RedbLogStore {
    pub fn new(db: Arc<redb::Database>) -> Self;
}

// impl openraft::RaftLogStorage<TypeConfig> for RedbLogStore { ... }
// impl openraft::RaftLogReader<TypeConfig> for RedbLogStore { ... }
// Exact method set (Context §I confirms every signature against openraft 0.9.25):
// get_log_state, get_log_reader, save_vote, read_vote, append, truncate, purge
// (RaftLogStorage); try_get_log_entries (RaftLogReader). Implementation steps §4.
```

```rust
// crates/cluster/src/store/state_machine.rs
use std::sync::Arc;
use crate::{directory::DirectoryCache, types::TypeConfig};

/// `openraft::RaftStateMachine<TypeConfig>` + `RaftSnapshotBuilder<TypeConfig>` backed
/// by the same shared `redb::Database`'s `SM_META_TABLE`/`SM_DIRECTORY_TABLE`/
/// `SNAPSHOT_TABLE`. Owns the one `Arc<DirectoryCache>` `apply` keeps synchronized
/// with every durable commit (Context §E).
pub struct RedbStateMachine {
    // private: db: Arc<redb::Database>, directory: Arc<DirectoryCache>
}

impl RedbStateMachine {
    pub fn new(db: Arc<redb::Database>, directory: Arc<DirectoryCache>) -> Self;
}

// impl openraft::RaftStateMachine<TypeConfig> for RedbStateMachine { ... }
// impl openraft::RaftSnapshotBuilder<TypeConfig> for RedbStateMachine { ... }
// Exact method set (Context §I): applied_state, apply, get_snapshot_builder,
// begin_receiving_snapshot, install_snapshot, get_current_snapshot
// (RaftStateMachine); build_snapshot (RaftSnapshotBuilder). Implementation steps §5.
```

### `crates/cluster/src/directory.rs`

```rust
use std::collections::HashMap;
use rc_messaging::RegionId;
use crate::{ids::ClusterConfigEpoch, types::{DirectoryEntry, RegionLease}};

/// The read-mostly local directory cache (Context §E) — one instance per
/// `ClusterNode`, shared (`Arc`) with `RedbStateMachine`'s `apply` as its sole writer.
pub struct DirectoryCache {
    // private: parking_lot::RwLock<DirectoryCacheInner { entries: HashMap<RegionId,
    // DirectoryEntry>, config_epoch: ClusterConfigEpoch }>
}

impl DirectoryCache {
    pub fn new() -> Self;

    /// This cache's whole purpose: a non-blocking, non-network lookup. Returns `None`
    /// if `region` has no current directory entry (unowned, or never observed by this
    /// node yet). Consistency contract: Context §E — never blocks, may be
    /// CLUSTER-D7-bounded-stale on a follower.
    pub fn lease_of(&self, region: RegionId) -> Option<RegionLease>;

    /// This node's own last-observed cluster config epoch (Context §B).
    pub fn config_epoch(&self) -> ClusterConfigEpoch;

    /// Applied exactly once per committed `DirectoryCommand` by `RedbStateMachine`'s
    /// own `apply` (never called from anywhere else — Context §E). `entry: None`
    /// removes `region`'s row (an `UnassignRegion` commit); `Some(_)` inserts/replaces
    /// it (an `AssignRegion` commit). `config_epoch` is always this call's own
    /// resulting `ClusterConfigEpoch` — monotonically increasing, never supplied
    /// out of order by a correct caller.
    pub fn apply_commit(&self, region: RegionId, entry: Option<DirectoryEntry>, config_epoch: ClusterConfigEpoch);

    /// Every current entry, snapshotted under one read lock. Cold-path use only
    /// (diagnostics; a freshly-attached consumer's initial bulk load before switching
    /// to incremental `apply_commit`-fed updates).
    pub fn snapshot(&self) -> Vec<(RegionId, DirectoryEntry)>;
}
```

### `crates/cluster/src/admin.rs`

```rust
use crate::{error::ClusterError, ids::NodeId};

/// The in-process control-plane surface a cluster member exposes (Context §G) —
/// CLUSTER-D14's join-flow target and health/membership introspection.
/// `ClusterNode` implements this directly; a future admin-RPC blueprint exposes it
/// remotely over the wire (Context §A item 1) — this trait is the exact boundary that
/// remote wiring plugs into, unchanged.
pub trait ClusterAdminApi: Send + Sync {
    /// CLUSTER-D14's bootstrap step, called on a pristine instance naming exactly
    /// `self` as the sole voter. Errors `ClusterError::AlreadyInitialized` if this
    /// raft instance already has log/vote state.
    fn init_single_node(&self, bind_addr: String) -> impl std::future::Future<Output = Result<(), ClusterError>> + Send;

    /// CLUSTER-D14's join step, run on the seed a joining node contacts: adds
    /// `joining_id`/`joining_node` as a learner (blocking until log-caught-up), then
    /// promotes it to voter. Errors `ClusterError::NotLeader` if this node is not
    /// currently the raft leader — the caller (the join-flow orchestrator, never this
    /// method) retries against a different seed.
    fn admit_learner_and_promote(&self, joining_id: NodeId, joining_node: openraft::BasicNode) -> impl std::future::Future<Output = Result<(), ClusterError>> + Send;

    /// Finer-grained primitive `admit_learner_and_promote` composes: add `id` as a
    /// non-voting learner only (a legitimate standalone operation, e.g. read-scaling
    /// a node that should never become a voter).
    fn add_learner(&self, id: NodeId, node: openraft::BasicNode) -> impl std::future::Future<Output = Result<(), ClusterError>> + Send;

    /// Promote an already-caught-up learner to voter.
    fn promote_to_voter(&self, id: NodeId) -> impl std::future::Future<Output = Result<(), ClusterError>> + Send;

    /// Operator-initiated removal of `id` from raft membership entirely — distinct
    /// from CLUSTER-D16's automatic failure takeover, which only reassigns the failed
    /// node's REGIONS (`propose_assign_region`) and never touches raft membership
    /// itself (Context §H's own note: a transiently-partitioned node rejoining should
    /// resume as a member without re-joining raft from scratch).
    fn remove_member(&self, id: NodeId) -> impl std::future::Future<Output = Result<(), ClusterError>> + Send;

    /// A live watch over this node's own `openraft::RaftMetrics` — CLUSTER-D15's
    /// concrete, subscribable surface (Context §H).
    fn metrics(&self) -> tokio::sync::watch::Receiver<openraft::RaftMetrics<NodeId, openraft::BasicNode>>;
}

/// The join-flow client surface a joining node uses against a seed (Context §G step
/// 3's join branch). Production implementations dial the seed over the network (a
/// future sibling blueprint's job, Context §A item 1); this blueprint's own tests
/// supply an in-process implementation that calls a seed's own
/// `ClusterAdminApi::admit_learner_and_promote` directly.
pub trait JoinClient: Send + Sync {
    fn request_join(&self, seed_addr: &str, joining_id: NodeId, joining_node: openraft::BasicNode) -> impl std::future::Future<Output = Result<(), ClusterError>> + Send;
}
```

### `crates/cluster/src/health.rs`

```rust
use crate::ids::NodeId;

/// One derived liveness signal (Context §H) — never raft-committed, never persisted;
/// a purely local observation of `openraft`'s own leader-side replication metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeHealthEvent {
    /// This node (currently raft leader) has not observed `id`'s replicated log id
    /// advance for longer than `HealthMonitorConfig::failure_threshold`.
    Failed { id: NodeId },
    /// A previously `Failed` peer's replicated log id has resumed advancing.
    Recovered { id: NodeId },
    /// This node's own view of the current raft leader changed.
    LeadershipChanged { leader: Option<NodeId> },
}

#[derive(Copy, Clone, Debug)]
pub struct HealthMonitorConfig {
    /// How long a follower's replication may stall (while this node is leader)
    /// before `Failed` fires. Seed default: 2x `RaftTuning::election_timeout_max_ms`
    /// — calibration-pending (Context §H).
    pub failure_threshold: std::time::Duration,
    /// How often the leader polls `raft.metrics()` for replication changes. Seed
    /// default `Duration::from_millis(200)`.
    pub poll_interval: std::time::Duration,
}

impl HealthMonitorConfig {
    /// `failure_threshold` derived from `election_timeout_max_ms` per Context §H's
    /// 2x rule; `poll_interval` fixed at 200ms.
    pub fn from_raft_tuning(election_timeout_max_ms: u64) -> Self;
}

pub struct HealthMonitor {
    // private
}

impl HealthMonitor {
    pub fn new(config: HealthMonitorConfig) -> Self;

    /// Spawns a background `tokio` task polling `metrics_rx` at `poll_interval`,
    /// diffing successive `RaftMetrics.replication` snapshots (Context §H's exact
    /// algorithm), emitting derived events on the returned channel. Stops when
    /// `metrics_rx` closes (the owning `Raft` shut down).
    pub fn spawn(self, metrics_rx: tokio::sync::watch::Receiver<openraft::RaftMetrics<NodeId, openraft::BasicNode>>) -> tokio::sync::mpsc::UnboundedReceiver<NodeHealthEvent>;
}
```

### `crates/cluster/src/node.rs`

```rust
use std::sync::Arc;
use rc_messaging::RegionId;
use crate::{
    directory::DirectoryCache, error::ClusterError, health::NodeHealthEvent,
    ids::NodeId, types::{DirectoryCommand, DirectoryCommandResponse, TypeConfig},
};

/// Where this node's `redb::Database` lives. `InMemory` is test/dev-only — a
/// production node always uses `File` (Context §I item 4).
#[derive(Clone, Debug)]
pub enum StorageLocation {
    File(std::path::PathBuf),
    InMemory,
}

/// `openraft::Config`'s tuning knobs this crate exposes (Context §H's seed defaults).
#[derive(Copy, Clone, Debug)]
pub struct RaftTuning {
    pub heartbeat_interval_ms: u64,
    pub election_timeout_min_ms: u64,
    pub election_timeout_max_ms: u64,
}

impl Default for RaftTuning {
    /// Seed defaults, calibration-pending: `heartbeat_interval_ms: 250`,
    /// `election_timeout_min_ms: 750`, `election_timeout_max_ms: 1500`.
    fn default() -> Self;
}

/// `ClusterNode`'s constructor arguments — a direct restatement of CLUSTER-D27's
/// `node_id`/`bind`/`seeds`/`bootstrap` TOML fields (Context §G). TOML parsing itself
/// is out of this crate's scope (Context §A) — a caller constructs this struct
/// directly from whatever config-loading mechanism it owns.
#[derive(Clone)]
pub struct ClusterNodeConfig {
    pub node_id: NodeId,
    pub bind_addr: String,
    pub storage: StorageLocation,
    pub seeds: Vec<String>,
    pub bootstrap: bool,
    pub raft: RaftTuning,
}

/// The top-level composition type this blueprint ships: one `openraft::Raft<TypeConfig>`
/// plus its `DirectoryCache`, assembled per Context §G's algorithm. Non-generic —
/// `openraft::Raft::new` consumes its network/storage type parameters at
/// construction and returns a handle generic only over `TypeConfig` (Context §G,
/// verified against `openraft` 0.9.25's own `Raft::new` signature).
pub struct ClusterNode {
    // private: raft: openraft::Raft<TypeConfig>, directory: Arc<DirectoryCache>,
    // config: ClusterNodeConfig, health_events: Option<tokio::sync::mpsc::UnboundedReceiver<NodeHealthEvent>>
}

impl ClusterNode {
    /// Context §G's full algorithm: opens/creates local storage, constructs
    /// `openraft::Raft::new` (resuming prior state if any), and — only on a
    /// genuinely fresh store — either bootstraps (`config.bootstrap == true`) or runs
    /// the join flow against `config.seeds` via `join_client` (Context §G steps 3-4).
    /// `network_factory` is this node's own injected `openraft::RaftNetworkFactory`
    /// (Context §A item 1 — a sibling blueprint supplies the real QUIC-backed one;
    /// this blueprint's own tests supply an in-process one).
    pub async fn open_or_bootstrap(
        config: ClusterNodeConfig,
        network_factory: impl openraft::RaftNetworkFactory<TypeConfig> + 'static,
        join_client: impl crate::admin::JoinClient + 'static,
    ) -> Result<Self, ClusterError>;

    /// The shared, read-mostly directory cache (Context §E).
    pub fn directory(&self) -> &Arc<DirectoryCache>;

    /// The raw `openraft::Raft<TypeConfig>` handle, for callers needing lower-level
    /// access than this type's own convenience methods provide.
    pub fn raft(&self) -> &openraft::Raft<TypeConfig>;

    /// This node's own identity.
    pub fn node_id(&self) -> &NodeId;

    /// Takes this node's `NodeHealthEvent` receiver — callable exactly once
    /// (returns `None` on every call after the first); the intended consumer is a
    /// future takeover-orchestration blueprint (CLUSTER-D16, Context §A item 2).
    pub fn take_health_events(&mut self) -> Option<tokio::sync::mpsc::UnboundedReceiver<NodeHealthEvent>>;

    /// Propose (via `raft.client_write`) assigning `region` to `node` — the exact
    /// call CLUSTER-D2's rebalancer and CLUSTER-D16's takeover both make (Context
    /// §A item 2; this blueprint provides the call, not the decision of when/whom to
    /// call it with). Errors `ClusterError::NotLeader` if this node is not currently
    /// leader (`openraft`'s own `client_write` forwards to the true leader when it
    /// knows one — Context §I flags confirming this forwarding behavior at
    /// implementation time; if it does not auto-forward, callers must retry against
    /// `raft.metrics().borrow().current_leader`).
    pub async fn propose_assign_region(&self, region: RegionId, node: NodeId) -> Result<DirectoryCommandResponse, ClusterError>;

    /// As `propose_assign_region`, for `DirectoryCommand::UnassignRegion`.
    pub async fn propose_unassign_region(&self, region: RegionId) -> Result<DirectoryCommandResponse, ClusterError>;

    /// Graceful shutdown: signals `openraft` to stop and joins its internal task.
    pub async fn shutdown(self) -> Result<(), ClusterError>;
}

impl crate::admin::ClusterAdminApi for ClusterNode {
    // Thin wrappers over `self.raft()`'s own `init`/`add_learner`/`change_membership`/
    // `metrics` — Implementation steps §6.
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus every `src/*.rs` file from Deliverables with every function body replaced with `todo!()` (fields, derives, doc comments, and every `openraft`/`redb` trait `impl` block's method signatures stay exactly as specified — only executable bodies are stubbed), plus `Cargo.toml`. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/cluster/tests/`, must not add/remove/rename any test case listed below, and must not weaken any assertion.

### `crates/cluster/tests/support/mod.rs` (test-only, not a deliverable — mirrors `M0-B03`'s `FakeRegion` framing)

Defines `InProcessRouter` and `InProcessRaftNetwork`: a `parking_lot::RwLock<HashMap<NodeId, openraft::Raft<TypeConfig>>>` shared router, registered into by each in-process "node" as it is constructed; `InProcessRaftNetwork`'s `openraft::RaftNetwork<TypeConfig>` impl (`append_entries`/`vote`/`full_snapshot`) looks up the target `NodeId` in the router and calls the corresponding method directly on that `Raft` handle (`raft.append_entries(rpc).await`, etc. — `Raft<C>`'s own public inbound-RPC-handler methods, the same ones a real network transport's receiving side would call) — zero serialization, zero real sockets, the standard in-process `openraft` testing technique. Also defines `InProcessJoinClient` implementing `JoinClient::request_join` by looking up the seed's `NodeId` in the same router and calling that node's own `ClusterAdminApi::admit_learner_and_promote` directly. Both types are constructed fresh per test; router entries are added via `router.write().insert(id, raft_handle.clone())` (`openraft::Raft` is cheaply `Clone`, an internal-handle clone, per its own documented design) immediately after each in-process node's `ClusterNode::open_or_bootstrap` call returns.

### `crates/cluster/tests/raft_cluster_inprocess.rs`

Uses `crates/cluster/tests/support` (`mod support;`).

1. `raft_bootstraps_single_node_and_commits_first_directory_entry` — construct one `ClusterNode` with `bootstrap: true`, `seeds: vec![]`; await `raft.metrics()` until `current_leader == Some(own id)` (bounded poll loop, timeout `election_timeout_max_ms * 4`, test failure not a hang on timeout); call `node.propose_assign_region(RegionId(1), NodeId::new("node-a")).await.unwrap()`; assert the returned `DirectoryCommandResponse.new_epoch == Some(Epoch::FIRST)`; assert `node.directory().lease_of(RegionId(1)) == Some(RegionLease { region: RegionId(1), node: NodeId::new("node-a"), epoch: Epoch::FIRST })`.
2. `raft_three_node_join_flow_converges_on_one_leader` — construct node A with `bootstrap: true`; wait for A's leadership; construct node B and node C, both `bootstrap: false`, `seeds: vec!["a".into()]` (the router's own key, standing in for a real dial address in this in-process test), each via `InProcessJoinClient` targeting the shared router; await all three nodes' `raft.metrics()` reporting the identical `current_leader` and identical `membership_config` voter set of exactly `{A, B, C}`, within the same bounded timeout as test 1.
3. `directory_update_propagates_to_every_node_within_one_election_timeout` — the 3-node cluster from test 2; the leader calls `propose_assign_region(RegionId(5), NodeId::new("node-b"))`; poll all three nodes' `DirectoryCache::lease_of(RegionId(5))` until every one reports the identical `RegionLease`, asserting this converges within `election_timeout_max_ms` (a generous bound reflecting Context §E's CLUSTER-D7-derived staleness contract, not a tight timing assertion — this in-process test has no real network latency to model).
4. `leader_failover_elects_a_new_leader_and_directory_stays_readable` — the 3-node cluster from test 2, with one prior `propose_assign_region` commit already converged; **drop** the leader's `ClusterNode` (removing it from the router so the other two can no longer reach it, simulating a hard node failure — never a graceful shutdown, since a graceful `shutdown()` is a different code path this test does not exercise); poll the two survivors' `raft.metrics()` until a new leader is elected among them, bounded by `election_timeout_max_ms * 4`; assert both survivors' `DirectoryCache::lease_of` for the region committed before the failure still returns the correct, unchanged `RegionLease` throughout (never `None`, never a torn value) — proving `DirectoryCache` reads are unaffected by a raft leadership change.

### `crates/cluster/tests/lease_fencing.rs`

`fencing_rejects_a_stale_epoch_after_reassignment` (a `proptest!` property test, no `openraft`/`redb` involved — pure logic over `RegionLease`/`Epoch`): generates a `Vec<u8>` of length `1..20` (a sequence of "reassignment happened" events); starting from `Epoch::FIRST`, fold the sequence into a `Vec<Epoch>` where each step is the previous epoch's `.next()`; for the final `RegionLease` (region/node arbitrary fixed values, epoch = the last element of the folded sequence), assert `is_current` against that same final epoch returns `true`, and assert `is_current` against **every strictly earlier** epoch in the folded sequence (including `Epoch::FIRST` if the sequence has more than one element) returns `false`.

### `crates/cluster/tests/bootstrap_flows.rs`

Uses `crates/cluster/tests/support` (`mod support;`) for test 2 only.

1. `single_node_bootstrap_produces_a_leader_with_empty_directory` — as `raft_cluster_inprocess.rs` test 1's setup, but this test only asserts the leadership/emptiness precondition (`directory().snapshot().is_empty()` immediately after construction, before any `propose_assign_region` call) — kept as its own focused test per this blueprint's task brief naming "bootstrap flows (1-node...)" as its own acceptance item, distinct from test 1's directory-mutation assertions.
2. `three_node_bootstrap_join_flow_converges` — identical setup and assertion to `raft_cluster_inprocess.rs` test 2 (this blueprint's task brief names "3-node" join flow as its own acceptance item alongside 1-node bootstrap and rejoin-after-restart; the assertion is intentionally the same proven property, exercised here as its own named acceptance case rather than cross-referenced, matching this project's own "restate, never 'see test X'" discipline even between two files inside the same crate's test changeset).
3. `rejoin_after_restart_resumes_without_re_bootstrap_or_re_join` — construct a single node with `bootstrap: true`, `storage: StorageLocation::InMemory` via a **manually constructed** `redb::Database` (not the convenience path — this test needs to keep the `Database` handle alive across the simulated restart, Context §D's "no filesystem needed" testing technique); after leadership + one `propose_assign_region(RegionId(9), ...)` commit, **drop** the `ClusterNode` (dropping its `Raft` handle and `RedbLogStore`/`RedbStateMachine`, but not the underlying `redb::Database`, which the test kept its own `Arc` clone of); construct a **second** `ClusterNode` from a **fresh** `RedbLogStore`/`RedbStateMachine` pair wrapping the **same** `Arc<redb::Database>`, with `bootstrap: false, seeds: vec![]` (deliberately empty/unreachable — Context §G's algorithm must never consult them on this path); assert the new instance resumes as leader (a single-voter cluster's sole member always resumes as leader once it can elect itself, per ordinary raft behavior) **without** calling `raft.initialize` or the join flow (asserted indirectly: construction succeeds despite `seeds` being empty, which would make a genuine join-flow attempt fail with `NoReachableSeed` — succeeding here is proof the join-flow branch was never taken); assert `directory().lease_of(RegionId(9))` still returns the pre-restart `RegionLease` unchanged.

### `crates/cluster/tests/redb_store_roundtrip.rs`

Direct, `Raft`-bypassing unit tests against `RedbLogStore`/`RedbStateMachine` alone, using `redb::backends::InMemoryBackend` (Context §I item 4).

1. `redb_log_store_roundtrips_append_read_truncate_purge_and_vote` — construct a `RedbLogStore`; `save_vote` a synthetic `Vote`, `read_vote` returns it back equal; `append` 5 synthetic `Entry<TypeConfig>` values (blank/`Normal(DirectoryCommand::AssignRegion{...})` mixed) via its `LogFlushed` callback (awaited to completion); `get_log_reader().try_get_log_entries(0..5)` returns all 5 in order, equal to what was appended; `truncate(3)` then `try_get_log_entries(0..5)` returns only indices `0..3`; re-`append` 2 more; `purge(1)` then `get_log_state()` reports the correct last-purged/last-log-id pair and `try_get_log_entries` on a purged range returns entries as "not found" per `RaftLogReader`'s own documented semantics (never an error).
2. `redb_state_machine_roundtrips_apply_snapshot_and_install` — construct a `RedbStateMachine` with a fresh `DirectoryCache`; `apply` a batch of 3 entries (`AssignRegion(RegionId(1), NodeId::new("a"))`, `AssignRegion(RegionId(2), NodeId::new("b"))`, `UnassignRegion(RegionId(1))`); assert `applied_state()` reports the correct `last_applied` log id; assert `DirectoryCache::snapshot()` now contains exactly one entry (`RegionId(2)`); call `get_snapshot_builder().build_snapshot()`; construct a **second**, fresh `RedbStateMachine`/`DirectoryCache` pair; `begin_receiving_snapshot()` + `install_snapshot(meta, data)` on the second instance using the first's built snapshot; assert the second instance's `DirectoryCache::snapshot()` now matches the first's exactly, and `get_current_snapshot()` on the second instance returns a handle whose data, once read back, deserializes to the same directory content.

### `crates/cluster/tests/directory_cache_concurrency.rs`

`directory_cache_stays_consistent_under_concurrent_readers_and_one_writer` — construct a fresh `DirectoryCache`; `std::thread::scope` with 1 writer thread calling `apply_commit(RegionId(1), Some(DirectoryEntry { node: NodeId::new(&format!("n{i}")), epoch: Epoch(i) }), ClusterConfigEpoch(i))` for `i in 1..=10_000` (strictly increasing epoch/config-epoch each call) and 8 reader threads, each looping `lease_of(RegionId(1))`/`config_epoch()` 10,000 times, asserting on every non-`None` read that the returned `RegionLease.epoch.0` (parsed back out of the deterministic `node`/`epoch` pairing the writer used, `node == format!("n{epoch}")`) matches its own `node` field exactly (proving no torn read ever mixes one commit's `node` with a different commit's `epoch`) and that `config_epoch().0 >= ` the epoch just observed on `lease_of` minus a small race-tolerant margin is **not** asserted (only the intra-entry non-tearing property is checked — cross-field ordering between `lease_of` and a separate `config_epoch()` call is explicitly not a guarantee this type makes, since they are two separate reads under two potentially-different lock acquisitions if the implementer chooses that internal shape; Implementation steps' own reference shape reads both under one lock acquisition, which happens to make them consistent too, but this test does not depend on that detail).

## Implementation steps

1. **`Cargo.toml` + module skeleton.** Create `crates/cluster/Cargo.toml` exactly as Deliverables. Create every `src/*.rs` file with full type/trait signatures and `todo!()` bodies (the test changeset's own state, per the changeset boundary above) if not already present from that changeset. Observable: `cargo metadata` resolves; `cargo build -p rc-cluster` fails only on `todo!()` bodies, not on missing types/imports.
2. **`ids.rs`, `types.rs`, `error.rs`.** Real bodies — every type here is plain data with derive-generated behavior plus a handful of trivial constructors/arithmetic (`Epoch::next` is `Epoch(self.0 + 1)`; `ClusterConfigEpoch::next` identically; `RegionLease::is_current` is `self.epoch == presented_epoch`). Add `impl openraft::NodeId for NodeId {}` only if step 7's build reports it missing (Context §I item 2). Observable: `cargo build -p rc-cluster` succeeds for these three files' own content.
3. **`store/schema.rs`, `store/log_store.rs`.** Real `RaftLogStorage`/`RaftLogReader` bodies: `get_log_state` reads `LOG_TABLE`'s highest key (last log id) and `RAFT_META_TABLE["last_purged_log_id"]`; `save_vote`/`read_vote` postcard-round-trip through `RAFT_META_TABLE["vote"]`; `append` opens one write transaction, inserts every entry keyed by its own log index (postcard-encoded value), commits, then invokes the `LogFlushed` callback exactly once after the commit succeeds (never before — the callback's whole purpose is "this data is now durable"); `truncate(log_id)` removes every `LOG_TABLE` entry with key `>= log_id.index` in one write transaction; `purge(log_id)` removes every entry with key `<= log_id.index` and updates `RAFT_META_TABLE["last_purged_log_id"]` in the same transaction; `try_get_log_entries(range)` opens a read transaction and collects every present key in `range`, postcard-decoding each (a missing key inside the range is simply skipped, never an error, per `RaftLogReader`'s own documented semantics). Observable: `cargo nextest run -p rc-cluster --test redb_store_roundtrip -- redb_log_store` passes.
4. **`store/state_machine.rs`.** Real `RaftStateMachine`/`RaftSnapshotBuilder` bodies: `applied_state()` reads `SM_META_TABLE["last_applied"]`/`["last_membership"]`. `apply(entries)`: for each entry, in order, open one write transaction; match `entry.payload`: `EntryPayload::Normal(DirectoryCommand::AssignRegion{region, node})` → read the region's current entry from `SM_DIRECTORY_TABLE` (if any) to compute the new epoch (`existing.epoch.next()` or `Epoch::FIRST`), write the new `DirectoryEntry`, bump `SM_META_TABLE["config_epoch"]`, commit the transaction, **then** call `self.directory.apply_commit(region, Some(new_entry), new_config_epoch)` (redb-durable-first ordering, Context §D), push a `DirectoryCommandResponse{region, new_epoch: Some(new_epoch), config_epoch: new_config_epoch}`; `UnassignRegion{region}` mirrors this with a table removal and `apply_commit(region, None, ...)`; any non-`Normal` payload (`Membership`/`Blank`) still bumps `SM_META_TABLE["config_epoch"]`/`["last_membership"]` as appropriate, commits, and pushes the Context §I item 5 placeholder response — never calls `directory.apply_commit` (nothing region-shaped changed). Update `SM_META_TABLE["last_applied"]` once per `apply` call (after the loop, one final small transaction) to the last entry's log id. `get_snapshot_builder()` returns `self` (this type implements both traits, `RaftSnapshotBuilder::build_snapshot` reads every `SM_DIRECTORY_TABLE` row plus current `config_epoch` under one read transaction, postcard-encodes the pair, writes it into `SNAPSHOT_TABLE` under a freshly generated id (e.g. `format!("snap-{last_applied_index}")`), and returns the `Snapshot<TypeConfig>` `openraft` expects (Context §I item 6 flags exact field names to confirm). `begin_receiving_snapshot`/`install_snapshot`/`get_current_snapshot` mirror this in reverse (decode, clear + repopulate `SM_DIRECTORY_TABLE`, call `directory.apply_commit` once per restored region plus one final call establishing the restored `config_epoch`, update `SM_META_TABLE["current_snapshot_meta"]`). Observable: `cargo nextest run -p rc-cluster --test redb_store_roundtrip -- redb_state_machine` passes.
5. **`directory.rs`.** `DirectoryCache::new` is `Self::default()`-shaped (empty map, `ClusterConfigEpoch::ZERO`). `lease_of`/`config_epoch`/`snapshot` take a `parking_lot::RwLock` read guard; `apply_commit` takes a write guard, mutates both the entry map and the stored config epoch under that **one** guard acquisition (satisfying `directory_cache_stays_consistent_under_concurrent_readers_and_one_writer`'s non-tearing requirement by construction — a reader's guard acquisition can never observe a partially-applied commit). Observable: `cargo nextest run -p rc-cluster --test directory_cache_concurrency` passes.
6. **`admin.rs`, `health.rs`, `node.rs`.** `ClusterAdminApi` for `ClusterNode`: `init_single_node` calls `self.raft.initialize(...)`, mapping `openraft`'s own `InitializeError` (non-pristine instance) to `ClusterError::AlreadyInitialized`; `admit_learner_and_promote` calls `self.raft.add_learner(id, node, true).await` then `self.raft.change_membership(...)`, mapping a non-leader error to `ClusterError::NotLeader`; `add_learner`/`promote_to_voter`/`remove_member`/`metrics` are direct one-line forwards to the corresponding `openraft::Raft` methods. `HealthMonitor::spawn`: a `tokio::spawn`ed loop, `tokio::time::interval(poll_interval)`, each tick reads `metrics_rx.borrow().clone()`, diffs `.replication` against the previous tick's snapshot (a `HashMap<NodeId, (Option<LogId<NodeId>>, std::time::Instant)>` the task owns locally) per Context §H's algorithm, sends any resulting `NodeHealthEvent`s, and separately compares `.current_leader` against the previous tick's value to emit `LeadershipChanged`; exits when `metrics_rx.changed().await` returns `Err` (sender dropped). `ClusterNode::open_or_bootstrap` implements Context §G's full algorithm exactly as pseudocoded there; `propose_assign_region`/`propose_unassign_region` call `self.raft.client_write(DirectoryCommand::AssignRegion{..}).await`, mapping any error to `ClusterError::RaftClientWrite`, and unwrap the successful response's `.data` (a `DirectoryCommandResponse`) directly. Observable: `cargo build -p rc-cluster` succeeds with zero `todo!()` remaining.
7. **Run the full acceptance suite.** `cargo nextest run -p rc-cluster` — every test named in Acceptance tests passes across all five test files. Apply Context §I's flagged reconciliation steps as needed if any signature mismatch surfaces here.
8. **Doctests, lints.** `cargo test --doc -p rc-cluster`; `cargo run -p xtask -- fmt-check`; `cargo run -p xtask -- lint`; `cargo run -p xtask -- lint-deps` — all exit 0 (the last passes trivially, Header field's own note: `rc-cluster`'s only intra-workspace edge is `rc-cluster -> rc-messaging`, which no WS-D3 rule forbids).
9. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/cluster/tests/` (including `tests/support/mod.rs`) is committed first, alongside `todo!()`-stubbed (but otherwise complete: full field lists, full derive lists, full trait-impl method signatures, full doc comments) `src/*.rs` files and `Cargo.toml`. The implementation changeset (Implementation steps 1-9) fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case named in Acceptance tests, and must not weaken any assertion (in particular, `lease_fencing.rs`'s exhaustive "every strictly earlier epoch is rejected" check and `directory_cache_concurrency.rs`'s non-tearing check must survive unchanged).

(b) **No new external dependencies beyond the pinned set — zero workspace-root `Cargo.toml` edits.** Every crate this blueprint's `crates/cluster/Cargo.toml` names (`rc-messaging`, `openraft`, `redb`, `postcard`, `serde`, `thiserror`, `tracing`, `tokio`, `parking_lot`, `proptest`) is already in `[workspace.dependencies]` at `12-workspace-structure.md`'s pinned version. Do not add `quinn`, `rc-transport-net`, `rc-scheduler`, `tempfile`, `tracing-opentelemetry`, `opentelemetry-otlp`, or any other crate under any circumstance — each is a named, deliberate exclusion (Context §A) belonging to a different, future blueprint.

(c) **No Mojang or third-party reimplementation code.** This blueprint is pure distributed-systems library integration (`openraft`, `redb`) against this project's own `13-cluster-architecture.md` decisions — nothing here touches protocol wire format, decompiled game logic, or any other reimplementation project's source (ASSET-D18/D19/D30).

(d) **No `unsafe` code.** Every type and function in this blueprint's Deliverables is implementable in 100% safe Rust — `openraft`, `redb`, `parking_lot`, `tokio`, `postcard` are all safe-to-use crate APIs; no raw pointers, no `unsafe impl`, no FFI.

(e) **Scope boundary — restated from Context §A, binding.** This blueprint does not implement: a real network transport for `openraft::RaftNetworkFactory<TypeConfig>` or `JoinClient` (QUIC/`rc-transport-net`, a sibling M7 blueprint); CLUSTER-D16's takeover *algorithm* (which live node gets a failed node's regions — a sibling blueprint, most plausibly depending on `rc-scheduler`'s own ARCH-D19 EWMA data, an edge this blueprint's own Cargo.toml does not create); CLUSTER-D2's rebalancer (load-driven region reassignment decisions); any wiring into `rusty-clanker-server`'s `main.rs`/config parsing/role selection (WS-D5(a)'s `cluster` feature, CLUSTER-D27's TOML parsing — a sibling composition-root-extension blueprint); `rc-proxy`'s connection-forwarding/handoff logic (CLUSTER-D20-D24); a storage backend's own conditional-write check against `RegionLease` (WORLD-D17/`ObjectStoreBackend`, owned by `03-world-chunks-persistence.md`'s blueprint phase); a process-wide `tracing` subscriber or any OTLP exporter dependency pin. Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it, with the trait boundaries (`RaftNetworkFactory<TypeConfig>`, `JoinClient`, `ClusterAdminApi`, `ClusterNode::propose_assign_region`) each future consumer plugs into fixed precisely.

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

Expected: every command exits 0. `cargo nextest run -p rc-cluster` runs 4 (`raft_cluster_inprocess.rs`) + 1 (`lease_fencing.rs`, one property-test case regardless of internal proptest-generated input count) + 3 (`bootstrap_flows.rs`) + 2 (`redb_store_roundtrip.rs`) + 1 (`directory_cache_concurrency.rs`) = 11 test cases named in Acceptance tests — all pass. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
