# M0-B02 — Core Types & Messaging Substrate

| Field | Content |
|---|---|
| ID | M0-B02 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | M0-B01 (workspace scaffold: root `Cargo.toml`, `crates/core/`, `crates/messaging/` exist as empty-shell crates already wired into the workspace and into `xtask lint-deps`'s Rule 3 check) |
| Implements | ARCH-D24 (location-transparent addressing types), ARCH-D25 (message envelope), ARCH-D26 (`Transport` trait), ARCH-D28 (pooling seam only — not the pool itself), ARCH-D29 (delivery/ordering guarantees), ARCH-D30 (`RegionMessageBus` ECS-facing API); the parts of ARCH-D5/D6 needed to define `RegionId`'s identity contract; ARCH-D11's Stage-1/Stage-4 timing contract restated as the drain/apply invariant the bus's callers must uphold; CLUSTER-D12 (serde-derive requirement rc-messaging's types must satisfy so `postcard` can consume them unmodified in a later milestone) |
| Crates touched | `rc-core` (`crates/core/`), `rc-messaging` (`crates/messaging/`); one line added to the workspace root `Cargo.toml`'s `[workspace.dependencies]` table |
| Estimated scope | L |

## Goal & Done definition

Fill in `rc-core`'s foundational coordinate/identifier types and `rc-messaging`'s complete message-passing substrate — addressing, the envelope, the `Transport` trait signature, the `RegionMessage` payload enum (`BorderUpdateEvent`, `RegionTransferRequest`), and the ECS-facing send/receive bus — with **no transport implementation** (`InProcessTransport` is `rc-transport-inproc`'s job, a later blueprint). Every type that can appear inside a `Message<RegionMessage>` derives `serde::Serialize`/`Deserialize` so a later milestone's `postcard`-based `NetworkTransport` (CLUSTER-D12) consumes them completely unmodified. Both crates keep `xtask lint-deps`'s Rule 3 (`rc-messaging`'s exact normal-dependency set is `{rc-core, serde, thiserror}`, M0-B01) green throughout.

Done when:

- [ ] `cargo build -p rc-core -p rc-messaging --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-core -p rc-messaging`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 (Rule 3's exact set is unchanged by this blueprint — only `[dev-dependencies]` grow).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0 against the two crates' new content.
- [ ] `cargo test --doc -p rc-core -p rc-messaging` exits 0 (every doc example, if any is written as a runnable example, compiles and passes — none is required by this blueprint, but none may be broken).
- [ ] Every public type reachable from `RegionMessage` round-trips through `postcard` byte-for-byte equal to its original value (the envelope round-trip acceptance test).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test` — M0-B01's four gates, the only Tier-1 content that exists at this point in the project) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Why `rc-core` gains a dependency M0-B01 did not give it

M0-B01 scaffolded `crates/core/Cargo.toml` with **zero** dependencies — correct for an empty-shell placeholder, since no rule in `12-workspace-structure.md`'s WS-D3 restricts `rc-core`'s dependency set the way Rule 3 restricts `rc-messaging`'s exact set. This blueprint adds exactly one line, `serde = { workspace = true }`, to `crates/core/Cargo.toml`'s `[dependencies]`. It is required, not optional: ARCH-D25 requires every `RegionMessage` variant to derive `serde::Serialize`/`Deserialize`, and `BorderUpdateEvent`/`EntitySnapshot` (defined in `rc-messaging`) embed `rc-core`'s `ChunkKey`, `BlockPos`, and `RcEntityId` directly — for `#[derive(Serialize, Deserialize)]` to work on a struct, every field's type must itself implement those traits, so `rc-core`'s own coordinate/identifier types must derive them too. `xtask lint-deps`'s four rules (M0-B01) say nothing about `rc-core`'s dependency count, so this addition violates no CI-enforced rule.

### `rc-messaging` cannot depend on `bevy_ecs` — this shapes the bus design

`xtask lint-deps`'s Rule 3 (M0-B01, sourced from `12`'s WS-D3 rule 3) is CI-enforced and exact: `rc-messaging`'s complete set of **normal** dependencies must be `{rc-core, serde, thiserror}` — no more, no fewer. `bevy_ecs` is not in that set. This matters because ARCH-D30 describes `RegionMessageBus` as "injected like `Commands`" — bevy_ecs's own `Commands` type is built on bevy_ecs's `Deferred<T>`/`SystemBuffer` machinery, which this crate cannot use. It does not need to: `01-server-architecture.md`'s ARCH-D3 already establishes that Rusty Clanker's tick execution runs on a **custom** executor (RC-Executor/RC-WorkerPool), not on `bevy_ecs`'s own `Schedule`/`MultiThreaded` executor — so "injected like `Commands`" describes the *developer experience* ARCH-D30 wants (a system gets a private outbound buffer; buffered sends become visible only at a sync point), not a requirement that this crate reuse `bevy_ecs`'s own `SystemParam` trait machinery to get there.

This blueprint therefore splits the bus into two plain, `bevy_ecs`-free types (full design in Deliverables):

- **`RegionMessageBus`** — a private, per-invocation send buffer. Whoever hands one to a running domain system (that integration is `rc-scheduler`'s job, a later M0 blueprint — see Constraints) gives each system its own instance, so concurrently-running systems in the same ARCH-D8 domain group never contend over shared mutable state through this type.
- **`RegionMessageState`** — the region-owned canonical state: every finished system's `RegionMessageBus` is `merge`d into it (in merge order) at whatever sync-point mechanism `rc-scheduler` implements; it also holds the current tick's drained inbound queue.

This mirrors the *shape* of `bevy_ecs`'s own `Commands`/`CommandQueue` pattern (private buffer, merged at a sync point) without depending on `bevy_ecs`'s specific trait machinery — a deliberate, documented substitution, not an oversight.

### Location-transparent addressing (ARCH-D24)

Two global identifiers exist independent of runtime placement. `ChunkKey` is **permanent per chunk** and lives in `rc-core` (12's Crate Manifest: "`ChunkKey`, `DimensionId`... the graph's root leaf"). `RcEntityId` is **monotonic, allocated once at spawn**, distinct from the ephemeral intra-`World` `bevy_ecs::Entity` index+generation, and stable across ARCH-D10 transfers — also `rc-core`. The two directories this blueprint does **not** implement (`ChunkKey -> RegionId`, `RcEntityId -> RegionId` — ARCH-D24 says these are "mutated only at ARCH-D9 sync points... read-only during a tick") belong to whichever later blueprint drives region ownership (`rc-scheduler`); this blueprint only fixes the key types those directories will be keyed by.

`DimensionId` is not given a concrete Rust shape anywhere in the planning corpus — this blueprint fixes one: a `Copy`, cheap `u16` handle (not a namespaced string), because `ChunkKey` is used as a hash-map key on a hot path (every cross-region directory lookup) and must stay trivially `Copy`. Concrete dimension registration (mapping a data-pack-declared dimension to one of these handles) is out of scope here — this blueprint only reserves indices `0`/`1`/`2` for vanilla's three built-in dimensions so debug output is stable across builds regardless of when other dimensions get registered.

`RegionId` is `rc-messaging`'s (12's Crate Manifest: "Location-transparent addressing (`RegionId`, `Address`)... `rc-messaging`"). Its identity contract, restated from the parts of ARCH-D5/D6 that matter here: a region (ARCH-D5) owns a contiguous, mutable set of chunks, and no two regions ever hold a chunk simultaneously; regions are built from, merged from, and split along 16×16-chunk grid cells (ARCH-D6). This blueprint does not implement region build/merge/split (that is `rc-scheduler`'s ARCH-D6 responsibility) — it only fixes that **`RegionId` values are never reused within one server process's lifetime**, even after the region they named has since merged away. This is required so a `Message<RegionMessage>` still in flight at the moment of a merge remains unambiguously attributable to the region that sent or was meant to receive it. A `u64` newtype gives more than enough range that this invariant is free to uphold (any allocation policy that never decrements or recycles a counter satisfies it) — this blueprint does not ship an allocator for `RegionId` itself, since deciding *when* a new region gets an ID is `rc-scheduler`'s ARCH-D6 algorithm, not an addressing concern.

### The envelope (ARCH-D25)

Exact shape, copied from ARCH-D25: `Message<T> { from: RegionId, to: Address, tick_stamp: u64, seq: u32, payload: T }`, where `Address` is `Region(RegionId) | Entity(RcEntityId) | Chunk(ChunkKey)`. ARCH-D25: "The sending region resolves `Address::Entity`/`Address::Chunk` to a concrete destination `RegionId` at emission time via the ARCH-D24 directories — never re-resolved by the receiver." This resolution step is **not** implemented by this blueprint (it needs the ARCH-D24 directories, owned elsewhere) — it happens inside whichever concrete `Transport` implementation calls `Transport::send` (ARCH-D27 confirms this for the in-process case: "the destination `RegionId -> Sender` lookup reuses that same region ownership table"). This crate's `Message.to` field simply carries whatever `Address` the caller of `RegionMessageBus::send` specified, unresolved — the payload itself (e.g. `BorderUpdateEvent.chunk`) already carries whatever chunk/entity-level detail matters downstream, so nothing is lost by not resolving here.

`tick_stamp` is region-local (CLUSTER-D25 confirms this stays true even in cluster mode: "each region's own monotonic tick counter, unchanged by cluster mode") — this blueprint's `RegionMessageState::drain_outbox` takes it as a caller-supplied parameter rather than tracking it itself, since region tick-counter ownership belongs to `rc-scheduler`.

`seq` semantics are **not** pinned by any planning document beyond existing on the envelope — this blueprint fixes them concretely: a **monotonic counter per distinct `to: Address` value**, starting at `0`, persisting across ticks (never reset). This is deliberately keyed by the raw, unresolved `Address` value, because ARCH-D29 defines its ordering/exactly-once guarantee "per ordered `(from: RegionId, to: Address)` pair" — using the literal `Address`, not a value resolved through a directory this crate does not have. `RegionMessageState` owns this counter (one `HashMap<Address, u32>` entry per distinct destination `Address` ever sent to by that region) so it survives across ticks without needing an external store.

### The `Transport` trait (ARCH-D26) and delivery/ordering guarantees (ARCH-D29)

Exact signature, copied from ARCH-D26: `trait Transport: Send + Sync + 'static { fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError>; fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>>; }`. This blueprint defines the trait and `TransportError`; it defines **zero** implementations of the trait (`InProcessTransport` is `rc-transport-inproc`'s crate and a separate blueprint; this crate must not gain a `crossbeam-channel` dependency, which would break Rule 3).

ARCH-D29's guarantees, restated as the exact properties any `Transport` implementation (and this blueprint's test-only mock) must uphold:

1. **FIFO per `(from, to)` pair.** For any two messages `m1` sent before `m2` with the same `(from, to)`, `try_recv` on the destination never returns `m2` before `m1`.
2. **Exactly-once, process lifetime.** A sent message is delivered to exactly one `try_recv` call, exactly once, for the process's lifetime — never lost, never duplicated.
3. **No cross-pair ordering.** Messages from different `(from, to)` pairs may interleave in any order at the receiver — a region's Stage-1 drain must tolerate arbitrary arrival-order variance across senders.
4. **Never blocks the sender.** A full destination inbox returns `TransportError::Backpressure` rather than blocking — ARCH-D29: "a region never blocks its own tick waiting on another region's inbox."

`TransportError`'s one pinned variant is `Backpressure` (ARCH-D29's exact name); this document's Open Questions flag that the *retry mechanism* for it is unspecified ("defines a single retry-next-tick signal but not a bounded retry policy... needs a blueprint-phase decision"). This blueprint makes that decision: `send` fully consumes `msg` by value per ARCH-D26's exact signature, so the only way a caller can retry the *same* message next tick is if the error itself hands it back — `TransportError::Backpressure(Message<RegionMessage>)` returns the un-delivered message, mirroring `std::sync::mpsc::SendError<T>`'s well-established "give the value back" convention. This is a concrete resolution of a decision the planning corpus explicitly deferred to blueprint phase, not a deviation from it.

### The pooling seam (ARCH-D28) — without owning the pool

ARCH-D28: `BorderUpdateEvent` is sized to fit inline, ≤128 bytes, no heap allocation; `RegionTransferRequest`'s larger `EntitySnapshot` payload is drawn from a pool (`crossbeam-queue::SegQueue<Box<EntitySnapshot>>`) that a later blueprint (`rc-transport-inproc`, per 12's Crate Manifest: "the `SegQueue` slot-pool allocator for large payloads") owns — `rc-messaging` cannot depend on `crossbeam-queue` (Rule 3). The seam this blueprint provides is the **type shape**, nothing more: `RegionMessage::RegionTransferRequest` carries a `Box<EntitySnapshot>`. A `Box<T>` popped from a `SegQueue<Box<T>>` and a `Box<T>` from the ordinary global allocator are indistinguishable at the type level — whichever allocation strategy constructs the box is invisible to `rc-messaging` and to anything that only ever sees the already-boxed value. `BorderUpdateEvent` is embedded directly (no `Box`) so it stays inline; this blueprint's own acceptance tests assert `size_of::<RegionMessage>() <= 128` as a standing regression guard on that inline budget.

`EntitySnapshot`'s internal shape is a **placeholder** at M0: concrete entity components do not exist until `05-game-mechanics.md` lands (`11-roadmap-milestones.md`'s M4 scope: cross-region entity transfer is "exercised with real players and mobs... for the first time" only at M4; M0's own acceptance criteria exercise only `BorderUpdateEvent`). `EntitySnapshot` therefore carries an opaque `component_data: Vec<u8>` field today. Per ARCH-D25's own extension-point framing ("`13-cluster-architecture.md` may add cluster-only variants... without changing this envelope"), the blueprint that first implements real entity-component snapshotting replaces this field's *contents* with concrete typed data without changing `RegionMessage::RegionTransferRequest`'s outer `Box<EntitySnapshot>` shape, so nothing downstream (the `Transport` trait, the pooling seam, the bus) needs to change when that happens.

### The ECS-facing bus API and the Stage-1/Stage-10 contract (ARCH-D30, ARCH-D11)

ARCH-D30: systems never call `Transport::send`/`try_recv` directly; sends are buffered and flushed to `dyn Transport` at Stage 10 in emission order (the same deferred-apply discipline as ARCH-D9); inbound messages are drained from `dyn Transport` into a per-tick queue at Stage 1, consumed read-only by any Stage-1..N system. This blueprint fixes the exact contract a later `rc-scheduler` blueprint's tick driver must implement against the types below — it does **not** implement the driver itself (there is no tick pipeline yet; that is ARCH-D1–D9/D12/D18–D23, explicitly a separate M0 blueprint per M0-B01's own Constraint (d)):

> **Stage-1 contract.** Before any Stage-1..N system for a region runs, the driver calls `Transport::try_recv(region_id)` repeatedly until it returns `None`, collecting every returned message's `.payload` in return order, then calls `RegionMessageState::set_inbox` **exactly once** with the full collected batch. No system calls `try_recv` directly.
>
> **Stage-10 contract.** After every system in the tick has run and every `RegionMessageBus` it produced has been `merge`d into the region's `RegionMessageState` (in merge order), the driver calls `RegionMessageState::drain_outbox(this_region_id, this_tick_counter)` exactly once, then calls `Transport::send` once per returned `Message`, in the order returned.
>
> **Timing consequence (ARCH-D11).** A message flushed at the sender's Stage 10 of tick N becomes visible via `try_recv` no earlier than the destination's very next Stage 1 — never within the sender's own tick N, and (once `Transport::send` has returned `Ok`) never delayed past the destination's next Stage-1 drain. This is what makes ARCH-D11's "applied... on its next tick" rule concrete and testable once a real `Transport` and tick driver exist (a later blueprint's acceptance test, not this one's — this blueprint's own tests exercise the bus and a mock transport in isolation, without a tick pipeline).

### Serde derive requirement (CLUSTER-D12) — the complete checklist

CLUSTER-D12: `postcard` consumes every `RegionMessage` variant's existing `serde` derives "directly with zero additional derive burden." For that to be literally true once `rc-transport-net` exists, **every** type reachable from `Message<RegionMessage>` must derive both `serde::Serialize` and `serde::Deserialize` today, not just `RegionMessage` itself. Complete list this blueprint is responsible for: `RegionId`, `Address`, `Message<T>` (generic — derives propagate the bound to `T`), `RegionMessage`, `BorderUpdateEvent`, `BorderUpdateKind`, `EntitySnapshot` (all `rc-messaging`), plus `ChunkKey`, `DimensionId`, `BlockPos`, `RcEntityId` (all `rc-core`, reached transitively). `TransportError` is **not** on this list — it is a local, in-process `Result` error that never itself crosses a wire (a `NetworkTransport` failure is a different, transport-owned error path), so it derives `thiserror::Error` only, not `serde`.

### Determinism note on `HashMap<Address, u32>`

`RegionMessageState`'s per-destination `seq` counters live in a `std::collections::HashMap`, whose iteration order is not deterministic across runs. This is safe here specifically because nothing in this blueprint ever *iterates* that map — every access is a point lookup/insert keyed by a specific `Address` value the caller already has, and `HashMap`'s point-access results are exactly deterministic for a given sequence of keys queried, regardless of internal bucket layout. This project's determinism-testing program (`09-testing-quality.md`'s TEST-D17–20) cares about iteration-order leaks into observable behavior; there is none here.

### Known limitation, not solved by this blueprint

`RegionMessageState.seq_counters` never removes an entry — a region that has ever sent a message to `Address::Entity(id)` keeps that counter alive even after the entity dies or transfers away. This is unbounded-but-slow growth (one `u32`-sized entry per distinct destination `Address` ever addressed, for that region's lifetime), invisible at M0's synthetic-load scale and explicitly deferred: real per-entity-addressed traffic does not exist before M4 (see the `EntitySnapshot` placeholder note above). A later blueprint (most plausibly whichever one first exercises real entity-targeted messaging at scale) should add pruning keyed off the ARCH-D24 directories' own liveness information, which this crate does not have access to.

## Deliverables

### `crates/core/Cargo.toml` (modify — add one dependency)

```toml
[package]
name = "rc-core"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
serde = { workspace = true }
```

### `crates/core/src/lib.rs`

```rust
//! `rc-core` — foundational shared types with zero I/O: coordinate math, entity-id
//! types, and the workspace-wide error/result convention every other crate follows
//! (see the crate-level docs on `rc_messaging::TransportError` for the first concrete
//! instantiation of that convention: a `thiserror`-derived, crate-local error enum,
//! never `Box<dyn Error>` or `anyhow`).
//!
//! `rc-core` itself has no fallible public constructors — every type here accepts any
//! value of its underlying representation without validation (e.g. `BlockPos` performs
//! no world-height range check; that belongs to whichever crate owns world bounds).

mod coords;
mod entity_id;

pub use coords::{BlockPos, ChunkKey, DimensionId};
pub use entity_id::{RcEntityId, RcEntityIdAllocator};
```

### `crates/core/src/coords.rs`

```rust
/// A dimension identifier: a small, `Copy` handle into the server's dimension table.
/// Indices `0`/`1`/`2` are reserved for vanilla's three built-in dimensions so debug
/// output is stable across builds; registration of additional (data-pack/mod) dimensions
/// into further indices is not implemented by this crate (a later blueprint's concern —
/// see this blueprint's Context section).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct DimensionId(pub u16);

impl DimensionId {
    pub const OVERWORLD: DimensionId = DimensionId(0);
    pub const THE_NETHER: DimensionId = DimensionId(1);
    pub const THE_END: DimensionId = DimensionId(2);
}

/// A chunk's permanent, location-independent identity (ARCH-D24). Exact field shape
/// pinned by ARCH-D25.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ChunkKey {
    pub dimension: DimensionId,
    pub x: i32,
    pub z: i32,
}

impl ChunkKey {
    pub const fn new(dimension: DimensionId, x: i32, z: i32) -> Self;
}

/// An absolute block position. `x`/`z` are horizontal, `y` is vertical (vanilla's own
/// axis convention). No range validation is performed by this type — the pinned
/// target's vertical bounds (-64..320) are enforced by whichever crate owns world
/// bounds, not by this coordinate type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self;

    /// The x coordinate of the 16x16-chunk column this position falls in: floor
    /// division by 16 (`x >> 4`, exact for `i32`'s arithmetic-shift semantics on
    /// negative values — floors toward negative infinity, matching vanilla's own
    /// chunk-coordinate convention).
    pub const fn chunk_x(self) -> i32;

    /// As `chunk_x`, for the z axis.
    pub const fn chunk_z(self) -> i32;

    /// This position's `ChunkKey` in the given dimension.
    pub const fn chunk_key(self, dimension: DimensionId) -> ChunkKey;
}
```

### `crates/core/src/entity_id.rs`

```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// A globally unique, monotonically-allocated entity identifier (ARCH-D24): "monotonic,
/// allocated once at spawn, distinct from the ephemeral intra-`World` `bevy_ecs::Entity`
/// index+generation, and stable across ARCH-D10 transfers." This type does not itself
/// enforce uniqueness on construction — use `RcEntityIdAllocator::alloc` for that; a raw
/// constructor is exposed for deserialization and test-fixture use, where reconstructing
/// a specific previously-allocated value (not minting a new one) is exactly the point.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RcEntityId(pub u64);

impl RcEntityId {
    /// Reconstruct a specific id value (deserialization, tests, or a future spawn-time
    /// integration handing this crate an id it decides not to hand out via
    /// `RcEntityIdAllocator`). Never call this to mint a *new* id in production code —
    /// that is `RcEntityIdAllocator::alloc`'s exclusive job.
    pub const fn from_raw(id: u64) -> Self;
}

/// A thread-safe, lock-free monotonic `RcEntityId` allocator. Every value returned by
/// `alloc` is strictly greater than every previously-returned value from the same
/// instance, and no two calls (even concurrent, from different `RC-WorkerPool` threads,
/// ARCH-D18) ever return the same value. Intended to be shared as a single
/// server-lifetime instance (e.g. behind an `Arc` or a `static`) — `alloc` takes `&self`,
/// not `&mut self`, precisely so callers never need external synchronization.
pub struct RcEntityIdAllocator(AtomicU64);

impl RcEntityIdAllocator {
    /// The first `alloc()` call on a freshly-constructed instance returns `RcEntityId(1)`.
    pub const fn new() -> Self;

    /// Allocate the next id. Thread-safe; never blocks.
    pub fn alloc(&self) -> RcEntityId;
}

impl Default for RcEntityIdAllocator {
    fn default() -> Self;
}
```

### `crates/messaging/Cargo.toml` (modify — add dev-dependencies only; normal deps unchanged from M0-B01)

```toml
[package]
name = "rc-messaging"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
serde = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
postcard = { workspace = true }
proptest = { workspace = true }
```

`postcard` and `proptest` are dev-only (test-target) dependencies. `xtask lint-deps`'s Rule 3 checks only **normal** dependencies (M0-B01's own `rule3_ignores_dev_dependency` test case establishes this exactly) — the normal-dependency set stays `{rc-core, serde, thiserror}`, unchanged.

### Workspace root `Cargo.toml` (modify — add one line to `[workspace.dependencies]`)

Add, alphabetically among the existing entries:

```toml
proptest          = "1.11.0"   # TEST-D27; used by rc-messaging's FIFO property test
```

`09-testing-quality.md`'s TEST-D27 pins `proptest` at exactly `1.11.0`; `12-workspace-structure.md`'s `[workspace.dependencies]` table predates full reconciliation with `09`'s tooling pins and does not yet list it (a gap, not a contradiction — nothing in `12` forbids `proptest`, it simply never got added). This blueprint adds the single missing line, sourced from TEST-D27's exact version, rather than declaring an ad hoc unpinned version string in `rc-messaging/Cargo.toml` (which WS-D7 forbids).

### `crates/messaging/src/lib.rs`

```rust
//! `rc-messaging` — location-transparent addressing, the `Message<RegionMessage>`
//! envelope, the `Transport` trait, the `RegionMessage` payload enum, and the
//! ECS-facing send/receive bus (ARCH-D24-D26, D28-D30). No transport implementation
//! and no network/IO dependency (`xtask lint-deps` Rule 3, M0-B01): this crate's
//! complete normal-dependency set is `{rc-core, serde, thiserror}`.

mod address;
mod bus;
mod envelope;
mod region_message;
mod transport;

pub use address::{Address, RegionId};
pub use bus::{RegionMessageBus, RegionMessageState};
pub use envelope::Message;
pub use region_message::{BorderUpdateEvent, BorderUpdateKind, EntitySnapshot, RegionMessage};
pub use transport::{Transport, TransportError};
```

### `crates/messaging/src/address.rs`

```rust
use rc_core::{ChunkKey, RcEntityId};

/// A region's identity (ARCH-D24's `RegionId -> ...` directory key). Never reused
/// within one server process's lifetime, even after the region it named merges away
/// (this blueprint's Context section explains why). This crate does not allocate
/// `RegionId` values — that is `rc-scheduler`'s ARCH-D6 region-lifecycle job; this
/// type only fixes the identifier's shape.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RegionId(pub u64);

/// Where a `RegionMessage` is headed. Exact shape pinned by ARCH-D25. Resolution of
/// `Entity`/`Chunk` to a concrete owning `RegionId` happens inside whichever concrete
/// `Transport` implementation calls `Transport::send` (ARCH-D25/ARCH-D27) — this crate
/// never performs that resolution itself and never re-resolves a received `Address`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Address {
    Region(RegionId),
    Entity(RcEntityId),
    Chunk(ChunkKey),
}
```

### `crates/messaging/src/envelope.rs`

```rust
use crate::Address;
use crate::RegionId;

/// The cross-partition message envelope (ARCH-D25). Exact field shape pinned there.
/// Generic over the payload so the type is reusable if a future revision ever needs a
/// second payload enum (none does today — every current use is `Message<RegionMessage>`).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message<T> {
    pub from: RegionId,
    pub to: Address,
    /// The *sending* region's own tick counter at emission time (CLUSTER-D25: stays
    /// region-local even in cluster mode).
    pub tick_stamp: u64,
    /// Monotonic per distinct `to: Address` value, starting at 0, persisting across
    /// ticks — this blueprint's concrete resolution of ARCH-D25's otherwise-unpinned
    /// `seq` semantics (see Context). Assigned by `RegionMessageState::drain_outbox`.
    pub seq: u32,
    pub payload: T,
}
```

### `crates/messaging/src/region_message.rs`

```rust
use rc_core::{BlockPos, ChunkKey, RcEntityId};

/// ARCH-D11: a block/redstone update whose propagation crosses into a neighbor region.
/// Applied as the first sub-step of the destination's next Stage 4 (a later blueprint's
/// tick-driver responsibility — see this blueprint's Stage-1/Stage-10 contract note).
/// Embedded inline (no `Box`) in `RegionMessage` — together with `BorderUpdateKind`,
/// this keeps the whole variant comfortably inside ARCH-D28's 128-byte inline budget,
/// asserted by this blueprint's own acceptance tests.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BorderUpdateEvent {
    /// The neighbor-owned chunk this update targets.
    pub chunk: ChunkKey,
    /// Absolute block position of the update.
    pub pos: BlockPos,
    pub kind: BorderUpdateKind,
}

/// What kind of border-crossing update `BorderUpdateEvent` carries.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BorderUpdateKind {
    /// A block state changed; `new_state` is the raw global block-state numeric id
    /// (vanilla's own ID space). Stored as a raw `u32` rather than a typed
    /// `BlockStateId` because the block-state registry (`rc-registries`, WORLD-D3)
    /// does not exist yet at M0 — `rc-messaging` must not gain a dependency on it.
    BlockChanged { new_state: u32 },
    /// A neighbor-update notification only — no block changed at this position
    /// (e.g. a redstone signal-level recompute trigger, ARCH-D13's neighbor-changed
    /// fan-out).
    NeighborChanged,
}

/// ARCH-D10/D28: a full entity-component snapshot moving to a new owning region.
/// `component_data` is a placeholder (opaque bytes) until `05-game-mechanics.md`'s
/// concrete entity components exist (M4) — see this blueprint's Context section for
/// why that is safe to defer without breaking `RegionMessage`'s outer shape.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntitySnapshot {
    pub entity_id: RcEntityId,
    /// The chunk the entity was in immediately before the transfer, in the source
    /// region — carried for diagnostic/ordering purposes at the destination.
    pub source_chunk: ChunkKey,
    /// Opaque serialized component-bundle bytes. Replaced with concrete typed fields
    /// by the blueprint that first implements real entity-component snapshotting,
    /// without changing `RegionMessage::RegionTransferRequest`'s outer `Box<EntitySnapshot>`
    /// shape (ARCH-D25's extension-point framing, applied here by analogy).
    pub component_data: Vec<u8>,
}

/// The two native cross-region payload variants ARCH-D25 ships. `13-cluster-architecture.md`
/// may add cluster-only variants later without changing this envelope (ARCH-D25's stated
/// extension point) — not done by this blueprint.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RegionMessage {
    BorderUpdateEvent(BorderUpdateEvent),
    /// Boxed so a pooled allocator (`rc-transport-inproc`'s `SegQueue`-backed slot
    /// pool, ARCH-D28) can hand out a reused `Box<EntitySnapshot>` transparently —
    /// see this blueprint's "pooling seam" Context note.
    RegionTransferRequest(Box<EntitySnapshot>),
}
```

### `crates/messaging/src/transport.rs`

```rust
use crate::{Message, RegionId, RegionMessage};

/// One `RegionMessage` delivery failure mode (ARCH-D29's own name and the only variant
/// it pins). Carries the un-delivered message back to the caller — `Transport::send`
/// fully consumes `msg` by value per ARCH-D26's exact signature, so this is the only
/// way a caller can retry the *same* message next tick, mirroring
/// `std::sync::mpsc::SendError<T>`'s "give the value back" convention. See this
/// blueprint's Context section for why this is a deliberate, cited resolution of an
/// explicitly-deferred planning decision, not an invented deviation.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("destination inbox is full; message returned for retry (ARCH-D29 backpressure)")]
    Backpressure(Message<RegionMessage>),
}

/// The one substrate every cross-partition communication goes through, in either
/// deployment mode (ARCH-D26). Exact signature pinned there. Zero implementations of
/// this trait exist in this crate — `InProcessTransport` (`rc-transport-inproc`) and
/// `NetworkTransport` (`rc-transport-net`, cluster feature) are separate crates that
/// depend on this one, never the reverse (`xtask lint-deps` Rule 3).
///
/// Guarantees every implementation must uphold (ARCH-D29, restated in full in this
/// blueprint's Context section): FIFO and exactly-once per `(from, to)` pair for the
/// process's lifetime; no ordering guarantee across different pairs; never blocks the
/// caller (`Backpressure` instead).
pub trait Transport: Send + Sync + 'static {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError>;
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>>;
}
```

### `crates/messaging/src/bus.rs`

```rust
use std::collections::HashMap;

use crate::{Address, Message, RegionId, RegionMessage};

/// Per-invocation outbound send buffer (ARCH-D30: "injected like `Commands`" —
/// restated in `bevy_ecs`-free form; see this blueprint's Context section for why).
/// Whoever hands one of these to a running domain system gives each system its own
/// private instance, so concurrently-running systems never contend over shared
/// mutable state through this type — that integration is `rc-scheduler`'s job, not
/// implemented by this blueprint.
#[derive(Debug, Default)]
pub struct RegionMessageBus {
    pending: Vec<(Address, RegionMessage)>,
}

impl RegionMessageBus {
    pub fn new() -> Self;

    /// Buffer an outbound message. Not visible anywhere else (not in any
    /// `RegionMessageState`, not flushed to `dyn Transport`) until this whole buffer
    /// is passed to `RegionMessageState::merge`.
    pub fn send(&mut self, to: Address, message: RegionMessage);
}

/// The region-owned canonical message state (ARCH-D30): every finished system's
/// `RegionMessageBus` merged in order, plus the current tick's drained inbound queue.
/// One instance per region (ARCH-D5) — placing it there and driving the Stage-1/
/// Stage-10 contract below is `rc-scheduler`'s job, not implemented by this blueprint.
#[derive(Debug, Default)]
pub struct RegionMessageState {
    outbox: Vec<(Address, RegionMessage)>,
    inbox: Vec<RegionMessage>,
    seq_counters: HashMap<Address, u32>,
}

impl RegionMessageState {
    pub fn new() -> Self;

    /// Append one finished system's buffered sends onto the outbox, preserving
    /// emission order (this call's entries appended after everything already
    /// merged this tick). Consumes `bus`.
    pub fn merge(&mut self, bus: RegionMessageBus);

    /// Stage 10: stamp and drain every message merged so far this tick into
    /// ready-to-send envelopes, in emission (merge) order. `from`/`tick_stamp` are
    /// supplied by the caller (the region's own identity and current tick counter —
    /// owned by `rc-scheduler`, not this crate). `seq` is assigned here: a monotonic
    /// counter per distinct `to: Address` value, persisting across ticks (see
    /// Context). Empties the outbox; `seq` counters are **not** reset.
    pub fn drain_outbox(&mut self, from: RegionId, tick_stamp: u64) -> Vec<Message<RegionMessage>>;

    /// Stage 1: install this tick's freshly-drained inbound queue (ARCH-D30),
    /// **replacing** whatever was left from last tick (not appending).
    pub fn set_inbox(&mut self, messages: Vec<RegionMessage>);

    /// Read-only inbound access for any Stage-1..N system.
    pub fn inbox(&self) -> &[RegionMessage];
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus `crates/core/src/{coords.rs, entity_id.rs}` and `crates/messaging/src/{address.rs, envelope.rs, region_message.rs, transport.rs, bus.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (fields/derives/doc comments stay exactly as specified — only executable bodies are stubbed), plus the two `Cargo.toml` edits and the root `Cargo.toml` `proptest` line. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/core/tests/` or `crates/messaging/tests/`, and must not change any type's field list, derive list, or public signature from what the test changeset already compiled against.

### `crates/core/tests/coords.rs`

1. `dimension_id_builtin_constants_are_distinct` — `DimensionId::OVERWORLD != DimensionId::THE_NETHER`, `!= DimensionId::THE_END`, all three pairwise distinct.
2. `chunk_key_equality_and_copy` — `ChunkKey::new(DimensionId::OVERWORLD, 3, -5) == ChunkKey::new(DimensionId::OVERWORLD, 3, -5)`; a `ChunkKey` value is used twice after a `let a = ...; let b = a;` (proves `Copy`, would fail to compile otherwise); changing any one of the three constructor arguments produces an unequal value (three sub-cases: different dimension, different x, different z).
3. `block_pos_chunk_conversion_positive` — `BlockPos::new(48, 70, 5).chunk_x() == 3`, `.chunk_z() == 0`.
4. `block_pos_chunk_conversion_negative` — `BlockPos::new(-3, 70, -17).chunk_x() == -1`, `.chunk_z() == -2` (floor division toward negative infinity, not truncation — `-17 / 16 == -1` by truncation but the correct chunk is `-2`).
5. `block_pos_chunk_key_matches_manual_construction` — `BlockPos::new(48, 70, 5).chunk_key(DimensionId::OVERWORLD) == ChunkKey::new(DimensionId::OVERWORLD, 3, 0)`.
6. `coords_are_hashable` — construct a `std::collections::HashSet<ChunkKey>` and a `HashSet<BlockPos>`, insert several values including one duplicate each, assert final lengths equal the count of *distinct* values inserted (proves `Hash`+`Eq` are consistent, not just present).

### `crates/core/tests/entity_id.rs`

1. `allocator_first_alloc_is_one` — a fresh `RcEntityIdAllocator::new()`'s first `.alloc()` returns `RcEntityId(1)`.
2. `allocator_is_strictly_monotonic` — 1,000 sequential `.alloc()` calls on one instance produce strictly increasing values with no gaps assumed (only strict increase asserted, since gaps are not a claimed guarantee).
3. `allocator_is_thread_safe_and_unique_under_contention` — spawn 8 `std::thread::scope` threads sharing one `&RcEntityIdAllocator`, each calling `.alloc()` 1,000 times and collecting its results; after joining, collect all 8,000 values into one `HashSet`; assert the set's length is exactly 8,000 (zero duplicates across threads).
4. `from_raw_round_trips` — `RcEntityId::from_raw(42).0 == 42`.

### `crates/messaging/tests/address_invariants.rs`

1. `region_id_equality_and_copy` — same pattern as `chunk_key_equality_and_copy` above, for `RegionId`.
2. `address_variants_distinct_even_with_same_inner_value` — `Address::Region(RegionId(7)) != Address::Entity(RcEntityId(7))` and `!= Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 0, 0))` even though none of the three share a common representation (proves the enum discriminant, not just inner-value equality, participates in `PartialEq`).
3. `address_is_hashable` — build a `HashMap<Address, u32>` (exactly `RegionMessageState`'s own `seq_counters` shape), insert one entry per `Address` variant plus a second, distinct `Address::Entity` value, assert `.len() == 4` and each key looks up its own value correctly.
4. `address_is_copy` — `let a = Address::Region(RegionId(1)); let b = a;` then use both `a` and `b` (compile-time proof).

### `crates/messaging/tests/envelope_roundtrip.rs`

Uses `postcard::to_allocvec`/`postcard::from_bytes` (dev-dependency).

1. `border_update_event_round_trips` — construct `Message { from: RegionId(1), to: Address::Region(RegionId(2)), tick_stamp: 42, seq: 7, payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent { chunk: ChunkKey::new(DimensionId::OVERWORLD, 3, -5), pos: BlockPos::new(48, 70, -80), kind: BorderUpdateKind::BlockChanged { new_state: 123 } }) }`; serialize; deserialize back into `Message<RegionMessage>`; assert equal to the original.
2. `border_update_event_neighbor_changed_round_trips` — as above but `kind: BorderUpdateKind::NeighborChanged` (covers the unit-like variant separately from the struct-like one).
3. `region_transfer_request_round_trips` — construct a `Message` whose payload is `RegionMessage::RegionTransferRequest(Box::new(EntitySnapshot { entity_id: RcEntityId::from_raw(99), source_chunk: ChunkKey::new(DimensionId::THE_NETHER, 0, 0), component_data: vec![1, 2, 3, 4, 5] }))`; round-trip; assert equal.
4. `region_message_size_bound` — `assert!(std::mem::size_of::<RegionMessage>() <= 128)` (ARCH-D28's inline-budget regression guard; a plain `size_of` assertion, no serialization involved).
5. `transport_trait_is_object_safe` — a free function `fn _assert_object_safe(_: &dyn Transport) {}` (never called; its mere presence in the compiled test binary is the assertion — this test's "body" is that the crate compiles at all with this function present, so the test itself can simply assert `true` after referencing the function pointer, e.g. `let _: fn(&dyn Transport) = _assert_object_safe;`).

### `crates/messaging/tests/bus_semantics.rs`

1. `bus_send_is_invisible_until_merged` — create a `RegionMessageBus`, call `.send(...)` twice, create a fresh `RegionMessageState`, assert `state.drain_outbox(RegionId(1), 0).is_empty()` (nothing merged yet, so nothing to drain).
2. `merge_preserves_emission_order_within_one_bus` — one `RegionMessageBus`, three `.send()` calls with distinguishable payloads (use `BorderUpdateKind::BlockChanged { new_state: N }` with `N` = 0, 1, 2 as the distinguishing marker), merge into a fresh `RegionMessageState`, `drain_outbox`, assert the three returned messages' `new_state` values appear in order `[0, 1, 2]`.
3. `merge_preserves_order_across_multiple_buses` — two separate `RegionMessageBus` instances, first sends markers `[0, 1]`, second sends marker `[2]`; merge the first, then the second, into one `RegionMessageState`; `drain_outbox`; assert markers appear in order `[0, 1, 2]` (proves merge-order, not just within-bus order).
4. `drain_outbox_empties_and_stays_empty` — after the previous scenario's `drain_outbox` call, call `drain_outbox` again on the same `RegionMessageState`; assert it returns an empty `Vec`.
5. `drain_outbox_stamps_from_and_tick_stamp` — merge one send, `drain_outbox(RegionId(9), 12345)`, assert the single returned message's `.from == RegionId(9)` and `.tick_stamp == 12345`.
6. `seq_is_per_destination_and_monotonic_across_ticks` — one `RegionMessageState`; one `RegionMessageBus` with three `.send()` calls in this exact order: `(Address::Region(RegionId(5)), marker 0)`, `(Address::Region(RegionId(5)), marker 1)`, `(Address::Region(RegionId(6)), marker 2)` (markers via `BorderUpdateKind::BlockChanged { new_state }` as in test 2 above); merge this one bus; `drain_outbox` once — assert the returned messages' `seq` values are `[0, 1, 0]` in the same order (the two `RegionId(5)`-destined messages get `0` then `1`; the `RegionId(6)`-destined message independently gets `0`). Then merge one more bus containing a single send to `Address::Region(RegionId(5))` and `drain_outbox` again — assert its `seq == 2` (the `RegionId(5)` counter persisted across the two `drain_outbox` calls, i.e. across a simulated tick boundary, while a fresh destination would still start at `0`).
7. `set_inbox_replaces_not_appends` — fresh `RegionMessageState`; `set_inbox(vec![A, B])`; assert `.inbox() == [A, B]`; `set_inbox(vec![C])`; assert `.inbox() == [C]` (not `[A, B, C]`).
8. `inbox_is_read_only_and_repeatable` — after one `set_inbox` call, call `.inbox()` three times in a row; assert all three calls return the same slice contents (proves reading does not consume/mutate).

### `crates/messaging/tests/fifo_property.rs`

Defines a test-only `MockTransport` (not a deliverable — lives entirely inside this test file, using only `std::sync::Mutex`/`std::collections::{HashMap, VecDeque}`, no new dependency):

```rust
struct MockTransport {
    inboxes: std::sync::Mutex<std::collections::HashMap<RegionId, std::collections::VecDeque<Message<RegionMessage>>>>,
}

impl MockTransport {
    fn new() -> Self { /* empty map */ }
}

impl Transport for MockTransport {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError> {
        let to = match msg.to { Address::Region(r) => r, _ => panic!("mock only targets Address::Region") };
        self.inboxes.lock().unwrap().entry(to).or_default().push_back(msg);
        Ok(())
    }
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>> {
        self.inboxes.lock().unwrap().get_mut(&into).and_then(|q| q.pop_front())
    }
}
```

`fifo_and_no_loss_no_duplication_per_pair` (a `proptest!` property test): generates an arbitrary `Vec<(u8, u32)>` of length 0..200 where each element's first component is a destination selector in `0..3u8` (mapped to `RegionId(0)`, `RegionId(1)`, `RegionId(2)`) and the second component is simply that element's own 0-based index in the input vector (guaranteeing uniqueness without an extra constraint). For each element, in vector order, construct a `Message` with a fixed synthetic `from: RegionId(999)`, `to: Address::Region(<selected RegionId>)`, `tick_stamp: 0`, `seq: 0` (unused by this test), and `payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent { chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0), pos: BlockPos::new(0, 0, 0), kind: BorderUpdateKind::BlockChanged { new_state: <the index> } })`, and call `transport.send(msg).unwrap()`. After all sends, for each of the 3 destination `RegionId`s, drain via repeated `try_recv` until `None`, collecting each received message's `new_state` marker. Assert: (a) the union of all three destinations' received-marker lists, as a set, equals the set of all indices actually sent (no loss, no duplication — every sent marker appears in exactly one destination's list exactly once); (b) for each destination independently, its received-marker list is exactly the subsequence of the *original send order* restricted to markers that were sent to that destination (FIFO per `(from, to)` pair — cross-destination interleaving is explicitly not checked, matching ARCH-D29's "no ordering guaranteed across different pairs").

## Implementation steps

1. **`rc-core`.** Add the `serde` dependency line to `crates/core/Cargo.toml`. Write `crates/core/src/coords.rs` and `crates/core/src/entity_id.rs` with real bodies matching the Deliverables signatures exactly (field lists and derives were already fixed by the test changeset — do not change them). `chunk_x`/`chunk_z` use `self.x >> 4` / `self.z >> 4` (arithmetic right shift on `i32`, floor semantics). `chunk_key` constructs `ChunkKey::new(dimension, self.chunk_x(), self.chunk_z())`. `RcEntityIdAllocator::new()` is `Self(AtomicU64::new(1))`; `alloc` is `RcEntityId(self.0.fetch_add(1, Ordering::Relaxed))`; `Default::default()` is `Self::new()` (same starting state, not a zero-initialized counter). Update `crates/core/src/lib.rs`'s doc comment if needed (module declarations/re-exports are already correct from the test changeset). Observable: `cargo build -p rc-core` succeeds; `cargo nextest run -p rc-core` — both `coords.rs` and `entity_id.rs` test files pass in full.
2. **Root `Cargo.toml`.** Add the `proptest = "1.11.0"` line to `[workspace.dependencies]`. Observable: `cargo metadata` still succeeds workspace-wide.
3. **`rc-messaging` — address, envelope, region_message, transport.** Add `postcard`/`proptest` dev-dependencies to `crates/messaging/Cargo.toml` (normal deps already correct from M0-B01/test changeset — do not touch them). Write `src/address.rs`, `src/envelope.rs`, `src/region_message.rs`, `src/transport.rs` with real bodies (all are plain data types with derive-generated behavior — `ChunkKey::new`-style constructors are the only hand-written bodies needed; everything else is `#[derive(...)]`). Observable: `cargo build -p rc-messaging` succeeds for everything except `bus.rs` (still `todo!()`-stubbed at this point).
4. **`rc-messaging` — bus.** Write `src/bus.rs`. `RegionMessageBus::new`/`RegionMessageState::new` are each `Self::default()` (both types already derive `Default`). `RegionMessageBus::send` is `self.pending.push((to, message))`. `RegionMessageState::merge` is `self.outbox.extend(bus.pending)` (preserves order: `Vec::extend` appends in iterator order). `drain_outbox` drains `self.outbox` via `std::mem::take(&mut self.outbox)`, then for each `(to, payload)` in the taken vector (in order): look up or insert `0` into `self.seq_counters` for `to`, read the current value as `seq`, increment the stored value by 1, push `Message { from, to, tick_stamp, seq, payload }` onto the result vector. `set_inbox` is `self.inbox = messages`. `inbox` is `&self.inbox`. Observable: `cargo build -p rc-messaging` succeeds with zero `todo!()` remaining.
5. **Run the full acceptance suite.** `cargo nextest run -p rc-core -p rc-messaging` — every test named in the Acceptance tests section passes.
6. **Doctests.** `cargo test --doc -p rc-core -p rc-messaging` passes (no runnable doc examples are required by this blueprint; this only guards against accidentally introducing a broken one).
7. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `cargo run -p xtask -- lint`, `cargo run -p xtask -- lint-deps`, `cargo run -p xtask -- test` — all four still exit 0 (Rule 3's exact normal-dependency set for `rc-messaging` is unchanged; `rc-core` gaining `serde` violates no rule).
8. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs of `.github/workflows/ci.yml` (M0-B01) go green on a clean checkout — the authoritative done-signal (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/core/tests/` and `crates/messaging/tests/` is committed first, alongside `todo!()`-stubbed (but otherwise complete: full field lists, full derive lists, full doc comments) `src/*.rs` files for both crates and the two `Cargo.toml`/root-`Cargo.toml` edits. The implementation changeset (steps 1–8 above) fills in real bodies only — it must not edit any test file, must not add, remove, or rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, the exact expected values in `block_pos_chunk_conversion_negative`, `seq_is_per_destination_and_monotonic_across_ticks`, and every round-trip equality check must survive unchanged).

(b) **No new external dependencies beyond the pinned set, with exactly one named exception.** Every external crate this blueprint's deliverables use is already in the workspace root's `[workspace.dependencies]` table (`serde`, `thiserror`, `postcard`), except `proptest`, which this blueprint itself adds to that table at TEST-D27's exact pinned version (`1.11.0`) — a cited, deliberate addition, not an invented one. Do not add `bevy_ecs`, `crossbeam-channel`, `crossbeam-queue`, `quinn`, `anyhow`, `bincode`, `rkyv`, or any other crate to either `rc-core`'s or `rc-messaging`'s `Cargo.toml` under any circumstance — `rc-messaging`'s normal-dependency set staying exactly `{rc-core, serde, thiserror}` is a CI-enforced hard rule (`xtask lint-deps` Rule 3, M0-B01), not a style preference.

(c) **No Mojang or third-party reimplementation code.** Nothing in this blueprint touches protocol wire format, decompiled game logic, or any other project's source — every type and algorithm here is derived solely from `docs/planning/01-server-architecture.md` and this blueprint's own concrete, cited resolutions of what that document left open (ASSET-D18/D19/D30).

(d) **No `unsafe` code.** Every type and function in this blueprint's deliverables is implementable in 100% safe Rust (the one atomic primitive used, `std::sync::atomic::AtomicU64`, is a safe-to-use `std` type — no raw pointers, no `unsafe impl`, no FFI).

(e) **Scope boundary — do not implement beyond this blueprint's two crates.** This blueprint does not implement `InProcessTransport` or the `SegQueue`-backed slot pool (`rc-transport-inproc`, ARCH-D27/D28 — a separate blueprint); does not implement the ARCH-D24 `ChunkKey -> RegionId`/`RcEntityId -> RegionId` directories or `RegionId` allocation (ARCH-D6, `rc-scheduler` — a separate blueprint); does not implement the 11-stage tick pipeline, RC-Executor, or the Stage-1/Stage-10 driver that calls into this blueprint's `RegionMessageState`/`Transport` API (ARCH-D1–D9/D12/D18–D23, `rc-scheduler` — a separate M0 blueprint, per M0-B01's own Constraint (d)); does not implement real entity-component snapshotting inside `EntitySnapshot` (`05-game-mechanics.md`, M4). Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item above stays exactly as unimplemented as this blueprint's Deliverables show it.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-core -p rc-messaging --all-features
cargo nextest run -p rc-core -p rc-messaging
cargo test --doc -p rc-core -p rc-messaging
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-core -p rc-messaging` runs all 6 (`coords.rs`) + 4 (`entity_id.rs`) + 4 (`address_invariants.rs`) + 5 (`envelope_roundtrip.rs`) + 8 (`bus_semantics.rs`) + 1 (`fifo_property.rs`) = 28 test cases named in Acceptance tests (the property test counts as one `#[test]`-level case regardless of how many proptest-generated inputs it runs internally) — all pass. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
