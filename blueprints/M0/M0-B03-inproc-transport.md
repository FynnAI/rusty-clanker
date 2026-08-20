# M0-B03 — In-Process Transport & Cross-Region Timing

| Field | Content |
|---|---|
| ID | M0-B03 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | M0-B01 (workspace scaffold: `crates/transport-inproc/` exists as an empty-shell crate depending only on `rc-messaging`, already wired into `xtask lint-deps`'s Rule 2 check); M0-B02 (`rc-messaging`'s complete, real `Transport` trait, `TransportError`, `Message<T>`, `Address`, `RegionId`, `RegionMessage`/`BorderUpdateEvent`/`BorderUpdateKind`/`EntitySnapshot`, `RegionMessageBus`, `RegionMessageState` — this blueprint builds on every one of these signatures exactly as M0-B02 fixed them, never modifying `rc-messaging`) |
| Implements | ARCH-D27 (`InProcessTransport` itself); ARCH-D23 (`parking_lot`-guarded region table); ARCH-D28 (`EntitySnapshotPool`, the `SegQueue`-backed slot pool); ARCH-D29 (delivery/ordering guarantees — FIFO, exactly-once, no-cross-pair-order, never-blocks — verified by this blueprint's own tests, including under real concurrent send); ARCH-D11's timing consequence and M0-B02's Stage-1/Stage-10 contract (both restated in full below and, for the first time, exercised end-to-end against a real `Transport` implementation instead of `rc-messaging`'s own in-isolation mock); TEST-D27 (proptest, already pinned by M0-B02) |
| Crates touched | `rc-transport-inproc` (`crates/transport-inproc/`) only — no other crate is modified |
| Estimated scope | L |

## Goal & Done definition

Implement `InProcessTransport` — the monolithic-mode `Transport` trait implementation ARCH-D27 specifies — plus ARCH-D28's `EntitySnapshotPool`, both inside `rc-transport-inproc`. Prove, with a deterministic, single-threaded, stepped test harness that stands in for the tick driver `rc-scheduler` has not yet implemented, that a `BorderUpdateEvent`-shaped message sent from one region during its own tick becomes observable at a second, artificially-registered region's `RegionMessageState` inbox at exactly that destination's next Stage-1 boundary — never earlier, never later — satisfying M0's acceptance criterion 2 in full. Also prove, under real concurrent multi-threaded send, that `InProcessTransport` upholds ARCH-D29's FIFO-per-pair and exactly-once guarantees.

Done when:

- [ ] `cargo build -p rc-transport-inproc --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-transport-inproc`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-transport-inproc`'s new normal dependencies (`crossbeam-channel`, `crossbeam-queue`, `parking_lot`) touch no rule: Rule 2 (SIM `↔` NETRENDER isolation) is the only rule that names `rc-transport-inproc` at all, and none of these three crates, nor `rc-messaging` (already Rule-3-clean per M0-B02), ever reach `rc-scheduler`/`rc-mechanics`.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-transport-inproc` exits 0.
- [ ] The cross-region timing test (`cross_region_timing.rs`) is fully deterministic: it uses no real thread, no sleep, no wall-clock read anywhere — every "tick" boundary is an explicit method call the test itself sequences — so a from-clean-checkout CI run reproduces the identical pass/fail outcome every time (TEST-D50's determinism expectation).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test` — M0-B01's four gates — now also carrying `rc-transport-inproc`'s full `nextest` suite and its TEST-D27 proptest case, per TEST-D37's Tier-1 membership rules) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### What already exists, and what this blueprint adds

M0-B01 scaffolded `crates/transport-inproc/Cargo.toml` as an empty shell with exactly one dependency, `rc-messaging`, and an empty `src/lib.rs` doc-comment placeholder. M0-B02 delivered `rc-messaging`'s complete, real API: the `Transport` trait, `TransportError::Backpressure(Message<RegionMessage>)`, the `Message<T>` envelope, `Address`, `RegionId`, `RegionMessage`/`BorderUpdateEvent`/`BorderUpdateKind`/`EntitySnapshot`, and the ECS-facing `RegionMessageBus`/`RegionMessageState` pair — all specified in M0-B02's own Deliverables and restated below wherever this blueprint depends on their exact shape. Nothing in `rc-messaging` is modified by this blueprint. This blueprint's entire job is to fill in `rc-transport-inproc`'s two real types (`InProcessTransport`, `EntitySnapshotPool`) and to write the acceptance test that proves M0's cross-region timing criterion.

### `InProcessTransport`'s concurrency structure (ARCH-D27, ARCH-D23) — restated concretely

ARCH-D27's exact text: "One bounded `crossbeam-channel` 0.5.16 MPSC per live `RegionId` (capacity 4096 messages, configurable), created/destroyed exactly at ARCH-D6 split/merge boundaries under the `parking_lot`-guarded region ownership table (ARCH-D23). `Message<RegionMessage>` values are moved — never cloned, never serialized — from sender to receiver."

Concretely, this blueprint's `InProcessTransport` holds exactly one piece of state:

```
channels: parking_lot::RwLock<HashMap<RegionId, RegionChannel>>
```

where `RegionChannel` is a private pair `{ sender: crossbeam_channel::Sender<Message<RegionMessage>>, receiver: crossbeam_channel::Receiver<Message<RegionMessage>> }` created together by one `crossbeam_channel::bounded(capacity)` call. Both halves live and die together in the same `HashMap` entry — there is no code path in this blueprint that drops one half independently of the other, which is why `crossbeam_channel::TryRecvError::Disconnected` can never actually occur in this implementation (see the `try_recv` note below). `parking_lot::RwLock` 0.12.5 is the exact primitive ARCH-D23 names for "cold-path bookkeeping (region ownership table...)" — read locks (taken by every `send`/`try_recv` call, the hot path) are cheap and freely concurrent; write locks (taken only by `register_region`/`deregister_region`, the cold path) are rare, matching ARCH-D23's framing exactly.

`register_region`/`deregister_region` are this blueprint's realization of "created/destroyed exactly at ARCH-D6 split/merge boundaries" — but ARCH-D6's actual split/merge algorithm is `rc-scheduler`'s responsibility (a separate, not-yet-written M0 blueprint; `rc-scheduler` cannot even be a dependency of this crate — see the next Context subsection). This blueprint therefore exposes the registration API and documents its intended call site; it does not, and cannot, call it from anywhere except its own tests. A later `rc-scheduler` blueprint calls `register_region` the moment it mints a new `RegionId` (at boot, or at an ARCH-D6 split) and `deregister_region` the moment a `RegionId` is retired (at an ARCH-D6 merge) — always paired with that same blueprint's own ARCH-D24 directory updates, at the same ARCH-D9 sync point, by calling convention rather than by sharing one literal data structure between the two crates.

### Why `Address::Entity`/`Address::Chunk` resolution is out of this blueprint's scope

ARCH-D25 states resolution of `Address::Entity`/`Address::Chunk` to a concrete destination `RegionId` happens "via the ARCH-D24 directories," and M0-B02's own Context section says this resolution "happens inside whichever concrete `Transport` implementation calls `Transport::send`" — naming this crate as the place it eventually belongs. But `xtask lint-deps`'s Rule 2 (M0-B01, sourced from WS-D3) places `rc-transport-inproc` in the `NETRENDER` set and `rc-scheduler` in the `SIM` set, and forbids either from transitively reaching the other in **either** direction — so `rc-transport-inproc` can never depend on `rc-scheduler`, the crate that will actually own the ARCH-D24 `ChunkKey -> RegionId`/`RcEntityId -> RegionId` directories (M0-B02's own Context section: "belong to whichever later blueprint drives region ownership (`rc-scheduler`)"). Building real directory-based resolution today would require either violating Rule 2 or inventing an injected-resolver abstraction nothing in M0's acceptance criteria exercises — M0-B01's own stated principle applies directly here: "adding an external dependency to an as-yet-empty crate's manifest today would not be checked by anything `xtask lint-deps` validates... so pre-guessing the rest only adds unverifiable content... for zero acceptance-criterion benefit." M0's acceptance criterion 2 only exercises `Address::Region`-addressed messages, and both of ARCH-D25's native payload variants have a plausible, simpler resolution path that never needs this machinery in practice: `BorderUpdateEvent` (ARCH-D11) is always sent to an already-known bordering neighbor's `RegionId` (border-halo neighbor relationships are established when a region's owned cells are fixed, not looked up per-message), and `RegionTransferRequest` (ARCH-D10) is always sent to whatever region a Stage-6 system already resolved as the entity's new owner. This blueprint's concrete, deliberate resolution: `InProcessTransport::send` fully handles `Address::Region(id)`; for `Address::Entity`/`Address::Chunk` it returns `Err(TransportError::Backpressure(msg))` immediately — never panics, reuses the one error variant M0-B02's fixed `Transport`/`TransportError` API already provides, and is semantically honest ("cannot deliver this right now" is exactly what a caller holding an unresolvable address needs to hear, and ARCH-D29's own retry-next-tick contract is exactly the right response). A later blueprint that actually needs `Address::Entity`/`Address::Chunk` resolution inside `InProcessTransport` must design its own mechanism (most plausibly an injected resolver supplied at construction time by whatever crate is allowed to depend on both `rc-transport-inproc` and `rc-scheduler`) — not designed here, since nothing in M0 exercises it.

### `EntitySnapshotPool` (ARCH-D28) — restated concretely, including this blueprint's own resolution of its open pre-sizing/exhaustion question

ARCH-D28: `BorderUpdateEvent` stays inline (no heap allocation, ≤128 bytes — M0-B02's own `region_message_size_bound` test already guards this at the `rc-messaging` level, unaffected by this blueprint). `RegionTransferRequest`'s `Box<EntitySnapshot>` payload is drawn from "a global lock-free slot pool (`crossbeam-queue` 0.3.13 `SegQueue<Box<EntitySnapshot>>`...): the sending worker pops a free slot to build the snapshot, the payload moves through the channel by value, and the *destination* region's Stage-1 apply pass returns the slot to the pool once consumed." `01-server-architecture.md`'s own Open Questions flag this pool's "pre-sizing... and its exhaustion behavior (block the sending worker, drop the transfer and retry next tick, or grow the pool dynamically)" as unresolved, explicitly inviting a blueprint-phase decision. This blueprint's concrete resolution:

- **Exhaustion never blocks and never drops.** `acquire(value: EntitySnapshot) -> Box<EntitySnapshot>` pops a previously-released box and overwrites its contents with `value` if one is available; otherwise it allocates a fresh `Box::new(value)`. Both paths return an equally valid, fully-populated box — a popped-and-reused allocation and a freshly-heap-allocated one are indistinguishable at the type level (M0-B02's own framing, applied here). This upholds ARCH-D29's "never blocks the sender" principle for the pool specifically, not only for the channel.
- **Pre-sizing is caller-configurable, not hard-coded.** The pool starts **empty** (no dummy pre-filled `EntitySnapshot` values — there is no sensible placeholder content before a real transfer supplies one) and grows its retained-slot count lazily as `release` calls arrive, capped at a `capacity` fixed at construction. `InProcessTransportConfig::entity_snapshot_pool_capacity` (default `256`) is this blueprint's own seed default — explicitly *not* a calibrated production value (ARCH-D28's Open Question is still open; this default only unblocks M0, which has no real entity traffic yet — `EntitySnapshot`'s content is itself still M0-B02's placeholder `component_data: Vec<u8>` until `05-game-mechanics.md` lands, per M0-B02's own Context note).
- **`release(slot)` beyond `capacity` drops the box** instead of growing the pool unbounded — a released slot when the pool already holds `capacity` free slots is simply deallocated normally.

`acquire`/`release` are called by whichever future blueprint implements Stage 6 (sender side, ARCH-D10) and Stage 1 (receiver side, ARCH-D10's apply pass) — not by `InProcessTransport::send`/`try_recv` themselves, which only ever move an already-fully-constructed `Message<RegionMessage>` (its `Box<EntitySnapshot>` payload, if present, already built) — this crate's `send`/`try_recv` never touch the pool directly. `InProcessTransport` merely **owns** one pool instance and exposes it via `entity_snapshot_pool()` so a later caller reaches it through the same handle it already holds for `dyn Transport`.

`SegQueue` itself needs no external lock (it is `crossbeam-queue`'s own lock-free MPMC structure — the entire reason ARCH-D28 rejected a per-worker-thread bump arena in favor of it, per `01`'s own rationale: "the worker that allocates a `RegionTransferRequest` snapshot and the worker whose region consumes it at Stage 1 are not guaranteed to be the same OS thread"). The `capacity` cap is enforced by a separate `std::sync::atomic::AtomicUsize` counter, reserved via a compare-exchange loop *before* pushing, so the queue's real length never exceeds `capacity` even under concurrent `release` calls from multiple threads. Under concurrent `acquire`/`release`, this counter can transiently *under*-report the queue's true length for a few instructions (a benign race: `acquire`'s `pop()` and its `fetch_sub` are not one atomic operation) — the only possible consequence is a `release` occasionally, conservatively dropping a box slightly before strictly necessary; the cap is never exceeded and there is no unsoundness. This is a documented, deliberate best-effort accounting choice, acceptable because M0 has no real load to make it matter and a later blueprint calibrating real usage can revisit it.

### The Stage-1/Stage-10 contract this blueprint's test harness drives (restated from M0-B02, ARCH-D30/D11)

M0-B02 fixed this contract but could not test it (no real `Transport` existed yet). Restated in full, since this blueprint's acceptance test is the first thing that actually exercises it end-to-end:

> **Stage-1 contract.** Before any Stage-1..N system for a region runs, the driver calls `Transport::try_recv(region_id)` repeatedly until it returns `None`, collecting every returned message's `.payload` in return order, then calls `RegionMessageState::set_inbox` exactly once with the full collected batch.
>
> **Stage-10 contract.** After every system in the tick has run and every `RegionMessageBus` it produced has been `merge`d into the region's `RegionMessageState` (in merge order), the driver calls `RegionMessageState::drain_outbox(this_region_id, this_tick_counter)` exactly once, then calls `Transport::send` once per returned `Message`, in the order returned.
>
> **Timing consequence (ARCH-D11).** A message flushed at the sender's Stage 10 of tick N becomes visible via `try_recv` no earlier than the destination's very next Stage 1 — never within the sender's own tick N, and never delayed past the destination's next Stage-1 drain.

`rc-scheduler`'s real tick driver (ARCH-D1–D9/D12/D18–D23, RC-Executor, the 11-stage pipeline) does not exist yet — it is a separate, not-yet-written M0 blueprint, exactly as M0-B01's own Constraint (d) and M0-B02's own Constraint (e) already establish. `rc-transport-inproc` cannot depend on it even once it exists (Rule 2, above). This blueprint's acceptance test therefore drives the Stage-1/Stage-10 contract itself, directly, via a small test-only struct (`FakeRegion`, specified in full in Acceptance tests below) that calls nothing but `rc-messaging`'s already-real, already-fixed public API (`RegionMessageBus`, `RegionMessageState`, `Transport`) plus this blueprint's own `InProcessTransport` — exactly mirroring how M0-B02's own `fifo_property.rs` test defined a test-only `MockTransport` entirely inside its test file rather than adding production scaffolding. `FakeRegion` is **not** a deliverable of this blueprint (it is not part of `rc-transport-inproc`'s public API, does not ship, and is not `rc-scheduler`) — it exists solely inside this blueprint's test file as a deterministic, single-threaded, explicitly-stepped stand-in for the tick driver, giving the test full control over exactly when each Stage-1 and Stage-10 boundary occurs, with no real thread, no sleep, and no wall clock anywhere. This is the "fake/stepped clock, single-threaded stepping mode" the test harness design calls for: "ticks" advance only when the test itself calls `.stage1(...)`/`.emit(...)`/`.stage10(...)`, so ordering is 100% deterministic and reproducible from a clean checkout every run (TEST-D50).

The M0 milestone document's own acceptance criterion 2 asks for "two regions exchanging a synthetic `BorderUpdateEvent`-shaped message across `InProcessTransport`" between "two artificially-split regions" — this blueprint's tests register two manually-chosen `RegionId` values standing in for what a real ARCH-D6 split would eventually produce (ARCH-D6's actual split algorithm does not exist yet), matching that phrasing exactly.

A note on what the "not same tick" half of the test actually proves: `InProcessTransport::send` only ever enqueues into a channel — it never invokes any code on the destination region's side synchronously. This means "a sent message cannot retroactively appear in a Stage-1 draw the destination already completed before the send happened" is true *by construction* of using a channel rather than a direct call, not something a runtime assertion discovers for the first time. The test still asserts it explicitly (capturing the destination's pre-send Stage-1 draw as an owned, already-returned value, then re-affirming it is still empty after the sender's flush) because that assertion is a cheap, permanent regression guard against any future refactor that could reintroduce synchronous delivery — not because the property is in doubt today.

### Dependency additions (WS-D7-compliant — every version already pinned)

`crates/transport-inproc/Cargo.toml` gains three normal dependencies, all already present, at these exact versions, in the workspace root `Cargo.toml`'s `[workspace.dependencies]` table (M0-B01): `crossbeam-channel = "0.5.16"`, `crossbeam-queue = "0.3.13"`, `parking_lot = "0.12.5"`. It gains two dev-dependencies: `rc-core` (path dependency — this crate's own tests construct `ChunkKey`/`BlockPos`/`DimensionId`/`RcEntityId` values directly to build synthetic `BorderUpdateEvent`/`EntitySnapshot` payloads) and `proptest = "1.11.0"` (already added to `[workspace.dependencies]` by M0-B02 — this blueprint does not add a second entry). No new line is added to the workspace root `Cargo.toml` by this blueprint at all — every external crate this blueprint's deliverables use was already pinned before this blueprint started.

### Determinism note on `HashMap<RegionId, RegionChannel>`

As with M0-B02's own `RegionMessageState.seq_counters`, this map's iteration order is never observed anywhere in this blueprint — `register_region`/`deregister_region`/`send`/`try_recv` all perform point lookups/inserts/removals keyed by a specific `RegionId` the caller already has, never an iteration. `HashMap`'s point-access determinism for a fixed key sequence is exact regardless of internal bucket layout, so this is safe under the same reasoning M0-B02 already established.

### Known limitations, not solved by this blueprint

`Address::Entity`/`Address::Chunk` resolution (above) is deferred to whichever later blueprint is allowed to bridge `rc-transport-inproc` and `rc-scheduler`. `register_region`/`deregister_region` are never called anywhere except this blueprint's own tests — real ARCH-D6 split/merge wiring is `rc-scheduler`'s job. `EntitySnapshotPool`'s `256`-slot default is an unvalidated seed value pending real load calibration (ARCH-D28's own still-open Open Question). This blueprint's tests are plain Tier-1 unit/integration tests plus one TEST-D27 proptest case — they are **not** yet wired into TEST-D17–19's worker-pool-size/region-topology/deployment-mode determinism corpora, since those require a real TEST-D11 scenario corpus and a real tick driver, neither of which exists yet.

## Deliverables

### `crates/transport-inproc/Cargo.toml` (modify)

```toml
[package]
name = "rc-transport-inproc"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-messaging = { path = "../messaging" }
crossbeam-channel = { workspace = true }
crossbeam-queue = { workspace = true }
parking_lot = { workspace = true }

[dev-dependencies]
rc-core = { path = "../core" }
proptest = { workspace = true }
```

### `crates/transport-inproc/src/lib.rs`

```rust
//! `rc-transport-inproc` — `InProcessTransport` (ARCH-D27): the monolithic-mode
//! `Transport` implementation, one bounded `crossbeam-channel` MPSC per live `RegionId`
//! under a `parking_lot`-guarded region table (ARCH-D23), plus `EntitySnapshotPool`
//! (ARCH-D28), the global `SegQueue`-backed slot pool for large `RegionTransferRequest`
//! payloads.

mod entity_snapshot_pool;
mod transport;

pub use entity_snapshot_pool::EntitySnapshotPool;
pub use transport::{InProcessTransport, InProcessTransportConfig};
```

### `crates/transport-inproc/src/entity_snapshot_pool.rs`

```rust
use std::sync::atomic::AtomicUsize;

use crossbeam_queue::SegQueue;
use rc_messaging::EntitySnapshot;

/// ARCH-D28's global, lock-free slot pool for `RegionTransferRequest`'s
/// `Box<EntitySnapshot>` payload. Never blocks (`acquire` always returns a usable box;
/// `release` never blocks the caller). See this blueprint's Context section for the
/// exhaustion/pre-sizing policy this type implements (this blueprint's own resolution of
/// ARCH-D28's Open Question).
pub struct EntitySnapshotPool {
    free: SegQueue<Box<EntitySnapshot>>,
    free_count: AtomicUsize,
    capacity: usize,
}

impl EntitySnapshotPool {
    /// An empty pool retaining at most `capacity` released slots for reuse.
    pub fn new(capacity: usize) -> Self;

    /// Reuse a previously `release`d allocation (overwriting its contents with `value`)
    /// if one is available; otherwise allocate fresh via `Box::new(value)`. Never blocks.
    pub fn acquire(&self, value: EntitySnapshot) -> Box<EntitySnapshot>;

    /// Return a consumed slot for reuse. Dropped instead of retained if the pool already
    /// holds `capacity` free slots.
    pub fn release(&self, slot: Box<EntitySnapshot>);

    /// Current (best-effort — see Context's concurrency note) count of free, reusable
    /// slots. Never exceeds `capacity`. Test/diagnostic use.
    pub fn free_count(&self) -> usize;
}
```

### `crates/transport-inproc/src/transport.rs`

```rust
use std::collections::HashMap;

use parking_lot::RwLock;
use rc_messaging::{Address, Message, RegionId, RegionMessage, Transport, TransportError};

use crate::EntitySnapshotPool;

/// Tunable knobs for `InProcessTransport`: ARCH-D27's per-region channel capacity
/// ("capacity 4096 messages, configurable") and ARCH-D28's `EntitySnapshotPool`
/// pre-sizing (this blueprint's own seed default — see Context).
#[derive(Copy, Clone, Debug)]
pub struct InProcessTransportConfig {
    pub channel_capacity: usize,
    pub entity_snapshot_pool_capacity: usize,
}

impl Default for InProcessTransportConfig {
    /// `channel_capacity: 4096` (ARCH-D27's literal number), `entity_snapshot_pool_capacity: 256`.
    fn default() -> Self;
}

/// ARCH-D27's monolithic-mode `Transport` implementation. One bounded
/// `crossbeam-channel` MPSC per live `RegionId`, under a `parking_lot::RwLock`-guarded
/// region table (ARCH-D23), plus one shared `EntitySnapshotPool` (ARCH-D28).
/// `Message<RegionMessage>` values move through the channel by value — never cloned,
/// never serialized.
pub struct InProcessTransport {
    // fields are private; see Context for the exact internal shape
}

impl InProcessTransport {
    /// An empty transport (no regions registered) using `config`.
    pub fn new(config: InProcessTransportConfig) -> Self;

    /// Create `id`'s inbound channel. Calling this again for an already-registered `id`
    /// silently replaces its channel — drops any still-in-flight messages and the old
    /// receiver. A correct caller never does this (`RegionId`'s own identity contract,
    /// `rc-messaging`'s Context, guarantees `RegionId` values are never reused). Intended
    /// call site: an ARCH-D6 split/merge boundary, owned by a later `rc-scheduler`
    /// blueprint — see Constraints.
    pub fn register_region(&self, id: RegionId);

    /// Destroy `id`'s inbound channel. Any message already in flight toward `id` (sent
    /// but not yet drained) is dropped. Idempotent: deregistering an unregistered `id`
    /// is a no-op.
    pub fn deregister_region(&self, id: RegionId);

    /// Whether `id` currently has a live channel.
    pub fn is_registered(&self, id: RegionId) -> bool;

    /// The shared, global `EntitySnapshotPool` (ARCH-D28).
    pub fn entity_snapshot_pool(&self) -> &EntitySnapshotPool;
}

impl Transport for InProcessTransport {
    /// Resolves `msg.to` to a destination `RegionId`: `Address::Region(id) => id`
    /// directly. `Address::Entity`/`Address::Chunk` are out of this blueprint's scope
    /// (see Context) and immediately return `Err(TransportError::Backpressure(msg))`,
    /// same as an unregistered `Address::Region` destination or a full channel — this
    /// blueprint's own deliberate unification of all three "cannot deliver right now"
    /// cases onto the one error variant `rc-messaging` provides. Never blocks
    /// (`crossbeam_channel::Sender::try_send`, non-blocking).
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError>;

    /// Non-blocking single-message drain from `into`'s channel. Returns `None` if `into`
    /// has no live channel or its channel is currently empty — both cases are
    /// indistinguishable via this call alone (`is_registered` answers the first
    /// separately, if a caller needs to).
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>>;
}
```

`InProcessTransport`'s fields are intentionally left unlisted in this Deliverables signature block (private, internal-shape freedom per the blueprint spec) — Implementation steps below fixes the exact internal shape this blueprint's own reasoning already committed to: a private `RegionChannel { sender: crossbeam_channel::Sender<Message<RegionMessage>>, receiver: crossbeam_channel::Receiver<Message<RegionMessage>> }` struct, and `InProcessTransport { channels: RwLock<HashMap<RegionId, RegionChannel>>, config: InProcessTransportConfig, entity_snapshot_pool: EntitySnapshotPool }`.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus `crates/transport-inproc/src/{entity_snapshot_pool.rs, transport.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments stay exactly as specified — including `InProcessTransport`'s private field shape fixed in the paragraph above, which the test changeset's `src/transport.rs` must already declare, since `register_region`/`deregister_region`/`send`/`try_recv` need somewhere to `todo!()` from), plus the one `Cargo.toml` edit. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/transport-inproc/tests/`, and must not change any type's field list, derive list, or public signature from what the test changeset already compiled against.

### `crates/transport-inproc/tests/basic_transport.rs`

Uses `rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId}` and `rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage, Transport, TransportError}` plus `rc_transport_inproc::{InProcessTransport, InProcessTransportConfig}`. A local helper:

```rust
fn synthetic_message(from: RegionId, to: Address, marker: u32) -> Message<RegionMessage> {
    Message {
        from,
        to,
        tick_stamp: 0,
        seq: 0,
        payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
            chunk: ChunkKey::new(DimensionId::OVERWORLD, 1, 1),
            pos: BlockPos::new(16, 64, 16),
            kind: BorderUpdateKind::BlockChanged { new_state: marker },
        }),
    }
}
```

1. `send_and_recv_single_message_between_two_regions` — new transport (`InProcessTransportConfig::default()`); `register_region(RegionId(1))`, `register_region(RegionId(2))`; `transport.send(synthetic_message(RegionId(1), Address::Region(RegionId(2)), 1)).unwrap()`; `transport.try_recv(RegionId(2))` returns `Some(msg)` where `msg == synthetic_message(RegionId(1), Address::Region(RegionId(2)), 1)`; a second `try_recv(RegionId(2))` returns `None`.
2. `try_recv_on_unregistered_region_returns_none` — fresh transport, no registration; `try_recv(RegionId(42))` returns `None`.
3. `send_to_unregistered_region_returns_backpressure_with_original_message` — fresh transport (`RegionId(99)` never registered); `let msg = synthetic_message(RegionId(1), Address::Region(RegionId(99)), 7);` `let err = transport.send(msg.clone()).unwrap_err();` assert `matches!(err, TransportError::Backpressure(returned) if returned == msg)`.
4. `send_respects_bounded_capacity_and_reports_backpressure` — `InProcessTransportConfig { channel_capacity: 2, ..Default::default() }`; `register_region(RegionId(1))`; send two messages (markers `1`, `2`) successfully (`.unwrap()` both); send a third (marker `3`) — assert `Backpressure` returned, and the returned message's marker is `3` (extracted via pattern match on `BorderUpdateKind::BlockChanged`); `try_recv(RegionId(1))` once (drains marker `1`, freeing one slot); send a fourth (marker `4`) — assert `Ok(())`.
5. `register_region_is_idempotent_and_replaces_channel` — `register_region(RegionId(1))`; send one message to it (not yet drained); `register_region(RegionId(1))` again; assert `try_recv(RegionId(1))` now returns `None` (old in-flight message dropped by the replacement); send a new message; assert `try_recv` returns it (new channel functions correctly).
6. `deregister_region_drops_channel_and_future_sends_backpressure` — `register_region(RegionId(1))`; `deregister_region(RegionId(1))`; assert `!is_registered(RegionId(1))`; `send(...)` to it returns `Backpressure`; `try_recv(RegionId(1))` returns `None`.
7. `deregister_unregistered_region_is_a_noop` — fresh transport; `deregister_region(RegionId(7))` does not panic; `is_registered(RegionId(7))` stays `false` before and after.
8. `address_entity_and_address_chunk_currently_return_backpressure` — fresh transport; `let msg_entity = synthetic_message(RegionId(1), Address::Entity(RcEntityId::from_raw(5)), 1);` `let err = transport.send(msg_entity.clone()).unwrap_err();` assert `matches!(err, TransportError::Backpressure(r) if r == msg_entity)`; repeat identically for `Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 0, 0))`. Documents this blueprint's explicit current-scope limitation (Context).
9. `default_config_matches_arch_d27_and_d28` — `InProcessTransportConfig::default().channel_capacity == 4096`; `InProcessTransportConfig::default().entity_snapshot_pool_capacity == 256`.

### `crates/transport-inproc/tests/entity_snapshot_pool.rs`

Uses `rc_core::{ChunkKey, DimensionId, RcEntityId}`, `rc_messaging::EntitySnapshot`, `rc_transport_inproc::EntitySnapshotPool`.

1. `acquire_on_empty_pool_allocates_fresh` — `EntitySnapshotPool::new(4)`; `pool.free_count() == 0`; `let v = EntitySnapshot { entity_id: RcEntityId::from_raw(1), source_chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0), component_data: vec![9] };` `let slot = pool.acquire(v.clone());` assert `*slot == v`.
2. `release_then_acquire_reuses_the_same_allocation` — `EntitySnapshotPool::new(4)`; `let v_a = EntitySnapshot { entity_id: RcEntityId::from_raw(1), source_chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0), component_data: vec![1] };` `let slot_a = pool.acquire(v_a.clone());` `let addr_a = std::ptr::addr_of!(*slot_a) as usize;` `pool.release(slot_a);` assert `pool.free_count() == 1`; `let v_b = EntitySnapshot { entity_id: RcEntityId::from_raw(2), source_chunk: ChunkKey::new(DimensionId::THE_NETHER, 1, 1), component_data: vec![2, 3] };` `let slot_b = pool.acquire(v_b.clone());` assert `std::ptr::addr_of!(*slot_b) as usize == addr_a` (proves the same allocation was reused, not a fresh one) and `*slot_b == v_b` (proves the reused allocation's contents were fully overwritten, no stale data from `v_a` leaking through); assert `pool.free_count() == 0` after this second `acquire`.
3. `release_beyond_capacity_drops_the_extra_slot` — `EntitySnapshotPool::new(1)`; acquire two fresh boxes with distinct values `v1`/`v2` (queue starts empty, so both are fresh allocations, not reuses); `pool.release(box1)` — assert `pool.free_count() == 1`; `pool.release(box2)` — assert `pool.free_count() == 1` still (not `2`; the second release was dropped since the pool was already at `capacity`).
4. `acquire_and_release_are_thread_safe_under_contention` — `EntitySnapshotPool::new(16)`; `std::thread::scope` with 8 threads, each performing 200 iterations of `let v = EntitySnapshot { entity_id: RcEntityId::from_raw(thread_idx as u64 * 1000 + i as u64), source_chunk: ChunkKey::new(DimensionId::OVERWORLD, thread_idx as i32, i as i32), component_data: Vec::new() }; let slot = pool.acquire(v); pool.release(slot);` sharing one `&EntitySnapshotPool`; after joining, assert `pool.free_count() <= 16` (the capacity bound is never exceeded even under concurrent acquire/release — the test's only assertion, since the exact count is a benign race per Context).

### `crates/transport-inproc/tests/fifo_property.rs`

A `proptest!` property test exercising the **real** `InProcessTransport` (not a mock) under genuinely concurrent multi-threaded send, verifying ARCH-D29's FIFO-per-`(from, to)`-pair and exactly-once guarantees:

```rust
use proptest::prelude::*;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage, Transport};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

const SENDER_IDS: [RegionId; 4] = [RegionId(100), RegionId(101), RegionId(102), RegionId(103)];
const DESTINATION: RegionId = RegionId(999);

fn marker_payload(marker: u32) -> RegionMessage {
    RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        pos: BlockPos::new(0, 0, 0),
        kind: BorderUpdateKind::BlockChanged { new_state: marker },
    })
}

proptest! {
    #[test]
    fn fifo_and_exactly_once_under_concurrent_send(
        entries in prop::collection::vec(0u8..4, 0..200)
    ) {
        // `entries[i]` selects one of 4 synthetic sender RegionIds; `i` (its own 0-based
        // index) is this element's globally unique marker.
        let transport = InProcessTransport::new(InProcessTransportConfig::default());
        transport.register_region(DESTINATION);

        // Partition into one bucket per sender, preserving original relative order —
        // each bucket becomes one thread's strictly sequential send order.
        let mut buckets: [Vec<u32>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (idx, &selector) in entries.iter().enumerate() {
            buckets[selector as usize].push(idx as u32);
        }

        std::thread::scope(|scope| {
            for (bucket_idx, bucket) in buckets.iter().enumerate() {
                let transport_ref = &transport;
                let from = SENDER_IDS[bucket_idx];
                let bucket = bucket.clone();
                scope.spawn(move || {
                    for marker in bucket {
                        let msg = Message {
                            from,
                            to: Address::Region(DESTINATION),
                            tick_stamp: 0,
                            seq: 0,
                            payload: marker_payload(marker),
                        };
                        transport_ref.send(msg).expect("default capacity 4096 exceeds this test's bound (200)");
                    }
                });
            }
        });

        let mut received: Vec<(RegionId, u32)> = Vec::new();
        while let Some(msg) = transport.try_recv(DESTINATION) {
            let marker = match msg.payload {
                RegionMessage::BorderUpdateEvent(e) => match e.kind {
                    BorderUpdateKind::BlockChanged { new_state } => new_state,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            };
            received.push((msg.from, marker));
        }

        // (a) No loss, no duplication.
        let mut received_markers: Vec<u32> = received.iter().map(|(_, m)| *m).collect();
        received_markers.sort_unstable();
        let mut expected_markers: Vec<u32> = (0..entries.len() as u32).collect();
        expected_markers.sort_unstable();
        prop_assert_eq!(received_markers, expected_markers);

        // (b) FIFO per (from, to) pair: each sender's received subsequence matches its
        // own original emission order exactly.
        for (bucket_idx, bucket) in buckets.iter().enumerate() {
            let from = SENDER_IDS[bucket_idx];
            let this_sender_received: Vec<u32> =
                received.iter().filter(|(f, _)| *f == from).map(|(_, m)| *m).collect();
            prop_assert_eq!(&this_sender_received, bucket);
        }
    }
}
```

(Counts as one `#[test]`-level case under `cargo nextest run`, per proptest's own macro expansion and consistent with M0-B02's identical framing of its own `fifo_property.rs`.)

### `crates/transport-inproc/tests/cross_region_timing.rs`

Defines a test-only harness (not a deliverable):

```rust
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage,
    RegionMessageBus, RegionMessageState, Transport,
};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

/// Test-only, single-threaded stand-in for `rc-scheduler`'s not-yet-existing tick
/// driver. Implements exactly the Stage-1/Stage-10 contract this blueprint's Context
/// section restates from M0-B02, one explicit method call per stage boundary — no real
/// thread, no sleep, no wall clock; every "tick" advances only when the test calls one
/// of these methods.
struct FakeRegion {
    id: RegionId,
    state: RegionMessageState,
    tick_counter: u64,
}

impl FakeRegion {
    fn new(id: RegionId) -> Self {
        Self { id, state: RegionMessageState::new(), tick_counter: 0 }
    }

    /// Stage 1: drain every currently-queued inbound message from `transport`, call
    /// `set_inbox` exactly once with the payloads, and return the full envelopes drained
    /// (for this test's own inspection — production code only ever sees `.inbox()`'s
    /// payload-only view).
    fn stage1(&mut self, transport: &dyn Transport) -> Vec<Message<RegionMessage>> {
        let mut drained = Vec::new();
        while let Some(msg) = transport.try_recv(self.id) {
            drained.push(msg);
        }
        let payloads = drained.iter().map(|m| m.payload.clone()).collect();
        self.state.set_inbox(payloads);
        drained
    }

    /// Stand-in for one domain system's buffered send merged into region state.
    fn emit(&mut self, to: Address, message: RegionMessage) {
        let mut bus = RegionMessageBus::new();
        bus.send(to, message);
        self.state.merge(bus);
    }

    /// Stage 10: drain this region's outbox (stamping `from`/`tick_stamp`/`seq`), flush
    /// every resulting envelope through `transport` in order, then advance this region's
    /// own tick counter.
    fn stage10(&mut self, transport: &dyn Transport) {
        let outgoing = self.state.drain_outbox(self.id, self.tick_counter);
        for msg in outgoing {
            transport
                .send(msg)
                .expect("default capacity 4096 is never exhausted by this test");
        }
        self.tick_counter += 1;
    }
}

fn synthetic_border_update(marker: u32) -> RegionMessage {
    RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 5, -3),
        pos: BlockPos::new(80, 64, -48),
        kind: BorderUpdateKind::BlockChanged { new_state: marker },
    })
}
```

1. `border_update_applied_at_destination_next_stage1_not_same_tick_not_two_later` — **the precise M0 acceptance criterion 2 test.** `let transport = InProcessTransport::new(InProcessTransportConfig::default());` register `RegionId(1)` (region A) and `RegionId(2)` (region B); construct `FakeRegion`s for both. Sequence:
   - `let before_send = region_b.stage1(&transport);` — assert `before_send.is_empty()` (B's Stage-1, run before A has sent anything, observes nothing).
   - `let _ = region_a.stage1(&transport);` (A's own Stage 1 — nothing inbound for A in this test) — `region_a.emit(Address::Region(RegionId(2)), synthetic_border_update(777));` — `region_a.stage10(&transport);` (flushes the one message through the transport, stamping `tick_stamp: 0` since this is A's first `stage10` call).
   - Re-assert `before_send.is_empty()` (the value captured before A's send is an owned, already-returned `Vec` — restating this after the send documents that nothing sent later can retroactively appear in it; see Context for why this is structurally guaranteed, not merely asserted).
   - `let next_after_send = region_b.stage1(&transport);` — assert `next_after_send.len() == 1`; assert `next_after_send[0].payload == synthetic_border_update(777)`; assert `next_after_send[0].from == RegionId(1)`; assert `next_after_send[0].tick_stamp == 0`; assert `region_b.state.inbox() == &[synthetic_border_update(777)]`.
   - `let one_more_after_that = region_b.stage1(&transport);` — assert `one_more_after_that.is_empty()` (not delayed further, not duplicated).
2. `multiple_border_updates_in_one_flush_preserve_emission_order_at_next_stage1` — register `RegionId(10)`/`RegionId(20)`; `region_a.stage1(&transport)` once; `region_a.emit(Address::Region(RegionId(20)), synthetic_border_update(0))`, `region_a.emit(Address::Region(RegionId(20)), synthetic_border_update(1))`, `region_a.emit(Address::Region(RegionId(20)), synthetic_border_update(2))` (three separate emissions, i.e. three separate merged buses, mirroring three separate domain systems each emitting once); `region_a.stage10(&transport)`; `let received = region_b.stage1(&transport);` extract each received message's `new_state` marker via the same pattern used in `fifo_property.rs`; assert the markers equal `[0, 1, 2]` in that exact order.
3. `bidirectional_exchange_between_two_regions` — register `RegionId(100)`/`RegionId(200)`; both regions call `stage1` once (no-op, nothing pending); A emits+flushes marker `1` addressed to B; B emits+flushes marker `2` addressed to A; both regions then call `stage1` again; assert A received exactly one message, payload `synthetic_border_update(2)`, `from == RegionId(200)`; assert B received exactly one message, payload `synthetic_border_update(1)`, `from == RegionId(100)` — proving both directions work correctly over one shared `InProcessTransport` between two artificially-split regions.

## Implementation steps

1. **`Cargo.toml`.** Add the three normal dependencies and two dev-dependencies exactly as specified in Deliverables. Observable: `cargo metadata` resolves; `cargo build -p rc-transport-inproc` still only compiles against `todo!()` bodies at this point if starting from the test changeset's red state.
2. **`src/entity_snapshot_pool.rs`.** Real bodies: `new(capacity)` is `Self { free: SegQueue::new(), free_count: AtomicUsize::new(0), capacity }`. `acquire(value)`: if `self.free.pop()` returns `Some(mut slot)`, `self.free_count.fetch_sub(1, Ordering::AcqRel); *slot = value; slot` — else `Box::new(value)`. `release(slot)`: loop { `let current = self.free_count.load(Ordering::Acquire); if current >= self.capacity { return; } if self.free_count.compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Acquire).is_ok() { self.free.push(slot); return; } }`. `free_count()` is `self.free_count.load(Ordering::Acquire)`. Observable: `cargo nextest run -p rc-transport-inproc --test entity_snapshot_pool` — all 4 cases pass (once step 1's `Cargo.toml` is in place; `src/transport.rs` may remain `todo!()`-stubbed at this point).
3. **`src/transport.rs` — types and constructor.** Fix the private shape: `struct RegionChannel { sender: crossbeam_channel::Sender<Message<RegionMessage>>, receiver: crossbeam_channel::Receiver<Message<RegionMessage>> }`; `pub struct InProcessTransport { channels: RwLock<HashMap<RegionId, RegionChannel>>, config: InProcessTransportConfig, entity_snapshot_pool: EntitySnapshotPool }`. `InProcessTransportConfig::default()` returns `Self { channel_capacity: 4096, entity_snapshot_pool_capacity: 256 }`. `InProcessTransport::new(config)` returns `Self { channels: RwLock::new(HashMap::new()), entity_snapshot_pool: EntitySnapshotPool::new(config.entity_snapshot_pool_capacity), config }`.
4. **`src/transport.rs` — registration methods.** `register_region(id)`: `let (sender, receiver) = crossbeam_channel::bounded(self.config.channel_capacity); self.channels.write().insert(id, RegionChannel { sender, receiver });` (insert overwrites unconditionally on re-registration, per the documented replace semantics). `deregister_region(id)`: `self.channels.write().remove(&id);`. `is_registered(id)`: `self.channels.read().contains_key(&id)`. `entity_snapshot_pool()`: `&self.entity_snapshot_pool`.
5. **`src/transport.rs` — `Transport` impl.** `send(msg)`: match `msg.to` (copies out — `Address` is `Copy` per M0-B02, so `msg` itself is not partially moved) — `Address::Region(destination) => destination`, `Address::Entity(_) | Address::Chunk(_) => return Err(TransportError::Backpressure(msg))`; then `let channels = self.channels.read(); match channels.get(&destination) { Some(channel) => match channel.sender.try_send(msg) { Ok(()) => Ok(()), Err(crossbeam_channel::TrySendError::Full(returned)) | Err(crossbeam_channel::TrySendError::Disconnected(returned)) => Err(TransportError::Backpressure(returned)) }, None => Err(TransportError::Backpressure(msg)) }`. `try_recv(into)`: `let channels = self.channels.read(); let channel = channels.get(&into)?; match channel.receiver.try_recv() { Ok(msg) => Some(msg), Err(_) => None }` (both `Empty` and the by-construction-unreachable `Disconnected` map to `None` — see Context). Observable: `cargo build -p rc-transport-inproc` succeeds with zero `todo!()` remaining.
6. **Run the full acceptance suite.** `cargo nextest run -p rc-transport-inproc` — every test named in Acceptance tests passes, across all four test files.
7. **Doctests.** `cargo test --doc -p rc-transport-inproc` passes (no runnable doc examples required by this blueprint; guards against accidentally introducing a broken one).
8. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `cargo run -p xtask -- lint`, `cargo run -p xtask -- lint-deps`, `cargo run -p xtask -- test` — all four still exit 0.
9. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs of `.github/workflows/ci.yml` (M0-B01) go green on a clean checkout — the authoritative done-signal (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/transport-inproc/tests/` is committed first, alongside `todo!()`-stubbed (but otherwise complete: full field lists, full derives, full doc comments, including `InProcessTransport`'s private field shape) `src/{entity_snapshot_pool.rs, transport.rs}` and the one `Cargo.toml` edit. The implementation changeset (steps 1–9 above) fills in real bodies only — it must not edit any test file, must not add, remove, or rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, the exact tick-stamp/ordering/pointer-identity assertions in `cross_region_timing.rs` and `entity_snapshot_pool.rs`'s reuse test must survive unchanged).

(b) **No new external dependencies beyond the pinned set.** Every external crate this blueprint's deliverables use — `crossbeam-channel`, `crossbeam-queue`, `parking_lot` (all already in `[workspace.dependencies]` since M0-B01) and `proptest` (already added by M0-B02, not re-added here) — is already pinned; this blueprint adds **zero** new lines to the workspace root `Cargo.toml`. Do not add `tokio`, `crossbeam-deque`, `crossbeam-utils`, `anyhow`, or any other crate to `rc-transport-inproc`'s `Cargo.toml` under any circumstance.

(c) **No Mojang or third-party reimplementation code.** Nothing in this blueprint touches protocol wire format, decompiled game logic, or any other project's source — every type and algorithm here is derived solely from `docs/planning/01-server-architecture.md`'s ARCH-D23/D27/D28/D29/D11 and this blueprint's own concrete, cited resolutions of what those decisions and their Open Questions left unresolved (ASSET-D18/D19/D30).

(d) **No `unsafe` code.** Every type and function in this blueprint's deliverables is implementable in 100% safe Rust — `parking_lot::RwLock`, `crossbeam_channel::{Sender, Receiver}`, `crossbeam_queue::SegQueue`, and `std::sync::atomic::AtomicUsize` are all safe-to-use `std`/crate types; no raw pointers beyond the test-only `std::ptr::addr_of!` read used purely for pointer-identity comparison in `entity_snapshot_pool.rs`'s reuse test (a safe operation — it takes a reference's address, dereferences nothing through a raw pointer), no `unsafe impl`, no FFI.

(e) **Scope boundary — do not implement beyond this blueprint's one crate.** This blueprint does not implement `rc-scheduler`'s 11-stage tick pipeline, RC-Executor, RC-WorkerPool, or the real Stage-1/Stage-10 driver that will eventually call into this crate's `InProcessTransport` from production code (ARCH-D1–D9/D12/D18–D23 — a separate, not-yet-written M0 blueprint); does not implement ARCH-D6's real region split/merge algorithm or ARCH-D24's `ChunkKey -> RegionId`/`RcEntityId -> RegionId` directories (`rc-scheduler`'s job — structurally unreachable from this crate per Rule 2, see Context); does not implement `Address::Entity`/`Address::Chunk` resolution inside `InProcessTransport` (see Context — returns `Backpressure` instead); does not implement `NetworkTransport` or any cluster-mode code (`13-cluster-architecture.md`'s `rc-transport-net`, gated behind the `cluster` feature, a wholly separate crate). This blueprint's tests are Tier-1 only — it does not wire anything into TEST-D17–19's determinism corpora, TEST-D11's differential scenario corpus, or M0 acceptance criterion 1's 10-minute 8-region soak test (all three require a real tick driver this blueprint deliberately does not build). Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-transport-inproc --all-features
cargo nextest run -p rc-transport-inproc
cargo test --doc -p rc-transport-inproc
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-transport-inproc` runs 9 (`basic_transport.rs`) + 4 (`entity_snapshot_pool.rs`) + 1 (`fifo_property.rs`, one property-test case regardless of internal proptest-generated input count) + 3 (`cross_region_timing.rs`) = 17 test cases named in Acceptance tests — all pass. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
