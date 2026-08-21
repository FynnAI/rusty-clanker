# M7-B07 — Cross-Node Handoff & Pre-Warming (Node-Side)

| Field | Content |
|---|---|
| ID | M7-B07 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | M7-B01 (`rc-transport-net` — `NodeId`, `NodeDirectory`, `NetworkTransport`'s exact public API, restated where touched; one additive method, Finding F2). M7-B02 (`rc-cluster` — cited for `Epoch`/`RegionLease` framing only; this blueprint never depends on `rc-cluster` directly). M7-B06 (`rc-proxy` — **already built and merged**: `ProxyServer`, `NodeAcceptor`, `ProxyRoutingTable`, `ControlFrame`, `ProxyDirectory`, `ForwardedIdentity`/`SignedIdentity`, `ProxyConnectionId`, `ProxyMetricsSink`. This blueprint restates every part of that API it builds against in full below and adds exactly two additive methods to `NodeAcceptor` — Finding F5 — never touching anything else in that crate). M4-B08 (`rc-mechanics::entity::transfer` + `rusty-clanker-server::play::{player_transfer, two_region_world}` — the monolithic cross-region transfer protocol this blueprint extends; restated in full: `PlayerTransferPayload`'s exact shape, `PlayerMarker`'s `routing`-field precedent, `EntityArrivalDriver`'s exact `fn`-pointer-only signature, ARCH-D9/D10/D24's "+1 tick, entity absent from every `World`" invariant). M4-B01 (`EntitySnapshot`'s shape, consumed unmodified via M4-B08). |
| Implements | CLUSTER-D22 (the six-step handoff — the *node-side* half of it: M7-B06 already implements every proxy-side step; this blueprint implements the node-side triggers M7-B06's own Context §K explicitly names as "out of this blueprint's own crate boundary — `rc-scheduler`, PLAN-D3"). CLUSTER-D24 (pre-warming — M7-B06 §L explicitly confirms the node-to-node dial "is initiated node-to-node via `rc-transport-net`'s own already-existing QUIC infrastructure... not by anything `rc-proxy` does" — this blueprint is that node-side trigger). CLUSTER-D7 (the co-located latency budget this blueprint's timing decomposition builds on). CLUSTER-D9/D10 (confirmed, by construction, unmodified — zero new `RegionMessage` variants, zero change to `RegionTransferRequest`'s wire shape). CLUSTER-D19 (epoch fencing — cited, not re-implemented). CLUSTER-D17 (durability/data-loss-window model — cited for the node-failure-during-handoff fault case). PLAN-D3 (hard constraint, restated in full — **zero changes to any M0–M6 crate's ECS/tick-pipeline code**, `rc-scheduler`/`rc-mechanics`/`rc-messaging` all untouched). ARCH-D9/D24 (the "+1 tick" invariant this blueprint's fault analysis builds on, unmodified). TEST-D27 (property-test coverage). TEST-D45/D46 (test-first changeset boundary). TEST-D50 (CI-is-authority). ASSET-D18/D19/D30 (no Mojang/third-party source consulted). |
| Crates touched | `rc-proxy` (`crates/proxy/`, additive: two new public methods on the already-shipped `NodeAcceptor` type — Finding F5, no new files, no change to any existing method's signature). `rc-transport-net` (`crates/transport-net/`, one additive public method on `NetworkTransport` — Finding F2). `rc-core` (`crates/core/`, one new file, one additive `lib.rs` re-export — Finding F3). `rusty-clanker-server` (`crates/server/`, one new file `play/cluster_handoff.rs`; one additive field on the already-shipped `PlayerTransferPayload` and one on `PlayerMarker`, both from M4-B08 — Finding F4; one `play/mod.rs` edit). **Not touched, by construction (PLAN-D3): `rc-scheduler`, `rc-mechanics`, `rc-messaging`, `rc-cluster`.** Not touched, by construction (M7-B06 already owns them, and this blueprint changes nothing about them beyond Finding F5's two additive methods): `rc-proxy`'s `ProxyServer`, `ProxyRoutingTable`, `ControlFrame`, `ProxyDirectory`, `identity.rs`, `metrics.rs`. |
| Estimated scope | M — this blueprint's scope shrank substantially once M7-B06 (which did not exist when this blueprint was first drafted) turned out to already implement the entire proxy-side buffering/flip/flush state machine, the wire protocol, and the identity-forwarding mechanism. What remains is genuinely node-side: classification, the two `HandoffBegin`/`HandoffReady` emission points, the one payload field, and pre-warming. |

## Goal & Done definition

M7-B06 already ships the *entire proxy-side* of CLUSTER-D22's six-step handoff (`ProxyRoutingTable::begin_handoff`/`buffer_inbound`/`complete_handoff`, `ControlFrame::{Begin,Ready,Complete}`, identity forwarding, directory-watch) and explicitly, precisely names what it does not: the node's own act of *sending* `HandoffBegin`/`HandoffReady` and *receiving* `HandoffComplete` ("out of this blueprint's own crate boundary — `rc-scheduler`, PLAN-D3", M7-B06 §K step 3). This blueprint builds exactly that node-side half, plus CLUSTER-D24 pre-warming (M7-B06 §L: confirmed not `rc-proxy`'s job), plus the one payload field M7-B06 §N's own Needs-from list names as required ("a `ProxyConnectionId`-shaped value... must survive both an ordinary `RegionTransferRequest` transfer and a cold shared-storage load"). Concretely: (1) `NodeAcceptor` gains two small, additive public methods (Finding F5) so node-local code can push a `ControlFrame` to its owning proxy and poll for inbound ones — the one genuine gap in M7-B06's own shipped API, named by that blueprint's own text; (2) a pure crossing classifier (`rusty-clanker-server`) distinguishing same-region / same-node cross-region / cross-node cross-region, extending M4-B08's player-crossing detection; (3) the Stage-6b system that, on a cross-node crossing, stamps `PlayerTransferPayload.connection_id` (Finding F4) and calls `NodeAcceptor::send_control(Begin)`; (4) the post-`tick_region` hook (Finding F1, correcting an internal Stage-10/Stage-11 ordering inconsistency both `13-cluster-architecture.md` and M7-B06 §K repeat) that calls `send_control(Ready)` — achieved with **zero** new `rc-scheduler` hooks, reusing M4-B08's own `EntityArrivalDriver` mechanism exactly as shipped; (5) `NetworkTransport::prewarm` (Finding F2) and the node-side trigger that calls it, per CLUSTER-D24's exact radius; (6) a precise, honest per-step latency budget; (7) the full acceptance-test suite this task requires, built against M7-B06's *real* `ProxyServer`/`NodeAcceptor`/`FakeVanillaClient`-style test-support pattern rather than a parallel one.

Done when:

- [ ] `cargo build -p rc-proxy -p rc-transport-net -p rc-core -p rusty-clanker-server --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-proxy -p rc-transport-net -p rc-core -p rusty-clanker-server` on both OS legs.
- [ ] Every pre-existing `rc-proxy` test from M7-B06 (`login_through_proxy.rs`, `routing_races.rs`, `identity_integrity.rs`, `multi_proxy_consistency.rs`, `node_death.rs`, `dependency_graph.rs` — 12 cases) still passes unchanged, proving Finding F5's two additive methods are behavior-preserving for every existing caller.
- [ ] `handoff_end_to_end_real_quic_meets_two_tick_budget` (the criterion-1 measurement harness) passes: a scripted two-node-plus-proxy crossing over real loopback QUIC, using M7-B06's own `ProxyServer`/`NodeAcceptor`/`spawn_fake_node`-style test support, logs the same position/packet-continuity delta `play_region_transfer_player_walk.rs` (M4-B08) established, plus an end-to-end `send_control(Begin)`-to-`ProxyRoutingTable::complete_handoff` timestamp delta asserted `< HANDOFF_BUDGET_MS` (100 ms).
- [ ] `fault_injection_node_side_boundaries` (5 cases) all pass, each proving the exact node-side rollback/degradation behavior Context §E's fault table names.
- [ ] `ping_pong_stress_node_side_no_leak_or_stuck_state` passes: 500 rapid alternating cross-node/same-node reclassifications for one player leave zero leaked `PendingHandoffConfirmations` entries and zero stuck `NodeAcceptor` control-send state.
- [ ] `prewarm_trigger_and_release_hysteresis` (4 cases) passes.
- [ ] `cluster_crossing_classification_pure` (5 cases) passes against `classify_player_crossing` with zero `bevy_ecs::World` involved.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — no new dependency-graph edges anywhere in this blueprint (`rc-core`'s new type is a leaf addition; `rc-proxy`'s two new methods use only types the crate already imports; `rusty-clanker-server`'s new file depends only on already-permitted crates under the `cluster` feature).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-proxy -p rc-transport-net -p rc-core -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### §A — Scope boundary: what M7-B06 already built, and what is genuinely left

**PLAN-D3, restated exactly as this project's binding constraint on every M7 blueprint:** cluster mode is *only* the `Transport` implementation swap (M7-B01), the `RegionId -> NodeId` directory hop (M7-B02), the proxy/control-plane roles (M7-B06), and the storage-backend swap behind existing traits — **no change to M0–M6's ECS/tick-pipeline/domain code.** This blueprint honors that constraint by construction: every mechanism it adds lives in `rc-proxy` (two additive methods only), `rc-transport-net` (one additive method), `rc-core` (one additive newtype), and `rusty-clanker-server`'s own new, additive `play/cluster_handoff.rs` (compiled unconditionally but inert outside cluster mode, exactly `PlayerRouting`'s own established precedent, M4-B08).

**M7-B06's own shipped surface, restated exactly as this blueprint builds against it (no paraphrase drift — these are the real, merged signatures):**

```rust
// rc_proxy::ControlFrame (already shipped, M7-B06 §J — restated verbatim)
pub enum ControlFrame {
    PlayerJoin { connection_id: ProxyConnectionId, identity: SignedIdentity },
    HandoffBegin { connection_id: ProxyConnectionId, dest_node: rc_transport_net::NodeId },
    HandoffReady { connection_id: ProxyConnectionId },
    HandoffComplete { connection_id: ProxyConnectionId },
    PlayerDisconnected { connection_id: ProxyConnectionId },
    DirectorySnapshot { regions: Vec<(rc_messaging::RegionId, rc_transport_net::NodeId, u64)>, nodes: Vec<(rc_transport_net::NodeId, std::net::SocketAddr)>, config_epoch: u64 },
}

// rc_proxy::ProxyConnectionId (already shipped, M7-B06 §ids.rs — restated verbatim)
pub struct ProxyConnectionId(pub u64);

// rc_proxy::NodeAcceptor (already shipped, M7-B06 §node_acceptor.rs — restated verbatim,
// the four methods below are UNCHANGED by this blueprint)
impl NodeAcceptor {
    pub fn new(config: NodeAcceptorConfig, directory: Arc<rc_cluster::DirectoryCache>, admin: Arc<dyn rc_cluster::ClusterAdminApi>, runtime: tokio::runtime::Handle) -> Result<Self, ProxyBuildError>;
    pub fn with_metrics_sink(self, sink: Arc<dyn ProxyMetricsSink>) -> Self;
    pub fn try_recv(&self, id: ProxyConnectionId) -> Option<rc_protocol::RawPacket>;
    pub fn relay_sink(&self, id: ProxyConnectionId) -> Option<RelaySink>;
    pub fn shutdown(&self, timeout: std::time::Duration);
}
```

**The precise, named gap.** M7-B06's own Context §K, describing CLUSTER-D22 step 3, writes: "Node-side (**out of this blueprint's own crate boundary** — `rc-scheduler`, PLAN-D3): `dest_node`'s Stage 1 applies the `RegionTransferRequest`; its Stage 10 sends `ControlFrame::HandoffReady{connection_id}`..." — M7-B06 defines `ControlFrame::HandoffReady`'s *shape* and `ProxyRoutingTable::complete_handoff`'s *reaction* to it, but ships **no method anywhere on `NodeAcceptor` (or any other type) for node-local code to actually transmit a `ControlFrame` to the proxy it is already connected to, nor to receive one** (`try_recv` is scoped to relayed `RawPacket` Play-state traffic only, per M7-B06 §I's own text: "the proxy never calls `try_decode_frame`... for Play-state traffic" — `ControlFrame`s are a structurally separate channel, §J, that `NodeAcceptor` privately owns a stream for but exposes no public seam onto). This is not an oversight this blueprint needs to work around with a parallel connection — `NodeAcceptor` already holds exactly the QUIC connection and control stream this needs; **Finding F5** (Deliverables §C) adds the two missing methods, additively, to the type that already owns the resource.

### §B — The six-step handoff, restated exactly, split by who owns each step

| Step | Decision ID | Owner | Status |
|---|---|---|---|
| 0. Pre-warm (proactive, ahead of any crossing) | CLUSTER-D24 | Node (source) | **This blueprint** — M7-B06 §L confirms this is not `rc-proxy`'s job. |
| 1. Initiation: despawn + `RegionTransferRequest` + `send_control(Begin)` | CLUSTER-D22 step 1 | Node (source) | ARCH-D10 half unmodified (M4-B08); `send_control(Begin)` is **this blueprint** (Finding F5 + §D below). |
| 2. Buffering: proxy transitions to `HandoffPending`, buffers inbound | CLUSTER-D22 step 2 | Proxy | **M7-B06**, already shipped (`ProxyRoutingTable::begin_handoff`/`buffer_inbound`). |
| 3. Destination apply + `send_control(Ready)` | CLUSTER-D22 step 3 | Node (dest) | ARCH-D10 half unmodified (M4-B08); `send_control(Ready)` is **this blueprint** (Finding F1 + F5, §D/§E below). |
| 4. Flip + flush | CLUSTER-D22 step 4 | Proxy | **M7-B06**, already shipped (`ProxyRoutingTable::complete_handoff`). |
| 5. Completion: `HandoffComplete` received, local bookkeeping torn down | CLUSTER-D22 step 5 | Node (source) | Proxy's own send is **M7-B06** (`server.rs`'s own reaction to `complete_handoff`'s return); the node's own receipt via `try_recv_control` and bookkeeping teardown is **this blueprint** (Finding F5, §E). |

### §C — Node-side crossing classification

**Only player connections ever trigger any of this blueprint's machinery** — mobs have no client connection to route, so CLUSTER-D10's own unmodified-reuse promise already carries a mob's `RegionTransferRequest` across a node boundary exactly as transparently as `InProcessTransport` carries it across a region boundary intra-node (M7-B01 §H's same-node short-circuit / ordinary QUIC delivery). This blueprint touches no mob-transfer code anywhere.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossingKind {
    None,
    /// Destination region owned by this same node — ordinary ARCH-D10 transfer, zero
    /// proxy involvement, `PlayerTransferPayload.connection_id` stays `None`.
    SameNode { dest_region: rc_messaging::RegionId },
    /// Destination region owned by a different node.
    CrossNode { dest_region: rc_messaging::RegionId, dest_node: rc_transport_net::NodeId },
}

/// Pure core, no `bevy_ecs::World` reference — mirrors `detect_mob_crossings`'s own
/// established "ECS-agnostic core, adapter at the production call site" pattern (M4-B08).
/// Skips (`CrossingKind::None`) the same narrower-contract cases `detect_mob_crossings`
/// already skips: a `resolve()` result that is not `Address::Region`, or a
/// `NodeDirectory::resolve` miss (deferred, retried next tick — never an error).
pub fn classify_player_crossing(
    pos: [f64; 3],
    dimension: rc_core::DimensionId,
    ownership: &rc_mechanics::border::RegionOwnership,
    node_directory: &dyn rc_transport_net::NodeDirectory,
) -> CrossingKind;
```

### §D — Finding F1: the Stage-10/Stage-11 ordering correction

`13-cluster-architecture.md`'s CLUSTER-D22 text and M7-B06's own restatement of it (§K step 3) both say the destination's *Stage 10* sends `HandoffReady` "once its own Stage-11 encode has produced" the player's first packet — impossible under ARCH-D12's fixed pipeline, where Stage 10 strictly precedes Stage 11. **This blueprint's binding resolution, achieved with zero `rc-scheduler` changes**: `HandoffReady` is emitted from the composition root's own code, immediately after `executor.tick_region(...)` returns for the tick that applied the arrival — which is, by construction, strictly after Stage 11 has run, since `tick_region` executes the whole fixed pipeline before returning. The already-shipped `EntityArrivalDriver` mechanism (M4-B08, a plain `fn(&mut World, Vec<EntitySnapshot>)`, no captured state) runs at Stage 1 — too early to send `Ready` itself, but exactly the right place to *record intent*: it pushes `connection_id` into a small `bevy_ecs::Resource`, `PendingHandoffConfirmations(Vec<ProxyConnectionId>)` (Deliverables §D), for every arriving player snapshot whose `connection_id` is `Some(_)`. The composition root's own per-region tick loop, after `tick_region` returns, drains that resource and calls `NodeAcceptor::send_control(id, ControlFrame::HandoffReady { connection_id: id })` (Finding F5) for each entry — satisfying CLUSTER-D22's own explicit intent ("guaranteeing something is ready to send before the switch") exactly, and resolving the same inconsistency in both `13`'s own text and M7-B06's restatement of it, without a single line added to `rc-scheduler`.

### §E — Finding F5: `NodeAcceptor`'s two additive methods, and the fault behavior they enable

```rust
// crates/proxy/src/node_acceptor.rs (MODIFY — additive; every existing method unchanged)
impl NodeAcceptor {
    // ... every existing method (Context §A, M7-B06 Deliverables) unchanged ...

    /// NEW (M7-B07, Finding F5). Sends `frame` to the specific proxy connection this
    /// node already knows owns `connection_id` — the same internal routing `try_recv`/
    /// `relay_sink` already key off, since a player's connection is served by exactly
    /// one proxy instance (CLUSTER-D21) at a time. Non-blocking (the same discipline
    /// every other method on this type and on `NetworkTransport::send` already uphold,
    /// ARCH-D29): internally queued and flushed on the shared Tokio runtime, mirroring
    /// `ProxyConnectionHandle::try_send_payload`'s own enqueue-then-background-flush
    /// shape (M7-B06 §C). Returns `Err(SendControlError::UnknownConnection)` if
    /// `connection_id` is not currently associated with any live proxy connection this
    /// `NodeAcceptor` knows about (never a panic).
    pub fn send_control(&self, connection_id: ProxyConnectionId, frame: ControlFrame) -> Result<(), SendControlError>;

    /// NEW (M7-B07, Finding F5). Non-blocking pop of the next inbound `ControlFrame`
    /// this node has received from any proxy it is connected to — mirrors `try_recv`'s
    /// own polling shape exactly, for the symmetric reason: `HandoffComplete` and
    /// `PlayerDisconnected` are proxy-to-node control traffic this node's own composition
    /// root must be able to observe, and no existing method exposes it. `None` if
    /// nothing is currently pending.
    pub fn try_recv_control(&self) -> Option<(ProxyConnectionId, ControlFrame)>;
}

#[derive(Debug, thiserror::Error)]
pub enum SendControlError {
    #[error("connection {0:?} is not currently associated with any live proxy connection")]
    UnknownConnection(ProxyConnectionId),
}
```

**Per-step-boundary fault behavior, restated precisely (the task's own required fault-injection coverage — every case *this blueprint* is responsible for; M7-B06's own already-shipped fault tests, `routing_races.rs`/`node_death.rs`, cover the proxy-side halves and are not re-tested here):**

| Fault | Behavior |
|---|---|
| `send_control(Begin)` fails (`SendControlError::UnknownConnection`, or the underlying QUIC write never lands) | The source node retries with the same backoff schedule `NetworkTransport`'s own reconnection logic already uses (`INITIAL_RECONNECT_BACKOFF`/`MAX_RECONNECT_BACKOFF`, M7-B01 §K, reused for consistency). Meanwhile, per M4-B08 Part 1.7's own already-established rule ("must look up its target fresh, every tick, by `RcEntityId`... simply finds no match... no special-casing"), any packet still arriving for this connection at the source node — because the proxy, having never received `Begin`, keeps routing there — silently no-ops against the now-despawned entity. A bounded, rare, monitored degradation, not new machinery. |
| Destination node crashes after applying the arrival, before `send_control(Ready)` | CLUSTER-D16's own takeover path (M7-B05, unmodified) reassigns the destination's regions to a survivor from the last persisted snapshot — this player's just-arrived, not-yet-saved state is lost, the bounded consequence CLUSTER-D17's own durability model already accepts. The proxy's own `ProxyRoutingTable` entry, meanwhile, follows M7-B06's own already-shipped "unplanned reassignment" path (§K) — no new mechanism needed on the proxy side; this blueprint's own contribution is exactly nothing further here, restated so it is not mistaken for an unhandled case. |
| `try_recv_control` never observes a `HandoffComplete` (the frame is lost, or the proxy itself crashed, M7-B06's own `node_death.rs`) | The source node applies its own bounded local garbage-collection to the small pending-confirmation bookkeeping entry it kept only for its own `Begin`-retry logic — harmless, since the entity is already gone from the source's `World` regardless of whether `Complete` ever arrives. Pure bookkeeping cleanup, zero simulation-correctness impact. |
| `classify_player_crossing` observes `CrossingKind::CrossNode` but `NodeDirectory::resolve` for the *same* region flips to a different node before `send_control` actually flushes (a directory update racing the crossing decision within the same tick) | `send_control` is keyed by `connection_id`, not by region — the frame this blueprint sends still names whatever `dest_node` `classify_player_crossing` resolved *at decision time* (an ordinary, already-accepted read of a bounded-staleness cache, M7-B02 §E's own consistency contract, cited not re-derived). If that node turns out to be stale by the time the proxy processes `Begin`, the destination node's own `EntityArrivalDriver` simply never fires (the `RegionTransferRequest` itself, per M7-B01 §H's own directory-redirect mechanism, is independently retargeted to whichever node is actually current) — the proxy's own `HandoffPending` state for this connection times out under M7-B06's own already-shipped rollback path exactly as any other stalled handoff does. This blueprint adds no new machinery for this case; it is an ordinary instance of the "destination node stalls" row above. |
| Repeated rapid re-classification (a ping-ponging player) | See Context §H — a deliberate non-mechanism, not a fault. |

### §F — Relationship to M4-B08: what is shared, unmodified, and what is new

**Shared, unmodified.** ARCH-D10's own transfer mechanism (despawn + `RegionTransferRequest` send at Stage 10, arrival application at the destination's Stage 1 via `EntityArrivalDriver`) is **not** modified — CLUSTER-D10 confirms this reuse is unconditional. `PlayerTransferPayload`'s existing nine fields carry exactly the same simulation state cross-node that they already carry cross-region intra-node — this is precisely what "align with M4-B01's `EntitySnapshot` + session state" means: there is no separate cluster-mode session-state payload, because M4-B08 already built the one that exists.

**New, additive, this blueprint's own (Finding F4 — directly fulfilling M7-B06 §N's own Needs-from item 4).** `PlayerTransferPayload` gains one field:

```rust
// crates/server/src/play/player_transfer.rs (MODIFY — additive; every existing field,
// derive, and method unchanged)
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlayerTransferPayload {
    pub uuid: u128,
    pub username: String,
    pub network_entity_id: i32,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub fall_distance: f64,
    pub tracked_entities: Vec<u64>,
    /// NEW (M7-B07, Finding F4). `Some(id)` iff the source node classified this crossing
    /// as cross-node and is expecting the destination node to confirm arrival via
    /// `NodeAcceptor::send_control`. `None` for every monolithic-mode crossing (M4-B08,
    /// unchanged behavior) and every same-node cross-region cluster crossing — the same
    /// "`None` means no cluster-specific meaning" precedent `PlayerMarker`'s own
    /// `routing` field (M4-B08) already established. This carries the identical numeric
    /// value `rc_proxy::ProxyConnectionId` names on the wire (Context §G) — kept as a
    /// separate type only because `rc-messaging`'s `EntitySnapshot`/`component_data`
    /// path this field rides through must remain reachable from code that never depends
    /// on `rc-proxy` (WS-D5(a)'s optional-dependency gating; `rc-messaging` and this
    /// payload's own defining crate, `rusty-clanker-server`, are both compiled
    /// unconditionally). This is precisely M7-B06 §N's own Needs-from item 4 — a
    /// `ProxyConnectionId`-shaped value that "survives both an ordinary
    /// `RegionTransferRequest` transfer and a cold shared-storage load" — fulfilled here
    /// by riding the existing `EntitySnapshot.component_data` envelope M4-B08 already
    /// built, without any change to `rc-messaging`'s own `EntitySnapshot` outer type
    /// (M7-B06's own explicit non-goal, honored, not contradicted).
    pub connection_id: Option<rc_core::ConnectionId>,
}
```

`PlayerMarker` (`rusty-clanker-server::play::world`, M1-B05/M2-B07/M4-B01/M4-B08) gains one more additive field via the identical `..Default::default()` precedent M4-B08 already used twice: `pub connection_id: Option<rc_core::ConnectionId>`, defaulting `None` — populated only by a cluster-mode join path (M7-B06's own `FirstJoinResolver`/`PlayerJoin` flow, a future composition-root blueprint's job to wire, not this one's) and threaded forward into `PlayerTransferPayload.connection_id` at every subsequent crossing by this blueprint's own crossing-detection system, so it survives an arbitrary number of same-node crossings and is always available, unchanged, the moment a genuinely cross-node crossing needs it.

### §G — `rc_core::ConnectionId`: why it exists, and its exact relationship to `ProxyConnectionId`

```rust
// crates/core/src/connection_id.rs (NEW — Finding F3)
/// The identical per-connection numeric identity `rc_proxy::ProxyConnectionId` names,
/// mirrored into `rc-core` because it must be nameable from code that compiles in every
/// build configuration (`PlayerTransferPayload`, `rc-messaging`-adjacent) as well as
/// cluster-only code (`rc-proxy`) — `rc-proxy` cannot be a dependency of unconditionally-
/// compiled `rusty-clanker-server` code under WS-D5(a)'s optional-dependency gating, so
/// no single type can live in only one of the two crates. Conversion at the one call
/// site that depends on both (`cluster_handoff.rs`, compiled unconditionally but only
/// ever *exercised* under the `cluster` feature) is a trivial, zero-cost, always-lossless
/// `ProxyConnectionId(id.0) <-> ConnectionId(id.0)` roundtrip — both wrap the same `u64`
/// representation `rc_proxy::ProxyConnectionId` already established (M7-B06 §ids.rs),
/// deliberately chosen here to match rather than introduce a third, incompatible shape.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnectionId(pub u64);
```

### §H — Budget decomposition (CLUSTER-D22's ≤100 ms / 2-tick bound)

CLUSTER-D7's own reference figures, restated: same-AZ round-trip is "typically <2 ms," p99 worst-case ≤30 ms one-way. This blueprint extends that same co-located-topology assumption to the proxy↔node link (M7-B06 §D's own second QUIC endpoint rides the identical CLUSTER-D11 mutual-TLS transport under the identical deployment-topology requirement).

| Segment | Typical | Owner |
|---|---|---|
| Source node: Stage 6b crossing detected → Stage 10 `RegionTransferRequest` + `send_control(Begin)` | 0 ms additional | This blueprint (§C/§D) + M4-B08 (unmodified) |
| Network hop, node → proxy (`Begin`) | ~2 ms | M7-B06's own QUIC link |
| Proxy `begin_handoff` | <1 ms | M7-B06, already shipped |
| Wait for destination's own next tick boundary (Stage 1) | 0–50 ms, independent tick clocks (CLUSTER-D25) | Dominant term |
| Destination: Stage 1 apply → Stage 11 encode → post-`tick_region` hook → `send_control(Ready)` | a few ms, inside the tick already running | M4-B08 (unmodified) + this blueprint (§D) |
| Network hop, node → proxy (`Ready`) | ~2 ms | M7-B06's own QUIC link |
| Proxy `complete_handoff` (flip + flush) | <1 ms | M7-B06, already shipped ("this blueprint's own steps add no artificial delay," M7-B06 §K's own timing-budget paragraph) |
| **Typical total** | **≈ 55 ms** | Comfortably inside the 100 ms/2-tick bound, dominated by the one-tick phase-misalignment wait CLUSTER-D22's own "2 ticks" framing already budgets for. |

**Worst-case p99**: `2 × 30 ms + 50 ms = 110 ms` — 10 ms over the nominal figure, stated honestly rather than papered over, restating precisely what CLUSTER-D22's own wording already concedes ("the common case") and what CLUSTER-D7's own rationale already frames ("leaving headroom... without requiring either threshold to be revised" — headroom for *one* hop's jitter, not a guarantee against both hops simultaneously landing at their tail together).

### §I — In-flight packet handling, both directions

**Inbound (client → server).** Fully owned by M7-B06 (`ProxyRoutingTable::buffer_inbound`/`complete_handoff`'s FIFO drain, tested directly by M7-B06's own `handoff_mid_packet_burst_preserves_order_and_drops_nothing`). This blueprint adds nothing here.

**Outbound (server → client).** Needs no buffering or reordering machinery anywhere — a direct consequence of ARCH-D24/ARCH-D9's own exclusivity invariant: the entity is live in at most one `World` at a time, so outbound packets are single-sourced by construction (the source node produces them through its own last tick of ownership, the destination produces nothing before its own Stage 1 applies the arrival). There is no tick at which both nodes have something to encode for this player.

**The one residual dangling-packet race** (a relayed packet reaching the source node microseconds after its own Stage-10 despawn, because the proxy had not yet processed `Begin`) is **not new** — it is the identical case M4-B08's own Part 1.7 "transfer during damage/pickup" edge case already generically covers, restated here as applying equally to the cluster-mode network-hop case, not merely the intra-process one it was originally written for.

### §J — Repeated rapid crossings (ping-pong): a deliberate non-mechanism, and why

This blueprint does **not** add any position- or decision-level suppression to slow a rapidly ping-ponging player, despite the task's own framing inviting one. Any mechanism that defers the *entity transfer itself* (not merely the connection-handoff half) to avoid thrashing would, for however many ticks it deferred, leave the player's entity simulated by a region that no longer geometrically contains its position — an unbounded extension of the *already-accepted* one-tick ARCH-D10 in-flight window (M4-B08's own "costs exactly one tick... below the threshold of human perception" framing) this blueprint is not willing to sign off on without the same load-testing rigor every other numeric threshold in this corpus already demands (ARCH-D6/D19, CLUSTER-D2/D3's own "seed default, calibration-pending" framing). Since the connection-handoff half cannot be safely decoupled from the entity-transfer half, suppressing one without the other is not an option either.

Instead, this blueprint relies on three things already true by construction: (1) M7-B06's own `ProxyRoutingTable` is proven, by that blueprint's own tests, to handle rapid handoff/reassignment cycles without leaking state; this blueprint's own `ping_pong_stress_node_side_no_leak_or_stuck_state` proves the *node-side* half (repeated `classify_player_crossing`/`PendingHandoffConfirmations`/`send_control` cycles) carries the identical property; (2) M7-B06's own connection pooling (§D/§L — one QUIC connection per `(proxy, node)` pair, shared across every player routed through it) means a thrashing border pays only per-handoff protocol overhead on each crossing, never repeated QUIC/TLS handshake cost, once warm; (3) the system-level answer for a genuinely hot border is CLUSTER-D2/D8's own existing rebalancer mechanism, extended to entity-crossing frequency by a future rebalancer blueprint — the same deferral M4-B08 already made for the identical concern.

### §K — Concurrent events mid-handoff (damage, pickup)

No combat or item-pickup system exists at this project's own current scope. This blueprint's binding rule, restated from M4-B08 Part 1.7 and extended explicitly to the cross-node case: any future system targeting a player mid-handoff must look up its target fresh, every tick, by `RcEntityId` — never a cached `bevy_ecs::Entity` handle — and must tolerate finding no match for however many ticks the entity is genuinely absent from every `World`. No special-casing exists or is needed for the cluster-mode case beyond this already-established discipline.

### §L — Pre-warming (CLUSTER-D24)

M7-B06 §L states the exact scope boundary this blueprint fills: "that pre-emptive dial is initiated node-to-node via `rc-transport-net`'s own already-existing QUIC infrastructure... not by anything `rc-proxy` does." **What is pre-warmed, and why nothing else needs to be:** only the source node's own `NetworkTransport` connection to the anticipated destination node (Finding F2). Chunk data needs no pre-warming — CLUSTER-D18's shared-storage design already means a node that owns a region has it loaded via ordinary WORLD-D17 loading, independent of any player's movement. Entity-snapshot pre-transfer does not occur ahead of the crossing either — the snapshot is built and sent exactly once, at the moment of the actual crossing, never speculatively.

```rust
// crates/server/src/play/cluster_handoff.rs
pub const PREWARM_TRIGGER_RADIUS_CHUNKS: i32 = 2; // CLUSTER-D24's own exact value
pub const PREWARM_RELEASE_RADIUS_CHUNKS: i32 = 4; // this blueprint's own hysteresis band,
                                                    // cited: the same "don't re-trigger
                                                    // right at the boundary" pattern ARCH-D6
                                                    // already uses for merge/split.

/// Every neighboring region within `PREWARM_TRIGGER_RADIUS_CHUNKS` of `pos`'s own chunk,
/// owned by a node other than `node_directory.local_node_id()`, deduplicated. Probes the
/// 8 chunks at exactly `PREWARM_TRIGGER_RADIUS_CHUNKS` offset in each cardinal + diagonal
/// direction against `ownership.resolve` — reuses `RegionOwnership`'s own established
/// chunk-resolution interface (M3-B01/M4-B08) unmodified; no new spatial data structure.
pub fn prewarm_targets(
    pos: [f64; 3],
    dimension: rc_core::DimensionId,
    ownership: &rc_mechanics::border::RegionOwnership,
    node_directory: &dyn rc_transport_net::NodeDirectory,
) -> Vec<rc_transport_net::NodeId>;
```

### `rc-transport-net`'s own addition (Finding F2)

```rust
// crates/transport-net/src/transport.rs (MODIFY — one additive public method)
impl NetworkTransport {
    // ... every existing method (M7-B01 Deliverables) unchanged ...

    /// NEW (M7-B07, Finding F2). CLUSTER-D24's pre-warming requirement has no existing
    /// hook here — every prior connection-establishment path is purely lazy, triggered
    /// only by `send()` needing to reach a peer (M7-B01 §D/§H). This method triggers the
    /// identical internal dial/connector-task machinery `send()` already uses without
    /// sending any `RegionMessage` — a fire-and-forget, non-blocking hint. Idempotent: a
    /// no-op if a connection to `node` already exists or is already being dialed. Never
    /// blocks the caller (ARCH-D29's discipline, restated). Flagged for M7-B01's own next
    /// revision to formally absorb.
    pub fn prewarm(&self, node: &NodeId);
}
```

### §M — Explicit non-goals

This blueprint does not implement: `rc-proxy`'s ordinary connection forwarding table, NET-D6's cluster-mode execution, or any join-flow/first-assignment logic (M7-B06's own crate, unmodified beyond Finding F5); CLUSTER-D2/D16's rebalancer or takeover *decision* logic (M7-B05's own crate); a NET-D8 ingress adapter consuming `NodeAcceptor::try_recv`'s output, or any change to `rc-scheduler`/`rc-mechanics`'s Stage-11 encode path to call `NodeAcceptor::relay_sink` (both explicitly named by M7-B06 §N's own Needs-from list as unclaimed — this blueprint does **not** claim them either: they are a materially larger scope, "wire the entire node's normal packet flow to the proxy relay," than this task's own six-step-handoff-and-pre-warm brief, and would require touching `rc-scheduler`/`rc-mechanics`, which PLAN-D3 forbids for a cluster-mode blueprint without an extraordinarily well-justified finding this blueprint does not attempt to manufacture); any change to `rc-messaging`'s `EntitySnapshot` or any entity component schema; Bedrock-specific handoff branching. Building placeholder versions of any of these is out of scope, not a shortcut to take.

## Deliverables

### `crates/core/src/connection_id.rs` (NEW — Finding F3)

Exactly as given in Context §G.

### `crates/core/src/lib.rs` (MODIFY — one additive re-export line; every existing line unchanged)

```rust
mod connection_id;
pub use connection_id::ConnectionId;
```

### `crates/transport-net/src/transport.rs` (MODIFY — one additive public method, Finding F2)

Exactly as given in Context §L.

### `crates/proxy/src/node_acceptor.rs` (MODIFY — two additive public methods, Finding F5; every existing method, field, and derive unchanged)

`send_control`/`try_recv_control`/`SendControlError` exactly as given in Context §E.

### `crates/proxy/src/lib.rs` (MODIFY — one additive re-export line; every existing line unchanged)

```rust
pub use node_acceptor::{NodeAcceptor, RelaySink, SendControlError};
```

### `crates/server/src/play/player_transfer.rs` (MODIFY — one additive field, Context §F/Finding F4)

Adds `pub connection_id: Option<rc_core::ConnectionId>` to `PlayerTransferPayload` exactly as specified. Every other field, derive, and method unchanged.

### `crates/server/src/play/world.rs` (MODIFY — one additive field on `PlayerMarker`, Context §F)

`PlayerMarker` gains `pub connection_id: Option<rc_core::ConnectionId>`, defaulting `None` via the same `..Default::default()` precedent M4-B08 already used for `routing`/`tracked_entities`. No other change.

### `crates/server/src/play/cluster_handoff.rs` (NEW)

```rust
//! Node-side cluster-mode handoff machinery (M7-B07): crossing classification, the two
//! `NodeAcceptor::send_control` emission points, and pre-warm triggering. Compiled
//! unconditionally (mirrors `PlayerRouting`'s own established precedent) but inert
//! outside cluster mode. Zero changes to `rc-scheduler` anywhere in this file (PLAN-D3,
//! Context §A).

use bevy_ecs::prelude::*;
use rc_core::{ConnectionId, DimensionId};
use rc_mechanics::border::RegionOwnership;
use rc_messaging::{Address, RegionId};
use rc_proxy::{ControlFrame, NodeAcceptor, ProxyConnectionId};
use rc_transport_net::{NetworkTransport, NodeDirectory, NodeId};

pub const PREWARM_TRIGGER_RADIUS_CHUNKS: i32 = 2;
pub const PREWARM_RELEASE_RADIUS_CHUNKS: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossingKind {
    None,
    SameNode { dest_region: RegionId },
    CrossNode { dest_region: RegionId, dest_node: NodeId },
}

/// Context §C's pure classification core.
pub fn classify_player_crossing(
    pos: [f64; 3],
    dimension: DimensionId,
    ownership: &RegionOwnership,
    node_directory: &dyn NodeDirectory,
) -> CrossingKind;

/// Context §L's pure trigger core.
pub fn prewarm_targets(
    pos: [f64; 3],
    dimension: DimensionId,
    ownership: &RegionOwnership,
    node_directory: &dyn NodeDirectory,
) -> Vec<NodeId>;

/// Per-player dedup/hysteresis state for pre-warming (Context §L) — avoids redundant
/// `NetworkTransport::prewarm` calls every tick. `bevy_ecs::Component`, one per player.
#[derive(Component, Default, Debug, Clone)]
pub struct PrewarmState {
    // implementer's freedom: e.g. a small `HashSet<NodeId>` of currently-armed targets.
}

impl PrewarmState {
    pub fn new() -> Self;
    /// Returns exactly the subset of `current_targets` that should actually call
    /// `NetworkTransport::prewarm` this tick — already-armed targets inside the release
    /// radius are skipped; a target the player has retreated past the release radius
    /// from is disarmed, eligible to re-trigger on future re-entry.
    pub fn targets_to_prewarm(&mut self, current_targets: &[NodeId]) -> Vec<NodeId>;
}

/// Finding F1's own confirmation-recording resource, a small process-wide
/// `bevy_ecs::Resource` a cluster-mode composition root inserts (mirroring
/// `RegionOwnership`'s own "inserted by whichever code bootstraps the region"
/// precedent).
#[derive(Resource, Default, Debug, Clone)]
pub struct PendingHandoffConfirmations(pub Vec<ProxyConnectionId>);

/// `EntityArrivalDriver`-shaped (`fn(&mut World, Vec<rc_messaging::EntitySnapshot>)`,
/// M4-B08's own exact signature — no closures, no captured state, Context §D). A
/// cluster-mode composition root's own combined driver calls this after decoding a
/// `PlayerTransferPayload` arrival: pushes `payload.connection_id` (converted to
/// `ProxyConnectionId`, Context §G) into `PendingHandoffConfirmations` iff it is
/// `Some(_)` — a same-node arrival's `None` value is a deliberate no-op.
pub fn record_pending_confirmation(world: &mut World, connection_id: Option<ConnectionId>);

#[cfg(feature = "server-systems")]
pub mod ecs {
    use super::*;
    use rc_scheduler::{DomainGroup, RcExecutorBuilder};

    /// Registers this module's crossing-detection + pre-warm-trigger system into
    /// `DomainGroup::EntityPhysicsIntegration` (Stage 6b) — the same slot M4-B08's own
    /// monolithic player-crossing system occupies; a cluster-mode composition root
    /// registers this system *instead of*, never alongside, that one. On
    /// `CrossingKind::CrossNode`, stamps `payload.connection_id` and calls
    /// `NodeAcceptor::send_control(id, ControlFrame::HandoffBegin { connection_id: id,
    /// dest_node })` via a `Res<Arc<NodeAcceptor>>` this composition root inserts,
    /// alongside `Res<Arc<NetworkTransport>>` (for `prewarm`) and the ordinary
    /// `Res<RegionOwnership>`/`ResMut<RegionMessageOutbox>`/`Commands` M4-B08 already
    /// established.
    pub fn register_cluster_crossing_detection(builder: &mut RcExecutorBuilder);
}
```

### `crates/server/src/play/mod.rs` (MODIFY — one additive module declaration + re-export; every existing line unchanged)

```rust
mod cluster_handoff;
pub use cluster_handoff::{
    classify_player_crossing, prewarm_targets, record_pending_confirmation, CrossingKind,
    PendingHandoffConfirmations, PrewarmState, PREWARM_RELEASE_RADIUS_CHUNKS,
    PREWARM_TRIGGER_RADIUS_CHUNKS,
};
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46), restated exactly.** Every file below, plus every `src/*.rs` file named in Deliverables with executable bodies replaced by `todo!()` (fields, derives, doc comments, and every signature unchanged), plus the four cited additive-field/re-export edits, is the test-authoring changeset, committed first. The implementation changeset fills in bodies only — it must not modify any already-merged test file anywhere in the workspace, in particular no file under `crates/proxy/tests/` from M7-B06 and no file under `crates/transport-net/tests/` from M7-B01.

### `crates/proxy/tests/node_acceptor_control_channel.rs` (NEW, integration — real `NodeAcceptor` + `ProxyServer`, reusing M7-B06's own `tests/support/mod.rs` — `spawn_fake_node`, `generate_test_tls` — unmodified)

1. `send_control_reaches_the_proxy_and_ready_flips_routing` — a `spawn_fake_node`-backed `NodeAcceptor` connected to a real `ProxyServer` with one live relayed connection routed to it; the test calls `send_control(id, ControlFrame::HandoffBegin { connection_id: id, dest_node: some_other_node })`, then, from a second `spawn_fake_node`, calls `send_control(id, ControlFrame::HandoffReady { connection_id: id })`; asserts `ProxyServer::routing_table().current_node(id)` equals the second node's own id, matching M7-B06's own already-tested `complete_handoff` behavior — proving `send_control` is a correct, real client of that already-shipped mechanism, not a parallel one.
2. `try_recv_control_observes_handoff_complete` — after case 1's sequence completes (the proxy sends `HandoffComplete` to the first node per M7-B06's own shipped reaction), the first node's `try_recv_control()` returns `Some((id, ControlFrame::HandoffComplete { .. }))`.
3. `send_control_to_unknown_connection_id_errors_not_panics` — `send_control` called with a `ProxyConnectionId` this `NodeAcceptor` has never seen; returns `Err(SendControlError::UnknownConnection(_))`, never panics.
4. `try_recv_control_is_none_when_nothing_pending` — a freshly constructed `NodeAcceptor` with no inbound control traffic; `try_recv_control()` returns `None`.
5. `pre_existing_m7_b06_suite_still_passes` — not a new test case; this file's own header comment restates that `cargo nextest run -p rc-proxy` running M7-B06's own 12 pre-existing cases unchanged is part of this blueprint's own Done-definition, verified by CI rather than duplicated here.

### `crates/server/tests/cluster_crossing_classification.rs` (pure — `classify_player_crossing`/`prewarm_targets` directly)

1. `same_region_is_none` — a position whose chunk resolves to `ownership.local`; returns `CrossingKind::None`.
2. `cross_region_same_node_is_same_node` — destination region resolves to a different `RegionId` but the same `NodeId`; returns `CrossingKind::SameNode { dest_region }`.
3. `cross_region_different_node_is_cross_node` — destination region resolves to a genuinely different `NodeId`; returns `CrossingKind::CrossNode { dest_region, dest_node }` matching exactly.
4. `unknown_region_owner_defers_as_none` — `node_directory.resolve` returns `None` for the destination region; returns `CrossingKind::None`.
5. `prewarm_targets_finds_neighbors_within_radius_and_excludes_local_node` — three distinct neighbor regions at chunk offsets `1`, `2`, `3`, two owned by distinct remote nodes and one by the local node; asserts exactly the two remote nodes are returned, deduplicated.

### `crates/server/tests/prewarm_trigger_and_release.rs`

1. `trigger_fires_once_within_radius` — first call to `targets_to_prewarm` within `PREWARM_TRIGGER_RADIUS_CHUNKS` returns the target; an immediately repeated call at the same position returns `[]`.
2. `no_trigger_outside_radius` — `prewarm_targets` at `PREWARM_TRIGGER_RADIUS_CHUNKS + 1` returns `[]`.
3. `release_and_re_trigger_hysteresis` — after triggering, a position beyond `PREWARM_RELEASE_RADIUS_CHUNKS` disarms the target; re-entering the trigger radius re-triggers it.
4. `distinct_targets_tracked_independently` — two distinct remote nodes within trigger radius simultaneously, each independently re-triggerable per its own release/re-entry.

### `crates/server/tests/play_cluster_handoff_walk.rs` (integration — the task's own required full harness, criterion 1)

Fixture: two real `NetworkTransport` instances (node X, node Y, mirroring M7-B01's own localhost-QUIC/`rcgen`-TLS test pattern) each registered with one region; two real `NodeAcceptor`s (mirroring M7-B06's own `spawn_fake_node`); one real `ProxyServer`; a real loopback player connection joined in West territory via `ProxyServer`'s own player-facing socket, mirroring M7-B06's own `FakeVanillaClient` login-through-proxy pattern.

`player_crosses_a_live_node_boundary_within_budget_with_bounded_position_delta`:

1. The test drives the same 64-step, `+0.5`-per-tick scripted westward-to-eastward walk `play_region_transfer_player_walk.rs` (M4-B08) already established, across a boundary now owned by two distinct `NetworkTransport`/`RcExecutor`/`NodeAcceptor` triples instead of two regions in one process.
2. Records the same position-delta log M4-B08's own harness defines, asserting the identical continuity properties: at most one `None` tick, consecutive `Some` deltas exactly `0.5`, no discontinuity beyond the one-tick in-flight window.
3. Instruments `Instant::now()` at the moment `send_control(Begin)` is called (a test-only hook on `NodeAcceptor`) and at the moment `ProxyServer::routing_table().current_node(id)` first observes the flip to Y (polled); asserts the delta is `< Duration::from_millis(HANDOFF_BUDGET_MS)` (100) — the literal, direct measurement criterion 1 requires, over real QUIC sockets.
4. Asserts the connection's own loopback socket never receives a disconnect at any point during the crossing.

### `crates/server/tests/fault_injection_node_side_boundaries.rs` (5 cases, Context §E's own fault table)

1. `begin_send_failure_retries_and_eventually_succeeds` — the test's own `NodeAcceptor` test hook drops the first `send_control(Begin)` deterministically; asserts the source node retries per the cited backoff schedule and the handoff eventually completes.
2. `destination_crash_before_ready_is_absorbed_by_takeover_unaffected_here` — after the destination's `EntityArrivalDriver` applies the arrival but before the post-`tick_region` hook fires, the test drops that node's `NetworkTransport`/`NodeAcceptor` entirely; asserts no panic anywhere and that this blueprint's own bookkeeping (`PendingHandoffConfirmations`) is not left in a state that causes a later, unrelated confirmation to be misattributed.
3. `handoff_complete_never_arriving_is_garbage_collected` — the test's own `NodeAcceptor` hook drops every `HandoffComplete` deterministically; asserts the source node's own local pending-confirmation bookkeeping is cleared after its own bounded window and this has zero effect on simulation correctness.
4. `stale_dest_node_at_send_time_degrades_to_ordinary_stall` — `classify_player_crossing` resolves a `dest_node` that the test then immediately reassigns (simulating the directory-race row in Context §E); asserts `send_control(Begin)` still succeeds (it is not the proxy's job to validate `dest_node` freshness at receipt, M7-B06's own design) and the eventual stall is absorbed by M7-B06's own already-tested rollback path, not by any new mechanism this blueprint adds.
5. `same_node_crossing_never_calls_send_control` — a scripted crossing between two regions both owned by the same node; asserts the test's own `send_control` call-count instrumentation observes **zero** calls for the entire crossing, confirming `CrossingKind::SameNode` never touches `NodeAcceptor` at all.

### `crates/server/tests/ping_pong_stress_node_side.rs`

`ping_pong_stress_node_side_no_leak_or_stuck_state` — 500 iterations of alternating `classify_player_crossing` results (cross-node, then cross-node back) for one fixed player, each immediately followed by a simulated arrival (`record_pending_confirmation`) and hook-driven `send_control(Ready)`; after every iteration, asserts `PendingHandoffConfirmations` is empty and no panic occurred; after all 500 iterations, asserts the test's own `send_control` call log shows exactly one `Begin` and one `Ready` per iteration, in strict alternation, with no leaked or duplicated entries.

## Implementation steps

1. **`rc-core`.** Add `connection_id.rs`, the `lib.rs` re-export. Observable: `cargo build -p rc-core` succeeds; every pre-existing `rc-core` test unaffected.
2. **`rc-transport-net`.** Add `NetworkTransport::prewarm` (Finding F2), reusing `connection.rs`'s existing per-node connector-task spawn point. Observable: every pre-existing M7-B01 test still passes; this blueprint's own small additive `crates/transport-net/tests/prewarm.rs` (`prewarm_is_idempotent_and_non_blocking`) passes.
3. **`rc-proxy`.** Add `NodeAcceptor::send_control`/`try_recv_control`/`SendControlError` (Finding F5), reusing the control-stream object `NodeAcceptor` already privately owns (M7-B06's own `node_acceptor.rs` internals — implementer's freedom on the exact private field this reaches into, per the blueprint spec's own "internal helpers are the implementer's freedom" allowance). Observable: all 12 pre-existing M7-B06 `rc-proxy` tests still pass unchanged; `node_acceptor_control_channel.rs` (4 new cases) passes.
4. **`rusty-clanker-server` — `player_transfer.rs`/`world.rs`.** The two additive fields (Finding F4). Observable: every pre-existing `rusty-clanker-server` test (M1-B05 through M4-B08) still compiles and passes unchanged.
5. **`rusty-clanker-server` — `cluster_handoff.rs`.** `classify_player_crossing`, `prewarm_targets`, `PrewarmState`, `PendingHandoffConfirmations`, `record_pending_confirmation`, `ecs::register_cluster_crossing_detection`. Observable: `cluster_crossing_classification.rs` (5 cases) and `prewarm_trigger_and_release.rs` (4 cases) pass.
6. **`rusty-clanker-server` — `play/mod.rs`.** Module declaration + re-exports. Observable: `cargo build -p rusty-clanker-server --all-features` succeeds.
7. **Integration harness, criterion-1, fault-injection, ping-pong suites.** `play_cluster_handoff_walk.rs`, `fault_injection_node_side_boundaries.rs`, `ping_pong_stress_node_side.rs` — new integration-test-only files, no production code changes beyond steps 1–6. Observable: all cases pass on both OS legs.
8. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
9. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly as restated above. No already-merged test file anywhere in the workspace is touched, including every file under `crates/proxy/tests/` from M7-B06.

(b) **PLAN-D3 is binding: zero changes to `rc-scheduler`, `rc-mechanics`, or `rc-messaging`.** Finding F1's own resolution exists specifically to satisfy this constraint without weakening CLUSTER-D22's intent.

(c) **No new external dependencies.** Every crate this blueprint touches already carries every dependency this blueprint needs; no new `[workspace.dependencies]` entry is added.

(d) **Every named finding (F1–F5) is a documented, cited, bounded correction or additive extension — never a silent edit.** F1 corrects an internal inconsistency present in both `13-cluster-architecture.md`'s own text and M7-B06's restatement of it. F2 adds one method to `NetworkTransport`. F3 adds one newtype to `rc-core`. F4 adds one `Option<_>`-typed, `None`-by-default field each to M4-B08's `PlayerTransferPayload`/`PlayerMarker`, directly fulfilling M7-B06 §N's own Needs-from item 4. F5 adds two methods to M7-B06's own `NodeAcceptor`, directly fulfilling the gap that blueprint's own text names ("out of this blueprint's own crate boundary").

(e) **`rc-proxy`'s own already-shipped mechanism (`ProxyRoutingTable`, `ControlFrame`'s existing variants, identity forwarding, directory-watch) is never re-implemented, duplicated, or modified beyond Finding F5's two additive methods.** An implementer who finds themselves writing a second buffering/flip/flush state machine, a second `ControlFrame`-shaped enum, or a second QUIC control channel has misread Context §A and must reread it before proceeding.

(f) **No Mojang or third-party reimplementation code.** Every mechanism here is derived solely from `13-cluster-architecture.md`'s CLUSTER-D7/D9/D10/D17/D19/D22/D24, `01-server-architecture.md`'s ARCH-D9/D10/D12/D24/D29, and this blueprint's own cited resolutions of gaps M4-B08/M7-B01/M7-B06's own texts left open (ASSET-D18/D19/D30).

(g) **Scope boundary.** This blueprint does not implement: `rc-proxy`'s ordinary connection forwarding table or NET-D6 execution (M7-B06's own crate); CLUSTER-D2/D16's rebalancer/takeover decision logic (M7-B05's own crate); a NET-D8 ingress adapter or any Stage-11 encode-path change (explicitly named, explicitly unclaimed, Context §M); any position/decision-level ping-pong suppression mechanism (Context §J's deliberate non-mechanism); Bedrock-specific branching. Do not add placeholder implementations of any of these as a shortcut.

(h) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-proxy -p rc-transport-net -p rc-core -p rusty-clanker-server --all-features
cargo nextest run -p rc-proxy -p rc-transport-net -p rc-core -p rusty-clanker-server
cargo test --doc -p rc-proxy -p rc-transport-net -p rc-core -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run` across the four crates additionally runs: M7-B06's own 12 pre-existing `rc-proxy` cases (unchanged) + 4 (`node_acceptor_control_channel.rs`) + 1 (`prewarm.rs`) + 5 (`cluster_crossing_classification.rs`) + 4 (`prewarm_trigger_and_release.rs`) + 1 (`play_cluster_handoff_walk.rs`) + 5 (`fault_injection_node_side_boundaries.rs`) + 1 (`ping_pong_stress_node_side.rs`) = 33 total `rc-proxy`-adjacent cases plus this blueprint's own 21 new cases across `rc-transport-net`/`rusty-clanker-server`, alongside every pre-existing test in all four crates, all still passing. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
