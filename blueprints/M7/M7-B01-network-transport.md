# M7-B01 — Network Transport (`NetworkTransport`, QUIC/postcard)

| Field | Content |
|---|---|
| ID | M7-B01 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | M0-B02 (`rc-messaging` — the exact `Transport` trait, `TransportError`, `Message<T>`, `Address`, `RegionId`, `RegionMessage`/`BorderUpdateEvent`/`BorderUpdateKind`/`EntitySnapshot` this blueprint implements against unmodified; restated in full below). M0-B03 (`rc-transport-inproc::InProcessTransport` — the semantics reference: FIFO/exactly-once per `(from, to)`, the register/deregister calling convention, the `Address::Entity`/`Address::Chunk` out-of-scope resolution this blueprint reuses verbatim; restated in full below). M0-B01 (workspace scaffold — `crates/transport-net/` already exists as an empty-shell crate with `Cargo.toml`'s only dependency `rc-messaging = { path = "../messaging" }` and `src/lib.rs` holding only a doc comment; `rusty-clanker-server`'s `Cargo.toml` **already** carries `rc-transport-net = { path = "../transport-net", optional = true }` under its `cluster` feature — this blueprint does not touch `rusty-clanker-server` at all, restated under Context/§L). |
| Implements | CLUSTER-D9 (mid-migration message routing, resolved entirely inside this crate); CLUSTER-D10 (unmodified reuse of `RegionTransferRequest`, confirmed — no new `RegionMessage` variant); CLUSTER-D11 (QUIC transport, `quinn` 0.11.11, per-pair stream mapping, mutual TLS); CLUSTER-D12 (`postcard` 1.1.3 wire serialization); ARCH-D26/D29 (the `Transport` trait contract this crate satisfies with a second implementation); PERF-D30 (confirming `quinn-udp`'s own GSO batching needs no engine-level code); PERF-D31 (cross-node application-level message packing); WS-D3 Rule 2 (dependency-direction isolation — this crate never depends on `rc-scheduler`/`rc-cluster`); WS-D5(a) (cluster Cargo-feature gating, already wired by M0-B01, restated); TEST-D27/D45/D46/D50 (property-test toolchain, test-first changeset boundary, CI authority). |
| Crates touched | `rc-transport-net` (`crates/transport-net/`) only. Root `Cargo.toml`'s `[workspace.dependencies]` table gains exactly one new line (`rcgen`, dev-tooling for this crate's own test TLS material — see Deliverables). No other crate, including `rusty-clanker-server`, `rc-cluster`, or `rc-proxy`, is modified. |
| Estimated scope | L |

## Goal & Done definition

Give `rc-transport-net` a complete, real `NetworkTransport` — the cluster-mode `Transport` implementation ARCH-D26/CLUSTER-D11 name — built on `quinn` 0.11.11 QUIC connections (one per node pair, mutually TLS-authenticated) and `postcard` 1.1.3 wire serialization, satisfying the identical FIFO/exactly-once-per-`(from, to)`-pair contract `InProcessTransport` (M0-B03) already proves, this time over real sockets across process boundaries. Concretely: (1) the exact `Transport` trait signature, unmodified; (2) one QUIC stream per ordered `(from: RegionId, to: Address)` pair, opened lazily, closed on region deregistration or destination-node migration; (3) a narrow `NodeDirectory` trait this crate *consumes* for the `RegionId -> NodeId` hop (CLUSTER-D9), implemented by a future `rc-cluster` blueprint, never the reverse dependency; (4) per-pair application-level batching (PERF-D31) capped at 16 KiB per flush; (5) a same-node short-circuit for two locally-owned regions (a necessary consequence of "one `Transport` impl per process" this blueprint makes explicit, since 13 does not); (6) a node-to-node control side-channel closing CLUSTER-D9's residual staleness race; (7) an observability seam (`NetworkTransportMetricsSink`) a future composition-root blueprint bridges to `rc-scheduler::metrics::MetricsRegistry` (M6-B02), never a direct dependency; (8) the full M0-B02/M0-B03 FIFO/exactly-once property suite re-run against two real `NetworkTransport` instances over localhost QUIC, plus migration/redirect/batching/two-process coverage this blueprint adds.

Done when:

- [ ] `cargo build -p rc-transport-net --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-transport-net` on both OS legs.
- [ ] `network_transport_fifo_and_exactly_once_over_quic` (the full M0-B02/M0-B03 property re-run, real localhost QUIC sockets) passes — FIFO per `(from, to)`, exactly-once, no loss under a proptest-driven multi-sender load.
- [ ] `migration_redirects_stream_to_new_owner_within_same_tick` and `stale_directory_triggers_not_owner_and_self_heals` both pass.
- [ ] `batch_flush_splits_at_byte_cap_without_loss_or_reorder` passes.
- [ ] `same_node_regions_short_circuit_without_quic` passes (a message between two locally-registered regions never touches a socket — asserted via a zero-open-streams check on the local node's own connection table).
- [ ] `two_process_smoke_real_sockets` passes: a spawned second OS process (this crate's own `echo_peer` test-support binary) and the test's own in-process `NetworkTransport` exchange messages over real loopback UDP/QUIC.
- [ ] `cluster_feature_absence_removes_crate_from_dependency_graph` passes: `cargo metadata --no-default-features --features monolithic -p rusty-clanker-server` (already wired by M0-B01, unmodified) resolves with zero `rc-transport-net` package or dependency-graph node.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's new normal dependencies (`quinn`, `postcard`, `rustls`, `tokio`, `crossbeam-channel`, `parking_lot`, `tracing`, `serde`, `thiserror`) touch no `xtask lint-deps` rule: Rule 2 already places `rc-transport-net` in `NETRENDER`, unaffected by any of these; Rule 3 (`rc-messaging`'s exact dependency set) is untouched since this blueprint never modifies `rc-messaging`.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-transport-net` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### §A — Scope boundary and the dependency-direction rule this whole design obeys

`12-workspace-structure.md`'s WS-D3 Rule 2 places `rc-transport-net` in the `NETRENDER` set alongside `rc-transport-inproc`/`rc-auth`/`rc-cluster`/`rc-proxy`/`rc-render`/`rc-protocol`, and forbids either direction of reachability between `NETRENDER` and `SIM = [rc-scheduler, rc-mechanics]`. The same document's own dependency-graph table additionally fixes the direction *within* `NETRENDER` that matters most here: `rc-cluster --> rc-transport-net` and `rc-proxy --> rc-transport-net` — `rc-cluster` (the future blueprint owning `openraft`/`redb`, CLUSTER-D13) and `rc-proxy` (the future blueprint owning CLUSTER-D20–D24) both depend **on** this crate; this crate depends on neither, ever. Every design choice below that might otherwise reach for "just ask the raft directory" or "just call into the metrics registry" is shaped by this one rule: this crate defines narrow traits it *consumes*; a later M7 blueprint implements them. This blueprint's own crate list is exactly `rc-transport-net` — it does not create `rc-cluster`, `rc-proxy`, or touch `rusty-clanker-server`'s already-correct feature wiring (§L).

### §B — The exact `Transport` trait this crate implements a second time (ARCH-D26/D29, restated)

Copied verbatim from `rc-messaging` (M0-B02, unmodified by this or any blueprint):

```rust
pub trait Transport: Send + Sync + 'static {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError>;
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>>;
}
```

`Message<T> { from: RegionId, to: Address, tick_stamp: u64, seq: u32, payload: T }`, `Address = Region(RegionId) | Entity(RcEntityId) | Chunk(ChunkKey)`, `RegionMessage = BorderUpdateEvent(BorderUpdateEvent) | RegionTransferRequest(Box<EntitySnapshot>)`, `TransportError::Backpressure(Message<RegionMessage>)` (the only variant, carrying the undelivered message back per M0-B02's own "give the value back" convention). ARCH-D29's guarantees this implementation must uphold, restated exactly as M0-B03 already proved for `InProcessTransport`: **(1) FIFO per `(from, to)` pair** — for `m1` sent before `m2` with the same `(from, to)`, `try_recv` never returns `m2` before `m1`; **(2) exactly-once, process lifetime** — no message lost or duplicated while both endpoints stay alive (CLUSTER-D17 is this blueprint's own confirmation that "process lifetime" is the whole guarantee — no cross-restart delivery promise, see §K); **(3) no ordering across different pairs**; **(4) never blocks the sender** — a message that cannot currently be delivered returns `Backpressure` rather than blocking (§I enumerates every case that triggers it for this implementation specifically).

CLUSTER-D10 confirms no new `RegionMessage` variant is needed for cluster-mode entity handoff — `RegionTransferRequest` crosses this transport completely unmodified, same as `BorderUpdateEvent`. This blueprint adds **zero** variants to `RegionMessage` and touches **zero** files in `rc-messaging`.

### §C — `InProcessTransport`'s semantics as this crate's reference (M0-B03, restated)

`InProcessTransport` resolves `Address::Region(id) => id` directly in `send`; for `Address::Entity`/`Address::Chunk` it returns `Backpressure` immediately, never panics, because the ARCH-D24 `ChunkKey -> RegionId`/`RcEntityId -> RegionId` directories belong to `rc-scheduler`, a crate `rc-transport-inproc` cannot depend on (Rule 2). The identical constraint applies to this crate for the identical reason (§A) — **`NetworkTransport` resolves only `Address::Region`; `Address::Entity`/`Address::Chunk` return `Backpressure(msg)` immediately**, same unification M0-B03 already established ("cannot deliver this right now" is exactly what a caller holding an unresolvable address needs to hear). `register_region`/`deregister_region` are never called by this crate itself — they are called by whichever future composition-root/`rc-cluster` call site executes an ARCH-D6 split/merge or a CLUSTER-D16 takeover, in lockstep with that same call site's raft-directory commit, by calling convention rather than by this crate reaching into `rc-cluster` — the exact non-ownership M0-B03 already established for `rc-scheduler`'s ARCH-D6 split/merge calls, restated here for the cluster case.

### §D — QUIC transport: `quinn` 0.11.11, connection and stream shape (CLUSTER-D11, restated and made concrete)

CLUSTER-D11, verbatim: "QUIC via the `quinn` crate, version 0.11.11 (crates.io, published 2026-06-22, MSRV Rust 1.85; verified current as of this writing), rustls backend, one persistent multiplexed QUIC connection per (node, node) pair and per (proxy, node) pair — never per player. Every ordered `(from: RegionId | ConnectionId, to: Address)` pair that ARCH-D29 requires FIFO/exactly-once for is mapped to its own QUIC stream within that shared connection, opened lazily on first message and closed on region merge/split/migration." Re-verified for this blueprint (2026-08-21): `quinn` `0.11.11` is current on crates.io; its rustls integration is via the adapter types `quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig}` wrapping an already-built `rustls::ClientConfig`/`rustls::ServerConfig` (`TryFrom`), which are then handed to `quinn::ClientConfig::new(Arc<dyn crypto::client::ClientConfig>)`/`quinn::ServerConfig::with_crypto(Arc<dyn crypto::ServerConfig>)`. **Moderate-confidence flag** (this corpus's own established convention, M0-B04/M6-B02 precedent): the exact adapter module path and whether `quinn`'s default Cargo features already pull a working rustls crypto backend (`rustls-ring` vs `rustls-aws-lc-rs`) plus `runtime-tokio` were not independently re-verified byte-for-byte against `0.11.11`'s published feature table at blueprint-writing time — confirm via `cargo doc -p quinn` at implementation time; if the crate's defaults do not already include a crypto backend + Tokio runtime integration, add the verified feature names explicitly to this crate's own `Cargo.toml` `quinn` entry (a scoped amendment, not a change to `12`'s workspace-level version pin) and note the finding in this blueprint's own PR description per CLAUDE.md's "any needed change to an existing seam is a finding, not a silent edit."

This crate maps CLUSTER-D11's "per (node, node) pair" connection exactly: **one `quinn::Endpoint`** per `NetworkTransport` instance, serving both roles (accepting inbound connections as a QUIC server and dialing outbound as a QUIC client — `quinn::Endpoint::server`/`Endpoint::client`-shaped construction is not used separately; this crate builds one `Endpoint` via `Endpoint::new(EndpointConfig, Some(server_config), socket, runtime)` so the same UDP socket serves both directions, matching CLUSTER-D27's single `bind` config field). Per remote `NodeId`, **at most one** live `quinn::Connection` is held (dialed lazily, §H); every `(from: RegionId, to: Address)` pair destined for that peer maps to its **own unidirectional QUIC stream** (`open_uni`) within that one connection — unidirectional because `RegionMessage` traffic within one ordered pair only ever flows sender-to-receiver (unlike CLUSTER-D23's proxy↔node control channel, which this crate does not implement). One additional **bidirectional** stream per connection, opened once at connection establishment, carries this crate's own small control-frame protocol (§J) — a second channel, never mixed with `RegionMessage` batch bytes on the same stream, mirroring CLUSTER-D23's "separate protocol, separate stream" pattern by analogy (CLUSTER-D23 itself governs only the *proxy*↔node channel; this crate's node↔node control stream is this blueprint's own extension of that same design pattern, not CLUSTER-D23 itself — restated as a Constraint, §M).

### §E — Wire serialization: `postcard` 1.1.3 and this blueprint's length-prefix framing (CLUSTER-D12, restated and completed)

CLUSTER-D12: every `RegionMessage` variant already derives `serde::Serialize`/`Deserialize` (ARCH-D25, M0-B02) — `postcard` consumes those derives directly, zero additional derive burden, zero modification to `rc-messaging`'s types. `postcard`'s own encoding has no self-terminating end marker a QUIC byte-stream reader can rely on without first knowing a boundary, so this blueprint fixes the missing piece CLUSTER-D12 leaves open: **every flushed batch on a `(from, to)` pair's stream is a fixed 4-byte little-endian `u32` byte-length prefix (deliberately a plain fixed-width integer, not `postcard`'s own variable-width varint, so a reader can allocate its receive buffer before parsing anything variable-width) immediately followed by exactly that many bytes of `postcard::to_allocvec(&Vec<Message<RegionMessage>>)` output.** This is the concrete, cited resolution PERF-D31's own text explicitly leaves at "postcard-encoded, length-prefixed batch" without specifying the prefix's own shape — this blueprint's decision, not a deviation from it. The control stream (§D/§J) uses the identical `[u32 LE length][postcard bytes]` framing for its own, much smaller `ControlFrame` values, for consistency and code reuse (one `wire::write_framed`/`wire::read_framed` pair serves both).

### §F — Node identity and mutual TLS (CLUSTER-D11/D20/D27, restated and completed)

CLUSTER-D11: connections are "mutually authenticated via TLS 1.3 client+server certs issued by one operator-supplied cluster-internal CA (config-provided cert/key paths; distribution/rotation mechanics are an operational concern, not specified further here)." CLUSTER-D27's own example config table shows `node_id = "node-a"` (a **string**, not an integer — "stable identity, persisted across restarts") and `ca_cert = "/etc/rustyclanker/cluster-ca.pem"`, plus `node_cert`/`node_key` for *this* node's own certificate/key path. This blueprint realizes that shape concretely: **`NodeId` is a cheap-to-clone newtype over `Arc<str>`** (never a `u64` — matching CLUSTER-D27's own string literal exactly, and cheap because every `send()` call clones a `NodeId` out of the directory lookup on the hot path), and **this blueprint's own `TlsMaterial` struct carries the node's own cert chain and private key alongside the trusted CA**, filling the config surface CLUSTER-D27 names (`node_cert`/`node_key` fields alongside `ca_cert`). Server-side `rustls::ServerConfig` is built with a client-certificate verifier trusting only the cluster-internal CA (`rustls::server::WebPkiClientVerifier`) plus `with_single_cert(node_cert_chain, node_key)`; client-side `rustls::ClientConfig` trusts the same CA via `with_root_certificates` and additionally presents its own cert via `with_client_auth_cert` — true mutual authentication in both directions, matching CLUSTER-D11's "client+server certs" wording exactly (every node is simultaneously a TLS client, when dialing, and a TLS server, when accepted). CA certificate *issuance*/*distribution*/*rotation* stay exactly as `13`'s own Open Questions leave them — an operational concern this blueprint does not solve, only consumes the resulting PEM/DER bytes.

This crate never calls `rustls::crypto::CryptoProvider::install_default()` itself — installing a process-wide default crypto provider is inherently a do-this-exactly-once-per-process operation, and a library crate silently doing it risks a panic if another crate (e.g. a future `rc-auth`/`reqwest`-based caller, NET-D6) also installs one. **This blueprint's own resolution:** `NetworkTransport::new` takes already-constructed `rustls::ServerConfig`/`ClientConfig` material (via `TlsMaterial`'s raw cert/key bytes, which this crate turns into `rustls::ServerConfig`/`ClientConfig` using whichever `CryptoProvider` is already installed process-wide) — the **composition root** (a future M7 blueprint's `main.rs`) is responsible for calling `install_default()` exactly once before constructing anything TLS-related, stated here as a binding contract on that future caller (Constraints, §M).

### §G — The Tokio↔RC-WorkerPool boundary, restated fresh for this crate (ARCH-D21/D22)

ARCH-D21: "All network I/O runs on a separate, isolated Tokio multi-thread runtime... This runtime's threads never execute RC-WorkerPool work and vice versa." QUIC inter-node traffic **is** network I/O under ARCH-D21's own umbrella wording ("all network I/O"), so this blueprint's own resolution is to **share the single ARCH-D21 Tokio runtime**, not spin up a second, competing one — `NetworkTransport::new` takes a `tokio::runtime::Handle` supplied by its caller (the future composition root, which already owns ARCH-D21's runtime instance for NET-D7's player-connection layer) rather than constructing or owning a `Runtime` itself. `01-server-architecture.md`'s own Message-Passing Substrate section is explicit that the `RegionMessage` substrate's sync/async boundary is "distinct from — and does not share channels, threads, or types with — ARCH-D22's Tokio↔RC-WorkerPool boundary, which carries player packet events, not `RegionMessage` traffic" — this blueprint honors that distinctness at the **channel** level (its own, separate `crossbeam-channel` instances per region, §H) while honoring ARCH-D21's "one isolated runtime for all network I/O" at the **runtime** level, by sharing the OS-thread pool but never the channels or message types. `send()`/`try_recv()` are always called from RC-WorkerPool threads (Stage 10/Stage 1, per M0-B02's own contract) and never themselves execute on, block, or `.await` anything on the shared Tokio runtime — every actual QUIC read/write happens inside a spawned Tokio task, communicating with the sync side only through bounded channels, exactly `InProcessTransport`'s own boundary discipline extended across a real async runtime.

### §H — The `NodeDirectory` seam and connection lifecycle (CLUSTER-D9/D5, resolved)

This crate defines, and depends only on, a narrow trait for the `RegionId -> NodeId` hop:

```rust
pub trait NodeDirectory: Send + Sync + 'static {
    fn local_node_id(&self) -> &NodeId;
    fn resolve(&self, region: RegionId) -> Option<NodeId>;
    fn node_address(&self, node: &NodeId) -> Option<SocketAddr>;
}
```

A future `rc-cluster` blueprint implements this over its own raft-committed, already-warm in-memory directory (CLUSTER-D5/D13) and hands an `Arc<dyn NodeDirectory>` to `NetworkTransport::new` — never the reverse dependency (§A). **This crate's own resolution of CLUSTER-D9's "holds its own soft cache" wording:** rather than duplicating a second cache inside `NetworkTransport`, this crate calls `directory.resolve(region)` fresh on **every** `send()` call, trusting the trait's own binding contract (stated in its doc comment, enforced only by that future implementation's own design, not by this crate) that a conforming implementation answers from already-committed, already-local in-memory state — never a network round trip, never a lock held across an `.await` — in effectively O(1) time. This keeps exactly one source of directory freshness (whichever crate owns CLUSTER-D13's raft state machine) instead of two independently-staling caches, and is why `resolve()`'s changing answer alone is enough to redirect traffic **within the same tick**: Stage 10 (M0-B02's contract) calls `Transport::send` once per message in a tight synchronous loop with no intervening `.await`; if the very first `send()` in that loop observes a fresh `resolve()` result different from a pair's currently-open stream target, this crate closes the stale stream and opens a fresh one to the new node **before processing the next message in the same loop** — no waiting for an async redial signal, satisfying CLUSTER-D9's "redialed... within the same tick if possible" as a direct, synchronous consequence of never caching.

**Same-node short-circuit (this blueprint's own necessary addition, not stated by `13`):** ARCH-D26 wires exactly one `Transport` implementation per process; a cluster node process owns multiple regions (CLUSTER-D1: assignable unit is one region, a node typically owns several). When `resolve(to_region) == Some(local_node_id)` and `to_region` is currently registered locally, `send()` delivers the message directly into that region's own local inbound `crossbeam-channel` queue (§I) — **no QUIC connection, no stream, no serialization** — reusing the identical in-memory hop `InProcessTransport` already performs, since two regions the same node owns communicating over a network loopback would be pure overhead and would not actually change ARCH-D29's guarantee, only its cost. This is what makes CLUSTER-D6's "the ARCH-D11 border-halo mechanism is reused *unmodified* across a node boundary" true without penalizing the common case where the boundary is not actually crossed.

**Residual staleness — the control-frame defense.** `resolve()`'s freshness is bounded only by whatever propagation delay the future raft-directory implementation carries — a real, if narrow, race exists where this node's own view is still stale at the exact instant CLUSTER-D5's authoritative commit already moved a region elsewhere. When a receiving node gets a `RegionMessage` batch addressed to a `RegionId` it does not currently have registered, it replies on the connection's control stream (§D/§J) with `ControlFrame::NotOwner { region }`. On receipt, the **sending** side does not retry the specific already-sent message (no cross-restart or cross-race delivery guarantee is ever promised, CLUSTER-D17) but records `region` as "known-stale as observed against `NodeId`" for a short grace window — **`NOT_OWNER_GRACE_WINDOW = Duration::from_millis(200)`**, a seed default pending real load-testing calibration like every other numeric threshold this corpus already carries (ARCH-D6/D19's own framing) — during which a subsequent `send()` to the same region, if `resolve()` still returns the identical stale `NodeId`, short-circuits straight to `Backpressure` rather than repeating an already-known-wrong network write; once the window expires, or `resolve()` returns a different value, normal resolution resumes. `on_directory_redirect` (§K) fires whenever this path is taken, giving a future `rc-cluster` blueprint an observable signal that its own directory propagation lagged reality.

### §I — Message batching and coalescing (PERF-D30/D31, restated and made concrete)

PERF-D30 confirms `quinn-udp`'s own GSO batching needs no engine-level code — this blueprint adds none, and never disables it. PERF-D31: every `RegionMessage` a region emits in one tick destined for the same `(from, to)` pair is buffered and flushed as one length-prefixed `postcard` batch via a single `stream.write_all()` call, capped at **`DEFAULT_BATCH_BYTE_CAP = 16 * 1024`** bytes (a seed default, configurable), splitting into an early flush plus a fresh batch when a pending message would exceed the cap rather than growing an unbounded backlog. Because `Transport::send` takes one message at a time (§B — unmodified from M0-B02), this crate cannot literally hook "the same tick-boundary dispatch point NET-D8 already uses" (a protocol-layer concept this crate has no visibility into); **this blueprint's own concrete resolution:** each `(from, to)` pair owns a small in-memory pending buffer plus a `tokio::sync::Notify`; `send()` (running synchronously on an RC-WorkerPool thread) pushes the message into the buffer and calls `notify_one()` — an idempotent, non-blocking wake — then returns immediately. A dedicated per-pair Tokio task, once woken, drains everything currently queued, encodes it as one (or, at the byte cap, several) framed batch(es) (§E), and issues exactly one `write_all()` per flushed batch. Because Stage 10's own drain loop (M0-B02's contract) calls `send()` repeatedly in a tight synchronous burst for every message a region's tick produced, by the time the writer task actually gets scheduled on the shared Tokio runtime (inherently later than the synchronous burst that woke it, since it requires an OS thread hand-off), it typically finds the burst's entire contents already queued — achieving PERF-D31's intended per-tick coalescing as an emergent property of the notify-then-drain pattern, without inventing a fake tick-boundary signal this crate has no way to observe correctly. The per-pair pending buffer is itself capped at `DEFAULT_PAIR_QUEUE_CAPACITY = 4096` messages (reusing ARCH-D27's own 4096-message per-region channel capacity for symmetry) — a full buffer makes `send()` return `Backpressure` immediately (§J), never blocking to wait for the writer task to drain.

### §J — Backpressure mapping to the `Transport` trait's error contract

`send()` returns `Err(TransportError::Backpressure(msg))`, never blocks, in exactly these cases, checked in this order: (1) `msg.to` is `Address::Entity`/`Address::Chunk` (§C, out of scope, unconditional); (2) `directory.resolve(region)` returns `None` (unknown/retired region); (3) the resolved `NodeId` is under an active `NOT_OWNER_GRACE_WINDOW` for that exact region (§H); (4) the resolved `NodeId` is remote and its per-pair pending buffer (§I) is already at `DEFAULT_PAIR_QUEUE_CAPACITY`; (5) the resolved `NodeId` is remote, no `quinn::Connection` currently exists to it, and the per-*node* pending-connection queue (bounding how many distinct never-yet-connected peers may have buffered traffic at once, `DEFAULT_MAX_PENDING_CONNECTIONS = 64`, a seed default) is already full — a connection attempt is still triggered asynchronously in every other remote case, never synchronously inside `send()` itself. Case (5)'s cap exists purely as a memory-safety bound on a cluster with many simultaneously-unreachable peers, not a correctness requirement — ordinary operation (a live cluster with working connections) never approaches it.

```rust
/// One control-plane frame exchanged over a connection's dedicated bidirectional
/// control stream (§D). Never mixed with `Message<RegionMessage>` batch bytes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ControlFrame {
    /// "I do not currently own `region`" — sent by a receiver that got a
    /// `RegionMessage` batch addressed to an unregistered region (§H).
    NotOwner { region: RegionId },
}
```

### §K — Connection-failure semantics and the metrics/observability seam (feeding a future takeover blueprint, M6-B02 integration)

CLUSTER-D15 is explicit that failure *detection* is raft's own leader-driven heartbeat, not this transport's job — this crate never decides "is that node dead." What this crate **does** own is graceful behavior when its own QUIC connection to a peer observably dies (peer process killed, network partition, TLS failure): every per-pair writer task and the connector task treat a `quinn::ConnectionError` identically — the connection is marked down, every pair routed through it starts returning `Backpressure` from `send()` (via case (4)/(5) above, since no pending buffer can drain), and a background reconnection loop with exponential backoff (`INITIAL_RECONNECT_BACKOFF = Duration::from_millis(100)`, doubling, capped at `MAX_RECONNECT_BACKOFF = Duration::from_secs(5)`, reset to initial on any successful connect — seed defaults, calibration-pending like every other numeric threshold in this corpus) keeps retrying in the background without blocking any caller. This self-heals automatically once either the original peer process resumes, **or** (the CLUSTER-D16 takeover case) the raft directory commits a new owner for the affected regions and `resolve()` starts returning a different `NodeId` for them — no special-cased "was this a takeover" branch exists in this crate; a stale connection simply stops being asked for by `resolve()`'s changing answers and a fresh one to the new owner gets dialed instead, exactly the same mechanism §H already establishes for planned migration.

This crate exposes an optional observability seam feeding both the connection-liveness signal above and PERF-D31/batching internals into whatever a future composition-root blueprint wires it to (M6-B02's `MetricsRegistry` is the natural target, but this crate never depends on `rc-scheduler` directly, per §A):

```rust
pub enum ConnectionLostReason { TimedOut, Reset, LocallyClosed, ApplicationClosed, Io(String) }

pub trait NetworkTransportMetricsSink: Send + Sync + 'static {
    fn on_message_sent(&self, from: RegionId, to: Address, byte_len: usize);
    fn on_message_received(&self, from: RegionId, to: Address, byte_len: usize);
    fn on_batch_flushed(&self, from: RegionId, to: Address, message_count: usize, byte_len: usize);
    fn on_backpressure(&self, from: RegionId, to: Address);
    fn on_connection_established(&self, node: NodeId);
    fn on_connection_lost(&self, node: NodeId, reason: ConnectionLostReason);
    fn on_stream_opened(&self, node: NodeId, from: RegionId, to: Address);
    fn on_directory_redirect(&self, region: RegionId, stale_node: NodeId, fresh_node: Option<NodeId>);
}
```

`NetworkTransport::with_metrics_sink` attaches one (optional — a transport built without it reports nothing, exactly `InProcessTransport`'s own opt-in-metrics pattern via `RcExecutorBuilder::with_metrics`, M6-B02). This directly maps onto CLUSTER-D28's own required-metrics list ("message-latency histograms per `(from_node, to_node)` pair tagged by `RegionMessage` variant... QUIC connection/stream-count and retransmission-rate gauges") without this crate adding an OTLP/`tracing-opentelemetry` dependency itself — ordinary `tracing::debug_span!`/`tracing::event!` calls at the same call sites (unconditional, per TEST-D30's own "an unsubscribed span/event is effectively free" framing, exactly M6-B02's own established stance) provide the distributed-trace correlation surface CLUSTER-D28 names, reusing ARCH-D25's envelope tuple `(from, to, tick_stamp, seq)` as the correlation key, per CLUSTER-D28's own binding rule — no new field added anywhere.

### §L — Feature gating (WS-D5(a)), already correct — nothing for this blueprint to change

`rc-transport-net`'s Cargo-feature gating is entirely a property of *its consumer's* manifest, not its own: WS-D5(a) fixes `rc-transport-net`/`rc-cluster`/`rc-proxy` as `optional = true` dependencies of `rusty-clanker-server`, unified under one Cargo feature `cluster` (in that binary's `default` feature list), with a from-source minimal build passing `--no-default-features --features monolithic` to strip them. **M0-B01 already scaffolded this exactly**: `rusty-clanker-server`'s `Cargo.toml` already carries `rc-transport-net = { path = "../transport-net", optional = true }` under `cluster = ["dep:rc-cluster", "dep:rc-transport-net", "dep:rc-proxy"]` plus a `monolithic = []` marker feature, and `xtask lint-deps`'s own `clean_graph_has_zero_violations` test already includes `rc-transport-net` in its expected edge table. This crate's own `Cargo.toml` carries no internal feature flag of its own (nothing about *its* content is conditionally compiled — the gate is entirely "is this crate in the dependency graph at all," decided one level up) — this blueprint verifies that already-correct wiring stays correct (Acceptance tests, `cluster_feature_absence_removes_crate_from_dependency_graph`) rather than re-implementing it. Runtime selection between `InProcessTransport`/`NetworkTransport` behind `dyn Transport` remains entirely config-presence-driven (CLUSTER-D26/D27) — a future composition-root blueprint's job, not this one's.

### §M — Explicit non-goals

This blueprint does **not** implement: `rc-cluster`'s raft-backed `NodeDirectory` (a future M7 blueprint — this crate only defines and consumes the trait, §H); the proxy role, CLUSTER-D20–D24's connection-termination/handoff/pre-warming machinery, or CLUSTER-D23's *proxy*↔node control channel (a different channel from this blueprint's own node↔node control stream, §D — the two must never be confused: CLUSTER-D23 governs player-connection routing state, this crate's control stream governs only `NotOwner` staleness signaling between two `NetworkTransport` peers); raft's own RPC/heartbeat transport (`openraft`'s network trait — CLUSTER-D15's failure detection rides on whatever `rc-cluster` wires it to, not necessarily this crate's QUIC connections, and this blueprint does not decide that); an `EntitySnapshotPool`-equivalent slot pool (ARCH-D28's `SegQueue` pooling is specifically an in-process, zero-copy optimization; a `RegionTransferRequest` crossing this transport is serialized to bytes regardless, so pooling the pre-serialization `Box<EntitySnapshot>` buys nothing here); TLS certificate issuance, distribution, or rotation (13's own stated Open Question, consumed as opaque bytes only, §F); installing a process-wide `rustls::CryptoProvider` (the composition root's job, §F). Building placeholder versions of any of these is out of scope, not a shortcut to take.

## Deliverables

### `crates/transport-net/Cargo.toml` (modify)

```toml
[package]
name = "rc-transport-net"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-messaging      = { path = "../messaging" }
quinn             = { workspace = true }
postcard          = { workspace = true }
rustls            = { workspace = true }
tokio             = { workspace = true }
serde             = { workspace = true }
thiserror         = { workspace = true }
parking_lot       = { workspace = true }
crossbeam-channel = { workspace = true }
tracing           = { workspace = true }

[dev-dependencies]
rc-core  = { path = "../core" }
proptest = { workspace = true }
rcgen    = { workspace = true }

[[bin]]
name = "echo_peer"
path = "tests/bin/echo_peer.rs"
test = false
doc = false
```

### Root `Cargo.toml` (modify — one new `[workspace.dependencies]` entry)

```toml
[workspace.dependencies]
# ... every existing entry unchanged ...
rcgen = "0.14.9"   # rc-transport-net's own dev-only test TLS material (self-signed cluster-internal
                    # CA + node certs for localhost QUIC tests) — this blueprint's own new pin, the
                    # same "cited, deliberate addition" M0-B02 already established for proptest.
                    # rustls org's own crate (github.com/rustls/rcgen); MIT OR Apache-2.0 (TEST-D35's
                    # allow-list). Moderate-confidence flag: re-verify current version at
                    # implementation time per this corpus's standing convention.
```

### `crates/transport-net/src/lib.rs`

```rust
//! `rc-transport-net` — `NetworkTransport` (ARCH-D26, CLUSTER-D11/D12): the cluster-mode
//! `Transport` implementation. QUIC (`quinn`) connections, one per (node, node) pair,
//! mutually TLS-authenticated; one QUIC stream per ordered `(from: RegionId, to: Address)`
//! pair (CLUSTER-D11); `postcard`-encoded, length-prefixed, per-pair-batched wire payloads
//! (CLUSTER-D12, PERF-D31). Depends only on `rc-messaging` plus this crate's own external
//! pins — never on `rc-scheduler` or `rc-cluster` (WS-D3 Rule 2; `rc-cluster`/`rc-proxy`
//! depend on this crate, never the reverse).

mod config;
mod connection;
mod control;
mod directory;
mod error;
mod inbound;
mod metrics;
mod tls;
mod transport;
mod wire;

pub use config::{
    NetworkTransportConfig, TlsMaterial, DEFAULT_BATCH_BYTE_CAP, DEFAULT_INBOUND_CHANNEL_CAPACITY,
    DEFAULT_MAX_PENDING_CONNECTIONS, DEFAULT_PAIR_QUEUE_CAPACITY, INITIAL_RECONNECT_BACKOFF,
    MAX_RECONNECT_BACKOFF, NOT_OWNER_GRACE_WINDOW,
};
pub use directory::{NodeDirectory, NodeId};
pub use error::NetworkTransportBuildError;
pub use metrics::{ConnectionLostReason, NetworkTransportMetricsSink};
pub use transport::NetworkTransport;
```

### `crates/transport-net/src/config.rs`

```rust
use std::net::SocketAddr;
use std::time::Duration;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// ARCH-D27's own per-region channel capacity, reused verbatim for this crate's per-pair
/// outbound pending-buffer cap and per-region inbound queue cap (§I/§J).
pub const DEFAULT_INBOUND_CHANNEL_CAPACITY: usize = 4096;
pub const DEFAULT_PAIR_QUEUE_CAPACITY: usize = 4096;
/// PERF-D31's own byte cap for one flushed batch.
pub const DEFAULT_BATCH_BYTE_CAP: usize = 16 * 1024;
/// §J case (5)'s memory-safety bound on distinct never-yet-connected peers.
pub const DEFAULT_MAX_PENDING_CONNECTIONS: usize = 64;
/// §H's residual-staleness grace window.
pub const NOT_OWNER_GRACE_WINDOW: Duration = Duration::from_millis(200);
/// §K's reconnection backoff bounds.
pub const INITIAL_RECONNECT_BACKOFF: Duration = Duration::from_millis(100);
pub const MAX_RECONNECT_BACKOFF: Duration = Duration::from_secs(5);

/// This node's own certificate chain + private key, plus the trusted cluster-internal CA
/// (CLUSTER-D11/D27, §F) — opaque bytes this crate turns into `rustls::ServerConfig`/
/// `ClientConfig`. Never contains a `CryptoProvider` install call (§F — the composition
/// root's own responsibility, exactly once, process-wide).
#[derive(Clone)]
pub struct TlsMaterial {
    pub node_cert_chain: Vec<CertificateDer<'static>>,
    pub node_key: PrivateKeyDer<'static>,
    pub trusted_ca: Vec<CertificateDer<'static>>,
}

#[derive(Clone)]
pub struct NetworkTransportConfig {
    pub local_node_id: crate::NodeId,
    /// CLUSTER-D27's `bind` field — the QUIC listen address, serving both inbound accept
    /// and outbound dial from the same UDP socket (§D).
    pub bind_addr: SocketAddr,
    pub tls: TlsMaterial,
    pub inbound_channel_capacity: usize,
    pub pair_queue_capacity: usize,
    pub batch_byte_cap: usize,
    pub max_pending_connections: usize,
}

impl NetworkTransportConfig {
    /// Every numeric field defaulted to this module's own constants; only `local_node_id`,
    /// `bind_addr`, and `tls` require a caller-supplied value.
    pub fn new(local_node_id: crate::NodeId, bind_addr: SocketAddr, tls: TlsMaterial) -> Self;
}
```

### `crates/transport-net/src/directory.rs`

```rust
use std::net::SocketAddr;
use std::sync::Arc;
use rc_messaging::RegionId;

/// A cluster node's stable identity (CLUSTER-D27: "stable identity, persisted across
/// restarts" — a string, e.g. `"node-a"`, never a `u64`). Cheap to clone (`Arc<str>`
/// inside) since every `send()` call clones one out of a directory lookup (§H).
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(Arc<str>);

impl NodeId {
    pub fn new(id: impl Into<Arc<str>>) -> Self;
    pub fn as_str(&self) -> &str;
}

/// The narrow `RegionId -> NodeId` seam this crate consumes (CLUSTER-D9/D5, §H) — never
/// implemented by this crate itself. A conforming implementation (a future `rc-cluster`
/// blueprint's own raft-backed directory) MUST answer every method from already-committed,
/// already-local in-memory state: never a network round trip, never blocking, never
/// holding a lock across an `.await` — `NetworkTransport` calls `resolve` on every `send()`
/// (§H) and trusts this contract instead of maintaining a second, duplicate cache.
pub trait NodeDirectory: Send + Sync + 'static {
    /// This process's own node identity.
    fn local_node_id(&self) -> &NodeId;
    /// Best-effort current owner of `region`. `None` means "not currently known" (never
    /// seen, or retired) — treated identically to an unregistered destination.
    fn resolve(&self, region: RegionId) -> Option<NodeId>;
    /// The dialable QUIC address for `node`. `None` if unknown.
    fn node_address(&self, node: &NodeId) -> Option<SocketAddr>;
}
```

### `crates/transport-net/src/metrics.rs`

```rust
use rc_messaging::{Address, RegionId};
use crate::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionLostReason { TimedOut, Reset, LocallyClosed, ApplicationClosed, Io(String) }

/// Optional observability seam (§K) — never a dependency on `rc-scheduler::metrics`
/// (WS-D3 Rule 2); a future composition-root blueprint bridges an implementation of this
/// trait to `rc-scheduler::metrics::MetricsRegistry` (M6-B02).
pub trait NetworkTransportMetricsSink: Send + Sync + 'static {
    fn on_message_sent(&self, from: RegionId, to: Address, byte_len: usize);
    fn on_message_received(&self, from: RegionId, to: Address, byte_len: usize);
    fn on_batch_flushed(&self, from: RegionId, to: Address, message_count: usize, byte_len: usize);
    fn on_backpressure(&self, from: RegionId, to: Address);
    fn on_connection_established(&self, node: NodeId);
    fn on_connection_lost(&self, node: NodeId, reason: ConnectionLostReason);
    fn on_stream_opened(&self, node: NodeId, from: RegionId, to: Address);
    fn on_directory_redirect(&self, region: RegionId, stale_node: NodeId, fresh_node: Option<NodeId>);
}
```

### `crates/transport-net/src/error.rs`

```rust
use std::net::SocketAddr;

/// `NetworkTransport::new`'s own construction failure — distinct from `TransportError`
/// (`rc-messaging`'s own, unmodified, `Transport::send`-only error type, §B). Construction
/// failures never cross the `Transport` trait's own method signatures.
#[derive(Debug, thiserror::Error)]
pub enum NetworkTransportBuildError {
    #[error("failed to bind QUIC endpoint at {addr}: {source}")]
    Bind { addr: SocketAddr, #[source] source: std::io::Error },
    #[error("invalid TLS material: {0}")]
    Tls(String),
}
```

### `crates/transport-net/src/wire.rs` (internal — `pub(crate)`, no public API)

Fixes the `[u32 LE length][postcard bytes]` framing (§E) shared by both `RegionMessage` batches and `ControlFrame`s: `pub(crate) async fn write_framed<T: serde::Serialize>(stream: &mut quinn::SendStream, value: &T) -> Result<(), WriteFramedError>`; `pub(crate) async fn read_framed<T: serde::de::DeserializeOwned>(stream: &mut quinn::RecvStream, max_len: usize) -> Result<T, ReadFramedError>` (rejects, without panicking, a length prefix exceeding `max_len` — a defensive bound against a corrupted or malicious peer, TEST-D26's own cluster-message-decode fuzz-target rationale extended to this crate's own framing layer, not only `postcard`'s decode itself).

### `crates/transport-net/src/control.rs` (internal — `pub(crate)`)

```rust
use rc_messaging::RegionId;

/// §J's control-plane frame. Never mixed with `Message<RegionMessage>` batch bytes —
/// carried on its own dedicated bidirectional stream per connection (§D).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ControlFrame {
    NotOwner { region: RegionId },
}
```

Plus the internal grace-window bookkeeping (`pub(crate) struct StalenessTracker`, a `parking_lot::Mutex<HashMap<RegionId, (NodeId, std::time::Instant)>>` — implementer's freedom on exact shape) implementing §H/§J's rule: record a `(region, stale_node)` pair with the current instant on `NotOwner` receipt; a query `is_within_grace(region, candidate_node, now) -> bool` returns `true` only if an entry exists for `region`, its recorded node equals `candidate_node`, and `now - recorded_instant < NOT_OWNER_GRACE_WINDOW`.

### `crates/transport-net/src/connection.rs` (internal — `pub(crate)`)

Owns, per remote `NodeId`: the `quinn::Connection` handle (once established), a `HashMap<(RegionId, rc_messaging::Address), OutboundPairState>` (each holding a pending-message `VecDeque`, a `tokio::sync::Notify`, and the currently-open `quinn::SendStream` once one exists), and the connector/reconnect-backoff state machine (§K). Exposes `pub(crate)` methods the `transport.rs` module's `send()` calls: `enqueue(pair, msg) -> Result<(), EnqueueError>` (bounded per §I/§J), and internal spawn points for one writer task per pair (drains on `Notify`, flushes via `wire::write_framed`, §I) and one connector task per node (dials via `Endpoint::connect_with`, retries with backoff, §K). Also owns the control-stream reader/writer half for `ControlFrame` exchange (§H/§J) and the receive-side `accept_uni` loop that hands newly-accepted streams to `inbound.rs`'s per-stream reader tasks.

### `crates/transport-net/src/inbound.rs` (internal — `pub(crate)`)

Owns `RwLock<HashMap<RegionId, crossbeam_channel::Sender<Message<RegionMessage>>>>` (the sender halves) plus the matching `Receiver` halves `try_recv` pops from — structurally identical to `InProcessTransport`'s own `channels: RwLock<HashMap<RegionId, RegionChannel>>` (M0-B03), reused here as this crate's own Tokio↔RC-WorkerPool boundary instance (§G), never the same instance ARCH-D22 already fixed for player packets. Per-stream reader tasks (spawned from `connection.rs`'s `accept_uni` loop) call `wire::read_framed::<Vec<Message<RegionMessage>>>`, then for each decoded message: extract `Address::Region(id)` from `.to` (any other variant arriving over the wire is a defensive-only case — log + metric + drop, never panic, §I's own reasoning applied to the receive side); if `id` has a live sender, forward it (`try_send`, itself bounded at `inbound_channel_capacity` — a full inbound queue drops with a metric, the same "never block a Tokio task" discipline ARCH-D22 already established, restated here); if `id` has no live sender (unregistered — §H's staleness case), reply `ControlFrame::NotOwner { region: id }` on the connection's control stream instead of silently discarding.

### `crates/transport-net/src/tls.rs` (internal — `pub(crate)`)

`pub(crate) fn build_server_config(tls: &TlsMaterial) -> Result<quinn::ServerConfig, NetworkTransportBuildError>` and `pub(crate) fn build_client_config(tls: &TlsMaterial) -> Result<quinn::ClientConfig, NetworkTransportBuildError>` — the mutual-TLS construction §F fixes (`WebPkiClientVerifier` trusting `tls.trusted_ca`, `with_single_cert`/`with_client_auth_cert` presenting `tls.node_cert_chain`/`tls.node_key`, wrapped via `quinn::crypto::rustls::{QuicServerConfig, QuicClientConfig}::try_from` per §D's moderate-confidence-flagged adapter path). Never calls `rustls::crypto::CryptoProvider::install_default()` (§F).

### `crates/transport-net/src/transport.rs`

```rust
use std::sync::Arc;
use std::time::Duration;
use rc_messaging::{Address, Message, RegionId, RegionMessage, Transport, TransportError};
use crate::{NetworkTransportBuildError, NetworkTransportConfig, NetworkTransportMetricsSink, NodeDirectory};

/// ARCH-D26/CLUSTER-D11's cluster-mode `Transport` implementation. See this blueprint's
/// Context (§A–§K) for the complete design; fields are private (implementer's freedom,
/// per `00-blueprint-spec.md`'s Deliverables note) but must realize: one `quinn::Endpoint`
/// (§D); a `connection.rs`-owned per-`NodeId` connection/pair-state table; an
/// `inbound.rs`-owned per-`RegionId` inbound channel table (§G); the `Arc<dyn
/// NodeDirectory>` this crate consumes (§H); an optional `Arc<dyn
/// NetworkTransportMetricsSink>` (§K); and the `tokio::runtime::Handle` every background
/// task is spawned onto (§G — never a runtime this crate owns or constructs).
pub struct NetworkTransport {
    // private
}

impl NetworkTransport {
    /// Binds the QUIC endpoint at `config.bind_addr`, spawns the background accept loop
    /// onto `runtime`. Establishes no outbound connections yet — those are dialed lazily
    /// on first `send()` targeting a region owned by that peer (§H/§K).
    pub fn new(
        config: NetworkTransportConfig,
        directory: Arc<dyn NodeDirectory>,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, NetworkTransportBuildError>;

    /// Registers `id` as locally owned: creates its inbound queue (§G) so
    /// QUIC-delivered or same-node-short-circuited (§H) messages become observable via
    /// `try_recv`. Never called by this crate itself — called by whichever future
    /// composition-root/`rc-cluster` call site executes an ARCH-D6 split/merge or a
    /// CLUSTER-D16 takeover, in lockstep with that same call site's raft-directory commit
    /// (§C, the exact `InProcessTransport`/`rc-scheduler` calling-convention pattern,
    /// restated for the cluster case). Idempotent-by-replacement, mirroring
    /// `InProcessTransport::register_region` exactly: re-registering an already-live `id`
    /// replaces its queue, dropping any not-yet-drained in-flight message.
    pub fn register_region(&self, id: RegionId);
    /// Drops `id`'s inbound queue and closes every outbound stream keyed with `id` as
    /// `from` (CLUSTER-D11: "closed... on region merge/split/migration"). Idempotent.
    pub fn deregister_region(&self, id: RegionId);
    pub fn is_registered(&self, id: RegionId) -> bool;

    /// Attaches an observability sink (§K). Optional — a transport built without this
    /// call reports nothing, exactly `RcExecutorBuilder::with_metrics`'s own opt-in
    /// pattern (M6-B02).
    pub fn with_metrics_sink(self, sink: Arc<dyn NetworkTransportMetricsSink>) -> Self;

    /// Gracefully closes every connection and stops accepting new ones, waiting up to
    /// `shutdown_timeout` for in-flight writer-task flushes to finish before forcing
    /// closure. Blocking.
    pub fn shutdown(&self, shutdown_timeout: Duration);
}

impl Transport for NetworkTransport {
    /// §H (directory resolution, same-node short-circuit, migration redirect), §I
    /// (per-pair enqueue), §J (the complete backpressure-case enumeration). Never blocks.
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError>;
    /// Non-blocking single-message pop from `into`'s inbound queue (§G/§H). `None` if
    /// `into` has no live queue or it is currently empty — indistinguishable via this
    /// call alone, exactly `InProcessTransport::try_recv`'s own documented behavior.
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>>;
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46):** the test changeset is every file listed below plus every `src/*.rs` file from Deliverables with executable bodies replaced by `todo!()` (field lists, derives, doc comments, and every public/`pub(crate)` signature stay exactly as specified), plus the two `Cargo.toml` edits. The implementation changeset (Implementation steps) fills in real bodies only — it must not modify any file under `crates/transport-net/tests/`, must not add/remove/rename a test case, and must not weaken an assertion.

### `crates/transport-net/tests/support/mod.rs` (test-only, not a deliverable)

A shared helper module: `fn generate_test_tls(node_ids: &[&str]) -> HashMap<String, TlsMaterial>` — uses `rcgen` to build one self-signed CA plus one leaf cert per requested node id, all signed by that CA, returning each node's `TlsMaterial` (§F) ready to hand to `NetworkTransportConfig::new`. `struct StaticNodeDirectory` — a test-only, mutable-via-`parking_lot::RwLock` `NodeDirectory` implementation (mirrors M0-B03's own test-only `MockTransport`/`FakeRegion` pattern: lives entirely in test code, never a crate deliverable) exposing `fn set(&self, region: RegionId, node: NodeId, addr: SocketAddr)` and `fn remove(&self, region: RegionId)` so tests can script directory changes, including mid-test staleness/migration scenarios. `struct RecordingMetricsSink` — a test-only `NetworkTransportMetricsSink` recording every callback into a `Mutex<Vec<...>>` for assertions.

### `crates/transport-net/tests/basic_send_recv.rs`

Two `NetworkTransport` instances (`node_a` at one ephemeral localhost port, `node_b` at another), one shared multi-thread Tokio runtime, one shared `StaticNodeDirectory` mapping a handful of synthetic `RegionId`s to each node.

1. `send_and_recv_single_message_across_real_quic` — register `RegionId(1)` on `node_a`, `RegionId(2)` on `node_b`; directory maps `RegionId(2) -> node_b`; `node_a.send(msg to Address::Region(RegionId(2)))`; poll `node_b.try_recv(RegionId(2))` (with a bounded retry loop and short sleep, since real QUIC delivery is not synchronous with `send()`'s own return) until `Some`, assert equal to the original message.
2. `try_recv_on_unregistered_region_returns_none` — fresh `node_b`, no registration; `try_recv(RegionId(42))` returns `None` immediately.
3. `send_to_unresolvable_region_returns_backpressure_with_original_message` — directory has no entry for `RegionId(99)`; `send(msg to Address::Region(RegionId(99)))` returns `Backpressure(returned)` with `returned == msg`, synchronously (no network round trip needed to know this).
4. `address_entity_and_address_chunk_return_backpressure_immediately` — mirrors M0-B03's identically-named/-shaped test exactly, restated for `NetworkTransport` (§C).
5. `same_node_regions_short_circuit_without_quic` — both `RegionId(10)` and `RegionId(20)` registered on `node_a`, directory maps both to `node_a`'s own `NodeId`; `node_a.send(...)` from 10 to 20; assert delivery via `try_recv`; assert (via a `pub(crate)`-visible test-only accessor, `#[cfg(test)]`-gated, or via the `RecordingMetricsSink` never observing `on_stream_opened`/`on_connection_established` for this pair) that no QUIC connection or stream was ever opened for this pair.
6. `deregister_region_drops_queue_and_closes_streams` — mirrors M0-B03's `deregister_region_drops_channel_and_future_sends_backpressure`, restated: after `deregister_region`, `is_registered` is `false`, `try_recv` returns `None`, and a `send` still addressed there (from the sender's own now-stale directory view) eventually surfaces `ControlFrame::NotOwner` back to the sender (asserted via `RecordingMetricsSink::on_directory_redirect`).

### `crates/transport-net/tests/fifo_property.rs`

The full FIFO/exactly-once property re-run (§B, mirroring M0-B02's `fifo_property.rs`/M0-B03's own concurrent variant exactly, this time over real sockets): `network_transport_fifo_and_exactly_once_over_quic` — a `proptest!` case generating `Vec<(u8, u32)>` (destination selector `0..4` mapping to four synthetic sender `RegionId`s registered on `node_a`, all targeting one destination `RegionId` registered on `node_b`; the second component is each element's own index, guaranteeing uniqueness); sends every element in original order from `node_a` (single-threaded send loop is sufficient here — `InProcessTransport`'s own multi-threaded variant already proves the trait's thread-safety at the sync boundary; this test's own job is proving the *network* leg preserves order, not re-proving sync-side thread-safety), waits (bounded retry loop) until `node_b` has drained a message count equal to the input length, then asserts: (a) the received marker set, as a set, equals the sent marker set (no loss, no duplication); (b) each of the four senders' own received-marker subsequence exactly matches its original relative send order (FIFO per `(from, to)` — cross-sender interleaving unchecked, matching ARCH-D29's own "no ordering guaranteed across different pairs").

### `crates/transport-net/tests/migration_and_staleness.rs`

1. `migration_redirects_stream_to_new_owner_within_same_tick` — `RegionId(5)` initially mapped to `node_b`; `node_a` sends marker `1` to it, confirms receipt on `node_b`; directory is updated (`StaticNodeDirectory::set`) to map `RegionId(5)` to a third instance `node_c` (also registering `RegionId(5)` there); **without any additional delay or synchronization beyond the directory mutation itself**, `node_a` sends marker `2`; assert marker `2` arrives at `node_c`, never at `node_b` — proving `resolve()`'s changed answer alone redirects the very next `send()` call (§H), with no explicit "flush"/"tick" signal needed.
2. `stale_directory_triggers_not_owner_and_self_heals` — `RegionId(6)` mapped to `node_b`, registered there; `node_b` deregisters `RegionId(6)` (simulating it having already migrated away from `node_b`'s own point of view) **without** `node_a`'s directory yet reflecting the change (a deliberately-stale view, mirroring the CLUSTER-D5-propagation-lag race §H names); `node_a` sends marker `1` — assert `send()` itself still returns `Ok` (the staleness is not yet known at send time) but `RecordingMetricsSink::on_directory_redirect` eventually fires with `stale_node == node_b`; a second send within `NOT_OWNER_GRACE_WINDOW` while the directory is still (deliberately, by the test) left stale returns `Backpressure` immediately (§H/§J's grace-window short-circuit) without a second wasted network round trip; after the grace window elapses (test uses a short, injectable clock — see Constraints on avoiding real sleeps where a fake clock is feasible, or a bounded real sleep no longer than `NOT_OWNER_GRACE_WINDOW * 3` if not) and the directory is updated to a real destination, a further send succeeds normally.

### `crates/transport-net/tests/batching.rs`

`batch_flush_splits_at_byte_cap_without_loss_or_reorder` — configure `batch_byte_cap` deliberately small (e.g. `512` bytes, well under `DEFAULT_BATCH_BYTE_CAP`, via `NetworkTransportConfig`'s override field) so a modest burst of synthetic `BorderUpdateEvent` messages (e.g. 200 messages, each large enough that the burst spans several batch flushes at this cap) sent in a tight loop from `node_a` to one `(from, to)` pair on `node_b` is guaranteed to split across multiple physical `write_all()` flushes; asserts every message arrives, in original order, exactly once — proving the batch-boundary framing (§E/§I) never loses or reorders data at a split point. `RecordingMetricsSink::on_batch_flushed` call count is asserted `> 1` (proving the test actually exercised a multi-batch scenario, not accidentally fitting everything in one flush).

### `crates/transport-net/tests/two_process_smoke.rs`

`two_process_smoke_real_sockets` — spawns `env!("CARGO_BIN_EXE_echo_peer")` (this crate's own `echo_peer` test-support binary, Deliverables) as a real child OS process via `std::process::Command`, passing its bind address, node id, and TLS material file paths (written to a temp directory by the test via `generate_test_tls`) as CLI arguments; the child process constructs its own `NetworkTransport`, registers one `RegionId`, and echoes every received `BorderUpdateEvent` back to whatever `RegionId` sent it (its own small, self-contained `main()`, not a deliverable of this crate's library surface); the test's own in-process `NetworkTransport` sends a handful of synthetic messages to the child and asserts the echoed replies arrive correctly; the child process is terminated (graceful `shutdown` signal via stdin close, falling back to `kill` after a bounded timeout) at test end. This is the literal "two-process... real sockets" integration smoke test — genuinely two OS processes, real UDP/QUIC loopback traffic, real TLS handshake.

### `crates/transport-net/tests/dependency_graph.rs`

`cluster_feature_absence_removes_crate_from_dependency_graph` — invokes `cargo metadata --no-default-features --features monolithic -p rusty-clanker-server --format-version 1` as a subprocess (the identical mechanism `xtask lint-deps`/WS-D11's own `--no-default-features --features monolithic` CI leg already uses, M0-B01), parses the resulting JSON, and asserts no package named `rc-transport-net` appears anywhere in `resolve.nodes` — proving WS-D5(a)'s already-correct gating (§L) stays correct. This test does not modify `rusty-clanker-server`'s `Cargo.toml`; it only observes the graph M0-B01 already produces.

## Implementation steps

1. **`Cargo.toml` + root `Cargo.toml`.** Add every dependency exactly as Deliverables specify; add the `rcgen` workspace pin. Observable: `cargo metadata` resolves workspace-wide.
2. **`config.rs`, `directory.rs`, `metrics.rs`, `error.rs`.** Plain data types and traits — real bodies are straightforward field assignment / trivial derive-backed behavior. Observable: these four files compile standalone.
3. **`wire.rs`.** Implement `write_framed`/`read_framed` per §E's exact `[u32 LE length][postcard bytes]` shape, with `read_framed`'s `max_len` rejection path returning an error (never panicking) on an oversized claimed length. Observable: a small round-trip unit test inside this file's own `#[cfg(test)]` block (implementer's freedom — not part of the Acceptance tests changeset) passes.
4. **`tls.rs`.** Implement `build_server_config`/`build_client_config` per §F, using the `quinn::crypto::rustls::{QuicServerConfig, QuicClientConfig}` adapter path flagged in §D — **verify the exact adapter module path and quinn's default-feature crypto-backend/Tokio-runtime coverage against `cargo doc -p quinn` before writing this file's body**; if quinn's defaults are insufficient, add the verified feature names to this crate's own `quinn` dependency line and note the finding per Constraints.
5. **`control.rs`.** `ControlFrame` (already fixed) plus `StalenessTracker`'s `record`/`is_within_grace` per §H/§J's exact rule.
6. **`inbound.rs`.** The per-region `crossbeam-channel` table (mirroring `InProcessTransport`'s own `RwLock<HashMap<RegionId, RegionChannel>>` construction pattern, M0-B03) plus the stream-reader task body: `read_framed::<Vec<Message<RegionMessage>>>`, per-message `Address::Region` extraction with defensive drop-and-metric on any other variant, `try_send` into the matching region's channel (bounded, drop-with-metric on full), `ControlFrame::NotOwner` reply on an unregistered destination.
7. **`connection.rs`.** The per-`NodeId` connection table, per-pair `OutboundPairState`/writer-task/`Notify` mechanism (§I), the connector/backoff state machine (§K), the `accept_uni`/control-stream wiring feeding `inbound.rs`. This is the largest single file — implement incrementally, verified against `basic_send_recv.rs`'s first few cases before attempting `fifo_property.rs`/`migration_and_staleness.rs`.
8. **`transport.rs`.** `NetworkTransport::new`/`register_region`/`deregister_region`/`with_metrics_sink`/`shutdown`, and the `Transport` impl's `send`/`try_recv` wiring §H's same-node short-circuit, §J's complete backpressure-case ordering, and delegation into `connection.rs`/`inbound.rs`. Observable: `cargo build -p rc-transport-net` succeeds with zero `todo!()` remaining.
9. **`tests/support/mod.rs` real bodies**, then **`tests/bin/echo_peer.rs`** (a small, self-contained `main()` using this crate's own now-real public API — not a library deliverable, but real code, part of the implementation changeset since it lives under `tests/` only by convention for `[[bin]]` placement, not because it is itself a test file TEST-D46 protects — restated explicitly in Constraints).
10. **Run the full acceptance suite.** `cargo nextest run -p rc-transport-net` — every test named in Acceptance tests passes, across all six test files.
11. **Doctests.** `cargo test --doc -p rc-transport-net` passes.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `cargo run -p xtask -- lint`, `cargo run -p xtask -- lint-deps`, `cargo run -p xtask -- test` — all four exit 0.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file under `crates/transport-net/tests/` (including `tests/support/mod.rs` and `tests/bin/echo_peer.rs`) is committed first, alongside `todo!()`-stubbed `src/*.rs` files carrying every field/derive/signature already fixed. The implementation changeset (steps 1–13) fills in real bodies only — it must not edit a test file, must not add/remove/rename a test case, must not weaken an assertion (in particular, `migration_and_staleness.rs`'s exact grace-window/ordering assertions and `fifo_property.rs`'s exact loss/duplication/FIFO checks must survive unchanged). `tests/bin/echo_peer.rs` is real implementation code by content (it is not itself a test, it is a test-support binary target) but is committed in the test-authoring changeset per its `tests/` placement convention, since it is consumed only by `two_process_smoke.rs` and never by production code — restated explicitly here to avoid ambiguity against TEST-D46's path-guard, which keys on directory, not on "is this file a `#[test]` function."

(b) **No new external dependencies beyond the pinned set, with exactly one named exception.** Every external crate this blueprint's deliverables use (`quinn`, `postcard`, `rustls`, `tokio`, `serde`, `thiserror`, `parking_lot`, `crossbeam-channel`, `tracing`, `proptest`) is already in `[workspace.dependencies]`, except `rcgen`, which this blueprint itself adds at the version verified in Context §F/Deliverables — a cited, deliberate addition, not an invented one. Do not add `bevy_ecs`, `openraft`, `redb`, `object_store`, `anyhow`, or any crate this blueprint does not name to `rc-transport-net`'s `Cargo.toml` under any circumstance.

(c) **Dependency-direction discipline (WS-D3 Rule 2, §A).** `rc-transport-net` must never gain a dependency on `rc-scheduler`, `rc-mechanics`, `rc-cluster`, or `rc-proxy` — the `NodeDirectory` and `NetworkTransportMetricsSink` traits (§H/§K) are this crate's *entire* mechanism for cluster-directory and metrics integration; do not "simplify" by importing a concrete type from either forbidden crate.

(d) **No Mojang or third-party reimplementation code.** Every type and algorithm here is derived solely from `13-cluster-architecture.md`'s CLUSTER-D9–D12/D28, `14-performance-engineering.md`'s PERF-D30/D31, and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30).

(e) **No `unsafe` code.** Every type and function in this blueprint's deliverables is implementable in 100% safe Rust — `quinn`, `rustls`, `tokio`, `parking_lot`, `crossbeam-channel` are all safe-to-use crate APIs; no raw pointers, no `unsafe impl`, no FFI.

(f) **The composition-root contract stated in §F/§G is binding on whoever calls this crate, not enforced by this crate.** `NetworkTransport::new` neither installs a process-wide `rustls::CryptoProvider` nor constructs its own Tokio runtime — a future composition-root blueprint must do both exactly once before calling `NetworkTransport::new`. This blueprint's own tests satisfy this contract themselves (each test's `#[tokio::test]`/manually-built runtime installs a `CryptoProvider` once via a `std::sync::Once`-guarded test helper) but this crate's production code never does so implicitly.

(g) **Scope boundary — do not implement beyond this blueprint's one crate (§M).** This blueprint does not implement `rc-cluster`'s raft-backed `NodeDirectory`, the proxy role or CLUSTER-D20–D24's handoff/pre-warming machinery, CLUSTER-D23's proxy↔node control channel, raft's own RPC transport, or any composition-root wiring inside `rusty-clanker-server` (already correctly scaffolded by M0-B01, §L — this blueprint verifies it, never edits it). Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-transport-net --all-features
cargo nextest run -p rc-transport-net
cargo test --doc -p rc-transport-net
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-transport-net` runs 6 (`basic_send_recv.rs`) + 1 (`fifo_property.rs`, one property-test case regardless of internal proptest-generated input count, consistent with M0-B02/M0-B03's own identical framing) + 2 (`migration_and_staleness.rs`) + 1 (`batching.rs`) + 1 (`two_process_smoke.rs`) + 1 (`dependency_graph.rs`) = 12 test cases named in Acceptance tests — all pass. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
