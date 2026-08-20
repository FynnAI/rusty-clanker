# M0-B05 — RC-Executor: Conflict Graph & 11-Stage Tick Pipeline

| Field | Content |
|---|---|
| ID | M0-B05 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | M0-B02 (`rc-messaging`'s complete, real `Transport` trait, `TransportError`, `Message<T>`, `Address`, `RegionId`, `RegionMessage`, `RegionMessageBus`, `RegionMessageState` — this blueprint calls every one of these exactly as M0-B02 fixed them, never modifying `rc-messaging`); M0-B04 (RC-WorkerPool: `rc-scheduler::pool::RcWorkerPool`, already merged into `crates/scheduler/` with `crossbeam-deque`/`crossbeam-utils`/`parking_lot` already added to `crates/scheduler/Cargo.toml` — this blueprint depends only on the dispatch-facing subset of that type restated in full in Context below, and adds nothing to `pool.rs` itself) |
| Implements | ARCH-D1–D4 (`bevy_ecs` 0.19.1 standalone configuration, dynamic component registration primitive — context only, not implemented here); ARCH-D8 (five domain groups, fixed inter-group order, `Access<ComponentId>` conflict graph, Kahn's-algorithm topological layering with declaration-index tie-break); ARCH-D9 (the two command-buffer sync points and the Stage-4 inline-mutation exception); ARCH-D12 (the full 11-stage tick pipeline); ARCH-D13 (Stage 4's mandatory sequential collapse); ARCH-D14 (Stage 5's per-chunk-seeded-RNG design fact, documented but not exercised — no random-tick content exists at M0) |
| Crates touched | `rc-scheduler` (`crates/scheduler/`) only |
| Estimated scope | L |

## Goal & Done definition

Implement RC-Executor inside `rc-scheduler`: a system-registration API that captures each system's `bevy_ecs::query::Access<ComponentId>` and `DomainGroup`; a pure, directly-testable conflict-graph algorithm (`compute_waves`) that partitions each domain group's fixed system list into sequential "waves" of pairwise-concurrency-safe systems via Kahn's-algorithm topological layering; a per-region tick driver (`RcExecutor::tick_region`) that advances one `bevy_ecs::World` through the fixed 11-stage pipeline once, dispatching each domain group's waves onto M0-B04's `RcWorkerPool`, applying ARCH-D9's two command-buffer sync points (Stage 1, Stage 10) with Stage 4's inline exception, and fulfilling M0-B02's exact Stage-1/Stage-10 `RegionMessageBus`/`Transport` driver contract. Every stage with no mechanics content yet (2, 3, 5, 7, and the bodies of 4/6/8/9/11 when nothing is registered into them) is a correct, tested no-op — the pipeline's *shape* and *ordering/concurrency guarantees* are what this blueprint proves, exercised entirely with synthetic test systems, since no real mechanics content (`rc-mechanics`) exists until `M3`/`M4`.

**Out of scope, explicitly** (see Constraints for the full list): ARCH-D5/D6 region build/merge/split and the ARCH-D24 `ChunkKey -> RegionId`/`RcEntityId -> RegionId` directories (a separate, not-yet-written `rc-scheduler` blueprint); ARCH-D10/D11's actual application of inbound `RegionMessage` payloads (needs `rc-mechanics`, `M3`/`M4`); ARCH-D15's Stage-6a/6b sub-phase split and reconciliation pass, ARCH-D16's Stage-8 chunk-parallel specifics, ARCH-D17's Stage-7 block-entity ordering (all need real entity/lighting/block-entity components that do not exist at `M0`); ARCH-D18–D23 (RC-WorkerPool's own elastic sizing, EDF admission, the Tokio boundary — M0-B04's and a later real-time-loop blueprint's jobs); the real-time, wall-clock-paced multi-region 20 TPS soak loop (M0's acceptance criterion 1 — a later blueprint composes this blueprint's `tick_region` into that loop).

Done when:

- [ ] `cargo build -p rc-scheduler --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `bevy_ecs` and `thiserror` (both already in `[workspace.dependencies]`) are the only new entries this blueprint adds to `rc-scheduler`'s normal-dependency set, and `rc-scheduler` is in neither `NETRENDER` nor gains an edge into it (Rule 2 unaffected).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler` exits 0.
- [ ] The `same_final_state_across_worker_counts` and `same_emitted_message_sequence_across_worker_counts` determinism tests pass identically whether `RcWorkerPool` is constructed with 1, 2, or 8 threads — no wall-clock sleep, no flakiness (every synchronization point in every test is an explicit barrier/atomic, never a timing assumption).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### ECS foundation this blueprint builds on (ARCH-D1–D4), restated

`bevy_ecs` 0.19.1, pulled `default-features = false, features = ["std"]` (ARCH-D1/D2) — already the workspace-pinned version (`12-workspace-structure.md`'s Workspace Dependency Versions table), not altered here. Rusty Clanker never uses `bevy_ecs`'s own `Schedule`/`MultiThreaded` executor or `bevy_tasks::ComputeTaskPool` (ARCH-D3): this blueprint's own `RcExecutor` is the entire execution strategy, built on M0-B04's `RcWorkerPool`. `World::register_component_with_descriptor` (ARCH-D4) is the dynamic-registration primitive isomorphic mods will use — this blueprint does not call it (no mod loader exists at `M0`) but the conflict-graph algorithm below is written so that a future mod-registered `ComponentId` participates identically to a native one (nothing in `compute_waves` or `ComponentAccessSummary` distinguishes how a `ComponentId` was minted).

### The five domain groups and their stage mapping (ARCH-D8), this blueprint's concrete resolution

ARCH-D8 names five fixed domain groups, run in this fixed inter-group order every tick: **Block/Redstone → Entity AI+Physics → Lighting → Chunk Serialization → Network Encode/Decode**. This blueprint's concrete, cited mapping onto the 11-stage table (below) — the planning corpus names the groups and the stages separately but does not spell out the mapping table itself, so this is this blueprint's own resolution, chosen because it is the only mapping consistent with both the groups' stated fixed order and the stages' fixed numeric order:

| `DomainGroup` | Maps to `Stage` | Dispatch style |
|---|---|---|
| `BlockRedstone` | 4 (Scheduled Block Tick) | Sequential, inline command-apply (ARCH-D13, ARCH-D9 exception) |
| `AiPhysics` | 6 (Entity AI+Physics) | Conflict-graph-batched, deferred |
| `Lighting` | 8 (Lighting) | Conflict-graph-batched, deferred |
| `ChunkSerialize` | 9 (Chunk Snapshot) | Conflict-graph-batched, deferred |
| `NetCodec` | 11 (Network Outbound Encode) | Conflict-graph-batched, **read-only** (no deferred-command support — see Deliverables) |

Stages 1, 2, 3, 5, 7, 10 accept **no** domain-group registration in this blueprint — they are executor-internal structural stages (1 and 10 are the two sync points; 2, 3, 5, 7 are content-less no-ops at `M0` since no mechanics exist). A later mechanics blueprint that needs Stage 5 (random tick) or Stage 7 (block entities) to accept registered systems extends `DomainGroup`/the stage-mapping table above; this blueprint does not pre-guess that extension.

`SystemHandle`, restated verbatim from `01-server-architecture.md`'s Domain Conflict Model section (this blueprint's `registry.rs`/`executor.rs` implement exactly this shape, split across a builder-time and an instantiated-per-region form — see Deliverables):

```rust
struct SystemHandle {
    system: Box<dyn bevy_ecs::system::System<In = (), Out = ()>>,
    access: bevy_ecs::query::Access<bevy_ecs::component::ComponentId>, // from system.component_access()
    group: DomainGroup,       // Block/Redstone | AiPhysics | Lighting | ChunkSerialize | NetCodec
    order_tag: u32,           // declaration index; deterministic tie-break
}
```

Two systems in the **same** group may run concurrently iff their access sets are disjoint or read/read (checked once at startup, hard boot-time error on a startup-detected structural-authority conflict — see "Structural-write validation" below; there is no runtime fallback). Systems in **different** groups never run concurrently against each other — inter-group order is the fixed sequence above, enforced by treating each group as a full-drain barrier. Within Stage 4, the group's system list collapses to a single worker **regardless of declared access** — sequencing there is a correctness requirement (ARCH-D13), not a conflict-avoidance one.

### The conflict-graph algorithm (ARCH-D8: Kahn's-algorithm topological sort, declaration-index tie-break)

`ComponentAccessSummary` normalizes a system's declared access into a form independent of `bevy_ecs::query::Access`'s own exact API surface (see "bevy_ecs 0.19.1 API points to verify" below):

```
reads: HashSet<ComponentId>
writes: HashSet<ComponentId>
reads_all: bool   // true for an unrestricted dynamic query (e.g. FilteredEntityRef, ARCH-D4)
writes_all: bool  // true for an unrestricted dynamic mutable query (e.g. FilteredEntityMut, ARCH-D4)
```

Two summaries are **compatible** (may run concurrently) iff: neither declares `writes_all`; if one declares `reads_all`, the other must declare neither `writes_all` nor any concrete `writes`; and `writes` is pairwise disjoint from the other's `writes` and `reads`. (Read/read — including `reads_all`/`reads_all` — is never a conflict.) A system declaring `writes_all`/`reads_all` is, by this rule, incompatible with every other system in its group and is therefore always placed alone in its own wave by the algorithm below — a proven consequence of the compatibility rule, not a special case the algorithm needs to implement separately.

`compute_waves` (the pure, directly-testable core — full signature in Deliverables): given one domain group's fixed, declaration-ordered system list (index = declaration order = `order_tag`), build a directed graph where an edge `i -> j` (`i < j`) exists exactly when systems `i` and `j` are **incompatible**. Because every edge always points from a lower to a higher index, this graph is acyclic by construction — no cycle-detection step is needed. Run Kahn's algorithm: repeatedly collect every not-yet-processed node with in-degree 0 into one "wave" (this set is pairwise compatible by construction — see the proof note in Deliverables), mark them processed, decrement their successors' in-degree, and repeat until every node is processed. Waves are returned in execution order; a wave's members are conflict-free and dispatched together; wave `k+1` never starts until every member of wave `k` has finished. Within one wave, members are submitted to `RcWorkerPool` in ascending `order_tag` (the "declaration-index tie-break" ARCH-D8 names) for deterministic submission order — this does not affect *correctness* (the wave is conflict-free by definition) but keeps dispatch reproducible for debugging.

### The two sync points and the Stage-4 exception (ARCH-D9), this blueprint's exact scope

ARCH-D9's exact text: structural `World` mutations go through `CommandQueue`-equivalent buffers during stages 3–9 and are applied only at two sync points — **pre-tick (Stage 1**, applies previous tick's buffers + inbound cross-region transfers**)** and **post-tick (Stage 10**, applies this tick's buffers in `(stage index, emission order, chunk/entity id)` order, populating destination regions' Stage-1 inboxes**)**. **Exception:** Stage 4 applies its own mutations immediately, inline, block-by-block — never deferred through this mechanism, because ARCH-D13's mandatory single-worker sequential execution for that stage already gives same-tick visibility for free.

This blueprint's concrete, resolved reading, restated so nothing is left to interpretation:

- **Stage 1's "previous tick's buffers"** are exactly the inbound `RegionMessage` payloads drained from `dyn Transport` this same call (a message sent by another region's own prior Stage 10 *is* "a previous tick's buffer," delivered via the messaging substrate rather than re-applying this region's own already-flushed Stage-10 commands a second time — Stage 10 already applied those once, within the tick that produced them). This blueprint implements the generic drain-into-inbox mechanism (M0-B02's own exact Stage-1 contract, below) but **not** the actual application of a transfer/border-update payload as a structural `World` mutation (ARCH-D10/D11) — that needs `rc-mechanics` components that do not exist before `M3`/`M4`. The drained inbox is exposed read-only exactly as ARCH-D30 specifies, ready for a future Stage-1..N system to consume; consuming it is out of this blueprint's scope.
- **Stage 10's "(stage, emission order)"** is this blueprint's full scope (the "chunk/entity id" tertiary tie-break needs mechanics-specific typed commands able to compare positions/ids, which do not exist at `M0` — achieving that finer order **within** one system's own command stream is that future system's own responsibility, out of scope here, exactly as `bevy_ecs`'s own `Commands` already preserves whatever order commands were pushed in). This blueprint's total order is: primary key = originating stage number (only Stage 6, 8, 9 contribute at `M0`, since Stage 4 is excepted and Stages 2/3/5/7 register nothing), secondary key = `order_tag` ascending. Every system that ran this tick (in any wave, on any worker thread) has its own deferred command state applied, **at Stage 10, single-threaded**, in that exact order — regardless of which worker thread actually ran it or in what wall-clock order it finished (this is exactly what makes the emitted `World` state independent of `RcWorkerPool`'s worker count — see "Determinism guarantee" below).
- **Stage 4's exception** is implemented at system granularity: each Stage-4 system's own deferred-command state is applied immediately after that one system finishes, before the next Stage-4 system starts — never batched with any other system's commands, never carried to Stage 10.
- **Stage 11 is read-only** (per the pipeline table): this blueprint's Stage-11 dispatch never applies, or even inspects, any Stage-11 system's deferred-command state. A Stage-11 system that misuses `Commands` anyway has its accumulated state silently retained and never flushed for the lifetime of that region (a documented limitation, not a silent correctness bug — see Constraints).

### Structural-write validation (ARCH-D8's Domain Conflict Model), this blueprint's concrete resolution

The planning corpus states RC-Executor "validates at startup that no system both declares direct mutable `Query` access to a component *and* defers structural changes to that same component through a command in the same tick region." `Access<ComponentId>` alone cannot answer "does this system's `Commands` usage structurally touch component X" — a `Commands`-taking system's structural writes are not visible in its static `Access` set at all (that is the whole point of deferring them). This blueprint's resolution: system registration takes an explicit `structural_writes: Vec<ComponentId>` list (supplied by whoever registers the system, describing which components its own `Commands` usage may spawn/despawn/add/remove) alongside its `access`. `RcExecutorBuilder::build` rejects, at build time (`ExecutorBuildError`, never a panic, never a runtime fallback), any system whose `access.writes` intersects its own `structural_writes` — a single component must have exactly one mutation authority per system (live `Query<&mut T>` **or** deferred `Commands`, never both), matching the planning text's own framing exactly. This check is purely per-system (self-consistency); it does not affect `compute_waves`'s cross-system compatibility check, which only ever reads `access`.

### `ComponentId` consistency across regions — an invariant this blueprint must uphold

`bevy_ecs` assigns `ComponentId` values **per `World` instance**, in registration order. RC-Executor's conflict graph is computed **once** (ARCH-D8: "once at startup... reused for every tick of every region") against `ComponentId` values obtained from one throwaway "prototype" `World`, then reused unchanged for every region's own, separate `World`. This is only sound if every region's `World` registers the exact same components in the exact same order as the prototype did. This blueprint enforces that by construction: `RcExecutorBuilder::new` takes a `component_bootstrap: fn(&mut World)` function pointer, called exactly once against the prototype `World` (to compute the graph) and once again, identically, against every region's `World` at `RcExecutor::spawn_region` time, **before** any system is initialized against that region's `World`. At `M0` this function is empty (no statically-known components exist yet beyond whatever a registered system's own `#[derive(Component)]` types cause `bevy_ecs` to auto-register on first `initialize` — which happens in the same fixed system-declaration order in every `World` for the same reason). A future blueprint that adds real, explicitly-pre-registered component types extends this one function; this blueprint does not.

### Why message-sending is not modeled as a `bevy_ecs` `SystemParam` in this blueprint

M0-B02 deliberately built `RegionMessageBus`/`RegionMessageState` as plain, `bevy_ecs`-free types (`rc-messaging` cannot depend on `bevy_ecs`, WS-D3 Rule 3) and explicitly left "how a running domain system obtains a private bus instance" to this blueprint. ARCH-D30's two *native* consumers of that integration — ARCH-D10's transfer application and ARCH-D11's border-tick injection — are both mechanics content that does not exist before `M3`/`M4`; **no system this blueprint tests needs to send a `RegionMessage` from inside a `bevy_ecs::System`**. This blueprint's Stage-1/Stage-10 driver contract (below) is therefore implemented and fully tested against `RegionMessageState`'s own public API (`set_inbox`, `merge`, `drain_outbox` — all callable directly, not only from inside a running system), without needing to resolve the genuinely open question of exactly which `bevy_ecs` 0.19.1 mechanism (a custom `SystemParam`, `Local<T>`, or something else) should hand a private buffer to a real domain system — that resolution is deferred to whichever future blueprint first implements a system that actually calls `RegionMessageBus::send`.

### M0-B02's Stage-1/Stage-10 contract — restated verbatim, this blueprint is the driver

> **Stage-1 contract.** Before any Stage-1..N system for a region runs, the driver calls `Transport::try_recv(region_id)` repeatedly until it returns `None`, collecting every returned message's `.payload` in return order, then calls `RegionMessageState::set_inbox` **exactly once** with the full collected batch. No system calls `try_recv` directly.
>
> **Stage-10 contract.** After every system in the tick has run and every `RegionMessageBus` it produced has been `merge`d into the region's `RegionMessageState` (in merge order), the driver calls `RegionMessageState::drain_outbox(this_region_id, this_tick_counter)` exactly once, then calls `Transport::send` once per returned `Message`, in the order returned.

This blueprint's `RcExecutor::tick_region` **is** that driver. At `M0`, no system merges a `RegionMessageBus` into `region.message_state` during the tick itself (see previous subsection) — `drain_outbox` therefore returns an empty `Vec` unless a **test** calls `region.message_state.merge(...)` directly before ticking, which the acceptance tests below do, to prove the Stage-10 half of the contract independent of the open `SystemParam` question. `TransportError::Backpressure` on a Stage-10 `send` call is dropped (logged in a future blueprint, not this one — ARCH-D29's own retry-policy Open Question is unresolved in the planning corpus and not decided here).

### Assumed `RcWorkerPool` dispatch contract (M0-B04), restated

`rc-scheduler::pool::RcWorkerPool` (M0-B04, already merged) is assumed to provide at least this subset — the only part `executor.rs` calls:

```rust
pub struct RcWorkerPool { /* M0-B04's own fields — opaque to this blueprint */ }

impl RcWorkerPool {
    /// Constructs a pool with exactly `num_threads` worker threads (M0-B04's own
    /// constructor signature may offer additional configuration; this is the minimal
    /// form this blueprint's tests need to force a specific worker count).
    pub fn new(num_threads: usize) -> Self;

    /// Runs every task in `tasks` to completion, distributing them across worker
    /// threads (ARCH-D18's `Injector`/`Worker`/`Stealer` machinery, M0-B04's own
    /// implementation detail). **Blocks** the calling thread until every task has
    /// finished — a *scoped* dispatch, not a fire-and-forget queue: tasks may borrow
    /// data with any lifetime `'a` that outlives this call, never `'static` only.
    /// If any task panics, exactly one panic is propagated to the caller after every
    /// task has finished running (join-like semantics, matching
    /// `std::thread::scope`'s own panic behavior).
    pub fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>);
}
```

**Hard requirement on this contract:** `run_batch` must accept non-`'static` task closures and block until completion. `executor.rs`'s dispatch borrows `&mut region.world` for the duration of one wave; a `'static`-only or non-blocking submission API is unsound for that use and would need a thin scoped-adapter at the call sites in `executor.rs` — the conflict-graph and pipeline-ordering algorithms elsewhere in this blueprint are unaffected either way. If M0-B04's actual shipped signature differs, update only the two call sites in `executor.rs` marked `// pool dispatch` below.

### `bevy_ecs` 0.19.1 API points to verify at implementation time

`01-server-architecture.md`'s own Open Questions flag this exactly: "Exact public-API method name(s) on `bevy_ecs::system::System` for extracting a system's `Access<ComponentId>`... must be re-verified against the pinned 0.19.1 API surface once implementation starts." This blueprint's algorithms and types do not depend on the *exact* names below being right — only on functionality of this *shape* existing, which has been stable across many `bevy_ecs` releases. Before writing `access.rs`'s `ComponentAccessSummary::from_bevy_access` and `executor.rs`'s dispatch bodies, confirm against the actually-installed `bevy_ecs` 0.19.1 docs (`cargo doc --open -p bevy_ecs` or docs.rs once resolved):

1. How to obtain a system's `Access<ComponentId>` after `.initialize(&mut World)` — ARCH-D8's own pseudocode names `system.component_access()`; use that name unless the installed docs show otherwise.
2. `Access<ComponentId>`'s own accessor methods for enumerating declared reads/writes and any "read all"/"write all" flag (used only inside `from_bevy_access`; the rest of this blueprint operates on `ComponentAccessSummary`, not `Access` directly).
3. `World::as_unsafe_world_cell(&mut self) -> UnsafeWorldCell` (or equivalent) and `System`'s unsafe, `UnsafeWorldCell`-taking run method (historically `run_unsafe`) plus `update_archetype_component_access` (called once per system, once per tick, before that system's first `run`/`run_unsafe` call each tick — refreshes cached archetype-access state against any structural changes since the system's last run).
4. The safe `System::run(&mut self, In, world: &mut World)` method (used for every wave of size 1, including all of Stage 4) and the method that applies a system's own accumulated deferred `Commands` state to a `&mut World` (historically `apply_deferred`).
5. `System::initialize(&mut self, world: &mut World)` (called once per system instance, at `RcExecutor::spawn_region` time).

None of these five points changes this blueprint's Deliverables' *signatures* — only their bodies' exact method-call spelling.

### Determinism guarantee, restated as four testable properties

1. **Concurrency safety.** Two systems in the same domain group whose `ComponentAccessSummary`s are incompatible never execute in overlapping wall-clock windows within the same region-tick.
2. **Deterministic apply order.** The sequence of structural mutations actually applied at a sync point (Stage 4 inline, or Stage 10 batched) depends only on the fixed `(stage, order_tag)` key — never on which worker thread ran which system or the wall-clock order tasks completed. Consequently, the region's final `World` state and the exact sequence of `Message<RegionMessage>` values a tick emits are identical regardless of `RcWorkerPool`'s worker count.
3. **Stage ordering.** Every system in stage `K` (and, for `K` ∈ {4, 6, 8, 9}, that stage's own sync/flush behavior) fully completes before any system in stage `K+1` begins.
4. **Stage-4 sequential + inline.** Stage-4 systems execute one at a time, in ascending `order_tag`, and each system's own structural mutations are visible to the very next Stage-4 system before it starts — never deferred to Stage 10.

## Deliverables

### `crates/scheduler/Cargo.toml` (modify — add `bevy_ecs`, `thiserror`; `rc-core`/`rc-messaging`/`rc-mod-host` and M0-B04's `crossbeam-deque`/`crossbeam-utils`/`parking_lot` already present, unchanged)

```toml
[package]
name = "rc-scheduler"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-messaging = { path = "../messaging" }
rc-mod-host = { path = "../mod-host" }
bevy_ecs = { workspace = true }
thiserror = { workspace = true }
crossbeam-deque = { workspace = true }
crossbeam-utils = { workspace = true }
parking_lot = { workspace = true }
```

(The last three lines are M0-B04's own additions, reproduced here only so this file's full expected content is unambiguous — this blueprint does not modify anything about them.)

### `crates/scheduler/src/lib.rs` (modify — add new module declarations/re-exports; `pub mod pool;` is M0-B04's, untouched)

```rust
//! `rc-scheduler` — RC-Executor, RC-WorkerPool, the 11-stage tick pipeline driver,
//! region lifecycle, the ARCH-D8 startup conflict graph, the Tokio<->RC-WorkerPool
//! boundary types (ARCH-D1-D9, D12, D18-D23). Depends on `dyn Transport` only, never
//! a concrete transport (`rc-messaging`'s `Transport` trait).

pub mod pool; // M0-B04 — not modified by this blueprint

mod access;
mod conflict_graph;
mod pipeline;
mod region;
mod registry;
mod executor;

pub use access::ComponentAccessSummary;
pub use conflict_graph::compute_waves;
pub use pipeline::{DomainGroup, Stage};
pub use region::RegionState;
pub use registry::{ExecutorBuildError, SystemFactory, SystemId, RcExecutorBuilder};
pub use executor::{RcExecutor, TickReport};
```

### `crates/scheduler/src/pipeline.rs`

```rust
/// The fixed 11-stage tick pipeline (ARCH-D12), identical for every region, every
/// 50ms tick. Numeric values match the pipeline table 1:1 so `Stage as u8` sorts in
/// pipeline order (used by Stage 10's `(stage, order_tag)` apply-order key).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Stage {
    PreTickSync = 1,
    WorldUpdate = 2,
    NetworkInboundApply = 3,
    ScheduledBlockTick = 4,
    RandomBlockTick = 5,
    EntityAiPhysics = 6,
    BlockEntityTick = 7,
    Lighting = 8,
    ChunkSnapshot = 9,
    PostTickFlush = 10,
    NetworkOutboundEncode = 11,
}

/// The five ARCH-D8 domain groups. `stage()` is this blueprint's own concrete,
/// cited stage mapping (Context: "The five domain groups and their stage mapping").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainGroup {
    BlockRedstone,
    AiPhysics,
    Lighting,
    ChunkSerialize,
    NetCodec,
}

impl DomainGroup {
    pub const ALL: [DomainGroup; 5] = [
        DomainGroup::BlockRedstone,
        DomainGroup::AiPhysics,
        DomainGroup::Lighting,
        DomainGroup::ChunkSerialize,
        DomainGroup::NetCodec,
    ];

    pub const fn stage(self) -> Stage;
    /// 0-based index into `RcExecutor`'s internal 5-element group array; stable,
    /// matches `Self::ALL`'s declaration order.
    pub const fn index(self) -> usize;
}
```

### `crates/scheduler/src/access.rs`

```rust
use std::collections::HashSet;
use bevy_ecs::component::ComponentId;

/// A normalized summary of one system's declared component access (Context:
/// "The conflict-graph algorithm"). Decoupled from `bevy_ecs::query::Access`'s own
/// API surface so `compute_waves` never depends on its exact 0.19.1 shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentAccessSummary {
    pub reads: HashSet<ComponentId>,
    pub writes: HashSet<ComponentId>,
    pub reads_all: bool,
    pub writes_all: bool,
}

impl ComponentAccessSummary {
    /// Construct directly from a fixed read/write set — the primary constructor this
    /// blueprint's own tests use (synthetic `ComponentId` values, no real `World`
    /// needed). `reads_all`/`writes_all` default to `false`.
    pub fn new(reads: impl IntoIterator<Item = ComponentId>, writes: impl IntoIterator<Item = ComponentId>) -> Self;

    /// `reads`/`writes` empty, `reads_all`/`writes_all` as given — for systems using
    /// an unrestricted dynamic query (ARCH-D4's `FilteredEntityRef`/`FilteredEntityMut`).
    pub fn wildcard(reads_all: bool, writes_all: bool) -> Self;

    /// Extracted from a real, `.initialize`d `bevy_ecs::system::System`'s
    /// `component_access()` (or the installed 0.19.1 crate's equivalent — Context's
    /// "bevy_ecs 0.19.1 API points to verify", point 1/2). Not exercised by this
    /// blueprint's pure `compute_waves` tests, which construct `ComponentAccessSummary`
    /// directly via `new`/`wildcard`; exercised only by the integration tests that run
    /// real systems.
    pub fn from_bevy_access(access: &bevy_ecs::query::Access<ComponentId>) -> Self;

    /// True iff `self` and `other` may run concurrently (Context's compatibility rule).
    pub fn is_compatible(&self, other: &Self) -> bool;
}
```

### `crates/scheduler/src/conflict_graph.rs`

```rust
use crate::access::ComponentAccessSummary;

/// Pure conflict-graph construction + Kahn's-algorithm topological layering
/// (ARCH-D8). `systems[i]` is the `i`-th declared system in one domain group
/// (`i` == that system's `order_tag`). Returns waves in execution order; within a
/// wave, indices are ascending (submission-order tie-break). Every index in
/// `0..systems.len()` appears in exactly one wave. Two systems whose summaries are
/// incompatible (`ComponentAccessSummary::is_compatible` returns `false`) are
/// **guaranteed** to land in different waves, with the earlier-declared one's wave
/// strictly preceding the later-declared one's — this is `compute_waves`'s central
/// correctness property, proven directly by `compute_waves_conflict_graph.rs`'s
/// acceptance tests below.
pub fn compute_waves(systems: &[ComponentAccessSummary]) -> Vec<Vec<usize>>;
```

### `crates/scheduler/src/registry.rs`

```rust
use bevy_ecs::component::ComponentId;
use bevy_ecs::system::System;
use crate::pipeline::DomainGroup;

/// Constructs one fresh, `.initialize`-ready system instance. Called once per
/// region at `RcExecutor::spawn_region` time (Context: "`ComponentId` consistency
/// across regions" — never shared across regions).
pub type SystemFactory = Box<dyn Fn() -> Box<dyn System<In = (), Out = ()>> + Send + Sync>;

/// Identifies one registered system by its group and declaration index.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SystemId {
    pub group: DomainGroup,
    pub order_tag: u32,
}

/// Accumulates system registrations, then computes the ARCH-D8 conflict graph once.
pub struct RcExecutorBuilder {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [Vec<Registration>; 5],
}

struct Registration {
    factory: SystemFactory,
    structural_writes: Vec<ComponentId>,
}

impl RcExecutorBuilder {
    /// `bootstrap` is called once against the internal prototype `World` used to
    /// compute the conflict graph, and once again, identically, against every
    /// region's own `World` at `spawn_region` time (Context: "`ComponentId`
    /// consistency across regions").
    pub fn new(bootstrap: fn(&mut bevy_ecs::world::World)) -> Self;

    /// Registers one system into `group`. `order_tag` is assigned automatically as
    /// this call's 0-based index within `group` (declaration order, ARCH-D8).
    /// `structural_writes` lists the components this system's own `Commands` usage
    /// may structurally mutate (Context: "Structural-write validation") — pass an
    /// empty `Vec` for a system that never uses `Commands`.
    pub fn register_system(&mut self, group: DomainGroup, factory: SystemFactory, structural_writes: Vec<ComponentId>) -> SystemId;

    /// Instantiates one prototype system per registration against a throwaway
    /// `World` (after calling `bootstrap` on it), extracts each
    /// `ComponentAccessSummary`, validates the structural-write rule (Context), runs
    /// `compute_waves` once per group, and returns the built, immutable `RcExecutor`.
    /// Returns `Err` on the first structural-write violation found (deterministic
    /// order: groups in `DomainGroup::ALL` order, then ascending `order_tag`).
    pub fn build(self) -> Result<crate::executor::RcExecutor, ExecutorBuildError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorBuildError {
    #[error("system {system:?} declares mutable Query access to component {component:?} that is also listed in its own structural_writes — a component must have exactly one mutation authority per system, never both (ARCH-D8's Domain Conflict Model)")]
    AmbiguousMutationAuthority { system: SystemId, component: ComponentId },
}
```

### `crates/scheduler/src/region.rs`

```rust
use bevy_ecs::world::World;
use bevy_ecs::system::System;
use rc_messaging::{RegionId, RegionMessageState};

/// One region's `bevy_ecs::World` plus its per-system instances (Context:
/// "`ComponentId` consistency across regions" — never shared with any other
/// region) and its `rc-messaging` state. Constructed only via
/// `RcExecutor::spawn_region`. ARCH-D5/D6's region *lifecycle* (build/merge/split,
/// chunk ownership) is a separate, not-yet-written blueprint's job — this type is
/// deliberately minimal: one fixed `World` for the lifetime of the value.
pub struct RegionState {
    pub id: RegionId,
    pub world: World,
    pub tick_counter: u64,
    pub message_state: RegionMessageState,
    pub(crate) system_instances: [Vec<Box<dyn System<In = (), Out = ()>>>; 5],
}
```

### `crates/scheduler/src/executor.rs`

```rust
use rc_messaging::Transport;
use crate::pipeline::{DomainGroup, Stage};
use crate::pool::RcWorkerPool;
use crate::region::RegionState;
use crate::access::ComponentAccessSummary;
use crate::registry::SystemFactory;
use bevy_ecs::component::ComponentId;

struct CompiledSystem {
    factory: SystemFactory,
    access: ComponentAccessSummary,
    structural_writes: std::collections::HashSet<ComponentId>,
}

struct CompiledGroup {
    systems: Vec<CompiledSystem>,     // index == order_tag
    waves: Vec<Vec<usize>>,           // from compute_waves; ignored by Stage 4's dispatch
}

/// The built, immutable RC-Executor (ARCH-D8: conflict graph computed once,
/// "reused for every tick of every region"). `Send + Sync` — safe to share
/// (`&RcExecutor`) across multiple regions' ticks running concurrently on
/// different threads, a later blueprint's use case, not exercised here.
pub struct RcExecutor {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [CompiledGroup; 5],
}

/// Minimal per-tick result. Extended by later blueprints as needed (e.g. per-stage
/// timing for ARCH-D19's hotness EWMA) — not this blueprint's scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickReport {
    pub tick_counter: u64,
}

impl RcExecutor {
    /// Creates a fresh region: a new `World` (bootstrapped identically to the
    /// prototype `World` used at build time), one freshly-`.initialize`d instance
    /// of every registered system, zeroed tick counter, empty `RegionMessageState`.
    pub fn spawn_region(&self, id: rc_messaging::RegionId) -> RegionState;

    /// Advances `region` through the fixed 11-stage pipeline exactly once
    /// (ARCH-D12), dispatching each domain group's waves onto `pool`, applying the
    /// two ARCH-D9 sync points with Stage 4's inline exception, and fulfilling
    /// M0-B02's exact Stage-1/Stage-10 driver contract against `transport`.
    /// Synchronous — this is the "synchronous test-mode tick driver" shape
    /// `09-testing-quality.md`'s TEST-D14 describes, bypassing real-time EDF
    /// admission entirely; a later blueprint wraps this in the wall-clock-paced,
    /// multi-region 20 TPS loop (out of scope here).
    pub fn tick_region(&self, region: &mut RegionState, pool: &RcWorkerPool, transport: &dyn Transport) -> TickReport;
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus `crates/scheduler/src/{access.rs, conflict_graph.rs, pipeline.rs, region.rs, registry.rs, executor.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments stay exactly as specified), plus the one `Cargo.toml` edit and the `lib.rs` module/re-export additions. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/scheduler/tests/`, and must not change any type's field list, derive list, or public signature from what the test changeset already compiled against.

Every test file below defines its own tiny synthetic `bevy_ecs::Component` marker types and system functions directly inside the test file (never in `src/`), mirroring M0-B02's own `MockTransport`-in-test-file convention. A shared `crates/scheduler/tests/common/mod.rs` holds the handful of marker components/helpers reused across files:

```rust
// tests/common/mod.rs
use bevy_ecs::prelude::*;

#[derive(Component, Default)] pub struct A(pub i64);
#[derive(Component, Default)] pub struct B(pub i64);
#[derive(Component, Default)] pub struct Marker;

pub fn empty_bootstrap(_world: &mut World) {}

/// A `MockTransport` identical in shape to M0-B02's own `fifo_property.rs` mock
/// (bounded per-`RegionId` `VecDeque` behind a `Mutex`), reused here as this
/// blueprint's own test-only `Transport` implementation — not a dependency on
/// `rc-transport-inproc` (which `rc-scheduler` must never depend on, `xtask
/// lint-deps` Rule 2).
pub struct MockTransport { /* ... */ }
impl MockTransport {
    pub fn new() -> Self;
    /// Test helper: pushes `msg` directly into `into`'s inbox queue, bypassing `send`.
    pub fn seed(&self, into: rc_messaging::RegionId, msg: rc_messaging::Message<rc_messaging::RegionMessage>);
    /// Test helper: returns every message ever passed to `send`, in call order.
    pub fn sent(&self) -> Vec<rc_messaging::Message<rc_messaging::RegionMessage>>;
}
impl rc_messaging::Transport for MockTransport { /* send/try_recv, as M0-B02's mock */ }
```

### `crates/scheduler/tests/compute_waves_conflict_graph.rs` (pure algorithm — no `World`, no threads)

1. `all_disjoint_systems_form_a_single_wave` — 4 synthetic `ComponentAccessSummary`s over 4 pairwise-disjoint components (via `ComponentAccessSummary::new`, using `World::new().register_component::<T>()`-obtained real `ComponentId`s for 4 distinct marker types, or any 4 distinct `ComponentId` values obtainable without a full system — implementer's choice of the cheapest way to get 4 distinct `ComponentId`s). Assert `compute_waves` returns exactly one wave containing all 4 indices (order within the wave: `[0,1,2,3]`).
2. `fully_conflicting_chain_serializes_completely` — 3 summaries all writing the *same* single component. Assert `compute_waves` returns exactly `[[0],[1],[2]]` (three separate waves, one system each, ascending order).
3. `read_read_never_conflicts` — 2 summaries both reading (never writing) the same component. Assert one wave, `[[0,1]]`.
4. `write_read_conflicts` — summary 0 writes component X; summary 1 reads X. Assert two waves, `[[0],[1]]`.
5. `mixed_graph_batches_disjoint_pairs_together` — 4 summaries: 0 writes X; 1 writes Y (disjoint from 0); 2 writes X (conflicts with 0 only); 3 reads Y (conflicts with 1 only). Assert `compute_waves` returns exactly `[[0,1],[2,3]]` (0 and 1 are mutually compatible and both have in-degree 0 initially; 2 depends on 0, 3 depends on 1; 2 and 3 are mutually compatible with each other).
6. `wildcard_write_is_isolated_from_every_other_system` — 3 summaries: 0 is a normal writer of X; 1 is `ComponentAccessSummary::wildcard(false, true)` (writes_all); 2 is a normal reader of Y (disjoint from X). Assert `compute_waves` returns exactly `[[0,2],[1]]` (1 conflicts with both 0 and 2 by the `writes_all` rule and is declared after both, so it lands alone in the final wave; 0 and 2 are mutually compatible and share the first wave).
7. `empty_group_returns_no_waves` — `compute_waves(&[])` returns `[]`.

### `crates/scheduler/tests/access_compatibility.rs` (pure, `ComponentAccessSummary::is_compatible` directly)

1. `disjoint_writes_are_compatible` — summary A writes X, summary B writes Y (X != Y): `A.is_compatible(&B)` and `B.is_compatible(&A)` both `true`.
2. `same_write_is_incompatible` — both summaries write X: `is_compatible` `false` both directions.
3. `write_and_read_of_same_component_is_incompatible` — A writes X, B reads X: `is_compatible` `false` both directions.
4. `two_reads_of_same_component_are_compatible` — both summaries read X, write nothing: `is_compatible` `true` both directions.
5. `reads_all_conflicts_with_any_write` — a `wildcard(true, false)` summary vs. a normal single-component writer: `is_compatible` returns `false`.
6. `reads_all_is_compatible_with_reads_all` — two `wildcard(true, false)` summaries: `is_compatible` returns `true`.
7. `writes_all_conflicts_with_everything_including_itself` — `wildcard(false, true)` vs. an empty (`Default::default()`) summary, and vs. another `wildcard(false, true)`: both `false`.

### `crates/scheduler/tests/registration_validation.rs` (uses `RcExecutorBuilder` against a real, throwaway `World`)

1. `structural_write_conflicting_with_declared_mutable_access_is_rejected` — register one system whose `access` (via a real `Query<&mut common::A>` system function) writes `common::A`'s `ComponentId`, and whose `structural_writes` also contains that same `ComponentId`. `builder.build()` returns `Err(ExecutorBuildError::AmbiguousMutationAuthority { .. })` naming that exact system and component.
2. `structural_write_on_a_different_component_than_declared_access_is_accepted` — same system's `access` writes `common::A`, `structural_writes` names `common::B`'s `ComponentId` instead. `builder.build()` returns `Ok(_)`.
3. `structural_write_alone_with_no_query_access_is_accepted` — a system with no `Query` parameter at all (e.g. a `Commands`-only system), `structural_writes` naming `common::A`. `builder.build()` returns `Ok(_)`.

### `crates/scheduler/tests/pipeline_ordering.rs` (integration — real `RcExecutor`, real `RcWorkerPool`)

1. `stages_4_6_8_9_11_execute_in_ascending_order` — register one instrumented system into each of the five domain groups, each appending its own `Stage` value to a shared `Arc<Mutex<Vec<Stage>>>` when it runs (no other side effects — no `Query`/`Commands`, `structural_writes: vec![]`, so nothing conflicts with anything). Build the executor, `spawn_region`, `tick_region` once with `RcWorkerPool::new(4)` and a fresh `MockTransport`. Assert the recorded log equals `[Stage::ScheduledBlockTick, Stage::EntityAiPhysics, Stage::Lighting, Stage::ChunkSnapshot, Stage::NetworkOutboundEncode]` exactly (proves inter-group full-drain-barrier ordering).
2. `conflicting_systems_in_the_same_group_never_overlap` — two systems both declaring `Query<&mut common::A>` (conflicting) registered into `DomainGroup::AiPhysics`; each system, on entry, increments a shared `Arc<AtomicI32>` "active count", records the post-increment value into a shared `Arc<Mutex<Vec<i32>>>`, sleeps 20ms (a deliberately generous window to make an overlap bug easy to observe, not a correctness-relevant delay), then decrements the active count on exit. Run with `RcWorkerPool::new(4)`. Assert every recorded value in the log is `1` (never `2`) — the active-count high-water-mark never exceeded 1 for this conflicting pair.
3. `disjoint_systems_in_the_same_group_can_overlap` — as test 2 but one system writes `common::A`, the other writes `common::B` (disjoint), both registered into `DomainGroup::Lighting`; both use a shared `std::sync::Barrier::new(2)`, each calling `.wait()` mid-body. Run with `RcWorkerPool::new(2)`. The test passes iff `tick_region` returns at all within a bounded overall test timeout (a hung barrier — proving the two systems were *not* forcibly serialized — would otherwise deadlock the test process; `cargo-nextest`'s own per-test timeout, M0-B01's WS-D10, is the backstop). This is a constructive proof that RC-Executor does *not* over-conservatively serialize compatible systems.

### `crates/scheduler/tests/sync_points.rs` (integration)

1. `deferred_command_in_stage_9_is_invisible_until_after_stage_10` — register two systems into `DomainGroup::ChunkSerialize`: order_tag 0 spawns one entity with `common::Marker` via `Commands` (`structural_writes: vec![marker_component_id]`, no `Query` access, so it never conflicts with order_tag 1); order_tag 1 has `Query<&common::Marker>` and, on entry, records `query.iter().count()` into a shared `Arc<AtomicUsize>`. `tick_region` once. Assert the recorded count is `0` (the spawn was not yet visible inside Stage 9, deferred). After `tick_region` returns, query `region.world` directly (test code, not a system) for `common::Marker` and assert exactly 1 entity now exists (applied at Stage 10).
2. `stage_4_command_is_visible_to_the_very_next_stage_4_system_inline` — as test 1 but both systems registered into `DomainGroup::BlockRedstone` (Stage 4) instead, order_tag 0 then 1. Assert the recorded count **is** `1` (visible immediately, the ARCH-D9 exception) — directly contrasting with test 1's `0`.
3. `stage_10_apply_order_is_stage_then_order_tag_ascending` — register three systems whose `Commands` bodies each spawn a distinct-valued `common::A(i64)` in a way whose *insertion order into the final `World`* is externally observable only through a monotonic counter resource (simplest: give each spawned entity's `common::A.0` the value `100*stage_number + order_tag`, spawned via `Commands`), spread across `DomainGroup::AiPhysics` (order_tag 0 and 1) and `DomainGroup::Lighting` (order_tag 0) — so declared conflict-graph waves are irrelevant to this test's assertion (each domain group has its own independent `order_tag` numbering). After `tick_region`, query `region.world` for all `common::A` values and assert they are present; this test's real assertion is on **test 4** below, which needs actual apply-order-sensitive state (a single shared counter component each system's command increments) — see test 4.
4. `stage_10_apply_order_is_deterministic_and_matches_declaration_order` — one single pre-existing entity holding a `common::A(0)` (spawned directly into `region.world` by test setup, before ticking, so it already exists at Stage 6/8/9 time). Three systems, each with `Query<&mut common::A>` (all mutually conflicting, so `compute_waves` fully serializes each group's own internal writers, but that is irrelevant here since each is in a *different* group): one in `AiPhysics` sets `A.0 = A.0 * 10 + 6`; one in `Lighting` sets `A.0 = A.0 * 10 + 8`; one in `ChunkSerialize` sets `A.0 = A.0 * 10 + 9` — **but** applied via direct live `Query<&mut A>` mutation, not `Commands`, so this specifically tests that non-deferred, live-`Query` mutations (already governed purely by inter-group ordering, no sync point involved) also respect the fixed stage sequence. After `tick_region`, assert the entity's final `A.0 == 689` (6 applied at Stage 6, then 8 at Stage 8, then 9 at Stage 9 — proving both intra-tick stage ordering and, since each stage's write is immediately live, that live mutations need no sync point to observe stage-ordering correctly).
5. `inbound_messages_are_invisible_until_drained_at_stage_1` — build an `RcExecutor` with zero registered systems; `spawn_region`; a `MockTransport` seeded (via `.seed(...)`) with one `BorderUpdateEvent`-shaped message addressed to the region. Before calling `tick_region`, assert `region.message_state.inbox().is_empty()`. Call `tick_region` once. Assert `region.message_state.inbox()` now contains exactly that one message's payload (M0-B02's Stage-1 contract, fulfilled).
6. `outbound_bus_merged_before_tick_is_flushed_at_stage_10` — build an `RcExecutor` with zero registered systems; `spawn_region`; before calling `tick_region`, construct a `RegionMessageBus`, call `.send(Address::Region(RegionId(999)), RegionMessage::BorderUpdateEvent(..))` on it, and `region.message_state.merge(bus)` (simulating what a future message-emitting system will eventually do automatically — Context's "Why message-sending is not modeled..." note). Call `tick_region` once with a fresh `MockTransport`. Assert `transport.sent()` contains exactly one message, with `.from == region.id`, `.tick_stamp == 0` (the tick counter's value *before* this tick's `tick_counter += 1`, i.e. the tick that produced it), and the correct payload.

### `crates/scheduler/tests/determinism.rs` (integration — the same setup ticked under different `RcWorkerPool` sizes)

1. `same_final_state_across_worker_counts` — a single pre-existing entity holding `common::A(0)`; four systems all declaring `Query<&mut common::A>` (fully mutually conflicting, forced sequential regardless of worker count) registered into `DomainGroup::AiPhysics`, each doing `a.0 += 1`. For each of `RcWorkerPool::new(n)` with `n` in `{1, 2, 8}`: build a **fresh** executor/region (state must not carry over between runs), `tick_region` once, read the entity's final `A.0`. Assert all three runs produce `A.0 == 4`.
2. `same_emitted_message_sequence_across_worker_counts` — five systems, each with distinct, mutually-disjoint `Query<&mut T>` access (five distinct marker components, so all land in one wave and may run fully concurrently), spread one each across the five domain groups; each system's `structural_writes` is empty and each merges a synthetic `RegionMessageBus` (built and `merge`d by *test setup* before ticking, one per system, each containing one message whose marker payload encodes that system's `(stage, order_tag)` — reusing the "merge before ticking" pattern from `sync_points.rs` test 6, since real systems cannot yet call `.send()` themselves per Context) into `region.message_state` **in a fixed, pre-tick order matching each system's `(stage, order_tag)`** before `tick_region` runs. For `n` in `{1, 2, 8}`: fresh executor/region, `tick_region` once, capture `transport.sent()`'s marker sequence. Assert all three runs produce the identical marker sequence, equal to the `(stage, order_tag)`-ascending order the merges were performed in (proving Stage 10's apply order is worker-count-independent — this test's real target is `drain_outbox`'s FIFO/merge-order guarantee already fixed by M0-B02, exercised here end-to-end through `tick_region` across worker counts, since `tick_region`'s own dispatch concurrency must never reorder or duplicate a `RegionMessageState` whose contents were fixed before it was ever called).

## Implementation steps

1. **`pipeline.rs`.** Implement `Stage`/`DomainGroup` bodies: `DomainGroup::stage` is the five-arm match from the Context mapping table; `DomainGroup::index` matches `ALL`'s declaration order (`BlockRedstone=0, AiPhysics=1, Lighting=2, ChunkSerialize=3, NetCodec=4`). Observable: `cargo build -p rc-scheduler` succeeds for this file in isolation.
2. **`access.rs`.** Implement `ComponentAccessSummary::new`/`wildcard` (trivial struct literals) and `is_compatible` per the Context compatibility rule exactly. Leave `from_bevy_access`'s body for step 6 (needs the verified `bevy_ecs` API points). Observable: `compute_waves_conflict_graph.rs` and `access_compatibility.rs` (both construct summaries only via `new`/`wildcard`) can now compile and pass.
3. **`conflict_graph.rs`.** Implement `compute_waves` per the pseudocode in Context (build directed incompatibility edges `i -> j` for `i < j`; Kahn's algorithm layering; ascending-index tie-break within a wave — trivially satisfied since the ready-set filter iterates `0..n` in order). Observable: all 7 `compute_waves_conflict_graph.rs` cases pass.
4. **`registry.rs`.** Implement `RcExecutorBuilder::new`/`register_system` (accumulate into `groups[group.index()]`, `order_tag = groups[group.index()].len()` before pushing). `build`'s body needs `access.rs`'s `from_bevy_access` (step 6) — implement `build` now with a `todo!()` placeholder only inside the `from_bevy_access` call site if sequencing this before step 6; otherwise implement steps 5-6 first. Observable (once complete): `registration_validation.rs` passes.
5. **`region.rs`.** Field-only struct; no method bodies beyond what `executor.rs` needs (its fields are `pub`/`pub(crate)` as specified — no additional methods required by this blueprint). Observable: compiles once `executor.rs` references it correctly.
6. **Verify and implement the five `bevy_ecs` API points** (Context's dedicated subsection) against the actually-installed `bevy_ecs` 0.19.1 documentation. Implement `access.rs`'s `from_bevy_access` and `executor.rs`'s dispatch bodies (`spawn_region`'s `.initialize` calls; `run_group_deferred`'s safe single-member-wave `.run()` path and unsafe multi-member-wave `UnsafeWorldCell` path with the required `update_archetype_component_access` call before each `run_unsafe`; Stage 4's `.run()` + immediate apply-deferred loop; Stage 10's sorted apply-deferred loop; Stage 11's run-without-apply loop). Every `unsafe` block carries a `// SAFETY:` comment citing `compute_waves`'s compatibility proof by name (Constraints (d)). Observable: `cargo build -p rc-scheduler` succeeds with zero `todo!()` remaining; `pipeline_ordering.rs` and `sync_points.rs` pass.
7. **`RcExecutorBuilder::build`.** Complete its body: prototype `World::new()` + `bootstrap`; for each group, for each registration, construct one instance via `factory()`, `.initialize(&mut prototype)`, extract `ComponentAccessSummary::from_bevy_access(&system.component_access())`, check `access.writes ∩ structural_writes` (return `Err` on the first violation, deterministic order per Deliverables' doc comment), then `compute_waves` over that group's summaries. Observable: `registration_validation.rs` fully passes (test 1's rejection path now reachable).
8. **Determinism suite.** No new production code beyond what steps 1-7 already deliver — `determinism.rs`'s tests exercise the already-complete `RcExecutor`/`RegionState`/`RegionMessageState` stack under varied `RcWorkerPool::new(n)`. Observable: both `determinism.rs` cases pass for `n` in `{1, 2, 8}`.
9. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four still exit 0.
10. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/scheduler/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full field lists, full derives, full doc comments) and the `Cargo.toml`/`lib.rs` edits. The implementation changeset (steps 1-10) fills in real bodies only — it must not edit any test file, must not add, remove, or rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, `same_final_state_across_worker_counts`'s exact expected value `4`, `stage_10_apply_order_is_deterministic_and_matches_declaration_order`'s exact expected value `689`, and every ordering assertion in `pipeline_ordering.rs`/`sync_points.rs` must survive unchanged).

(b) **No new external dependencies beyond the pinned set.** Only `bevy_ecs` and `thiserror` are added to `rc-scheduler`'s `Cargo.toml` by this blueprint, both already present in the workspace root's `[workspace.dependencies]` table (M0-B01) at their pinned versions — neither version is altered. `crossbeam-deque`/`crossbeam-utils`/`parking_lot` are M0-B04's own additions, left untouched. Do not add `rayon`, `dashmap`, `anyhow`, or any other crate under any circumstance.

(c) **No Mojang or third-party reimplementation code.** Nothing in this blueprint touches protocol, mechanics content, or any decompiled source — every algorithm here (the conflict graph, the sync-point mechanism) is derived solely from `01-server-architecture.md`'s ARCH-D1–D4/D8/D9/D12/D13/D14 and this blueprint's own concrete, cited resolutions of what those decisions left open (ASSET-D18/D19/D30).

(d) **Unsafe-code policy — permitted, narrowly, with a mandatory safety argument.** Unlike M0-B01/M0-B02, this blueprint's Deliverables **do** use `unsafe`: dispatching two or more systems to run concurrently against the same `bevy_ecs::World` requires `UnsafeWorldCell` (the same technique `bevy_ecs`'s own built-in `MultiThreaded` executor uses internally — ARCH-D3's own rationale: "Reusing `bevy_ecs`'s `System::component_access()` output while replacing only the execution strategy keeps the safety guarantees without the pool rigidity"). The **sole** invariant that makes this sound is `compute_waves`'s compatibility proof (Deliverables' doc comment on `compute_waves`): every wave with more than one member has already been proven pairwise access-compatible before `executor.rs` ever dispatches it concurrently. Every `unsafe` block in this blueprint's Deliverables must carry a `// SAFETY:` comment citing this invariant by name. No other use of `unsafe` is permitted anywhere in this blueprint's Deliverables — in particular, `run_batch`'s M0-B04-owned internals are never reimplemented or bypassed here.

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: ARCH-D5/D6 region build/merge/split or the ARCH-D24 directories (a separate `rc-scheduler` blueprint); ARCH-D10/D11's actual application of inbound `RegionMessage` payloads (needs `rc-mechanics`, `M3`/`M4`); ARCH-D15's Stage-6a/6b split and reconciliation pass, ARCH-D16/D17's Stage-8/Stage-7 mechanics-specific ordering (all need real components that do not exist at `M0` — this blueprint's Stage 6/7/8 dispatch is deliberately generic, with Stage 7 accepting no registration at all); ARCH-D18–D23 (RC-WorkerPool's own sizing/EDF, M0-B04's and a later blueprint's jobs); the real-time wall-clock-paced multi-region 20 TPS loop and `xtask`-level soak-test harness (M0's acceptance criterion 1 — composes this blueprint's `tick_region`, not implemented here); the `bevy_ecs` `SystemParam`/`Local<T>` mechanism for handing a running system its own private `RegionMessageBus` (Context's "Why message-sending is not modeled..." note — deferred to whichever blueprint first implements a real message-emitting system). Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it.

(f) **Known limitation, not solved by this blueprint.** A Stage-11 system that uses `Commands` despite the stage's documented read-only contract has its accumulated deferred-command state silently retained and never flushed for the lifetime of its region (Context: "Stage 11 is read-only"). This is a slow, unbounded memory growth in that pathological case, not a correctness bug in normal operation (no native system registers into `NetCodec`/Stage 11 with `Commands` usage at `M0`, since no mechanics content exists). A future blueprint that first registers a real Stage-11 system should either enforce "no `Commands` parameter" at registration time or add an explicit discard-without-apply drain call — neither is implemented here.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler --all-features
cargo nextest run -p rc-scheduler
cargo test --doc -p rc-scheduler
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-scheduler` runs all 7 (`compute_waves_conflict_graph.rs`) + 7 (`access_compatibility.rs`: 4 base-case tests bundled under item 1, plus items 2-4) + 3 (`registration_validation.rs`) + 3 (`pipeline_ordering.rs`) + 6 (`sync_points.rs`) + 2 (`determinism.rs`) = 28 test cases named in Acceptance tests — all pass, with `pipeline_ordering.rs`'s test 3 and every `determinism.rs` case run against real `RcWorkerPool` instances of sizes 1, 2, 4, and 8 across the suite, with zero flakiness (no test in this suite uses `std::thread::sleep` as a synchronization mechanism — only as a deliberately generous window in `conflicting_systems_in_the_same_group_never_overlap` to make a genuine overlap bug easy to observe, never to make a passing assertion depend on timing). CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
